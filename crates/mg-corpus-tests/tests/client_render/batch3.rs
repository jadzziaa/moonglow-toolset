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
    let c = instance(&placing, ResType::UTC, &bp.root, at, &[]).unwrap();
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
            ("commoner", 0.40, 0.42),
            ("armoire", 0.58, 0.42),
        ] {
            eprintln!("{case}: {what}: {:?}", count_at(&client, x, y));
        }
    }
}
