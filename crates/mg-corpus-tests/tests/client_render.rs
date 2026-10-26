//! L5 image check: the game client draws reference scenes and Moonglow's
//! renderer draws the same scenes; they must look alike.
//!
//! Each scene is a 4×4 interior area (tic01) with an armoire in front of the
//! player, the player invisible and the camera fixed by script, and no fog,
//! shadows or post effects that Moonglow does not draw (bloom, contrast,
//! sharpen, SSAO, vibrance):
//!
//! - `sun`: lit by the area's sun alone (tile lights off);
//! - `tile lights`: the sun black, every tile's main light 1 white
//!   (lightcolor.2da row 2): the point-light falloff and colours.
//!
//! The client runs sandboxed (`tools/nwclient/run-client.sh`: no Steam, no
//! network, no sound, a scratch user directory) on the off-screen display
//! (`tools/aurora/headless.sh start`). Its camera for the script's
//! `SetCameraFacing(90, 12, 45)` was fitted to its images (`MG_FIT=1`
//! searches again); the scripted distance and pitch do not map one to one
//! onto the view.
//!
//! Compared: mean colours of floor, wall and armoire regions away from the
//! game's GUI, Moonglow drawing with the Shaders switch off (its own
//! shader) and on (the game's).
//!
//! `light_uniforms_match_the_client` reads the client's lighting uniforms
//! back instead: a debug copy of the game's own `inc_standard.shd` (patched
//! at run time, in the scratch user directory's `override`) paints them into
//! the image, and they are compared with what Moonglow uploads.
//!
//! Needs the game, a GPU and the off-screen display; run by hand:
//! `DISPLAY=:1 cargo test -p mg-corpus-tests --test client_render -- --ignored --nocapture`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use glam::{Mat4, Quat, Vec3};
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_image::Rgba;
use mg_mdl::Model;
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_render::{AreaLight, Camera, Gpu, GpuModel, Instance, Renderer, Scene};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::{SETTINGS, client_screenshot, compile, save_png};

#[path = "client_render/ee_visuals.rs"]
mod ee_visuals;
#[path = "client_render/programs.rs"]
mod programs;
#[path = "client_render/uv_sets.rs"]
mod uv_sets;

/// The scripted camera: facing (degrees anticlockwise from east).
const FACING: f32 = 90.0;
/// The armoire, metres north of the player.
const AHEAD: f32 = 3.0;
/// The view the client shows for `SetCameraFacing(90, 12, 45)`: field of
/// view (degrees), focus height, distance, pitch (degrees from straight
/// down).
const FIT_FOV: f32 = 48.0;
const FIT_FOCUS: f32 = 1.25;
const FIT_DISTANCE: f32 = 19.0;
const FIT_PITCH: f32 = 50.0;

const ENTER: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
void main()
{
    object pc = GetEnteringObject();
    if (!GetIsPC(pc)) return;
    // The player is not in the area yet: use the start location.
    location start = GetStartingLocation();
    vector p = GetPositionFromLocation(start);
    CreateObject(OBJECT_TYPE_PLACEABLE, "plc_armoire", Location(GetAreaFromLocation(start), Vector(p.x, p.y + 3.0, p.z), 270.0));
    ApplyEffectToObject(DURATION_TYPE_PERMANENT, EffectVisualEffect(VFX_DUR_CUTSCENE_INVISIBILITY), pc);
    AssignCommand(pc, SetCameraFacing(90.0, 12.0, 45.0, CAMERA_TRANSITION_TYPE_SNAP));
    LockCameraDirection(pc, TRUE);
    LockCameraPitch(pc, TRUE);
    LockCameraDistance(pc, TRUE);
    DelayCommand(4.0, Log("MG_READY"));
}
"#;

/// Regions compared (fractions of the image, top-left origin), clear of the
/// game's GUI, with the largest mean difference allowed per channel. Most
/// match within 1; the far floor, which borders a dark wall, within 5 (the
/// fitted camera is a fraction of a pixel off).
const REGIONS: [(&str, [f32; 4], f32); 6] = [
    ("centre floor", [0.40, 0.47, 0.60, 0.56], 3.0),
    ("left floor", [0.18, 0.45, 0.35, 0.60], 3.0),
    ("right floor", [0.65, 0.45, 0.82, 0.60], 3.0),
    ("far floor", [0.35, 0.28, 0.65, 0.34], 6.0),
    ("armoire", [0.48, 0.39, 0.52, 0.44], 3.0),
    ("left wall", [0.05, 0.30, 0.12, 0.40], 3.0),
];

/// A scene's lighting.
#[derive(Clone, Copy)]
struct Lighting {
    ambient: u32,
    diffuse: u32,
    /// lightcolor.2da row for every tile's main light 1 (0: off).
    main_light: u8,
    /// Only this tile's main light (index in Tile_List), if given.
    only_tile: Option<usize>,
}

/// The mean colour of a rectangle (fractions of the image, top-left origin).
fn mean(img: &Rgba, r: [f32; 4]) -> [f32; 3] {
    let (w, h) = (img.width as f32, img.height as f32);
    let mut sum = [0f64; 3];
    let mut n = 0f64;
    for y in (r[1] * h) as u32..(r[3] * h) as u32 {
        for x in (r[0] * w) as u32..(r[2] * w) as u32 {
            let i = (y as usize * img.width as usize + x as usize) * 4;
            for (c, s) in sum.iter_mut().enumerate() {
                *s += f64::from(img.data[i + c]);
            }
            n += 1.0;
        }
    }
    sum.map(|s| (s / n.max(1.0)) as f32)
}

/// The module: one area, lit as asked, the entry script.
fn build_module(game: &GameData, dir: &Path, light: Lighting) -> (mg_module::Module, Gff) {
    build_module_on(game, dir, light, "tic01", 0)
}

/// [`build_module`] on another tileset, with a skybox (skyboxes.2da row).
fn build_module_on(
    game: &GameData,
    dir: &Path,
    light: Lighting,
    tileset: &str,
    sky_box: u8,
) -> (mg_module::Module, Gff) {
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(game, "MgScene", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Scene".into(),
        tileset: ResRef::from_str(tileset).unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, game, &spec, &mut rng).unwrap();
    let are_key = ResKey::new(area, ResType::ARE);
    let mut are = m.gff(&are_key).unwrap().unwrap();
    for (label, v) in [
        ("SunAmbientColor", light.ambient),
        ("MoonAmbientColor", light.ambient),
        ("SunDiffuseColor", light.diffuse),
        ("MoonDiffuseColor", light.diffuse),
    ] {
        are.root.set(label, Value::Dword(v));
    }
    for (label, v) in [
        ("SunFogAmount", 0u8),
        ("MoonFogAmount", 0),
        ("SunShadows", 0),
        ("MoonShadows", 0),
        ("IsNight", 0),
        ("DayNightCycle", 0),
        ("SkyBox", sky_box),
    ] {
        are.root.set(label, Value::Byte(v));
    }
    for (i, t) in are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
        let on = light.only_tile.is_none_or(|only| only == i);
        t.set("Tile_MainLight1", Value::Byte(if on { light.main_light } else { 0 }));
        for label in ["Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2"] {
            t.set(label, Value::Byte(0));
        }
    }
    m.set_gff(are_key, &are).unwrap();
    let enter = compile(dir, "mg_enter", ENTER);
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), enter);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
    m.set_info(&info).unwrap();
    (m, are)
}

/// The same scene for Moonglow: the tiles (with their lights), the
/// armoire, the camera.
fn moonglow_scene(gpu: &Gpu, game: &GameData, are: &Gff, light: Lighting) -> (Scene, Camera) {
    let rm = &game.resman;
    let load = |name: &str| {
        rm.get_named(name, ResType::MDL).ok().and_then(|d| Model::read(&d).ok()).map(Arc::new)
    };
    let set = mg_set::Tileset::parse(
        &rm.get_named("tic01", ResType::SET).unwrap(),
        game.language.codepage(),
    )
    .unwrap();
    let colors = game.table("lightcolor").unwrap();
    let color = |row: u8| -> Option<Vec3> {
        let c = |col| colors.get(row as usize, col).and_then(|v| v.parse::<f32>().ok());
        Some(Vec3::new(c("RED")?, c("GREEN")?, c("BLUE")?))
    };
    let width = are.root.int("Width").unwrap_or(4) as usize;
    let mut instances = Vec::new();
    let mut lights = Vec::new();
    let mut cache: HashMap<String, Option<Arc<GpuModel>>> = HashMap::new();
    for (i, t) in are.root.list("Tile_List").unwrap().iter().enumerate() {
        let id = t.int("Tile_ID").unwrap_or(0) as usize;
        let Some(tile) = set.tiles.get(id) else { continue };
        let name = tile.model.to_ascii_lowercase();
        let model = cache
            .entry(name.clone())
            .or_insert_with(|| load(&name).map(|m| Arc::new(GpuModel::new(gpu, m))))
            .clone();
        let Some(model) = model else { continue };
        let (x, y) = ((i % width) as f32 * 10.0 + 5.0, (i / width) as f32 * 10.0 + 5.0);
        let z = t.int("Tile_Height").unwrap_or(0) as f32 * set.general.transition;
        let turns = t.int("Tile_Orientation").unwrap_or(0) as f32;
        let transform = Mat4::from_rotation_translation(
            Quat::from_rotation_z(turns * std::f32::consts::FRAC_PI_2),
            Vec3::new(x, y, z),
        );
        let on = light.only_tile.is_none_or(|only| only == i);
        let (ml1, ml2) = (if on { light.main_light } else { 0 }, 0u8);
        lights.extend(mg_render::anim::lights(
            &model.model,
            None,
            0.0,
            &model.rest,
            transform,
            &|slot| color(if slot == 0 { ml1 } else { ml2 }),
        ));
        instances.push(Instance::new(model, transform));
    }
    let pc = Vec3::new(20.0, 20.0, 0.0);
    let armoire = Arc::new(GpuModel::new(gpu, load("plc_a01").unwrap()));
    instances.push(Instance::new(
        armoire,
        Mat4::from_rotation_translation(
            // The game turns a model by its facing − 90°: the model's +Y
            // points along the facing (matched against the client).
            Quat::from_rotation_z((270f32 - 90.0).to_radians()),
            pc + Vec3::new(0.0, AHEAD, 0.0),
        ),
    ));
    let scene = Scene {
        instances,
        lights,
        area: AreaLight::from_are(
            light.ambient,
            light.diffuse,
            Vec3::new(4000.0, 4500.0, 7000.0).normalize(),
        ),
        ..Default::default()
    };
    (scene, fitted_camera(pc, FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH))
}

fn fitted_camera(pc: Vec3, fov: f32, focus: f32, distance: f32, pitch: f32) -> Camera {
    let elevation = (90.0 - pitch).to_radians();
    let facing = FACING.to_radians();
    let focus = pc + Vec3::new(0.0, 0.0, focus);
    let dir = Vec3::new(
        -facing.cos() * elevation.cos(),
        -facing.sin() * elevation.cos(),
        elevation.sin(),
    );
    Camera {
        eye: focus + dir * distance,
        target: focus,
        fov_y: fov.to_radians(),
        near: 0.1,
        far: 200.0,
    }
}

/// How far a picture of Moonglow's is from the client's: the mean
/// difference per channel (of 255) over the picture away from the game's
/// GUI (its middle, from a tenth in at each side to seven tenths down).
fn picture_difference(client: &Rgba, ours: &Rgba) -> f64 {
    let (mut sum, mut n) = (0f64, 0f64);
    let (w, h) = (client.width as usize, client.height as usize);
    for y in h / 10..h * 7 / 10 {
        for x in w / 10..w * 9 / 10 {
            let i = (y * w + x) * 4;
            sum += (0..3)
                .map(|c| f64::from(client.data[i + c].abs_diff(ours.data[i + c])))
                .sum::<f64>()
                / 3.0;
            n += 1.0;
        }
    }
    sum / n.max(1.0)
}

/// The mean colour and the mean saturation (HSV, 0 to 1) of a rectangle
/// (fractions of the image, top-left origin).
fn mean_and_saturation(img: &Rgba, r: [f32; 4]) -> ([f32; 3], f32) {
    let (w, h) = (img.width as f32, img.height as f32);
    let (mut sum, mut sat, mut n) = ([0f64; 3], 0f64, 0f64);
    for y in (r[1] * h) as u32..(r[3] * h) as u32 {
        for x in (r[0] * w) as u32..(r[2] * w) as u32 {
            let i = (y as usize * img.width as usize + x as usize) * 4;
            let p = &img.data[i..i + 3];
            for (c, s) in sum.iter_mut().enumerate() {
                *s += f64::from(p[c]);
            }
            let (max, min) =
                (p.iter().max().copied().unwrap_or(0), p.iter().min().copied().unwrap_or(0));
            if max > 0 {
                sat += f64::from(max - min) / f64::from(max);
            }
            n += 1.0;
        }
    }
    let n = n.max(1.0);
    (sum.map(|s| (s / n) as f32), (sat / n) as f32)
}

/// The pictures' mean colours and saturations, over the picture clear of
/// the game's GUI and over each of a 3×3 grid of it, printed one line a
/// region: the client's beside each of Moonglow's.
fn region_report(what: &str, pictures: &[(&str, &Rgba)]) {
    let mut regions = vec![("whole".to_string(), [0.1, 0.1, 0.9, 0.7])];
    for row in 0..3 {
        for column in 0..3 {
            let (x, y) = (0.1 + column as f32 * 0.8 / 3.0, 0.1 + row as f32 * 0.6 / 3.0);
            regions.push((format!("r{row}{column}"), [x, y, x + 0.8 / 3.0, y + 0.2]));
        }
    }
    for (region, rect) in regions {
        let mut line = format!("{what} {region:5}");
        for (name, img) in pictures {
            let (m, s) = mean_and_saturation(img, rect);
            line += &format!(" | {name} ({:5.1}, {:5.1}, {:5.1}) S {s:.3}", m[0], m[1], m[2]);
        }
        eprintln!("{line}");
    }
}

/// Moonglow's pictures of a scene with the Shaders switch off (its own
/// shader: `{name}.png`) and on (the game's: `{name}_on.png`), and how far
/// each is from the client's ([`picture_difference`]), printed.
#[allow(clippy::too_many_arguments)]
fn off_and_on(
    gpu: &Gpu,
    game: &GameData,
    scene: &Scene,
    camera: &Camera,
    (w, h): (u32, u32),
    client: Option<&Rgba>,
    dir: &Path,
    name: &str,
) {
    let mut r = Renderer::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let off = r.render_image(gpu, &game.resman, scene, camera, w, h);
    r.game_shaders = true;
    let on = r.render_image(gpu, &game.resman, scene, camera, w, h);
    eprintln!(
        "{name}: Shaders on drew {} of {} meshes with the game's programs",
        r.drawn.game, r.drawn.meshes
    );
    save_png(&off, &dir.join(format!("{name}.png")));
    save_png(&on, &dir.join(format!("{name}_on.png")));
    if let Some(c) = client {
        eprintln!(
            "{}/{name}: from the client's picture, Shaders off {:.2}, on {:.2} of 255 \
             (failures {:?})",
            dir.display(),
            picture_difference(c, &off),
            picture_difference(c, &on),
            r.game_shader_failures()
        );
        if std::env::var_os("MG_REGIONS").is_some() {
            region_report(name, &[("client", c), ("off", &off), ("on", &on)]);
        }
    }
}

/// Exploration (`MG_FIT=1`): the camera that matches the client's image best
/// (grey levels, outside the GUI).
fn fit_camera(r: &mut Renderer, gpu: &Gpu, game: &GameData, scene: &Scene, client: &Rgba) {
    let grey = |img: &Rgba| -> Vec<f32> {
        img.data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0)
            .collect()
    };
    let cg = grey(client);
    let (w, h) = (client.width as usize, client.height as usize);
    let pc = Vec3::new(20.0, 20.0, 0.0);
    let mut best = (f32::MAX, [0.0; 4]);
    for pitch in (40..=56).step_by(2) {
        for fov in (36..=64).step_by(4) {
            for focus in [0.75f32, 1.0, 1.25, 1.5] {
                for distance in 12..=26 {
                    let p = [fov as f32, focus, distance as f32, pitch as f32];
                    let cam = fitted_camera(pc, p[0], p[1], p[2], p[3]);
                    let img =
                        r.render_image(gpu, &game.resman, scene, &cam, client.width, client.height);
                    let g = grey(&img);
                    let (mut err, mut n) = (0.0, 0.0);
                    for y in (h / 10..h * 7 / 10).step_by(4) {
                        for x in (w / 10..w * 8 / 10).step_by(4) {
                            err += (g[y * w + x] - cg[y * w + x]).abs();
                            n += 1.0;
                        }
                    }
                    if err / n < best.0 {
                        best = (err / n, p);
                    }
                }
            }
        }
    }
    eprintln!("best fit: error {:.1}: fov, focus, distance, pitch {:?}", best.0, best.1);
}

