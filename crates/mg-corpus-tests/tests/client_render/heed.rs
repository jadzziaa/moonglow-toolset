//! Heed's interior (his report of October 2026: with the Shaders switch on
//! an interior lit by dark gray area lights looked dark and muddy, with
//! it off brighter and lit by the tile lights, and with a white ambient
//! flat, the tile lights overpowered).
//!
//! His area isn't ours to read; the scene is a stock one set as his is: a
//! 4×4 `tin01` interior with barrels, crates and bedrolls (Chapter 1's
//! `map_m1q1k002`, its tiles' main and source lights as they are), an
//! anvil and a commoner put beside its brazier, the area Always Bright
//! (the sun's colors, the moon's set the same), black fog of 5, no
//! shadows, the "Interior Bright" scheme, and the area's ambient and
//! diffuse of each of [`CASES`]. The client's pictures are kept
//! (`captures/client/heed_<case>.png`, with where the player stood beside
//! them); `MG_CLIENT_CAPTURED=1` reads them instead of running the client
//! again.
//!
//! `MG_HEED_SHADERS=FOLDER` puts a builder's pair (a vertex and a fragment
//! program, told apart by their `SHADER_TYPE`: Heed's `vhss_skintile` and
//! `fhss_skintile`) on every texture of the area's tiles, by an MTR named
//! as each in the scratch override, with `nUvmap` 0 (the model's own
//! coordinates): his walls and floors are drawn by that pair.

use super::*;

/// The area's ambient and diffuse colors (0xBBGGRR, the sun's and the
/// moon's), by case: "Interior Bright"'s own (60 and 170, his swatches'
/// dark and lighter gray), a darker diffuse, his white ambient, and none
/// (the tile lights alone).
const CASES: [(&str, u32, u32); 4] = [
    ("gray", 0x3C3C3C, 0xAAAAAA),
    ("gray_150", 0x3C3C3C, 0x969696),
    ("white", 0xFFFFFF, 0xAAAAAA),
    ("tiles_only", 0, 0),
];

/// How far Moonglow's pictures may be from the client's: each channel of
/// the mean color of the picture (clear of the game's GUI) and of each of
/// a 3×3 grid of it, of 255. (The picture's mean difference pixel by
/// pixel, printed, is mostly the texture's grain a fraction of a pixel
/// apart, and grows with the light.) Where the grid's regions differ most
/// (a torch's and the brazier's flames, by 4 to 8) they do with no area
/// light at all.
const WHOLE_WITHIN: f32 = 3.0;
const REGION_WITHIN: f32 = 10.0;

/// How far apart Moonglow's two pictures (the Shaders switch off and on)
/// may be, mean difference of 255: Heed's report was that they differ.
const SWITCH_WITHIN: f64 = 0.5;

/// The area set as Heed's is, with `ambient` and `diffuse`.
fn heed_area(are: &mut Gff, ambient: u32, diffuse: u32) {
    for prefix in ["Sun", "Moon"] {
        are.root.set(&format!("{prefix}AmbientColor"), Value::Dword(ambient));
        are.root.set(&format!("{prefix}DiffuseColor"), Value::Dword(diffuse));
        are.root.set(&format!("{prefix}FogColor"), Value::Dword(0));
        are.root.set(&format!("{prefix}FogAmount"), Value::Byte(5));
        are.root.set(&format!("{prefix}Shadows"), Value::Byte(0));
    }
    for (label, v) in [("IsNight", 0u8), ("DayNightCycle", 0), ("SkyBox", 0), ("ShadowOpacity", 60)]
    {
        are.root.set(label, Value::Byte(v));
    }
    are.root.set("LightingScheme", Value::Byte(12));
    are.root.set("FogClipDist", Value::Float(45.0));
}

/// The area's placeables and an anvil and a commoner (its scripts taken
/// away: it stands) beside the brazier the camera looks at; the area's
/// own creatures left out (they walk).
fn heed_objects(game: &GameData, git: &mut Gff) {
    use mg_module::instances::{Placement, Placing, instance};
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let blueprint = |name: &str, t: ResType| {
        Gff::read(&game.resman.get_named(name, t).unwrap_or_else(|_| panic!("{name}"))).unwrap()
    };
    let at = |x: f32, y: f32, facing: f32| Placement {
        position: [x, y, 0.0],
        rotation: facing.to_radians(),
    };
    let mut commoner = blueprint("nw_commale", ResType::UTC).root;
    for label in [
        "ScriptAttacked",
        "ScriptDamaged",
        "ScriptDeath",
        "ScriptDialogue",
        "ScriptDisturbed",
        "ScriptEndRound",
        "ScriptHeartbeat",
        "ScriptOnBlocked",
        "ScriptOnNotice",
        "ScriptRested",
        "ScriptSpawn",
        "ScriptSpellAt",
        "ScriptUserDefine",
    ] {
        commoner.set(label, Value::resref(ResRef::EMPTY));
    }
    let creature = instance(&placing, ResType::UTC, &commoner, at(14.5, 10.0, 200.0), &[]).unwrap();
    git.root.set("Creature List", Value::List(vec![creature]));
    let anvil = blueprint("plc_anvil", ResType::UTP).root;
    let mut list = git.root.list("Placeable List").map(<[_]>::to_vec).unwrap_or_default();
    list.push(instance(&placing, ResType::UTP, &anvil, at(16.5, 8.0, 20.0), &[]).unwrap());
    git.root.set("Placeable List", Value::List(list));
}

