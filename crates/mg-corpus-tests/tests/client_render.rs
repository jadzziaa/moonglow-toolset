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
//! game's GUI.
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
        let ours = r.render_image(&gpu, &game.resman, &scene, &camera, client.width, client.height);
        save_png(&ours, &dir.join("moonglow.png"));
        for (region, rect, tolerance) in REGIONS {
            let (a, b) = (mean(&client, rect), mean(&ours, rect));
            let diff: Vec<f32> = (0..3).map(|c| b[c] - a[c]).collect();
            eprintln!("{name}: {region:12} client {a:5.1?} moonglow {b:5.1?} diff {diff:5.1?}");
            if diff.iter().any(|d| d.abs() > tolerance) {
                failures.push(format!("{name}: {region} differs by {diff:?}"));
            }
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
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let ours = r.render_image(&gpu, &game.resman, &scene, &camera, w, h);
    save_png(&ours, &dir.join("moonglow.png"));
    eprintln!("{}: client {}", dir.display(), client.is_some());
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
/// right to left; 32: top first). Both views go to
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
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let ours = r.render_image(&gpu, &game.resman, &scene, &camera, w, h);
    save_png(&ours, &dir.join("moonglow.png"));
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
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let ours = r.render_image(&gpu, &game.resman, &scene, &camera, w, h);
    save_png(&ours, &dir.join("moonglow.png"));
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
    let user = custom.as_ref().map(|_| dir.join("user"));
    let game = GameData::open(&GameInstall::new(&root, user, "en")).unwrap();
    let light = Lighting { ambient: 0x808080, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
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
    let items: HashMap<ResRef, mg_gff::Struct> = [("mg_armor", &armor), ("mg_cloak", &cloak)]
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
        let mut worn = vec![equip("mg_armor", 0x2), equip("mg_cloak", 0x40)];
        // `MG_WORN`: only the armor (1) or only the cloak (2).
        match std::env::var("MG_WORN").as_deref() {
            Ok("0") => worn.clear(),
            Ok("1") => worn.truncate(1),
            Ok("2") => drop(worn.remove(0)),
            _ => {}
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
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let ours = r.render_image(&gpu, &game.resman, &scene, &camera, w, h);
    save_png(&ours, &dir.join("moonglow.png"));
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
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let ours = r.render_image(&gpu, &game.resman, &scene, &camera, w, h);
    save_png(&ours, &dir.join(format!("moonglow_{mode}.png")));
    eprintln!("{}: client {}", dir.display(), client.is_some());
}