#[test]
#[ignore]
fn reference_scenes_match_the_client() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let scenes = [
        ("sun", Lighting { ambient: 0x404040, diffuse: 0xB0B0B0, main_light: 0, only_tile: None }),
        ("tile lights", Lighting { ambient: 0, diffuse: 0, main_light: 2, only_tile: None }),
    ];
    let mut failures = Vec::new();
    for (name, light) in scenes {
        let dir = scratch_dir(&format!("client_render_{}", name.replace(' ', "_")));
        let (mut m, are) = build_module(&game, &dir, light);
        std::fs::create_dir_all(dir.join("user")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let Some(client) = client_screenshot(&dir, "MgScene") else {
            panic!("{name}: the client did not reach the scene (see {})", dir.display());
        };
        let (scene, camera) = moonglow_scene(&gpu, &game, &are, light);
        if std::env::var_os("MG_FIT").is_some() {
            fit_camera(&mut r, &gpu, &game, &scene, &client);
        }
        // Moonglow's own shader (the Shaders switch off) and the game's
        // (on), each against the client.
        for on in [false, true] {
            r.game_shaders = on;
            let ours =
                r.render_image(&gpu, &game.resman, &scene, &camera, client.width, client.height);
            assert_eq!(r.game_shader_failures(), &[], "{name}");
            let switch = if on { "on" } else { "off" };
            save_png(&ours, &dir.join(format!("moonglow_{switch}.png")));
            let mut worst = 0f32;
            for (region, rect, tolerance) in REGIONS {
                let (a, b) = (mean(&client, rect), mean(&ours, rect));
                let diff: Vec<f32> = (0..3).map(|c| b[c] - a[c]).collect();
                worst = diff.iter().fold(worst, |w, d| w.max(d.abs()));
                eprintln!(
                    "{name} ({switch}): {region:12} client {a:5.1?} moonglow {b:5.1?} diff {diff:5.1?}"
                );
                if diff.iter().any(|d| d.abs() > tolerance) {
                    failures.push(format!("{name} ({switch}): {region} differs by {diff:?}"));
                }
            }
            // The whole picture below the game's GUI: the mean difference.
            let (mut sum, mut n) = (0f64, 0f64);
            let (w, h) = (client.width as usize, client.height as usize);
            for y in h / 10..h * 7 / 10 {
                for x in w / 10..w * 9 / 10 {
                    let i = (y * w + x) * 4;
                    sum += (0..3)
                        .map(|c| f64::from(client.data[i + c].abs_diff(ours.data[i + c])))
                        .sum::<f64>()
                        / 3.0;
                    n += 1.0;
                }
            }
            eprintln!(
                "{name} ({switch}): regions within {worst:.1} of 255; the picture {:.2} of 255 on average",
                sum / n
            );
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Fragment code added to the game's own `inc_standard.shd` (patched at test
/// time, in the scratch user directory's `override`): lit fragments in a
/// band of 32×32 blocks (rows 6–9 from the bottom of the screen) show one
/// uniform each, as `floor(x·255)/255` in red and the remainder in green.
const DEBUG_FUNCTIONS: &str = r#"
#if SHADER_TYPE == 2 && LIGHTING == 1 && GAMMA_CORRECTION == 1 && (FRAGMENT_LIGHTING == 1 || FRAGMENT_NORMAL == 1)
float MgValue(int k)
{
    if (k == 0) return 0.5;
    if (k == 1) return float(numLights) / 64.0;
    if (k == 2) return lightMaxIntensityInv / 4.0;
    if (k == 3) return lightFalloffFactor / 4096.0;
    if (k >= 4 && k < 7) return lightAreaAmbient[k - 4] / 4.0;
    if (k >= 7 && k < 10) return lightAreaDiffuse[k - 7] / 4.0;
#if FOG == 1
    if (k == 10) return fogParams.x;
    if (k == 11) return fogParams.y / 1024.0;
    if (k == 12) return fogParams.z / 1024.0;
    if (k == 13) return fogParams.w;
    if (k >= 14 && k < 17) return fogColor[k - 14];
#endif
    // The time uniforms' bytes, and the module's clock.
    if (k >= 17 && k < 21) return (float((worldtimerTimeOfDay >> (8 * (k - 17))) & 255) + 0.5) / 256.0;
    if (k >= 21 && k < 25) return (float((sceneCurrentFrame >> (8 * (k - 21))) & 255) + 0.5) / 256.0;
    if (k == 25) return (float(moduleHour) + 0.5) / 256.0;
    if (k == 26) return (float(moduleMinutesPerHour) + 0.5) / 256.0;
    if (k == 27) return (float(moduleDawnHour) + 0.5) / 256.0;
    if (k == 28) return (float(moduleDuskHour) + 0.5) / 256.0;
    if (k == 29) return (float(moduleDay) + 0.5) / 256.0;
    if (k == 30) return (float(moduleMonth) + 0.5) / 256.0;
    // How many of the draw's lights have no green (which of several were
    // kept).
    if (k == 31)
    {
        int n = 0;
        for (int i = 0; i < numLights; i++) if (lightColor[i].g < 0.001) n++;
        return float(n) / 64.0;
    }
    if (k >= 32 && k < 32 + 8 * 8)
    {
        int i = (k - 32) / 8;
        int j = k - 32 - i * 8;
        if (i >= numLights) return 0.0;
        vec4 c = lightColor[i];
        if (j < 3) return c[j] / 8.0;
        if (j == 3) return sqrt(abs(c.a)) / 256.0;
        if (j == 4) return c.a < 0.0 ? 1.0 : 0.0;
        return 0.0;
    }
    return 0.0;
}
void MgDebug(inout vec4 c)
{
    int row = int(gl_FragCoord.y / 32.0) - 6;
    if (row < 0 || row >= 4) return;
    float x = clamp(MgValue(row * 32 + int(gl_FragCoord.x / 32.0)), 0.0, 0.99999);
    float hi = floor(x * 255.0);
    c = vec4(hi / 255.0, fract(x * 255.0), 0.0, 1.0);
}
#endif
"#;

/// The game's `inc_standard.shd` with [`DEBUG_FUNCTIONS`] applied after
/// everything else.
fn debug_shader(game: &GameData) -> String {
    let src = game.resman.get_named("inc_standard", ResType::SHD).expect("inc_standard.shd");
    let mut src = String::from_utf8_lossy(&src).replace("\r\n", "\n");
    // The fragment shader's ApplyStandardShader is the last one.
    let at = src.rfind("void ApplyStandardShader()").expect("ApplyStandardShader");
    src.insert_str(at, &format!("{DEBUG_FUNCTIONS}\n"));
    let end = "\tApplyDebugModeOutput(FragmentColor);\n}";
    let at = src.rfind(end).expect("ApplyDebugModeOutput");
    src.insert_str(
        at + end.len() - 1,
        "#if SHADER_TYPE == 2 && LIGHTING == 1 && GAMMA_CORRECTION == 1 && \
         (FRAGMENT_LIGHTING == 1 || FRAGMENT_NORMAL == 1)\n\tMgDebug(FragmentColor);\n#endif\n",
    );
    src
}

/// Uniform `k` shown by [`DEBUG_FUNCTIONS`], unscaled by `scale`.
fn debug_value(img: &Rgba, k: usize, scale: f32) -> f32 {
    let (row, col) = (k / 32, k % 32);
    let y = img.height as usize - 1 - ((row + 6) * 32 + 16);
    let i = (y * img.width as usize + col * 32 + 16) * 4;
    (img.data[i] as f32 / 255.0 + img.data[i + 1] as f32 / 65025.0) * scale
}

/// An int the debug shader shows in four blocks from `k`, a byte each.
fn debug_int(img: &Rgba, k: usize) -> i64 {
    let byte = |j: usize| (debug_value(img, k + j, 256.0) - 0.5).round() as i64;
    let v = byte(0) | byte(1) << 8 | byte(2) << 16 | byte(3) << 24;
    // (Two's complement.)
    if v >= 1 << 31 { v - (1 << 32) } else { v }
}

/// A byte the debug shader shows in block `k`.
fn debug_byte(img: &Rgba, k: usize) -> i64 {
    (debug_value(img, k, 256.0) - 0.5).round() as i64
}

/// The entry script with the module's clock written to the log at the
/// start and every five seconds after.
const TIME_ENTER: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
void Tick()
{
    Log("MG_TIME " + IntToString(GetTimeHour()) + ":" + IntToString(GetTimeMinute()) + ":"
        + IntToString(GetTimeSecond()) + "." + IntToString(GetTimeMillisecond()));
    DelayCommand(5.0, Tick());
}
void main()
{
    object pc = GetEnteringObject();
    if (!GetIsPC(pc)) return;
    location start = GetStartingLocation();
    ApplyEffectToObject(DURATION_TYPE_PERMANENT, EffectVisualEffect(VFX_DUR_CUTSCENE_INVISIBILITY), pc);
    AssignCommand(pc, SetCameraFacing(90.0, 12.0, 45.0, CAMERA_TRANSITION_TYPE_SNAP));
    LockCameraDirection(pc, TRUE);
    LockCameraPitch(pc, TRUE);
    LockCameraDistance(pc, TRUE);
    Tick();
    DelayCommand(4.0, Log("MG_READY"));
}
"#;

/// The time uniforms' units (`docs/research/notes_shaders.md` §B.4):
/// `worldtimerTimeOfDay` and `sceneCurrentFrame` read back through the
/// debug shader in five screenshots 3 seconds apart (each screenshot's
/// time measured), beside the module's clock, which the entry script logs,
/// in modules starting at other hours and days and with other minutes an
/// hour. Seen on 2026-10-21 (89.8193.37-17): the timer counts real
/// milliseconds (999-1000 a second, at 2 and at 10 minutes an hour), the
/// frame counter the client's frames (54-55 a second here, about 370 some
/// 7 seconds after entering the area). The timer's value follows neither
/// the module's hour nor its day: about 424,800 there at 2 minutes an
/// hour, 3,304,900 at 10 (six game hours' worth more), its origin unknown. Needs the game, a GPU and the off-screen display;
/// run by hand (`--ignored`).
#[test]
#[ignore]
fn time_uniforms_in_the_client() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let shader = debug_shader(&game);
    // (Modules starting at other hours and days, with another number of
    // minutes an hour: what the timer depends on.) (start hour, minutes an
    // hour, start day)
    for (hour, minutes, day) in [(13u8, 2u8, 1u8), (6, 2, 1), (13, 10, 1), (13, 2, 15)] {
        let dir = scratch_dir(&format!("client_time_{hour}_{minutes}_{day}"));
        let light =
            Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
        let (mut m, _) = build_module(&game, &dir, light);
        m.set(
            ResKey::parse("mg_enter", ResType::NCS).unwrap(),
            compile(&dir, "mg_enter", TIME_ENTER),
        );
        let mut info = m.info().unwrap();
        info.root.write(&ifo::MOD_START_HOUR, hour);
        info.root.write(&ifo::MOD_MIN_PER_HOUR, minutes);
        info.root.write(&ifo::MOD_START_DAY, day);
        m.set_info(&info).unwrap();
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        std::fs::write(dir.join("user/override/inc_standard.shd"), &shader).unwrap();
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let shots = common::client_screenshots(&dir, "MgScene", 5, Duration::from_secs(3))
            .unwrap_or_else(|| {
                panic!("the client did not reach the scene (see {})", dir.display())
            });
        let log =
            std::fs::read_to_string(dir.join("user/logs/nwclientLog1.txt")).unwrap_or_default();
        for line in log.lines().filter(|l| l.contains("MG_TIME")) {
            eprintln!("{hour}h {minutes}min/h day {day}: {line}");
        }
        let mut rows: Vec<(f64, i64, i64)> = Vec::new();
        for (i, (at, img)) in shots.iter().enumerate() {
            save_png(img, &dir.join(format!("shot_{i}.png")));
            let (timer, frame) = (debug_int(img, 17), debug_int(img, 21));
            eprintln!(
                "{hour}h {minutes}min/h day {day}, shot {i} at {at:6.3} s: worldtimerTimeOfDay {timer}, \
                 sceneCurrentFrame {frame}, moduleHour {}, moduleMinutesPerHour {}, dawn {}, \
                 dusk {}, day {}, month {}",
                debug_byte(img, 25),
                debug_byte(img, 26),
                debug_byte(img, 27),
                debug_byte(img, 28),
                debug_byte(img, 29),
                debug_byte(img, 30),
            );
            // (A screenshot of a window the client hadn't drawn anew since
            // the last is the last one again: left out.)
            if rows.last().is_none_or(|r| r.2 != frame) {
                rows.push((*at, timer, frame));
            }
        }
        let (first, last) = (rows[0], rows[rows.len() - 1]);
        let dt = last.0 - first.0;
        let (timer_rate, frame_rate) =
            ((last.1 - first.1) as f64 / dt, (last.2 - first.2) as f64 / dt);
        eprintln!(
            "{hour}h {minutes}min/h day {day}: over {dt:.3} s worldtimerTimeOfDay {timer_rate:.1} a second, \
             sceneCurrentFrame {frame_rate:.1} a second ({} shots)",
            rows.len()
        );
        assert!(rows.len() >= 2 && dt > 5.0, "{rows:?}");
        // Milliseconds of real time, whatever the module's minutes per hour.
        assert!((timer_rate - 1000.0).abs() < 50.0, "{timer_rate}");
        // A frame count (as fast as the client draws).
        assert!(frame_rate > 10.0, "{frame_rate}");
    }
}

/// L5: the client's light uniforms (read back through a debug shader) match
/// what Moonglow uploads: the area's sun colours, the attenuation constants,
/// and a tile light's colour and cutoff, from lightcolor.2da through the
/// tile model's main light nodes (whose radius the engine replaces) or a
/// source light (fx_flame01, coloured by its animation).
#[test]
#[ignore]
fn light_uniforms_match_the_client() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let rm = &game.resman;
    let shader = debug_shader(&game);
    let load =
        |name: &str| rm.get_named(name, ResType::MDL).ok().and_then(|d| Model::read(&d).ok());
    let colors = game.table("lightcolor").unwrap();
    let color = |row: u8| -> Option<Vec3> {
        let c = |col| colors.get(row as usize, col).and_then(|v| v.parse::<f32>().ok());
        Some(Vec3::new(c("RED")?, c("GREEN")?, c("BLUE")?))
    };
    // Tile (2, 1) of the scene; its model has both main lights and source
    // lights.
    let tile = 6;
    let flame = Arc::new(load("fx_flame01").unwrap());
    // (name, sun ambient and diffuse, main light 1, main light 2, source
    // light 1)
    let cases = [
        ("sun", (0x203040, 0xB0A090), 0u8, 0u8, 0u8),
        ("white", (0, 0), 2, 0, 0),
        ("yellow", (0, 0), 7, 0, 0),
        ("dim white", (0, 0), 1, 0, 0),
        ("main light 2", (0, 0), 0, 2, 0),
        ("source light", (0, 0), 0, 0, 1),
        ("blue source light", (0, 0), 0, 0, 8),
    ];
    let mut failures = Vec::new();
    for (name, (ambient, diffuse), ml1, ml2, sl1) in cases {
        let dir = scratch_dir(&format!("client_uniforms_{}", name.replace(' ', "_")));
        let light = Lighting { ambient, diffuse, main_light: 0, only_tile: Some(tile) };
        let (mut m, mut are) = build_module(&game, &dir, light);
        let t = &mut are.root.list_mut("Tile_List").unwrap()[tile];
        t.set("Tile_MainLight1", Value::Byte(ml1));
        t.set("Tile_MainLight2", Value::Byte(ml2));
        t.set("Tile_SrcLight1", Value::Byte(sl1));
        let model_name = {
            let set = mg_set::Tileset::parse(
                &rm.get_named("tic01", ResType::SET).unwrap(),
                game.language.codepage(),
            )
            .unwrap();
            set.tiles[t.int("Tile_ID").unwrap() as usize].model.to_ascii_lowercase()
        };
        let area =
            m.info().unwrap().root.list("Mod_Area_list").unwrap()[0].resref("Area_Name").unwrap();
        m.set_gff(ResKey::new(area, ResType::ARE), &are).unwrap();
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        std::fs::write(dir.join("user/override/inc_standard.shd"), &shader).unwrap();
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let Some(client) = client_screenshot(&dir, "MgScene") else {
            panic!("{name}: the client did not reach the scene (see {})", dir.display());
        };
        // Moonglow's lights for the same settings.
        let sun = AreaLight::from_are(ambient, diffuse, Vec3::Z);
        let ours = if ml1 == 0 && ml2 == 0 && sl1 == 0 {
            Vec::new()
        } else if sl1 == 0 {
            let model = load(&model_name).unwrap();
            let pose = mg_render::rest_pose(&model);
            mg_render::anim::lights(&model, None, 0.0, &pose, Mat4::IDENTITY, &|slot| {
                color(if slot == 0 { ml1 } else { ml2 })
            })
        } else {
            let anim = flame.animations.iter().find(|a| a.name == sl1.to_string()).unwrap();
            let pose = mg_render::anim::pose(&flame, anim, 0.0);
            mg_render::anim::lights(&flame, Some(anim), 0.0, &pose, Mat4::IDENTITY, &|_| None)
        };
        let (max_inv, falloff) = mg_render::attenuation_params();
        let mut checks = vec![
            ("light count", debug_value(&client, 1, 64.0), ours.len() as f32, 0.01),
            ("1 / max intensity", debug_value(&client, 2, 4.0), max_inv, 0.002),
            ("falloff factor", debug_value(&client, 3, 4096.0), falloff, 0.2),
        ];
        for (c, what) in ["ambient red", "ambient green", "ambient blue"].into_iter().enumerate() {
            checks.push((what, debug_value(&client, 4 + c, 4.0), sun.ambient[c], 0.002));
        }
        for (c, what) in ["diffuse red", "diffuse green", "diffuse blue"].into_iter().enumerate() {
            checks.push((what, debug_value(&client, 7 + c, 4.0), sun.diffuse[c], 0.002));
        }
        if let Some(l) = ours.first() {
            checks.extend([
                ("red", debug_value(&client, 32, 8.0), l.color.x, 0.002),
                ("green", debug_value(&client, 33, 8.0), l.color.y, 0.002),
                ("blue", debug_value(&client, 34, 8.0), l.color.z, 0.002),
                ("cutoff", debug_value(&client, 35, 256.0), l.cutoff, 0.02),
                ("ambient only", debug_value(&client, 36, 1.0), l.ambient_only as u8 as f32, 0.01),
            ]);
        }
        for (what, client, moonglow, tolerance) in checks {
            eprintln!("{name}: {what:18} client {client:9.4} moonglow {moonglow:9.4}");
            if (client - moonglow).abs() > tolerance {
                failures.push(format!("{name}: {what}: client {client}, moonglow {moonglow}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The light a placeable's appearance carries (placeables.2da's
/// `LightColor`), as the client hands it to its shader in a dark area, is
/// the light Moonglow makes of that color with a radius of 10: one light,
/// the color linearised, ending 20 m out (further for a color brighter
/// than 1), whether the placeable is static or not and at any height:
/// for the game's invisible "Light, White" static and not, raised and on
/// the ground, a colored one and a Shaft of Light. `MG_LIGHT_ROWS` gives
/// other rows (comma-separated).
#[test]
#[ignore]
fn placeable_light_uniforms() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let shader = debug_shader(&game);
    let colors = game.table("lightcolor").unwrap();
    let table = game.table("placeables").unwrap();
    // (row, static, height)
    let mut cases: Vec<(usize, bool, f32)> = vec![
        (15112, true, 1.5),
        (15112, false, 1.5),
        (15112, true, 0.0),
        (15111, true, 1.5),
        (166, true, 0.0),
    ];
    if let Ok(rows) = std::env::var("MG_LIGHT_ROWS") {
        cases = rows.split(',').map(|r| (r.trim().parse().unwrap(), true, 1.5)).collect();
    }
    for (row, is_static, height) in cases {
        let dir = scratch_dir(&format!("client_plc_light_{row}_{}_{height}", u8::from(is_static)));
        let light = Lighting { ambient: 0, diffuse: 0, main_light: 0, only_tile: None };
        let (mut m, _are) = build_module(&game, &dir, light);
        let enter: String =
            ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
        m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
        let git_key = *m.keys_of(ResType::GIT).next().unwrap();
        let mut git = m.gff(&git_key).unwrap().unwrap();
        let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Light", 0);
        bp.root.set("Appearance", Value::Dword(row as u32));
        bp.root.set("Static", Value::Byte(u8::from(is_static)));
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        // (Just north of the player, over the floor the camera looks at.)
        let at = Placement { position: [20.0, 21.0, height], rotation: 0.0 };
        let placed = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
        git.root.set("Placeable List", Value::List(vec![placed]));
        m.set_gff(git_key, &git).unwrap();
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        std::fs::write(dir.join("user/override/inc_standard.shd"), &shader).unwrap();
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let Some(client) = client_screenshot(&dir, "MgScene") else {
            eprintln!("row {row}: the client did not reach the scene");
            continue;
        };
        let label = table.get(row, "Label").unwrap_or("?");
        let c = table.get_int(row, "LightColor").unwrap_or(0) as usize;
        let of = |col: &str| colors.get_float(c, col).unwrap_or(0.0);
        eprintln!(
            "row {row} {label} static {is_static} height {height}: lightcolor row {c} \
             ({:.2}, {:.2}, {:.2}); client: lights {:.0}, color ({:.3}, {:.3}, {:.3}), \
             cutoff {:.2}, ambient only {:.0}; second light color ({:.3}, {:.3}, {:.3}) \
             cutoff {:.2}",
            of("RED"),
            of("GREEN"),
            of("BLUE"),
            debug_value(&client, 1, 64.0),
            debug_value(&client, 32, 8.0),
            debug_value(&client, 33, 8.0),
            debug_value(&client, 34, 8.0),
            debug_value(&client, 35, 256.0),
            debug_value(&client, 36, 1.0),
            debug_value(&client, 40, 8.0),
            debug_value(&client, 41, 8.0),
            debug_value(&client, 42, 8.0),
            debug_value(&client, 43, 256.0),
        );
        // As Moonglow makes a light of that color with a radius of 10.
        let ours = mg_render::PointLight::new(
            Vec3::ZERO,
            Vec3::new(of("RED"), of("GREEN"), of("BLUE")),
            10.0,
            false,
            4,
        );
        let what = format!("row {row} static {is_static} height {height}");
        assert!((debug_value(&client, 1, 64.0) - 1.0).abs() < 0.01, "{what}: one light");
        assert!((debug_value(&client, 35, 256.0) - ours.cutoff).abs() < 0.05, "{what}: cutoff");
        for (k, channel) in ours.color.to_array().into_iter().enumerate() {
            let theirs = debug_value(&client, 32 + k, 8.0);
            assert!((theirs - channel).abs() < 0.005, "{what}: color {k}: {theirs} {channel}");
        }
    }
}

/// L5: the client's fog uniforms (read back through the debug shader) are
/// Moonglow's fog: the end at the fog clip distance, the start the fog
/// amount nearer than 30 m (at most 1 m before the end), with or without a
/// skybox. `MG_FOG_CASES=amount:clip,...` measures others.
#[test]
#[ignore]
fn fog_uniforms_match_the_client() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let shader = debug_shader(&game);
    let cases: Vec<(u8, f32, u8)> = std::env::var("MG_FOG_CASES")
        .ok()
        .map(|v| {
            v.split(',')
                .map(|c| {
                    let (a, d) = c.split_once(':').unwrap();
                    (a.parse().unwrap(), d.parse().unwrap(), 0)
                })
                .collect()
        })
        .unwrap_or_else(|| vec![(0, 45.0, 0), (10, 45.0, 0), (50, 100.0, 1), (0, 25.0, 0)]);
    let mut failures = Vec::new();
    for (amount, clip, sky) in cases {
        let dir = scratch_dir(&format!("client_fog_{amount}_{clip}"));
        let light =
            Lighting { ambient: 0x404040, diffuse: 0xB0B0B0, main_light: 0, only_tile: None };
        let (mut m, mut are) = build_module(&game, &dir, light);
        are.root.set("SunFogAmount", Value::Byte(amount));
        are.root.set("SunFogColor", Value::Dword(0x806040));
        are.root.set("FogClipDist", Value::Float(clip));
        are.root.set("SkyBox", Value::Byte(sky));
        let area =
            m.info().unwrap().root.list("Mod_Area_list").unwrap()[0].resref("Area_Name").unwrap();
        m.set_gff(ResKey::new(area, ResType::ARE), &are).unwrap();
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        std::fs::write(dir.join("user/override/inc_standard.shd"), &shader).unwrap();
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let Some(client) = client_screenshot(&dir, "MgScene") else {
            panic!("fog {amount}: the client did not reach the scene (see {})", dir.display());
        };
        // The start from 1 / (end - start): the read-back clamps below 0.
        let end = debug_value(&client, 12, 1024.0);
        let start = end - 1.0 / debug_value(&client, 13, 1.0);
        let color = [14, 15, 16].map(|k| debug_value(&client, k, 1.0));
        let lighting = mg_area::Lighting::read(&are.root, None);
        let ours = mg_area::fog(&lighting, false);
        eprintln!(
            "fog {amount} clip {clip} sky {sky}: client {start:.2}..{end:.2}, \
             moonglow {:.2}..{:.2}",
            ours.start, ours.end
        );
        let close = |a: f32, b: f32, tolerance: f32| (a - b).abs() <= tolerance;
        if debug_value(&client, 10, 1.0) < 0.5 {
            failures.push(format!("fog {amount}: the client has fog off"));
        }
        if !close(start, ours.start, 0.3) || !close(end, ours.end, 0.1) {
            failures.push(format!(
                "fog {amount} clip {clip}: client {start}..{end}, moonglow {}..{}",
                ours.start, ours.end
            ));
        }
        if (0..3).any(|c| !close(color[c], ours.color[c], 0.003)) {
            failures.push(format!("fog {amount}: colour {color:?} vs {}", ours.color));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// A rural field (ttr01) with its grass and the Grass_Clear skybox, as the
/// client and Moonglow draw it, side by side for a look (written to the
/// scratch directory: `client.png`, `moonglow.png`); no comparison.
#[test]
#[ignore]
fn grass_and_sky_look() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let light = Lighting { ambient: 0x404040, diffuse: 0xB0B0B0, main_light: 0, only_tile: None };
    let dir = scratch_dir("client_render_grass");
    let (mut m, are) = build_module_on(&game, &dir, light, "ttr01", 1);
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let settings = SETTINGS
        .replace("[graphics.grass]\n\t\tmode = 0", "[graphics.grass]\n\t\tmode = 2")
        .replace(
            "[graphics.skyboxes]\n\t\tenabled = false",
            "[graphics.skyboxes]\n\t\tenabled = true",
        );
    assert!(settings.contains("mode = 2") && settings.contains("enabled = true"));
    std::fs::write(dir.join("user/settings.tml"), settings).unwrap();
    // MG_CLOSE=1: the camera low and close, to see the client's grass.
    if std::env::var_os("MG_CLOSE").is_some() {
        let enter =
            ENTER.replace("SetCameraFacing(90.0, 12.0, 45.0", "SetCameraFacing(90.0, 3.0, 80.0");
        let ncs = compile(&dir, "mg_enter", &enter);
        m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), ncs);
    }
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = client_screenshot(&dir, "MgScene");
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let git = m.gff(&git_key).unwrap().unwrap();
    let tileset = mg_area::tileset(&game, ResRef::from_str("ttr01").unwrap()).ok();
    let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let (w, h) = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    off_and_on(&gpu, &game, &scene, &camera, (w, h), client.as_ref(), &dir, "moonglow");
    eprintln!("{}: client {}", dir.display(), client.is_some());
}

/// One grass override for [`grass_overrides_look`], as Moonglow writes it.
fn grass_override(
    material: u32,
    texture: &str,
    density: f32,
    height: f32,
    color: [f32; 3],
) -> mg_gff::Struct {
    let look = mg_area::grass::GrassLook {
        texture: texture.into(),
        density,
        height,
        ambient: color,
        diffuse: color,
    };
    mg_area::grass::GrassOverride { material, look }.item()
}

/// Grass overrides as the client and Moonglow draw them, on a rural field
/// (ttr01) under the fitted camera, side by side for a look; no comparison.
/// `MG_GRASS` names the variants (comma-separated): `default`, `off` (the
/// tileset's grass disabled), `dense` and `sparse` (density 20 and 1 on
/// grass, material 3), `tall` (height 3), `red` (colors 1, 0, 0), `dirt`
/// (grass on dirt too, material 1). Each one's pictures:
/// `target/test-output/client_grass_<variant>/` `client.png` and
/// `moonglow.png`. `MG_DISTANCE` and `MG_PITCH` move the client's camera
/// (then Moonglow's is not fitted to it).
#[test]
#[ignore]
fn grass_overrides_look() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let light = Lighting { ambient: 0x404040, diffuse: 0xB0B0B0, main_light: 0, only_tile: None };
    let variants = std::env::var("MG_GRASS").unwrap_or_else(|_| "default".into());
    let pitch = std::env::var("MG_PITCH").unwrap_or_else(|_| "45.0".into());
    let distance = std::env::var("MG_DISTANCE").unwrap_or_else(|_| "12.0".into());
    let skip_client = std::env::var_os("MG_NO_CLIENT").is_some();
    for variant in variants.split(',') {
        let dir = scratch_dir(&format!("client_grass_{variant}"));
        let (mut m, are) = build_module_on(&game, &dir, light, "ttr01", 0);
        let git_key = *m.keys_of(ResType::GIT).next().unwrap();
        let mut git = m.gff(&git_key).unwrap().unwrap();
        let green = [0.5, 0.6, 0.3];
        let list = match variant {
            "dense" => vec![grass_override(3, "grass", 20.0, 1.2, green)],
            "sparse" => vec![grass_override(3, "grass", 1.0, 1.2, green)],
            "tall" => vec![grass_override(3, "grass", 5.0, 3.0, green)],
            "red" => vec![grass_override(3, "grass", 5.0, 1.2, [1.0, 0.0, 0.0])],
            "dirt" => vec![grass_override(1, "grass", 5.0, 1.2, [0.2, 0.2, 1.0])],
            _ => Vec::new(),
        };
        let Some(Value::Struct(props)) = git.root.get_mut("AreaProperties") else { panic!() };
        if variant == "off" {
            props.set("GrassDefDisabled", Value::Byte(1));
        }
        if !list.is_empty() {
            let mut ovr = mg_gff::Struct::new(101);
            ovr.set("GrassOvrList", Value::List(list));
            props.set("GrassOverrides", Value::Struct(ovr));
        }
        m.set_gff(git_key, &git).unwrap();
        std::fs::create_dir_all(dir.join("user")).unwrap();
        let settings =
            SETTINGS.replace("[graphics.grass]\n\t\tmode = 0", "[graphics.grass]\n\t\tmode = 2");
        std::fs::write(dir.join("user/settings.tml"), settings).unwrap();
        let camera = format!("SetCameraFacing(90.0, {distance}, {pitch}");
        let enter = ENTER
            .replace("SetCameraFacing(90.0, 12.0, 45.0", &camera)
            .replace("CreateObject(", "if (FALSE) CreateObject(");
        let ncs = compile(&dir, "mg_enter", &enter);
        m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), ncs);
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let client = if skip_client { None } else { client_screenshot(&dir, "MgScene") };
        let tileset = mg_area::tileset(&game, ResRef::from_str("ttr01").unwrap()).ok();
        let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
        let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
        let camera =
            fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
        let (w, h) = client.as_ref().map_or((1024, 768), |c| (c.width, c.height));
        let sight = mg_area::Sight::of(&camera, w as f32 / h as f32);
        let view = mg_area::View { fog: false, grass: Some(sight), ..mg_area::View::of(&model) };
        let scene = area_scene.scene(&model, &view);
        off_and_on(&gpu, &game, &scene, &camera, (w, h), client.as_ref(), &dir, "moonglow");
        eprintln!("{variant}: {} (client {})", dir.display(), client.is_some());
    }
}

/// An emitter for [`particles_look`]: at (x, y, z) of the probe model, its
/// particles 12 cm white squares rising 1 m/s for 3 s unless `extra` says
/// otherwise (later keywords win).
fn probe_emitter(name: &str, at: [f32; 3], extra: &str) -> String {
    format!(
        "node emitter {name}\n  parent plc_a01\n  position {} {} {}\n  orientation 0 0 0 0\n\
         update Fountain\n  render Normal\n  blend Normal\n  texture mgwhite\n  loop 1\n\
         xgrid 1\n  ygrid 1\n  spawntype 0\n  birthrate 40\n  lifeExp 3\n  velocity 1\n\
         randvel 0\n  spread 0\n  mass 0\n  particleRot 0\n  sizeStart 0.12\n  sizeEnd 0.12\n\
         xsize 0\n  ysize 0\n  alphaStart 1\n  alphaEnd 1\n  colorStart 1 1 1\n  colorEnd 1 1 1\n\
         {extra}\nendnode\n",
        at[0], at[1], at[2]
    )
}

/// A white 8×8 texture.
fn white_tga() -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 32, 8];
    t.extend([255u8; 8 * 8 * 4]);
    t
}

