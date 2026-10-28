//! Heed's armourer (his report of October 2026, the area `hss_armourer`
//! he sent: "the lighting" of the area view not the game's in his
//! interiors).
//!
//! His file isn't ours to keep; the area is built of stock content as his
//! is: a 4×4 `tni01` (TNO: City Interior) of the smithy group
//! (`smithy2x2_1`, tiles 296 to 299) in a ring of the tileset's black
//! filler (tile 94), each tile turned and its main lights, source lights
//! and animation loops set as he set them ([`TILES`]); the area lit as his
//! (no day and night, by day, ambient and diffuse 60 and 170 gray, the
//! moon's the same, black fog of 5, fog clip 45, no shadows, the "Interior
//! Bright" scheme, skybox 6) and the innkeeper where he stands (his scripts
//! taken away: he stands still). `MG_HEED_AREA=FOLDER` reads the `.are`
//! and `.git` of a folder instead (his own, by hand) and prints where its
//! tiles and lighting differ from the built one.
//!
//! The client's pictures are kept (`captures/client/armourer_<camera>.png`,
//! with where the player stood beside them); `MG_CLIENT_CAPTURED=1` reads
//! them instead of running the client again.

use super::heed::{keep_stood, regions};
use super::*;

/// A tile of [`TILES`].
type Tile = (i32, i32, [u8; 2], [u8; 2], [u8; 3]);

/// His tiles, in `Tile_List` order (rows from the south, each from the
/// west): the tileset's tile, its orientation, main lights 1 and 2 and
/// source lights 1 and 2 (lightcolor.2da rows) and animation loops 1 to 3.
/// Only the smithy's tiles have light nodes (the filler is a black plane):
/// their main lights are row 26 ("PaleRed", 2.0, 1.2, 1.2) and their
/// source lights off; the filler's (4, 30, 13, 14 and 0; 2 and 3) light
/// nothing.
#[rustfmt::skip]
const TILES: [Tile; 16] = [
    (94, 3, [4, 0], [2, 2], [1, 1, 1]),
    (94, 2, [30, 13], [3, 3], [1, 1, 1]),
    (94, 0, [4, 0], [3, 3], [1, 1, 1]),
    (94, 3, [0, 13], [3, 3], [1, 1, 1]),
    (94, 3, [0, 14], [2, 2], [1, 1, 1]),
    (297, 2, [26, 26], [0, 0], [1, 1, 0]),
    (296, 2, [26, 26], [0, 0], [1, 1, 0]),
    (94, 2, [0, 0], [3, 3], [1, 1, 1]),
    (94, 0, [4, 0], [0, 0], [1, 1, 1]),
    (298, 2, [26, 26], [0, 0], [1, 0, 0]),
    (299, 2, [26, 26], [0, 0], [1, 1, 0]),
    (94, 2, [30, 13], [3, 3], [1, 1, 1]),
    (94, 3, [0, 0], [0, 0], [0, 0, 0]),
    (94, 2, [0, 0], [0, 0], [0, 0, 0]),
    (94, 3, [0, 0], [0, 0], [0, 0, 0]),
    (94, 0, [0, 0], [0, 0], [0, 0, 0]),
];

/// Where the innkeeper stands, and his facing (degrees anticlockwise from
/// east).
const INNKEEPER: ([f32; 2], f32) = ([26.605, 14.958], 165.2);

/// The cameras: the player's place and the facing the scripted camera
/// looks along, at the distance and pitch [`fitted_camera`] fits.
/// `MG_ARMOURER_CAMERAS=name@x,y@facing;...` gives others.
const CAMERAS: [(&str, [f32; 2], f32); 4] = [
    ("north", [20.0, 17.0], 90.0),
    ("forge", [21.0, 21.0], 270.0),
    ("west", [25.0, 13.0], 180.0),
    ("east", [16.0, 20.0], 0.0),
];

/// How far Moonglow's pictures may be from the client's: each channel of
/// the mean color of the picture (clear of the game's GUI) and of each of
/// a 3×3 grid of it, of 255; as `heed_interior_look`'s.
const WHOLE_WITHIN: f32 = 3.0;
const REGION_WITHIN: f32 = 10.0;

/// How far apart Moonglow's two pictures (the Shaders switch off and on)
/// may be, mean difference of 255.
const SWITCH_WITHIN: f64 = 0.5;

/// The scripts a creature runs, taken away so that it stands.
const CREATURE_SCRIPTS: [&str; 13] = [
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
];

