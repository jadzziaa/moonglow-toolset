//! A builder's custom shaders drawn by Moonglow (`docs/shaders-proposal.md`,
//! M2): The Domains of Dread's, from its haks where they are on this machine
//! (`MOONGLOW_HAKS`, a folder with `tdod_p_tap.hak`; skipped without). Its
//! wind-swayed grass moves with the clock with the Shaders switch on and
//! stands still with it off; its fountain's water slides along its flow
//! map; its roof shader shows a mesh in an interior area only to a camera
//! below the material's height. Pictures in
//! `target/test-output/game_shaders/` to look at.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_core::ResType;
use mg_image::mtr::Mtr;
use mg_mdl::Model;
use mg_render::{Assets, Camera, Environment, Gpu, GpuModel, Instance, Renderer, Scene};
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResMan, priority};
use mg_testkit::{builder_hak_dirs, corpus};

/// The folder of The Domains of Dread's haks, if `MOONGLOW_HAKS` names it.
fn tdod_haks() -> Option<PathBuf> {
    let dir = builder_hak_dirs().into_iter().find(|d| d.join("tdod_p_tap.hak").is_file());
    if dir.is_none() {
        eprintln!("skipped: no tdod_p_tap.hak in MOONGLOW_HAKS");
    }
    dir
}

/// The game with The Domains of Dread's shader and placeable haks over it.
fn tdod(root: &std::path::Path, dir: &std::path::Path) -> ResMan {
    let mut rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    for name in ["tdod_t_tap", "tdod_p_tap", "tdod_shaders"] {
        let path = dir.join(format!("{name}.hak"));
        if let Ok(c) = ErfContainer::open(&path) {
            rm.add(priority::HAK_USER, format!("hak:{name}"), LayerClass::Erf, c);
        }
    }
    rm
}

fn gpu() -> Option<Gpu> {
    mg_testkit::gpu::open(Gpu::headless)
}

fn model(gpu: &Gpu, rm: &ResMan, name: &str) -> Arc<GpuModel> {
    let data = rm.get_named(name, ResType::MDL).unwrap_or_else(|e| panic!("{name}: {e}"));
    let model = Model::read(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
    Arc::new(GpuModel::new(gpu, Arc::new(model)))
}

/// The box at rest of the model's meshes that `which` takes.
fn bounds(m: &GpuModel, which: impl Fn(&mg_render::model::GpuMesh) -> bool) -> (Vec3, Vec3) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for mesh in m.meshes.iter().filter(|m| which(m)) {
        let at = m.rest.get(mesh.node).copied().unwrap_or(Mat4::IDENTITY);
        for c in 0..8 {
            let pick = |i: usize, a: f32, b: f32| if c & (1 << i) == 0 { a } else { b };
            let p = Vec3::new(
                pick(0, mesh.min.x, mesh.max.x),
                pick(1, mesh.min.y, mesh.max.y),
                pick(2, mesh.min.z, mesh.max.z),
            );
            let p = at.transform_point3(p);
            (lo, hi) = (lo.min(p), hi.max(p));
        }
    }
    (lo, hi)
}

/// A camera on the model at `at`, from the south and above.
fn framing(m: &GpuModel, at: Vec3, height: f32) -> Camera {
    framing_of(m, at, height, |_| true)
}

/// A camera on the meshes `which` takes of the model at `at`.
fn framing_of(
    m: &GpuModel,
    at: Vec3,
    height: f32,
    which: impl Fn(&mg_render::model::GpuMesh) -> bool,
) -> Camera {
    let (lo, hi) = bounds(m, which);
    let middle = at + (lo + hi) * 0.5;
    let size = (hi - lo).length().max(0.5);
    let eye = middle + Vec3::new(0.0, -1.6 * size, 0.0) + Vec3::Z * height;
    Camera { eye, target: middle, fov_y: 0.8, near: 0.05, far: 200.0 }
}

fn save(img: &mg_image::Rgba, name: &str) {
    let path = mg_testkit::output_dir("game_shaders").join(name);
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    let mut e = png::Encoder::new(&mut out, img.width, img.height);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(&img.data).unwrap();
    eprintln!("wrote {}", path.display());
}

/// How many pixels differ between two pictures (by more than 2 of 255 in a
/// channel).
fn differing(a: &mg_image::Rgba, b: &mg_image::Rgba) -> usize {
    a.data
        .chunks(4)
        .zip(b.data.chunks(4))
        .filter(|(p, q)| (0..3).any(|i| p[i].abs_diff(q[i]) > 2))
        .count()
}

/// The pixels a picture's model covers (not the background).
fn covered(img: &mg_image::Rgba, background: [u8; 3]) -> usize {
    img.data.chunks(4).filter(|p| (0..3).any(|i| p[i].abs_diff(background[i]) > 2)).count()
}

