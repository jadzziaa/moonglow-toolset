//! The renderer on a GPU (skipped without one): the lighting formula
//! against a CPU evaluation of the same equations, and game models drawn
//! offscreen (written to `target/test-output/render/` to look at).

use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_mdl::{Face, Mesh, Model, Node, NodeKind};
use mg_render::{
    AreaLight, Camera, DEPTH_FORMAT, Gpu, GpuModel, Instance, NoAssets, Renderer, Scene, Targets,
};

fn gpu() -> Option<Gpu> {
    mg_testkit::gpu::hold();
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
/// (softened, as at the "High Quality" setting) and a white environment map
/// (all vectors in view space).
fn expected(n: Vec3, v: Vec3, l: Vec3, ambient: Vec3, diffuse: Vec3) -> Vec3 {
    let fresnel = |s0: f32, c: f32| s0 + (1.0 - s0) * (1.0 - c).powi(5);
    let (spec0, rough) = (0.04f32, 0.55f32);
    let (r2, k) = (rough * rough, rough);
    let n_dot_v = n.dot(v);
    let n_dot_l = n.dot(l);
    let smoothstep = |x: f32| {
        let t = (x / 2.0).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let d = diffuse * 2.0 * smoothstep(n_dot_l * 0.8 + 0.2);
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
        // Grazing (N·L ≈ 0.45), where softening shows.
        direction: Vec3::new(1.0, -0.6, 0.6).normalize(),
    };
    let scene =
        Scene { instances: vec![Instance::new(model, Mat4::IDENTITY)], area, ..Default::default() };
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
    let dir = mg_testkit::output_dir("render");
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
            instances: vec![Instance::new(model, Mat4::IDENTITY)],
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

/// The bind pose derived from the rest pose (a bone's model transform
/// inverted, times the skin's), which the renderer uses for ASCII models,
/// is the one compiled models store per node (`qbone_ref_inv`,
/// `tbone_ref_inv`) for 98% of the bones of every skin in the game; the
/// rest (a few oozes and dragon wings) were bound in another pose, and the
/// renderer uses the stored values for compiled models.
#[test]
fn skin_bind_poses_match_the_stored_ones() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let (mut skins, mut bones, mut bad) = (0, 0, Vec::new());
    for resref in rm.list(mg_core::ResType::MDL) {
        let data = rm.get(&mg_resman::ResKey::new(resref, mg_core::ResType::MDL)).unwrap();
        if !mg_mdl::is_binary(&data) {
            continue;
        }
        let model = Model::read(&data).unwrap();
        let rest = mg_render::rest_pose(&model);
        for (i, n) in model.nodes.iter().enumerate() {
            let Some(mg_mdl::MeshExtra::Skin(s)) = n.mesh().map(|m| &m.extra) else { continue };
            if s.inverse_bind.len() != model.nodes.len() {
                continue;
            }
            skins += 1;
            for &b in &s.bones {
                bones += 1;
                let ours = rest[b].inverse() * rest[i];
                let (q, t) = s.inverse_bind[b];
                let stored = Mat4::from_rotation_translation(
                    glam::Quat::from_array(q).normalize(),
                    Vec3::from(t),
                );
                let close = ours
                    .to_cols_array()
                    .iter()
                    .zip(stored.to_cols_array())
                    .all(|(a, b)| (a - b).abs() < 2e-3 * a.abs().max(1.0));
                if !close {
                    // Scale anywhere on the path from the root.
                    let mut scaled = false;
                    let mut at = Some(b);
                    while let Some(k) = at {
                        scaled |= (model.nodes[k].scale - 1.0).abs() > 1e-4;
                        at = model.nodes[k].parent;
                    }
                    bad.push(format!(
                        "{}:{} bone {} (scaled path: {scaled})",
                        model.name, n.name, model.nodes[b].name
                    ));
                }
            }
        }
    }
    eprintln!("{skins} skins, {bones} bones, {} differ", bad.len());
    for b in bad.iter().take(10) {
        eprintln!("  {b}");
    }
    assert!(skins > 300);
    assert!(bad.len() * 50 < bones, "more than 2% of bind poses differ");
}

/// Animations, with supermodels and skins: a creature mid-walk and a
/// placeable opening look different from their rest poses.
#[test]
fn animated_models_render() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(gpu) = gpu() else { return };
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let load = |n: &str| {
        rm.get_named(n, mg_core::ResType::MDL).ok().and_then(|d| Model::read(&d).ok()).map(Arc::new)
    };
    let dir = mg_testkit::scratch_dir("render-anim");
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    for (name, anim, t) in
        [("c_golemerald", "walk", 0.3), ("plc_a01", "open", 0.5), ("c_wolf", "cwalk", 0.3)]
    {
        let model = load(name).unwrap();
        let anims = mg_render::anim::animations(&model, &load);
        let (_, owner) =
            anims.iter().find(|(n, _)| n.eq_ignore_ascii_case(anim)).unwrap_or_else(|| {
                panic!("{name} has no {anim}: {:?}", anims.iter().map(|a| &a.0).collect::<Vec<_>>())
            });
        let pose = Arc::new(mg_render::anim::pose(&model, owner.animation(anim).unwrap(), t));
        let gm = Arc::new(GpuModel::new(&gpu, model));
        let camera = framing(&gm);
        let mut images = Vec::new();
        for p in [None, Some(pose.clone())] {
            let scene = Scene {
                instances: vec![Instance { pose: p, ..Instance::new(gm.clone(), Mat4::IDENTITY) }],
                area: AreaLight::default(),
                background: [0.2, 0.25, 0.3],
                ..Default::default()
            };
            images.push(r.render_image(&gpu, &rm, &scene, &camera, 320, 320));
        }
        save(&images[0], &dir.join(format!("{name}-rest.png")));
        save(&images[1], &dir.join(format!("{name}-{anim}.png")));
        let changed = images[0].data.iter().zip(&images[1].data).filter(|(a, b)| a != b).count();
        eprintln!("{name} {anim}: {changed} bytes differ");
        assert!(changed > 1000, "{name}: {anim} looks like the rest pose");
    }
}

/// Emitters: a brazier's fire and smoke, and a tile's lightning, after three
/// seconds of simulation.
#[test]
fn emitters_render() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(gpu) = gpu() else { return };
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let dir = mg_testkit::scratch_dir("render-particles");
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    // The tile's bolts are 0.2 m strips with a thin line in them, seen from
    // across the whole tile: a few pixels.
    for (name, min_changed) in [("plc_i05", 500), ("tsw01_a03_01", 20)] {
        let Ok(data) = rm.get_named(name, mg_core::ResType::MDL) else { continue };
        let model = Arc::new(Model::read(&data).unwrap());
        let gm = Arc::new(GpuModel::new(&gpu, model.clone()));
        let mut camera = framing(&gm);
        camera.eye = camera.target + (camera.eye - camera.target) * 1.6;
        let mut p = mg_render::particles::Particles::new(&model);
        for _ in 0..30 {
            p.update(&model, None, 0.0, 0.1, &gm.rest, Mat4::IDENTITY);
        }
        let batches = p.batches(&model, None, 0.0, &gm.rest, Mat4::IDENTITY, camera.view());
        let quads: usize = batches.iter().map(|b| b.vertices.len() / 6).sum();
        let scene = |particles| Scene {
            instances: vec![Instance::new(gm.clone(), Mat4::IDENTITY)],
            area: AreaLight::default(),
            background: [0.1, 0.1, 0.12],
            particles,
            ..Default::default()
        };
        let without = r.render_image(&gpu, &rm, &scene(Vec::new()), &camera, 320, 320);
        let with = r.render_image(&gpu, &rm, &scene(batches), &camera, 320, 320);
        save(&with, &dir.join(format!("{name}.png")));
        let changed = without.data.iter().zip(&with.data).filter(|(a, b)| a != b).count();
        eprintln!("{name}: {quads} particles, {changed} bytes changed");
        assert!(quads > 0 && changed > min_changed, "{name}: no visible particles");
    }
}

/// Mesh animations: a water tile's animated mesh (its vertex and UV sets
/// interpolated at the right times) and a placeable whose "on" animation
/// lights a mesh look different from their rest state.
#[test]
fn mesh_animations_render() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(gpu) = gpu() else { return };
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let load = |n: &str| {
        rm.get_named(n, mg_core::ResType::MDL).ok().and_then(|d| Model::read(&d).ok()).map(Arc::new)
    };
    let dir = mg_testkit::scratch_dir("render-mesh-anim");
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    for (name, anim, t) in [("tno01_i69_01", "default", 3.0), ("plc_k01", "on", 0.5)] {
        let model = load(name).unwrap();
        let a = model.animation(anim).unwrap_or_else(|| panic!("{name}: no {anim}"));
        let gm = Arc::new(GpuModel::new(&gpu, model.clone()));
        let state = mg_render::anim::mesh_state(&gm, a, t);
        assert!(
            state.meshes.iter().any(|m| m.vertices.is_some() || m.selfillum.is_some()),
            "{name}: {anim} changes no mesh"
        );
        // Animated vertices: between two sets, halfway at 1.5 periods.
        for (i, m) in state.meshes.iter().enumerate() {
            let Some(v) = &m.vertices else { continue };
            let data = gm.mesh_data(i).unwrap();
            assert_eq!(v.len(), data.vertices.len());
            let node = &model.nodes[gm.meshes[i].node];
            let an = a.nodes.iter().find(|n| n.name.eq_ignore_ascii_case(&node.name)).unwrap();
            let sets = an.anim_mesh.as_ref().unwrap();
            let mid = mg_render::anim::mesh_state(&gm, a, sets.sample_period * 1.5);
            let mid = mid.meshes[i].vertices.as_ref().unwrap();
            let (s, u) = (data.source[0] as usize, data.source_uv[0] as usize);
            let want =
                (Vec3::from(sets.vertex_sets[1][s]) + Vec3::from(sets.vertex_sets[2][s])) / 2.0;
            assert!(Vec3::from(mid[0].pos).distance(want) < 1e-4, "{name}: vertex");
            let want_uv =
                (glam::Vec2::from(sets.uv_sets[1][u]) + glam::Vec2::from(sets.uv_sets[2][u])) / 2.0;
            assert!(glam::Vec2::from(mid[0].uv).distance(want_uv) < 1e-4, "{name}: UV");
        }
        let pose = Arc::new(mg_render::anim::pose(&model, a, t));
        let camera = framing(&gm);
        let mut images = Vec::new();
        for animated in [false, true] {
            let inst = if animated {
                Instance {
                    pose: Some(pose.clone()),
                    state: Some(Arc::new(state.clone())),
                    ..Instance::new(gm.clone(), Mat4::IDENTITY)
                }
            } else {
                Instance::new(gm.clone(), Mat4::IDENTITY)
            };
            let scene = Scene {
                instances: vec![inst],
                area: AreaLight::default(),
                background: [0.2, 0.25, 0.3],
                ..Default::default()
            };
            images.push(r.render_image(&gpu, &rm, &scene, &camera, 320, 320));
        }
        save(&images[1], &dir.join(format!("{name}-{anim}.png")));
        let changed = images[0].data.iter().zip(&images[1].data).filter(|(a, b)| a != b).count();
        eprintln!("{name} {anim}: {changed} bytes differ");
        assert!(changed > 500, "{name}: {anim} looks like the rest state");
    }
}

/// Dangly meshes: a creature's hair and coat lag when it moves, within
/// their limits, and draw bent.
#[test]
fn dangly_meshes_sway() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(gpu) = gpu() else { return };
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let model =
        Arc::new(Model::read(&rm.get_named("c_antoine", mg_core::ResType::MDL).unwrap()).unwrap());
    let gm = Arc::new(GpuModel::new(&gpu, model));
    let mut d = mg_render::dangly::Dangly::new(&gm);
    assert!(!d.is_empty());
    let at = |x: f32| Mat4::from_translation(Vec3::new(x, 0.0, 0.0));
    d.update(0.0, &gm.rest, at(0.0), Vec3::ZERO);
    // Walking pace, then a stop.
    for i in 1..=10 {
        d.update(1.0 / 30.0, &gm.rest, at(i as f32 * 0.05), Vec3::ZERO);
    }
    let offset = d.max_offset(&gm.rest, at(0.5));
    assert!(offset > 1e-3 && offset < 0.05, "offset {offset}");
    let mut state = mg_render::MeshState::new(&gm);
    d.apply(&mut state, &gm.rest, at(0.5));
    let bent = state.meshes.iter().filter(|m| m.vertices.is_some()).count();
    assert!(bent > 0);
    let mut camera = framing(&gm);
    camera.eye += Vec3::X * 0.5;
    camera.target += Vec3::X * 0.5;
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let render = |r: &mut Renderer, state: Option<mg_render::MeshState>| {
        let scene = Scene {
            instances: vec![Instance {
                state: state.map(Arc::new),
                ..Instance::new(gm.clone(), at(0.5))
            }],
            area: AreaLight::default(),
            background: [0.2, 0.25, 0.3],
            ..Default::default()
        };
        r.render_image(&gpu, &rm, &scene, &camera, 320, 320)
    };
    let still = render(&mut r, None);
    let moving = render(&mut r, Some(state));
    let changed = still.data.iter().zip(&moving.data).filter(|(a, b)| a != b).count();
    eprintln!("{bent} dangly meshes bent by up to {offset:.4} m: {changed} bytes differ");
    assert!(changed > 50);
    // Standing still, they settle.
    for _ in 0..600 {
        d.update(1.0 / 30.0, &gm.rest, at(0.5), Vec3::ZERO);
    }
    assert!(!d.moving());
}

/// Textures and MTRs held in memory.
#[derive(Default)]
struct TestAssets {
    textures: std::collections::HashMap<String, mg_image::Texture>,
    materials: std::collections::HashMap<String, mg_image::mtr::Mtr>,
    txis: std::collections::HashMap<String, mg_image::txi::Txi>,
    shaders: std::collections::HashMap<String, String>,
}

impl TestAssets {
    /// A 4×4 texture of one colour.
    fn solid(&mut self, name: &str, rgba: [u8; 4]) {
        let data = rgba.repeat(16);
        let t = mg_image::Rgba { width: 4, height: 4, data }.into_texture(true);
        self.textures.insert(name.into(), t);
    }
}

impl mg_render::Assets for TestAssets {
    fn texture(&self, name: &str) -> Option<mg_render::LoadedTexture> {
        Some(mg_render::LoadedTexture {
            texture: self.textures.get(name)?.clone(),
            txi: self.txis.get(name).cloned().unwrap_or_default(),
            mtr: None,
        })
    }

    fn material(&self, name: &str) -> Option<mg_image::mtr::Mtr> {
        self.materials.get(name).cloned()
    }

    fn txi(&self, name: &str) -> Option<mg_image::txi::Txi> {
        self.txis.get(name).cloned()
    }

    fn shader(&self, name: &str) -> Option<String> {
        self.shaders.get(name).cloned()
    }
}

