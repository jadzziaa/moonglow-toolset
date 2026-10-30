//! The deferred sprint's fourth client batch (2026-10-30): readings of the
//! client that answer the deferred list's questions, beside the first three
//! batches' (`readings.rs`, `batch2.rs`, `batch3.rs`, whose scaffolding
//! they use). Each writes the client's pictures to its scratch directory
//! (`target/test-output/client_reading_<name>/`) and prints where; what
//! was seen is in its comment.
//!
//! Needs the game and the off-screen display; run by hand, one client at a
//! time:
//! `DISPLAY=:2 cargo test -p mg-corpus-tests --release --test client_render batch4 -- --ignored --nocapture --test-threads 1`.

use super::readings::{INVISIBLE, Reading, enter, placeables_with, print_log};
use super::*;
use std::path::PathBuf;

/// A placeable's model `name` of a root dummy and `nodes` (ASCII nodes
/// whose parent is the root).
fn probe_model(name: &str, nodes: &[String]) -> String {
    format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification Character\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\n\
         endnode\n{}endmodelgeom {name}\ndonemodel {name}\n",
        nodes.concat()
    )
}

/// [`Reading::shoot`] from the camera `(facing, distance, pitch)` (given a
/// second after entry, which the client takes), with Moonglow's area view
/// beside it from about the camera the client shows for it (as
/// [`TURNED_DISTANCE`] and [`TURNED_PITCH`] were fitted for (f, 12, 45):
/// 0.92 of the distance, a degree less pitch; not fitted for these):
/// `moonglow.png`, `moonglow_on.png`.
fn shoot_close(
    r: Reading,
    body: &str,
    settings: &str,
    (facing, distance, pitch): (f32, f32, f32),
    view: &dyn Fn(&mut mg_area::View),
) -> (PathBuf, Rgba) {
    let (are, git) = (r.are.clone(), r.git.clone());
    let dir = r.dir.clone();
    let game = GameData::open(&GameInstall::new(
        mg_testkit::nwn_root().unwrap(),
        Some(dir.join("user")),
        "en",
    ))
    .unwrap();
    let (dir, client) =
        r.shoot(&enter(&super::readings::logged(body), (facing, distance, pitch)), settings);
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        return (dir, client);
    };
    let tileset = mg_area::tileset(&game, are.root.resref("Tileset").unwrap()).ok();
    let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let pc = player_stood(&dir, &game, &model, glam::Vec2::new(20.0, 20.0));
    let camera = camera_facing(pc, facing, FIT_FOV, FIT_FOCUS, distance * 0.92, pitch - 1.0);
    let mut v = mg_area::View { fog: false, ..mg_area::View::of(&model) };
    view(&mut v);
    let mut scene = area_scene.scene(&model, &v);
    if v.animate {
        let mut sims = HashMap::new();
        scene.particles = area_scene.particles(&model, &v, &mut sims, 1.0 / 60.0, camera.view());
    }
    off_and_on(
        &gpu,
        &game,
        &scene,
        &camera,
        (client.width, client.height),
        Some(&client),
        &dir,
        "moonglow",
    );
    (dir, client)
}

/// An emitter of [`probe_model`] `model` at (x, y, z): 12 cm white squares
/// rising 1 m/s for 3 s unless `extra` says otherwise (later keywords win).
fn emitter(model: &str, name: &str, at: [f32; 3], extra: &str) -> String {
    probe_emitter(name, at, extra).replace("parent plc_a01", &format!("parent {model}"))
}

/// The bounding box (x0, y0, x1, y1) of the pixels of `img` within the
/// rectangle `r` (fractions, top-left origin) that `keep` keeps, and how
/// many there are.
fn extent(img: &Rgba, r: [f32; 4], keep: &dyn Fn([u8; 3]) -> bool) -> ([u32; 4], usize) {
    let (w, h) = (img.width as f32, img.height as f32);
    let mut b = [u32::MAX, u32::MAX, 0, 0];
    let mut n = 0;
    for y in (r[1] * h) as u32..(r[3] * h) as u32 {
        for x in (r[0] * w) as u32..(r[2] * w) as u32 {
            let i = (y as usize * img.width as usize + x as usize) * 4;
            if keep([img.data[i], img.data[i + 1], img.data[i + 2]]) {
                b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
                n += 1;
            }
        }
    }
    (b, n)
}

