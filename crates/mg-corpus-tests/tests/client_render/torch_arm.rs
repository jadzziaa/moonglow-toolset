//! How the client holds a torch (a report on the previews: "raises its
//! whole arm above its head; the client bends only the forearm").
//!
//! A bandit (`nw_bandit001`: appearance 269, `c_ynpc_h_m03`, whose
//! skeleton's supermodels are `a_ba` and `a_ba_non_combat`) holding the
//! game's torch (`nw_it_torch001`) in his left hand, 1.6 m north of the
//! player, close (`SetCameraFacing(90, 4, 75)`), facing the camera, a
//! quarter turned each way and away from it, beside Moonglow's area view
//! of the same from the same camera (`<run>.png`, `<run>_moonglow.png`,
//! `<run>_moonglow_on.png` in `target/test-output/torch_arm/`;
//! `MG_CLIENT_PICTURES=target/test-output` draws Moonglow's side again
//! beside the client's pictures kept there, without the client).
//!
//! Needs the game and the off-screen display; run by hand, one client at a
//! time:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render torch_arm -- --ignored --nocapture --test-threads 1`
//! (`MG_RUNS=front,steep` for some).
//!
//! Seen (2026-11-26, EE 89.8193.37):
//! - The client raises the whole arm: `torchl` (`a_ba_non_combat`,
//!   `animroot lbicep_g`) keys the orientations of `lbicep_g`,
//!   `lforearm_g`, `lforearm`, `lhand_g` and `lhand`, and each replaces
//!   what the idle keys there, the upper arm's too. The upper arm points
//!   ahead, about level, and the forearm up: from the side the torch is a
//!   forearm's length ahead of the face. The rest of the body idles on.
//! - Looked steeply down on from ahead (`steep`: `SetCameraFacing(90, 10,
//!   40)`, about the report's camera) the arm ahead is drawn below
//!   the head, which reads as an upper arm hanging and a forearm bent: it
//!   is the same pose.
//! - `pause` (`PlayAnimation(ANIMATION_LOOPING_PAUSE)` forced): as
//!   without, so what he idles in is no part of it.
//! - Moonglow's area view holds the torch where the client does, in each
//!   of the four ([`torch_arm_held_as_the_client_holds_it`]).

use super::batch4::{put_creature, shoot_close};
use super::readings::{INVISIBLE, Reading, print_log};
use super::*;

/// The camera of the close runs, and of the report's picture.
const CLOSE: (f32, f32, f32) = (90.0, 4.0, 75.0);
const STEEP: (f32, f32, f32) = (90.0, 10.0, 40.0);

/// Where the bandit stands.
const AT: [f32; 3] = [20.0, 21.6, 0.0];

/// The stick's length: the marker in place of the torch's model, from the
/// hand's node along its y axis, as the torch's model lies.
const STICK: f32 = 0.5;

/// An 8 by 8 TGA of one color.
fn flat_tga(rgb: [u8; 3]) -> Vec<u8> {
    let mut tga = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 24, 0];
    tga.extend([rgb[2], rgb[1], rgb[0]].repeat(64));
    tga
}

/// The torch's model (`it_torch_000`) as a magenta stick from the hand's
/// node 0.5 m along its y axis (the torch's own length), with no flame: what is found in a picture.
fn stick() -> Vec<(&'static str, Vec<u8>)> {
    let m = "it_torch_000";
    let node = super::part_textures::box_node(
        m,
        "stick",
        "mg_sticktex",
        [-0.03, 0.0, -0.03],
        [0.03, STICK, 0.03],
    );
    vec![
        ("mg_sticktex.tga", flat_tga([255, 0, 255])),
        ("it_torch_000.mdl", super::part_emitters::model_with(m, &[node], &[]).into_bytes()),
    ]
}