/// Textures read apart (on threads of their own, as the area view reads a
/// large area's): a model whose texture is being read is left out of the
/// frame, the waker is called once it is read, and the frame after shows
/// the model as one drawn with its texture read in the frame.
#[test]
fn a_model_is_drawn_once_its_textures_are_read_apart() {
    let Some(gpu) = gpu() else { return };
    let assets = || {
        let mut a = TestAssets::default();
        let checker: Vec<u8> = (0..64u32)
            .flat_map(
                |i| if (i + i / 8) % 2 == 0 { [250, 40, 40, 255] } else { [20, 20, 200, 255] },
            )
            .collect();
        a.textures.insert(
            "checker".into(),
            mg_image::Rgba { width: 8, height: 8, data: checker }.into_texture(false),
        );
        a
    };
    let mut m = quad();
    let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
    mesh.uvs[0] = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    mesh.source_uv = vec![0, 1, 2, 3];
    mesh.textures[0] = Some("checker".into());
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(m)));
    let scene = Scene {
        instances: vec![Instance::new(model, Mat4::IDENTITY)],
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    let camera = Camera {
        eye: Vec3::new(0.0, -0.5, 4.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let here = r.render_image(&gpu, &assets(), &scene, &camera, 64, 64);

    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let woken = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let w = woken.clone();
    r.set_game_waker(move || {
        w.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });
    let apart: Arc<dyn mg_render::Assets + Send + Sync> = Arc::new(assets());
    r.read_textures_apart(&gpu, Some(apart));
    r.textures_apart = true;
    let first = r.render_image(&gpu, &NoAssets, &scene, &camera, 64, 64);
    assert_eq!(r.drawn.textures_pending, 1, "left out while its texture is read");
    assert!(first.data.chunks(4).all(|p| p[..3] == [0, 0, 0]), "nothing drawn yet");
    let started = std::time::Instant::now();
    while woken.load(std::sync::atomic::Ordering::Relaxed) == 0 {
        assert!(started.elapsed() < std::time::Duration::from_secs(10), "never read");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    // (The assets the frame is given aren't read: the textures came apart,
    // the environment map's asked for once the texture's TXI said which.)
    let mut read = r.render_image(&gpu, &NoAssets, &scene, &camera, 64, 64);
    while r.drawn.textures_pending > 0 {
        assert!(started.elapsed() < std::time::Duration::from_secs(10), "never read");
        std::thread::sleep(std::time::Duration::from_millis(2));
        read = r.render_image(&gpu, &NoAssets, &scene, &camera, 64, 64);
    }
    assert!(r.textures_read() >= 1);
    assert_eq!(read.data, here.data, "as read in the frame");
}

/// Material maps, as the game's normal-mapped shaders read them: a flat
/// normal map changes nothing, a tilted one turns the surface towards or
/// away from the light along the texture's u axis; a specular map makes
/// the surface shinier; a self-illumination map lights it in the dark; a
/// height map shifts the texture; an MTR's slots and parameters apply.
#[test]
fn material_maps_shade() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    assets.solid("white", [200, 200, 200, 255]);
    assets.solid("flat_n", [128, 128, 255, 255]);
    // Normal (±0.5, 0, 0.87) in tangent space: towards +u or −u.
    assets.solid("plus_u_n", [191, 128, 255, 255]);
    assets.solid("minus_u_n", [64, 128, 255, 255]);
    // Towards +v (green up, as OpenGL normal maps).
    assets.solid("plus_v_n", [128, 191, 255, 255]);
    assets.solid("shiny_s", [255, 255, 255, 255]);
    assets.solid("dull_s", [10, 10, 10, 255]);
    assets.solid("glow_i", [255, 255, 255, 255]);
    // A checkerboard and a height map with a pit on one side.
    let checker: Vec<u8> = (0..64u32)
        .flat_map(|i| {
            if (i % 8 / 2 + i / 16) % 2 == 0 { [250, 250, 250, 255] } else { [20, 20, 20, 255] }
        })
        .collect();
    assets.textures.insert(
        "checker".into(),
        mg_image::Rgba { width: 8, height: 8, data: checker }.into_texture(false),
    );
    let height: Vec<u8> = (0..64u32)
        .flat_map(|i| if i % 8 < 4 { [255, 255, 255, 255] } else { [0, 0, 0, 255] })
        .collect();
    assets.textures.insert(
        "pits_h".into(),
        mg_image::Rgba { width: 8, height: 8, data: height }.into_texture(false),
    );
    let mut mtr = mg_image::mtr::Mtr::default();
    mtr.textures[1] = Some("plus_u_n".into());
    mtr.params.push(("Roughness".into(), mg_image::mtr::Param::Float(vec![0.9])));
    assets.materials.insert("tilted".into(), mtr);

    // The quad with u along +X, lit from +X and above, seen from above;
    // with tangents, when given. With maps or a material it has a render
    // hint: without one the game draws `fslit`, which reads no map.
    let tangents = std::cell::Cell::new(None::<[f32; 4]>);
    let hinted = std::cell::Cell::new(true);
    let model = |bitmap: &str, maps: [Option<&str>; 3], mtr: Option<&str>| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        mesh.uvs[0] = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        mesh.source_uv = vec![0, 1, 2, 3];
        if hinted.get() && (maps.iter().any(Option::is_some) || mtr.is_some()) {
            mesh.renderhint = Some("NormalAndSpecMapped".into());
        }
        if let Some(t) = tangents.get() {
            mesh.tangents = vec![t; 4];
        }
        mesh.textures[0] = Some(bitmap.into());
        for (i, t) in maps.iter().enumerate() {
            mesh.textures[i + 1] = t.map(String::from);
        }
        mesh.material = mtr.map(String::from);
        Arc::new(GpuModel::new(&gpu, Arc::new(m)))
    };
    let camera = Camera {
        eye: Vec3::new(0.0, -0.5, 4.0),
        target: Vec3::ZERO,
        fov_y: 0.6,
        near: 0.1,
        far: 100.0,
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let mut shot = |m: Arc<GpuModel>, area: AreaLight| {
        let scene =
            Scene { instances: vec![Instance::new(m, Mat4::IDENTITY)], area, ..Default::default() };
        r.render_image(&gpu, &assets, &scene, &camera, 64, 64)
    };
    let lit = AreaLight {
        ambient: Vec3::splat(0.02),
        diffuse: Vec3::splat(0.8),
        direction: Vec3::new(1.0, 0.0, 1.0).normalize(),
    };
    let grey = |img: &mg_image::Rgba| {
        let p = img.pixel(32, 32);
        (f32::from(p[0]) + f32::from(p[1]) + f32::from(p[2])) / 3.0
    };
    let plain = grey(&shot(model("white", [None; 3], None), lit));
    let flat = grey(&shot(model("white", [Some("flat_n"), None, None], None), lit));
    let towards = grey(&shot(model("white", [Some("plus_u_n"), None, None], None), lit));
    let away = grey(&shot(model("white", [Some("minus_u_n"), None, None], None), lit));
    eprintln!("plain {plain}, flat normal map {flat}, towards {towards}, away {away}");
    assert!((plain - flat).abs() <= 3.0);
    assert!(towards > flat + 10.0 && away < flat - 10.0);
    // Without a render hint, the maps aren't read (as the client draws
    // such a mesh: `fslit`).
    hinted.set(false);
    let unhinted = grey(&shot(model("white", [Some("plus_u_n"), None, None], None), lit));
    hinted.set(true);
    eprintln!("towards, without a render hint: {unhinted}");
    assert!((unhinted - plain).abs() <= 1.0);

    let shiny = grey(&shot(model("white", [None, Some("shiny_s"), None], None), lit));
    let dull = grey(&shot(model("white", [None, Some("dull_s"), None], None), lit));
    eprintln!("specular map: shiny {shiny}, dull {dull}");
    assert!((shiny - dull).abs() > 5.0);

    // +v is +Y on the quad: lit from +Y, a +v tilt brightens.
    let from_y = AreaLight { direction: Vec3::new(0.0, 1.0, 1.0).normalize(), ..lit };
    let plain_y = grey(&shot(model("white", [None; 3], None), from_y));
    let towards_v = grey(&shot(model("white", [Some("plus_v_n"), None, None], None), from_y));
    eprintln!("lit from +Y: plain {plain_y}, towards +v {towards_v}");
    assert!(towards_v > plain_y + 10.0);

    // The model's own tangents, as the stock shaders take them: along +u
    // they agree with the derivatives' frame; turned to +v, a +u tilt leans
    // towards +Y; a negative sign turns the bitangent to −v.
    tangents.set(Some([1.0, 0.0, 0.0, 1.0]));
    let towards_t = grey(&shot(model("white", [Some("plus_u_n"), None, None], None), lit));
    let towards_v_t = grey(&shot(model("white", [Some("plus_v_n"), None, None], None), from_y));
    tangents.set(Some([0.0, 1.0, 0.0, 1.0]));
    let turned = grey(&shot(model("white", [Some("plus_u_n"), None, None], None), from_y));
    tangents.set(Some([1.0, 0.0, 0.0, -1.0]));
    let mirrored = grey(&shot(model("white", [Some("plus_v_n"), None, None], None), from_y));
    tangents.set(None);
    eprintln!(
        "tangents along +u: {towards_t}, {towards_v_t}; along +v: {turned}; mirrored {mirrored}"
    );
    assert!((towards_t - towards).abs() <= 2.0 && (towards_v_t - towards_v).abs() <= 2.0);
    assert!(turned > plain_y + 10.0);
    assert!(mirrored < plain_y - 10.0);

    let dark = AreaLight { ambient: Vec3::ZERO, diffuse: Vec3::ZERO, direction: Vec3::Z };
    let unlit = grey(&shot(model("white", [None; 3], None), dark));
    assert!(unlit < 5.0, "unlit {unlit}");
    // Self-illumination comes through an MTR (slot 5).
    let mut glow = mg_image::mtr::Mtr::default();
    glow.textures[5] = Some("glow_i".into());
    assets.materials.insert("glow".into(), glow);
    let mut r2 = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let scene = Scene {
        instances: vec![Instance::new(model("white", [None; 3], Some("glow")), Mat4::IDENTITY)],
        area: dark,
        ..Default::default()
    };
    let glowing = grey(&r2.render_image(&gpu, &assets, &scene, &camera, 64, 64));
    eprintln!("self-illumination map in the dark: {glowing}");
    assert!(glowing > 240.0);

    // Height map: the checkerboard shifts and the pits darken; no NaNs
    // (black) on the lit quad.
    let mut pits = mg_image::mtr::Mtr::default();
    pits.textures[4] = Some("pits_h".into());
    assets.materials.insert("pits".into(), pits);
    let mut r3 = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let mut shot3 = |m: Arc<GpuModel>| {
        let scene = Scene {
            instances: vec![Instance::new(m, Mat4::IDENTITY)],
            area: lit,
            ..Default::default()
        };
        r3.render_image(&gpu, &assets, &scene, &camera, 64, 64)
    };
    let flat_checker = shot3(model("checker", [None; 3], None));
    let displaced = shot3(model("checker", [None; 3], Some("pits")));
    let changed = flat_checker.data.iter().zip(&displaced.data).filter(|(a, b)| a != b).count();
    let black = (16..48)
        .flat_map(|y| (16..48).map(move |x| (x, y)))
        .filter(|&(x, y)| displaced.pixel(x, y)[..3] == [0, 0, 0])
        .count();
    eprintln!("height map: {changed} bytes differ, {black} black pixels");
    assert!(changed > 100 && black == 0);

    // An MTR named by the model: its normal map and roughness apply.
    let tilted = grey(&shot3(model("white", [None; 3], Some("tilted"))));
    let plain_again = grey(&shot3(model("white", [None; 3], None)));
    eprintln!("MTR normal map: {tilted} (plain {plain_again})");
    assert!(tilted > plain_again + 10.0);
}

/// Environment maps: a cube map (TXI `cube 1`, faces `name0`…`name5`)
/// reflects its up face on a mirror-like floor; a texture that names no
/// environment map reflects the object's (its alpha the reflectivity)
/// instead of being cut out; the base game's cube maps load.
#[test]
fn environment_maps_reflect() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    let txi = |text: &str| mg_image::txi::Txi::parse(text.as_bytes());
    // Faces +X, -X, +Y, -Y, +Z (up: red), -Z (down: blue).
    let colours =
        [[40, 40, 40], [40, 40, 40], [40, 40, 40], [40, 40, 40], [255, 0, 0], [0, 0, 255]];
    for (i, c) in colours.iter().enumerate() {
        assets.solid(&format!("sky{i}"), [c[0], c[1], c[2], 255]);
    }
    assets.txis.insert("sky".into(), txi("cube 1\nfilerange 6\n"));
    // A mirror: grey, alpha 0 (fully reflective under the legacy rule).
    assets.solid("mirror", [128, 128, 128, 0]);
    assets.txis.insert("mirror".into(), txi("envmaptexture sky\n"));
    assets.solid("plain_clear", [128, 128, 128, 0]);
    assets.shaders.insert("fslit_nm".into(), "#define ENVIRONMENT_MAP 0\n".into());
    assets.shaders.insert("fslit_sm".into(), "#define ENVIRONMENT_MAP 1\n".into());
    let mtr = |text: &str| mg_image::mtr::Mtr::parse(text.as_bytes());
    assets.materials.insert("hair".into(), mtr("customshaderFS fslit_nm\ntransparency 1\n"));
    assets.materials.insert("solid".into(), mtr("customshaderFS fslit_nm\n"));
    assets.materials.insert("shiny".into(), mtr("customshaderFS fslit_sm\ntransparency 1\n"));

    let camera = Camera {
        // Steep: the reflected view points mostly up (+Z).
        eye: Vec3::new(0.0, -0.8, 4.0),
        target: Vec3::ZERO,
        fov_y: 0.6,
        near: 0.1,
        far: 100.0,
    };
    let quad_with = |bitmap: &str| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        mesh.textures[0] = Some(bitmap.into());
        Arc::new(GpuModel::new(&gpu, Arc::new(m)))
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let lit =
        AreaLight { ambient: Vec3::splat(0.05), diffuse: Vec3::splat(0.05), direction: Vec3::Z };
    let mut shot = |inst: Instance| {
        let scene = Scene {
            instances: vec![inst],
            area: lit,
            background: [0.0, 1.0, 0.0],
            ..Default::default()
        };
        r.render_image(&gpu, &assets, &scene, &camera, 64, 64).pixel(32, 32)
    };
    let mirror = shot(Instance::new(quad_with("mirror"), Mat4::IDENTITY));
    eprintln!("mirror floor under a red sky: {mirror:?}");
    // Fully reflective under the legacy rule means metal: the reflection
    // takes the grey albedo's brightness, tinted by the sky.
    assert!(mirror[0] > mirror[2] + 8 && mirror[0] > mirror[1] + 8, "{mirror:?}");

    // No TXI environment map: cut out by its alpha, unless the object has
    // an environment map.
    let clear = shot(Instance::new(quad_with("plain_clear"), Mat4::IDENTITY));
    let object_env = shot(Instance {
        env_map: Some("sky".into()),
        ..Instance::new(quad_with("plain_clear"), Mat4::IDENTITY)
    });
    eprintln!("alpha 0 texture: alone {clear:?}, with the object's environment map {object_env:?}");
    assert_eq!(clear[..3], [0, 255, 0]);
    assert!(object_env[0] > object_env[1] + 8, "{object_env:?}");

    // An MTR naming a shader with the environment map switched off (the
    // game's `fslit_nm`): nothing is reflected. With `transparency` the
    // alpha is see-through again; without, the mesh is solid, in the
    // texture's own color.
    let mut with = |material: &str| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        mesh.textures[0] = Some("plain_clear".into());
        mesh.material = Some(material.into());
        let model = Arc::new(GpuModel::new(&gpu, Arc::new(m)));
        shot(Instance { env_map: Some("sky".into()), ..Instance::new(model, Mat4::IDENTITY) })
    };
    let (hair, solid, shiny) = (with("hair"), with("solid"), with("shiny"));
    eprintln!("no environment map in the shader: hair {hair:?}, solid {solid:?}; with: {shiny:?}");
    assert_eq!(hair[..3], [0, 255, 0], "see-through");
    assert!(solid[1] < 200 && solid[0].abs_diff(solid[1]) < 8, "solid grey: {solid:?}");
    assert!(shiny[0] > shiny[1] + 8, "{shiny:?}");

    // The base game's cube maps: TXI and six faces.
    if let Some(root) = mg_testkit::nwn_root() {
        let rm =
            mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        use mg_render::Assets;
        for name in ["ttr01__env", "tno01__env"] {
            assert!(rm.txi(name).is_some_and(|t| t.cube()), "{name}");
            for i in 0..6 {
                assert!(Assets::texture(&rm, &format!("{name}{i}")).is_some(), "{name}{i}");
            }
        }
    }
}

#[test]
fn the_sky_is_behind_everything_beyond_the_far_plane_and_unfogged() {
    let Some(gpu) = gpu() else { return };
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(quad())));
    // A white plane 500 m below the camera (five times its far plane),
    // the sky; a quad at the origin in front of it; fog everything red past
    // a metre.
    let sky = Instance::new(
        model.clone(),
        Mat4::from_translation(Vec3::new(0.0, 0.0, -500.0)) * Mat4::from_scale(Vec3::splat(2000.0)),
    );
    let scene = Scene {
        instances: vec![Instance::new(model, Mat4::IDENTITY)],
        area: AreaLight { ambient: Vec3::splat(0.2), diffuse: Vec3::ZERO, direction: Vec3::Z },
        fog: Some(mg_render::Fog { start: 0.0, end: 1.0, color: Vec3::new(1.0, 0.0, 0.0) }),
        sky: Some(sky),
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
    // Around the quad: the sky, white (unlit, not fogged).
    assert_eq!(img.pixel(0, 0), [255, 255, 255, 255]);
    // The quad, in front of it: fogged red.
    let px = img.pixel(32, 32);
    assert!(px[0] > 200 && px[1] < 60 && px[2] < 60, "{px:?}");
}

/// The scene's depths stay in the depth buffer: what a caller draws after
/// it, tested against them, is hidden where the scene is nearer.
#[test]
fn depth_is_kept_for_drawing_after_the_scene() {
    let Some(gpu) = gpu() else { return };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let (size, samples) = (64, 4);
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(quad())));
    let scene =
        Scene { instances: vec![Instance::new(model, Mat4::IDENTITY)], ..Default::default() };
    // From above: the quad fills the middle, at a depth of about 0.99.
    let camera = Camera::orbit(Vec3::ZERO, 4.0, 0.0, 1.5);
    let mut renderer = Renderer::new(&gpu, format, samples);
    let targets = Targets::new(&gpu, format, samples, size, size);
    renderer.render(
        &gpu,
        &NoAssets,
        &scene,
        &camera,
        targets.render_view(),
        targets.resolve_view(),
        &targets.depth,
        (size, size),
    );
    // Red over everything at depth 0.9999: behind the quad, before the
    // cleared background.
    let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("after"),
        source: wgpu::ShaderSource::Wgsl(
            "@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
                let p = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                return vec4(p[i], 0.9999, 1.0);
            }
            @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0, 0.0, 0.0, 1.0); }"
                .into(),
        ),
    });
    let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("after"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(format.into())],
        }),
        multiview_mask: None,
        cache: None,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("after"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: targets.render_view(),
                resolve_target: targets.resolve_view(),
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..3, 0..1);
    }
    gpu.queue.submit([encoder.finish()]);
    let img = gpu.read_rgba(&targets.color);
    let px = |x: u32, y: u32| {
        let i = ((y * size + x) * 4) as usize;
        [img.data[i], img.data[i + 1], img.data[i + 2]]
    };
    let red = [255, 0, 0];
    assert_ne!(px(size / 2, size / 2), red, "the quad hides what is behind it");
    assert_eq!(px(1, 1), red, "the background does not");
}

