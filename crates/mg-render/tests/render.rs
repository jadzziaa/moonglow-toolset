//! The renderer on a GPU (skipped without one): the lighting formula
//! against a CPU evaluation of the same equations, and game models drawn
//! offscreen (written to `target/test-output/render/` to look at).

use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_mdl::{Face, Mesh, Model, Node, NodeKind};
use mg_render::{AreaLight, Camera, Gpu, GpuModel, Instance, NoAssets, Renderer, Scene};

fn gpu() -> Option<Gpu> {
    let g = Gpu::headless();
    if g.is_none() {
        eprintln!("skipped: no GPU adapter");
    }
    g
}

/// A 2×2 quad on the ground, facing up, untextured.
fn quad() -> Model {
    let mesh = Mesh {
        diffuse: [1.0; 3],
        ambient: [1.0; 3],
        render: true,
        vertices: vec![[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 4],
        uvs: [vec![[0.0, 0.0]; 4], vec![], vec![], vec![]],
        faces: vec![
            Face { vertices: [0, 1, 2], material: 0 },
            Face { vertices: [0, 2, 3], material: 0 },
        ],
        source: vec![0, 1, 2, 3],
        ..Default::default()
    };
    let mut root = Node::new("quad", NodeKind::Dummy);
    root.children = vec![1];
    let mut m = Node::new("plane", NodeKind::Mesh(Box::new(mesh)));
    m.parent = Some(0);
    Model { name: "quad".into(), animation_scale: 1.0, nodes: vec![root, m], ..Default::default() }
}

/// The shader's equations for an untextured white surface, one area light
/// and a white environment map (all vectors in view space).
fn expected(n: Vec3, v: Vec3, l: Vec3, ambient: Vec3, diffuse: Vec3) -> Vec3 {
    let fresnel = |s0: f32, c: f32| s0 + (1.0 - s0) * (1.0 - c).powi(5);
    let (spec0, rough) = (0.04f32, 0.55f32);
    let (r2, k) = (rough * rough, rough);
    let n_dot_v = n.dot(v);
    let n_dot_l = n.dot(l);
    let d = diffuse * n_dot_l;
    let v_dot_h = (v.dot(l) * 0.5 + 0.5).sqrt();
    let n_dot_h = ((n_dot_l + n_dot_v) * 0.5 / v_dot_h).clamp(0.0, 1.0);
    let den = n_dot_h * n_dot_h * (r2 - 1.0) + 1.0;
    let g_l = 1.0 / (n_dot_l * (1.0 - k) + k);
    let mut specular = d * (1.0 / (den * den)) * g_l * fresnel(spec0, v_dot_h);
    specular *= r2 * 0.25 / (n_dot_v * (1.0 - k) + k);
    let env_spec = fresnel(spec0, n_dot_v) + (spec0 - fresnel(spec0, n_dot_v)) * rough.sqrt();
    specular += (ambient + d) * env_spec;
    let total = (1.0 - env_spec) * (ambient + d);
    (total + specular).powf(1.0 / 2.2).min(Vec3::ONE)
}

#[test]
fn lighting_matches_the_equations() {
    let Some(gpu) = gpu() else { return };
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(quad())));
    let area = AreaLight {
        ambient: Vec3::new(0.05, 0.04, 0.03),
        diffuse: Vec3::new(0.6, 0.5, 0.4),
        direction: Vec3::new(0.3, -0.2, 1.0).normalize(),
    };
    let scene = Scene {
        instances: vec![Instance { model, transform: Mat4::IDENTITY, pose: None }],
        area,
        ..Default::default()
    };
    let camera = Camera {
        eye: Vec3::new(0.0, -2.0, 4.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let img = r.render_image(&gpu, &NoAssets, &scene, &camera, 64, 64);
    let view = camera.view();
    let n = view.transform_vector3(Vec3::Z).normalize();
    let v = -view.transform_point3(Vec3::ZERO).normalize();
    let l = view.transform_vector3(area.direction).normalize();
    let want = expected(n, v, l, area.ambient, area.diffuse) * 255.0;
    // The image centre is the quad's centre.
    let px = img.pixel(32, 32);
    for c in 0..3 {
        assert!(
            (f32::from(px[c]) - want[c]).abs() <= 2.0,
            "channel {c}: got {px:?}, want {want:?}"
        );
    }
    // Outside the quad: the background.
    assert_eq!(img.pixel(0, 0), [0, 0, 0, 255]);
}

fn save(img: &mg_image::Rgba, path: &std::path::Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&img.data).unwrap();
}

/// A camera that frames a model's rest-pose meshes.
fn framing(model: &GpuModel) -> Camera {
    let pose = mg_render::rest_pose(&model.model);
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for m in &model.meshes {
        for corner in 0..8 {
            let p = Vec3::new(
                if corner & 1 == 0 { m.min.x } else { m.max.x },
                if corner & 2 == 0 { m.min.y } else { m.max.y },
                if corner & 4 == 0 { m.min.z } else { m.max.z },
            );
            let w = pose[m.node].transform_point3(p);
            min = min.min(w);
            max = max.max(w);
        }
    }
    let centre = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(0.1);
    let distance = radius / (20f32.to_radians()).sin() * 1.1;
    Camera::orbit(centre, distance, -60f32.to_radians(), 25f32.to_radians())
}

#[test]
fn game_models_render() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(gpu) = gpu() else { return };
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let dir = mg_testkit::scratch_dir("render");
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    for name in [
        "plc_a01",
        "tcn01_a01_01",
        "t_door09",
        "c_golemerald",
        "wswls_b_061",
        "pmh0_head001",
        "c_wolf",
    ] {
        let data = rm.get_named(name, mg_core::ResType::MDL).unwrap();
        let model = Arc::new(GpuModel::new(&gpu, Arc::new(Model::read(&data).unwrap())));
        let camera = framing(&model);
        let scene = Scene {
            instances: vec![Instance { model, transform: Mat4::IDENTITY, pose: None }],
            area: AreaLight::default(),
            background: [0.2, 0.25, 0.3],
            ..Default::default()
        };
        let img = r.render_image(&gpu, &rm, &scene, &camera, 320, 320);
        save(&img, &dir.join(format!("{name}.png")));
        let bg = img.pixel(0, 0);
        let covered = img.data.as_chunks::<4>().0.iter().filter(|p| p[..3] != bg[..3]).count();
        let share = covered as f32 / (320.0 * 320.0);
        eprintln!("{name}: {:.0}% of the image", share * 100.0);
        assert!(share > 0.02 && share < 0.98, "{name}: {share}");
    }
}