/// A white 64×64 texture whose alpha rises evenly from 0 at its left edge
/// to 1 at its right.
fn ramp_tga() -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 64, 0, 32, 8];
    for _ in 0..64 {
        for x in 0..64u32 {
            t.extend([255, 255, 255, (x * 255 / 63) as u8]);
        }
    }
    t
}

/// Exploration: particles in the client. The armoire's model (`plc_a01`,
/// overridden in the scratch user directory) holds probe emitters, under a
/// red sun and green ambient light; the client's view is written to
/// `target/test-output/client_particles_<set>/client.png`. `MG_PARTICLES`
/// picks the set: `stops` (default: colour, alpha and size stops, tinting),
/// `motion` (bounce, wind), `inherit` (weight on particles an emitter keeps)
/// and the others below.
#[test]
#[ignore]
fn particles_look() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let set = std::env::var("MG_PARTICLES").unwrap_or_else(|_| "stops".into());
    let light = match set.as_str() {
        "stops2" => {
            Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None }
        }
        "tilelights" => Lighting { ambient: 0, diffuse: 0, main_light: 2, only_tile: None },
        _ => Lighting { ambient: 0x004000, diffuse: 0x0000C0, main_light: 0, only_tile: None },
    };
    let dir = scratch_dir(&format!("client_particles_{set}"));
    let (mut m, _) = build_module(&game, &dir, light);
    let column = |i: usize| -2.4 + 0.6 * i as f32;
    let emitters: Vec<String> = match set.as_str() {
        "motion" => vec![
            // Thrown sideways from 1.5 m: a trajectory per emitter.
            probe_emitter(
                "bounce",
                [-2.0, 0.0, 1.5],
                "orientation 0 1 0 1.5707964\n  mass 1\n  lifeExp 4\n  bounce 1\n  bounce_co 0.5",
            ),
            probe_emitter(
                "nobounce",
                [-2.0, 1.0, 1.5],
                "orientation 0 1 0 1.5707964\n  mass 1\n  lifeExp 4\n  bounce_co 0.5",
            ),
            probe_emitter("wind", [1.5, 0.0, 0.0], "affectedByWind 1"),
            probe_emitter("calm", [2.1, 0.0, 0.0], ""),
        ],
        "physics" | "bounce" => {
            // Green dots marking the plane of the throws (model x, z).
            let mut v: Vec<String> = [
                (0.0, 0.0),
                (-1.0, 0.0),
                (-2.0, 0.0),
                (-3.0, 0.0),
                (0.0, 1.5),
                (-1.0, 1.5),
                (-2.0, 1.5),
                (-3.0, 3.0),
                (0.0, 3.0),
            ]
            .iter()
            .enumerate()
            .map(|(i, (x, z))| {
                probe_emitter(
                    &format!("dot{i}"),
                    [*x, 0.0, *z],
                    "velocity 0\n  birthrate 4\n  lifeExp 2\n  sizeStart 0.08\n  sizeEnd 0.08\n  \
                         colorStart 0 1 0\n  colorEnd 0 1 0",
                )
            })
            .collect();
            let throw =
                "orientation 0 1 0 -1.5707964\n  lifeExp 4\n  sizeStart 0.06\n  sizeEnd 0.06";
            if set == "physics" {
                v.push(probe_emitter(
                    "mass1",
                    [0.0, 0.0, 1.5],
                    &format!("{throw}\n  mass 1\n  colorStart 1 0 0\n  colorEnd 1 0 0"),
                ));
                v.push(probe_emitter(
                    "mass05",
                    [0.0, 0.0, 3.0],
                    &format!("{throw}\n  mass 0.5\n  colorStart 0 0 1\n  colorEnd 0 0 1"),
                ));
            } else {
                v.push(probe_emitter("co05", [0.0, 0.0, 1.5], &format!("{throw}\n  mass 1\n  bounce 1\n  bounce_co 0.5\n  colorStart 1 0 0\n  colorEnd 1 0 0")));
                v.push(probe_emitter("co1", [0.0, 0.0, 3.0], &format!("{throw}\n  mass 1\n  bounce 1\n  bounce_co 1\n  colorStart 0 0 1\n  colorEnd 0 0 1")));
            }
            v
        }
        "inherit" => {
            // Thrown sideways with weight, 1.5 m apart in height: the red
            // one keeps its particles in its own space (`inherit`), the
            // blue one does not. Both fall the same way: down is the
            // world's, not the emitter's axis. Green dots mark the plane.
            let mut v: Vec<String> = [(0.0, 0.0), (-1.0, 0.0), (-2.0, 0.0), (0.0, 1.5), (0.0, 3.0)]
                .iter()
                .enumerate()
                .map(|(i, (x, z))| {
                    probe_emitter(
                        &format!("dot{i}"),
                        [*x, 0.0, *z],
                        "velocity 0\n  birthrate 4\n  lifeExp 2\n  sizeStart 0.08\n  \
                         sizeEnd 0.08\n  colorStart 0 1 0\n  colorEnd 0 1 0",
                    )
                })
                .collect();
            let throw = "orientation 0 1 0 -1.5707964\n  lifeExp 4\n  sizeStart 0.06\n  \
                         sizeEnd 0.06\n  mass 0.5";
            v.push(probe_emitter(
                "kept",
                [0.0, 0.0, 3.0],
                &format!("{throw}\n  inherit 1\n  colorStart 1 0 0\n  colorEnd 1 0 0"),
            ));
            v.push(probe_emitter(
                "free",
                [0.0, 0.0, 1.5],
                &format!("{throw}\n  colorStart 0 0 1\n  colorEnd 0 0 1"),
            ));
            v
        }
        "punch" => {
            // One large particle each, still, showing a white texture
            // whose alpha rises from nothing at its left to solid at its
            // right: `Punch-Through` on the left, `Normal` on the right.
            // How much of the left one's width is there is where it is cut.
            let one = "birthrate 1\n  lifeExp 30\n  velocity 0\n  sizeStart 1\n  sizeEnd 1\n  \
                       texture mgramp";
            vec![
                probe_emitter(
                    "punch",
                    [column(2), 0.0, 1.5],
                    &format!("{one}\n  blend Punch-Through"),
                ),
                probe_emitter("normal", [column(6), 0.0, 1.5], one),
            ]
        }
        "linked" => {
            // A few large particles born still, anywhere in a 1.5 m
            // square: red `Linked`, blue `Normal`. What joins the red ones
            // (and how wide) is what `Linked` draws.
            let few = "xsize 150\n  ysize 150\n  birthrate 2\n  lifeExp 3\n  velocity 0\n  \
                       sizeStart 0.3\n  sizeEnd 0.3";
            vec![
                probe_emitter(
                    "linked",
                    [column(1), 0.0, 1.0],
                    &format!("{few}\n  render Linked\n  colorStart 1 0 0\n  colorEnd 1 0 0"),
                ),
                probe_emitter(
                    "normal",
                    [column(6), 0.0, 1.0],
                    &format!("{few}\n  colorStart 0 0 1\n  colorEnd 0 0 1"),
                ),
            ]
        }
        "linked2" => {
            // As `linked`, fading out over their lives and showing a
            // texture of four colors (red and green on its first row,
            // blue and white on its last): how a ribbon's pieces fade, and
            // what of the texture each shows.
            let few = "xsize 200\n  ysize 200\n  birthrate 3\n  lifeExp 4\n  velocity 0\n  \
                       sizeStart 0.6\n  sizeEnd 0.6\n  alphaStart 1\n  alphaEnd 0\n  \
                       texture mgquad";
            // (`MG_GRID`: the texture as a flip-book of its four colors,
            // one a second.)
            let grid = std::env::var_os("MG_GRID")
                .map_or("", |_| "\n  xgrid 2\n  ygrid 2\n  frameStart 0\n  frameEnd 3\n  fps 1");
            let few = format!("{few}{grid}");
            vec![
                probe_emitter("linked", [column(1), 0.0, 1.0], &format!("{few}\n  render Linked")),
                probe_emitter("normal", [column(6), 0.0, 1.0], &few),
            ]
        }
        "linked4" => {
            // A column of particles rising half a metre apart, each 0.6 m
            // and showing the four-color texture: what joins them when
            // they are `Linked` (left of the plain ones).
            let column_of = "birthrate 2\n  sizeStart 0.6\n  sizeEnd 0.6\n  texture mgquad";
            vec![
                probe_emitter(
                    "linked",
                    [column(1), 0.0, 0.0],
                    &format!("{column_of}\n  render Linked"),
                ),
                probe_emitter("normal", [column(6), 0.0, 0.0], column_of),
            ]
        }
        "sides" => {
            // Upright quads (`worldz` shows they are), seen from above (low)
            // and below (high).
            let upright = "render Aligned_to_World_Z\n  sizeStart 0.3\n  sizeEnd 0.3\n  birthrate 4\n  \
                        velocity 0.5\n  lifeExp 2";
            vec![
                probe_emitter(
                    "low_one",
                    [column(1), 0.0, 0.0],
                    &format!("{upright}\n  colorStart 1 0 0\n  colorEnd 1 0 0"),
                ),
                probe_emitter(
                    "low_two",
                    [column(3), 0.0, 0.0],
                    &format!("{upright}\n  twosidedtex 1\n  colorStart 0 0 1\n  colorEnd 0 0 1"),
                ),
                probe_emitter(
                    "high_one",
                    [column(5), 0.0, 4.0],
                    &format!("{upright}\n  colorStart 1 0 0\n  colorEnd 1 0 0"),
                ),
                probe_emitter(
                    "high_two",
                    [column(7), 0.0, 4.0],
                    &format!("{upright}\n  twosidedtex 1\n  colorStart 0 0 1\n  colorEnd 0 0 1"),
                ),
            ]
        }
        "worldz" => {
            // One still quad each, just above the floor (an upright one is
            // half sunk in it): red `Aligned_to_World_Z`, blue
            // `Billboard_to_World_Z`. The red one is cut in half: it stands,
            // and the blue one lies flat.
            let still = "sizeStart 1\n  sizeEnd 1\n  birthrate 1\n  velocity 0\n  lifeExp 2";
            vec![
                probe_emitter(
                    "aligned",
                    [column(1), 0.0, 0.1],
                    &format!(
                        "{still}\n  render Aligned_to_World_Z\n  colorStart 1 0 0\n  colorEnd 1 0 0"
                    ),
                ),
                probe_emitter(
                    "billboard",
                    [column(6), 0.0, 0.1],
                    &format!(
                        "{still}\n  render Billboard_to_World_Z\n  colorStart 0 0 1\n  colorEnd 0 0 1"
                    ),
                ),
            ]
        }
        "tilelights" => vec![
            probe_emitter("tinted0", [column(0), 0.0, 0.0], "m_isTinted 1"),
            probe_emitter("tinted3", [column(3), 0.0, 0.0], "m_isTinted 1"),
            probe_emitter("tinted6", [column(6), 0.0, 0.0], "m_isTinted 1"),
            probe_emitter("plain", [column(7), 0.0, 0.0], ""),
        ],
        "stops2" => {
            let mid = "colorStart 1 0 0\n  colorMid 0 1 0\n  colorEnd 0 0 1\n  ";
            vec![
                probe_emitter(
                    "p100",
                    [column(0), 0.0, 0.0],
                    &format!("{mid}percentStart 0\n  percentMid 50\n  percentEnd 100"),
                ),
                probe_emitter(
                    "p1",
                    [column(1), 0.0, 0.0],
                    &format!("{mid}percentStart 0\n  percentMid 0.5\n  percentEnd 1"),
                ),
                probe_emitter(
                    "p255",
                    [column(2), 0.0, 0.0],
                    &format!("{mid}percentStart 0\n  percentMid 128\n  percentEnd 255"),
                ),
                probe_emitter(
                    "keyed",
                    [column(3), 0.0, 0.0],
                    &format!("{mid}percentStart 10\n  percentMid 50\n  percentEnd 90"),
                ),
                probe_emitter(
                    "grey_tinted",
                    [column(5), 0.0, 0.0],
                    "m_isTinted 1\n  colorStart 0.5 0.5 0.5\n  colorEnd 0.5 0.5 0.5",
                ),
                probe_emitter("white_tinted", [column(6), 0.0, 0.0], "m_isTinted 1"),
                probe_emitter(
                    "grey",
                    [column(7), 0.0, 0.0],
                    "colorStart 0.5 0.5 0.5\n  colorEnd 0.5 0.5 0.5",
                ),
            ]
        }
        _ => vec![
            probe_emitter("base", [column(0), 0.0, 0.0], "colorStart 1 0 0\n  colorEnd 0 0 1"),
            probe_emitter(
                "mid",
                [column(1), 0.0, 0.0],
                "colorStart 1 0 0\n  colorMid 0 1 0\n  colorEnd 0 0 1\n  percentMid 0.5",
            ),
            probe_emitter(
                "mid50",
                [column(2), 0.0, 0.0],
                "colorStart 1 0 0\n  colorMid 0 1 0\n  colorEnd 0 0 1\n  percentMid 50",
            ),
            probe_emitter(
                "stops",
                [column(3), 0.0, 0.0],
                "colorStart 1 0 0\n  colorMid 0 1 0\n  colorEnd 0 0 1\n  percentStart 0.2\n  \
                 percentMid 0.5\n  percentEnd 0.8",
            ),
            probe_emitter(
                "alpha",
                [column(4), 0.0, 0.0],
                "alphaStart 1\n  alphaMid 0\n  alphaEnd 1\n  percentMid 0.5",
            ),
            probe_emitter(
                "size",
                [column(5), 0.0, 0.0],
                "sizeStart 0.04\n  sizeMid 0.3\n  sizeEnd 0.04\n  percentMid 0.5",
            ),
            probe_emitter("tinted", [column(6), 0.0, 0.0], "m_isTinted 1"),
            probe_emitter("plain", [column(7), 0.0, 0.0], ""),
        ],
    };
    let mut model = format!(
        "newmodel plc_a01\nsetsupermodel plc_a01 NULL\nclassification Character\n\
         setanimationscale 1\nbeginmodelgeom plc_a01\nnode dummy plc_a01\n  parent NULL\n\
         endnode\n{}endmodelgeom plc_a01\ndonemodel plc_a01\n",
        emitters.concat()
    );
    // `MG_MODEL`: a model's ASCII file to look at in the armoire's place
    // (a builder's placeable), with the files beside it (its textures).
    let beside = std::env::var_os("MG_MODEL").map(std::path::PathBuf::from);
    if let Some(path) = &beside {
        let text = std::fs::read_to_string(path).unwrap();
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        model = text.replace(&name, "plc_a01");
    }
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    std::fs::write(user.join("override/plc_a01.mdl"), &model).unwrap();
    std::fs::write(user.join("override/mgwhite.tga"), white_tga()).unwrap();
    std::fs::write(user.join("override/mgquad.tga"), quadrant_tga(0)).unwrap();
    std::fs::write(user.join("override/mgramp.tga"), ramp_tga()).unwrap();
    if let Some(dir) = beside.as_deref().and_then(|p| p.parent()) {
        for file in std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()) {
            if file.is_file() && file.extension().is_some_and(|e| e != "mdl") {
                std::fs::copy(&file, user.join("override").join(file.file_name().unwrap()))
                    .unwrap();
            }
        }
    }
    std::fs::write(user.join("settings.tml"), SETTINGS).unwrap();
    // From the side, closer and lower than the reference scenes.
    let enter =
        ENTER.replace("SetCameraFacing(90.0, 12.0, 45.0", "SetCameraFacing(90.0, 9.0, 80.0");
    let ncs = compile(&dir, "mg_enter", &enter);
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), ncs);
    m.save_as(&ModuleLocation::Archive(user.join("modules/MgScene.mod"))).unwrap();
    if std::env::var_os("MG_PREDICT").is_some() {
        mg_testkit::gpu::hold();
        let gpu = Gpu::headless().unwrap();
        let (_, are) = build_module(&game, &dir, light);
        let (scene, _) = moonglow_scene(&gpu, &game, &are, light);
        for i in [0, 3, 6] {
            let at = Vec3::new(20.0 - column(i), 23.0, 0.0);
            eprintln!("tint at column {i}: {:?}", mg_render::tint_light(&scene, at) * 255.0);
        }
        return;
    }
    let client = client_screenshot(&dir, "MgScene");
    if let Some(c) = &client {
        save_png(c, &dir.join("client.png"));
    }
    eprintln!("{}: client {}", dir.display(), client.is_some());
    assert!(client.is_some());
}