/// How far the client stretches a `Motion_Blur` particle (the deferred
/// list's "Emitters start as they look once going": Magic Sparks are long
/// streaks in the client, dots in Moonglow): a dark tic01 room, seen from
/// the side (`SetCameraFacing(90, 9, 80)`, given a second after entry),
/// and north of the player a row of emitters 0.6 m apart, west to east,
/// each throwing a 12 cm white square up every so often (one at a time
/// in view): a `Normal` one (the square's size on the picture), then
/// `Motion_Blur` at velocity 1, 3 and 6 with `blurlength` 1, at 3 with
/// `blurlength` 0 and 3, and at 3 with 30 cm squares; and green dots, still,
/// 1 m apart up the last column (the picture's scale).
#[test]
#[ignore]
fn reading_motion_blur_length() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let m = "mg_blur";
    let column = |i: usize| -2.1 + 0.6 * i as f32;
    // (One particle every 1.5 m of its rise, gone after 3 m.)
    let thrown = |v: f32, extra: &str| {
        format!(
            "velocity {v}\n  birthrate {}\n  lifeExp {}\n  render Motion_Blur\n  {extra}",
            v / 1.5,
            3.0 / v
        )
    };
    let mut nodes = vec![
        emitter(m, "normal", [column(0), 0.0, 0.2], "birthrate 0.667\n  lifeExp 3"),
        emitter(m, "v1", [column(1), 0.0, 0.2], &thrown(1.0, "blurlength 1")),
        emitter(m, "v3", [column(2), 0.0, 0.2], &thrown(3.0, "blurlength 1")),
        emitter(m, "v6", [column(3), 0.0, 0.2], &thrown(6.0, "blurlength 1")),
        emitter(m, "b0", [column(4), 0.0, 0.2], &thrown(3.0, "blurlength 0")),
        emitter(m, "b3", [column(5), 0.0, 0.2], &thrown(3.0, "blurlength 3")),
        emitter(
            m,
            "big",
            [column(6), 0.0, 0.2],
            &thrown(3.0, "blurlength 1\n  sizeStart 0.3\n  sizeEnd 0.3"),
        ),
    ];
    for k in 0..3 {
        nodes.push(emitter(
            m,
            &format!("dot{k}"),
            [column(7), 0.0, 0.5 + k as f32],
            "velocity 0\n  birthrate 4\n  lifeExp 2\n  sizeStart 0.06\n  sizeEnd 0.06\n  \
             colorStart 0 1 0\n  colorEnd 0 1 0",
        ));
    }
    let (table, first) = placeables_with(&base, &[(m, 0)]);
    let overrides = [
        ("mg_blur.mdl", probe_model(m, &nodes).into_bytes()),
        ("mgwhite.tga", white_tga()),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x101010, diffuse: 0, main_light: 0, only_tile: None };
    let mut r = Reading::new("motion_blur_length", "tic01", light, &overrides);
    // (Facing south: the model's x runs west to east.)
    r.place(first, [20.0, 23.0, 0.0], 0.0, &[]);
    let (dir, client) = r.shoot(&enter(INVISIBLE, (90.0, 9.0, 80.0)), SETTINGS);
    print_log(&dir);
    // Each column's white pixels, and the green dots'.
    let bright = |p: [u8; 3]| p.iter().all(|&c| c > 128);
    let green = |p: [u8; 3]| p[1] > 128 && p[0] < 80 && p[2] < 80;
    let (dots, n) = extent(&client, [0.0, 0.0, 1.0, 0.85], &green);
    eprintln!("green dots: {dots:?} ({n} px)");
    let (_, whole) = extent(&client, [0.0, 0.0, 1.0, 0.85], &bright);
    eprintln!("white pixels: {whole}");
}

/// [`reading_motion_blur_length`] again, its columns told apart (each
/// there threw a particle every 1.5 m of its 3 m rise, which every streak
/// filled): `Motion_Blur` squares of 12 cm rising 2 m/s, west to east, at
/// birthrate 1, 2 and 4 (lifeExp 1.5); at 2 with lifeExp 0.75; at 2 with
/// `blurlength` 0.5 and 0.1 (else 1); at 2 thrown in a cone (`spread`
/// 0.6); a `Normal` emitter at birthrate 1.5 (the first reading's 0.667
/// showed nothing); and the green dots, 1 m apart.
#[test]
#[ignore]
fn reading_motion_blur_rule() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let m = "mg_blur2";
    let column = |i: usize| -2.0 + 0.5 * i as f32;
    let blur = |rate: f32, life: f32, extra: &str| {
        format!(
            "velocity 2\n  birthrate {rate}\n  lifeExp {life}\n  render Motion_Blur\n  \
             blurlength 1\n  {extra}"
        )
    };
    let mut nodes = vec![
        emitter(m, "r1", [column(0), 0.0, 0.2], &blur(1.0, 1.5, "")),
        emitter(m, "r2", [column(1), 0.0, 0.2], &blur(2.0, 1.5, "")),
        emitter(m, "r4", [column(2), 0.0, 0.2], &blur(4.0, 1.5, "")),
        emitter(m, "life", [column(3), 0.0, 0.2], &blur(2.0, 0.75, "")),
        emitter(m, "bl05", [column(4), 0.0, 0.2], &blur(2.0, 1.5, "blurlength 0.5")),
        emitter(m, "bl01", [column(5), 0.0, 0.2], &blur(2.0, 1.5, "blurlength 0.1")),
        emitter(m, "cone", [column(6), 0.0, 0.2], &blur(2.0, 1.5, "spread 0.6")),
        emitter(m, "normal", [column(7), 0.0, 0.2], "birthrate 1.5\n  lifeExp 3"),
    ];
    for k in 0..3 {
        nodes.push(emitter(
            m,
            &format!("dot{k}"),
            [column(8), 0.0, 0.5 + k as f32],
            "velocity 0\n  birthrate 4\n  lifeExp 2\n  sizeStart 0.06\n  sizeEnd 0.06\n  \
             colorStart 0 1 0\n  colorEnd 0 1 0",
        ));
    }
    let (table, first) = placeables_with(&base, &[(m, 0)]);
    let overrides = [
        ("mg_blur2.mdl", probe_model(m, &nodes).into_bytes()),
        ("mgwhite.tga", white_tga()),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x101010, diffuse: 0, main_light: 0, only_tile: None };
    let mut r = Reading::new("motion_blur_rule", "tic01", light, &overrides);
    r.place(first, [20.0, 23.0, 0.0], 0.0, &[]);
    let (dir, _) = shoot_close(r, INVISIBLE, SETTINGS, (90.0, 9.0, 80.0), &|v| v.animate = true);
    print_log(&dir);
}