const BACKGROUND: [f32; 3] = [1.0, 0.0, 1.0];
const SIZE: u32 = 320;

/// `scene` of one instance at `at` at two times, with the game's shaders on
/// and off: the pictures, and what the renderer drew with them on.
fn two_times(
    gpu: &Gpu,
    assets: &dyn Assets,
    scene: &Scene,
    camera: &Camera,
    times: [f32; 2],
    on: bool,
) -> ([mg_image::Rgba; 2], mg_render::Drawn, Vec<(String, String)>) {
    let mut r = Renderer::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    r.game_shaders = on;
    let mut shot = |time: f32| {
        let s = Scene { time, ..scene.clone() };
        r.render_image(gpu, assets, &s, camera, SIZE, SIZE)
    };
    let a = shot(times[0]);
    let b = shot(times[1]);
    (([a, b]), r.drawn, r.game_shader_failures().to_vec())
}

/// The grass (`dd_tap_valete577`, `dd_tap_wgrass_vs`/`_fs` by its material
/// `dd_tap_wg01_0469`) sways: two times, two pictures, with the game's
/// shaders; with Moonglow's own it stands still. (Placed away from the
/// world's x axis: the sway's phase is the world's y times the clock.)
#[test]
fn tdod_wind_grass_sways_with_the_shaders_on() {
    let root = corpus!();
    let Some(dir) = tdod_haks() else { return };
    let Some(gpu) = gpu() else { return };
    let rm = tdod(&root, &dir);
    let grass = model(&gpu, &rm, "dd_tap_valete577");
    let at = Vec3::new(5.0, 12.0, 0.0);
    let camera = framing(&grass, at, 0.3);
    let scene = Scene {
        instances: vec![Instance::new(grass.clone(), Mat4::from_translation(at))],
        background: BACKGROUND,
        environment: Environment::from_are(0, 1),
        ..Default::default()
    };
    let ([a, b], drawn, failures) = two_times(&gpu, &rm, &scene, &camera, [1.0, 1.7], true);
    save(&a, "wind_grass_on_1.0s.png");
    save(&b, "wind_grass_on_1.7s.png");
    assert_eq!(failures, Vec::<(String, String)>::new());
    assert!(drawn.game > 0 && drawn.timed > 0, "{drawn:?}");
    let moved = differing(&a, &b);
    let ([c, d], own, _) = two_times(&gpu, &rm, &scene, &camera, [1.0, 1.7], false);
    save(&c, "wind_grass_off_1.0s.png");
    let still = differing(&c, &d);
    let shown = covered(&a, [255, 0, 255]);
    eprintln!("grass: {shown} pixels shown, {moved} moved with the shaders, {still} without");
    assert_eq!((own.game, own.timed), (0, 0));
    assert!(shown > 500, "the grass is drawn: {shown}");
    assert!(moved > shown / 50, "the grass sways: {moved} of {shown}");
    assert_eq!(still, 0, "Moonglow's own shader moves nothing");
}

/// The fountain's water (`dd_tap_stron1030`, `dd_tap_flow_fs` by its
/// material `dd_tap_fm01_0529`, its flow map in `texture6`) slides: two
/// times, two pictures, with the game's shaders; with Moonglow's own it is
/// still. (The flow-mapped mesh alone: the fountain's other water ripples
/// in either.)
#[test]
fn tdod_flow_map_water_slides_with_the_shaders_on() {
    let root = corpus!();
    let Some(dir) = tdod_haks() else { return };
    let Some(gpu) = gpu() else { return };
    let rm = tdod(&root, &dir);
    let fountain = model(&gpu, &rm, "dd_tap_stron1030");
    let flowing =
        |m: &mg_render::model::GpuMesh| m.material.texture.as_deref() == Some("dd_tap_fm01_0529");
    let camera = framing_of(&fountain, Vec3::ZERO, 0.0, flowing);
    assert!(fountain.meshes.iter().any(flowing), "a mesh of the flow material");
    let mut state = mg_render::MeshState::new(&fountain);
    for (mesh, o) in fountain.meshes.iter().zip(&mut state.meshes) {
        if !flowing(mesh) {
            o.alpha = Some(0.0);
        }
    }
    let scene = |state: Option<mg_render::MeshState>| Scene {
        instances: vec![Instance {
            state: state.map(Arc::new),
            ..Instance::new(fountain.clone(), Mat4::IDENTITY)
        }],
        background: BACKGROUND,
        ..Default::default()
    };
    let whole =
        two_times(&gpu, &rm, &scene(None), &framing(&fountain, Vec3::ZERO, 2.5), [1.0, 2.0], true)
            .0;
    save(&whole[0], "flow_fountain_on.png");
    let flow = scene(Some(state));
    let ([a, b], drawn, failures) = two_times(&gpu, &rm, &flow, &camera, [1.0, 2.0], true);
    save(&a, "flow_on_1s.png");
    save(&b, "flow_on_2s.png");
    assert_eq!(failures, Vec::<(String, String)>::new());
    assert!(drawn.game >= 1 && drawn.timed == 1, "{drawn:?}");
    let moved = differing(&a, &b);
    let ([c, d], _, _) = two_times(&gpu, &rm, &flow, &camera, [1.0, 2.0], false);
    save(&c, "flow_off_1s.png");
    let still = differing(&c, &d);
    let shown = covered(&a, [255, 0, 255]);
    eprintln!("flow mesh: {shown} pixels shown, {moved} moved with the shaders, {still} without");
    assert!(shown > 300, "the flow mesh is drawn: {shown}");
    assert!(moved > shown / 10, "the water slides: {moved} of {shown}");
    assert_eq!(still, 0, "Moonglow's own shader slides nothing");
}