/// A 64×64 TGA in four colors (as its file's rows and columns go: first
/// row red then green, last row blue then white), its descriptor as given.
fn quadrant_tga(descriptor: u8) -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 64, 0, 32, descriptor];
    for y in 0..64 {
        for x in 0..64 {
            // (BGRA.)
            t.extend(match (y < 32, x < 32) {
                (true, true) => [0, 0, 255, 255],
                (true, false) => [0, 255, 0, 255],
                (false, true) => [255, 0, 0, 255],
                (false, false) => [255, 255, 255, 255],
            });
        }
    }
    t
}

/// Exploration: placed placeables in the client beside Moonglow's drawing
/// of the same area. Three armoires north of the player: the west one
/// tilted (a visual transform turned about all three axes), the middle one
/// plain, the east one tilted and static. `MG_TGA` (a number) gives the
/// armoire's texture as a four-color TGA with that image descriptor (16:
/// right to left; 32: top first); `MG_OVERRIDE` a folder of files for the
/// scratch override (another armoire model). Both views go to
/// `target/test-output/client_placeables/`.
#[test]
#[ignore]
fn placeables_look() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let dir = scratch_dir("client_placeables");
    std::fs::create_dir_all(dir.join("user/override")).unwrap();
    // `MG_OVERRIDE`: a folder of files for the scratch user folder's
    // override (the armoire's model, `plc_a01.mdl`, made another).
    if let Some(from) = std::env::var_os("MG_OVERRIDE") {
        for f in std::fs::read_dir(from).unwrap().flatten() {
            std::fs::copy(f.path(), dir.join("user/override").join(f.file_name())).unwrap();
        }
    }
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    if let Some(d) = std::env::var("MG_TGA").ok().and_then(|d| d.parse::<u8>().ok()) {
        let model = base.resman.get_named("plc_a01", ResType::MDL).unwrap();
        let model = Model::read(&model).unwrap();
        let texture = model
            .nodes
            .iter()
            .find_map(|n| match &n.kind {
                mg_mdl::NodeKind::Mesh(m) => m.textures[0].clone(),
                _ => None,
            })
            .unwrap();
        eprintln!("texture {texture}");
        std::fs::write(dir.join(format!("user/override/{texture}.tga")), quadrant_tga(d)).unwrap();
    }
    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let (mut m, are) = build_module(&game, &dir, light);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let bp = game.resman.get_named("plc_armoire", ResType::UTP).unwrap();
    let bp = Gff::read(&bp).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let axis = |v: f32| {
        let mut s = mg_gff::Struct::new(0);
        s.set("LerpType", Value::Int(0));
        s.set("ValueTo", Value::Float(v));
        Value::Struct(s)
    };
    let tilt = || {
        let mut e = mg_gff::Struct::new(6);
        e.set("Scope", Value::Int(0));
        e.set("AnimationSpeed", axis(1.0));
        for (prefix, v) in
            [("Scale", [1.0; 3]), ("Rotate", [30.0, 20.0, 45.0]), ("Translate", [0.0; 3])]
        {
            for (i, a) in ["X", "Y", "Z"].iter().enumerate() {
                e.set(&format!("{prefix}{a}"), axis(v[i]));
            }
        }
        Value::List(vec![e])
    };
    let mut placed = Vec::new();
    for (x, y, tilted, is_static) in
        [(17.5, 21.0, true, false), (20.0, 23.0, false, false), (22.5, 21.0, true, true)]
    {
        let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
        let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
        if tilted {
            p.set("VisTransformList", tilt());
        }
        p.set("Static", Value::Byte(u8::from(is_static)));
        placed.push(p);
    }
    git.root.set("Placeable List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = client_screenshot(&dir, "MgScene");

    let model = mg_area::AreaModel::read(
        &game,
        &are.root,
        &git.root,
        mg_area::tileset(&game, ResRef::from_str("tic01").unwrap()).ok().as_ref(),
    );
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let (w, h) = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    off_and_on(&gpu, &game, &scene, &camera, (w, h), client.as_ref(), &dir, "moonglow");
    eprintln!("{}: client {}", dir.display(), client.is_some());
}