/// [`reading_motion_blur_rule`] with one particle in a column at a time
/// (birthrate 1, each rising 3 m), so that no younger particle is near:
/// west to east velocity 1, 2 and 4 with `blurlength` 1; 2 with 0.25, 0.5
/// and 2; 1 with 0.5; and the green dots. Four pictures, about a second
/// apart (`client_<i>.png`).
#[test]
#[ignore]
fn reading_motion_blur_single() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let m = "mg_blur3";
    let column = |i: usize| -2.1 + 0.6 * i as f32;
    let blur = |v: f32, bl: f32| {
        format!(
            "velocity {v}\n  birthrate 1\n  lifeExp {}\n  render Motion_Blur\n  blurlength {bl}",
            3.0 / v
        )
    };
    let cases =
        [(1.0, 1.0), (2.0, 1.0), (4.0, 1.0), (2.0, 0.25), (2.0, 0.5), (2.0, 2.0), (1.0, 0.5)];
    let mut nodes: Vec<String> = cases
        .iter()
        .enumerate()
        .map(|(i, &(v, bl))| emitter(m, &format!("c{i}"), [column(i), 0.0, 0.2], &blur(v, bl)))
        .collect();
    for k in 0..3 {
        nodes.push(emitter(
            m,
            &format!("dot{k}"),
            [column(7), 0.0, 0.5 + k as f32],
            "velocity 0\n  birthrate 4\n  lifeExp 2\n  sizeStart 0.06\n  sizeEnd 0.06\n  \
             colorStart 0 1 0\n  colorEnd 0 1 0",
        ));
    }
    let (table, first) = placeables_with(&base, &[(m, 0)]);
    let overrides = [
        ("mg_blur3.mdl", probe_model(m, &nodes).into_bytes()),
        ("mgwhite.tga", white_tga()),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x101010, diffuse: 0, main_light: 0, only_tile: None };
    let mut r = Reading::new("motion_blur_single", "tic01", light, &overrides);
    r.place(first, [20.0, 23.0, 0.0], 0.0, &[]);
    r.save(&enter(INVISIBLE, (90.0, 9.0, 80.0)), SETTINGS);
    let shots = common::client_screenshots(&r.dir, "MgScene", 4, Duration::from_millis(700))
        .expect("the client's pictures");
    for (t, _) in &shots {
        eprintln!("picture at {t:.2} s");
    }
    print_log(&r.dir);
}

/// `ascii` (a model named `name`) compiled by nwnmdlcomp in `dir`.
fn nwnmdlcomp(dir: &Path, name: &str, ascii: &str) -> Vec<u8> {
    let tool = mg_testkit::nwn_tool("nwnmdlcomp").expect("nwnmdlcomp");
    let work = dir.join(format!("compile_{name}"));
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("model.mdl"), ascii).unwrap();
    let out = std::process::Command::new(tool)
        .args(["-c", "model.mdl"])
        .current_dir(&work)
        .env("NWNDIR", mg_testkit::nwn_root().unwrap())
        .output()
        .unwrap();
    std::fs::read(work.join("model.mdl.mdl"))
        .unwrap_or_else(|_| panic!("nwnmdlcomp: {}", String::from_utf8_lossy(&out.stdout)))
}

/// Particles' turn and birth in the client, from the reference camera
/// beside Moonglow's area view (Animations on): north of the player
/// (`mg_part`), west to east, 40 cm squares of the four-color texture
/// rising slowly, one a second, spinning very slowly (`particleRot`
/// 0.002: each keeps the angle it started at) and not spinning; a `Linked`
/// column of 60 cm squares of it and the same not linked (which way up a
/// ribbon's piece shows its picture); then two emitters of 12 cm squares
/// rising 1 m/s, red at `birthrate` 0.5 and blue at 1.5, in an ASCII model
/// (`mg_rate`, west) and the same compiled by nwnmdlcomp (`mg_ratec`,
/// east): whether a birthrate under 1, or its fraction, is kept.
#[test]
#[ignore]
fn reading_particle_turn_and_birth() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let quad = "texture mgquad\n  sizeStart 0.4\n  sizeEnd 0.4\n  birthrate 1\n  velocity 0.25\n  \
                lifeExp 10";
    let column = "birthrate 2\n  sizeStart 0.6\n  sizeEnd 0.6\n  texture mgquad";
    let part = probe_model(
        "mg_part",
        &[
            emitter("mg_part", "spin", [-2.4, 0.0, 0.3], &format!("{quad}\n  particleRot 0.002")),
            emitter("mg_part", "still", [-1.6, 0.0, 0.3], quad),
            emitter("mg_part", "linked", [-0.4, 0.0, 0.0], &format!("{column}\n  render Linked")),
            emitter("mg_part", "plain", [0.6, 0.0, 0.0], column),
        ],
    );
    let rate = |name: &str| {
        probe_model(
            name,
            &[
                emitter(
                    name,
                    "half",
                    [-0.3, 0.0, 0.2],
                    "birthrate 0.5\n  colorStart 1 0 0\n  \
                 colorEnd 1 0 0",
                ),
                emitter(
                    name,
                    "oneandahalf",
                    [0.3, 0.0, 0.2],
                    "birthrate 1.5\n  colorStart 0 0 1\n  \
                 colorEnd 0 0 1",
                ),
            ],
        )
    };
    let dir = scratch_dir("client_reading_particle_turn_and_birth");
    let compiled = nwnmdlcomp(&dir, "mg_ratec", &rate("mg_ratec"));
    let (table, first) = placeables_with(&base, &[("mg_part", 0), ("mg_rate", 0), ("mg_ratec", 0)]);
    let overrides = [
        ("mg_part.mdl", part.into_bytes()),
        ("mg_rate.mdl", rate("mg_rate").into_bytes()),
        ("mg_ratec.mdl", compiled),
        ("mgquad.tga", quadrant_tga(0)),
        ("mgwhite.tga", white_tga()),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x202020, diffuse: 0, main_light: 0, only_tile: None };
    let mut r = Reading::new("particle_turn_and_birth", "tic01", light, &overrides);
    r.place(first, [19.5, 24.0, 0.0], 0.0, &[]);
    r.place(first + 1, [22.0, 24.0, 0.0], 0.0, &[]);
    r.place(first + 2, [23.5, 24.0, 0.0], 0.0, &[]);
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|v| v.animate = true);
    print_log(&dir);
}