/// One run: the bandit facing `facing` with the torch, seen from `camera`;
/// `marked`: the torch a magenta stick; `pause`: his pause animation
/// asked for by script. The scratch folder and the client's picture.
fn shoot(
    run: &str,
    facing: f32,
    camera: (f32, f32, f32),
    marked: bool,
    pause: bool,
) -> (PathBuf, Rgba) {
    let lit = Lighting { ambient: 0x808080, diffuse: 0xa0a0a0, main_light: 0, only_tile: None };
    let files = if marked { stick() } else { Vec::new() };
    let mut r = Reading::new(&format!("torch_arm_{run}"), "tic01", lit, &files);
    put_creature(&mut r, "nw_bandit001", AT, facing, &[], &[(0x20, "nw_it_torch001")]);
    let body = if pause {
        format!(
            "{INVISIBLE}\n    object bandit = GetObjectByTag(\"NW_BANDIT001\");\n    \
             Log(\"MG_BANDIT \" + IntToString(GetIsObjectValid(bandit)));\n    \
             AssignCommand(bandit, PlayAnimation(ANIMATION_LOOPING_PAUSE, 1.0, 600.0));"
        )
    } else {
        INVISIBLE.into()
    };
    let (dir, shot) = shoot_close(r, &body, SETTINGS, camera, &|_| {});
    print_log(&dir);
    if pause {
        let log = std::fs::read_to_string(dir.join("user/logs/nwclientLog1.txt")).unwrap();
        assert!(log.contains("MG_BANDIT 1"), "{run}: the script did not find the bandit");
    }
    let kept = mg_testkit::test_output().join("torch_arm");
    std::fs::create_dir_all(&kept).unwrap();
    save_png(&shot, &kept.join(format!("{run}.png")));
    for side in ["moonglow", "moonglow_on"] {
        let _ =
            std::fs::copy(dir.join(format!("{side}.png")), kept.join(format!("{run}_{side}.png")));
    }
    (dir, shot)
}

/// What was seen: the module's header. The bandit with the game's torch
/// from ahead, from each side and from behind, from the report's camera,
/// and with his pause animation forced.
#[test]
#[ignore]
fn torch_arm_look() {
    let _ = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let only = std::env::var("MG_RUNS").ok();
    let runs = [
        ("front", 180.0, CLOSE, false),
        ("left", 90.0, CLOSE, false),
        ("right", 270.0, CLOSE, false),
        ("back", 0.0, CLOSE, false),
        ("steep", 180.0, STEEP, false),
        ("pause", 180.0, CLOSE, true),
    ];
    for (run, facing, camera, pause) in runs {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|g| g == run)) {
            continue;
        }
        shoot(run, facing, camera, false, pause);
    }
    eprintln!("pictures: {}", mg_testkit::test_output().join("torch_arm").display());
}

