//! The sun's and moon's shadows in the game client beside Moonglow's area
//! view of the same area: which way they fall, how long and how dark they
//! are at an area's Shadow Opacity, what they fall on, and when there are
//! none (Sun Shadows or Moon Shadows off for the time of day).
//!
//! The area is the reference scenes' camera over a 4×4 area of ttr01 (or
//! `MG_SHADOW_TILESET`: tic01's walls and pillars cast long shadows over
//! its floor), the player invisible at its middle and the camera fixed by
//! script, with the client's environment shadows on. Four armoires stand
//! north of the player in two pairs, each a caster with another a little
//! to its south-west, where its shadow falls (the sun is towards 4000,
//! 4500, 7000): on the left both dynamic, on the right the one shadowed
//! static. Each lighting below is one client run (its pictures in
//! `target/test-output/client_shadows_<tileset>_<lighting>/`: `client.png`;
//! Moonglow's area view of the same area from the same camera,
//! `moonglow.png` with its own shader and `moonglow_on.png` with the
//! game's, `moonglow_none.png` without shadows, as before Moonglow drew
//! them).
//!
//! What was measured in the client (EE 89.8193.37, ttr01, ambient 0x606060,
//! diffuse 0xC0C0C0), as the darkening of the floor in the left pair's
//! shadow (1 less its brightness over the lit floor's beside it, against
//! the same with no shadows):
//!
//! - 0.30 at an opacity of 30, 0.45 at 60 and at 100: black laid over the
//!   picture as opaque as the opacity, no more than the shadow plane's
//!   fade up the screen allows (0.47 there, `fs_shadowplane`);
//! - the same by night with Moon Shadows (the same direction, the moon's
//!   colors being the sun's here), none by day with only Moon Shadows on
//!   or by night with only Sun Shadows;
//! - the static armoire's face darkened, the dynamic one's beside it not:
//!   static placeables receive shadows, dynamic ones none (not even their
//!   own).
//!
//! The test holds Moonglow's floor and faces to the client's within
//! [`TOLERANCE`], with both of its shaders.
//!
//! Creatures and lights: `MG_SHADOW_CREATURE=nw_ogre01` places an ogre
//! between the pairs (its sun shadow measured too), `MG_CREATURE_SHADOWS`
//! sets the client's Creature Shadow Detail (Best casts the ogre's and
//! the dynamic armoires' shadows; Fast and Off, none), and on tic01
//! (`MG_SHADOW_TILESET=tic01`) `MG_SHADOW_LAMP=73@19.8,25.5` stands a lamp
//! post behind the ogre, `MG_SHADOW_SOURCES` lights the tiles' torches,
//! `MG_SHADOW_COLORS` darkens the area and `MG_SHADOW_LIGHTS=1` draws
//! Moonglow's lights' shadows: with the sun's and moon's off at the time
//! (`day_off`, `day_off_30`) the ogre casts one from the nearest lamp,
//! measured against the same lighting's capture with Creature Shadow
//! Detail Off (made first, `MG_CREATURE_SHADOWS=0`).
//!
//! Needs the game, a GPU and the off-screen display; run by hand:
//! `DISPLAY=:1 cargo test -p mg-corpus-tests --release --test client_render shadows -- --ignored --nocapture`.
//! `MG_CLIENT_CAPTURED=1` reads the client's pictures kept from an earlier
//! run (`~/.local/share/moonglow-oracle/captures/client/shadows_<tileset>_<lighting>.png`)
//! instead of running it again; `MG_SHADOW_ONLY` names the lightings to run
//! (the first, `day_off`, is every other's measure of no shadow).

use super::*;
use mg_module::instances::{Placement, Placing, instance};

/// A lighting: night (the moon's), the area's Sun Shadows and Moon
/// Shadows, and its Shadow Opacity.
#[derive(Debug, Clone, Copy)]
struct Shadows {
    name: &'static str,
    night: bool,
    sun: bool,
    moon: bool,
    opacity: u8,
}

const LIGHTINGS: [Shadows; 7] = [
    Shadows { name: "day_off", night: false, sun: false, moon: true, opacity: 100 },
    Shadows { name: "day_off_30", night: false, sun: false, moon: true, opacity: 30 },
    Shadows { name: "day_100", night: false, sun: true, moon: false, opacity: 100 },
    Shadows { name: "day_60", night: false, sun: true, moon: false, opacity: 60 },
    Shadows { name: "day_30", night: false, sun: true, moon: false, opacity: 30 },
    Shadows { name: "night_100", night: true, sun: false, moon: true, opacity: 100 },
    Shadows { name: "night_off", night: true, sun: true, moon: false, opacity: 100 },
];

/// How far a darkening of Moonglow's may be from the client's.
const TOLERANCE: f32 = 0.08;

