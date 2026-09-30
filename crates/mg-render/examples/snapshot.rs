//! Renders a game model to PNGs from four sides, for looking at by eye:
//! `cargo run -p mg-render --example snapshot -- MODEL OUT_DIR [ANIMATION SECONDS]`.
//! MODEL is a resource name or an .mdl file.

use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_mdl::Model;
use mg_render::{AreaLight, Camera, Gpu, GpuModel, Instance, Renderer, Scene};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (name, out) = (&args[0], std::path::Path::new(&args[1]));
    let root = mg_testkit::nwn_root().expect("no game install");
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let data = if name.ends_with(".mdl") {
        std::fs::read(name).unwrap()
    } else {
        rm.get_named(name, mg_core::ResType::MDL).unwrap().into_owned()
    };
    let model = Arc::new(Model::read(&data).unwrap());
    let gpu = Gpu::headless().expect("no GPU");
    let gm = Arc::new(GpuModel::new(&gpu, model.clone()));
    let mut inst = Instance::new(gm.clone(), Mat4::IDENTITY);
    if let (Some(anim), Some(t)) = (args.get(2), args.get(3).and_then(|t| t.parse::<f32>().ok())) {
        let a = model.animation(anim).expect("no such animation");
        inst.pose = Some(Arc::new(mg_render::anim::pose(&model, a, t)));
        inst.state = Some(Arc::new(mg_render::anim::mesh_state(&gm, a, t)));
    }
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for m in &gm.meshes {
        for c in 0..8 {
            let p = Vec3::new(
                if c & 1 == 0 { m.min.x } else { m.max.x },
                if c & 2 == 0 { m.min.y } else { m.max.y },
                if c & 4 == 0 { m.min.z } else { m.max.z },
            );
            let w = gm.rest[m.node].transform_point3(p);
            (min, max) = (min.min(w), max.max(w));
        }
    }
    let radius = ((max - min).length() * 0.5).max(0.1);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    std::fs::create_dir_all(out).unwrap();
    for (i, yaw) in [-60f32, 30.0, 120.0, 210.0].into_iter().enumerate() {
        let camera: Camera = Camera::orbit(
            (min + max) * 0.5,
            radius / 20f32.to_radians().sin() * 1.1,
            yaw.to_radians(),
            30f32.to_radians(),
        );
        let scene = Scene {
            instances: vec![inst.clone()],
            area: AreaLight::default(),
            background: [0.2, 0.25, 0.3],
            ..Default::default()
        };
        let img = r.render_image(&gpu, &rm, &scene, &camera, 480, 480);
        let file = std::fs::File::create(out.join(format!("{}-{i}.png", model.name))).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.write_header().unwrap().write_image_data(&img.data).unwrap();
    }
}