#[test]
fn lines_hide_behind_meshes_but_not_under_the_ground_they_lie_on() {
    let Some(gpu) = gpu() else { return };
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(quad())));
    // The ground, 6 m across, and a 2 m board 1 m above its middle.
    let ground = Instance::new(model.clone(), Mat4::from_scale(Vec3::new(3.0, 3.0, 1.0)));
    let board = Instance::new(model, Mat4::from_translation(Vec3::Z));
    let area = AreaLight { ambient: Vec3::splat(0.3), ..Default::default() };
    // A red line along the ground, under the board.
    let line = mg_render::Line {
        from: Vec3::new(-2.5, 0.0, 0.0),
        to: Vec3::new(2.5, 0.0, 0.0),
        color: [1.0, 0.0, 0.0, 1.0],
    };
    let scene =
        Scene { instances: vec![ground, board], area, lines: vec![line], ..Default::default() };
    let camera = Camera {
        eye: Vec3::new(0.0, -4.0, 6.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let (w, h) = (128, 128);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let img = r.render_image(&gpu, &NoAssets, &scene, &camera, w, h);
    save(&img, &mg_testkit::output_dir("render").join("lines.png"));
    let clip = camera.projection(1.0) * camera.view();
    let red_near = |p: Vec3| {
        let ndc = clip.project_point3(p);
        let x = ((ndc.x + 1.0) / 2.0 * w as f32) as i32;
        let y = ((1.0 - ndc.y) / 2.0 * h as f32) as i32;
        (-2..=2).any(|dy| {
            (-2..=2).any(|dx| {
                let [r, g, _, _] = img.pixel((x + dx) as u32, (y + dy) as u32);
                r > g.saturating_add(100)
            })
        })
    };
    // On the open ground: shown.
    assert!(red_near(Vec3::new(-2.0, 0.0, 0.0)));
    assert!(red_near(Vec3::new(2.0, 0.0, 0.0)));
    // Under the board: hidden.
    assert!(!red_near(Vec3::new(0.0, 0.0, 0.0)));
}

#[test]
fn opacity_makes_an_instance_see_through() {
    let Some(gpu) = gpu() else { return };
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(quad())));
    let camera = Camera {
        eye: Vec3::new(0.0, -0.5, 4.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let area = AreaLight { ambient: Vec3::splat(0.5), ..Default::default() };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let mut shot = |opacity: f32| {
        let instance = Instance { opacity, ..Instance::new(model.clone(), Mat4::IDENTITY) };
        let scene = Scene { instances: vec![instance], area, ..Default::default() };
        r.render_image(&gpu, &NoAssets, &scene, &camera, 32, 32).pixel(16, 16)
    };
    let (whole, half) = (shot(1.0), shot(0.5));
    // Over the black background: about half as bright.
    for c in 0..3 {
        let ratio = f32::from(half[c]) / f32::from(whole[c]).max(1.0);
        assert!((ratio - 0.5).abs() < 0.08, "channel {c}: {whole:?} then {half:?}");
    }
}

/// An unlit instance (an editor's marker) shows its material's colour
/// whatever the light: in the dark too, where a lit one is black.
#[test]
fn unlit_instances_keep_their_colour_in_the_dark() {
    let Some(gpu) = gpu() else { return };
    let mut yellow = quad();
    if let NodeKind::Mesh(m) = &mut yellow.nodes[1].kind {
        m.diffuse = [240.0 / 255.0, 210.0 / 255.0, 40.0 / 255.0];
        m.ambient = m.diffuse;
    }
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(yellow)));
    let camera = Camera {
        eye: Vec3::new(0.0, -0.5, 4.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let dark = AreaLight { ambient: Vec3::ZERO, diffuse: Vec3::ZERO, ..Default::default() };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let mut shot = |unlit: bool| {
        let instance = Instance { unlit, ..Instance::new(model.clone(), Mat4::IDENTITY) };
        let scene = Scene { instances: vec![instance], area: dark, ..Default::default() };
        r.render_image(&gpu, &NoAssets, &scene, &camera, 32, 32).pixel(16, 16)
    };
    let (lit, unlit) = (shot(false), shot(true));
    assert!(lit[..3].iter().all(|&c| c < 8), "lit, in the dark: {lit:?}");
    for (c, want) in [240i32, 210, 40].into_iter().enumerate() {
        assert!((i32::from(unlit[c]) - want).abs() <= 3, "channel {c} of {unlit:?}");
    }
}

/// A see-through mesh never hides what is behind its see-through parts,
/// whatever order the meshes are drawn in: a plant's soft edge over ground
/// whose own texture has an alpha channel, with water under the ground.
/// (Drawn in order of their middles, the plant came first here, and
/// through its edge the water showed instead of the ground: blue-white
/// patches round plants and along shores, at a slant.)
#[test]
fn a_see_through_edge_does_not_hide_the_ground_behind_it() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    let solid = |assets: &mut TestAssets, name: &str, rgba: [u8; 4], alpha: bool| {
        let t = mg_image::Rgba { width: 4, height: 4, data: rgba.repeat(16) }.into_texture(alpha);
        assets.textures.insert(name.into(), t);
    };
    solid(&mut assets, "water", [0, 0, 255, 255], false);
    // (Opaque where the plant is, but with an alpha channel and a texel
    // below opaque in a corner: drawn with the see-through, as the game
    // takes it; all opaque, it would be solid.)
    let mut ground = [200u8, 0, 0, 255].repeat(16);
    ground[3] = 128;
    let t = mg_image::Rgba { width: 4, height: 4, data: ground }.into_texture(true);
    assets.textures.insert("ground".into(), t);
    solid(&mut assets, "plant", [0, 200, 0, 128], true);
    let model = |bitmap: &str| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        mesh.uvs[0] = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        mesh.source_uv = vec![0, 1, 2, 3];
        mesh.textures[0] = Some(bitmap.into());
        Arc::new(GpuModel::new(&gpu, Arc::new(m)))
    };
    let at = |p: Vec3, scale: f32| Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(scale));
    // Seen at a slant: the ground's middle is nearer than the plant on it.
    let plant = Vec3::new(0.0, 0.0, 0.6);
    let camera =
        Camera { eye: Vec3::new(0.0, -8.0, 3.0), target: plant, fov_y: 0.6, near: 0.1, far: 100.0 };
    solid(&mut assets, "lava", [255, 255, 0, 255], false);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let mut shot = |under: &str| {
        let scene = Scene {
            instances: vec![
                Instance::new(model(under), at(Vec3::ZERO, 20.0)),
                Instance::new(model("ground"), at(Vec3::new(0.0, -5.5, 0.5), 6.0)),
                Instance::new(model("plant"), at(plant, 0.5)),
            ],
            area: AreaLight { ambient: Vec3::splat(1.0), ..Default::default() },
            ..Default::default()
        };
        r.render_image(&gpu, &assets, &scene, &camera, 64, 64)
    };
    // What is under the ground makes no difference to the plant on it...
    let (over_water, over_lava) = (shot("water"), shot("lava"));
    assert_eq!(over_water.pixel(32, 32), over_lava.pixel(32, 32), "through the plant");
    // ...though it shows beyond the ground (the test sees it).
    assert!(
        (0..64).any(|y| over_water.pixel(32, y) != over_lava.pixel(32, y)),
        "beyond the ground's far edge"
    );
}

/// What is drawn see-through (a tile about to be placed) shows its nearer
/// surfaces over its farther ones, whatever order its meshes are drawn
/// in: a bridge over water, the bridge's middle farther from the eye than
/// the water's (a builder's report: the water was drawn over the bridge).
#[test]
fn a_see_through_instance_s_near_surfaces_hide_its_far_ones() {
    let Some(gpu) = gpu() else { return };
    let (bridge, water) =
        (colored_quad(&gpu, [1.0, 0.0, 0.0]), colored_quad(&gpu, [0.0, 0.0, 1.0]));
    let camera = Camera {
        eye: Vec3::new(0.0, -1.0, 10.0),
        target: Vec3::ZERO,
        fov_y: 1.4,
        near: 0.1,
        far: 100.0,
    };
    let at = |p: Vec3, scale: f32| Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(scale));
    let ghost = |model: &Arc<GpuModel>, place: Mat4| Instance {
        opacity: 0.6,
        ..Instance::new(model.clone(), place)
    };
    let scene = Scene {
        instances: vec![
            ghost(&bridge, at(Vec3::new(6.0, 0.0, 1.0), 0.5)),
            ghost(&water, at(Vec3::ZERO, 10.0)),
        ],
        area: AreaLight { ambient: Vec3::splat(1.0), ..Default::default() },
        ..Default::default()
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let img = r.render_image(&gpu, &NoAssets, &scene, &camera, 64, 64);
    let pixels: Vec<[u8; 4]> =
        (0..64).flat_map(|y| (0..64).map(move |x| (x, y))).map(|(x, y)| img.pixel(x, y)).collect();
    let over = |p: &[u8; 4], a: usize, b: usize| u16::from(p[a]) > 3 * u16::from(p[b]);
    let (blue, red) = (|p: &[u8; 4]| over(p, 2, 0), |p: &[u8; 4]| over(p, 0, 2));
    assert!(pixels.iter().any(|p| p[2] > 60 && blue(p)), "the water");
    assert!(pixels.iter().any(|p| p[0] > 60 && red(p)), "the bridge, the water not over it");
}

/// A [`quad`] of a colour.
fn colored_quad(gpu: &Gpu, color: [f32; 3]) -> Arc<GpuModel> {
    let mut model = quad();
    if let NodeKind::Mesh(m) = &mut model.nodes[1].kind {
        (m.diffuse, m.ambient) = (color, color);
    }
    Arc::new(GpuModel::new(gpu, Arc::new(model)))
}

