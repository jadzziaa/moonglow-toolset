//! Renders an area of a module to PNG, as the area editor first shows it.
//!
//! `cargo run -p mg-corpus-tests --release --example render_area -- MODULE AREA OUT.png
//! [yaw_degrees pitch_degrees distance_fraction]`: MODULE is a `.mod` or
//! `.nwm` path; the camera orbits the area's centre. Fog only with `MG_FOG=1`.

use mg_area::{AreaModel, AreaScene, View, overview};
use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_module::Module;
use mg_render::{Camera, Gpu, Renderer};
use mg_resman::{GameInstall, LayerClass, ResKey, priority};
use mg_rules::GameData;

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
    let haks = m.haks().unwrap();
    game.resman.add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
    game.resman.add(priority::MODULE, "module", LayerClass::Erf, m.container());
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
    let s = scene.scene(&model, &View { fog, ..View::of(&model) });
    let mut camera = overview(&model);
    if let [yaw, pitch, distance] = rest {
        let (w, h) = model.size();
        let span = w.max(h);
        let target = glam::Vec3::new(w / 2.0, h / 2.0, 0.0);
        let f = |v: &String| v.parse::<f32>().unwrap();
        camera =
            Camera::orbit(target, span * f(distance), f(yaw).to_radians(), f(pitch).to_radians());
    }
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let img = r.render_image(&gpu, &game.resman, &s, &camera, 1280, 800);
    let file = std::fs::File::create(out).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(&img.data).unwrap();
}
