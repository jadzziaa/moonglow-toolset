//! Renders an area of a module to PNG, as the area editor first shows it.
//!
//! `cargo run -p mg-corpus-tests --release --example render_area -- MODULE AREA OUT.png
//! [yaw_degrees pitch_degrees distance_fraction]`: MODULE is a `.mod` or
//! `.nwm` path; the camera orbits the area's centre. Fog only with
//! `MG_FOG=1`; Moonglow's own shader unless `MG_SHADERS=1` (the game's, the
//! app's default); the area's lighting unless `MG_LIT=0` (the Lighting
//! switch off); `MG_EYE=x,y,z` and `MG_TARGET=x,y,z` place the camera
//! instead; `MG_LIGHTS=1` lists the scene's lights.

use mg_area::{AreaModel, AreaScene, View, overview};
use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_module::Module;
use mg_render::{Camera, Gpu, Renderer};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;

fn vec_var(name: &str) -> Option<glam::Vec3> {
    let v = std::env::var(name).ok()?;
    let p: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    (p.len() == 3).then(|| glam::Vec3::new(p[0], p[1], p[2]))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [module, area, out, rest @ ..] = args.as_slice() else {
        eprintln!("usage: render_area MODULE AREA OUT.png [yaw pitch distance]");
        std::process::exit(2);
    };
    let root = mg_testkit::nwn_root().expect("no game install");
    let install = GameInstall::new(&root, None, "en");
    let m = Module::open(std::path::Path::new(module)).unwrap();
    let mut game = GameData::open(&install).unwrap();
    let (missing, wrong) = m.stack_onto(&mut game.resman, &install);
    assert!(missing.is_empty() && wrong.is_none(), "{missing:?} {wrong:?}");
    let area = ResRef::from_str(area).unwrap();
    let gff = |t| Gff::read(&game.resman.get(&ResKey::new(area, t)).unwrap()).unwrap();
    let (are, git) = (gff(ResType::ARE), gff(ResType::GIT));
    let tileset = are.root.resref("Tileset").and_then(|r| mg_area::tileset(&game, r).ok());
    let model = AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
    eprintln!("problems: {:?}", model.problems);
    let gpu = Gpu::headless().expect("no GPU");
    let scene = AreaScene::new(&gpu, &game, &model);
    eprintln!("missing models: {:?}", scene.missing);
    let fog = std::env::var_os("MG_FOG").is_some();
    let lit = std::env::var("MG_LIT").ok().is_none_or(|v| v != "0");
    let s = scene.scene(&model, &View { fog, lit, ..View::of(&model) });
    if std::env::var_os("MG_LIGHTS").is_some() {
        for l in &s.lights {
            eprintln!(
                "light at {:6.2?} color {:5.3?} cutoff {:6.2} ambient only {} priority {}",
                l.position.to_array(),
                l.color.to_array(),
                l.cutoff,
                l.ambient_only,
                l.priority
            );
        }
        eprintln!("{} lights; area {:?}", s.lights.len(), s.area);
    }
    let mut camera = overview(&model);
    if let [yaw, pitch, distance] = rest {
        let (w, h) = model.size();
        let span = w.max(h);
        let target = glam::Vec3::new(w / 2.0, h / 2.0, 0.0);
        let f = |v: &String| v.parse::<f32>().unwrap();
        camera =
            Camera::orbit(target, span * f(distance), f(yaw).to_radians(), f(pitch).to_radians());
    }
    if let (Some(eye), Some(target)) = (vec_var("MG_EYE"), vec_var("MG_TARGET")) {
        camera = Camera { eye, target, ..camera };
    }
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    r.game_shaders = std::env::var("MG_SHADERS").is_ok_and(|v| v == "1");
    let img = r.render_image(&gpu, &game.resman, &s, &camera, 1280, 800);
    if r.game_shaders {
        eprintln!("game shader failures: {:?}", r.game_shader_failures());
        eprintln!("drawn: {:?}", r.drawn);
    }
    let file = std::fs::File::create(out).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(&img.data).unwrap();
}