/// Exploration: a builder's mirror given a visual transform by Adjust
/// Location (Scale 0.46, X Rotation -50.8: `mg_area::edit`'s edits), in
/// the client beside Moonglow's drawing. Five mirrors (placeables.2da
/// `MG_APPEARANCE`, default 447, Mirror) north of the player, west to
/// east: transformed and dynamic, not Useable; transformed, dynamic and
/// Useable; transformed and static; plain; and dynamic with the scale in
/// `ScaleX` alone, as the engine's own save writes `SetObjectVisualTransform`'s
/// scale. `MG_BEARING` turns them all (degrees; the builder's was 89.63).
/// `MG_ROTATIONS` places others instead, dynamic, each given as `rx,ry,rz`
/// and optionally `,sx,sy,sz,tx,ty,tz` (`;` between them): with
/// `MG_APPEARANCE=179` (a chair, whose back shows which way it turns),
/// `0,0,0;45,0,0;0,45,0;0,0,45;90,45,0`, then
/// `0,45,45;90,0,45;90,45,45;60,30,70;-40,60,20` and translations and
/// scales one axis at a time settled `VisualTransform::matrix`. Both
/// views go to `target/test-output/client_visual_transforms/`.
#[test]
#[ignore]
fn visual_transforms_look() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let dir = scratch_dir("client_visual_transforms");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let (mut m, are) = build_module(&game, &dir, light);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let mut bp = Gff::read(&game.resman.get_named("plc_barrel", ResType::UTP).unwrap()).unwrap();
    let appearance =
        std::env::var("MG_APPEARANCE").ok().and_then(|a| a.parse().ok()).unwrap_or(447u32);
    bp.root.set("Appearance", Value::Dword(appearance));
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let mirror = mg_area::VisualTransform {
        scale: Vec3::splat(0.46),
        rotate: Vec3::new(-50.8, 0.0, 0.0),
        translate: Vec3::ZERO,
    };
    let x_only = mg_area::VisualTransform { scale: Vec3::new(0.46, 1.0, 1.0), ..mirror };
    // `MG_BEARING`: each one's Bearing, degrees (the builder's was 89.63).
    let bearing = std::env::var("MG_BEARING")
        .ok()
        .and_then(|b| b.parse::<f32>().ok())
        .unwrap_or(0.0)
        .to_radians();
    // `MG_ROTATIONS`: other mirrors instead, dynamic, each turned by its
    // `x,y,z` (degrees; `;` between mirrors), to tell the axes apart.
    let mirrors: Vec<(Option<mg_area::VisualTransform>, u8, u8)> =
        match std::env::var("MG_ROTATIONS") {
            Ok(list) => list
                .split(';')
                .map(|r| {
                    let r: Vec<f32> = r.split(',').map(|v| v.trim().parse().unwrap()).collect();
                    let at = |i: usize, or: f32| {
                        Vec3::from_array(std::array::from_fn(|k| {
                            r.get(i + k).copied().unwrap_or(or)
                        }))
                    };
                    let v = mg_area::VisualTransform {
                        rotate: at(0, 0.0),
                        scale: at(3, 1.0),
                        translate: at(6, 0.0),
                    };
                    (Some(v), 0, 0)
                })
                .collect(),
            Err(_) => vec![
                (Some(mirror), 0, 0),
                (Some(mirror), 0, 1),
                (Some(mirror), 1, 0),
                (None, 0, 0),
                (Some(x_only), 0, 0),
            ],
        };
    let mut placed = Vec::new();
    for (i, (visual, is_static, useable)) in mirrors.into_iter().enumerate() {
        let at = Placement { position: [17.2 + 1.4 * i as f32, 23.0, 0.0], rotation: bearing };
        let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
        p.set("Static", Value::Byte(is_static));
        p.set("Useable", Value::Byte(useable));
        if let Some(v) = visual {
            p.set("VisTransformList", mg_area::edit::visual_transform_list(v));
        }
        placed.push(p);
    }
    git.root.set("Placeable List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = client_screenshot(&dir, "MgScene");

    let model = mg_area::AreaModel::read(
        &game,
        &are.root,
        &git.root,
        mg_area::tileset(&game, ResRef::from_str("tic01").unwrap()).ok().as_ref(),
    );
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let (w, h) = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    off_and_on(&gpu, &game, &scene, &camera, (w, h), client.as_ref(), &dir, "moonglow");
    eprintln!("{}: client {}", dir.display(), client.is_some());
}

/// Exploration: items lying in an area, in the client beside Moonglow's
/// drawing: a shield, a potion, a sword and armor of each weight, north
/// of the player. `MG_ITEMS` gives other blueprints (comma-separated),
/// `MG_CLIENT_CAMERA` the client's camera as nwscript and `MG_CAMERA`
/// Moonglow's (`distance,pitch,focus,fov`). Both views go to
/// `target/test-output/client_items/`.
#[test]
#[ignore]
fn items_look() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let dir = scratch_dir("client_items");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let (mut m, are) = build_module(&game, &dir, light);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    let enter = match std::env::var("MG_CLIENT_CAMERA") {
        Ok(code) => enter.replace(
            "AssignCommand(pc, SetCameraFacing(90.0, 12.0, 45.0, CAMERA_TRANSITION_TYPE_SNAP));",
            &code,
        ),
        Err(_) => enter,
    };
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let names = std::env::var("MG_ITEMS").unwrap_or_else(|_| {
        "nw_ashsw001,nw_it_mpotion001,nw_wswss001,nw_cloth001,nw_aarcl001,nw_aarcl004,nw_aarcl007"
            .into()
    });
    let names: Vec<&str> = names.split(',').map(str::trim).collect();
    let mut placed = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let bp = game.resman.get_named(name, ResType::UTI).unwrap();
        let bp = Gff::read(&bp).unwrap();
        let x = 20.0 + (i as f32 - (names.len() - 1) as f32 / 2.0) * 1.2;
        let at = Placement { position: [x, 22.0, 0.0], rotation: 0.0 };
        placed.push(instance(&placing, ResType::UTI, &bp.root, at, &[]).unwrap());
    }
    git.root.set(mg_area::ObjectKind::Item.list(), Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = client_screenshot(&dir, "MgScene");

    let model = mg_area::AreaModel::read(
        &game,
        &are.root,
        &git.root,
        mg_area::tileset(&game, ResRef::from_str("tic01").unwrap()).ok().as_ref(),
    );
    for o in &model.objects {
        eprintln!("object {:?} laid {} problem {:?}", o.kind, o.laid, o.problem);
    }
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let c: Vec<f32> = std::env::var("MG_CAMERA")
        .unwrap_or_else(|_| "5,60,0.3,40".into())
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    let camera = fitted_camera(Vec3::new(20.0, 20.0, 0.0), c[3], c[2], c[0], c[1]);
    let (w, h) = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    off_and_on(&gpu, &game, &scene, &camera, (w, h), client.as_ref(), &dir, "moonglow");
    eprintln!("{}: client {}", dir.display(), client.is_some());
}

/// Exploration: creatures in the client beside Moonglow's drawing. A human
/// (west) and an elf (east), each in an armor whose torso has colors of
/// its own (the rest of the armor another) and a cloak, standing north of
/// the player and facing it. Both views go to
/// `target/test-output/client_creatures/`. `MG_CAMERA` gives Moonglow's
/// camera as `distance,pitch,focus,fov` (it is not fitted for this view).
#[test]
#[ignore]
fn creatures_look() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_rules::items::{ArmorChannel, set_armor_part_color};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let dir = scratch_dir("client_creatures");
    std::fs::create_dir_all(dir.join("user/override")).unwrap();
    // `MG_OVERRIDE`: a folder of files for the scratch user folder's
    // override (a custom creature's model, textures and appearance.2da),
    // with `MG_APPEARANCES=west,east` for the two creatures' rows.
    let custom = std::env::var_os("MG_OVERRIDE").map(std::path::PathBuf::from);
    if let Some(from) = &custom {
        for f in std::fs::read_dir(from).unwrap().flatten() {
            std::fs::copy(f.path(), dir.join("user/override").join(f.file_name())).unwrap();
        }
    }
    // (At the settings Moonglow's programs are made for, High Quality: at
    // the game's default, Performance, a cut-out's edges are not lit
    // through from behind.)
    std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
    let user = custom.as_ref().map(|_| dir.join("user"));
    let game = GameData::open(&GameInstall::new(&root, user, "en")).unwrap();
    // `MG_LIGHT`: the area's ambient and diffuse colors (0xBBGGRR, as an
    // ARE has them), as `ambient,diffuse` in hexadecimal.
    let colors: Vec<u32> = std::env::var("MG_LIGHT")
        .ok()
        .map(|l| l.split(',').map(|v| u32::from_str_radix(v.trim(), 16).unwrap()).collect())
        .unwrap_or_else(|| vec![0x808080, 0xC0C0C0]);
    let light = Lighting { ambient: colors[0], diffuse: colors[1], main_light: 0, only_tile: None };
    let (mut m, are) = build_module(&game, &dir, light);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    // `MG_CLIENT_CAMERA`: the client's camera instead, as nwscript
    // (`SetCameraMode(pc, CAMERA_MODE_CHASE_CAMERA); AssignCommand(pc, SetCameraFacing(90.0, 5.0, 70.0, CAMERA_TRANSITION_TYPE_SNAP));`).
    // What each creature wears, as the game has it, goes to the log.
    let enter = enter.replace(
        "    LockCameraDirection(pc, TRUE);",
        "    object o = GetFirstObjectInArea(GetAreaFromLocation(start));\n    while (GetIsObjectValid(o)) {\n        if (GetObjectType(o) == OBJECT_TYPE_CREATURE) {\n            if (!GetIsObjectValid(GetItemInSlot(INVENTORY_SLOT_CLOAK, o))) { object k = CreateItemOnObject(\"mg_cloak\", o); AssignCommand(o, ActionEquipItem(k, INVENTORY_SLOT_CLOAK)); Log(\"MG_GIVEN \" + IntToString(GetIsObjectValid(k))); }\n            Log(\"MG_WORN cloak [\" + GetTag(GetItemInSlot(INVENTORY_SLOT_CLOAK, o)) + \"] chest [\" + GetTag(GetItemInSlot(INVENTORY_SLOT_CHEST, o)) + \"] wings \" + IntToString(GetCreatureWingType(o)));\n            DelayCommand(3.0, Log(\"MG_LATER cloak [\" + GetTag(GetItemInSlot(INVENTORY_SLOT_CLOAK, o)) + \"]\"));\n        }\n        o = GetNextObjectInArea(GetAreaFromLocation(start));\n    }\n    LockCameraDirection(pc, TRUE);",
    );
    let enter = match std::env::var("MG_CLIENT_CAMERA") {
        Ok(code) => enter.replace(
            "AssignCommand(pc, SetCameraFacing(90.0, 12.0, 45.0, CAMERA_TRANSITION_TYPE_SNAP));",
            &code,
        ),
        Err(_) => enter,
    };
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));

    // The first of the game's blueprints that fits.
    let first = |restype: ResType, fits: &dyn Fn(&mg_gff::Struct) -> bool| -> Gff {
        let mut keys: Vec<ResKey> = game
            .resman
            .entries()
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| k.restype == restype)
            .collect();
        keys.sort_by_key(|k| k.to_string());
        keys.iter()
            .filter_map(|k| Gff::read(&game.resman.get(k).ok()?).ok())
            .find(|g| fits(&g.root))
            .expect("a blueprint that fits")
    };
    let mut armor = first(ResType::UTI, &|s| {
        s.integer("BaseItem") == Some(16)
            && s.integer("ArmorPart_Torso").is_some_and(|t| t > 3)
            && s.integer("ArmorPart_Robe").unwrap_or(0) == 0
    });
    for channel in [
        "Cloth1Color",
        "Cloth2Color",
        "Leather1Color",
        "Leather2Color",
        "Metal1Color",
        "Metal2Color",
    ] {
        armor.root.set(channel, Value::Byte(20));
    }
    for channel in [
        ArmorChannel::Cloth1,
        ArmorChannel::Cloth2,
        ArmorChannel::Leather1,
        ArmorChannel::Leather2,
        ArmorChannel::Metal1,
        ArmorChannel::Metal2,
    ] {
        // (Part 7: the torso.)
        set_armor_part_color(&mut armor.root, 7, channel, Some(88));
    }
    // `MG_PARTS`: armor part numbers, as `Torso=32,LThigh=3,RThigh=3`.
    if let Ok(parts) = std::env::var("MG_PARTS") {
        for (part, n) in parts.split(',').filter_map(|p| p.split_once('=')) {
            let n: u8 = n.trim().parse().unwrap();
            let label = format!("ArmorPart_{}", part.trim());
            armor.root.set(&label, Value::Byte(n));
            let wide = mg_rules::items::wide_label(&label);
            if armor.root.get(&wide).is_some() {
                armor.root.set(&wide, Value::Word(u16::from(n)));
            }
        }
        armor.root.set("ArmorPart_Robe", Value::Byte(0));
    }
    let mut cloak = first(ResType::UTI, &|s| s.integer("BaseItem") == Some(80));
    // (Nothing that could keep a creature from wearing it.)
    cloak.root.set("PropertiesList", Value::List(Vec::new()));
    // `MG_CLOAK`: the cloak's six colors, all the same.
    if let Some(c) = std::env::var("MG_CLOAK").ok().and_then(|c| c.parse::<u8>().ok()) {
        for channel in [
            "Cloth1Color",
            "Cloth2Color",
            "Leather1Color",
            "Leather2Color",
            "Metal1Color",
            "Metal2Color",
        ] {
            cloak.root.set(channel, Value::Byte(c));
        }
    }
    eprintln!(
        "armor {:?} torso {:?}, cloak {:?}",
        armor.root.resref("TemplateResRef"),
        armor.root.integer("ArmorPart_Torso"),
        cloak.root.resref("TemplateResRef")
    );
    for f in &cloak.root.fields {
        let label = f.label.to_string_lossy();
        if label.contains("Color") || label.contains("Model") || label.contains("Col_") {
            eprintln!("cloak {label} = {:?}", f.value);
        }
    }
    let human = first(ResType::UTC, &|s| {
        s.integer("Appearance_Type") == Some(6) && s.integer("Gender") == Some(0)
    });
    let equip = |resref: &str, slot: u32| {
        let mut e = mg_gff::Struct::new(slot);
        e.set("EquippedRes", Value::resref(ResRef::from_str(resref).unwrap()));
        e
    };
    // `MG_HELD`: one of the game's item blueprints for the left hand
    // (a shield, a torch), as `nw_ashsw001`.
    let held = std::env::var("MG_HELD").ok().map(|name| {
        let key = ResKey::new(ResRef::from_str(&name).unwrap(), ResType::UTI);
        Gff::read(&game.resman.get(&key).expect("the held item's blueprint")).unwrap()
    });
    let mut made = vec![("mg_armor", &armor), ("mg_cloak", &cloak)];
    made.extend(held.as_ref().map(|h| ("mg_held", h)));
    let items: HashMap<ResRef, mg_gff::Struct> = made
        .into_iter()
        .map(|(name, g)| {
            let r = ResRef::from_str(name).unwrap();
            let mut g = g.clone();
            g.root.set("TemplateResRef", Value::resref(r));
            m.set_gff(ResKey::new(r, ResType::UTI), &g).unwrap();
            (r, g.root)
        })
        .collect();
    let item = |r: ResRef| items.get(&r).cloned();
    let placing = Placing { game: &game, item: &item };
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let mut placed = Vec::new();
    let rows: Vec<u16> = std::env::var("MG_APPEARANCES")
        .map(|r| r.split(',').map(|v| v.trim().parse().unwrap()).collect())
        .unwrap_or_else(|_| vec![6, 1]);
    for (x, appearance, race) in [(19.0, rows[0], 6), (21.0, rows[1], 1)] {
        let mut c = human.root.clone();
        c.set("Appearance_Type", Value::Word(appearance));
        c.set("Race", Value::Byte(race));
        // (Standing still: no scripts, nothing to say.)
        let scripts: Vec<String> = c
            .fields
            .iter()
            .map(|f| f.label.to_string_lossy())
            .filter(|l| l.starts_with("Script"))
            .collect();
        for label in scripts {
            c.set(&label, Value::resref(ResRef::EMPTY));
        }
        // `MG_NO_BODY`: the west creature without its body part fields,
        // as a creature made with a single model has none.
        if std::env::var_os("MG_NO_BODY").is_some() && x < 20.0 {
            c.fields.retain(|f| {
                let l = f.label.to_string_lossy();
                !(l.contains("BodyPart_") || l.contains("ArmorPart_RFoot"))
            });
        }
        let mut worn = vec![equip("mg_armor", 0x2), equip("mg_cloak", 0x40)];
        // `MG_WORN`: only the armor (1) or only the cloak (2).
        match std::env::var("MG_WORN").as_deref() {
            Ok("0") => worn.clear(),
            Ok("1") => worn.truncate(1),
            Ok("2") => drop(worn.remove(0)),
            _ => {}
        }
        if held.is_some() {
            worn.push(equip("mg_held", 0x20));
        }
        c.set("Equip_ItemList", Value::List(worn));
        // `MG_Y`: how far north they stand (the player is at 20).
        let y = std::env::var("MG_Y").ok().and_then(|y| y.parse().ok()).unwrap_or(22.5);
        let at = Placement { position: [x, y, 0.0], rotation: std::f32::consts::PI };
        placed.push(instance(&placing, ResType::UTC, &c, at, &[]).unwrap());
    }
    git.root.set("Creature List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = client_screenshot(&dir, "MgScene");

    let model = mg_area::AreaModel::read(
        &game,
        &are.root,
        &git.root,
        mg_area::tileset(&game, ResRef::from_str("tic01").unwrap()).ok().as_ref(),
    );
    for o in &model.objects {
        eprintln!("object {:?} problem {:?}", o.kind, o.problem);
    }
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let c: Vec<f32> = std::env::var("MG_CAMERA")
        .unwrap_or_else(|_| "19,50,1.25,48".into())
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    let camera = fitted_camera(Vec3::new(20.0, 20.0, 0.0), c[3], c[2], c[0], c[1]);
    let (w, h) = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    off_and_on(&gpu, &game, &scene, &camera, (w, h), client.as_ref(), &dir, "moonglow");
    eprintln!("{}: client {}", dir.display(), client.is_some());
}

