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

    // The quad with u along +X, lit from +X and above, seen from above.
    let model = |bitmap: &str, maps: [Option<&str>; 3], mtr: Option<&str>| {
        let mut m = quad();
        let NodeKind::Mesh(mesh) = &mut m.nodes[1].kind else { unreachable!() };
        mesh.uvs[0] = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        mesh.source_uv = vec![0, 1, 2, 3];
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