/// [`MG_HEED_SHADERS`](self): the folder's pair in `over`, and an MTR
/// naming it for every texture of the area's tiles.
fn heed_pair(game: &GameData, are: &Gff, from: &Path, over: &Path) {
    let (mut vs, mut fs) = (String::new(), String::new());
    for f in mg_testkit::files_with_ext(from, "shd") {
        let text = std::fs::read_to_string(&f).unwrap();
        let stem = f.file_stem().unwrap().to_string_lossy().to_ascii_lowercase();
        if text.contains("#define SHADER_TYPE 1") {
            vs = stem.clone();
        } else if text.contains("#define SHADER_TYPE 2") {
            fs = stem.clone();
        }
        std::fs::write(over.join(format!("{stem}.shd")), text).unwrap();
    }
    assert!(!vs.is_empty() && !fs.is_empty(), "no pair in {}", from.display());
    let tileset = mg_area::tileset(game, are.root.resref("Tileset").unwrap()).unwrap();
    let mut textures = std::collections::BTreeSet::new();
    for t in are.root.list("Tile_List").unwrap() {
        let Some(tile) = tileset.tiles.get(t.int("Tile_ID").unwrap_or(0) as usize) else {
            continue;
        };
        let Ok(data) = game.resman.get_named(&tile.model.to_ascii_lowercase(), ResType::MDL) else {
            continue;
        };
        let Ok(model) = Model::read(&data) else { continue };
        for mesh in model.nodes.iter().filter_map(|n| n.mesh()) {
            if let Some(b) = mesh.textures[0].as_ref().filter(|b| *b != "null") {
                textures.insert(b.clone());
            }
        }
    }
    eprintln!("the pair {vs} / {fs} on {} textures of the tiles", textures.len());
    for t in textures {
        let mtr = format!(
            "texture0 {t}\ncustomshadervs {vs}\ncustomshaderfs {fs}\nparameter int nUvmap 0\n"
        );
        std::fs::write(over.join(format!("{t}.mtr")), mtr).unwrap();
    }
}

/// Where the client's player stood ([`player_stood`] reads it from the
/// client's log), kept beside its picture (`captures/client/<name>_pc.txt`)
/// when the client ran, and put back in the log from there when its kept
/// picture is read instead.
fn keep_stood(dir: &Path, name: &str) {
    let log = dir.join("user/logs/nwclientLog1.txt");
    let kept = format!("client/{name}_pc.txt");
    let line = std::fs::read_to_string(&log)
        .ok()
        .and_then(|l| l.lines().find(|l| l.contains("MG_PC ")).map(str::to_string));
    match line {
        Some(line) => {
            let base =
                std::env::var_os("MOONGLOW_ORACLE").map(std::path::PathBuf::from).or_else(|| {
                    std::env::var_os("HOME")
                        .map(|h| std::path::PathBuf::from(h).join(".local/share/moonglow-oracle"))
                });
            if let Some(base) = base {
                let _ = std::fs::write(base.join("captures").join(&kept), line);
            }
        }
        None => {
            if let Some(p) = mg_testkit::aurora_capture(&kept) {
                std::fs::create_dir_all(log.parent().unwrap()).unwrap();
                std::fs::copy(p, &log).unwrap();
            }
        }
    }
}

/// The picture clear of the game's GUI, and a 3×3 grid of it (fractions,
/// top-left origin), as [`region_report`] prints them.
fn regions() -> Vec<(String, [f32; 4])> {
    let mut out = vec![("whole".to_string(), [0.1, 0.1, 0.9, 0.7])];
    for row in 0..3 {
        for column in 0..3 {
            let (x, y) = (0.1 + column as f32 * 0.8 / 3.0, 0.1 + row as f32 * 0.6 / 3.0);
            out.push((format!("r{row}{column}"), [x, y, x + 0.8 / 3.0, y + 0.2]));
        }
    }
    out
}