/// The game's assets with materials of the test's own over them.
struct WithMaterials<'a> {
    rm: &'a ResMan,
    materials: HashMap<String, Mtr>,
}

impl Assets for WithMaterials<'_> {
    fn texture(&self, name: &str) -> Option<mg_render::LoadedTexture> {
        Assets::texture(self.rm, name)
    }

    fn material(&self, name: &str) -> Option<Mtr> {
        self.materials.get(name).cloned().or_else(|| Assets::material(self.rm, name))
    }

    fn txi(&self, name: &str) -> Option<mg_image::txi::Txi> {
        Assets::txi(self.rm, name)
    }

    fn shader(&self, name: &str) -> Option<String> {
        Assets::shader(self.rm, name)
    }
}

/// The roof shader (`shd_roofky_vs`, no material of The Domains of Dread's
/// names it: here an authored one on a stock placeable) raises a mesh kept
/// under the ground (its fragment program discards what is above it) by
/// `fRoofOffsetZ` in an interior area for a camera below
/// `fRoofCameraClipZ`, and hides it (500 m down) from one above, and
/// outdoors.
#[test]
fn tdod_roof_shader_shows_a_roof_by_the_camera_s_height() {
    let root = corpus!();
    let Some(dir) = tdod_haks() else { return };
    let Some(gpu) = gpu() else { return };
    let rm = tdod(&root, &dir);
    let crate_model = model(&gpu, &rm, "plc_a01");
    let mtr = "renderhint NormalAndSpecMapped\ncustomshadervs shd_roofky_vs\n\
        customshaderfs shd_roofky_fs\nparameter float fRoofOffsetZ 3.0\n\
        parameter float fRoofCameraClipZ 6.0\nparameter int iRoofInterior 0\n";
    let mut materials = HashMap::new();
    for mesh in &crate_model.meshes {
        if let Some(t) = &mesh.material.texture {
            materials.insert(t.to_ascii_lowercase(), Mtr::parse(mtr.as_bytes()));
        }
    }
    assert!(!materials.is_empty(), "plc_a01 has textures");
    let assets = WithMaterials { rm: &rm, materials };
    let shot = |height: f32, flags: u32| {
        let mut camera = framing(&crate_model, Vec3::ZERO, 0.0);
        camera.eye.z = height;
        let under = Mat4::from_translation(Vec3::new(0.0, 0.0, -3.0));
        let scene = Scene {
            instances: vec![Instance::new(crate_model.clone(), under)],
            background: BACKGROUND,
            environment: Environment::from_are(flags, 0),
            ..Default::default()
        };
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        r.game_shaders = true;
        let img = r.render_image(&gpu, &assets, &scene, &camera, SIZE, SIZE);
        assert_eq!(r.game_shader_failures(), &[]);
        assert!(r.drawn.game > 0, "{:?}", r.drawn);
        img
    };
    let (low, high, outdoors) = (shot(3.0, 1), shot(9.0, 1), shot(3.0, 0));
    save(&low, "roof_interior_camera_3m.png");
    save(&high, "roof_interior_camera_9m.png");
    save(&outdoors, "roof_outdoors_camera_3m.png");
    let magenta = [255, 0, 255];
    let (l, h, o) = (covered(&low, magenta), covered(&high, magenta), covered(&outdoors, magenta));
    eprintln!("roof: {l} pixels with the camera at 3 m, {h} at 9 m, {o} outdoors");
    assert!(l > 1000, "shown below the clip height: {l}");
    assert_eq!((h, o), (0, 0), "hidden above it, and outdoors");
}

/// The game's assets (its programs) with cut-out textures of the test's
/// own: each `blending punchthrough`.
struct WithCutOuts<'a> {
    rm: &'a ResMan,
    textures: HashMap<String, mg_image::Texture>,
}