/// A 64×64 TGA with an alpha channel: an orange disc, opaque to 26 texels
/// from the middle, its alpha falling to nothing by 30 over blue (which
/// shows where the alpha is blended and not cut).
fn disc_tga() -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 64, 0, 32, 8];
    for y in 0..64 {
        for x in 0..64 {
            let d = ((x as f32 - 31.5).powi(2) + (y as f32 - 31.5).powi(2)).sqrt();
            let a = ((30.0 - d) / 4.0).clamp(0.0, 1.0);
            // (BGRA.)
            t.extend(if d < 26.0 { [60, 160, 255, 255] } else { [255, 0, 0, (a * 255.0) as u8] });
        }
    }
    t
}

/// The cut-out variants of [`cutouts_look`], west to east: the texture's
/// name, its TXI, and its MTR (the mesh then names the material).
const CUTOUTS: [(&str, &str, Option<&str>); 6] = [
    ("mg_cut_p", "blending punchthrough\n", None),
    ("mg_cut_d", "blending punchthrough\ndecal 1\n", None),
    ("mg_cut_o", "decal 1\n", None),
    ("mg_cut_t", "blending punchthrough\n", Some("texture0 mg_cut_t\ntwosided 1\n")),
    ("mg_cut_n", "", None),
    ("mg_cut_k", "blending punchthrough\n", Some("texture0 mg_cut_k\n")),
];

/// The cards' model: for each variant a 1 m square standing on the ground
/// that faces the model's −Y (south, as placed) and above it one that faces
/// +Y, each a single face seen from one side.
fn cards_mdl() -> String {
    let mut s = String::from(
        "newmodel mg_cards\nsetsupermodel mg_cards NULL\nclassification character\n\
         setanimationscale 1\nbeginmodelgeom mg_cards\nnode dummy mg_cards\n  parent NULL\nendnode\n",
    );
    for (i, (texture, _, mtr)) in CUTOUTS.iter().enumerate() {
        for (row, faces) in
            [("0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0", "0 2 1 1 0 2 1 0\n    0 3 2 1 0 3 2 0")]
                .iter()
                .flat_map(|(f, b)| [(0, *f), (1, *b)])
        {
            let x = i as f32 * 1.2 - 3.5;
            let z = row as f32 * 1.2 + 0.1;
            let material = mtr.map_or(String::new(), |_| format!("  materialname {texture}\n"));
            s += &format!(
                "node trimesh c{i}_{row}\n  parent mg_cards\n  position {x} 0 {z}\n  \
                 orientation 0 0 0 0\n  ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  \
                 shininess 1\n  bitmap {texture}\n{material}  render 1\n  shadow 0\n  verts 4\n    \
                 0 0 0\n    1 0 0\n    1 0 1\n    0 0 1\n  tverts 4\n    0 0 0\n    1 0 0\n    \
                 1 1 0\n    0 1 0\n  faces 2\n    {faces}\nendnode\n"
            );
        }
    }
    s + "endmodelgeom mg_cards\ndonemodel mg_cards\n"
}

/// Exploration: what the client does with cut-out textures and with
/// placeables' shadows, beside Moonglow's drawing of the same area. North
/// of the player stand the cards of [`cards_mdl`] (each variant of
/// [`CUTOUTS`] seen from the front, below, and from behind, above), and
/// behind them placeables in pairs, the east one of each static: armoires,
/// and with `MG_HAK` (a hak) and `MG_MODEL` (a placeable model in it) that
/// model, its files copied to the scratch `override`. `MG_STRIP_DECAL=1`
/// drops `decal` from that model's TXIs. `MG_LIGHT`: `day` (the default),
/// `night` (ambient and diffuse 0x101010) or `shadow` (day, with the area's
/// and the client's shadows on); `MG_MAINLIGHT` a lightcolor.2da row for
/// the tiles' main lights; `MG_TILESET` another tileset;
/// `MG_TURN=1` turns the cards about (their fronts north, away from the
/// camera); `MG_DISCARD=1` paints every surface with the alpha the client
/// cuts it at;
/// `MG_CLIENT_CAMERA` the client's camera as nwscript. Both views go to
/// `target/test-output/client_cutouts/`.
#[test]
#[ignore]
fn cutouts_look() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let dir = scratch_dir("client_cutouts");
    let over = dir.join("user/override");
    std::fs::create_dir_all(&over).unwrap();
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();

    std::fs::write(over.join("mg_cards.mdl"), cards_mdl()).unwrap();
    // `MG_DISCARD=1`: every surface painted with the alpha it is cut at
    // (red: `fAlphaDiscardValue`; green: it is below 0, nothing is cut).
    if std::env::var_os("MG_DISCARD").is_some() {
        let src = base.resman.get_named("inc_standard", ResType::SHD).unwrap();
        let mut src = String::from_utf8_lossy(&src).replace("\r\n", "\n");
        let end = "\tApplyDebugModeOutput(FragmentColor);\n}";
        let at = src.rfind(end).expect("ApplyDebugModeOutput");
        src.insert_str(
            at + end.len() - 1,
            "#if SHADER_TYPE == 2 && NO_DISCARD != 1\n\tFragmentColor = \
             vec4(clamp(ALPHA_DISCARD_VALUE, 0.0, 1.0), ALPHA_DISCARD_VALUE < 0.0 ? 1.0 : 0.0, \
             0.25, 1.0);\n#endif\n",
        );
        std::fs::write(over.join("inc_standard.shd"), src).unwrap();
    }
    for (texture, txi, mtr) in CUTOUTS {
        std::fs::write(over.join(format!("{texture}.tga")), disc_tga()).unwrap();
        if !txi.is_empty() {
            std::fs::write(over.join(format!("{texture}.txi")), txi).unwrap();
        }
        if let Some(mtr) = mtr {
            std::fs::write(over.join(format!("{texture}.mtr")), mtr).unwrap();
        }
    }
    // A hak's model, as loose files.
    let extra = std::env::var("MG_MODEL").ok();
    if let (Some(model), Ok(hak)) = (&extra, std::env::var("MG_HAK")) {
        let data = std::fs::read(hak).unwrap();
        let erf = mg_erf::Erf::read(&data).unwrap();
        for e in &erf.entries {
            let name = e.resref.to_string().to_ascii_lowercase();
            let Some(ext) = e.restype.extension() else { continue };
            if !name.starts_with(model.as_str()) {
                continue;
            }
            let mut bytes = erf.data(e).unwrap().into_owned();
            if ext == "txi" && std::env::var_os("MG_STRIP_DECAL").is_some() {
                let text = String::from_utf8_lossy(&bytes);
                bytes = text
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("decal"))
                    .flat_map(|l| [l, "\n"])
                    .collect::<String>()
                    .into_bytes();
            }
            std::fs::write(over.join(format!("{name}.{ext}")), bytes).unwrap();
        }
    }
    // placeables.2da with rows for the models: the armoire's row (it casts
    // a shadow: ShadowSize 1), not static and static.
    let table = base.resman.get_named("placeables", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).into_owned();
    let header: Vec<&str> = table.lines().nth(2).unwrap().split_whitespace().collect();
    let column = |name: &str| header.iter().position(|c| *c == name).unwrap() + 1;
    let (model_col, static_col) = (column("ModelName"), column("Static"));
    let first = table.lines().count() - 3;
    let armoire: Vec<String> =
        table.lines().nth(3).unwrap().split_whitespace().map(str::to_owned).collect();
    let mut models = vec!["mg_cards", "plc_a01"];
    models.extend(extra.as_deref());
    let mut rows = Vec::new();
    for model in &models {
        for is_static in [0, 1] {
            let mut row = armoire.clone();
            row[0] = (first + rows.len()).to_string();
            row[model_col] = (*model).to_owned();
            row[static_col] = is_static.to_string();
            rows.push(row.join(" "));
        }
    }
    if !table.ends_with('\n') {
        table.push('\n');
    }
    table += &(rows.join("\n") + "\n");
    std::fs::write(over.join("placeables.2da"), table).unwrap();

    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let mode = std::env::var("MG_LIGHT").unwrap_or_else(|_| "day".into());
    let main_light = std::env::var("MG_MAINLIGHT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let light = match mode.as_str() {
        "night" => Lighting { ambient: 0x101010, diffuse: 0x101010, main_light, only_tile: None },
        _ => Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light, only_tile: None },
    };
    let tileset = std::env::var("MG_TILESET").unwrap_or_else(|_| "tic01".into());
    let (mut m, mut are) = build_module_on(&game, &dir, light, &tileset, 0);
    let mut settings = SETTINGS.to_owned();
    if mode == "shadow" {
        settings = settings
            .replace(
                "[graphics.shadows.creatures]\n\t\t\tmode = 0",
                "[graphics.shadows.creatures]\n\t\t\tmode = 2",
            )
            .replace(
                "[graphics.shadows.environment]\n\t\t\tenabled = false",
                "[graphics.shadows.environment]\n\t\t\tenabled = true",
            );
        assert!(
            settings.contains("mode = 2")
                && !settings.contains("enabled = false\n\t[graphics.skyboxes]")
        );
        let are_key = *m.keys_of(ResType::ARE).next().unwrap();
        for label in ["SunShadows", "MoonShadows"] {
            are.root.set(label, Value::Byte(1));
        }
        are.root.set("ShadowOpacity", Value::Byte(100));
        m.set_gff(are_key, &are).unwrap();
    }
    std::fs::write(dir.join("user/settings.tml"), settings).unwrap();
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    let enter = match std::env::var("MG_CLIENT_CAMERA") {
        Ok(code) => enter.replace(
            "AssignCommand(pc, SetCameraFacing(90.0, 12.0, 45.0, CAMERA_TRANSITION_TYPE_SNAP));",
            &code,
        ),
        Err(_) => enter,
    };
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let mut placed = Vec::new();
    // (Row of `rows`, where.)
    let mut spots = vec![(0, [20.0, 22.5]), (2, [13.0, 23.0]), (3, [27.0, 23.0])];
    if extra.is_some() {
        spots.extend([(4, [16.5, 28.0]), (5, [23.5, 28.0])]);
    }
    let turned = std::env::var_os("MG_TURN").is_some();
    for (row, [x, y]) in spots {
        let rotation = if row == 0 && turned { std::f32::consts::PI } else { 0.0 };
        let at = Placement { position: [x, y, 0.0], rotation };
        let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
        p.set("Appearance", Value::Dword((first + row) as u32));
        p.set("Static", Value::Byte((row % 2) as u8));
        placed.push(p);
    }
    git.root.set("Placeable List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = client_screenshot(&dir, "MgScene");
    if let Some(c) = &client {
        save_png(c, &dir.join(format!("client_{mode}.png")));
    }

    let model = mg_area::AreaModel::read(
        &game,
        &are.root,
        &git.root,
        mg_area::tileset(&game, ResRef::from_str(&tileset).unwrap()).ok().as_ref(),
    );
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let (w, h) = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    off_and_on(
        &gpu,
        &game,
        &scene,
        &camera,
        (w, h),
        client.as_ref(),
        &dir,
        &format!("moonglow_{mode}"),
    );
    eprintln!("{}: client {}", dir.display(), client.is_some());
}