/// Where `MG_SHADOW_CREATURE`'s creature stands: between the pairs, its
/// shadow falling on the floor towards the left pair.
const CREATURE_AT: [f32; 3] = [19.8, 22.6, 0.0];

/// The armoires: where (x, y), and whether static.
const ARMOIRES: [(f32, f32, bool); 4] =
    [(17.0, 24.5, false), (16.0, 23.4, false), (23.0, 24.5, false), (22.0, 23.4, true)];

/// Regions of the pictures (fractions, top-left origin), picked on the
/// client's: the floor in the left pair's shadow and the lit floor east of
/// it; the left (dynamic) and right (static) receivers' faces.
const FLOOR_SHADOW: [f32; 4] = [0.288, 0.449, 0.317, 0.469];
const FLOOR_LIT: [f32; 4] = [0.371, 0.449, 0.410, 0.469];
const FACE_DYNAMIC: [f32; 4] = [0.317, 0.371, 0.361, 0.423];
const FACE_STATIC: [f32; 4] = [0.557, 0.371, 0.601, 0.423];
/// `MG_SHADOW_CREATURE`'s ogre's shadow on the floor (nw_ogre01, picked
/// on the client's pictures) and the lit floor east of it.
const CREATURE_SHADOW: [f32; 4] = [0.425, 0.49, 0.455, 0.515];
const CREATURE_LIT: [f32; 4] = [0.53, 0.49, 0.56, 0.515];
/// On tic01, with the lamp at (19.8, 25.5) behind the ogre: the ogre's
/// shadow from it and the lit floor east of that.
const LAMP_SHADOW: [f32; 4] = [0.475, 0.495, 0.51, 0.525];
const LAMP_LIT: [f32; 4] = [0.56, 0.495, 0.595, 0.525];

/// The client's settings for the comparison with its shadows on.
pub(super) fn settings() -> String {
    grassy_settings(false)
}

/// `MG_CREATURE_SHADOWS`: the client's Creature Shadow Detail (0 Off, 1
/// Fast, 2 Best, the default).
fn creature_mode() -> u8 {
    std::env::var("MG_CREATURE_SHADOWS").ok().and_then(|m| m.parse().ok()).unwrap_or(2)
}

/// `MG_SHADOW_SOURCES=<value>`: every tile's source lights (`Tile_SrcLight1`
/// and 2, the flame and its light, `shadow 1`) set to it, for the lights'
/// shadows; `MG_SHADOW_COLORS=<ambient>,<diffuse>` (0xBBGGRR in hex) for
/// the area's sun and moon instead of 0x606060 and 0xC0C0C0.
fn sources() -> Option<u8> {
    std::env::var("MG_SHADOW_SOURCES").ok().and_then(|v| v.parse().ok())
}

fn colors() -> (u32, u32) {
    std::env::var("MG_SHADOW_COLORS")
        .ok()
        .and_then(|v| {
            let (a, d) = v.split_once(',')?;
            Some((u32::from_str_radix(a.trim(), 16).ok()?, u32::from_str_radix(d.trim(), 16).ok()?))
        })
        .unwrap_or((0x606060, 0xC0C0C0))
}

/// `MG_SHADOW_CREATURE`: a creature blueprint of the game's placed among
/// the armoires (where the player stands, 2 m north), its scripts taken
/// away so that it stands still.
fn creature() -> Option<String> {
    std::env::var("MG_SHADOW_CREATURE").ok().filter(|c| !c.is_empty())
}

/// [`settings`], with the client's grass where `grass` (its default, mode
/// 2).
fn grassy_settings(grass: bool) -> String {
    let settings = if grass {
        SETTINGS.replace("[graphics.grass]\n\t\tmode = 0", "[graphics.grass]\n\t\tmode = 2")
    } else {
        SETTINGS.to_string()
    };
    settings
        .replace(
            "[graphics.shadows.creatures]\n\t\t\tmode = 0",
            &format!("[graphics.shadows.creatures]\n\t\t\tmode = {}", creature_mode()),
        )
        .replace(
            "[graphics.shadows.environment]\n\t\t\tenabled = false",
            "[graphics.shadows.environment]\n\t\t\tenabled = true",
        )
        // (`MG_CASTING_LIGHTS`: the client's `max-casting-lights`, 0 to 3,
        // its default 3.)
        .replace(
            "\t[graphics.shadows]\n",
            &format!("\t[graphics.shadows]\n\t\tmax-casting-lights = {}\n", casting_lights()),
        )
}