impl Assets for WithCutOuts<'_> {
    fn texture(&self, name: &str) -> Option<mg_render::LoadedTexture> {
        Some(mg_render::LoadedTexture {
            texture: self.textures.get(name)?.clone(),
            txi: mg_image::txi::Txi::parse(b"blending punchthrough\n"),
        })
    }

    fn txi(&self, name: &str) -> Option<mg_image::txi::Txi> {
        self.textures
            .contains_key(name)
            .then(|| mg_image::txi::Txi::parse(b"blending punchthrough\n"))
    }

    fn shader(&self, name: &str) -> Option<String> {
        Assets::shader(self.rm, name)
    }
}

/// What is kept of a cut-out below opaque (its edge: The Domains of
/// Dread's fur cards on `dd_n2e_wolfdire`) is lit through from behind by
/// the game's stock programs at High Quality, the quality they are made
/// for: `inc_lighting`'s translucence adds the light behind a surface by
/// one minus its alpha. The client draws it so: `creatures_look` in
/// `client_render.rs` with the wolf (`MG_APPEARANCES=31002,31002`) shows a
/// light line along every backlit cut edge, gone with the translucence
/// defined away in a copy of `fslit` in its override, and none at the
/// game's default, Performance, which has no translucence. Moonglow's own
/// shader draws a cut-out's kept texels solid before it lights them (no
/// line). A light line there, with the game's shaders on, is the game's.
#[test]
fn a_backlit_cut_out_s_edge_is_lit_through_by_the_game_s_programs() {
    let root = corpus!();
    let Some(gpu) = gpu() else { return };
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    // A gray card, opaque, and the same at half its alpha (kept: the game
    // cuts at 0.2).
    let mut textures = HashMap::new();
    for (name, alpha) in [("mg_card_solid", 255), ("mg_card_edge", 128)] {
        let data = [128, 128, 128, alpha].repeat(16);
        let t = mg_image::Rgba { width: 4, height: 4, data }.into_texture(true);
        textures.insert(name.to_owned(), t);
    }
    let assets = WithCutOuts { rm: &rm, textures };
    // A quad facing +Z, seen from +Z, lit from behind it.
    let mesh = mg_mdl::Mesh {
        diffuse: [1.0; 3],
        ambient: [1.0; 3],
        render: true,
        vertices: vec![[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 4],
        uvs: [vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], vec![], vec![], vec![]],
        faces: vec![
            mg_mdl::Face { vertices: [0, 1, 2], material: 0 },
            mg_mdl::Face { vertices: [0, 2, 3], material: 0 },
        ],
        source: vec![0, 1, 2, 3],
        ..Default::default()
    };
    let area = mg_render::AreaLight {
        ambient: Vec3::splat(0.05),
        diffuse: Vec3::splat(0.8),
        direction: Vec3::new(0.3, 0.2, -0.9).normalize(),
    };
    let camera = Camera {
        eye: Vec3::new(0.0, -0.5, 4.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    for game in [true, false] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
        r.game_shaders = game;
        let mut shot = |texture: &str| {
            let mut mesh = mesh.clone();
            mesh.textures[0] = Some(texture.to_owned());
            let mut root_node = mg_mdl::Node::new("card", mg_mdl::NodeKind::Dummy);
            root_node.children = vec![1];
            let mut node = mg_mdl::Node::new("plane", mg_mdl::NodeKind::Mesh(Box::new(mesh)));
            node.parent = Some(0);
            let model = Model {
                name: "card".into(),
                animation_scale: 1.0,
                nodes: vec![root_node, node],
                ..Default::default()
            };
            let model = Arc::new(GpuModel::new(&gpu, Arc::new(model)));
            let scene = Scene {
                instances: vec![Instance::new(model, Mat4::IDENTITY)],
                area,
                background: BACKGROUND,
                ..Default::default()
            };
            let img = r.render_image(&gpu, &assets, &scene, &camera, 64, 64);
            assert_eq!(r.game_shader_failures(), &[]);
            assert_eq!(r.drawn.game, usize::from(game), "drawn by the game's programs: {game}");
            let p = img.pixel(32, 32);
            assert_ne!(&p[..3], &[255, 0, 255], "{texture} drawn (game's programs: {game})");
            p
        };
        let (solid, edge) = (shot("mg_card_solid"), shot("mg_card_edge"));
        let lightness = |p: [u8; 4]| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]);
        eprintln!("game's programs {game}: solid {solid:?}, edge {edge:?}");
        if game {
            assert!(
                lightness(edge) > lightness(solid) + 60,
                "the edge lit through from behind: {edge:?} against {solid:?}"
            );
        } else {
            assert!(
                lightness(edge).abs_diff(lightness(solid)) <= 6,
                "Moonglow's own shader draws the edge as the rest: {edge:?} against {solid:?}"
            );
        }
    }
}