/// The area built as his is: its ARE and GIT.
fn armourer_area(game: &GameData) -> (Gff, Gff) {
    use mg_module::instances::{Placement, Placing, instance};
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(game, "MgScene", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Armourer".into(),
        tileset: ResRef::from_str("tni01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, game, &spec, &mut rng).unwrap();
    let mut are = m.gff(&ResKey::new(area, ResType::ARE)).unwrap().unwrap();
    let mut git = m.gff(&ResKey::new(area, ResType::GIT)).unwrap().unwrap();
    for (t, (id, orientation, main, source, anim)) in
        are.root.list_mut("Tile_List").unwrap().iter_mut().zip(TILES)
    {
        t.set("Tile_ID", Value::Int(id));
        t.set("Tile_Orientation", Value::Int(orientation));
        t.set("Tile_Height", Value::Int(0));
        for (k, label) in ["Tile_MainLight1", "Tile_MainLight2"].into_iter().enumerate() {
            t.set(label, Value::Byte(main[k]));
        }
        for (k, label) in ["Tile_SrcLight1", "Tile_SrcLight2"].into_iter().enumerate() {
            t.set(label, Value::Byte(source[k]));
        }
        for (k, label) in
            ["Tile_AnimLoop1", "Tile_AnimLoop2", "Tile_AnimLoop3"].into_iter().enumerate()
        {
            t.set(label, Value::Byte(anim[k]));
        }
    }
    for prefix in ["Sun", "Moon"] {
        are.root.set(&format!("{prefix}AmbientColor"), Value::Dword(0x3C3C3C));
        are.root.set(&format!("{prefix}DiffuseColor"), Value::Dword(0xAAAAAA));
        are.root.set(&format!("{prefix}FogColor"), Value::Dword(0));
        are.root.set(&format!("{prefix}FogAmount"), Value::Byte(5));
        are.root.set(&format!("{prefix}Shadows"), Value::Byte(0));
    }
    for (label, v) in [("IsNight", 0u8), ("DayNightCycle", 0), ("SkyBox", 6), ("ShadowOpacity", 60)]
    {
        are.root.set(label, Value::Byte(v));
    }
    are.root.set("LightingScheme", Value::Byte(12));
    are.root.set("FogClipDist", Value::Float(45.0));
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let innkeeper =
        Gff::read(&game.resman.get_named("nw_innkeeper", ResType::UTC).unwrap()).unwrap().root;
    let ([x, y], facing) = INNKEEPER;
    let at = Placement { position: [x, y, 0.0], rotation: facing.to_radians() };
    let creature = instance(&placing, ResType::UTC, &innkeeper, at, &[]).unwrap();
    git.root.set("Creature List", Value::List(vec![creature]));
    (are, git)
}

/// [`MG_HEED_AREA`](self): the folder's area, and where it differs from
/// the built one, printed (its tiles' fields and its lighting).
fn read_area(folder: &Path, built: &Gff) -> (Gff, Gff) {
    let file = |ext: &str| {
        mg_testkit::files_with_ext(folder, ext)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("no .{ext} in {}", folder.display()))
    };
    let read = |ext: &str| Gff::read(&std::fs::read(file(ext)).unwrap()).unwrap();
    let (are, git) = (read("are"), read("git"));
    let tiles = |g: &Gff| g.root.list("Tile_List").unwrap().to_vec();
    let mut differ = 0;
    for (i, (a, b)) in tiles(&are).iter().zip(tiles(built)).enumerate() {
        for label in [
            "Tile_ID",
            "Tile_Orientation",
            "Tile_Height",
            "Tile_MainLight1",
            "Tile_MainLight2",
            "Tile_SrcLight1",
            "Tile_SrcLight2",
            "Tile_AnimLoop1",
            "Tile_AnimLoop2",
            "Tile_AnimLoop3",
        ] {
            if a.int(label) != b.int(label) {
                eprintln!("tile {i}: {label} {:?}, built {:?}", a.int(label), b.int(label));
                differ += 1;
            }
        }
    }
    for label in [
        "Tileset",
        "Width",
        "Height",
        "Flags",
        "SunAmbientColor",
        "SunDiffuseColor",
        "SunFogColor",
        "SunFogAmount",
        "SunShadows",
        "MoonAmbientColor",
        "MoonDiffuseColor",
        "MoonFogColor",
        "MoonFogAmount",
        "MoonShadows",
        "IsNight",
        "DayNightCycle",
        "SkyBox",
        "ShadowOpacity",
        "LightingScheme",
        "FogClipDist",
    ] {
        let (a, b) = (are.root.get(label), built.root.get(label));
        if format!("{a:?}") != format!("{b:?}") {
            eprintln!("{label}: {a:?}, built {b:?}");
            differ += 1;
        }
    }
    eprintln!("{}: {differ} fields differ from the built area", folder.display());
    (are, git)
}

/// [`CAMERAS`], or `MG_ARMOURER_CAMERAS`'s.
fn cameras() -> Vec<(String, glam::Vec2, f32)> {
    let Ok(v) = std::env::var("MG_ARMOURER_CAMERAS") else {
        return CAMERAS
            .iter()
            .map(|&(n, [x, y], f)| (n.to_string(), glam::vec2(x, y), f))
            .collect();
    };
    v.split(';')
        .filter(|c| !c.trim().is_empty())
        .map(|c| {
            let parts: Vec<&str> = c.split('@').collect();
            let p: Vec<f32> = parts[1].split(',').filter_map(|s| s.trim().parse().ok()).collect();
            (parts[0].trim().to_string(), glam::vec2(p[0], p[1]), parts[2].trim().parse().unwrap())
        })
        .collect()
}

/// Heed's armourer in the client and in Moonglow's area view (lit, with
/// fog, with the Shaders switch off and on), from each of [`CAMERAS`]: the
/// pictures in `target/test-output/client_armourer_<camera>/`, the mean
/// difference from the client's and the mean colors of a 3×3 grid of
/// regions printed. The two switches' pictures must be the same, and near
/// the client's ([`WHOLE_WITHIN`], [`REGION_WITHIN`]). Needs the game, a
/// GPU and the off-screen display (or the kept captures); run by hand
/// (`--ignored`).
#[test]
#[ignore]
fn heed_armourer_look() {
    use mg_rules::ObjectKind::{Encounter, Sound, Store, Trigger, Waypoint};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let mut report = Vec::new();
    let mut failures = Vec::new();
    for (name, pc, facing) in cameras() {
        let dir = scratch_dir(&format!("client_armourer_{name}"));
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        let install = GameInstall::new(&root, Some(dir.join("user")), "en");
        let game = GameData::open(&install).unwrap();
        let (mut are, mut git) = armourer_area(&game);
        if let Some(folder) = std::env::var_os("MG_HEED_AREA") {
            (are, git) = read_area(Path::new(&folder), &are);
        }
        for c in git.root.list_mut("Creature List").into_iter().flatten() {
            for label in CREATURE_SCRIPTS {
                c.set(label, Value::resref(ResRef::EMPTY));
            }
        }
        save_area_module_facing(&game, &dir, &mut are, &git, pc, &[], facing);
        let client = client_capture(&dir, "MgScene", &format!("armourer_{name}"));
        keep_stood(&dir, &format!("armourer_{name}"));
        if let Some(c) = &client {
            save_png(c, &dir.join("client.png"));
        }
        let tileset = mg_area::tileset(&game, are.root.resref("Tileset").unwrap()).ok();
        let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
        let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
        let pc = player_stood(&dir, &game, &model, pc);
        let camera = scripted_camera(pc, facing);
        let size = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
        // (Without the editor's markers: the client shows none.)
        let mut view = mg_area::View { lit: true, fog: true, ..mg_area::View::of(&model) };
        for kind in [Encounter, Sound, Store, Trigger, Waypoint] {
            view.show[kind.index()] = false;
        }
        // The tiles' emitters (the forge's fire and smoke) as the area view
        // draws them from its first frame: run ahead as they are new.
        let mut sims = HashMap::new();
        let mut scene = area_scene.scene(&model, &view);
        scene.particles = area_scene.particles(&model, &view, &mut sims, 0.0, camera.view());
        eprintln!(
            "{name}: {} lights, {} particles, area light {:?}",
            scene.lights.len(),
            scene.particles.iter().map(|b| b.vertices.len() / 4).sum::<usize>(),
            scene.area
        );
        for l in &scene.lights {
            eprintln!(
                "  light at {:.2?}: color {:.3?}, cutoff {:.1}, priority {}",
                l.position, l.color, l.cutoff, l.priority
            );
        }
        if let (Some(c), Some(_)) = (&client, std::env::var_os("MG_FIT")) {
            fit_turned_camera(&gpu, &game, &scene, c, pc, facing);
        }
        let (off, on) =
            off_and_on(&gpu, &game, &scene, &camera, size, client.as_ref(), &dir, "moonglow");
        let between = picture_difference(&off, &on);
        if between > SWITCH_WITHIN {
            failures
                .push(format!("{name}: the Shaders switch's pictures {between:.2} of 255 apart"));
        }
        let Some(client) = client else {
            eprintln!("{name}: no client picture ({})", dir.display());
            continue;
        };
        region_report(&name, &[("client", &client), ("off", &off), ("on", &on)]);
        let (d_off, d_on) = (picture_difference(&client, &off), picture_difference(&client, &on));
        let mut worst = [[0f32; 2]; 2];
        for (region, rect) in regions() {
            let c = mean(&client, rect);
            for (k, (switch, ours)) in [("off", &off), ("on", &on)].into_iter().enumerate() {
                let o = mean(ours, rect);
                let d = (0..3).map(|i| (o[i] - c[i]).abs()).fold(0f32, f32::max);
                let whole = region == "whole";
                let within = if whole { WHOLE_WITHIN } else { REGION_WITHIN };
                let w = &mut worst[usize::from(!whole)][k];
                *w = w.max(d);
                if d > within {
                    failures.push(format!(
                        "{name} (Shaders {switch}): {region} {o:.1?}, the client's {c:.1?}"
                    ));
                }
            }
        }
        report.push(format!(
            "{name}: from the client's picture, Shaders off {d_off:.2}, on {d_on:.2} of 255 pixel \
             by pixel; its mean color within {:.2} and {:.2}, the worst region's {:.2} and {:.2}; \
             the two {between:.2} apart",
            worst[0][0], worst[0][1], worst[1][0], worst[1][1]
        ));
    }
    for line in &report {
        eprintln!("{line}");
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
