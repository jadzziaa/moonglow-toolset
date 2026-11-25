//! The fog at night with a short fog clip distance (Moonglow Client's
//! upstream ask A8): Moonglow Client's first picture, The Domains of
//! Dread's Shard (tileset `tms01`, night, `FogClipDist` 22, fog amount
//! 12), had the briars ten metres off darker and crisper than the
//! client's and the far ground meeting the night in a visible line.
//!
//! Each case is a 4×4 area with things (armoires) standing about 5, 10,
//! 15 and 20 m north of the player (10 to 25 m from the camera) and open
//! ground running past the fog clip distance, the camera low behind the
//! player looking north, as the Shard's was (`SetCameraFacing(90, 5.5,
//! 84)`). Moonglow draws the same with its own shader and the game's,
//! with its fog; the means of each thing and of the ground at each depth
//! are printed and compared.
//!
//! The cause: the client's fog starts nearer as its camera comes nearer
//! the player (`fog_start_follows_the_camera`), 3.5 m off for the
//! Shard's camera at 5.5 m where Moonglow had it 18 m off, as for the
//! client's camera as a player enters (20 m), which every earlier reading
//! had (`mg_area::fog_from`, `View::camera`).
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render night_fog -- --ignored --nocapture --test-threads 1`
//! (`MG_NIGHT_FOG_CASES=shard_night_22,...` picks cases).

use super::readings::{INVISIBLE, Reading, enter, logged};
use super::*;

/// One scene: its tileset (and the tile laid over the whole area, if
/// given), interior, night, its fog (amount, colour as stored) and clip
/// distance, and the client's settings.
struct Case {
    name: &'static str,
    tileset: &'static str,
    tile: Option<i32>,
    interior: bool,
    night: bool,
    fog: (u8, u32),
    clip: f32,
    /// The player's own settings (the post effects on), else [`SETTINGS`].
    player: bool,
}

/// The Shard of The Domains of Dread (`ooc_shard.are`): `tms01`'s tile 12
/// everywhere, an interior, night, its moon's and sun's colours.
const SHARD_AMBIENT: u32 = 1_841_176;
const SHARD_DIFFUSE: u32 = 7_892_073;
const SHARD_FOG: (u8, u32) = (12, 1_841_691);

const CASES: [Case; 7] = [
    Case {
        name: "shard_night_22",
        tileset: "tms01",
        tile: Some(12),
        interior: true,
        night: true,
        fog: SHARD_FOG,
        clip: 22.0,
        player: false,
    },
    Case {
        name: "shard_night_45",
        tileset: "tms01",
        tile: Some(12),
        interior: true,
        night: true,
        fog: SHARD_FOG,
        clip: 45.0,
        player: false,
    },
    Case {
        name: "shard_day_22",
        tileset: "tms01",
        tile: Some(12),
        interior: true,
        night: false,
        fog: SHARD_FOG,
        clip: 22.0,
        player: false,
    },
    Case {
        name: "shard_night_22_player",
        tileset: "tms01",
        tile: Some(12),
        interior: true,
        night: true,
        fog: SHARD_FOG,
        clip: 22.0,
        player: true,
    },
    Case {
        name: "rural_night_22",
        tileset: "ttr01",
        tile: None,
        interior: false,
        night: true,
        fog: (12, 0x0040_3020),
        clip: 22.0,
        player: false,
    },
    Case {
        name: "rural_night_45",
        tileset: "ttr01",
        tile: None,
        interior: false,
        night: true,
        fog: (12, 0x0040_3020),
        clip: 45.0,
        player: false,
    },
    Case {
        name: "rural_day_22",
        tileset: "ttr01",
        tile: None,
        interior: false,
        night: false,
        fog: (12, 0x0040_3020),
        clip: 22.0,
        player: false,
    },
];

/// Where the player stands: near the area's south edge, so that the
/// ground runs some 35 m north of the camera.
const PC: [f32; 2] = [20.0, 6.0];

/// The things standing north of the player: (x, y), at about 5, 10, 15
/// and 20 m, to either side of the view's middle column (left free for
/// the ground's profile).
const THINGS: [[f32; 2]; 4] = [[17.0, 11.0], [23.5, 16.0], [16.0, 21.0], [24.5, 26.0]];

/// The client's camera, as the script gives it, and Moonglow's for it
/// (distance, pitch, focus: fitted to `rural_night_22`'s and
/// `shard_night_45`'s pictures with `MG_FIT=1`, both the same; Moonglow
/// Client had fitted the Shard's as 5.25 m, 83 degrees, 1.5 m).
const SCRIPTED: (f32, f32, f32) = (90.0, 5.5, 84.0);
const FITTED: (f32, f32, f32) = (5.0, 84.0, 1.25);

/// What a case's run gives: the client's fog uniforms (with
/// `Paint::Uniforms`) and the means compared (`(what, client, Moonglow's
/// own shader, the game's)`, gray levels of 255).
struct Shot {
    uniforms: Option<(f32, f32)>,
    means: Vec<(String, f32, f32, f32)>,
}

/// What the game's own shaders are patched to paint, in the scratch
/// user directory's `override` (read by the client and by Moonglow's game
/// path alike).
#[derive(Clone, Copy, PartialEq)]
enum Paint {
    /// Nothing: the scene as it is.
    No,
    /// `inc_fog`'s `ApplyFog` paints `(fFogFragCoord + 5) / 10` as gray:
    /// how far into the fog each fragment is, before it is clamped.
    Fog,
    /// The debug `inc_standard` of `light_uniforms_match_the_client`: the
    /// client's fog uniforms are read back.
    Uniforms,
}

/// The fog (amount and clip distance) and the script's camera (distance
/// and pitch) a case is shot with: the case's own and the Shard's camera
/// unless given.
#[derive(Clone, Copy)]
struct Setup {
    amount: u8,
    clip: f32,
    camera: (f32, f32),
    pc_y: f32,
}

impl Setup {
    fn of(case: &Case) -> Setup {
        Setup { amount: case.fog.0, clip: case.clip, camera: (SCRIPTED.1, SCRIPTED.2), pc_y: PC[1] }
    }
}

/// Each case (or those `MG_NIGHT_FOG_CASES` names; `MG_FOG_PAINT=fog`
/// shoots them with [`Paint::Fog`] instead) in the client and in
/// Moonglow, both drawing paths, with Moonglow's fog for the client's
/// camera (`View::camera`, 5.5 m; drawn from 5 m at 84 degrees about
/// 1.25 m up, fitted to these pictures with `MG_FIT=1`): the gray level of
/// the ground in the view's middle column at each view depth and above
/// the horizon within 1.5 of 255 of the client's, of each thing (an
/// armoire, 40 cm about its middle) within 5 (the post effects' case,
/// `shard_night_22_player`, is printed, not held to it).
///
/// Seen 2026-11-19 (89.8193.37), before the fix (Moonglow's fog started
/// 18 m off whatever the camera; the client's, 3.5 m off for its camera
/// at 5.5 m), the camera then Moonglow Client's fit (5.25 m, 83 degrees,
/// 1.5 m up): `rural_night_22`'s things at 10, 15 and 20 m 27.3, 32.1 and
/// 45.1 in the client against 11.0, 5.6 and 23.2, the ground at 14 and
/// 17 m 38.6 and 42.2 against 27.3 and 25.5 (the fog's gray 48.0): the
/// first picture's darker briars and the line where the unfogged ground
/// met the fog between 18 and 22 m. The Shard's alike (things 19.7, 19.0
/// and 25.9 against 11.0, 5.6 and 15.4, its fog's gray 27.0). Day and
/// night the same.
///
/// After it, both paths alike: `rural_night_22`'s things 25.3, 31.7 and
/// 44.6 against 22.9, 30.3 and 42.2, the Shard's 17.9, 18.9 and 25.6
/// against 16.2, 18.0 and 24.1; the ground within 0.6 in every case; the
/// pictures 1.75 to 3.3 of 255 apart on average (2.8 to 5.8 before, from
/// the same camera, the client's pictures drawn beside again with
/// `MG_CLIENT_PICTURES`). The
/// things' rest is the armoire's own light, not the fog: what is left is
/// largest with the least fog (`rural_night_45`, 27.8 against 23.5 at
/// 20 m), and the fog painted (`MG_FOG_PAINT=fog`) is the same on them
/// (`rural_night_45`'s things at 10, 15 and 20 m 131.2, 134.3 and 137.6
/// in the client against 131.0, 134.0 and 137.0 on the game's path, of
/// 255 at 25.5 a fog unit).
/// With the player's post effects the client's ground is 2 to 2.5 darker.
#[test]
#[ignore]
fn night_fog_matches_the_client() {
    let only = std::env::var("MG_NIGHT_FOG_CASES").unwrap_or_default();
    let paint = if std::env::var("MG_FOG_PAINT").is_ok_and(|p| p == "fog") {
        Paint::Fog
    } else {
        Paint::No
    };
    let mut failures = Vec::new();
    for case in CASES.iter().filter(|c| only.is_empty() || only.split(',').any(|o| o == c.name)) {
        let shot = shoot(case, Setup::of(case), paint);
        if paint != Paint::No || case.player {
            continue;
        }
        for (what, client, off, on) in shot.means {
            let tolerance = if what.starts_with("thing") { 5.0 } else { 1.5 };
            for (path, ours) in [("own shader", off), ("game's", on)] {
                if (ours - client).abs() > tolerance {
                    failures.push(format!(
                        "{} ({path}): {what}: client {client:.1}, Moonglow {ours:.1}",
                        case.name
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The client's fog start follows its camera's distance from the player
/// (its fog uniforms read back, as `fog_uniforms_match_the_client` reads
/// them for the camera as a player enters): `FogAmount` nearer than 30 m
/// less the camera's distance short of 20 m, at most a metre before the
/// end, which stays at `FogClipDist`; whatever the pitch, by day and at
/// night. Each reading (amount, clip, the script's distance and pitch)
/// against [`mg_area::fog_from`]; `MG_FOG_READINGS=amount:clip:distance:pitch[:d],...`
/// (`d` by day) reads others.
///
/// Read 2026-11-19 (89.8193.37), the player 20 m in: amount 12, clip 22:
/// 3.50 at 5.5 m (pitch 84 and 45 alike), 6.00 at 8, 10.00 at 12, 18.00
/// at 20; clip 45: 6.00 at 8. Amount 0, clip 45: 15.50 at 5.5, 22.00 at
/// 12, 30.00 at 20, 35.01 at 25 (pitch 45 and 84; clip 60 by day: 35.00):
/// past 20 m the start goes on moving back. Amount 12 clip 45: 14.01 at
/// 16. Amount 30: -11.98 at 8. Amount 12 clip 22 at 25 m (pitch 30):
/// 20.99, a metre before the end. The end 22.00 or 45.01 in
/// each. (`SetCameraFacing(90, 12, 45)` given as the player enters keeps
/// the client's own camera, 20 m: 18.00 and 30.00.)
#[test]
#[ignore]
fn fog_start_follows_the_camera() {
    let readings: Vec<(u8, f32, f32, f32, bool)> = std::env::var("MG_FOG_READINGS")
        .ok()
        .map(|v| {
            v.split(',')
                .map(|r| {
                    let p: Vec<&str> = r.split(':').collect();
                    let night = p.get(4) != Some(&"d");
                    (
                        p[0].parse().unwrap(),
                        p[1].parse().unwrap(),
                        p[2].parse().unwrap(),
                        p[3].parse().unwrap(),
                        night,
                    )
                })
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                (12, 22.0, 5.5, 84.0, true),
                (12, 22.0, 5.5, 45.0, true),
                (12, 22.0, 8.0, 84.0, true),
                (12, 22.0, 12.0, 84.0, false),
                (12, 22.0, 20.0, 84.0, true),
                (0, 45.0, 5.5, 84.0, false),
                (0, 45.0, 12.0, 84.0, true),
                (30, 45.0, 8.0, 84.0, true),
                (12, 45.0, 16.0, 84.0, true),
                (0, 45.0, 25.0, 45.0, true),
            ]
        });
    let mut failures = Vec::new();
    for (amount, clip, distance, pitch, night) in readings {
        let case = if night { &CASES[4] } else { &CASES[6] };
        let setup = Setup { amount, clip, camera: (distance, pitch), pc_y: 20.0 };
        let shot = shoot(case, setup, Paint::Uniforms);
        let (start, end) = shot.uniforms.unwrap();
        let sky = mg_area::Sky { fog_amount: amount, ..Default::default() };
        let mut lighting = mg_area::Lighting::read(&mg_gff::Struct::default(), None);
        (lighting.sun, lighting.moon, lighting.fog_clip) = (sky, sky, clip);
        let ours = mg_area::fog_from(&lighting, night, distance);
        let what =
            format!("amount {amount}, clip {clip}, camera {distance} at {pitch}, night {night}");
        eprintln!(
            "{what}: client {start:.2}..{end:.2}, Moonglow {:.2}..{:.2}",
            ours.start, ours.end
        );
        if (start - ours.start).abs() > 0.1 || (end - ours.end).abs() > 0.1 {
            failures.push(format!(
                "{what}: client {start}..{end}, Moonglow {}..{}",
                ours.start, ours.end
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

fn shoot(case: &Case, setup: Setup, paint: Paint) -> Shot {
    let light =
        Lighting { ambient: SHARD_AMBIENT, diffuse: SHARD_DIFFUSE, main_light: 0, only_tile: None };
    let mut overrides = Vec::new();
    if paint != Paint::No {
        let base =
            GameData::open(&GameInstall::new(mg_testkit::nwn_root().unwrap(), None, "en")).unwrap();
        if paint == Paint::Uniforms {
            overrides.push(("inc_standard.shd", debug_shader(&base).into_bytes()));
        } else {
            let src = base.resman.get_named("inc_fog", ResType::SHD).expect("inc_fog.shd");
            let src = String::from_utf8_lossy(&src).replace("\r\n", "\n");
            let from = "ColorIO.rgb = mix(ColorIO.rgb, fogColor.rgb, GetFogAlpha(fFogFragCoord));";
            assert!(src.contains(from), "inc_fog's ApplyFog");
            let to = "ColorIO.rgb = vec3(clamp((fFogFragCoord + 5.0) / 10.0, 0.0, 1.0));";
            overrides.push(("inc_fog.shd", src.replace(from, to).into_bytes()));
        }
    }
    let name = match paint {
        Paint::No => format!("night_fog_{}", case.name),
        Paint::Fog => format!("night_fog_{}_paint", case.name),
        Paint::Uniforms => format!(
            "night_fog_uniforms_{}_{}_{}_{}{}",
            setup.amount,
            setup.clip,
            setup.camera.0,
            setup.camera.1,
            if case.night { "_n" } else { "" }
        ),
    };
    let mut r = Reading::new(&name, case.tileset, light, &overrides);
    for prefix in ["Sun", "Moon"] {
        r.are.root.set(&format!("{prefix}FogAmount"), Value::Byte(setup.amount));
        r.are.root.set(&format!("{prefix}FogColor"), Value::Dword(case.fog.1));
    }
    r.are.root.set("IsNight", Value::Byte(u8::from(case.night)));
    r.are.root.set("FogClipDist", Value::Float(setup.clip));
    let flags = r.are.root.integer("Flags").unwrap_or(0) as u32;
    let flags = if case.interior { flags | 1 } else { flags & !1 };
    r.are.root.set("Flags", Value::Dword(flags));
    if let Some(tile) = case.tile {
        for t in r.are.root.list_mut("Tile_List").unwrap() {
            t.set("Tile_ID", Value::Int(tile));
            t.set("Tile_Height", Value::Int(0));
            t.set("Tile_Orientation", Value::Int(0));
        }
    }
    for [x, y] in THINGS {
        r.place(0, [x, y, 0.0], 270.0, &[("Static", Value::Byte(0))]);
    }
    let mut info = r.m.info().unwrap();
    info.root.write(&ifo::MOD_ENTRY_X, PC[0]);
    info.root.write(&ifo::MOD_ENTRY_Y, setup.pc_y);
    info.root.write(&ifo::MOD_ENTRY_DIR_X, 0.0);
    info.root.write(&ifo::MOD_ENTRY_DIR_Y, 1.0);
    r.m.set_info(&info).unwrap();
    let settings = if case.player { common::player_settings(false) } else { SETTINGS.to_string() };
    let (are, git) = (r.are.clone(), r.git.clone());
    let game = GameData::open(&GameInstall::new(
        mg_testkit::nwn_root().unwrap(),
        Some(r.dir.join("user")),
        "en",
    ))
    .unwrap();
    let body = format!("{INVISIBLE}\n    SetCameraLimits(pc, 1.0, 89.0, 1.0, 200.0);");
    let scripted = (SCRIPTED.0, setup.camera.0, setup.camera.1);
    let (dir, client) = r.shoot(&enter(&logged(&body), scripted), &settings);
    if paint == Paint::Uniforms {
        // The start from 1 / (end - start), as `fog_uniforms_match_the_client`.
        assert!(debug_value(&client, 10, 1.0) > 0.5, "{name}: the client's fog is off");
        let end = debug_value(&client, 12, 1024.0);
        let start = end - 1.0 / debug_value(&client, 13, 1.0);
        return Shot { uniforms: Some((start, end)), means: Vec::new() };
    }
    crate::common::gpu_after_display();
    let Some(gpu) = Gpu::headless() else {
        return Shot { uniforms: None, means: Vec::new() };
    };
    let tileset = mg_area::tileset(&game, are.root.resref("Tileset").unwrap()).ok();
    let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let pc = player_stood(&dir, &game, &model, glam::Vec2::new(PC[0], setup.pc_y));
    let camera = camera_facing(pc, SCRIPTED.0, FIT_FOV, FITTED.2, FITTED.0, FITTED.1);
    // (The fog for the client's camera: its distance as the script gave it.)
    let v = mg_area::View {
        fog: true,
        night: case.night,
        camera: setup.camera.0,
        ..mg_area::View::of(&model)
    };
    let scene = area_scene.scene(&model, &v);
    let (w, h) = (client.width, client.height);
    if std::env::var_os("MG_FIT").is_some() {
        fit(&gpu, &game, &scene, &client, pc);
    }
    let (off, on) =
        off_and_on(&gpu, &game, &scene, &camera, (w, h), Some(&client), &dir, "moonglow");
    let fog = scene.fog.unwrap();
    eprintln!(
        "{}: Moonglow's fog {:.1}..{:.1}, colour {:.3} (gray {:.1})",
        case.name,
        fog.start,
        fog.end,
        fog.color,
        (fog.color.x + fog.color.y + fog.color.z) / 3.0 * 255.0
    );
    // The three side by side, for a look.
    let mut side = Rgba { width: w * 3, height: h, data: vec![0; (w * 3 * h * 4) as usize] };
    for (k, img) in [&client, &off, &on].into_iter().enumerate() {
        for y in 0..h as usize {
            let row = &img.data[y * w as usize * 4..(y + 1) * w as usize * 4];
            let at = (y * w as usize * 3 + k * w as usize) * 4;
            side.data[at..at + row.len()].copy_from_slice(row);
        }
    }
    save_png(&side, &dir.join("side_by_side.png"));
    let gray = |img: &Rgba, rect: [f32; 4]| {
        let m = mean(img, rect);
        (m[0] + m[1] + m[2]) / 3.0
    };
    let view_proj = camera.projection(w as f32 / h as f32) * camera.view();
    let depth = |p: Vec3| -(camera.view() * p.extend(1.0)).z;
    let pixel = |p: Vec3| {
        let c = view_proj * p.extend(1.0);
        let n = c.truncate() / c.w;
        ((n.x + 1.0) / 2.0 * w as f32, (1.0 - n.y) / 2.0 * h as f32)
    };
    // (Only what is clear of the game's interface: a tenth in from each
    // side to seven tenths down.)
    let clear = |px: f32, py: f32, half: f32| {
        px - half > 0.1 * w as f32
            && px + half < 0.9 * w as f32
            && py - half > 0.1 * h as f32
            && py + half < 0.7 * h as f32
    };
    let mut means = Vec::new();
    let mut region = |what: String, rect: [f32; 4]| {
        let m = (what, gray(&client, rect), gray(&off, rect), gray(&on, rect));
        eprintln!("{} {:28} client {:5.1} own {:5.1} game's {:5.1}", case.name, m.0, m.1, m.2, m.3);
        means.push(m);
    };
    let rect = |px: f32, py: f32, hx: f32, hy: f32| {
        [(px - hx) / w as f32, (py - hy) / h as f32, (px + hx) / w as f32, (py + hy) / h as f32]
    };
    // Each thing: a square about its middle, a metre up, 40 cm across.
    for [x, y] in THINGS {
        let p = Vec3::new(x, y, pc.z + 1.0);
        let d = depth(p);
        let (px, py) = pixel(p);
        let half = (0.2 * h as f32 / (2.0 * d * (FIT_FOV.to_radians() / 2.0).tan())).max(2.0);
        if d > 0.0 && clear(px, py, half) {
            region(format!("thing {d:.1} m"), rect(px, py, half, half));
        }
    }
    // The ground in the middle column, from near to past the clip.
    for ahead in [6.0, 10.0, 14.0, 17.0, 19.0, 21.0, 23.0, 26.0, 30.0] {
        let p = Vec3::new(pc.x, camera.eye.y + ahead, pc.z);
        let d = depth(p);
        let (px, py) = pixel(p);
        if d > 0.0 && clear(px, py, 2.0) {
            let f = ((d - fog.start) / (fog.end - fog.start)).clamp(0.0, 1.0);
            region(format!("ground {d:.1} m (fog {f:.2})"), rect(px, py, 20.0, 2.0));
        }
    }
    region("above the horizon".into(), [0.4, 0.12, 0.6, 0.18]);
    eprintln!("{}: pictures in {}", case.name, dir.display());
    Shot { uniforms: None, means }
}

/// Exploration (`MG_FIT=1`): the distance, pitch and focus of Moonglow's
/// camera looking north over `pc` that match the client's picture best
/// (gray levels, clear of the game's interface), printed.
fn fit(gpu: &Gpu, game: &GameData, scene: &Scene, client: &Rgba, pc: Vec3) {
    let mut r = Renderer::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let gray = |img: &Rgba| -> Vec<f32> {
        img.data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0)
            .collect()
    };
    let cg = gray(client);
    let (w, h) = (client.width as usize, client.height as usize);
    let mut best = (f32::MAX, [0.0; 3]);
    for distance in (18..=28).map(|d| d as f32 / 4.0) {
        for pitch in (160..=172).map(|p| p as f32 / 2.0) {
            for focus in [1.0f32, 1.25, 1.5, 1.75, 2.0] {
                let cam = camera_facing(pc, SCRIPTED.0, FIT_FOV, focus, distance, pitch);
                let g = gray(&r.render_image(
                    gpu,
                    &game.resman,
                    scene,
                    &cam,
                    client.width,
                    client.height,
                ));
                let (mut err, mut n) = (0.0, 0.0);
                for y in (h / 10..h * 7 / 10).step_by(4) {
                    for x in (w / 10..w * 9 / 10).step_by(4) {
                        err += (g[y * w + x] - cg[y * w + x]).abs();
                        n += 1.0;
                    }
                }
                if err / n < best.0 {
                    best = (err / n, [distance, pitch, focus]);
                }
            }
        }
    }
    eprintln!("best fit: error {:.2}: distance, pitch, focus {:?}", best.0, best.1);
}