/// The magenta of a picture (the stick): how many pixels, their middle and
/// the direction they lie along (degrees from the picture's x axis, 0 to
/// 180, the picture's y down).
fn magenta(img: &Rgba) -> (usize, [f64; 2], f64) {
    let (mut n, mut sx, mut sy) = (0usize, 0f64, 0f64);
    let mut at = Vec::new();
    // (The game's GUI is left out: the picture's middle.)
    for y in 0..img.height as usize * 4 / 5 {
        for x in img.width as usize / 10..img.width as usize * 9 / 10 {
            let p = &img.data[(y * img.width as usize + x) * 4..][..3];
            let (r, g, b) = (i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
            if r > 70 && b > 70 && g * 2 < r && g * 2 < b {
                n += 1;
                sx += x as f64;
                sy += y as f64;
                at.push((x as f64, y as f64));
            }
        }
    }
    let (cx, cy) = (sx / n.max(1) as f64, sy / n.max(1) as f64);
    let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
    for (x, y) in at {
        xx += (x - cx) * (x - cx);
        xy += (x - cx) * (y - cy);
        yy += (y - cy) * (y - cy);
    }
    let angle = (0.5 * (2.0 * xy).atan2(xx - yy)).to_degrees().rem_euclid(180.0);
    (n, [cx, cy], angle)
}

/// How far apart two directions of [`magenta`] are, degrees.
fn turned(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(180.0);
    d.min(180.0 - d)
}

/// The torch is held where the client holds it. The torch's model made a
/// magenta stick from the hand's node (the scratch override's
/// `it_torch_000`), the bandit seen from ahead, from each side and from
/// behind (`mark_front`, `mark_left`, `mark_right`, `mark_back`):
///
/// - the stick's middle and direction in Moonglow's area view (Shaders
///   off and on) are the client's, within 10 pixels (of 1024 by 768; the
///   close camera is the client's only about, [`shoot_close`]) and 3
///   degrees;
/// - the stick is where the layering rule puts it: his pose worked out
///   from the animations' own keys (`pause1` with every node `torchl`
///   keys taken from `torchl`: [`mg_render::anim::pose_layers_scaled`],
///   the one rule under the previews, the area view and the viewer's
///   stage) and seen through the same camera, within 16 pixels and 3
///   degrees of the client's (the stick's middle as worked out is 5 to
///   8 pixels below the one found in each view: the fist hides the
///   stick's lower end, so what is seen of it has its middle higher);
/// - and not where the rule the report supposed would put it (the
///   forearm and the hand from `torchl`, the upper arm left to the idle):
///   that stick is 100 pixels and more from the client's in every view.
#[test]
#[ignore]
fn torch_arm_held_as_the_client_holds_it() {
    let _ = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let only = std::env::var("MG_RUNS").ok();
    let root = mg_testkit::nwn_root().expect("the game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let read = |name: &str| Model::read(&game.resman.get_named(name, ResType::MDL).unwrap());
    let (body, anims) = (read("c_ynpc_h_m03").unwrap(), read("a_ba_non_combat").unwrap());
    let anim = |name: &str| anims.animations.iter().find(|a| a.name == name).unwrap();
    let (idle, torch) = (anim("pause1"), anim("torchl"));
    // (The report's rule: the upper arm not the overlay's.)
    let mut forearm_only = torch.clone();
    forearm_only.nodes.retain(|n| !n.name.eq_ignore_ascii_case("lbicep_g"));
    let lhand = body.nodes.iter().position(|n| n.name == "lhand").unwrap();
    let hand = |over: &mg_mdl::Animation| {
        mg_render::anim::pose_layers_scaled(&body, &[(idle, 1.0), (over, 1.0)], 0.0)[lhand]
    };
    let (ours, supposed) = (hand(torch), hand(&forearm_only));
    let mut failures = Vec::new();
    for (run, facing) in
        [("mark_front", 180f32), ("mark_left", 90.0), ("mark_right", 270.0), ("mark_back", 0.0)]
    {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|g| g == run)) {
            continue;
        }
        let (dir, client) = shoot(run, facing, CLOSE, true, false);
        let (w, h) = (client.width as f32, client.height as f32);
        // The stick as a pose puts it, through the area view's camera.
        let camera = camera_facing(
            Vec3::new(20.0, 20.0, 0.0),
            CLOSE.0,
            FIT_FOV,
            FIT_FOCUS,
            CLOSE.1 * 0.92,
            CLOSE.2 - 1.0,
        );
        let placed =
            Mat4::from_translation(Vec3::from(AT)) * Mat4::from_rotation_z(facing.to_radians());
        let seen = |hand: Mat4, up: f32| {
            let p = (camera.projection(w / h) * camera.view() * placed * hand)
                .project_point3(Vec3::new(0.0, up, 0.0));
            [f64::from((p.x + 1.0) / 2.0 * w), f64::from((1.0 - p.y) / 2.0 * h)]
        };
        let posed = |hand: Mat4| {
            let (a, b) = (seen(hand, 0.0), seen(hand, STICK));
            let angle = (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees().rem_euclid(180.0);
            (seen(hand, STICK / 2.0), angle)
        };
        let apart = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        let (n, middle, angle) = magenta(&client);
        assert!(n > 150, "{run}: the client's stick is not seen ({n} pixels)");
        eprintln!("{run}: the client's stick at {middle:.1?}, {angle:.1} degrees ({n} pixels)");
        let mut held = |what: &str, at: [f64; 2], along: f64, within: f64| {
            let (d, a) = (apart(at, middle), turned(along, angle));
            eprintln!("{run}: {what} at {at:.1?}, {along:.1} degrees: {d:.1} pixels, {a:.1} off");
            if d > within || a > 3.0 {
                failures.push(format!("{run}: {what} {d:.1} pixels, {a:.1} degrees off"));
            }
        };
        for side in ["moonglow", "moonglow_on"] {
            let path = dir.join(format!("{side}.png"));
            if !path.exists() {
                eprintln!("{run}: {side} skipped: no GPU");
                continue;
            }
            let (n, at, along) = magenta(&common::read_png(&path));
            assert!(n > 150, "{run}: {side}'s stick is not seen ({n} pixels)");
            held(side, at, along, 10.0);
        }
        let (at, along) = posed(ours);
        held("the pose", at, along, 16.0);
        let (at, along) = posed(supposed);
        let d = apart(at, middle);
        eprintln!("{run}: the supposed rule at {at:.1?}, {along:.1} degrees: {d:.1} pixels");
        if d < 100.0 {
            failures.push(format!("{run}: the supposed rule is {d:.1} pixels from the client's"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
