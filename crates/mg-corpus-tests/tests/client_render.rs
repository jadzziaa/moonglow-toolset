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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
use common::compile;

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

/// Settings for a controlled comparison (merged into the client's defaults).
const SETTINGS: &str = r#"[graphics]
	[graphics.fbo]
		[graphics.fbo.hdr-bloom]
			enabled = false
		[graphics.fbo.high-contrast]
			enabled = false
		[graphics.fbo.sharpen]
			enabled = false
		[graphics.fbo.ssao]
			enabled = false
		[graphics.fbo.vibrance]
			enabled = false
	[graphics.general]
		shader-quality = "High Quality"
	[graphics.grass]
		mode = 0
	[graphics.hilite]
		enabled = false
	[graphics.intro]
		[graphics.intro.splash]
			enabled = false
	[graphics.keyholing]
		enabled = false
	[graphics.lod]
		enabled = false
	[graphics.movies]
		enabled = false
		[graphics.movies.intro]
			enabled = false
	[graphics.shadows]
		[graphics.shadows.creatures]
			mode = 0
		[graphics.shadows.environment]
			enabled = false
	[graphics.skyboxes]
		enabled = false
	[graphics.tile-borders]
		enabled = false
"#;

/// Regions compared (fractions of the image, top-left origin), clear of the
/// game's GUI, with the largest mean difference allowed per channel.
const REGIONS: [(&str, [f32; 4], f32); 6] = [
    ("centre floor", [0.40, 0.47, 0.60, 0.56], 10.0),
    ("left floor", [0.18, 0.45, 0.35, 0.60], 10.0),
    ("right floor", [0.65, 0.45, 0.82, 0.60], 10.0),
    ("far floor", [0.35, 0.28, 0.65, 0.34], 10.0),
    ("armoire", [0.48, 0.39, 0.52, 0.44], 10.0),
    ("left wall", [0.05, 0.30, 0.12, 0.40], 10.0),
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

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs the client until the scene is ready and screenshots its window.
fn client_screenshot(dir: &Path, module: &str) -> Option<Rgba> {
    // In single player the game's server logs to the client's log.
    let log = dir.join("user/logs/nwclientLog1.txt");
    let _ = std::fs::remove_file(&log);
    let mut child = Command::new(repo().join("tools/nwclient/run-client.sh"))
        .arg(dir)
        .args(["+TestNewModule", module])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    let mut ready = false;
    while start.elapsed() < Duration::from_secs(180) {
        if std::fs::read_to_string(&log).is_ok_and(|l| l.contains("MG_READY")) {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    std::thread::sleep(Duration::from_secs(3));
    let shot = dir.join("client.png");
    let ok = ready
        && Command::new("python3")
            .arg(repo().join("tools/aurora/xdrive.py"))
            .args(["winshot", "Neverwinter Nights: Enhanced Edition"])
            .arg(&shot)
            .status()
            .is_ok_and(|s| s.success());
    let _ = child.kill();
    let _ = child.wait();
    ok.then(|| read_png(&shot))
}

fn read_png(path: &Path) -> Rgba {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let data = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgb => {
            data.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect()
        }
        _ => data.to_vec(),
    };
    Rgba { width: info.width, height: info.height, data: rgba }
}

fn save_png(img: &Rgba, path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(&img.data).unwrap();
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
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(game, "MgScene", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Scene".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
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
        ("SkyBox", 0),
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
            // Models face −Y: a facing of f degrees turns them by f + 90.
            Quat::from_rotation_z((270f32 + 90.0).to_radians()),
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