/// What the camera can't see isn't drawn, and what it sees a part of is:
/// a quad across the picture's edge shows there, while those far to the
/// side, below the picture and behind the camera are left out.
#[test]
fn meshes_out_of_the_camera_s_sight_are_left_out() {
    let Some(gpu) = gpu() else { return };
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(quad())));
    // From above: the ground from -3.4 to 3.4 each way is in the picture.
    let camera = Camera {
        eye: Vec3::new(0.0, -0.01, 8.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let at = |x: f32, y: f32, z: f32| {
        Instance::new(model.clone(), Mat4::from_translation(Vec3::new(x, y, z)))
    };
    let scene = Scene {
        instances: vec![
            at(0.0, 0.0, 0.0),
            at(3.9, 0.0, 0.0),
            at(20.0, 0.0, 0.0),
            at(0.0, -30.0, 0.0),
            at(0.0, 0.0, 30.0),
        ],
        area: AreaLight { ambient: Vec3::splat(0.5), ..Default::default() },
        ..Default::default()
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let image = r.render_image(&gpu, &NoAssets, &scene, &camera, 64, 64);
    assert_eq!((r.drawn.meshes, r.drawn.hidden), (2, 3), "{:?}", r.drawn);
    let lit = |x: u32| image.pixel(x, 32)[0] > 40;
    assert!(lit(32), "the quad in the middle: {:?}", image.pixel(32, 32));
    assert!(!lit(50), "between the two: {:?}", image.pixel(50, 32));
    assert!(lit(62), "the quad across the edge: {:?}", image.pixel(62, 32));
}

/// Each draw takes its own values (they lie in one list, a draw's found by
/// its number): quads of three colours show each in its own, one of them
/// see-through (three draws, each with values of its own: its depth, and
/// its two parts) between the others.
#[test]
fn every_draw_takes_its_own_values() {
    let Some(gpu) = gpu() else { return };
    let colors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let camera = Camera {
        eye: Vec3::new(0.0, -0.01, 8.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let instances = colors
        .iter()
        .zip([-2.2, 0.0, 2.2])
        .map(|(color, x)| Instance {
            opacity: if x == 0.0 { 0.5 } else { 1.0 },
            ..Instance::new(colored_quad(&gpu, *color), Mat4::from_translation(Vec3::X * x))
        })
        .collect();
    let scene = Scene {
        instances,
        area: AreaLight { ambient: Vec3::splat(0.8), ..Default::default() },
        ..Default::default()
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let image = r.render_image(&gpu, &NoAssets, &scene, &camera, 64, 64);
    assert_eq!(r.drawn.meshes, 5, "{:?}", r.drawn);
    // (Where each quad is in the picture.)
    for (channel, x) in [11, 32, 53].into_iter().enumerate() {
        let pixel = image.pixel(x, 32);
        let others = (0..3).filter(|c| *c != channel).map(|c| pixel[c]).max().unwrap();
        // (Its own channel, well over the others: those have the shine.)
        assert!(pixel[channel] > 60 && others < pixel[channel] / 3, "the quad at {x}: {pixel:?}");
    }
    // The see-through one is the fainter.
    assert!(image.pixel(32, 32)[1] < image.pixel(11, 32)[0], "{:?}", image.pixel(32, 32));
}

/// A model in several places is one draw of as many instances, each with
/// its own values: of three quads of one model, the middle one lit (black,
/// in the dark) and the others not.
#[test]
fn a_model_in_several_places_is_one_draw() {
    let Some(gpu) = gpu() else { return };
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(quad())));
    let camera = Camera {
        eye: Vec3::new(0.0, -0.01, 8.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let instances = [-2.2, 0.0, 2.2]
        .into_iter()
        .map(|x| Instance {
            unlit: x != 0.0,
            ..Instance::new(model.clone(), Mat4::from_translation(Vec3::X * x))
        })
        .collect();
    let dark = AreaLight { ambient: Vec3::ZERO, diffuse: Vec3::ZERO, ..Default::default() };
    let scene = Scene { instances, area: dark, ..Default::default() };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let image = r.render_image(&gpu, &NoAssets, &scene, &camera, 64, 64);
    assert_eq!((r.drawn.meshes, r.drawn.batches), (3, 1), "{:?}", r.drawn);
    let bright = |x: u32| image.pixel(x, 32)[0] > 100;
    assert!(
        bright(11) && !bright(32) && bright(53),
        "{:?}",
        [11, 32, 53].map(|x| image.pixel(x, 32))
    );
}

/// Drawn as instances, a scene looks as it does with a draw for each mesh:
/// a mesh is not drawn before one it lies on or is seen through, though
/// the draws of its model before that one are.
#[test]
fn instances_are_drawn_in_the_order_of_the_meshes_they_meet() {
    let Some(gpu) = gpu() else { return };
    let [white, red, green] =
        [[1.0, 1.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].map(|c| colored_quad(&gpu, c));
    let camera = Camera {
        eye: Vec3::new(0.0, -0.01, 8.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let at = |model: &Arc<GpuModel>, x: f32, y: f32, z: f32, opacity: f32| Instance {
        unlit: true,
        opacity,
        ..Instance::new(model.clone(), Mat4::from_translation(Vec3::new(x, y, z)))
    };
    let scene = Scene {
        instances: vec![
            // On the ground, each lying half on the one before: white, red
            // on it, white on the red; and a white one by itself.
            at(&white, -1.6, 2.0, 0.0, 1.0),
            at(&red, -0.6, 2.0, 0.0, 1.0),
            at(&white, 0.4, 2.0, 0.0, 1.0),
            at(&white, 2.3, -2.3, 0.0, 1.0),
            // See-through, each over half of the one before: green, red,
            // green; and a green one by itself.
            at(&green, -1.6, -0.3, 0.3, 0.5),
            at(&red, -0.6, -0.3, 0.6, 0.5),
            at(&green, 0.4, -0.3, 0.9, 0.5),
            at(&green, -2.3, -2.4, 0.3, 0.5),
        ],
        area: AreaLight { ambient: Vec3::splat(0.5), ..Default::default() },
        ..Default::default()
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let instanced = r.render_image(&gpu, &NoAssets, &scene, &camera, 96, 96);
    let drawn = r.drawn;
    r.instancing = false;
    let one_by_one = r.render_image(&gpu, &NoAssets, &scene, &camera, 96, 96);
    assert_eq!(r.drawn.batches, r.drawn.meshes, "{:?}", r.drawn);
    // Solid: the first white, the red, and the two other whites together.
    // See-through (their fringes: nothing of them is solid): the same of
    // the greens, after their depths. Sixteen meshes (the
    // see-through ones in three parts: the depth and two).
    assert_eq!((drawn.meshes, drawn.batches), (16, 11), "{drawn:?}");
    assert!(instanced.data == one_by_one.data, "the same picture");
    // (And the see-through green over the red is what is seen: more green
    // than red there, as it would not be with the red drawn last.)
    let over = instanced.pixel(48, 52);
    assert!(over[1] > over[0], "green over red: {over:?}");
}

/// A face is seen from its front only, unless its material (MTR) says
/// `twosided 1`: then from behind too, lit as its front is. A cut-out
/// texture (`blending punchthrough`) or `decal 1` does not show a face's
/// back (all as the client draws them: `cutouts_look` in
/// `mg-corpus-tests/tests/client_render.rs`).
#[test]
fn only_a_twosided_material_shows_a_face_from_behind() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    assets.solid("cloth", [200, 200, 200, 255]);
    assets.solid("leaf", [200, 200, 200, 255]);
    assets.solid("glow", [200, 200, 200, 255]);
    let txi = |text: &str| mg_image::txi::Txi::parse(text.as_bytes());
    assets.txis.insert("leaf".into(), txi("blending punchthrough\n"));
    assets.txis.insert("glow".into(), txi("blending punchthrough\ndecal 1\n"));
    let mtr = |text: &str| mg_image::mtr::Mtr::parse(text.as_bytes());
    assets.materials.insert("sided".into(), mtr("texture0 cloth\ntwosided 1\n"));
    assets.materials.insert("plain".into(), mtr("texture0 cloth\n"));
    let area = AreaLight { ambient: Vec3::splat(0.5), diffuse: Vec3::ZERO, ..Default::default() };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    // (The quad faces up: from above its front, from below its back.)
    let mut shot = |texture: &str, material: Option<&str>, eye_z: f32| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        mesh.textures[0] = Some(texture.into());
        mesh.material = material.map(str::to_owned);
        let model = Arc::new(GpuModel::new(&gpu, Arc::new(m)));
        let camera = Camera {
            eye: Vec3::new(0.0, -0.5, eye_z),
            target: Vec3::ZERO,
            fov_y: 0.8,
            near: 0.1,
            far: 100.0,
        };
        let scene = Scene {
            instances: vec![Instance::new(model, Mat4::IDENTITY)],
            area,
            ..Default::default()
        };
        r.render_image(&gpu, &assets, &scene, &camera, 32, 32).pixel(16, 16)
    };
    let background = shot("cloth", None, -4.0);
    for (texture, material) in
        [("cloth", None), ("leaf", None), ("glow", None), ("cloth", Some("plain"))]
    {
        let front = shot(texture, material, 4.0);
        assert_ne!(front, background, "{texture} {material:?} from the front");
        assert_eq!(shot(texture, material, -4.0), background, "{texture} {material:?} from behind");
    }
    let (front, back) = (shot("cloth", Some("sided"), 4.0), shot("cloth", Some("sided"), -4.0));
    assert_ne!(front, background);
    assert_eq!(front, back, "two-sided: the same from both sides");
}

/// A cut-out texture (`blending punchthrough`) is cut where its alpha is
/// 0.2 or less, as any texture is, and solid above: the client's
/// `fAlphaDiscardValue` (`cutouts_look` with `MG_DISCARD=1`). It was cut
/// at 0.5, which thinned leaves' and fences' edges.
#[test]
fn a_cut_out_is_cut_at_the_client_s_alpha() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    for (name, alpha) in [("kept", 77), ("cut", 46), ("solid", 255)] {
        assets.solid(name, [200, 200, 200, alpha]);
        let txi = mg_image::txi::Txi::parse(b"blending punchthrough\n");
        assets.txis.insert(name.into(), txi);
    }
    let area = AreaLight { ambient: Vec3::splat(0.5), diffuse: Vec3::ZERO, ..Default::default() };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let mut shot = |texture: Option<&str>| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        mesh.textures[0] = texture.map(str::to_owned);
        let model = Arc::new(GpuModel::new(&gpu, Arc::new(m)));
        let camera = Camera {
            eye: Vec3::new(0.0, -0.5, 4.0),
            target: Vec3::ZERO,
            fov_y: 0.8,
            near: 0.1,
            far: 100.0,
        };
        let instances = texture.map(|_| Instance::new(model, Mat4::IDENTITY)).into_iter().collect();
        let scene = Scene { instances, area, ..Default::default() };
        r.render_image(&gpu, &assets, &scene, &camera, 32, 32).pixel(16, 16)
    };
    let background = shot(None);
    let solid = shot(Some("solid"));
    assert_ne!(solid, background);
    // (Alpha 0.3: kept, and drawn solid; 0.18: cut.)
    assert_eq!(shot(Some("kept")), solid, "alpha 0.3 is kept, and solid");
    assert_eq!(shot(Some("cut")), background, "alpha 0.18 is cut");
}

/// What is kept of a cut-out (`blending punchthrough`) is solid in the
/// picture, its alpha 1, whichever shader draws it: the game's programs
/// write the texture's alpha out (`fslit`: `gl_FragColor =
/// FragmentColor`), and where it was below 1 the picture was see-through
/// there. The Domains of Dread's fur cards had a white line along every
/// cut edge in the model and area views and in `mgv render` (the window or
/// the image viewer showing what is behind through it); the client shows
/// none (`creatures_look` in `client_render.rs`).
#[test]
fn a_cut_out_is_solid_in_the_picture_by_the_game_s_programs_too() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    for (name, alpha) in [("kept", 77), ("cut", 46)] {
        assets.solid(name, [200, 200, 200, alpha]);
        let txi = mg_image::txi::Txi::parse(b"blending punchthrough\n");
        assets.txis.insert(name.into(), txi);
    }
    // `vslit` and an `fslit` that passes the texture's alpha out and cuts
    // as the game's does.
    authored_lit_pair(&mut assets);
    let fs = r#"
#define SHADER_TYPE 2
uniform sampler2D texUnit0;
uniform float fAlphaDiscardValue;
varying vec2 vUv;
varying vec3 vBlack;
varying vec3 vWhite;
void main ()
{
    vec4 c = texture2D(texUnit0, vUv) * vec4(vWhite + vBlack, 1.0);
    if (fAlphaDiscardValue >= 0.0 && c.a <= fAlphaDiscardValue)
    {
        discard;
    }
    gl_FragColor = c;
}
"#;
    assets.shaders.insert("fslit".into(), fs.into());
    let area = AreaLight { ambient: Vec3::splat(0.5), diffuse: Vec3::ZERO, ..Default::default() };
    for game in [false, true] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        r.game_shaders = game;
        let mut shot = |texture: Option<&str>| {
            let mut m = quad();
            let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
            mesh.textures[0] = texture.map(str::to_owned);
            let model = Arc::new(GpuModel::new(&gpu, Arc::new(m)));
            let camera = Camera {
                eye: Vec3::new(0.0, -0.5, 4.0),
                target: Vec3::ZERO,
                fov_y: 0.8,
                near: 0.1,
                far: 100.0,
            };
            let instances =
                texture.map(|_| Instance::new(model, Mat4::IDENTITY)).into_iter().collect();
            let scene = Scene { instances, area, ..Default::default() };
            let px = r.render_image(&gpu, &assets, &scene, &camera, 32, 32).pixel(16, 16);
            (px, r.drawn.game)
        };
        let (background, _) = shot(None);
        let (kept, drawn) = shot(Some("kept"));
        let (cut, _) = shot(Some("cut"));
        assert_eq!(r.game_shader_failures(), &[]);
        assert_eq!(drawn, usize::from(game), "drawn by the game's programs: {game}");
        assert_ne!(kept, background, "alpha 0.3 is kept (game's programs: {game})");
        assert_eq!(kept[3], 255, "and solid in the picture (game's programs: {game}): {kept:?}");
        assert_eq!(cut, background, "alpha 0.18 is cut (game's programs: {game})");
    }
}

/// A `Punch-Through` particle is cut where its texture's alpha is 0.2 or
/// less and solid above, as the client draws it (`particles_look` with
/// `MG_PARTICLES=punch` in `client_render.rs`): of a texture whose alpha
/// rises evenly across it, four fifths show. It was cut at 0.5.
#[test]
fn a_punch_through_particle_is_cut_at_the_client_s_alpha() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    let ramp: Vec<u8> =
        (0..64 * 64).flat_map(|i| [255, 255, 255, ((i % 64) * 255 / 63) as u8]).collect();
    let ramp = mg_image::Rgba { width: 64, height: 64, data: ramp }.into_texture(true);
    assets.textures.insert("ramp".into(), ramp);
    assets.solid("solid", [255, 255, 255, 255]);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    // One particle, still, a metre across.
    let mut lit = |texture: &str| -> (usize, [u8; 4]) {
        let text = format!(
            "newmodel fx\nsetsupermodel fx NULL\nbeginmodelgeom fx\nnode dummy fx\n  parent NULL\n\
             endnode\nnode emitter one\n  parent fx\n  position 0 0 1\n  update Fountain\n  \
             render Normal\n  blend Punch-Through\n  texture {texture}\n  loop 1\n  xgrid 1\n  \
             ygrid 1\n  spawntype 0\n  birthrate 1\n  lifeExp 100\n  velocity 0\n  randvel 0\n  \
             spread 0\n  mass 0\n  particleRot 0\n  sizeStart 1\n  sizeEnd 1\n  xsize 0\n  ysize 0\n  alphaStart 1\n  \
             alphaEnd 1\n  colorStart 1 1 1\n  colorEnd 1 1 1\nendnode\nendmodelgeom fx\ndonemodel fx\n"
        );
        let model = Arc::new(Model::read(text.as_bytes()).unwrap());
        let gm = Arc::new(GpuModel::new(&gpu, model.clone()));
        let camera = Camera {
            eye: Vec3::new(0.0, -4.0, 1.0),
            target: Vec3::new(0.0, 0.0, 1.0),
            fov_y: 0.8,
            near: 0.1,
            far: 100.0,
        };
        // (Stepped until the first particle is born: one, not turned.)
        let mut p = mg_render::particles::Particles::new(&model);
        let mut particles = Vec::new();
        for _ in 0..200 {
            p.update(&model, None, 0.0, 0.02, &gm.rest, Mat4::IDENTITY);
            particles = p.batches(&model, None, 0.0, &gm.rest, Mat4::IDENTITY, camera.view());
            if particles.iter().any(|b| !b.vertices.is_empty()) {
                break;
            }
        }
        assert_eq!(particles.iter().map(|b| b.vertices.len()).sum::<usize>(), 6, "one particle");
        let scene = Scene {
            instances: vec![Instance::new(gm, Mat4::IDENTITY)],
            background: [0.0, 0.0, 0.0],
            particles,
            ..Default::default()
        };
        let image = r.render_image(&gpu, &assets, &scene, &camera, 256, 256);
        let row: Vec<[u8; 4]> = (0..256).map(|x| image.pixel(x, 128)).collect();
        let shown = row.iter().filter(|p| p[0] > 8).count();
        (shown, *row.iter().max_by_key(|p| p[0]).unwrap())
    };
    let (whole, white) = lit("solid");
    let (cut, kept) = lit("ramp");
    assert!(whole > 60, "the particle shows: {whole} pixels across");
    let share = cut as f32 / whole as f32;
    assert!((share - 0.8).abs() < 0.04, "{cut} of {whole} pixels: {share}");
    assert_eq!(kept, white, "what is kept is solid, not blended");
}

/// A pair standing in for the game's `vslit`/`fslit` (authored here: the
/// game's own are compared in `the_game_s_lit_shaders_match_moonglow_s`):
/// OpenGL's matrices as the engine uploads them, the texture by its
/// engine name, and two varyings of one type that the fragment program
/// uses in the other order (glslang numbers each stage's as they are first
/// used: unmatched, white and black swap, which wgpu can't see).
fn authored_lit_pair(assets: &mut TestAssets) {
    let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mv;
uniform mat4 m_proj;
uniform mat4 m_texture;
attribute vec4 vPos;
attribute vec3 vNormal;
attribute vec2 vTcIn;
varying vec3 vWhite;
varying vec3 vBlack;
varying vec2 vUv;
void main ()
{
    vUv = mat2(m_texture) * vTcIn;
    vWhite = vec3(1.0) + vNormal * 0.0;
    vBlack = vec3(0.0);
    gl_Position = m_proj * (m_mv * vPos);
}
"#;
    let fs = r#"
#define SHADER_TYPE 2
uniform sampler2D texUnit0;
uniform int texture0Bound;
varying vec2 vUv;
varying vec3 vBlack;
varying vec3 vWhite;
void main ()
{
    vec3 t = texture2D(texUnit0, vUv).rgb;
    vec3 c = vBlack;
    c += t * vWhite * float(texture0Bound);
    gl_FragColor = vec4(c, 1.0);
}
"#;
    assets.shaders.insert("vslit".into(), vs.into());
    assets.shaders.insert("fslit".into(), fs.into());
}

/// A quad of `size` standing up at `at`, facing -Y (the camera), with
/// texture `texture`.
fn standing_quad(gpu: &Gpu, texture: &str, at: Vec3, size: f32) -> Instance {
    let mut model = quad();
    if let NodeKind::Mesh(m) = &mut model.nodes[1].kind {
        m.textures[0] = Some(texture.into());
    }
    let model = Arc::new(GpuModel::new(gpu, Arc::new(model)));
    let t = Mat4::from_translation(at)
        * Mat4::from_rotation_x(90f32.to_radians())
        * Mat4::from_scale(Vec3::splat(size));
    Instance::new(model, t)
}

/// The scene for the game path's clip space: from the camera, looking
/// north, a large red quad; nearer, a small blue one before its middle; and
/// a green one just beyond the near plane at the upper left (where OpenGL's
/// depth is -0.9: not taken to wgpu's range, it would be clipped).
fn clip_space_scene(gpu: &Gpu) -> (Scene, Camera) {
    let camera = Camera {
        eye: Vec3::new(0.0, -10.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    // At 0.105 m the picture is 2 × 0.0444 m high.
    let near = 0.105;
    let scene = Scene {
        instances: vec![
            standing_quad(gpu, "blue", Vec3::new(0.0, -3.0, 1.0), 0.5),
            standing_quad(gpu, "red", Vec3::new(0.0, 0.0, 1.0), 3.0),
            standing_quad(gpu, "green", Vec3::new(-0.025, -10.0 + near, 1.025), 0.01),
        ],
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    (scene, camera)
}

impl TestAssets {
    /// A 4×4 texture of one colour, without an alpha channel.
    fn opaque(&mut self, name: &str, rgb: [u8; 3]) {
        let data = [rgb[0], rgb[1], rgb[2], 255].repeat(16);
        let t = mg_image::Rgba { width: 4, height: 4, data }.into_texture(false);
        self.textures.insert(name.into(), t);
    }
}

fn clip_space_assets() -> TestAssets {
    let mut assets = TestAssets::default();
    assets.opaque("red", [255, 0, 0]);
    assets.opaque("blue", [0, 0, 255]);
    assets.opaque("green", [0, 255, 0]);
    authored_lit_pair(&mut assets);
    assets
}

/// The game path draws OpenGL's clip space the right way up and at the
/// right depths: what is high in the scene is at the top of the picture, a
/// mesh at the near plane's side of OpenGL's depth range is drawn, the
/// nearer of two meshes hides the farther (drawn first, it is not drawn
/// over), the varyings arrive under their names and the texture under
/// `texUnit0`, bound. The meshes cover what Moonglow's own shader covers.
#[test]
fn the_game_path_draws_opengl_s_clip_space_right() {
    let Some(gpu) = gpu() else { return };
    let assets = clip_space_assets();
    let (scene, camera) = clip_space_scene(&gpu);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!(r.game_shader_failures(), &[]);
    assert_eq!(r.drawn.game, 3, "{:?}", r.drawn);
    save(&img, &mg_testkit::output_dir("render").join("game_clip_space.png"));
    let px = |x, y| img.pixel(x, y);
    // The middle: the blue quad, in front of the red one.
    assert_eq!(px(32, 32), [0, 0, 255, 255]);
    // Around it, the red one.
    assert_eq!(px(32, 12), [255, 0, 0, 255]);
    // Upper left: the green one at the near plane; not at the lower left.
    assert_eq!(px(14, 14), [0, 255, 0, 255]);
    assert_eq!(px(14, 50), [255, 0, 0, 255]);

    // Moonglow's own shader covers the same pixels with each.
    let mut own = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let theirs = own.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    let which = |p: [u8; 4]| (p[0] > p[1] && p[0] > p[2], p[1] > p[0], p[2] > p[0]);
    let mut differ = 0;
    for y in 0..64 {
        for x in 0..64 {
            if which(img.pixel(x, y)) != which(theirs.pixel(x, y)) {
                differ += 1;
            }
        }
    }
    assert!(differ <= 8, "{differ} pixels differ in what they show");
}

/// The game path on whatever GPU the platform has (CI's Windows runner
/// draws it with Direct3D 12, its macOS one with Metal, as no game install
/// is there for the stock programs): an authored pair written as the
/// engine's shaders are (loose uniforms, a `sampler2D` passed to a
/// function, varyings, an include), compiled by mg-shader, its uniforms in
/// the draw's block (`texture0Bound`, `materialFrontDiffuse`), the frame's
/// (`screenParams`, `m_proj`) and the lights' arrays, drawn and its pixels
/// read back. Prints the adapter it ran on to the test's output whether or
/// not output is captured, and fails rather than skip where an adapter is
/// there but the path doesn't draw.
#[test]
fn an_authored_glsl_pair_draws_on_this_platform_s_gpu() {
    use std::io::Write;
    let Some(gpu) = gpu() else {
        let _ = writeln!(std::io::stdout(), "game path: skipped, no GPU adapter");
        return;
    };
    let info = gpu.device.adapter_info();
    let mut assets = TestAssets::default();
    assets.opaque("red", [255, 0, 0]);
    let inc = r#"
#ifndef INC_PLATFORM
#define INC_PLATFORM
uniform mat4 m_mv;
uniform mat4 m_proj;
uniform mat4 m_texture;
uniform vec4 screenParams;
uniform vec4 materialFrontDiffuse;
uniform int texture0Bound;
uniform int numLights;
uniform vec4 lightColor[MAX_NUM_LIGHTS];
varying vec2 vUv;
varying vec4 vTint;
#endif
"#;
    let vs = r#"
#define SHADER_TYPE 1
#include "inc_platform"
attribute vec4 vPos;
attribute vec2 vTcIn;
// (The engine's baked light: Moonglow has none, its zeros.)
attribute vec4 vColor;
void main ()
{
    vUv = mat2(m_texture) * vTcIn;
    vTint = materialFrontDiffuse + vColor;
    gl_Position = m_proj * (m_mv * vPos);
}
"#;
    let fs = r#"
#define SHADER_TYPE 2
#include "inc_platform"
uniform sampler2D texUnit0;
vec3 Sample(sampler2D tex, vec2 uv)
{
    return texture2D(tex, uv).rgb * float(texture0Bound);
}
void main ()
{
    vec3 c = Sample(texUnit0, vUv) * vTint.rgb;
    for (int i = 0; i < numLights; i++)
        c += lightColor[i].rgb;
    // (The frame's block: the picture's width, 64 pixels.)
    c.b += screenParams.x / 64.0;
    gl_FragColor = vec4(c, 1.0);
}
"#;
    assets.shaders.insert("inc_platform".into(), inc.into());
    assets.shaders.insert("vslit".into(), vs.into());
    assets.shaders.insert("fslit".into(), fs.into());
    let camera = Camera {
        eye: Vec3::new(0.0, -10.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    // One quad, and the same mesh twice more to its sides: three
    // instances of one draw, each in its own place.
    let quad = standing_quad(&gpu, "red", Vec3::new(0.0, 0.0, 1.0), 1.0);
    let beside = |x: f32| crate::Instance {
        transform: Mat4::from_translation(Vec3::new(x, 0.0, 0.0)) * quad.transform,
        ..quad.clone()
    };
    // Lights that reach the quad on the left (one) and the one on the
    // right (two), each its own green: the program reads each instance's
    // lights through its places in the frame's (`mg_shader::LIGHT_TABLE`).
    let light = |x: f32, green: f32| mg_render::PointLight {
        position: Vec3::new(x, -1.0, 1.0),
        color: Vec3::new(0.0, green, 0.0),
        cutoff: 1.0,
        ambient_only: false,
        priority: 1,
    };
    let scene = Scene {
        instances: vec![beside(-3.0), quad.clone(), beside(3.0)],
        lights: vec![light(3.0, 0.5), light(-3.0, 0.25), light(3.0, 0.25)],
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    let _ = writeln!(
        std::io::stdout(),
        "game path: {} ({:?}, {:?}): {} of the game's draws, failures {:?}, middle pixel {:?}",
        info.name,
        info.backend,
        info.device_type,
        r.drawn.game,
        r.game_shader_failures(),
        img.pixel(32, 32),
    );
    assert_eq!(r.game_shader_failures(), &[]);
    assert_eq!((r.drawn.game, r.drawn.batches), (3, 1), "{:?}", r.drawn);
    save(&img, &mg_testkit::scratch_dir("render-platform").join("game_platform.png"));
    // The texture's red (bound, through the function), the material's
    // white, the lights' green (none in the middle), and the frame's width
    // as blue: in the middle and in each instance beside it.
    for (x, green) in [(32, 0), (9, 64), (55, 191)] {
        let p = img.pixel(x, 32);
        assert_eq!([p[0], p[2], p[3]], [255, 255, 255], "{x}: {p:?}");
        assert!(p[1].abs_diff(green) <= 1, "{x}: {p:?}, green {green}");
    }
    // Between and around the quads, the background.
    for (x, y) in [(1, 1), (20, 32), (44, 32)] {
        assert_eq!(img.pixel(x, y), [0, 0, 0, 255], "{x}, {y}");
    }
}

/// A program that doesn't compile is drawn with Moonglow's own shader
/// (the picture is the same as without the game's shaders) and reported
/// once, however many frames.
#[test]
fn a_program_that_does_not_compile_falls_back() {
    let Some(gpu) = gpu() else { return };
    let mut assets = clip_space_assets();
    assets
        .shaders
        .insert("fslit".into(), "#define SHADER_TYPE 2\nvoid main() { gl_FragColor = ; }\n".into());
    let (scene, camera) = clip_space_scene(&gpu);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    let again = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!(r.drawn.game, 0);
    let failures = r.game_shader_failures();
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(failures[0].0, "vslit/fslit");
    assert!(failures[0].1.contains("fslit"), "{failures:?}");
    let mut own = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let theirs = own.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert!(img.data == theirs.data && again.data == theirs.data);
}

/// A UV sphere of radius 1 (`rings` × `segments`), textured `texture`.
fn sphere(texture: &str, rings: u32, segments: u32) -> Model {
    let (mut vertices, mut normals, mut uvs, mut faces) = (vec![], vec![], vec![], vec![]);
    for r in 0..=rings {
        let theta = std::f32::consts::PI * r as f32 / rings as f32;
        for s in 0..=segments {
            let phi = std::f32::consts::TAU * s as f32 / segments as f32;
            let n = [theta.sin() * phi.cos(), theta.sin() * phi.sin(), theta.cos()];
            vertices.push(n);
            normals.push(n);
            uvs.push([s as f32 / segments as f32 * 2.0, r as f32 / rings as f32]);
        }
    }
    let at = |r: u32, s: u32| r * (segments + 1) + s;
    for r in 0..rings {
        for s in 0..segments {
            let (a, b, c, d) = (at(r, s), at(r + 1, s), at(r + 1, s + 1), at(r, s + 1));
            // Counter-clockwise seen from outside.
            faces.push(Face { vertices: [a, b, c], material: 0 });
            faces.push(Face { vertices: [a, c, d], material: 0 });
        }
    }
    let n = vertices.len() as u32;
    let mesh = Mesh {
        diffuse: [0.9, 0.85, 0.8],
        ambient: [1.0; 3],
        render: true,
        textures: [Some(texture.into()), None, None, None],
        vertices,
        normals,
        uvs: [uvs, vec![], vec![], vec![]],
        faces,
        source: (0..n).collect(),
        ..Default::default()
    };
    let mut root = Node::new("sphere", NodeKind::Dummy);
    root.children = vec![1];
    let mut m = Node::new("ball", NodeKind::Mesh(Box::new(mesh)));
    m.parent = Some(0);
    Model {
        name: "sphere".into(),
        animation_scale: 1.0,
        nodes: vec![root, m],
        ..Default::default()
    }
}

/// The authored textures, the install's shaders.
struct WithGameShaders<'a> {
    own: TestAssets,
    game: &'a mg_resman::ResMan,
}

impl mg_render::Assets for WithGameShaders<'_> {
    fn texture(&self, name: &str) -> Option<mg_render::LoadedTexture> {
        self.own.texture(name).or_else(|| mg_render::Assets::texture(self.game, name))
    }

    fn material(&self, name: &str) -> Option<mg_image::mtr::Mtr> {
        self.own.material(name).or_else(|| mg_render::Assets::material(self.game, name))
    }

    fn txi(&self, name: &str) -> Option<mg_image::txi::Txi> {
        self.own.txi(name).or_else(|| mg_render::Assets::txi(self.game, name))
    }

    fn shader(&self, name: &str) -> Option<String> {
        self.own.shader(name).or_else(|| mg_render::Assets::shader(self.game, name))
    }
}

/// How two pictures differ where either shows something other than the
/// background: per channel (0–255), the mean, the 99th percentile and the
/// largest difference, and over how many pixels; pixels where either has
/// a channel at 255 (what the game's post pass, which neither runs, would
/// tone down: Moonglow's shader clamps them as it does) are left out and
/// counted.
struct Difference {
    pixels: usize,
    saturated: usize,
    mean: f32,
    p99: u8,
    max: u8,
}

fn difference(
    a: &mg_image::Rgba,
    b: &mg_image::Rgba,
    background: [u8; 4],
) -> (Difference, mg_image::Rgba) {
    let mut diffs = Vec::new();
    let mut saturated = 0;
    let mut picture =
        mg_image::Rgba { width: a.width, height: a.height, data: vec![0; a.data.len()] };
    for (i, (p, q)) in a.data.as_chunks::<4>().0.iter().zip(b.data.as_chunks::<4>().0).enumerate() {
        if *p == background && *q == background {
            continue;
        }
        if p[..3].contains(&255) || q[..3].contains(&255) {
            saturated += 1;
            continue;
        }
        let d: [u8; 3] = std::array::from_fn(|c| p[c].abs_diff(q[c]));
        diffs.extend_from_slice(&d);
        for (c, d) in d.iter().enumerate() {
            picture.data[i * 4 + c] = d.saturating_mul(16);
        }
        picture.data[i * 4 + 3] = 255;
    }
    let mut sorted = diffs.clone();
    sorted.sort_unstable();
    let pct = |q: f32| {
        sorted
            .get(((sorted.len() as f32 * q) as usize).min(sorted.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    };
    let mean = diffs.iter().map(|&d| f32::from(d)).sum::<f32>() / diffs.len().max(1) as f32;
    (
        Difference {
            pixels: diffs.len() / 3,
            saturated,
            mean,
            p99: pct(0.99),
            max: sorted.last().copied().unwrap_or(0),
        },
        picture,
    )
}

/// The WGSL port's oracle: the game's own `vslit`/`fslit` (and `_nm`,
/// `_sm`, `_sm_nm` where a game model asks for them), compiled from the
/// install, against Moonglow's WGSL shader, on the same scenes: a textured
/// sphere on a textured ground in an area light, a point light and fog;
/// then a tile, a placeable and a skinned creature of the game's. Pictures
/// of both and of their difference (×16) are written to
/// `target/test-output/render/game_*.png`.
#[test]
fn the_game_s_lit_shaders_match_moonglow_s() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(gpu) = gpu() else { return };
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let dir = mg_testkit::output_dir("render");
    let mut own = TestAssets::default();
    // A checker of two colours with a gradient across it, no alpha.
    let (w, h) = (64u32, 64u32);
    let mut data = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let check = ((x / 8) + (y / 8)) % 2 == 0;
            let g = (y * 255 / h) as u8;
            data.extend_from_slice(&if check { [200, 120, g / 2, 255] } else { [60, g, 170, 255] });
        }
    }
    own.textures.insert(
        "mgchecker".into(),
        mg_image::Rgba { width: w, height: h, data: data.clone() }.into_texture(false),
    );
    // The same, its alpha the environment map's share (1 - reflectivity)
    // in every other check.
    let mut reflective = data;
    for (i, p) in reflective.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (x, y) = (i as u32 % w, i as u32 / w);
        p[3] = if ((x / 8) + (y / 8)) % 2 == 0 { 255 } else { 90 };
    }
    own.textures.insert(
        "mgshiny".into(),
        mg_image::Rgba { width: w, height: h, data: reflective }.into_texture(true),
    );
    // An environment map: a sky over a dark ground.
    let env: Vec<u8> = (0..32 * 32)
        .flat_map(|i| if i / 32 < 16 { [90, 140, 220, 255] } else { [70, 50, 30, 255] })
        .collect();
    own.textures.insert(
        "mgenv".into(),
        mg_image::Rgba { width: 32, height: 32, data: env }.into_texture(false),
    );
    // A cube map: six faces of 64 texels, each a checker of its own
    // colours (so that the level it is read at shows).
    own.txis.insert("mgcube".into(), mg_image::txi::Txi::parse(b"cube 1\nfilerange 6\n"));
    for face in 0..6u8 {
        let face_data: Vec<u8> = (0..64 * 64u32)
            .flat_map(|i| {
                let check = ((i % 64) / 4 + (i / 64) / 4) % 2 == 0;
                let c = 40 + face * 35;
                if check { [c, 255 - c, 120, 255] } else { [220, c / 2, 255 - c, 255] }
            })
            .collect();
        own.textures.insert(
            format!("mgcube{face}"),
            mg_image::Rgba { width: 64, height: 64, data: face_data }.into_texture(false),
        );
    }
    // A normal map of ridges along u, and a specular map of stripes.
    let normal: Vec<u8> = (0..w * h)
        .flat_map(|i| {
            let x = (i % w) as f32 / w as f32 * std::f32::consts::TAU * 4.0;
            [((0.3 * x.sin() + 0.5) * 255.0) as u8, 128, 255, 255]
        })
        .collect();
    own.textures.insert(
        "mgnormal".into(),
        mg_image::Rgba { width: w, height: h, data: normal }.into_texture(false),
    );
    let spec: Vec<u8> = (0..w * h)
        .flat_map(|i| if (i / w) % 16 < 8 { [200, 200, 200, 255] } else { [20, 20, 20, 255] })
        .collect();
    own.textures.insert(
        "mgspec".into(),
        mg_image::Rgba { width: w, height: h, data: spec }.into_texture(false),
    );
    // A decal: the checker, its TXI `decal 1`.
    own.textures.insert("mgdecal".into(), own.textures["mgchecker"].clone());
    own.txis.insert("mgdecal".into(), mg_image::txi::Txi::parse(b"decal 1\n"));
    let assets = WithGameShaders { own, game: &rm };

    let ball = Arc::new(GpuModel::new(&gpu, Arc::new(sphere("mgchecker", 24, 48))));
    let mut ground = quad();
    if let NodeKind::Mesh(m) = &mut ground.nodes[1].kind {
        m.textures[0] = Some("mgchecker".into());
        m.uvs[0] = vec![[0.0, 0.0], [3.0, 0.0], [3.0, 3.0], [0.0, 3.0]];
        m.diffuse = [0.8; 3];
    }
    let ground = Arc::new(GpuModel::new(&gpu, Arc::new(ground)));
    let area = AreaLight {
        ambient: Vec3::new(0.08, 0.08, 0.1),
        diffuse: Vec3::new(0.45, 0.42, 0.38),
        direction: Vec3::new(0.4, -0.5, 0.75).normalize(),
    };
    let light = mg_render::PointLight::new(
        Vec3::new(-1.8, -1.6, 1.2),
        Vec3::new(1.0, 0.75, 0.5),
        6.0,
        false,
        2,
    );
    let fog = mg_render::Fog { start: 6.0, end: 16.0, color: Vec3::new(0.45, 0.5, 0.55) };
    let base = Scene {
        area,
        lights: vec![light],
        fog: Some(fog),
        // (Magenta: what is left of it is the background.)
        background: [1.0, 0.0, 1.0],
        ..Default::default()
    };
    let sphere_scene = Scene {
        instances: vec![
            Instance::new(ball, Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0))),
            Instance::new(ground, Mat4::from_scale(Vec3::new(5.0, 5.0, 1.0))),
        ],
        ..base.clone()
    };
    let camera =
        Camera::orbit(Vec3::new(0.0, 0.0, 0.8), 7.0, -70f32.to_radians(), 28f32.to_radians());
    // A sphere that reflects (`_sm`), and one normal- and specular-mapped
    // with tangents (`_nm`), in the same light.
    let close =
        Camera::orbit(Vec3::new(0.0, 0.0, 1.0), 3.5, -70f32.to_radians(), 20f32.to_radians());
    let mut shiny = Instance::new(
        Arc::new(GpuModel::new(&gpu, Arc::new(sphere("mgshiny", 24, 48)))),
        Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    );
    shiny.env_map = Some("mgenv".into());
    // (The same reflecting a cube map: its level worked out in Moonglow's
    // shader, as Metal can't be given a cube's gradients. Cube maps are
    // uploaded with one level, so both read it at 0 for now.)
    let shiny_cube = Instance { env_map: Some("mgcube".into()), ..shiny.clone() };
    let mut bumpy = sphere("mgchecker", 24, 48);
    if let NodeKind::Mesh(m) = &mut bumpy.nodes[1].kind {
        m.textures[1] = Some("mgnormal".into());
        m.textures[2] = Some("mgspec".into());
        // (Its maps read with a render hint, as the game reads them.)
        m.renderhint = Some("normalandspecmapped".into());
        // Along +u, around the axis.
        m.tangents = m
            .vertices
            .iter()
            .map(|v| {
                let t = Vec3::new(-v[1], v[0], 0.0).normalize_or(Vec3::X);
                [t.x, t.y, t.z, 1.0]
            })
            .collect();
    }
    let bumpy = Instance::new(
        Arc::new(GpuModel::new(&gpu, Arc::new(bumpy))),
        Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    );
    // M4: a decal (`vsvt`/`fst`, unlit); a sphere without texture
    // coordinates (`vslitnotex`); a tile's ground turned a quarter with
    // its texture (`rotatetexture`: `m_texture`, and Moonglow's turn).
    let mut decal = quad();
    let mut bare = sphere("mgchecker", 24, 48);
    let mut turned = quad();
    for (m, texture) in
        [(&mut decal, "mgdecal"), (&mut bare, "mgchecker"), (&mut turned, "mgchecker")]
    {
        if let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind {
            mesh.textures[0] = Some(texture.into());
            if texture == "mgdecal" || mesh.vertices.len() == 4 {
                mesh.uvs[0] = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]];
            } else {
                mesh.uvs[0].clear();
            }
            mesh.rotate_texture = true;
        }
    }
    let decal = Instance::new(
        Arc::new(GpuModel::new(&gpu, Arc::new(decal))),
        Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    );
    let bare = Instance::new(
        Arc::new(GpuModel::new(&gpu, Arc::new(bare))),
        Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)),
    );
    let turned = Instance {
        tile: true,
        ..Instance::new(
            Arc::new(GpuModel::new(&gpu, Arc::new(turned))),
            Mat4::from_rotation_translation(
                glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                Vec3::new(0.0, 0.0, 1.0),
            ),
        )
    };
    let mut scenes = vec![
        ("sphere", sphere_scene, camera),
        ("reflecting", Scene { instances: vec![shiny], ..base.clone() }, close),
        ("reflecting_cube", Scene { instances: vec![shiny_cube], ..base.clone() }, close),
        ("normal_mapped", Scene { instances: vec![bumpy], ..base.clone() }, close),
        ("decal", Scene { instances: vec![decal], ..base.clone() }, close),
        ("no_uvs", Scene { instances: vec![bare], ..base.clone() }, close),
        ("tile_turned", Scene { instances: vec![turned], ..base.clone() }, close),
    ];
    for name in ["tcn01_a01_01", "plc_a01", "c_wolf"] {
        let data = rm.get_named(name, mg_core::ResType::MDL).unwrap();
        let model = Arc::new(GpuModel::new(&gpu, Arc::new(Model::read(&data).unwrap())));
        let camera = framing(&model);
        let mut scene = base.clone();
        // The point light by the model, at its size.
        let c = camera.target;
        let r = (camera.eye - c).length() * 0.3;
        scene.lights = vec![mg_render::PointLight::new(
            c + Vec3::new(-r, -r, r),
            Vec3::new(1.0, 0.75, 0.5),
            r * 2.0,
            false,
            2,
        )];
        scene.fog = Some(mg_render::Fog {
            start: (camera.eye - c).length() * 0.8,
            end: (camera.eye - c).length() * 2.5,
            ..fog
        });
        scene.instances = vec![Instance::new(model, Mat4::IDENTITY)];
        scenes.push((name, scene, camera));
    }

    // The tile's texture is turned: its picture is not the one it has
    // unturned (a mutation check of the scene that follows).
    {
        let (_, scene, camera) = scenes.iter().find(|(n, ..)| *n == "tile_turned").unwrap();
        let mut unturned = scene.clone();
        unturned.instances[0].tile = false;
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        r.game_shaders = true;
        let a = r.render_image(&gpu, &assets, scene, camera, 320, 320);
        let b = r.render_image(&gpu, &assets, &unturned, camera, 320, 320);
        let differ = a.data.chunks(4).zip(b.data.chunks(4)).filter(|(p, q)| p != q).count();
        assert!(differ > 5000, "the turned texture: {differ} pixels differ");
    }
    let mut failures = Vec::new();
    for (name, scene, camera) in &scenes {
        let mut theirs = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        theirs.game_shaders = true;
        let game = theirs.render_image(&gpu, &assets, scene, camera, 320, 320);
        let drawn = theirs.drawn;
        assert_eq!(theirs.game_shader_failures(), &[], "{name}");
        let mut ours = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        let wgsl = ours.render_image(&gpu, &assets, scene, camera, 320, 320);
        let bg = [255, 0, 255, 255];
        let (d, picture) = difference(&game, &wgsl, bg);
        save(&game, &dir.join(format!("game_{name}_stock.png")));
        save(&wgsl, &dir.join(format!("game_{name}_wgsl.png")));
        save(&picture, &dir.join(format!("game_{name}_diff.png")));
        eprintln!(
            "{name}: {} of {} meshes by the game's shaders; {} pixels compared ({} saturated left out): \
             mean {:.2}, 99% within {}, at most {} (of 255)",
            drawn.game, drawn.meshes, d.pixels, d.saturated, d.mean, d.p99, d.max
        );
        if drawn.game == 0 || d.pixels < 1000 {
            failures.push(format!("{name}: nothing compared ({drawn:?})"));
        }
        // The tolerance: the same equations, in another language; 1 in
        // 255 for rounding. The environment map is sampled as the game's
        // `texture2D(texUnitEnv, uv, bias)` samples it, the roughness a
        // bias over the level the derivatives give (M4: before, at the
        // roughness's level outright, `_sm` differed by 0.73 / 99% 15 and
        // `_nm` by 1.11 / 11 of 255; after, 0.00 / 0 and 0.08 / 2). `_nm`
        // takes those derivatives from the normal map's normal before the
        // parallax and the bend of a normal facing away, which the game
        // takes after: 2 in 255 at the 99th percentile.
        let (mean, p99) = if name.contains("normal") { (0.5, 3) } else { (0.5, 1) };
        if d.mean > mean || d.p99 > p99 {
            failures.push(format!("{name}: mean {:.2}, 99% {}, max {}", d.mean, d.p99, d.max));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// A material's custom pair is lit, by Moonglow's own shader, as its
/// fragment program lights it: the game's `fslit_nm` under another name
/// (as Heed's `fhss_skintile` is) without a render hint, as the normal-
/// mapped programs light (no light softened past the terminator), and the
/// game's `fslit` so named as the stock one does (softened); the game's
/// programs draw the two apart. Before, Moonglow's own shader lit both as
/// `fslit`.
#[test]
fn a_custom_pair_is_lit_as_its_fragment_program_lights() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(gpu) = gpu() else { return };
    let rm = mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let dir = mg_testkit::output_dir("render");
    let source = |name: &str| mg_render::Assets::shader(&rm, name).unwrap();
    let mut own = TestAssets::default();
    own.solid("mgplain", [180, 160, 140, 255]);
    // (The pairs under names of their own: custom, not the stock choice.)
    own.shaders.insert("vsmgnm".into(), source("vslit_nm"));
    own.shaders.insert("fsmgnm".into(), source("fslit_nm").replace("KEYHOLING 1", "KEYHOLING 0"));
    own.shaders.insert("vsmglit".into(), source("vslit"));
    own.shaders.insert("fsmglit".into(), source("fslit"));
    let mtr = |text: &str| mg_image::mtr::Mtr::parse(text.as_bytes());
    own.materials.insert("mgnm".into(), mtr("customshadervs vsmgnm\ncustomshaderfs fsmgnm\n"));
    own.materials.insert("mglit".into(), mtr("customshadervs vsmglit\ncustomshaderfs fsmglit\n"));
    let assets = WithGameShaders { own, game: &rm };
    let ball = |material: &str| {
        let mut m = sphere("mgplain", 32, 64);
        if let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind {
            mesh.material = Some(material.into());
        }
        Instance::new(
            Arc::new(GpuModel::new(&gpu, Arc::new(m))),
            Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        )
    };
    // Lit from the side, by the area and a point light, the terminator in
    // sight; a dark ambient, as Heed's.
    let scene = |material: &str| Scene {
        instances: vec![ball(material)],
        area: AreaLight {
            ambient: Vec3::splat(0.04),
            diffuse: Vec3::splat(0.4),
            direction: Vec3::new(1.0, 0.2, 0.3).normalize(),
        },
        lights: vec![mg_render::PointLight::new(
            Vec3::new(-2.5, -0.5, 1.5),
            Vec3::new(1.0, 0.8, 0.6),
            4.0,
            false,
            2,
        )],
        background: [1.0, 0.0, 1.0],
        ..Default::default()
    };
    let camera =
        Camera::orbit(Vec3::new(0.0, 0.0, 1.0), 3.5, -90f32.to_radians(), 5f32.to_radians());
    let bg = [255, 0, 255, 255];
    let mut pictures = Vec::new();
    for material in ["mgnm", "mglit"] {
        let scene = scene(material);
        let mut theirs = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        theirs.game_shaders = true;
        let game = theirs.render_image(&gpu, &assets, &scene, &camera, 320, 320);
        assert_eq!(theirs.game_shader_failures(), &[], "{material}");
        assert_eq!(theirs.drawn.game, 1, "{material}: drawn by its pair");
        let mut ours = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        let wgsl = ours.render_image(&gpu, &assets, &scene, &camera, 320, 320);
        let (d, picture) = difference(&game, &wgsl, bg);
        save(&game, &dir.join(format!("custom_{material}_game.png")));
        save(&wgsl, &dir.join(format!("custom_{material}_wgsl.png")));
        save(&picture, &dir.join(format!("custom_{material}_diff.png")));
        eprintln!(
            "{material}: {} pixels: mean {:.2}, 99% within {}, at most {}",
            d.pixels, d.mean, d.p99, d.max
        );
        assert!(d.pixels > 10_000, "{material}: the sphere is seen");
        assert!(d.mean <= 0.5 && d.p99 <= 2, "{material}: mean {:.2}, 99% {}", d.mean, d.p99);
        pictures.push(game);
    }
    // (The two programs light the sphere apart: what is compared shows.)
    let (d, _) = difference(&pictures[0], &pictures[1], bg);
    eprintln!("fslit_nm and fslit apart: mean {:.2}, at most {}", d.mean, d.max);
    assert!(d.max >= 8, "the two light alike: {}", d.max);
}

// Materials' own programs (`docs/shaders-proposal.md`, M2): authored
// stand-ins for a custom shader pair (the game's and The Domains of Dread's
// are read only from an install and its haks, in `mg-corpus-tests`).

/// A vertex program that sways a mesh along x by the clock and an MTR
/// parameter, and a fragment program that draws the texture, or an MTR's
/// color where an int parameter says so, with the texture's alpha.
fn authored_custom_pair(assets: &mut TestAssets) {
    let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mv;
uniform mat4 m_proj;
uniform mat4 m_m;
uniform int worldtimerTimeOfDay;
uniform float fSwayStrength;
attribute vec4 vPos;
attribute vec2 vTcIn;
varying vec2 vUv;
void main ()
{
    vUv = vTcIn;
    vec4 p = vPos;
    p.x += sin(float(worldtimerTimeOfDay) * 0.001) * fSwayStrength;
    gl_Position = m_proj * (m_mv * p) + vec4(0.0 * (m_m * vPos).y);
}
"#;
    let fs = r#"
#define SHADER_TYPE 2
uniform sampler2D texUnit0;
uniform vec4 vOverrideColor;
uniform int iForceOverrideColor;
varying vec2 vUv;
void main ()
{
    vec4 t = texture2D(texUnit0, vUv);
    vec3 c = iForceOverrideColor == 1 ? vOverrideColor.rgb : t.rgb;
    gl_FragColor = vec4(c, t.a);
}
"#;
    assets.shaders.insert("vs_sway".into(), vs.into());
    assets.shaders.insert("fs_tint".into(), fs.into());
}

/// A material that names a program pair is drawn with it, its parameters
/// filled by name (a float, a vector, an int), and moves with the clock:
/// two times, two pictures. With the game's shaders off, Moonglow's own
/// draws it, and it stands still.
#[test]
fn a_material_s_own_programs_draw_it_and_move_with_the_clock() {
    let Some(gpu) = gpu() else { return };
    let mut assets = clip_space_assets();
    authored_custom_pair(&mut assets);
    assets.opaque("swaying", [255, 0, 0]);
    let mtr = "customshadervs vs_sway\ncustomshaderfs fs_tint\n\
        parameter float fSwayStrength 0.5\nparameter float vOverrideColor 0.0 1.0 0.0 1.0\n\
        parameter int iForceOverrideColor 1\n";
    assets.materials.insert("swaying".into(), mg_image::mtr::Mtr::parse(mtr.as_bytes()));
    let camera = Camera {
        eye: Vec3::new(0.0, -10.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let shot = |r: &mut Renderer, time: f32| {
        let scene = Scene {
            instances: vec![standing_quad(&gpu, "swaying", Vec3::new(0.0, 0.0, 1.0), 1.0)],
            background: [0.0, 0.0, 0.0],
            time,
            ..Default::default()
        };
        r.render_image(&gpu, &assets, &scene, &camera, 64, 64)
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    let still = shot(&mut r, 0.0);
    assert_eq!(r.game_shader_failures(), &[]);
    assert_eq!((r.drawn.game, r.drawn.timed), (1, 1), "{:?}", r.drawn);
    // The MTR's color, by its int parameter.
    assert_eq!(still.pixel(32, 32), [0, 255, 0, 255]);
    // At 1.5 s it has swayed by half its width (sin 1.5 · 0.5 of its 2 m).
    let swayed = shot(&mut r, 1.5);
    save(&still, &mg_testkit::output_dir("render").join("game_custom_still.png"));
    save(&swayed, &mg_testkit::output_dir("render").join("game_custom_swayed.png"));
    let green = |img: &mg_image::Rgba| {
        (0..64)
            .flat_map(|y| (0..64).map(move |x| (x, y)))
            .filter(|&(x, y)| img.pixel(x, y)[1] > 128)
            .map(|(x, _)| x as f32)
            .sum::<f32>()
    };
    assert_ne!(still.data, swayed.data);
    assert!(green(&swayed) > green(&still), "it moved towards +x");

    // Off: Moonglow's own shader, the texture's red, still.
    let mut own = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    let (a, b) = (shot(&mut own, 0.0), shot(&mut own, 1.5));
    assert_eq!((own.drawn.game, own.drawn.timed), (0, 0));
    let p = a.pixel(32, 32);
    assert!(p[0] > 60 && p[1] < 40, "{p:?}");
    assert_eq!(a.data, b.data);
}

/// Made while the frames go on (`game_background`, the area view's): the
/// first frame draws the mesh with Moonglow's own shader and says a
/// pipeline is being made; the window is woken when it is; then the
/// material's programs draw it. A cache folder keeps the programs: a
/// renderer of its own reads them back instead of compiling them.
#[test]
fn the_game_s_programs_are_made_while_frames_go_on() {
    let Some(gpu) = gpu() else { return };
    let mut assets = clip_space_assets();
    authored_custom_pair(&mut assets);
    assets.opaque("swaying", [255, 0, 0]);
    let mtr = "customshadervs vs_sway\ncustomshaderfs fs_tint\n\
        parameter float vOverrideColor 0.0 1.0 0.0 1.0\nparameter int iForceOverrideColor 1\n";
    assets.materials.insert("swaying".into(), mg_image::mtr::Mtr::parse(mtr.as_bytes()));
    let camera = Camera {
        eye: Vec3::new(0.0, -10.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let scene = Scene {
        instances: vec![standing_quad(&gpu, "swaying", Vec3::new(0.0, 0.0, 1.0), 1.0)],
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    let cache = mg_testkit::scratch_dir("render-shader-cache");
    let woken = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    r.game_background = true;
    r.set_shader_cache(Some(cache.clone()));
    let w = woken.clone();
    r.set_game_waker(move || {
        w.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    let first = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    // (Moonglow's own: the texture's red.)
    assert_eq!((r.drawn.game, r.drawn.game_pending), (0, 1), "{:?}", r.drawn);
    let p = first.pixel(32, 32);
    assert!(p[0] > 60 && p[1] < 40, "{p:?}");
    let start = std::time::Instant::now();
    while woken.load(std::sync::atomic::Ordering::SeqCst) == 0 {
        assert!(start.elapsed().as_secs() < 60, "never made");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let then = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!((r.drawn.game, r.drawn.game_pending), (1, 0), "{:?}", r.drawn);
    assert_eq!(then.pixel(32, 32), [0, 255, 0, 255]);
    assert_eq!(r.game_shader_failures(), &[]);
    let stats = r.game_shader_stats();
    assert_eq!((stats.compiled, stats.cached, stats.pipelines), (2, 0, 1), "{stats:?}");
    // Another renderer, the same cache: read, not compiled; drawn alike.
    let mut again = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    again.game_shaders = true;
    again.set_shader_cache(Some(cache));
    let img = again.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    let stats = again.game_shader_stats();
    assert_eq!((stats.compiled, stats.cached), (0, 2), "{stats:?}");
    assert_eq!(img.data, then.data);
}

/// A material that names one program alone is drawn with the stock pair
/// (the engine takes a pair or neither); one whose program is of the other
/// stage (`customshaderVS fslit`) falls back to Moonglow's own shader and is
/// reported once, never drawn magenta.
#[test]
fn a_material_s_pair_needs_both_programs_of_their_stages() {
    let Some(gpu) = gpu() else { return };
    let mut assets = clip_space_assets();
    authored_custom_pair(&mut assets);
    let (scene, camera) = clip_space_scene(&gpu);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    // One alone: the stock pair (authored `vslit`/`fslit`), all three drawn.
    let alone = "customshadervs vs_sway\n";
    assets.materials.insert("red".into(), mg_image::mtr::Mtr::parse(alone.as_bytes()));
    let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!(r.game_shader_failures(), &[]);
    assert_eq!((r.drawn.game, r.drawn.timed), (3, 0), "{:?}", r.drawn);
    assert_eq!(img.pixel(32, 12), [255, 0, 0, 255]);

    // Of the wrong stage: the red quad is Moonglow's, reported once.
    let wrong = "customshadervs fslit\ncustomshaderfs fs_tint\n";
    assets.materials.insert("red".into(), mg_image::mtr::Mtr::parse(wrong.as_bytes()));
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!(r.drawn.game, 2, "{:?}", r.drawn);
    let failures = r.game_shader_failures();
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].1.contains("fragment program"), "{failures:?}");
    let p = img.pixel(32, 12);
    assert!(p[0] > 60 && p[1] < 40, "{p:?}");
}

/// A known custom shader (`vertexalpha`) is drawn by its hand port without
/// its source, unreported, and by its own programs where its source is
/// there.
#[test]
fn a_known_custom_shader_is_its_programs_where_they_are_there() {
    let Some(gpu) = gpu() else { return };
    let mut assets = clip_space_assets();
    let named = "customshadervs vertexalpha_vs\ncustomshaderfs vertexalpha_fs\n";
    assets.materials.insert("red".into(), mg_image::mtr::Mtr::parse(named.as_bytes()));
    let (scene, camera) = clip_space_scene(&gpu);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!(r.game_shader_failures(), &[]);
    assert_eq!(r.drawn.game, 2, "the hand port draws the red quad: {:?}", r.drawn);

    authored_custom_pair(&mut assets);
    let vs = assets.shaders["vs_sway"].clone();
    let fs = assets.shaders["fs_tint"].clone();
    assets.shaders.insert("vertexalpha_vs".into(), vs);
    assets.shaders.insert("vertexalpha_fs".into(), fs);
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!(r.game_shader_failures(), &[]);
    assert_eq!((r.drawn.game, r.drawn.timed), (3, 1), "{:?}", r.drawn);
}

/// A see-through mesh drawn by the game's programs is drawn in Moonglow's
/// two parts, among the other see-through meshes: its soft edge doesn't
/// hide the ground behind it (as `a_see_through_edge_does_not_hide_the_
/// ground_behind_it` holds Moonglow's own shader to), and what is solid of
/// it hides what is behind.
#[test]
fn a_see_through_mesh_is_drawn_in_two_parts_by_its_programs() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    authored_custom_pair(&mut assets);
    authored_lit_pair(&mut assets);
    let solid = |assets: &mut TestAssets, name: &str, rgba: [u8; 4], alpha: bool| {
        let t = mg_image::Rgba { width: 4, height: 4, data: rgba.repeat(16) }.into_texture(alpha);
        assets.textures.insert(name.into(), t);
    };
    solid(&mut assets, "water", [0, 0, 255, 255], false);
    solid(&mut assets, "lava", [255, 255, 0, 255], false);
    // (An alpha channel with a texel below opaque in it: see-through, as
    // the game takes it; all opaque, it would be solid.)
    solid(&mut assets, "ground", [200, 0, 0, 250], true);
    // Its edge half see-through, its middle solid.
    let mut plant = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            let edge = x == 0 || x == 3 || y == 0 || y == 3;
            plant.extend_from_slice(&[0, 200, 0, if edge { 128 } else { 255 }]);
        }
    }
    let t = mg_image::Rgba { width: 4, height: 4, data: plant }.into_texture(true);
    assets.textures.insert("plant".into(), t);
    let pair = "customshadervs vs_sway\ncustomshaderfs fs_tint\n";
    assets.materials.insert("plant".into(), mg_image::mtr::Mtr::parse(pair.as_bytes()));
    let model = |bitmap: &str| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        mesh.uvs[0] = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        mesh.source_uv = vec![0, 1, 2, 3];
        mesh.textures[0] = Some(bitmap.into());
        Arc::new(GpuModel::new(&gpu, Arc::new(m)))
    };
    let at = |p: Vec3, scale: f32| Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(scale));
    let plant_at = Vec3::new(0.0, 0.0, 0.6);
    let camera = Camera {
        eye: Vec3::new(0.0, -8.0, 3.0),
        target: plant_at,
        fov_y: 0.6,
        near: 0.1,
        far: 100.0,
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    let mut shot = |under: &str, plant: bool| {
        let mut instances = vec![
            Instance::new(model(under), at(Vec3::ZERO, 20.0)),
            Instance::new(model("ground"), at(Vec3::new(0.0, -5.5, 0.5), 6.0)),
        ];
        if plant {
            instances.push(Instance::new(model("plant"), at(plant_at, 0.5)));
        }
        let scene = Scene {
            instances,
            area: AreaLight { ambient: Vec3::splat(1.0), ..Default::default() },
            ..Default::default()
        };
        let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
        (img, r.drawn)
    };
    let (over_water, drawn) = shot("water", true);
    let (over_lava, _) = shot("lava", true);
    let (bare, _) = shot("water", false);
    let (bare_lava, _) = shot("lava", false);
    save(&over_water, &mg_testkit::output_dir("render").join("game_see_through.png"));
    assert_eq!(r.game_shader_failures(), &[]);
    // Water, ground (see-through: two parts each), plant (two parts).
    assert_eq!(drawn.game, 2 + 2 + 1, "{drawn:?}");
    let plant_px: Vec<(u32, u32)> = (0..64)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .filter(|&(x, y)| over_water.pixel(x, y) != bare.pixel(x, y))
        .collect();
    assert!(plant_px.len() > 20, "the plant is drawn: {}", plant_px.len());
    // What is under the ground makes no difference through the plant,
    // where the ground is behind it.
    for &(x, y) in plant_px.iter().filter(|&&(x, y)| bare.pixel(x, y) == bare_lava.pixel(x, y)) {
        // (The solid part, from 0.95 opaque, blends with what was drawn
        // before it: at most a twentieth of the water or the lava.)
        let (a, b) = (over_water.pixel(x, y), over_lava.pixel(x, y));
        let most = (0..3).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0);
        assert!(most <= 13, "through the plant at {x},{y}: {a:?} {b:?}");
    }
    // Its solid middle is its own green; its edge the ground's red and its
    // green blended.
    let middle = over_water.pixel(32, 32);
    assert!(middle[1] >= 190 && middle[0] < 15 && middle[2] < 15, "{middle:?}");
    assert!(
        plant_px.iter().any(|&(x, y)| {
            let p = over_water.pixel(x, y);
            p[0] > 60 && p[1] > 60
        }),
        "a blended edge"
    );
}

/// A custom pair that draws its texture by the texture coordinate set an
/// MTR int names (`nUvmap`: `vTcIn`, `vTcIn1`, `vTcIn2`, `vTcIn3`, the
/// mesh's `tverts`…`tverts3`), or (`nShow` 1) the mesh's tangent.
fn authored_uv_set_pair(assets: &mut TestAssets) {
    let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mv;
uniform mat4 m_proj;
uniform int nUvmap;
attribute vec4 vPos;
attribute vec2 vTcIn;
attribute vec2 vTcIn1;
attribute vec2 vTcIn2;
attribute vec2 vTcIn3;
attribute vec3 vTangent;
varying vec2 vUv;
varying vec3 vT;
void main ()
{
    vUv = nUvmap == 1 ? vTcIn1 : nUvmap == 2 ? vTcIn2 : nUvmap == 3 ? vTcIn3 : vTcIn;
    vT = vTangent;
    gl_Position = m_proj * (m_mv * vPos);
}
"#;
    let fs = r#"
#define SHADER_TYPE 2
uniform sampler2D texUnit0;
uniform int nShow;
varying vec2 vUv;
varying vec3 vT;
void main ()
{
    gl_FragColor = nShow == 1 ? vec4(abs(vT), 1.0) : vec4(texture2D(texUnit0, vUv).rgb, 1.0);
}
"#;
    assets.shaders.insert("vs_uvset".into(), vs.into());
    assets.shaders.insert("fs_uvset".into(), fs.into());
}

/// A custom shader reads every texture coordinate set of the mesh by the
/// set an MTR int picks (a builder's walls of `tverts2` drew one flat
/// color: the third and fourth sets were zero for every mesh); a set the
/// mesh doesn't have is zeros, as the client feeds it. And a mesh with no
/// tangents of its own has them only with a render hint, as in the client
/// (`custom_shader_uvs_look` in `mg-corpus-tests`' `client_render.rs`).
#[test]
fn a_custom_shader_reads_the_texture_coordinate_set_its_mtr_picks() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    authored_uv_set_pair(&mut assets);
    // Four columns: red, green, blue, white.
    let colors = [[255u8, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];
    let mut stripes = Vec::new();
    for _ in 0..4 {
        for c in colors {
            stripes.extend([c[0], c[1], c[2], 255]);
        }
    }
    let stripes = mg_image::Rgba { width: 4, height: 4, data: stripes }.into_texture(false);
    // Set k of the mesh: column k's middle at every corner.
    let middle = |k: usize| [k as f32 * 0.25 + 0.125, 0.5];
    let square = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let model = |bitmap: &str, sets: usize, uv0: Option<Vec<[f32; 2]>>| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        for (k, set) in mesh.uvs.iter_mut().enumerate().take(sets) {
            *set = vec![middle(k); 4];
        }
        if let Some(uv0) = uv0 {
            mesh.uvs[0] = uv0;
        }
        mesh.source_uv = vec![0, 1, 2, 3];
        mesh.textures[0] = Some(bitmap.into());
        Arc::new(GpuModel::new(&gpu, Arc::new(m)))
    };
    let camera = Camera {
        eye: Vec3::new(0.0, -10.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let standing = Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0))
        * Mat4::from_rotation_x(90f32.to_radians());
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    let mut draw = |name: &str, mtr: &str, model: Arc<GpuModel>| {
        assets.textures.insert(name.into(), stripes.clone());
        let pair = format!("customshadervs vs_uvset\ncustomshaderfs fs_uvset\n{mtr}");
        assets.materials.insert(name.into(), mg_image::mtr::Mtr::parse(pair.as_bytes()));
        let scene = Scene {
            instances: vec![Instance::new(model, standing)],
            background: [0.0, 0.0, 0.0],
            ..Default::default()
        };
        let img = r.render_image(&gpu, &assets, &scene, &camera, 32, 32);
        assert_eq!(r.game_shader_failures(), &[], "{name}");
        assert_eq!(r.drawn.game, 1, "{name}");
        img
    };
    for (k, color) in colors.iter().enumerate() {
        let name = format!("uvset{k}");
        let img = draw(&name, &format!("parameter int nUvmap {k}\n"), model(&name, 4, None));
        let p = img.pixel(16, 16);
        assert_eq!([p[0], p[1], p[2]], *color, "set {k}");
    }
    // A mesh of two sets, set 2 asked for: what (0, 0) samples.
    let missing = draw("uvmissing", "parameter int nUvmap 2\n", model("uvmissing", 2, None));
    let zero =
        draw("uvzero", "parameter int nUvmap 0\n", model("uvzero", 1, Some(vec![[0.0; 2]; 4])));
    assert_eq!(missing.pixel(16, 16), zero.pixel(16, 16));
    assert_ne!(missing.pixel(16, 16), [0, 0, 255, 255], "not set 2's blue");

    // Tangents: none of the mesh's own and no render hint, zeros; with the
    // MTR's render hint, made along +u (the quad's x).
    let mut tangent = |name: &str, hint: &str| {
        let img = draw(
            name,
            &format!("parameter int nShow 1\n{hint}"),
            model(name, 1, Some(square.clone())),
        );
        img.pixel(16, 16)
    };
    assert_eq!(tangent("untangent", ""), [0, 0, 0, 255]);
    let made = tangent("tangent", "renderhint NormalAndSpecMapped\n");
    assert!(made[0] > 250 && made[1] < 5 && made[2] < 5, "{made:?}");
}

// The special surfaces through the game's programs (`docs/shaders-proposal.md`,
// M5): authored stand-ins under the stock names the engine draws them with
// (read from the client: `surfaces_in_the_client` in `mg-corpus-tests`).

/// `vsvt`/`fst` standing in for the game's: the mesh in `globalColor`
/// (white, its alpha the mesh's), times magenta, so the program shows.
fn authored_unlit_pair(assets: &mut TestAssets) {
    let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mv;
uniform mat4 m_proj;
attribute vec4 vPos;
void main ()
{
    gl_Position = m_proj * (m_mv * vPos);
}
"#;
    let fs = r#"
#define SHADER_TYPE 2
uniform vec4 globalColor;
void main ()
{
    gl_FragColor = vec4(globalColor.rgb * vec3(1.0, 0.0, 1.0), 1.0);
}
"#;
    assets.shaders.insert("vsvt".into(), vs.into());
    assets.shaders.insert("fst".into(), fs.into());
}

/// The skybox is drawn with the game's `vsvt`/`fst`, at the far plane: a
/// sky five times as far as the far plane is not cut off by it, and what
/// is in front hides it.
#[test]
fn the_game_s_programs_draw_the_sky_at_the_far_plane() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    authored_unlit_pair(&mut assets);
    authored_lit_pair(&mut assets);
    let model = Arc::new(GpuModel::new(&gpu, Arc::new(quad())));
    let sky = Instance::new(
        model.clone(),
        Mat4::from_translation(Vec3::new(0.0, 0.0, -500.0)) * Mat4::from_scale(Vec3::splat(2000.0)),
    );
    let scene = Scene {
        instances: vec![Instance::new(model, Mat4::IDENTITY)],
        area: AreaLight { ambient: Vec3::splat(0.2), diffuse: Vec3::ZERO, direction: Vec3::Z },
        sky: Some(sky),
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
    r.game_shaders = true;
    r.game_programs = Some(Vec::new());
    let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!(r.game_shader_failures(), &[]);
    let programs = r.game_programs.take().unwrap();
    assert!(programs.iter().any(|p| p.vs == "vsvt" && p.fs == "fst" && p.no_discard));
    // Around the quad: the sky, by `fst` (magenta).
    assert_eq!(img.pixel(0, 0), [255, 0, 255, 255]);
    // The quad, in front of it, by the authored `fslit` (black: it has no
    // texture).
    assert_eq!(img.pixel(32, 32), [0, 0, 0, 255]);
}

/// An additive mesh (TXI `blending additive`) is drawn by the game's
/// programs, added to what is behind it, as Moonglow's own shader adds it.
#[test]
fn the_game_s_programs_draw_an_additive_mesh_added() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    authored_lit_pair(&mut assets);
    assets.opaque("red", [255, 0, 0]);
    assets.opaque("glow", [0, 128, 0]);
    assets.txis.insert("glow".into(), mg_image::txi::Txi::parse(b"blending additive\n"));
    let scene = Scene {
        instances: vec![
            standing_quad(&gpu, "red", Vec3::new(0.0, 0.0, 1.0), 3.0),
            standing_quad(&gpu, "glow", Vec3::new(0.0, -2.0, 1.0), 1.0),
        ],
        area: AreaLight { ambient: Vec3::ONE, diffuse: Vec3::ZERO, direction: Vec3::Z },
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    let camera = Camera {
        eye: Vec3::new(0.0, -10.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let mut pictures = Vec::new();
    for on in [false, true] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        r.game_shaders = on;
        r.game_programs = Some(Vec::new());
        pictures.push(r.render_image(&gpu, &assets, &scene, &camera, 64, 64));
        if on {
            assert_eq!(r.game_shader_failures(), &[]);
            assert_eq!(r.drawn.game, 2, "both by the game's programs: {:?}", r.drawn);
        }
    }
    let on = pictures[1].pixel(32, 32);
    // (The authored `fslit` draws the texture unlit: red and green, added.)
    assert_eq!(on, [255, 128, 0, 255], "added: {on:?}");
    // (Around it, the red quad alone; and with the switch off, the same.)
    assert!(pictures[1].data.chunks(4).any(|p| p == [255, 0, 0, 255]));
    // (Moonglow's own lights it, the authored `fslit` doesn't: added too.)
    let off = pictures[0].pixel(32, 32);
    assert!(off[0] == 255 && off[1] > 100, "Moonglow's own: {off:?}");
}

/// An additive mesh lying on another (a glow on a body) adds to all of it
/// when the body is drawn by the game's programs and the glow by Moonglow's
/// shader (it ripples: `proceduretype arturo`), seen from afar. Their
/// depths, reckoned apart, differed by the last bits and the glow showed
/// in patches that changed from frame to frame (the Domains of Dread's
/// `dd_ce3_zombieash`, two skins of the same vertices; the client draws
/// both with one program).
#[test]
fn a_glow_on_a_mesh_drawn_by_the_other_shader_covers_it() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    authored_lit_pair(&mut assets);
    assets.opaque("body", [96, 0, 0]);
    assets.opaque("glow", [0, 0, 96]);
    let txi = b"blending additive\nproceduretype arturo\ndistort 1\narturowidth 32\n\
                distortionamplitude 6\nspeed 20\n";
    assets.txis.insert("glow".into(), mg_image::txi::Txi::parse(txi));
    // A quad seen aslant from 60 m, as an area's view is.
    let at = Vec3::new(0.0, 0.0, 1.0);
    let quad_at = |texture: &str| {
        let mut q = standing_quad(&gpu, texture, at, 3.0);
        q.transform = Mat4::from_rotation_z(0.9) * q.transform;
        q
    };
    let camera = Camera {
        eye: at + Vec3::new(-20.0, -55.0, 15.0),
        target: at,
        fov_y: 0.06,
        near: 0.6,
        far: 640.0,
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    let mut shot = |glow: bool| {
        let mut instances = vec![quad_at("body")];
        if glow {
            instances.push(quad_at("glow"));
        }
        let scene = Scene {
            instances,
            area: AreaLight { ambient: Vec3::ONE, diffuse: Vec3::ZERO, direction: Vec3::Z },
            background: [0.0, 0.0, 0.0],
            ..Default::default()
        };
        (r.render_image(&gpu, &assets, &scene, &camera, 128, 128), r.drawn.game)
    };
    let (body, _) = shot(false);
    let (both, game) = shot(true);
    assert_eq!(r.game_shader_failures(), &[]);
    assert_eq!(game, 1, "the body by the game's programs, the glow by Moonglow's shader");
    let mut covered = 0;
    let mut missed = Vec::new();
    for (k, (a, b)) in body.data.chunks(4).zip(both.data.chunks(4)).enumerate() {
        if a[0] == 96 {
            covered += 1;
            if b[2] < 40 {
                missed.push((k % 128, k / 128));
            }
        }
    }
    assert!(covered > 2000, "the body is drawn: {covered} pixels");
    assert!(
        missed.is_empty(),
        "the glow misses {} of {covered}: {:?}…",
        missed.len(),
        &missed[..missed.len().min(8)]
    );
}

/// `vsgrass`/`fsgrass` standing in for the game's: cyan.
fn authored_grass_pair(assets: &mut TestAssets) {
    let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mv;
uniform mat4 m_proj;
attribute vec4 vPos;
void main ()
{
    gl_Position = m_proj * (m_mv * vPos);
}
"#;
    let fs = "#define SHADER_TYPE 2\nvoid main () { gl_FragColor = vec4(0.0, 1.0, 1.0, 1.0); }\n";
    assets.shaders.insert("vsgrass".into(), vs.into());
    assets.shaders.insert("fsgrass".into(), fs.into());
}

/// An area's grass (an instance marked so) is drawn with the game's
/// `vsgrass`/`fsgrass`; the same mesh not grass, with `vslit`/`fslit`.
#[test]
fn the_game_s_programs_draw_the_grass() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    authored_lit_pair(&mut assets);
    authored_grass_pair(&mut assets);
    assets.opaque("red", [255, 0, 0]);
    let grass =
        Instance { grass: true, ..standing_quad(&gpu, "red", Vec3::new(-1.0, 0.0, 1.0), 1.5) };
    let plain = standing_quad(&gpu, "red", Vec3::new(1.0, 0.0, 1.0), 1.5);
    let scene =
        Scene { instances: vec![grass, plain], background: [0.0, 0.0, 0.0], ..Default::default() };
    let camera = Camera {
        eye: Vec3::new(0.0, -8.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = true;
    r.game_programs = Some(Vec::new());
    let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
    assert_eq!(r.game_shader_failures(), &[]);
    let programs = r.game_programs.take().unwrap();
    assert_eq!(programs.iter().filter(|p| p.vs == "vsgrass" && p.fs == "fsgrass").count(), 1);
    assert_eq!(img.pixel(20, 32), [0, 255, 255, 255], "the grass");
    assert_eq!(img.pixel(44, 32), [255, 0, 0, 255], "the rest");
}

/// `vsparticle`/`fsparticle` standing in for the game's: the particle's
/// color (`vColor`) times its texture, its `blendMode` choosing the channel
/// it is drawn in (red normal, green additive, blue punch-through).
fn authored_particle_pair(assets: &mut TestAssets) {
    let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mv;
uniform mat4 m_proj;
attribute vec4 vPos;
attribute vec2 vTcIn;
attribute vec4 vColor;
varying vec2 vUv;
varying vec4 vTint;
void main ()
{
    vUv = vTcIn;
    vTint = vColor;
    gl_Position = m_proj * (m_mv * vPos);
}
"#;
    let fs = r#"
#define SHADER_TYPE 2
uniform sampler2D texUnit0;
uniform int blendMode;
varying vec2 vUv;
varying vec4 vTint;
void main ()
{
    vec4 c = vTint * texture2D(texUnit0, vUv);
    vec3 channel = blendMode == 1 ? vec3(0.0, 1.0, 0.0)
        : (blendMode == 2 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0));
    gl_FragColor = vec4(c.rgb * channel, c.a);
}
"#;
    assets.shaders.insert("vsparticle".into(), vs.into());
    assets.shaders.insert("fsparticle".into(), fs.into());
}

/// Particles are drawn by the game's `vsparticle`/`fsparticle`, each
/// emitter's in its blend (its `blendMode`); with the switch off, by
/// Moonglow's own (white).
#[test]
fn the_game_s_programs_draw_particles_in_their_blends() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    authored_particle_pair(&mut assets);
    assets.solid("white", [255, 255, 255, 255]);
    let emitter = |name: &str, x: f32, blend: &str| {
        format!(
            "node emitter {name}\n  parent fx\n  position {x} 0 1\n  update Fountain\n  \
             render Normal\n  blend {blend}\n  texture white\n  loop 1\n  xgrid 1\n  ygrid 1\n  \
             spawntype 0\n  birthrate 1\n  lifeExp 100\n  velocity 0\n  randvel 0\n  spread 0\n  \
             mass 0\n  particleRot 0\n  sizeStart 0.8\n  sizeEnd 0.8\n  xsize 0\n  ysize 0\n  \
             alphaStart 1\n  alphaEnd 1\n  colorStart 1 1 1\n  colorEnd 1 1 1\nendnode\n"
        )
    };
    let text = format!(
        "newmodel fx\nsetsupermodel fx NULL\nbeginmodelgeom fx\nnode dummy fx\n  parent NULL\n\
         endnode\n{}{}{}endmodelgeom fx\ndonemodel fx\n",
        emitter("normal", -1.2, "Normal"),
        emitter("lighten", 0.0, "Lighten"),
        emitter("punch", 1.2, "Punch-Through"),
    );
    let model = Arc::new(Model::read(text.as_bytes()).unwrap());
    let gm = Arc::new(GpuModel::new(&gpu, model.clone()));
    let camera = Camera {
        eye: Vec3::new(0.0, -4.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let mut p = mg_render::particles::Particles::new(&model);
    let mut particles = Vec::new();
    for _ in 0..200 {
        p.update(&model, None, 0.0, 0.02, &gm.rest, Mat4::IDENTITY);
        particles = p.batches(&model, None, 0.0, &gm.rest, Mat4::IDENTITY, camera.view());
        if particles.iter().filter(|b| !b.vertices.is_empty()).count() == 3 {
            break;
        }
    }
    let scene = Scene {
        instances: vec![Instance::new(gm, Mat4::IDENTITY)],
        background: [0.0, 0.0, 0.0],
        particles,
        ..Default::default()
    };
    let (w, h) = (192, 64);
    // (Where each emitter's particle is on the picture.)
    let at = |x: f32| {
        let c = camera.projection(w as f32 / h as f32)
            * camera.view()
            * Vec3::new(x, 0.0, 1.0).extend(1.0);
        (((c.x / c.w) * 0.5 + 0.5) * w as f32) as u32
    };
    for on in [false, true] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        r.game_shaders = on;
        r.game_programs = Some(Vec::new());
        let img = r.render_image(&gpu, &assets, &scene, &camera, w, h);
        let seen: Vec<[u8; 4]> = [-1.2, 0.0, 1.2].iter().map(|&x| img.pixel(at(x), 32)).collect();
        if !on {
            assert!(seen.iter().all(|p| p[..3] == [255, 255, 255]), "Moonglow's own: {seen:?}");
            continue;
        }
        assert_eq!(r.game_shader_failures(), &[]);
        let programs = r.game_programs.take().unwrap();
        let nd: Vec<(String, bool)> = programs
            .iter()
            .filter(|p| p.fs == "fsparticle")
            .map(|p| (p.node.clone(), p.no_discard))
            .collect();
        assert_eq!(nd.len(), 3, "{programs:?}");
        assert!(
            nd.contains(&("PunchThrough".into(), false)) && nd.contains(&("Normal".into(), true))
        );
        assert_eq!(seen[0][..3], [255, 0, 0], "normal: {seen:?}");
        assert_eq!(seen[1][..3], [0, 255, 0], "additive: {seen:?}");
        assert_eq!(seen[2][..3], [0, 0, 255], "punch-through: {seen:?}");
    }
}

/// See-through meshes the game's programs draw are instances of one draw
/// where they don't meet on the screen, as Moonglow's own are (M5: a field
/// of fires and grass was a draw for each part of each mesh), each with its
/// own values: three see-through quads in a row, red, green and blue by
/// their material colors, in two draws (each part one).
#[test]
fn see_through_meshes_by_the_game_s_programs_are_instances_of_one_draw() {
    let Some(gpu) = gpu() else { return };
    let mut assets = TestAssets::default();
    // (`fslit` standing in: the texture times the material's diffuse.)
    authored_lit_pair(&mut assets);
    let fs = r#"
#define SHADER_TYPE 2
uniform sampler2D texUnit0;
uniform vec4 materialFrontDiffuse;
varying vec2 vUv;
varying vec3 vBlack;
varying vec3 vWhite;
void main ()
{
    vec4 t = texture2D(texUnit0, vUv);
    gl_FragColor = vec4(t.rgb * materialFrontDiffuse.rgb * vWhite + vBlack, t.a);
}
"#;
    assets.shaders.insert("fslit".into(), fs.into());
    // (Opaque in the middle, half see-through around it.)
    let data: Vec<u8> = (0..16 * 16)
        .flat_map(|i| {
            let (x, y) = (i % 16, i / 16);
            let a = if (4..12).contains(&x) && (4..12).contains(&y) { 255 } else { 128 };
            [255, 255, 255, a]
        })
        .collect();
    let veil = mg_image::Rgba { width: 16, height: 16, data }.into_texture(true);
    assets.textures.insert("veil".into(), veil);
    let colored = |rgb: [f32; 3]| {
        let mut model = quad();
        if let NodeKind::Mesh(m) = &mut model.nodes[1].kind {
            m.textures[0] = Some("veil".into());
            m.diffuse = rgb;
        }
        Arc::new(GpuModel::new(&gpu, Arc::new(model)))
    };
    // (One model, three places; its color by the instance's model.)
    let red = colored([1.0, 0.0, 0.0]);
    let instances: Vec<Instance> = [-2.2, 0.0, 2.2]
        .into_iter()
        .map(|x| Instance::new(red.clone(), Mat4::from_translation(Vec3::new(x - 0.5, -0.5, 0.0))))
        .collect();
    let camera = Camera {
        eye: Vec3::new(0.0, -0.01, 8.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    let scene = Scene { instances, background: [0.0, 0.0, 0.0], ..Default::default() };
    let mut seen = Vec::new();
    for instancing in [true, false] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        r.game_shaders = true;
        r.instancing = instancing;
        let image = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
        assert_eq!(r.game_shader_failures(), &[]);
        assert_eq!(r.drawn.game, 6, "{:?}", r.drawn);
        seen.push((r.drawn.batches, [11, 32, 53].map(|x| image.pixel(x, 32))));
    }
    let ((batched, a), (each, b)) = (seen[0], seen[1]);
    assert_eq!((batched, each), (2, 6), "a draw for each part, or a draw for each mesh's");
    assert_eq!(a, b, "the same picture either way");
    assert!(a.iter().all(|p| p[0] > 100 && p[1] < 30), "each quad red where it is: {a:?}");
}

/// The framebuffer's copies as the game's programs read them
/// (`docs/framebuffer-proposal.md`): shiny water drawn by an authored
/// `vswater`/`fswater` that shows what lies behind it, read from
/// `texFBColor` at its own `gl_FragCoord` and, in a second program, at its
/// projected position, finds the picture drawn without the water at both:
/// `gl_FragCoord.y` and the copy's rows count from the bottom together,
/// as in OpenGL, whichever way a program finds its pixel. A third reads
/// `texFBDepth` as `fswater`'s `GetDepth` does: the distance behind the
/// water in metres. With 4x MSAA and without; without the targets'
/// copies (`Renderer::render`), the water is Moonglow's own.
#[test]
fn the_framebuffer_s_copies_are_read_as_opengl_reads_them() {
    let Some(gpu) = gpu() else { return };
    let mut assets = clip_space_assets();
    let txi = |text: &str| mg_image::txi::Txi::parse(text.as_bytes());
    assets.opaque("pool", [0, 0, 255]);
    assets.txis.insert("pool".into(), txi("bumpmaptexture shinywater\n"));
    let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mv;
uniform mat4 m_proj;
attribute vec4 vPos;
varying vec4 vView;
void main ()
{
    vView = m_mv * vPos;
    gl_Position = m_proj * vView;
}
"#;
    let fs = |read: &str| {
        format!(
            r#"
#define SHADER_TYPE 2
uniform mat4 m_proj;
uniform vec3 pixelParams;
uniform vec3 clipParams;
uniform sampler2D texFBColor;
uniform sampler2D texFBDepth;
varying vec4 vView;
void main ()
{{
    vec2 own = gl_FragCoord.xy * pixelParams.xy;
    vec4 clip = m_proj * vView;
    vec2 projected = 0.5 * clip.xy / clip.w + 0.5;
    {read}
}}
"#
        )
    };
    let camera = Camera {
        eye: Vec3::new(0.0, -10.0, 1.0),
        target: Vec3::new(0.0, 0.0, 1.0),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    // Behind, red above the middle and green below it, 10 m away; the
    // water in front, across the middle.
    let wall = [
        standing_quad(&gpu, "red", Vec3::new(0.0, 0.0, 3.0), 2.0),
        standing_quad(&gpu, "green", Vec3::new(0.0, 0.0, -1.0), 2.0),
    ];
    let water = standing_quad(&gpu, "pool", Vec3::new(0.0, -5.0, 1.0), 0.6);
    let scene = |with_water: bool| Scene {
        instances: wall.iter().cloned().chain(with_water.then(|| water.clone())).collect(),
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    let dir = mg_testkit::scratch_dir("render-framebuffer");
    for samples in [1, 4] {
        let mut plain = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, samples);
        plain.game_shaders = true;
        let behind = plain.render_image(&gpu, &assets, &scene(false), &camera, 64, 64);
        // (The water's pixels: where Moonglow's own draws it.)
        let mut own = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, samples);
        own.game_shaders = true;
        own.game_water = false;
        let owned = own.render_image(&gpu, &assets, &scene(true), &camera, 64, 64);
        save(&owned, &dir.join(format!("water_moonglow_{samples}x.png")));
        let pool: Vec<(u32, u32)> = (0..64)
            .flat_map(|y| (0..64).map(move |x| (x, y)))
            .filter(|&(x, y)| owned.pixel(x, y) != behind.pixel(x, y))
            .collect();
        assert!(pool.len() > 200, "{} pixels of water", pool.len());
        let top = pool.iter().filter(|&&(x, y)| behind.pixel(x, y)[0] > 200).count();
        assert!(top > 50 && pool.len() - top > 50, "the water crosses the middle: {top}");
        for (name, read) in [
            ("own", "gl_FragColor = vec4(texture2D(texFBColor, own).rgb, 1.0);"),
            ("projected", "gl_FragColor = vec4(texture2D(texFBColor, projected).rgb, 1.0);"),
            (
                "depth",
                "float d = clipParams.x / (1.0 - texture2D(texFBDepth, own).x * clipParams.z);\n    \
                 gl_FragColor = vec4(d / 20.0, 0.0, 0.0, 1.0);",
            ),
        ] {
            assets.shaders.insert("vswater".into(), vs.into());
            assets.shaders.insert("fswater".into(), fs(read));
            let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, samples);
            r.game_shaders = true;
            r.game_water = true;
            let img = r.render_image(&gpu, &assets, &scene(true), &camera, 64, 64);
            save(&img, &dir.join(format!("water_{name}_{samples}x.png")));
            assert_eq!(r.game_shader_failures(), &[], "{name}");
            assert_eq!(r.drawn.game, 3, "{name}: {:?}", r.drawn);
            let far = |&&(x, y): &&(u32, u32)| {
                let (a, b) = (img.pixel(x, y), behind.pixel(x, y));
                if name == "depth" {
                    // 10 m: 0.5 of 255.
                    a[0].abs_diff(128) > 3
                } else {
                    (0..3).any(|i| a[i].abs_diff(b[i]) > 8)
                }
            };
            let wrong = pool.iter().filter(far).count();
            assert!(wrong <= 4, "{name}, {samples}x: {wrong} of {} water pixels", pool.len());
        }
        // Without the targets' copies: Moonglow's own water.
        let targets =
            mg_render::Targets::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, samples, 64, 64);
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, samples);
        r.game_shaders = true;
        r.game_water = true;
        r.render(
            &gpu,
            &assets,
            &scene(true),
            &camera,
            targets.render_view(),
            targets.resolve_view(),
            &targets.depth,
            (64, 64),
        );
        assert_eq!(r.drawn.game, 2, "{:?}", r.drawn);
    }
}

/// Soft particles (`docs/framebuffer-proposal.md`, step 4): a particle
/// standing half under the floor, drawn by an authored `fsparticle` that
/// shows the game's fade (`sqr(clamp((sceneDepth − its depth) ·
/// particleSoftness, 0, 1))`, the scene's depth from `texFBDepth`), is dark
/// where it meets the floor and white higher up: the frame makes the
/// framebuffer's copy for the particle alone (no water), and gives it the
/// batch's softness. Off (`soft_particles`), or without the targets'
/// copies, it fades nowhere.
#[test]
fn particles_fade_where_they_meet_the_floor() {
    let Some(gpu) = gpu() else { return };
    let mut assets = clip_space_assets();
    let vs = r#"
#define SHADER_TYPE 1
uniform mat4 m_mv;
uniform mat4 m_proj;
attribute vec4 vPos;
attribute vec2 vTcIn;
attribute vec4 vColor;
varying vec4 vTint;
void main ()
{
    vTint = vColor + vec4(vTcIn, 0.0, 0.0) * 0.0;
    gl_Position = m_proj * (m_mv * vPos);
}
"#;
    let fs = r#"
#define SHADER_TYPE 2
uniform float particleSoftness;
uniform vec3 pixelParams;
uniform vec3 clipParams;
uniform sampler2D texFBDepth;
varying vec4 vTint;
void main ()
{
    float scene = clipParams.x / (1.0 - texture2D(texFBDepth, gl_FragCoord.xy * pixelParams.xy).x * clipParams.z);
    float own = clipParams.x / (1.0 - gl_FragCoord.z * clipParams.z);
    float fade = clamp((scene - own) * particleSoftness, 0.0, 1.0);
    fade *= fade;
    gl_FragColor = vec4(vec3(particleSoftness > 0.0 ? fade : 1.0) * vTint.rgb, 1.0);
}
"#;
    assets.shaders.insert("vsparticle".into(), vs.into());
    assets.shaders.insert("fsparticle".into(), fs.into());
    let camera = Camera {
        eye: Vec3::new(0.0, -6.0, 1.5),
        target: Vec3::new(0.0, 0.0, 0.3),
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    // The floor, 20 m square; the particle, 2 m square, standing in the
    // middle of it, half under it.
    let floor = {
        let mut model = quad();
        if let NodeKind::Mesh(m) = &mut model.nodes[1].kind {
            m.textures[0] = Some("blue".into());
        }
        let model = Arc::new(GpuModel::new(&gpu, Arc::new(model)));
        Instance::new(model, Mat4::from_scale(Vec3::new(10.0, 10.0, 1.0)))
    };
    let v = |x: f32, z: f32| mg_render::particles::ParticleVertex {
        pos: [x, 0.0, z],
        uv: [0.0, 0.0],
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let batch = mg_render::particles::ParticleBatch {
        texture: None,
        blend: mg_render::particles::ParticleBlend::Normal,
        render_order: 0,
        vertices: vec![
            v(-1.0, -1.0),
            v(1.0, -1.0),
            v(1.0, 1.0),
            v(-1.0, -1.0),
            v(1.0, 1.0),
            v(-1.0, 1.0),
        ],
        tint: None,
        softness: mg_render::particles::softness(2.0, 2.0, "Normal"),
    };
    let scene = Scene {
        instances: vec![floor],
        particles: vec![batch],
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    let dir = mg_testkit::scratch_dir("render-soft-particles");
    // The particle's middle column, from where it meets the (blue) floor
    // up: its grey pixels.
    let column = |img: &mg_image::Rgba| -> Vec<u8> {
        (0..64)
            .rev()
            .map(|y| img.pixel(32, y))
            .skip_while(|p| p[2] > p[0])
            .take_while(|p| p[0] == p[2])
            .map(|p| p[0])
            .collect()
    };
    for (soft, copies) in [(true, true), (false, true), (true, false)] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        r.game_shaders = true;
        r.soft_particles = soft;
        let img = if copies {
            r.render_image(&gpu, &assets, &scene, &camera, 64, 64)
        } else {
            let targets = mg_render::Targets::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1, 64, 64);
            r.render(
                &gpu,
                &assets,
                &scene,
                &camera,
                targets.render_view(),
                targets.resolve_view(),
                &targets.depth,
                (64, 64),
            );
            gpu.read_rgba(&targets.color)
        };
        assert_eq!(r.game_shader_failures(), &[]);
        save(&img, &dir.join(format!("soft_{soft}_copies_{copies}.png")));
        let c = column(&img);
        let foot = c.first().copied().unwrap_or(255);
        let top = c.iter().copied().max().unwrap_or(0);
        eprintln!("soft {soft}, copies {copies}: foot {foot}, column {c:?}");
        assert_eq!(top, 255, "white high up: {c:?}");
        if soft && copies {
            assert!(foot < 128, "dark where it meets the floor: {foot}, {c:?}");
        } else {
            // (Its foot white; above it, the background's black.)
            assert!(foot == 255 && c.iter().all(|&r| r == 255 || r == 0), "no fade: {c:?}");
        }
    }
}