/// The Domains of Dread's custom shaders in the client beside Moonglow's
/// drawing (`docs/shaders-proposal.md`, M2): its fountain
/// (`dd_tap_stron1030`, placeables.2da row 63754 of `tdod_top`; the
/// falling water by `dd_tap_flow_vs`/`_fs`) north of the player in the
/// reference room, with its haks (`MOONGLOW_HAKS`, linked into the scratch
/// user folder's `hak`; `tdod_shaders`' `inc_lighting` with them). Moonglow
/// draws it with the Shaders switch off and on; each is compared with the
/// client over the fountain's pixels (where Moonglow's drawing with it
/// differs from one without it) and over the floor regions: the mean
/// difference per channel. Pictures in `target/test-output/client_tdod_shaders/`.
#[test]
#[ignore]
fn tdod_custom_shaders_look() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let Some(haks) = mg_testkit::builder_hak_dirs()
        .into_iter()
        .find(|d| d.join("tdod_p_tap.hak").is_file() && d.join("tdod_top.hak").is_file())
    else {
        eprintln!("skipped: no tdod_p_tap.hak in MOONGLOW_HAKS");
        return;
    };
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let dir = scratch_dir("client_tdod_shaders");
    std::fs::create_dir_all(dir.join("user/hak")).unwrap();
    std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
    // (Highest priority first, as the module lists them.)
    let names = ["tdod_shaders", "tdod_p_tap", "tdod_top"];
    for name in names {
        let file = format!("{name}.hak");
        mg_testkit::link_or_copy(&haks.join(&file), &dir.join("user/hak").join(&file)).unwrap();
    }
    let mut game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    for name in names.iter().rev() {
        let c = mg_resman::ErfContainer::open(&haks.join(format!("{name}.hak"))).unwrap();
        game.resman.add(
            mg_resman::priority::HAK_USER,
            format!("hak:{name}"),
            mg_resman::LayerClass::Erf,
            c,
        );
    }
    game.invalidate();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let (mut m, are) = build_module(&game, &dir, light);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let mut info = m.info().unwrap();
    let list = mg_module::attach::hak_list(&info.root, &names.map(String::from));
    info.root.set("Mod_HakList", list);
    m.set_info(&info).unwrap();
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let bp = game.resman.get_named("plc_armoire", ResType::UTP).unwrap();
    let mut bp = Gff::read(&bp).unwrap();
    bp.root.set("Appearance", Value::Dword(63754));
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [20.0, 20.0 + AHEAD + 2.0, 0.0], rotation: 0.0 };
    let fountain = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    let mut empty = git.clone();
    git.root.set("Placeable List", Value::List(vec![fountain]));
    empty.root.set("Placeable List", Value::List(Vec::new()));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = client_screenshot(&dir, "MgScene");
    if let Some(c) = &client {
        save_png(c, &dir.join("client.png"));
    }

    let tileset = mg_area::tileset(&game, ResRef::from_str("tic01").unwrap()).ok();
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let (w, h) = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    let draw = |git: &Gff, on: bool, water: bool| {
        let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
        let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
        let view = mg_area::View { fog: false, time: 4.0, ..mg_area::View::of(&model) };
        let scene = area_scene.scene(&model, &view);
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
        r.game_shaders = on;
        r.game_water = water;
        let img = r.render_image(&gpu, &game.resman, &scene, &camera, w, h);
        (img, r.drawn, r.game_shader_failures().to_vec())
    };
    let (bare, _, _) = draw(&empty, false, false);
    let (off, _, _) = draw(&git, false, false);
    let (on, drawn, failures) = draw(&git, true, false);
    // (The basin's water by `fswater`, as the client draws it, but without
    // the framebuffer: M5 measured it, and left it out.)
    let (water, _, _) = draw(&git, true, true);
    save_png(&off, &dir.join("moonglow_off.png"));
    save_png(&on, &dir.join("moonglow_on.png"));
    save_png(&water, &dir.join("moonglow_water.png"));
    eprintln!("switch on: {drawn:?}, failures {failures:?}");
    let Some(client) = client else {
        eprintln!("{}: no client picture", dir.display());
        return;
    };
    // The fountain's pixels: where it changes Moonglow's picture.
    let mask: Vec<bool> = (bare.data.chunks(4).zip(off.data.chunks(4)))
        .map(|(a, b)| (0..3).any(|i| a[i].abs_diff(b[i]) > 6))
        .collect();
    let over = |img: &Rgba| {
        let (mut sum, mut n) = (0.0f64, 0.0f64);
        for (k, (a, b)) in img.data.chunks(4).zip(client.data.chunks(4)).enumerate() {
            if mask[k] {
                sum += (0..3).map(|i| f64::from(a[i].abs_diff(b[i]))).sum::<f64>() / 3.0;
                n += 1.0;
            }
        }
        (sum / n.max(1.0), n as usize)
    };
    let (d_off, n) = over(&off);
    let (d_on, _) = over(&on);
    let (d_water, _) = over(&water);
    eprintln!(
        "fountain ({n} pixels): Moonglow's own shader differs from the client by {d_off:.1} of 255 \
         on average, the game's shaders by {d_on:.1}, and with the game's water by {d_water:.1}"
    );
    for (name, r, _) in REGIONS.iter().filter(|(n, ..)| n.contains("floor")) {
        let c = mean(&client, *r);
        let (a, b) = (mean(&off, *r), mean(&on, *r));
        eprintln!("{name}: client {c:.1?}, own {a:.1?}, game's {b:.1?}");
    }
}

/// An area of a module (by default the Chapter 1 smithy, `pensmithy1` of
/// `Chapter1.nwm`: an interior at night lit by its tiles' lights) as the
/// client draws it and as Moonglow's area view draws it, lit and with the
/// Lighting switch off, from the scripted camera over the player (at
/// `MG_AREA_PC=x,y`, the area's middle by default). `MG_AREA_MODULE` and
/// `MG_AREA` give another; `MG_AREA_HAKS` the haks it needs (names, from
/// the folders of `MOONGLOW_HAKS`, highest first). The area's creatures
/// are left out (they walk), its placeables, doors and sounds kept. The
/// pictures go to `target/test-output/client_area_lighting/`, with the
/// mean difference from the client's printed; `MG_REGIONS` prints the
/// mean colours and saturations of a 3×3 grid of regions besides.
/// `MG_CLIENT_SETTINGS=owner` runs the client as the project's owner
/// plays (its post effects on, High Quality, 64 lights) and `default` at
/// its defaults (Performance), instead of the tests' settings.
/// `MG_MUTATE` paints Moonglow's game-shader picture magenta (the
/// override's `inc_standard`, after the client ran): that the game's
/// programs draw it.
///
/// Seen on 2026-10-25 (Heed's report that the game's shaders drew an
/// interior darker, grayer and colder than Moonglow's own): in seven
/// stock interiors (the Chapter 1 and XP1 smithies, the Crack'd Anvil,
/// the Yawning Portal, the prison's main floor, a `tdc01` crypt and a
/// `tdm01` mine, from 2 to 111 lights) Moonglow's two shaders drew the
/// same picture (130 pixels of 786,432 apart in the XP1 smithy, by at most
/// 4 of 255), within 2.5 to 15 of 255 of the client's, at the tests'
/// settings, the owner's and the defaults alike (the post effects make
/// the client's picture a little more saturated: 0.02 to 0.07). Where the
/// difference is large the client leaves out rooms it can't see from the
/// player's tile (the tileset's visibility), which Moonglow draws, as
/// Aurora does. With The Domains of Dread's `inc_lighting` (a diffuse
/// wrap) in a hak, the game's shaders draw a tenth darker, as the client
/// does, and Moonglow's own as before.
#[test]
#[ignore]
fn area_lighting_look() {
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
    let area_name = std::env::var("MG_AREA").unwrap_or_else(|_| "pensmithy1".into());
    let dir = scratch_dir("client_area_lighting");
    std::fs::create_dir_all(dir.join("user/hak")).unwrap();
    // `MG_CLIENT_SETTINGS`: `owner` (as the project's owner plays) or
    // `default` (the client's defaults) instead of the tests' settings
    // (the post effects Moonglow doesn't draw off).
    let settings = match std::env::var("MG_CLIENT_SETTINGS").as_deref() {
        Ok("owner") => common::player_settings(true),
        Ok("default") => common::player_settings(false),
        _ => SETTINGS.to_string(),
    };
    std::fs::write(dir.join("user/settings.tml"), settings).unwrap();
    let haks: Vec<String> = std::env::var("MG_AREA_HAKS")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    for name in &haks {
        let file = format!("{name}.hak");
        let from = mg_testkit::builder_hak_dirs()
            .into_iter()
            .map(|d| d.join(&file))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("{file} is in no folder of MOONGLOW_HAKS"));
        mg_testkit::link_or_copy(&from, &dir.join("user/hak").join(&file)).unwrap();
    }
    let install = GameInstall::new(&root, Some(dir.join("user")), "en");
    let mut game = GameData::open(&install).unwrap();
    game.resman.add_haks(&install, &haks).unwrap();
    let from = mg_module::Module::open(&source).unwrap();
    let area = ResRef::from_str(&area_name).unwrap();
    let read = |t| from.gff(&ResKey::new(area, t)).unwrap().unwrap();
    let (mut are, mut git) = (read(ResType::ARE), read(ResType::GIT));
    // The module: a new one with an area of the same tileset and size,
    // made this one (under the new area's name), its scripts left out.
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(&game, "MgScene", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Scene".into(),
        tileset: are.root.resref("Tileset").unwrap(),
        width: are.root.int("Width").unwrap() as u32,
        height: are.root.int("Height").unwrap() as u32,
    };
    let new_area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    are.root.set("ResRef", Value::resref(new_area));
    for label in ["OnEnter", "OnExit", "OnHeartbeat", "OnUserDefined"] {
        are.root.set(label, Value::resref(ResRef::EMPTY));
    }
    git.root.set("Creature List", Value::List(Vec::new()));
    // `MG_AREA_PLACE=row@x,y[@z][;...]`: placeables of those appearances
    // added (a builder's lamps), Illuminated as their blueprints are.
    if let Ok(places) = std::env::var("MG_AREA_PLACE") {
        use mg_module::instances::{Placement, Placing, instance};
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let mut list = git.root.list("Placeable List").map(<[_]>::to_vec).unwrap_or_default();
        for place in places.split(';').filter(|p| !p.trim().is_empty()) {
            let mut it = place.split('@');
            let row: u32 = it.next().unwrap().trim().parse().unwrap();
            let p: Vec<f32> =
                it.next().unwrap().split(',').filter_map(|v| v.trim().parse().ok()).collect();
            let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Placed", 0);
            bp.root.set("Appearance", Value::Dword(row));
            bp.root.set("Static", Value::Byte(1));
            let at = Placement {
                position: [p[0], p[1], p.get(2).copied().unwrap_or(0.0)],
                rotation: 0.0,
            };
            list.push(instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap());
        }
        git.root.set("Placeable List", Value::List(list));
    }
    let (w, h) = (spec.width as f32 * 10.0, spec.height as f32 * 10.0);
    let pc = std::env::var("MG_AREA_PC")
        .ok()
        .and_then(|v| {
            let p: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            (p.len() == 2).then(|| glam::Vec2::new(p[0], p[1]))
        })
        .unwrap_or(glam::Vec2::new(w / 2.0, h / 2.0));
    m.set_gff(ResKey::new(new_area, ResType::ARE), &are).unwrap();
    m.set_gff(ResKey::new(new_area, ResType::GIT), &git).unwrap();
    // (Where the player stands, written to the log: the engine puts it
    // on the walkmesh nearest the entry point.)
    let enter: String = ENTER
        .lines()
        .filter(|l| !l.contains("CreateObject"))
        .map(|l| {
            if l.contains("MG_READY") {
                "    DelayCommand(3.5, Log(\"MG_PC \" + FloatToString(GetPosition(pc).x, 0, 3) + \" \" \
                 + FloatToString(GetPosition(pc).y, 0, 3) + \" \" + FloatToString(GetPosition(pc).z, 0, 3)));\n"
                    .to_string()
                    + l
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ENTRY_X, pc.x);
    info.root.write(&ifo::MOD_ENTRY_Y, pc.y);
    info.root.write(&ifo::MOD_ENTRY_Z, 0.0);
    info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
    if !haks.is_empty() {
        let list = mg_module::attach::hak_list(&info.root, &haks);
        info.root.set("Mod_HakList", list);
    }
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = client_screenshot(&dir, "MgScene");
    if let Some(c) = &client {
        save_png(c, &dir.join("client.png"));
    }
    // Moonglow's area view of the same area.
    if std::env::var_os("MG_MUTATE").is_some() {
        let src = game.resman.get_named("inc_standard", ResType::SHD).unwrap();
        let src = String::from_utf8_lossy(&src).replace(
            "\tApplyDebugModeOutput(FragmentColor);\n}",
            "\tApplyDebugModeOutput(FragmentColor);\n\tFragmentColor = vec4(1.0, 0.0, 1.0, 1.0);\n}",
        );
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/override/inc_standard.shd"), src).unwrap();
        game = GameData::open(&install).unwrap();
        game.resman.add_haks(&install, &haks).unwrap();
    }
    let tileset = mg_area::tileset(&game, spec.tileset).ok();
    let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let log = std::fs::read_to_string(dir.join("user/logs/nwclientLog1.txt")).unwrap_or_default();
    let stood = log.lines().find_map(|l| {
        let p: Vec<f32> =
            l.split("MG_PC ").nth(1)?.split_whitespace().filter_map(|v| v.parse().ok()).collect();
        (p.len() == 3).then(|| Vec3::new(p[0], p[1], p[2]))
    });
    let pc = stood.unwrap_or_else(|| {
        pc.extend(mg_area::walk::Ground::new(&game, &model).height(pc, 100.0).unwrap_or(0.0))
    });
    eprintln!("the player stands at {pc:?} (from the client's log: {})", stood.is_some());
    let camera = fitted_camera(pc, FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let size = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    for (name, lit, fog) in
        [("lit", true, true), ("lit_nofog", true, false), ("unlit", false, false)]
    {
        let view = mg_area::View { lit, fog, ..mg_area::View::of(&model) };
        let scene = area_scene.scene(&model, &view);
        eprintln!("{name}: {} lights, area light {:?}", scene.lights.len(), scene.area);
        off_and_on(&gpu, &game, &scene, &camera, size, client.as_ref(), &dir, name);
    }
    eprintln!("{}: client {}", dir.display(), client.is_some());
}

/// A light node of an authored placeable model (ASCII, put in the scratch
/// user folder's override as the armoire's `plc_a01`).
#[derive(Debug, Clone, Copy)]
struct NodeLight {
    radius: f32,
    multiplier: f32,
    color: [f32; 3],
    priority: u32,
    ambient_only: bool,
    /// Where it is in the model, metres.
    at: [f32; 3],
}

impl NodeLight {
    fn white(radius: f32) -> NodeLight {
        NodeLight {
            radius,
            multiplier: 1.0,
            color: [1.0; 3],
            priority: 2,
            ambient_only: false,
            at: [0.0, 0.0, 1.5],
        }
    }
}

/// An ASCII placeable model named `name` of a small box (drawn) and
/// `lights` under it.
fn light_model(name: &str, lights: &[NodeLight]) -> String {
    let mut s = format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification tile\nsetanimationscale 1\n\
         beginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
         node trimesh box\n  parent {name}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap NULL\n  \
         render 1\n  shadow 0\n  verts 4\n    -0.2 -0.2 0.01\n    0.2 -0.2 0.01\n    \
         0.2 0.2 0.01\n    -0.2 0.2 0.01\n  faces 2\n    0 1 2 1 0 0 0 0\n    0 2 3 1 0 0 0 0\n\
         endnode\n"
    );
    for (i, l) in lights.iter().enumerate() {
        let [r, g, b] = l.color;
        let [x, y, z] = l.at;
        s += &format!(
            "node light mglight{i}\n  parent {name}\n  position {x} {y} {z}\n  \
             orientation 0 0 0 0\n  radius {}\n  multiplier {}\n  color {r} {g} {b}\n  \
             ambientonly {}\n  isdynamic 1\n  affectdynamic 1\n  shadow 0\n  lightpriority {}\n  \
             fadingLight 1\n  flareradius 0\nendnode\n",
            l.radius,
            l.multiplier,
            u8::from(l.ambient_only),
            l.priority
        );
    }
    s + &format!("endmodelgeom {name}\ndonemodel {name}\n")
}

/// L5: the lights of a placeable's own model (`light` nodes: a builder's
/// lamps, the Txpple pack's light nodes), as the client hands them to its
/// shader in a dark area, read back through the debug shader: whether
/// they are lights at all, what the node's `multiplier` does, which a draw
/// keeps past the light limit by `lightpriority`, and what **Illuminated**
/// (`LightState` 0) turns off, a model's lights or only the light of
/// placeables.2da's `LightColor`. Each case is printed beside the light
/// Moonglow makes of it (`mg_render::anim::Lamp::placed`).
/// `MG_LIGHT_CASES` runs only the cases whose names contain it.
#[test]
#[ignore]
fn model_light_uniforms() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let shader = debug_shader(&game);
    let white = NodeLight::white(5.0);
    let green = NodeLight { color: [0.0, 1.0, 0.0], ..NodeLight::white(15.0) };
    // A ring of 36 lights around the placeable, 2 of them red of priority
    // 1 and furthest out, the others green of priority 5.
    let crowd: Vec<NodeLight> = (0..36)
        .map(|i| {
            let turn = i as f32 / 36.0 * std::f32::consts::TAU;
            let (far, red) = if i < 2 { (3.0, true) } else { (1.0 + i as f32 * 0.02, false) };
            NodeLight {
                color: if red { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] },
                priority: if red { 1 } else { 5 },
                at: [far * turn.cos(), far * turn.sin(), 1.0],
                ..NodeLight::white(5.0)
            }
        })
        .collect();
    // (name, the model's lights (none: the stock "Light, White" placeable),
    // LightState (None: absent), static)
    type Case<'a> = (&'a str, Option<Vec<NodeLight>>, Option<u8>, bool);
    let cases: Vec<Case> = vec![
        ("white 5", Some(vec![white]), None, false),
        ("green 15", Some(vec![green]), None, false),
        (
            "green 15 multiplier 10",
            Some(vec![NodeLight { multiplier: 10.0, ..green }]),
            None,
            false,
        ),
        ("white 5 multiplier 2", Some(vec![NodeLight { multiplier: 2.0, ..white }]), None, false),
        (
            "dim 5 multiplier 2",
            Some(vec![NodeLight { multiplier: 2.0, color: [0.3; 3], ..white }]),
            None,
            false,
        ),
        ("white 5 multiplier 0.5", Some(vec![NodeLight { multiplier: 0.5, ..white }]), None, false),
        (
            "orange 5 multiplier 3",
            Some(vec![NodeLight { multiplier: 3.0, color: [1.0, 0.5, 0.0], ..white }]),
            None,
            false,
        ),
        (
            "bright 5 multiplier 2",
            Some(vec![NodeLight { multiplier: 2.0, color: [2.0, 1.0, 1.0], ..white }]),
            None,
            false,
        ),
        (
            "white 1 multiplier 6",
            Some(vec![NodeLight { multiplier: 6.0, at: [0.0, 0.0, 0.3], ..NodeLight::white(1.0) }]),
            None,
            false,
        ),
        ("bright 5", Some(vec![NodeLight { color: [2.0; 3], ..white }]), None, false),
        ("ambient only", Some(vec![NodeLight { ambient_only: true, ..white }]), None, false),
        ("white 5 static", Some(vec![white]), None, true),
        ("white 5 off", Some(vec![white]), Some(0), false),
        ("white 5 off static", Some(vec![white]), Some(0), true),
        ("row light", None, None, true),
        ("row light off", None, Some(0), true),
        ("row light off dynamic", None, Some(0), false),
        ("crowd", Some(crowd), None, false),
    ];
    let only = std::env::var("MG_LIGHT_CASES").ok();
    for (name, lights, light_state, is_static) in cases {
        if only.as_deref().is_some_and(|o| !name.contains(o)) {
            continue;
        }
        let dir = scratch_dir(&format!("client_model_light_{}", name.replace(' ', "_")));
        let light = Lighting { ambient: 0, diffuse: 0, main_light: 0, only_tile: None };
        let (mut m, are) = build_module(&game, &dir, light);
        let enter: String =
            ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
        m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
        let git_key = *m.keys_of(ResType::GIT).next().unwrap();
        let mut git = m.gff(&git_key).unwrap().unwrap();
        let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Light", 0);
        // The armoire's appearance (its model replaced) or "Light, White".
        let row = if lights.is_some() { 0 } else { 15112 };
        bp.root.set("Appearance", Value::Dword(row));
        bp.root.set("Static", Value::Byte(u8::from(is_static)));
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let at = Placement { position: [20.0, 21.0, 0.0], rotation: 0.0 };
        let mut placed = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
        match light_state {
            Some(v) => placed.set("LightState", Value::Byte(v)),
            None => {
                placed.remove("LightState");
            }
        }
        git.root.set("Placeable List", Value::List(vec![placed.clone()]));
        m.set_gff(git_key, &git).unwrap();
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        std::fs::write(dir.join("user/override/inc_standard.shd"), &shader).unwrap();
        let model_name = game
            .table("placeables")
            .unwrap()
            .get(row as usize, "ModelName")
            .map(str::to_ascii_lowercase);
        if let (Some(lights), Some(model_name)) = (&lights, &model_name) {
            std::fs::write(
                dir.join(format!("user/override/{model_name}.mdl")),
                light_model(model_name, lights),
            )
            .unwrap();
        }
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let Some(client) = client_screenshot(&dir, "MgScene") else {
            eprintln!("{name}: the client did not reach the scene");
            continue;
        };
        save_png(&client, &dir.join("client.png"));
        let count = debug_value(&client, 1, 64.0);
        let mut seen = format!(
            "{name}: client: {count:.0} lights ({:.0} without green)",
            debug_value(&client, 31, 64.0)
        );
        for i in 0..(count.round() as usize).min(2) {
            seen += &format!(
                "; light {i}: color ({:.3}, {:.3}, {:.3}) cutoff {:.2} ambient only {:.0}",
                debug_value(&client, 32 + 8 * i, 8.0),
                debug_value(&client, 33 + 8 * i, 8.0),
                debug_value(&client, 34 + 8 * i, 8.0),
                debug_value(&client, 35 + 8 * i, 256.0),
                debug_value(&client, 36 + 8 * i, 1.0),
            );
        }
        eprintln!("{seen}");
        // Moonglow's lights of the same area, as the area view makes them.
        let ours_game =
            GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
        let tileset = mg_area::tileset(&ours_game, ResRef::from_str("tic01").unwrap()).ok();
        let model = mg_area::AreaModel::read(&ours_game, &are.root, &git.root, tileset.as_ref());
        let area_scene = mg_area::AreaScene::new(&gpu, &ours_game, &model);
        let mut ours = area_scene.scene(&model, &mg_area::View::of(&model)).lights;
        // (A draw keeps the 32 most important: by priority, then nearness.)
        ours.sort_by_key(|l| l.priority);
        eprintln!("{name}: Moonglow: {} lights, the first {:?}", ours.len(), ours.first());
        if name == "crowd" {
            assert_eq!((count.round(), debug_value(&client, 31, 64.0).round()), (32.0, 2.0));
            assert_eq!(ours.len(), 36);
            assert_eq!(ours.iter().take(2).filter(|l| l.color.y == 0.0).count(), 2, "{name}");
            continue;
        }
        assert_eq!(count.round() as usize, ours.len(), "{name}: the lights");
        if let Some(l) = ours.first() {
            for (k, channel) in l.color.to_array().into_iter().enumerate() {
                let theirs = debug_value(&client, 32 + k, 8.0);
                assert!((theirs - channel).abs() < 0.005, "{name}: color {k}: {theirs} {channel}");
            }
            // (The read-back ends at 256 m.)
            let theirs = debug_value(&client, 35, 256.0);
            let cutoff_matches =
                if l.cutoff < 250.0 { (theirs - l.cutoff).abs() < 0.05 } else { theirs > 255.0 };
            assert!(cutoff_matches, "{name}: cutoff {theirs} {}", l.cutoff);
            let ambient = debug_value(&client, 36, 1.0).round() == 1.0;
            assert_eq!(ambient, l.ambient_only, "{name}: ambient only");
        }
    }
}