/// [`reading_particle_turn_and_birth`]'s questions taken further, from the
/// reference camera beside Moonglow's area view: north of the player,
/// west to east, 40 cm squares of the four-color texture spinning at
/// `particleRot` 0.05 and 0.25 (one a second, rising slowly: which angle
/// each starts at, and how far a second turns it); 40 cm squares falling
/// from 3 m (the emitter turned upside down: the oldest lowest, farthest
/// from the camera above; which of two overlapping is drawn over the
/// other); a `Linked` column of 30 cm squares a metre apart (a piece's
/// length); and red 12 cm squares at `birthrate` 0.5 rising 0.1 m/s for
/// 20 s (whether any is born in the scene's seconds).
#[test]
#[ignore]
fn reading_particle_spin_and_order() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let m = "mg_spin";
    let quad = "texture mgquad\n  sizeStart 0.4\n  sizeEnd 0.4\n  birthrate 1\n  velocity 0.25\n  \
                lifeExp 10";
    let model = probe_model(
        m,
        &[
            emitter(m, "spin5", [-2.4, 0.0, 0.3], &format!("{quad}\n  particleRot 0.05")),
            emitter(m, "spin25", [-1.6, 0.0, 0.3], &format!("{quad}\n  particleRot 0.25")),
            emitter(
                m,
                "fall",
                [-0.6, 0.0, 3.2],
                "texture mgquad\n  sizeStart 0.4\n  sizeEnd 0.4\n  birthrate 1\n  velocity 0.25\n  \
                 lifeExp 10\n  orientation 1 0 0 3.1415927",
            ),
            emitter(
                m,
                "linked",
                [0.6, 0.0, 0.0],
                "texture mgquad\n  sizeStart 0.3\n  sizeEnd 0.3\n  render Linked",
            ),
            emitter(
                m,
                "slow",
                [1.6, 0.0, 0.2],
                "birthrate 0.5\n  velocity 0.1\n  lifeExp 20\n  colorStart 1 0 0\n  colorEnd 1 0 0",
            ),
        ],
    );
    let (table, first) = placeables_with(&base, &[(m, 0)]);
    let overrides = [
        ("mg_spin.mdl", model.into_bytes()),
        ("mgquad.tga", quadrant_tga(0)),
        ("mgwhite.tga", white_tga()),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x202020, diffuse: 0, main_light: 0, only_tile: None };
    let mut r = Reading::new("particle_spin_and_order", "tic01", light, &overrides);
    r.place(first, [20.0, 24.0, 0.0], 0.0, &[]);
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|v| v.animate = true);
    print_log(&dir);
}

