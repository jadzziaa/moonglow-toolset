//! Renders an area of a module to PNG, as the area editor first shows it.
//!
//! `cargo run -p mg-corpus-tests --release --example render_area -- MODULE AREA OUT.png
//! [yaw_degrees pitch_degrees distance_fraction]`: MODULE is a `.mod` or
//! `.nwm` path; the camera orbits the area's centre. Fog only with
//! `MG_FOG=1`; Moonglow's own shader unless `MG_SHADERS=1` (the game's, the
//! app's default); the area's lighting unless `MG_LIT=0` (the Lighting
//! switch off); `MG_EYE=x,y,z` and `MG_TARGET=x,y,z` place the camera
//! instead; `MG_HIGH_SHADOWS=1` draws the shadows as stencil volumes (High);
//! `MG_LIGHTS=1` lists the scene's lights; `MG_TIME=<frames>` times
//! the scene's making over that many frames, the clock moving (the median);
//! `MG_TIME_RENDER=<frames>` times its making, its drawing (encoded) and the
//! GPU's work at the frame test's size, the camera turning (the medians).

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
    let view = View { fog, lit, ..View::of(&model) };
    // (`MG_TIME=<frames>`: the scene made that many times, the clock
    // moving on by a 144th of a second each, and the median printed.)
    if let Some(frames) = std::env::var("MG_TIME").ok().and_then(|n| n.parse::<usize>().ok()) {
        let mut took: Vec<std::time::Duration> = (0..frames.max(1))
            .map(|i| {
                let view = View { time: i as f32 / 144.0, ..view };
                let start = std::time::Instant::now();
                std::hint::black_box(scene.scene(&model, &view));
                start.elapsed()
            })
            .collect();
        took.sort();
        eprintln!(
            "scene made in {:.3} ms (median of {frames})",
            took[took.len() / 2].as_secs_f64() * 1e3
        );
    }
    let s = scene.scene(&model, &view);
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
    // (`MG_HIGH_SHADOWS=1`: the shadows of High quality, stencil volumes.)
    r.shadow_volumes = std::env::var("MG_HIGH_SHADOWS").is_ok_and(|v| v == "1");
    // (`MG_TIME_RENDER=<frames>`: the scene made and drawn that many times
    // into a target of the frame test's size (3000 by 1250, 4x MSAA), the
    // camera turning a little each, and the medians printed: the scene's
    // making, the draws' encoding and the GPU's.)
    if let Some(frames) = std::env::var("MG_TIME_RENDER").ok().and_then(|n| n.parse::<usize>().ok())
    {
        let targets = mg_render::Targets::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4, 3000, 1250);
        let wait = || {
            let _ = gpu.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            });
        };
        let (mut made, mut encoded, mut drawn) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..frames.max(1) + 4 {
            let view = View { time: i as f32 / 144.0, ..view };
            let mut camera = camera;
            camera.eye = camera.target
                + glam::Quat::from_rotation_z(i as f32 * 0.002) * (camera.eye - camera.target);
            wait();
            let t0 = std::time::Instant::now();
            let s = scene.scene(&model, &view);
            let t1 = std::time::Instant::now();
            r.render_targets(&gpu, &game.resman, &s, &camera, &targets);
            let t2 = std::time::Instant::now();
            wait();
            let t3 = std::time::Instant::now();
            if i >= 4 {
                made.push(t1 - t0);
                encoded.push(t2 - t1);
                drawn.push(t3 - t2);
            }
        }
        let median = |mut v: Vec<std::time::Duration>| {
            v.sort();
            v[v.len() / 2].as_secs_f64() * 1e3
        };
        eprintln!(
            "scene {:.2} ms, encoded {:.2} ms, GPU {:.2} ms (medians of {frames}); drawn: {:?}",
            median(made),
            median(encoded),
            median(drawn),
            r.drawn
        );
    }
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