/// Exploration: placeables whose appearance reflects an environment map
/// (placeables.2da `Reflection` `default` or named) and one whose texture
/// does (TXI `envmaptexture default`), static and not, in a lit interior
/// of each tileset of `MG_ENV_TILESETS` (by default `tin01`, whose map is
/// a copper brown, and `tic01`, a gray one), in the client beside
/// Moonglow's area view (the tileset's map the area's) and beside the
/// same scene with no area map (`chrome1` for every `default`). The
/// pictures go to `target/test-output/client_env_<tileset>/`.
/// `MG_ENV_ROWS` gives other appearances (comma-separated);
/// `MG_ENV_SHAPE=sphere` makes the models spheres (a reflection shows all
/// of its map); `MG_ENV_PAINT` paints every environment-mapped mesh with
/// the map bound to it, in the client and Moonglow's game path alike;
/// `MG_ENV_CREATURE=row` adds a creature of that appearance, a sphere.
///
/// Seen on 2026-10-25, the spheres painted: a placeable that isn't static
/// and whose `Reflection` is `default` is bound `chrome1` (silver) in
/// `tin01` and `tic01` alike; a texture's `envmaptexture default` (the
/// armoire's) and a creature's `ENVMAP` `default` (the bone golem's) the
/// tileset's map (copper in `tin01`). Flat cards facing the camera, as at
/// first, reflect one dark spot of either.
#[test]
#[ignore]
fn area_env_map_look() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    // Appearances whose models are made cards 2.2 m square facing the
    // camera, of gray textures whose alpha is their reflectivity where they
    // reflect (0 wholly, 128 half, 255 not at all): the environment map
    // they reflect, plain to see. (appearance, the texture's alpha, a TXI
    // `envmaptexture default`): the big mirror, a mirror and the dwarven
    // mirror (`Reflection` `default`), the armoire (none) with the TXI,
    // and the large treasure (`Reflection` `PLC_G_ENV`). `MG_ENV_ROWS`
    // gives other appearances, drawn as they are.
    let cards: Vec<(u32, u8, bool)> = vec![
        (428, 0, false),
        (447, 128, false),
        (15038, 255, false),
        (0, 0, true),
        (191, 0, false),
    ];
    let table = game.table("placeables").unwrap();
    let model_of = |row: u32| table.get(row as usize, "ModelName").unwrap().to_ascii_lowercase();
    let rows: Vec<u32> = std::env::var("MG_ENV_ROWS")
        .map(|v| v.split(',').filter_map(|r| r.trim().parse().ok()).collect())
        .unwrap_or_else(|_| cards.iter().map(|c| c.0).collect());
    let card = |name: &str, texture: &str| {
        format!(
            "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
             setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
             node trimesh card\n  parent {name}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
             ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {texture}\n  \
             render 1\n  shadow 0\n  verts 4\n    -1.1 0 0.1\n    1.1 0 0.1\n    1.1 0 2.3\n    \
             -1.1 0 2.3\n  tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  faces 4\n    \
             0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\n    0 2 1 1 0 2 1 0\n    0 3 2 1 0 3 2 0\n\
             endnode\nendmodelgeom {name}\ndonemodel {name}\n"
        )
    };
    // `MG_ENV_SHAPE=sphere`: spheres a metre across instead of cards, so
    // that a reflection shows all of its map, as on a curved piece of
    // armor, not one spot of it.
    let sphere = |name: &str, texture: &str| {
        let (rings, segments) = (16usize, 24usize);
        let (mut verts, mut tverts, mut faces) = (String::new(), String::new(), String::new());
        for i in 0..=rings {
            let v = i as f32 / rings as f32;
            let theta = v * std::f32::consts::PI;
            for j in 0..=segments {
                let u = j as f32 / segments as f32;
                let phi = u * std::f32::consts::TAU;
                let (x, y, z) = (theta.sin() * phi.cos(), theta.sin() * phi.sin(), theta.cos());
                verts += &format!("    {} {} {}\n", x * 0.5, y * 0.5, 1.2 + z * 0.5);
                tverts += &format!("    {u} {} 0\n", 1.0 - v);
            }
        }
        let mut n = 0;
        for i in 0..rings {
            for j in 0..segments {
                let a = i * (segments + 1) + j;
                let (b, c, d) = (a + segments + 1, a + segments + 2, a + 1);
                faces +=
                    &format!("    {a} {b} {c} 1 {a} {b} {c} 0\n    {a} {c} {d} 1 {a} {c} {d} 0\n");
                n += 2;
            }
        }
        let count = (rings + 1) * (segments + 1);
        format!(
            "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
             setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
             node trimesh ball\n  parent {name}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
             ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {texture}\n  \
             render 1\n  shadow 0\n  verts {count}\n{verts}  tverts {count}\n{tverts}  faces {n}\n{faces}\
             endnode\nendmodelgeom {name}\ndonemodel {name}\n"
        )
    };
    let spheres = std::env::var("MG_ENV_SHAPE").is_ok_and(|s| s == "sphere");
    let gray = |alpha: u8| {
        let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 16, 0, 32, 8];
        t.extend(std::iter::repeat_n([160u8, 160, 160, alpha], 256).flatten());
        t
    };
    let tilesets = std::env::var("MG_ENV_TILESETS").unwrap_or_else(|_| "tin01,tic01".into());
    for tileset in tilesets.split(',') {
        let dir = scratch_dir(&format!("client_env_{tileset}"));
        let light =
            Lighting { ambient: 0x808080, diffuse: 0xFFFFFF, main_light: 0, only_tile: None };
        let (mut m, are) = build_module_on(&game, &dir, light, tileset, 0);
        let enter: String =
            ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
        m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
        let git_key = *m.keys_of(ResType::GIT).next().unwrap();
        let mut git = m.gff(&git_key).unwrap().unwrap();
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let mut placed = Vec::new();
        // (Each twice: static at the back, not static in front.)
        for (is_static, y) in [(true, 23.0), (false, 19.5)] {
            for (i, &row) in rows.iter().enumerate() {
                let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Shiny", 0);
                bp.root.set("Appearance", Value::Dword(row));
                bp.root.set("Static", Value::Byte(u8::from(is_static)));
                let x = 20.0 + (i as f32 - (rows.len() as f32 - 1.0) / 2.0) * 2.4;
                let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
                placed.push(instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap());
            }
        }
        git.root.set("Placeable List", Value::List(placed));
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        // `MG_ENV_CREATURE=row`: a creature of that appearance (appearance.2da
        // `ENVMAP`), its model made a sphere of the fully reflecting gray,
        // just west of the player.
        let creature_row =
            std::env::var("MG_ENV_CREATURE").ok().and_then(|r| r.parse::<u32>().ok());
        if let Some(row) = creature_row {
            let utc = ResKey::parse("nw_bat", ResType::UTC).unwrap();
            let mut c = Gff::read(&game.resman.get(&utc).unwrap()).unwrap().root;
            c.set("Appearance_Type", Value::Word(row as u16));
            let scripts: Vec<String> = c
                .fields
                .iter()
                .map(|f| f.label.to_string_lossy())
                .filter(|l| l.starts_with("Script"))
                .collect();
            for label in scripts {
                c.set(&label, Value::resref(ResRef::EMPTY));
            }
            let at = Placement { position: [17.0, 21.5, 0.0], rotation: 0.0 };
            git.root.set(
                "Creature List",
                Value::List(vec![instance(&placing, ResType::UTC, &c, at, &[]).unwrap()]),
            );
            let appearance = game.table("appearance").unwrap();
            let model = appearance.get(row as usize, "RACE").unwrap().to_ascii_lowercase();
            std::fs::write(
                dir.join(format!("user/override/{model}.mdl")),
                sphere(&model, "mgshinyc"),
            )
            .unwrap();
            std::fs::write(dir.join("user/override/mgshinyc.tga"), gray(0)).unwrap();
        }
        m.set_gff(git_key, &git).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        // `MG_ENV_PAINT`: every environment-mapped mesh painted with the map
        // bound to it, laid out by its texture coordinates (the client and
        // Moonglow's game path both read this `inc_standard`): which map it
        // is, plain to see.
        if std::env::var_os("MG_ENV_PAINT").is_some() {
            let src = game.resman.get_named("inc_standard", ResType::SHD).unwrap();
            let src = String::from_utf8_lossy(&src).replace("\r\n", "\n").replace(
                "\tApplyDebugModeOutput(FragmentColor);\n}",
                "\tApplyDebugModeOutput(FragmentColor);\n#if SHADER_TYPE == 2 && ENVIRONMENT_MAP == 1\n\
                 \tFragmentColor = vec4(texture(texUnitEnv, vTexCoords.xy).rgb, 1.0);\n#endif\n}",
            );
            std::fs::write(dir.join("user/override/inc_standard.shd"), src).unwrap();
        }
        for &(row, alpha, txi) in &cards {
            if rows.contains(&row) {
                let model = model_of(row);
                let texture = format!("mgshiny{row}");
                let shape = if spheres { sphere(&model, &texture) } else { card(&model, &texture) };
                std::fs::write(dir.join(format!("user/override/{model}.mdl")), shape).unwrap();
                std::fs::write(dir.join(format!("user/override/{texture}.tga")), gray(alpha))
                    .unwrap();
                if txi {
                    std::fs::write(
                        dir.join(format!("user/override/{texture}.txi")),
                        "envmaptexture default\n",
                    )
                    .unwrap();
                }
            }
        }
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let client = client_screenshot(&dir, "MgScene");
        if let Some(c) = &client {
            save_png(c, &dir.join("client.png"));
        }
        let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
        let set = mg_area::tileset(&game, ResRef::from_str(tileset).unwrap()).ok();
        let model = mg_area::AreaModel::read(&game, &are.root, &git.root, set.as_ref());
        let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
        let mut scene =
            area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
        let camera =
            fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
        let size = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
        eprintln!("{tileset}: the area's environment map {:?}", scene.env_map);
        off_and_on(&gpu, &game, &scene, &camera, size, client.as_ref(), &dir, "area_view");
        scene.env_map = None;
        off_and_on(&gpu, &game, &scene, &camera, size, client.as_ref(), &dir, "no_area_map");
    }
}