/// What the client gives a lightning and a `Linked` emitter of
/// `particleSoftness` (the M5 item "Soft particles"), read as
/// `soft_particles_in_the_client` reads the others: the game's
/// `fsparticle.shd` patched to paint, over each particle, the softness it
/// was given and the emitter's number (its color's red × 16). North of
/// the player, 50 cm particles: 1 a `Linked` column, 2 a lightning bolt
/// (`update Lightning`, `sizeStart` 0.5) to a reference node 2 m east, 3 a
/// plain one (the client's 10 / 0.5 = 20).
#[test]
#[ignore]
fn reading_soft_particles_of_lightning_and_linked() {
    use super::soft_particles::{patched_fsparticle, read_marks};
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let m = "mg_softl";
    let sized = |i: u32| {
        let red = i as f32 / 16.0;
        format!("sizeStart 0.5\n  sizeEnd 0.5\n  colorStart {red} 1 1\n  colorEnd {red} 1 1")
    };
    let bolt = format!(
        "{}\n  update Lightning\n  birthrate 4\n  lifeExp 1\n  velocity 0\n  lightningDelay 0.2\n  \
         lightningRadius 0\n  lightningSubDiv 3\n  lightningScale 1",
        sized(2)
    );
    let target = format!(
        "node reference bolt_end\n  parent bolt\n  position 2 0 0\n  orientation 0 0 0 0\n  \
         refModel fx_ref\n  reattachable 0\nendnode\n"
    );
    let model = probe_model(
        m,
        &[
            emitter(m, "linked", [-2.0, 0.0, 0.0], &format!("{}\n  render Linked", sized(1))),
            emitter(m, "bolt", [-0.5, 0.0, 1.5], &bolt),
            target,
            emitter(m, "plain", [2.0, 0.0, 0.0], &sized(3)),
        ],
    );
    let (table, first) = placeables_with(&base, &[(m, 0)]);
    let overrides = [
        ("mg_softl.mdl", model.into_bytes()),
        ("mgwhite.tga", white_tga()),
        ("fsparticle.shd", patched_fsparticle(&base).into_bytes()),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("soft_particles_of_lightning_and_linked", "tic01", light, &overrides);
    r.place(first, [20.0, 23.0, 0.0], 0.0, &[]);
    let (dir, client) = r.shoot(&enter(INVISIBLE, (90.0, 12.0, 45.0)), SETTINGS);
    for (i, (n, values, flag)) in read_marks(&client) {
        let shown: Vec<String> =
            values.iter().take(3).map(|(v, c)| format!("{v:.3} ({c} px)")).collect();
        eprintln!("emitter {i}: {n} px, flag {flag}, particleSoftness {}", shown.join(", "));
    }
    print_log(&dir);
}

/// A `VisTransformList` entry of `scope`: scaled by `scale`, turned by
/// `rotate` (degrees about x, y, z) and moved by `translate` (metres), as
/// Aurora's Adjust Location writes one.
fn transform(scope: i32, scale: f32, rotate: [f32; 3], translate: [f32; 3]) -> mg_gff::Struct {
    let mut e = super::batch2::transform_entry(scope, scale, 0.0);
    for (prefix, v) in [("Rotate", rotate), ("Translate", translate)] {
        for (i, a) in ["X", "Y", "Z"].iter().enumerate() {
            let mut s = mg_gff::Struct::new(0);
            s.set("TimerType", Value::Int(0));
            s.set("ValueTo", Value::Float(v[i]));
            s.set("LerpType", Value::Int(0));
            e.set(&format!("{prefix}{a}"), Value::Struct(s));
        }
    }
    e
}

/// A creature of the blueprint `resref` placed at `at` facing `facing`
/// (degrees), standing still (no scripts), with `fields` set and wearing,
/// over what its blueprint gives it, the items `equip` (slot bit, item
/// blueprint).
fn put_creature(
    r: &mut Reading,
    resref: &str,
    at: [f32; 3],
    facing: f32,
    fields: &[(&str, Value)],
    equip: &[(u32, &str)],
) {
    use mg_module::instances::{Placement, Placing, instance};
    let mut bp = Gff::read(&r.game.resman.get_named(resref, ResType::UTC).unwrap()).unwrap();
    let mut list = bp.root.list("Equip_ItemList").map(<[_]>::to_vec).unwrap_or_default();
    for &(slot, item) in equip {
        list.retain(|e| e.id != slot);
        let mut e = mg_gff::Struct::new(slot);
        e.set("EquippedRes", Value::resref(ResRef::from_str(item).unwrap()));
        list.push(e);
    }
    bp.root.set("Equip_ItemList", Value::List(list));
    let game = &r.game;
    let item = |res: ResRef| {
        game.resman
            .get(&ResKey::new(res, ResType::UTI))
            .ok()
            .and_then(|d| Gff::read(&d).ok())
            .map(|g| g.root)
    };
    let placing = Placing { game, item: &item };
    let at = Placement { position: at, rotation: facing.to_radians() };
    let mut c = instance(&placing, ResType::UTC, &bp.root, at, &[]).unwrap();
    let scripts: Vec<String> = c
        .fields
        .iter()
        .map(|f| f.label.to_string_lossy())
        .filter(|l| l.starts_with("Script"))
        .collect();
    for label in scripts {
        c.set(&label, Value::resref(ResRef::EMPTY));
    }
    for (label, v) in fields {
        c.set(label, v.clone());
    }
    let mut creatures = r.git.root.list("Creature List").map(<[_]>::to_vec).unwrap_or_default();
    creatures.push(c);
    r.git.root.set("Creature List", Value::List(creatures));
}

/// How the client draws a part's visual transform (the deferred list's
/// "Lerps, animation speed and other scopes, how the client draws a
/// part's"): north of the player, facing the camera, west to east, a
/// commoner made a human (appearance 6) with the demon's wings
/// (wingmodel 2) and a cloak (`nw_maarcl055`), without a transform, then
/// with one entry of the scope `MG_B4_SCOPE` (254 the head, by default;
/// 252 the wings, 243 the cloak): scaled 2; turned 45° about x, about y,
/// about z; moved 0.3 m along x and z. For the head, a bandit (appearance
/// 269, one model) with its head scaled 2 at the east end. In the client
/// beside Moonglow's area view.
#[test]
#[ignore]
fn reading_part_scopes() {
    let scope: i32 = std::env::var("MG_B4_SCOPE").ok().and_then(|s| s.parse().ok()).unwrap_or(254);
    // (`MG_B4_FACING`: the creatures' facing, degrees; 180 by default.)
    let facing: f32 =
        std::env::var("MG_B4_FACING").ok().and_then(|s| s.parse().ok()).unwrap_or(180.0);
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new(&format!("part_scopes_{scope}"), "tic01", light, &[]);
    let lists: [Option<mg_gff::Struct>; 6] = [
        None,
        Some(transform(scope, 2.0, [0.0; 3], [0.0; 3])),
        Some(transform(scope, 1.0, [45.0, 0.0, 0.0], [0.0; 3])),
        Some(transform(scope, 1.0, [0.0, 45.0, 0.0], [0.0; 3])),
        Some(transform(scope, 1.0, [0.0, 0.0, 45.0], [0.0; 3])),
        Some(transform(scope, 1.0, [0.0; 3], [0.3, 0.0, 0.3])),
    ];
    for (i, list) in lists.into_iter().enumerate() {
        let mut fields = vec![
            ("Appearance_Type", Value::Word(6)),
            ("Wings_New", Value::Dword(2)),
            ("Appearance_Head", Value::Byte(1)),
        ];
        if let Some(e) = list {
            fields.push(("VisTransformList", Value::List(vec![e])));
        }
        let x = 16.9 + 1.0 * i as f32;
        let cloak: &[(u32, &str)] = if scope == 243 { &[(64, "nw_maarcl055")] } else { &[] };
        put_creature(&mut r, "nw_commale", [x, 21.6, 0.0], facing, &fields, cloak);
    }
    if scope == 254 {
        let e = transform(scope, 2.0, [0.0; 3], [0.0; 3]);
        put_creature(
            &mut r,
            "nw_bandit001",
            [22.95, 21.6, 0.0],
            facing,
            &[("VisTransformList", Value::List(vec![e]))],
            &[],
        );
    }
    let (dir, _) = shoot_close(r, INVISIBLE, SETTINGS, (90.0, 5.0, 75.0), &|_| {});
    print_log(&dir);
}

/// Which of an item's models each of its part scopes transforms (the same
/// entry's "An item's part scopes (255 to 251)"): longswords
/// (`nw_wswls001`, three models: blade, hilt, pommel) lying north of the
/// player, west to east, without a transform, then with one entry of scope
/// 255, 254, 253, 252 and 251, each scaled 2. In the client beside
/// Moonglow's area view, from close above.
#[test]
#[ignore]
fn reading_item_part_scopes() {
    use mg_module::instances::{Placement, Placing, instance};
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("item_part_scopes", "tic01", light, &[]);
    let bp = Gff::read(&r.game.resman.get_named("nw_wswls001", ResType::UTI).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &r.game, item: &none };
    let mut placed = Vec::new();
    for (i, scope) in
        [None, Some(255), Some(254), Some(253), Some(252), Some(251)].into_iter().enumerate()
    {
        let at = Placement { position: [17.5 + 1.0 * i as f32, 22.0, 0.0], rotation: 0.0 };
        let mut item = instance(&placing, ResType::UTI, &bp.root, at, &[]).unwrap();
        if let Some(s) = scope {
            item.set("VisTransformList", Value::List(vec![transform(s, 2.0, [0.0; 3], [0.0; 3])]));
        }
        placed.push(item);
    }
    r.git.root.set(mg_area::ObjectKind::Item.list(), Value::List(placed));
    let (dir, _) = shoot_close(r, INVISIBLE, SETTINGS, (90.0, 5.0, 40.0), &|_| {});
    print_log(&dir);
}

/// The Domains of Dread's haks (read only): `MOONGLOW_TDOD_HAKS`, else
/// `~/Projects/tdod-module/build/hak`.
fn tdod_haks() -> Option<PathBuf> {
    let dir = std::env::var_os("MOONGLOW_TDOD_HAKS").map(PathBuf::from).or_else(|| {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Projects/tdod-module/build/hak"))
    })?;
    dir.is_dir().then_some(dir)
}

/// The files `names` (`name.ext`) out of the hak `hak` of [`tdod_haks`].
fn from_hak(hak: &str, names: &[&str]) -> Vec<(String, Vec<u8>)> {
    let data = std::fs::read(tdod_haks().expect("the Domains of Dread's haks").join(hak)).unwrap();
    let erf = mg_erf::Erf::read(&data).unwrap();
    names
        .iter()
        .map(|n| {
            let (stem, ext) = n.rsplit_once('.').unwrap();
            let ty = ResType::from_extension(ext).unwrap();
            let bytes = erf.get(&ResRef::from_str(stem).unwrap(), ty).unwrap().unwrap();
            (n.to_string(), bytes.into_owned())
        })
        .collect()
}

/// The rippling glow on the Domains of Dread's burning zombie (the
/// deferred list's "A rippling glow (TXI `proceduretype arturo`)"):
/// `dd_ce3_zombieash` (appearance 28518 of `tdod_top`'s appearance.2da,
/// its model and textures out of `tdod_p_ce3`, in the scratch override),
/// two of them north of the player facing the camera, from close, in the
/// client beside Moonglow's area view; `MG_B4_ZOMBIE` picks the case:
/// `as_is` (default; three pictures a second apart besides,
/// `client_<i>.png`), `plain` (its glow's TXI without `proceduretype` and
/// `distort`: still dim, the blending; bright, the ripple).
#[test]
#[ignore]
fn reading_rippling_glow() {
    let case = std::env::var("MG_B4_ZOMBIE").unwrap_or_else(|_| "as_is".into());
    let mut files = from_hak(
        "tdod_p_ce3.hak",
        &[
            "dd_ce3_zombieash.mdl",
            "dd_ce3_fxpafire2.tga",
            "dd_ce3_txburneds.dds",
            "dd_ce3_txhotcoal.dds",
            "dd_ce3_txhotcoal.txi",
            "dd_ce3_txmoltmet.dds",
            "dd_ce3_txmoltmet.txi",
        ],
    );
    files.extend(from_hak("tdod_top.hak", &["appearance.2da"]));
    if case != "dds" {
        // (The hak has both; `dds` leaves the TGAs out.)
        files.extend(from_hak("tdod_p_ce3.hak", &["dd_ce3_txburneds.tga", "dd_ce3_txhotcoal.tga"]));
    }
    if case == "plain" {
        for (name, data) in files.iter_mut() {
            if name == "dd_ce3_txhotcoal.txi" {
                *data = b"blending additive\n".to_vec();
            }
        }
    }
    let overrides: Vec<(&str, Vec<u8>)> =
        files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let light = Lighting { ambient: 0x606060, diffuse: 0xA0A0A0, main_light: 0, only_tile: None };
    let mut r = Reading::new(&format!("rippling_glow_{case}"), "tic01", light, &overrides);
    for x in [19.2, 20.8] {
        put_creature(
            &mut r,
            "nw_commale",
            [x, 21.8, 0.0],
            180.0,
            &[("Appearance_Type", Value::Word(28518))],
            &[],
        );
    }
    let dir = r.dir.clone();
    let (dir, _) = shoot_close(r, INVISIBLE, SETTINGS, (90.0, 4.0, 75.0), &|_| {});
    print_log(&dir);
    if case == "as_is" {
        let shots = common::client_screenshots(&dir, "MgScene", 3, Duration::from_millis(1000));
        eprintln!("{} more pictures", shots.map_or(0, |s| s.len()));
    }
}

/// Whether the client plays a static placeable's `default` animation (the
/// deferred list's "Lights placed on the fly, the flicker"): a dark tic01
/// room, north of the player a placeable whose model (`mg_flick`, a small
/// card and a light node of radius 4) has a `default` animation of 2 s
/// keying the light's color red, blue, red and the card's
/// self-illumination the same; west a static one, east a dynamic one.
/// Five pictures half a second apart (`client_<i>.png`): whether the floor
/// and the card under each change color.
#[test]
#[ignore]
fn reading_static_placeable_default_animation() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let name = "mg_flick";
    let mut lamp = NodeLight::white(4.0);
    lamp.color = [1.0, 0.0, 0.0];
    lamp.at = [0.0, 0.0, 1.0];
    let anim = format!(
        "newanim default {name}\n  length 2\n  transtime 0\n  animroot {name}\n\
         node dummy {name}\n  parent NULL\nendnode\n\
         node trimesh box\n  parent {name}\n  selfillumcolorkey 3\n    0 1 0 0\n    1 0 0 1\n    \
         2 1 0 0\n  positionkey 3\n    0 0 0 0\n    1 0 0 1\n    2 0 0 0\nendnode\n\
         node light mglight0\n  parent {name}\n  colorkey 3\n    0 1 0 0\n    1 0 0 1\n    \
         2 1 0 0\nendnode\n\
         doneanim default {name}\n"
    );
    let model = light_model(name, &[lamp])
        .replace("classification tile", "classification Character")
        .replace(&format!("donemodel {name}\n"), &format!("{anim}donemodel {name}\n"));
    let (table, first) = placeables_with(&base, &[(name, 1)]);
    let overrides = [("mg_flick.mdl", model.into_bytes()), ("placeables.2da", table.into_bytes())];
    let light = Lighting { ambient: 0x202020, diffuse: 0, main_light: 0, only_tile: None };
    let mut r = Reading::new("static_default_animation", "tic01", light, &overrides);
    r.place(first, [17.5, 23.0, 0.0], 0.0, &[("Static", Value::Byte(1))]);
    r.place(first, [22.5, 23.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    // (First Moonglow's area view beside one picture, a second into the
    // animation, where the dynamic one's keys are blue: `moonglow.png`.)
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|v| {
        v.animate = true;
        v.time = 1.0;
    });
    let shots = common::client_screenshots(&dir, "MgScene", 5, Duration::from_millis(500))
        .expect("the client's pictures");
    for (i, (t, img)) in shots.iter().enumerate() {
        let west = mean(img, [0.25, 0.42, 0.4, 0.55]);
        let east = mean(img, [0.6, 0.42, 0.75, 0.55]);
        eprintln!("picture {i} at {t:.2} s: floor west {west:?}, east {east:?}");
    }
    print_log(&dir);
}

/// A sphere a metre across, 1.2 m up, of `texture`, as an ASCII model
/// `name` of `classification` (its texture's alpha 0: reflecting wholly).
fn sphere_model(name: &str, classification: &str, texture: &str, centre: f32) -> String {
    let (rings, segments) = (16usize, 24usize);
    let (mut verts, mut tverts, mut faces) = (String::new(), String::new(), String::new());
    for i in 0..=rings {
        let v = i as f32 / rings as f32;
        let theta = v * std::f32::consts::PI;
        for j in 0..=segments {
            let u = j as f32 / segments as f32;
            let phi = u * std::f32::consts::TAU;
            let (x, y, z) = (theta.sin() * phi.cos(), theta.sin() * phi.sin(), theta.cos());
            verts += &format!("    {} {} {}\n", x * 0.5, y * 0.5, centre + z * 0.5);
            tverts += &format!("    {u} {} 0\n", 1.0 - v);
        }
    }
    let mut n = 0;
    for i in 0..rings {
        for j in 0..segments {
            let a = i * (segments + 1) + j;
            let (b, c, d) = (a + 1, a + segments + 1, a + segments + 2);
            faces += &format!("    {a} {c} {b} 1 {a} {c} {b} 0\n    {b} {c} {d} 1 {b} {c} {d} 0\n");
            n += 2;
        }
    }
    let count = (rings + 1) * (segments + 1);
    format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification {classification}\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
         node trimesh ball\n  parent {name}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {texture}\n  \
         render 1\n  shadow 0\n  verts {count}\n{verts}  tverts {count}\n{tverts}  faces {n}\n{faces}\
         endnode\nendmodelgeom {name}\ndonemodel {name}\n"
    )
}