/// `MG_SHADOW_LAMP=<row>@<x>,<y>[;...]`: lit placeables' appearances and
/// places.
fn lamps() -> Vec<(u32, f32, f32)> {
    let v = std::env::var("MG_SHADOW_LAMP").unwrap_or_default();
    v.split(';')
        .filter_map(|l| {
            let (row, at) = l.split_once('@')?;
            let (x, y) = at.split_once(',')?;
            Some((row.trim().parse().ok()?, x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .collect()
}

/// `MG_CASTING_LIGHTS` (0 to 3, the client's default 3).
fn casting_lights() -> u8 {
    std::env::var("MG_CASTING_LIGHTS").ok().and_then(|v| v.parse().ok()).unwrap_or(3)
}

/// The mean brightness (of 255) of a rectangle (fractions of the image).
fn brightness(img: &Rgba, r: [f32; 4]) -> f32 {
    let m = mean(img, r);
    (m[0] + m[1] + m[2]) / 3.0
}

/// A picture's ratios: the shadowed floor's brightness over the lit
/// floor's, the static face's over the dynamic one's, the floor in the
/// creature's shadow over the lit floor beside it.
fn ratios(img: &Rgba) -> [f32; 4] {
    let over = |a: [f32; 4], b: [f32; 4]| brightness(img, a) / brightness(img, b).max(1e-3);
    [
        over(FLOOR_SHADOW, FLOOR_LIT),
        over(FACE_STATIC, FACE_DYNAMIC),
        over(CREATURE_SHADOW, CREATURE_LIT),
        over(LAMP_SHADOW, LAMP_LIT),
    ]
}

/// How much a picture's shadows darken the floor, the static face, the
/// floor under the creature's shadow and under its lamp's: 1 less its
/// ratios over those of the picture with none.
fn darkening(img: &Rgba, none: [f32; 4]) -> [f32; 4] {
    let r = ratios(img);
    std::array::from_fn(|i| 1.0 - r[i] / none[i])
}

/// The area of `tileset` lit as `s` says, with the armoires, saved as the
/// scratch user folder's module: its ARE and GIT.
fn module(game: &GameData, dir: &Path, tileset: &str, s: Shadows) -> (Gff, Gff) {
    let (ambient, diffuse) = colors();
    let light = Lighting { ambient, diffuse, main_light: 0, only_tile: None };
    let (mut m, mut are) = build_module_on(game, dir, light, tileset, 0);
    if let Some(value) = sources() {
        for t in are.root.list_mut("Tile_List").unwrap() {
            t.set("Tile_SrcLight1", Value::Byte(value));
            t.set("Tile_SrcLight2", Value::Byte(value));
        }
    }
    for (label, v) in [
        ("SunShadows", u8::from(s.sun)),
        ("MoonShadows", u8::from(s.moon)),
        ("ShadowOpacity", s.opacity),
        ("IsNight", u8::from(s.night)),
    ] {
        are.root.set(label, Value::Byte(v));
    }
    let are_key = *m.keys_of(ResType::ARE).next().unwrap();
    m.set_gff(are_key, &are).unwrap();
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let bp = game.resman.get_named("plc_armoire", ResType::UTP).unwrap();
    let bp = Gff::read(&bp).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let placed: Vec<_> = ARMOIRES
        .iter()
        .map(|&(x, y, is_static)| {
            // (Facing south, the camera, as the reference scenes' armoire:
            // turned by its facing less a quarter.)
            let at = Placement { position: [x, y, 0.0], rotation: std::f32::consts::PI };
            let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
            p.set("Static", Value::Byte(u8::from(is_static)));
            p
        })
        .collect();
    let mut placed = placed;
    // (`MG_SHADOW_LAMP=<row>@<x>,<y>`: a placeable of that appearance
    // there, its light lit: placeables.2da's `LightColor`, a light of
    // `shadow 1`.)
    for (row, x, y) in lamps() {
        let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Lamp", 0);
        bp.root.set("Appearance", Value::Dword(row));
        let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
        placed.push(instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap());
    }
    git.root.set("Placeable List", Value::List(placed));
    if let Some(name) = creature() {
        let utc = game.resman.get_named(&name, ResType::UTC).expect("the creature's blueprint");
        let mut c = Gff::read(&utc).unwrap().root;
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
        let at = Placement { position: CREATURE_AT, rotation: std::f32::consts::PI };
        let placed = instance(&placing, ResType::UTC, &c, at, &[]).unwrap();
        git.root.set("Creature List", Value::List(vec![placed]));
    }
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    (are, git)
}

#[test]
#[ignore]
fn shadows_look() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let tileset = std::env::var("MG_SHADOW_TILESET").unwrap_or_else(|_| "ttr01".into());
    let only = std::env::var("MG_SHADOW_ONLY").ok();
    // (`MG_SHADOW_GRASS=1`: with the grass, in the client and the view.)
    let grass = std::env::var_os("MG_SHADOW_GRASS").is_some();
    let mut label = if grass { format!("{tileset}_grass") } else { tileset.clone() };
    if let Some(c) = creature() {
        label = format!("{label}_{c}_c{}", creature_mode());
    }
    if let Some(v) = sources() {
        label = format!("{label}_sources{v}");
    }
    for (row, x, y) in lamps() {
        label = format!("{label}_lamp{row}_{x}_{y}");
    }
    if sources().is_some() || !lamps().is_empty() {
        label = format!("{label}_l{}", casting_lights());
    }
    if std::env::var_os("MG_SHADOW_COLORS").is_some() {
        let (a, d) = colors();
        label = format!("{label}_{a:06x}_{d:06x}");
    }
    // (`MG_SHADOW_LIGHTS=1`: Moonglow's lights cast too.)
    let light_shadows = std::env::var_os("MG_SHADOW_LIGHTS").is_some();
    let mut client_none = None;
    let mut failures = Vec::new();
    for s in LIGHTINGS
        .iter()
        .filter(|s| s.name == "day_off" || only.as_deref().is_none_or(|o| o.contains(s.name)))
    {
        let dir = scratch_dir(&format!("client_shadows_{label}_{}", s.name));
        std::fs::create_dir_all(dir.join("user")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), grassy_settings(grass)).unwrap();
        let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
        let (are, git) = module(&game, &dir, &tileset, *s);
        let client = client_capture(&dir, "MgScene", &format!("shadows_{label}_{}", s.name));
        if let Some(c) = &client {
            save_png(c, &dir.join("client.png"));
        }
        let model = mg_area::AreaModel::read(
            &game,
            &are.root,
            &git.root,
            mg_area::tileset(&game, ResRef::from_str(&tileset).unwrap()).ok().as_ref(),
        );
        let mut area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
        let camera =
            fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
        let (w, h) = client.as_ref().map_or((1024, 768), |c| (c.width, c.height));
        let sight = grass.then(|| mg_area::Sight::of(&camera, w as f32 / h as f32));
        if let Some(sight) = &sight {
            let start = std::time::Instant::now();
            while area_scene.grow_grass(&gpu, &model, sight) && start.elapsed().as_secs() < 30 {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        // (Creatures cast as the client's Best creature shadows.)
        let creature_shadows = creature_mode() == 2;
        let view = mg_area::View {
            fog: false,
            grass: sight,
            creature_shadows,
            light_shadows,
            ..mg_area::View::of(&model)
        };
        let scene = area_scene.scene(&model, &view);
        let (off, on) =
            off_and_on(&gpu, &game, &scene, &camera, (w, h), client.as_ref(), &dir, "moonglow");
        // Moonglow's measure of no shadow: the same view without.
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
        let bare = Scene { shadows: None, light_shadows: None, ..scene.clone() };
        let none = r.render_image(&gpu, &game.resman, &bare, &camera, w, h);
        save_png(&none, &dir.join("moonglow_none.png"));
        let ours_none = ratios(&none);
        if s.name == "day_off" {
            client_none = client.as_ref().map(ratios);
        }
        // (With lamps, the client's day_off has their shadows: its measure
        // of none is the same lighting's capture with its creature shadows
        // off, which takes the lights' away too, where it was made.)
        let lamp_none = (!lamps().is_empty() && creature_mode() == 2)
            .then(|| label.replace("_c2_", "_c0_"))
            .and_then(|l| mg_testkit::aurora_capture(&format!("client/shadows_{l}_{}.png", s.name)))
            .map(|p| ratios(&common::read_png(&p)));
        let client_none = if lamps().is_empty() { client_none } else { lamp_none };
        let theirs = client.as_ref().zip(client_none).map(|(c, none)| darkening(c, none));
        let ours = [("off", darkening(&off, ours_none)), ("on", darkening(&on, ours_none))];
        eprintln!(
            "{}: {}: the shadows darken the floor, the static face and the creature's shadow \
             by: client {theirs:.3?}, Moonglow {:.3?} (Shaders off), {:.3?} (on)",
            dir.display(),
            s.name,
            ours[0].1,
            ours[1].1,
        );
        let Some(t) = theirs else { continue };
        for (switch, o) in ours {
            // (The regions were picked on ttr01's pictures, the lamp's on
            // tic01's with its lamp behind the ogre.)
            let measured: &[_] = match (tileset == "ttr01", creature().is_some(), lamps().len()) {
                (true, true, 0) => &[("floor", 0), ("static face", 1), ("creature's shadow", 2)],
                (true, false, 0) => &[("floor", 0), ("static face", 1)],
                (false, true, 1) => &[("lamp's shadow", 3)],
                _ => &[],
            };
            for &(what, i) in measured {
                if (o[i] - t[i]).abs() > TOLERANCE {
                    failures.push(format!(
                        "{} (Shaders {switch}): the {what} darkened by {:.3}, the client's {:.3}",
                        s.name, o[i], t[i]
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