/// Heed's interior in the client and in Moonglow's area view (lit, with
/// fog, with the Shaders switch off and on), for each of [`CASES`]: the
/// pictures in `target/test-output/client_heed_<case>/`, the mean
/// difference from the client's and the mean colors of a 3×3 grid of
/// regions printed. The two switches' pictures must be the same, and near
/// the client's ([`WHOLE_WITHIN`], [`REGION_WITHIN`]). Needs the game, a
/// GPU and the off-screen display (or the kept captures); run by hand
/// (`--ignored`). `MG_AREA_MODULE`, `MG_AREA` and `MG_AREA_PC` give
/// another area and place.
///
/// Seen on 2026-10-27 (89.8193.37-17, the tests' client settings): the
/// two switches drew the same picture in every case (to 0.01 of 255), and
/// the client's mean color within 1.5 of 255 over the whole picture and
/// 8.5 in any region, white ambient and dark gray alike. With Heed's pair
/// on the tiles, Moonglow's own shader lit them as `fslit` until it read
/// the pair's `NORMAL_MAP 1`: 0.2 to 0.8 of 255 from the game's programs,
/// brighter in the dark corners; 0.00 after. With `MG_CLIENT_SETTINGS=default`
/// (the post effects on) the client's picture is more saturated and
/// warmer (saturation 0.50 against 0.43), not darker.
#[test]
#[ignore]
fn heed_interior_look() {
    use mg_rules::ObjectKind::{Encounter, Sound, Store, Trigger, Waypoint};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let source = std::env::var_os("MG_AREA_MODULE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("data/nwm/Chapter1.nwm"));
    let area_name = std::env::var("MG_AREA").unwrap_or_else(|_| "map_m1q1k002".into());
    let pair = std::env::var_os("MG_HEED_SHADERS").map(std::path::PathBuf::from);
    let from = mg_module::Module::open(&source).unwrap();
    let area = ResRef::from_str(&area_name).unwrap();
    let read = |t| from.gff(&ResKey::new(area, t)).unwrap().unwrap();
    let pc = env_pc().unwrap_or(glam::Vec2::new(10.0, 6.0));
    let mut report = Vec::new();
    let mut failures = Vec::new();
    // `MG_CLIENT_SETTINGS`: `owner` or `default`, the client as a player
    // has it (its post effects on), as `area_lighting_look` takes it.
    let settings = std::env::var("MG_CLIENT_SETTINGS").ok();
    for (case, ambient, diffuse) in CASES {
        let mut case = if pair.is_some() { format!("{case}_pair") } else { case.to_string() };
        let settings = match settings.as_deref() {
            Some(s @ ("owner" | "default")) => {
                case += &format!("_{s}");
                common::player_settings(s == "owner")
            }
            _ => SETTINGS.to_string(),
        };
        let dir = scratch_dir(&format!("client_heed_{case}"));
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), settings).unwrap();
        let install = GameInstall::new(&root, Some(dir.join("user")), "en");
        let (mut are, mut git) = (read(ResType::ARE), read(ResType::GIT));
        if let Some(pair) = &pair {
            let base = GameData::open(&install).unwrap();
            heed_pair(&base, &are, pair, &dir.join("user/override"));
        }
        let game = GameData::open(&install).unwrap();
        heed_area(&mut are, ambient, diffuse);
        heed_objects(&game, &mut git);
        save_area_module(&game, &dir, &mut are, &git, pc, &[]);
        let client = client_capture(&dir, "MgScene", &format!("heed_{case}"));
        keep_stood(&dir, &format!("heed_{case}"));
        if let Some(c) = &client {
            save_png(c, &dir.join("client.png"));
        }
        let tileset = mg_area::tileset(&game, are.root.resref("Tileset").unwrap()).ok();
        let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
        let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
        let pc = player_stood(&dir, &game, &model, pc);
        let camera = fitted_camera(pc, FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
        let size = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
        // (Without the editor's markers of sounds, waypoints and the like:
        // the client shows none.)
        let mut view = mg_area::View { lit: true, fog: true, ..mg_area::View::of(&model) };
        for kind in [Encounter, Sound, Store, Trigger, Waypoint] {
            view.show[kind.index()] = false;
        }
        let scene = area_scene.scene(&model, &view);
        eprintln!("{case}: {} lights, area light {:?}", scene.lights.len(), scene.area);
        let (off, on) =
            off_and_on(&gpu, &game, &scene, &camera, size, client.as_ref(), &dir, "moonglow");
        let between = picture_difference(&off, &on);
        if between > SWITCH_WITHIN {
            failures
                .push(format!("{case}: the Shaders switch's pictures {between:.2} of 255 apart"));
        }
        let Some(client) = client else {
            eprintln!("{case}: no client picture ({})", dir.display());
            continue;
        };
        region_report(&case, &[("client", &client), ("off", &off), ("on", &on)]);
        let (d_off, d_on) = (picture_difference(&client, &off), picture_difference(&client, &on));
        let mut worst = [0f32; 2];
        for (region, rect) in regions() {
            let c = mean(&client, rect);
            for (k, (switch, ours)) in [("off", &off), ("on", &on)].into_iter().enumerate() {
                let o = mean(ours, rect);
                let d = (0..3).map(|i| (o[i] - c[i]).abs()).fold(0f32, f32::max);
                let within = if region == "whole" { WHOLE_WITHIN } else { REGION_WITHIN };
                if region == "whole" {
                    worst[k] = worst[k].max(d);
                }
                if d > within {
                    failures.push(format!(
                        "{case} (Shaders {switch}): {region} {o:.1?}, the client's {c:.1?}"
                    ));
                }
            }
        }
        report.push(format!(
            "{case}: from the client's picture, Shaders off {d_off:.2}, on {d_on:.2} of 255 pixel \
             by pixel, its mean color within {:.2} and {:.2}; the two {between:.2} apart",
            worst[0], worst[1]
        ));
    }
    for line in &report {
        eprintln!("{line}");
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
