//! The deferred sprint's third client batch (2026-10-29): readings of the
//! client that answer the deferred list's questions, beside the first and
//! second batches' (`readings.rs`, `batch2.rs`, whose scaffolding they
//! use). Each writes the client's pictures to its scratch directory
//! (`target/test-output/client_reading_<name>/`) and prints where; what
//! was seen is in its comment.
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=:2 cargo test -p mg-corpus-tests --release --test client_render batch3 -- --ignored --nocapture`.

use super::readings::{INVISIBLE, Reading, enter};
use super::*;

/// The game's `inc_standard.shd` with the lit fragments of every other
/// four columns of pixels painted with the count of the lights their draw
/// was handed (the columns between left as drawn, to see what is where): red `floor(n / 64 × 255)`,
/// and blue the same of those lights without green (which of them were
/// kept), so that the floor, a creature and a placeable each show their
/// own draw's.
fn light_count_shader(game: &GameData) -> String {
    let src = debug_shader(game);
    let from = "    int row = int(gl_FragCoord.y / 32.0) - 6;\n    if (row < 0 || row >= 4) return;\n    float x = clamp(MgValue(row * 32 + int(gl_FragCoord.x / 32.0)), 0.0, 0.99999);\n    float hi = floor(x * 255.0);\n    c = vec4(hi / 255.0, fract(x * 255.0), 0.0, 1.0);";
    assert!(src.contains(from), "the debug shader's painting");
    src.replace(
        from,
        "    if (int(gl_FragCoord.x) % 8 < 4) return;\n    float n = floor(MgValue(1) * 255.0);\n    float g = floor(MgValue(31) * 255.0);\n    \
         c = vec4(n / 255.0, 0.0, g / 255.0, 1.0);",
    )
}

/// The count a byte of [`light_count_shader`] stands for.
fn count(byte: u8) -> u32 {
    ((byte as f32 + 0.5) * 64.0 / 255.0).round() as u32
}

/// The light counts on `img` and how many pixels show each (red, blue).
fn counts(img: &Rgba) -> std::collections::BTreeMap<(u32, u32), usize> {
    let mut out = std::collections::BTreeMap::new();
    for (i, p) in img.data.chunks_exact(4).enumerate() {
        if i % img.width as usize % 8 >= 4 && p[1] == 0 && (p[0] > 0 || p[2] > 0) {
            *out.entry((count(p[0]), count(p[2]))).or_default() += 1;
        }
    }
    out
}

/// The counts at (x, y) of `img` (fractions of its size, top-left origin;
/// the nearest painted column).
fn count_at(img: &Rgba, x: f32, y: f32) -> (u32, u32, [u8; 3]) {
    let (px, py) = ((x * img.width as f32) as usize, (y * img.height as f32) as usize);
    let px = px / 8 * 8 + 5;
    let i = (py * img.width as usize + px) * 4;
    let p = &img.data[i..i + 3];
    (count(p[0]), count(p[2]), [p[0], p[1], p[2]])
}

/// `img` with each count a bright color (for looking at).
fn count_picture(img: &Rgba) -> Rgba {
    let mut out = img.clone();
    let w = out.width as usize;
    for (i, p) in out.data.chunks_exact_mut(4).enumerate() {
        if i % w % 8 >= 4 && p[1] == 0 && p[0] > 0 {
            let n = count(p[0]);
            let c: [u8; 3] = match n {
                1 => [255, 0, 0],
                2 => [0, 255, 0],
                3 => [0, 0, 255],
                4..=8 => [255, 255, 0],
                9..=31 => [0, 255, 255],
                _ => [255, 0, 255],
            };
            p[..3].copy_from_slice(&c);
        }
    }
    out
}

/// A creature of `resref` placed at `at` facing `facing` (degrees).
fn put_creature(r: &mut Reading, resref: &str, at: [f32; 3], facing: f32) {
    use mg_module::instances::{Placement, Placing, instance};
    let bp = Gff::read(&r.game.resman.get_named(resref, ResType::UTC).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &r.game, item: &none };
    let at = Placement { position: at, rotation: facing.to_radians() };
    let mut c = instance(&placing, ResType::UTC, &bp.root, at, &[]).unwrap();
    // (Standing still: no scripts.)
    let scripts: Vec<String> = c
        .fields
        .iter()
        .map(|f| f.label.to_string_lossy())
        .filter(|l| l.starts_with("Script"))
        .collect();
    for label in scripts {
        c.set(&label, Value::resref(ResRef::EMPTY));
    }
    let mut list = r.git.root.list("Creature List").map(<[_]>::to_vec).unwrap_or_default();
    list.push(c);
    r.git.root.set("Creature List", Value::List(list));
}

/// Which draws a light reaches, and how many lights one draw takes (the
/// "Lights placed on the fly, way B" entry's two questions): in a dark
/// tic01 room (no sun, no tile lights, a gray ambient), a commoner at (18.5, 23) and an
/// armoire at (21.5, 23) facing the camera, and behind them, at (20, 24),
/// the light of each case; the debug shader paints every lit fragment
/// with its draw's light count ([`light_count_shader`]). Cases
/// (`MG_B3_CASES`, comma-separated, to run some): `none`; `row`, the
/// game's "Light, White" (placeables.2da `LightColor`); `node`, a box
/// (Box1's model replaced) with one white light node of radius 5; `many`,
/// 40 "Light, White" in a ring 2 m around (20, 24); `many-red`, the same
/// with 4 of them "Light, Red" nearest the creature.
///
/// Seen (2026-10-29, 89.8193.37): `none` 0 lights everywhere; `row` and
/// `node` 1 light on the floor, the commoner and the armoire alike;
/// `many` and `many-red` 32 on the floor and walls (the objects hidden by
/// the lights' glows), and 40 with `MG_MAX_LIGHTS=64`: a draw takes as
/// many as the client's Max Lights.
#[test]
#[ignore]
fn reading_lights_on_creatures_and_placeables() {
    let cases = ["none", "row", "node", "many", "many-red"];
    let only = std::env::var("MG_B3_CASES").ok();
    for case in cases {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|c| c == case)) {
            continue;
        }
        // (An ambient light, to see the objects between the painted
        // columns: it isn't one of the draw's lights.)
        let light = Lighting { ambient: 0x606060, diffuse: 0, main_light: 0, only_tile: None };
        let probe = Reading::new("light_probe", "tic01", light, &[]);
        let shader = light_count_shader(&probe.game).into_bytes();
        let mut overrides = vec![("inc_standard.shd", shader)];
        if case == "node" {
            let lamp = NodeLight::white(5.0);
            overrides.push(("plc_a03.mdl", light_model("plc_a03", &[lamp]).into_bytes()));
        }
        let mut r = Reading::new(&format!("lights_{case}"), "tic01", light, &overrides);
        put_creature(&mut r, "nw_commale", [18.5, 23.0, 0.0], 270.0);
        r.place(0, [21.5, 23.0, 0.0], 270.0, &[("Static", Value::Byte(0))]);
        let lamp = |r: &mut Reading, row: u32, x: f32, y: f32| {
            r.place(row, [x, y, 1.5], 0.0, &[("Static", Value::Byte(0))]);
        };
        match case {
            "row" => lamp(&mut r, 15112, 20.0, 24.0),
            "node" => r.place(2, [20.0, 24.0, 0.0], 0.0, &[("Static", Value::Byte(0))]),
            "many" | "many-red" => {
                for i in 0..40 {
                    let turn = i as f32 / 40.0 * std::f32::consts::TAU;
                    // (The four nearest the creature, west, red in many-red.)
                    let west = (turn - std::f32::consts::PI).abs() < 0.35;
                    let row = if case == "many-red" && west { 15111 } else { 15112 };
                    lamp(&mut r, row, 20.0 + 2.0 * turn.cos(), 24.0 + 2.0 * turn.sin());
                }
            }
            _ => {}
        }
        // (`MG_MAX_LIGHTS`: the client's Max Lights, 32 by default.)
        let settings = match std::env::var("MG_MAX_LIGHTS") {
            Ok(n) => format!("{SETTINGS}[graphics.lighting]\n\tmax-lights = {n}\n"),
            Err(_) => SETTINGS.to_string(),
        };
        let (dir, client) = r.shoot(&enter(INVISIBLE, (90.0, 12.0, 45.0)), &settings);
        save_png(&count_picture(&client), &dir.join("counts.png"));
        eprintln!("{case}: counts (lights, without green) → pixels: {:?}", counts(&client));
        for (what, x, y) in [
            ("floor near", 0.5, 0.75),
            ("floor middle", 0.5, 0.55),
            ("commoner", 0.44, 0.41),
            ("armoire", 0.56, 0.41),
        ] {
            eprintln!("{case}: {what}: {:?}", count_at(&client, x, y));
        }
    }
}

/// An entry script's statements writing where every creature and
/// placeable of the area stands to the log as the player enters
/// (`MG_AT <tag> x y z`).
const WHERE: &str = r#"object o = GetFirstObjectInArea(area);
    while (GetIsObjectValid(o))
    {
        vector v = GetPosition(o);
        if (GetObjectType(o) == OBJECT_TYPE_CREATURE || GetObjectType(o) == OBJECT_TYPE_PLACEABLE)
            Log("MG_AT " + GetTag(o) + " " + FloatToString(v.x, 0, 3) + " " + FloatToString(v.y, 0, 3) + " " + FloatToString(v.z, 0, 3));
        o = GetNextObjectInArea(area);
    }"#;

/// The client's settings with the environment's shadows on and its
/// Creature Shadow Detail `mode` (0 Off, 2 Best).
fn shadow_settings(mode: u8) -> String {
    SETTINGS
        .replace(
            "[graphics.shadows.creatures]\n\t\t\tmode = 0",
            &format!("[graphics.shadows.creatures]\n\t\t\tmode = {mode}"),
        )
        .replace(
            "[graphics.shadows.environment]\n\t\t\tenabled = false",
            "[graphics.shadows.environment]\n\t\t\tenabled = true",
        )
}

/// How much `best` is darker than `off` in a rectangle (fractions,
/// top-left origin): 1 less the ratio of their mean brightnesses.
fn darker(best: &Rgba, off: &Rgba, r: [f32; 4]) -> f32 {
    let b = |img: &Rgba| {
        let m = mean(img, r);
        (m[0] + m[1] + m[2]) / 3.0
    };
    1.0 - b(best) / b(off).max(1e-3)
}

/// The lights' shadows (the shadows entry's two client items): an ogre
/// standing still at (20, 22) in a dim tic01 room (ambient and diffuse
/// 0x303030, no sun or moon shadows), lit by lamp posts (placeables.2da
/// row 73: a `LightColor` light that casts), each case taken with the
/// client's Creature Shadow Detail Best and Off and the two pictures
/// compared. `near_<d>`: one lamp `d` m north of the ogre (1.5, 3 and
/// 5), its shadow falling south towards the camera, measured just south
/// of the ogre's feet (the same place on the screen in each, so the
/// shadow plane's fade up the screen is the same) and further along.
/// `equal_we` and `equal_ew`: two lamps 2.5 m west and east of the ogre
/// (in that order in the area's placeables, and the other way), whose
/// shadows would fall east and west: which the client casts.
/// `equal_ns` and `equal_sn` the same north and south. `MG_B3_CASES`
/// runs some.
///
/// Seen (2026-10-29, 89.8193.37): the floor's darkness in the shadow just
/// south of the feet (medians, below the ogre's own pixels) 0.50, 0.53
/// and 0.38 at 1.5, 3 and 5 m, the shadow reaching further each time; of
/// the equal pairs, in either order, the west lamp's shadow and the
/// south lamp's (`MG_AT`: the server has the ogre at (20, 22) and the
/// lamps where they were put).
#[test]
#[ignore]
fn reading_light_shadows() {
    let cases: [(&str, &[(f32, f32)]); 7] = [
        ("near_1.5", &[(20.0, 23.5)]),
        ("near_3", &[(20.0, 25.0)]),
        ("near_5", &[(20.0, 27.0)]),
        ("equal_we", &[(17.5, 22.0), (22.5, 22.0)]),
        ("equal_ew", &[(22.5, 22.0), (17.5, 22.0)]),
        ("equal_ns", &[(20.0, 24.5), (20.0, 19.5)]),
        ("equal_sn", &[(20.0, 19.5), (20.0, 24.5)]),
    ];
    let only = std::env::var("MG_B3_CASES").ok();
    for (case, lamps) in cases {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|c| c == case)) {
            continue;
        }
        let mut pictures = Vec::new();
        for mode in [2u8, 0] {
            let light =
                Lighting { ambient: 0x303030, diffuse: 0x303030, main_light: 0, only_tile: None };
            let mut r = Reading::new(&format!("light_shadows_{case}_{mode}"), "tic01", light, &[]);
            for (label, v) in [("SunShadows", 0u8), ("MoonShadows", 0), ("ShadowOpacity", 100)] {
                r.are.root.set(label, Value::Byte(v));
            }
            put_creature(&mut r, "nw_ogre01", [20.0, 22.0, 0.0], 270.0);
            for &(x, y) in lamps {
                r.place(73, [x, y, 0.0], 0.0, &[("Static", Value::Byte(0))]);
            }
            // (Where the ogre and the lamps stand in the game, written to
            // the log: the creature's place is the walkmesh's.)
            let body = format!("{INVISIBLE}\n{WHERE}");
            let (dir, client) = r.shoot(&enter(&body, (90.0, 12.0, 45.0)), &shadow_settings(mode));
            super::readings::print_log(&dir);
            pictures.push((dir, client));
        }
        let (best, off) = (&pictures[0].1, &pictures[1].1);
        // The difference, for looking at: where Best is darker, white.
        let mut diff = best.clone();
        let pairs = best.data.chunks_exact(4).zip(off.data.chunks_exact(4));
        for (d, (b, o)) in diff.data.chunks_exact_mut(4).zip(pairs) {
            let sum = |p: &[u8]| p[0] as f32 + p[1] as f32 + p[2] as f32;
            let v = ((sum(o) - sum(b)).max(0.0) * 2.0).min(255.0) as u8;
            d[..3].copy_from_slice(&[v, v, v]);
        }
        let shown = pictures[0].0.join("darker.png");
        save_png(&diff, &shown);
        eprintln!("{case}: where Best is darker: {}", shown.display());
        for (what, r) in [
            ("south of the feet", [0.47, 0.50, 0.53, 0.53]),
            ("further south", [0.47, 0.55, 0.53, 0.58]),
            ("west of the ogre", [0.36, 0.42, 0.44, 0.47]),
            ("east of the ogre", [0.56, 0.42, 0.64, 0.47]),
        ] {
            eprintln!("{case}: {what}: darker by {:.3}", darker(best, off, r));
        }
        // The shadow itself: the pixels about the ogre that Best darkens
        // (by 2 of 255 a channel or more), where they are and how dark.
        let (w, h) = (best.width as usize, best.height as usize);
        let (mut n, mut sums, mut bbox) = (0usize, [0.0f64; 2], [usize::MAX, usize::MAX, 0, 0]);
        let mut halves = [0usize; 2];
        // (And the floor's alone, below the ogre's feet: each pixel's.)
        let mut floor = Vec::new();
        for y in h * 35 / 100..h * 75 / 100 {
            for x in w * 30 / 100..w * 70 / 100 {
                let i = (y * w + x) * 4;
                let sum = |p: &[u8]| p[0] as f64 + p[1] as f64 + p[2] as f64;
                let (b, o) = (sum(&best.data[i..i + 3]), sum(&off.data[i..i + 3]));
                if o - b >= 6.0 {
                    if y * 1000 >= h * 505 {
                        floor.push(1.0 - b / o);
                    }
                    n += 1;
                    sums[0] += b;
                    sums[1] += o;
                    bbox = [bbox[0].min(x), bbox[1].min(y), bbox[2].max(x), bbox[3].max(y)];
                    halves[usize::from(x >= w / 2)] += 1;
                }
            }
        }
        eprintln!(
            "{case}: shadow: {n} pixels, darker by {:.3}, from ({}, {}) to ({}, {}); \
             west of the middle {}, east {}",
            1.0 - sums[0] / sums[1].max(1.0),
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            halves[0],
            halves[1]
        );
        floor.sort_by(f64::total_cmp);
        if !floor.is_empty() {
            eprintln!(
                "{case}: the floor below the feet: {} pixels, darker by {:.3} (the median)",
                floor.len(),
                floor[floor.len() / 2]
            );
        }
    }
}