/// A 16×16 gray TGA of alpha `alpha`.
fn gray_alpha_tga(alpha: u8) -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 16, 0, 32, 8];
    for _ in 0..256 {
        t.extend([128, 128, 128, alpha]);
    }
    t
}

/// The game's `inc_standard.shd` painting every environment-mapped
/// fragment with the map bound to it (`texUnitEnv`), laid out by its
/// texture coordinates, as `area_env_map_look`'s `MG_ENV_PAINT` does.
fn env_paint_shader(game: &GameData) -> String {
    let src = game.resman.get_named("inc_standard", ResType::SHD).unwrap();
    let from = "\tApplyDebugModeOutput(FragmentColor);\n}";
    let src = String::from_utf8_lossy(&src).replace("\r\n", "\n");
    assert!(src.contains(from), "inc_standard's end");
    src.replace(
        from,
        "\tApplyDebugModeOutput(FragmentColor);\n#if SHADER_TYPE == 2 && ENVIRONMENT_MAP == 1\n\
         \tFragmentColor = vec4(texture(texUnitEnv, vTexCoords.xy).rgb, 1.0);\n#endif\n}",
    )
}

/// Which environment map an item lying on the ground reflects (the M4
/// item "Not read in the client: which map an item on the ground
/// reflects"): a rural interior (`tin01`, whose map `tin01__ref01` is a
/// copper brown) with `inc_standard` painting each environment-mapped
/// fragment with its map ([`env_paint_shader`]); north of the player, west,
/// a longsword whose blade (`ModelPart3` 99: `w_swls_t_099`, made a sphere
/// of a gray texture of alpha 0) reflects by baseitems.2da's
/// `Part3EnvMap` 1; east, the big mirror (`Reflection` `default`, the
/// game's `chrome1`) made the same sphere. In the client beside Moonglow's
/// area view, from close.
#[test]
#[ignore]
fn reading_item_env_map() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let overrides = [
        ("w_swls_t_011.mdl", sphere_model("w_swls_t_011", "item", "mgshiny", 0.5).into_bytes()),
        ("px2_b02.mdl", sphere_model("px2_b02", "character", "mgshiny", 1.2).into_bytes()),
        ("mgshiny.tga", gray_alpha_tga(0)),
        ("inc_standard.shd", env_paint_shader(&base).into_bytes()),
    ];
    let light = Lighting { ambient: 0x808080, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("item_env_map", "tin01", light, &overrides);
    {
        use mg_module::instances::{Placement, Placing, instance};
        let bp = Gff::read(&r.game.resman.get_named("nw_wswls001", ResType::UTI).unwrap()).unwrap();
        let none = |_: ResRef| None;
        let placing = Placing { game: &r.game, item: &none };
        let at = Placement { position: [19.0, 22.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTI, &bp.root, at, &[]).unwrap();
        r.git.root.set(mg_area::ObjectKind::Item.list(), Value::List(vec![item]));
    }
    r.place(428, [21.5, 22.5, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = shoot_close(r, INVISIBLE, SETTINGS, (90.0, 5.0, 60.0), &|_| {});
    print_log(&dir);
}

/// What the client draws for a helmet whose model isn't there (the
/// deferred list's "Dynamic body parts from custom content": the report's
/// werejackal wears `helm_129`, in neither download): north of the
/// player, facing the camera, west to east, humans (appearance 6, head 1)
/// in a helmet (`nw_arhe001`) of `ModelPart1` 1 (`helm_001`), of 250
/// (no `helm_250` anywhere), and without one. In the client beside
/// Moonglow's area view, from close.
#[test]
#[ignore]
fn reading_missing_helmet_model() {
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("missing_helmet_model", "tic01", light, &[]);
    let body = [("Appearance_Type", Value::Word(6)), ("Appearance_Head", Value::Byte(1))];
    for (x, helmet) in [(18.5, Some(1u8)), (19.5, Some(2)), (20.5, Some(250)), (21.5, None)] {
        let equip: &[(u32, &str)] = if helmet.is_some() { &[(1, "nw_arhe001")] } else { &[] };
        put_creature(&mut r, "nw_commale", [x, 21.6, 0.0], 180.0, &body, equip);
        if let Some(part) = helmet {
            let list = r.git.root.list_mut("Creature List").unwrap();
            let c = list.last_mut().unwrap();
            let mut items = c.list("Equip_ItemList").map(<[_]>::to_vec).unwrap_or_default();
            for item in items.iter_mut().filter(|i| i.id == 1) {
                item.set("ModelPart1", Value::Byte(part));
                // (And its EE twin, which the game reads first.)
                item.set("xModelPart1", Value::Word(u16::from(part)));
                // (`MG_B4_NO_TEMPLATE`: the item names no blueprint there is, so
                // that only its own fields can give its look.)
                if std::env::var_os("MG_B4_NO_TEMPLATE").is_some() {
                    item.set(
                        "TemplateResRef",
                        Value::resref(ResRef::from_str("mg_nohelm").unwrap()),
                    );
                }
            }
            c.set("Equip_ItemList", Value::List(items));
        }
    }
    let (dir, _) = shoot_close(r, INVISIBLE, SETTINGS, (90.0, 3.5, 75.0), &|_| {});
    print_log(&dir);
}

/// Whether a static placeable's emitters run in the client, as its
/// `default` animation does not ([`reading_static_placeable_default_animation`]):
/// two braziers (placeables.2da 57, `plc_i05`, whose fire is an emitter)
/// north of the player, the west one static, the east one not, from the
/// reference camera beside Moonglow's area view (Animations on).
#[test]
#[ignore]
fn reading_static_placeable_emitters() {
    let light = Lighting { ambient: 0x404040, diffuse: 0x606060, main_light: 0, only_tile: None };
    let mut r = Reading::new("static_placeable_emitters", "tic01", light, &[]);
    r.place(57, [18.0, 23.0, 0.0], 0.0, &[("Static", Value::Byte(1))]);
    r.place(57, [22.0, 23.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|v| v.animate = true);
    print_log(&dir);
}
