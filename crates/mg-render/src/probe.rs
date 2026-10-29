//! The startup check: whether an adapter draws Moonglow's meshes at all.
//!
//! Two builders on AMD GPUs under Windows saw the area view draw nothing,
//! or lose its tiles, on one backend and draw right on the other (Heed's
//! Radeon RX 580 through Direct3D 12, El Grillo's Radeon PRO through
//! Vulkan). [`probe`] opens a device on the adapter about to be taken, as
//! the window will, draws a small fixed scene offscreen through the
//! renderer's own pipelines (Moonglow's WGSL: a solid mesh, two
//! see-through ones, one over it, and a skinned one posed away from where
//! it was bound; four samples a pixel, as the area view), reads it back
//! and compares it with what it must look like. The app takes the same
//! GPU's other backend where it fails ([`crate::backend::start`]).
//!
//! The game's own shaders can't be checked here (the game's data isn't
//! read yet), nor is a mesh lit or textured: the lit check ([`probe_lit`])
//! draws a lit, textured mesh (its texture BC1, as the game's are) by
//! Moonglow's own shader and by the game's `vslit`/`fslit` once the
//! game's data is read, apart from the window's thread (25 ms and 40 ms on
//! the development machine; the window runs it with no shader cache, which
//! the window's worker may be writing); and the area view's watch
//! ([`crate::watch`]) looks at what is
//! drawn as it is.

use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use glam::{Mat4, Vec3};
use mg_mdl::{Face, Mesh, MeshExtra, Model, Node, NodeKind, Skin};

use crate::{Assets, Camera, Gpu, GpuModel, Instance, NoAssets, Renderer, Scene, Targets};

/// The picture's side, in pixels.
const SIDE: u32 = 48;

/// What the check made of an adapter.
#[derive(Debug, Clone)]
pub struct Probe {
    /// The backend and the adapter's name.
    pub backend: wgpu::Backend,
    pub adapter: String,
    /// `Err`: what was drawn wrong (or what failed before it was drawn).
    pub outcome: Result<(), String>,
    /// Opening the device, making the renderer (its pipelines), and
    /// drawing the scene and reading it back.
    pub device: Duration,
    pub pipelines: Duration,
    pub draw: Duration,
}

impl Probe {
    pub fn passed(&self) -> bool {
        self.outcome.is_ok()
    }

    /// The whole check's time.
    pub fn time(&self) -> Duration {
        self.device + self.pipelines + self.draw
    }
}

impl fmt::Display for Probe {
    /// `Vulkan (AMD Radeon RX 580): drawn right in 41 ms (device 12 ms,
    /// pipelines 25 ms, draw 4 ms)`, or what it drew wrong.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        write!(f, "{} ({}): ", crate::backend::backend_name(self.backend), self.adapter)?;
        match &self.outcome {
            Ok(()) => write!(f, "drawn right")?,
            Err(why) => write!(f, "failed: {why}")?,
        }
        write!(
            f,
            " in {:.0} ms (device {:.0} ms, pipelines {:.0} ms, draw {:.0} ms)",
            ms(self.time()),
            ms(self.device),
            ms(self.pipelines),
            ms(self.draw)
        )
    }
}

/// Checks that `adapter` draws: a device opened on it as the window opens
/// one, the scene drawn and read back. Never panics: what goes wrong
/// (no device, a GPU error, a panic in wgpu, a picture that isn't the
/// scene's) is the outcome. The device is dropped after.
pub fn probe(adapter: &wgpu::Adapter) -> Probe {
    let info = adapter.get_info();
    let mut p = Probe {
        backend: info.backend,
        adapter: info.name.clone(),
        outcome: Ok(()),
        device: Duration::ZERO,
        pipelines: Duration::ZERO,
        draw: Duration::ZERO,
    };
    let start = Instant::now();
    let opened = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("moonglow startup check"),
            required_features: adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC,
            required_limits: adapter.limits(),
            ..Default::default()
        }))
    }));
    p.device = start.elapsed();
    let (device, queue) = match opened {
        Ok(Ok(dq)) => dq,
        Ok(Err(e)) => {
            p.outcome = Err(format!("no device: {e}"));
            return p;
        }
        Err(_) => {
            p.outcome = Err("opening the device panicked".into());
            return p;
        }
    };
    let errors: Arc<Mutex<Vec<String>>> = Arc::default();
    let noted = errors.clone();
    device.on_uncaptured_error(Arc::new(move |e: wgpu::Error| {
        if let Ok(mut errors) = noted.lock() {
            errors.push(e.to_string());
        }
    }));
    let gpu = Gpu::new(device, queue);
    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(&gpu, &mut p)));
    let outcome = match checked {
        Ok(outcome) => outcome,
        Err(payload) => Err(format!(
            "it panicked: {}",
            payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        )),
    };
    // (A GPU error is the first thing to say: what was drawn follows from
    // it.)
    let errors = errors.lock().map(|e| e.clone()).unwrap_or_default();
    p.outcome = match errors.first() {
        Some(e) => Err(format!("a GPU error: {}", first_line(e))),
        None => outcome,
    };
    p
}

fn first_line(text: &str) -> &str {
    text.lines().find(|l| !l.trim().is_empty()).unwrap_or(text).trim()
}

/// The scene drawn on `gpu` and compared; the times into `p`.
fn check(gpu: &Gpu, p: &mut Probe) -> Result<(), String> {
    let start = Instant::now();
    let mut renderer = Renderer::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let targets = Targets::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 4, SIDE, SIDE);
    p.pipelines = start.elapsed();
    let start = Instant::now();
    let (scene, camera) = scene(gpu);
    renderer.render_targets(gpu, &NoAssets, &scene, &camera, &targets);
    let image = gpu.try_read_rgba(&targets.color);
    p.draw = start.elapsed();
    compare(&image?, &camera)
}

/// A quad `side` metres across, flat at height `z`, in `color`
/// (unlit: drawn in it), skinned to bone 1 where `skinned`.
fn quad(name: &str, side: f32, z: f32, color: [f32; 3], skinned: bool) -> Model {
    let h = side / 2.0;
    let mesh = Mesh {
        diffuse: color,
        ambient: color,
        render: true,
        vertices: vec![[-h, -h, z], [h, -h, z], [h, h, z], [-h, h, z]],
        normals: vec![[0.0, 0.0, 1.0]; 4],
        uvs: [vec![[0.0, 0.0]; 4], vec![], vec![], vec![]],
        faces: vec![
            Face { vertices: [0, 1, 2], material: 0 },
            Face { vertices: [0, 2, 3], material: 0 },
        ],
        source: vec![0, 1, 2, 3],
        extra: if skinned {
            MeshExtra::Skin(Skin {
                bones: vec![1],
                weights: vec![[(0, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)]; 4],
                inverse_bind: Vec::new(),
            })
        } else {
            MeshExtra::None
        },
        ..Default::default()
    };
    let mut root = Node::new(name, NodeKind::Dummy);
    root.children = vec![1, 2];
    let mut bone = Node::new("bone", NodeKind::Dummy);
    bone.parent = Some(0);
    let mut m = Node::new("plane", NodeKind::Mesh(Box::new(mesh)));
    m.parent = Some(0);
    Model {
        name: name.into(),
        animation_scale: 1.0,
        nodes: vec![root, bone, m],
        ..Default::default()
    }
}

/// Where the scene's parts are (metres, on the ground; the camera looks
/// down on it from 12 m up, about 10 m across).
const SOLID: [f32; 2] = [-2.5, 2.0];
const SEE_THROUGH: [f32; 2] = [2.5, 2.0];
/// The skinned quad is bound here and posed 5 m east.
const BOUND: [f32; 2] = [-2.5, -2.5];
const POSED: [f32; 2] = [2.5, -2.5];

const RED: [f32; 3] = [1.0, 0.0, 0.0];
const GREEN: [f32; 3] = [0.0, 1.0, 0.0];
const BLUE: [f32; 3] = [0.0, 0.0, 1.0];
const YELLOW: [f32; 3] = [1.0, 1.0, 0.0];

fn scene(gpu: &Gpu) -> (Scene, Camera) {
    let at = |p: [f32; 2]| Mat4::from_translation(Vec3::new(p[0], p[1], 0.0));
    let model = |m: Model| Arc::new(GpuModel::new(gpu, Arc::new(m)));
    let unlit = |model, transform, opacity| Instance {
        unlit: true,
        opacity,
        casts_shadows: false,
        ..Instance::new(model, transform)
    };
    let pose = Arc::new(vec![
        Mat4::IDENTITY,
        Mat4::from_translation(Vec3::new(POSED[0] - BOUND[0], 0.0, 0.0)),
        Mat4::IDENTITY,
    ]);
    let skinned = Instance {
        pose: Some(pose),
        ..unlit(model(quad("skinned", 2.0, 0.0, YELLOW, true)), at(BOUND), 1.0)
    };
    let scene = Scene {
        instances: vec![
            unlit(model(quad("solid", 2.0, 0.0, RED, false)), at(SOLID), 1.0),
            // Over the solid one's middle, half see-through: blended with it.
            unlit(model(quad("over", 1.0, 0.5, GREEN, false)), at(SOLID), 0.5),
            unlit(model(quad("see-through", 2.0, 0.0, BLUE, false)), at(SEE_THROUGH), 0.5),
            skinned,
        ],
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    let camera = Camera {
        eye: Vec3::new(0.0, -0.01, 12.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    (scene, camera)
}

/// Where a point of the ground is in the picture, in pixels.
fn pixel(camera: &Camera, at: [f32; 2], z: f32) -> (u32, u32) {
    let clip = camera.projection(1.0) * camera.view();
    let ndc = clip.project_point3(Vec3::new(at[0], at[1], z));
    let x = ((ndc.x * 0.5 + 0.5) * SIDE as f32).clamp(0.0, SIDE as f32 - 1.0);
    let y = ((0.5 - ndc.y * 0.5) * SIDE as f32).clamp(0.0, SIDE as f32 - 1.0);
    (x as u32, y as u32)
}

/// The picture against what the scene must look like: each part where
/// it is, in its colour (a wide margin: backends round and blend apart),
/// the background elsewhere.
fn compare(image: &mg_image::Rgba, camera: &Camera) -> Result<(), String> {
    // (Channel by channel: 'h' high, 'm' half (a blend), 'l' low.)
    let is = |p: [u8; 4], want: [char; 3]| {
        p[..3].iter().zip(want).all(|(&c, w)| match w {
            'h' => c >= 160,
            'm' => (40..=225).contains(&c),
            _ => c <= 50,
        })
    };
    // The over quad covers the solid one's middle; the solid one is seen
    // around it (0.75 m from its middle).
    let solid_rim = [SOLID[0] - 0.75, SOLID[1] - 0.75];
    let corner = [SOLID[0] + 3.0, SOLID[1] - 5.5];
    let checks: [(&str, [f32; 2], f32, [char; 3]); 6] = [
        ("the solid mesh", solid_rim, 0.0, ['h', 'l', 'l']),
        ("the see-through mesh over the solid one", SOLID, 0.5, ['m', 'm', 'l']),
        ("the see-through mesh", SEE_THROUGH, 0.0, ['l', 'l', 'm']),
        ("the skinned mesh where its bone took it", POSED, 0.0, ['h', 'h', 'l']),
        ("the background where the skinned mesh was bound", BOUND, 0.0, ['l', 'l', 'l']),
        ("the background", corner, 0.0, ['l', 'l', 'l']),
    ];
    let wrong: Vec<String> = checks
        .iter()
        .filter_map(|(what, at, z, want)| {
            let (x, y) = pixel(camera, *at, *z);
            let p = image.pixel(x, y);
            (!is(p, *want)).then(|| format!("{what} is {:?} at ({x}, {y})", &p[..3]))
        })
        .collect();
    if wrong.is_empty() { Ok(()) } else { Err(wrong.join("; ")) }
}

/// What the lit check made of the window's GPU ([`probe_lit`]): whether
/// it draws a lit, textured mesh as a tile is drawn, through Moonglow's
/// own shader and through the game's programs.
#[derive(Debug, Clone)]
pub struct Lit {
    /// The backend and the adapter's name.
    pub backend: wgpu::Backend,
    pub adapter: String,
    /// Moonglow's own shader: `Err`, what was drawn wrong.
    pub own: Result<(), String>,
    /// The game's programs (`vslit`/`fslit`, as a lit tile's): `Err`, what
    /// was drawn wrong; `None`, not drawn by them (their pipeline couldn't
    /// be made: Moonglow's own draws such a mesh, and [`Lit::fell_back`]
    /// says why).
    pub game: Option<Result<(), String>>,
    pub fell_back: Option<String>,
    /// Making the renderer and drawing with Moonglow's own; making the
    /// game's pipeline (compiling its programs, or reading them from the
    /// cache) and drawing with it.
    pub own_time: Duration,
    pub game_time: Duration,
    /// What each drew (48 pixels a side): Moonglow's own, and the game's
    /// programs' where they drew; and the color of the mesh's middle in
    /// each.
    pub own_picture: Option<mg_image::Rgba>,
    pub game_picture: Option<mg_image::Rgba>,
    pub own_color: Option<[u8; 3]>,
    pub game_color: Option<[u8; 3]>,
    /// The program files the check read whose copy in the load order isn't
    /// the game's own, and where that copy is (`inc_lighting.shd
    /// (override)`): the check drew the game's own all the same
    /// ([`Assets::game_shader`]), as what they draw is known; the area
    /// view draws the load order's.
    pub overridden: Vec<String>,
}

impl Lit {
    /// The game's programs draw nothing (or wrong) where Moonglow's own
    /// draws right: Moonglow's own should draw instead.
    pub fn game_fails(&self) -> bool {
        self.own.is_ok() && matches!(self.game, Some(Err(_)))
    }

    /// Whether the mesh's middle is the background's (black) in a picture
    /// of the check's (its color, [`Lit::own_color`] or
    /// [`Lit::game_color`]): nothing of the mesh drawn there, rather than
    /// drawn in the wrong colors. `false` where there is no picture.
    pub fn drew_nothing(color: Option<[u8; 3]>) -> bool {
        color.is_some_and(|c| c.iter().all(|&v| v <= BACKGROUND_MAX))
    }
}

impl fmt::Display for Lit {
    /// `Direct3D 12 (AMD Radeon RX 580): Moonglow's own shader drew right
    /// in 90 ms; the game's vslit/fslit drew right in 40 ms`, or what
    /// either drew wrong.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        write!(f, "{} ({}): ", crate::backend::backend_name(self.backend), self.adapter)?;
        let color = |c: Option<[u8; 3]>| c.map(|c| format!(" ({c:?})")).unwrap_or_default();
        match &self.own {
            Ok(()) => write!(f, "Moonglow's own shader drew right{}", color(self.own_color))?,
            Err(why) => write!(f, "Moonglow's own shader drew wrong ({why})")?,
        }
        write!(f, " in {:.0} ms; the game's vslit/fslit ", ms(self.own_time))?;
        match (&self.game, &self.fell_back) {
            (Some(Ok(())), _) => write!(f, "drew right{}", color(self.game_color))?,
            (Some(Err(why)), _) => write!(f, "drew wrong ({why})")?,
            (None, Some(why)) => write!(f, "couldn't be made ({why})")?,
            (None, None) => write!(f, "weren't drawn")?,
        }
        write!(f, " in {:.0} ms", ms(self.game_time))?;
        if !self.overridden.is_empty() {
            write!(
                f,
                " (the game's own files drawn; the load order has {})",
                self.overridden.join(", ")
            )?;
        }
        Ok(())
    }
}

/// The lit check's texture (a 4 by 4 block of red, compressed as BC1 as
/// the game's DDS textures are) over the game's assets (`game`: its own
/// programs, `.shd`, whatever the load order puts over them); and the
/// program files read whose load order's copy isn't the game's.
struct LitAssets<'a> {
    game: &'a dyn Assets,
    overridden: Mutex<Vec<String>>,
}

/// The lit check's texture's name: none of the game's.
const LIT_TEXTURE: &str = "moonglow lit check";

impl Assets for LitAssets<'_> {
    fn texture(&self, name: &str) -> Option<crate::LoadedTexture> {
        (name == LIT_TEXTURE).then(|| crate::LoadedTexture {
            texture: mg_image::Texture {
                width: 4,
                height: 4,
                format: mg_image::Format::Bc1,
                // (Both end colours red, 5:6:5, every texel the first.)
                mips: vec![vec![0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0]],
                has_alpha: false,
                alpha_mean: None,
            },
            txi: Default::default(),
            mtr: None,
        })
    }

    fn shader(&self, name: &str) -> Option<String> {
        if let Some(layer) = self.game.shader_origin(name)
            && let Ok(mut overridden) = self.overridden.lock()
        {
            let file = format!("{name}.shd ({layer})");
            if !overridden.contains(&file) {
                overridden.push(file);
            }
        }
        self.game.game_shader(name)
    }
}

/// Checks that `gpu` (the window's) draws a lit, textured mesh as the area
/// view draws a tile: a quad lit by the area's light, its texture BC1
/// where the GPU reads BC, drawn by Moonglow's own shader and then by the
/// game's programs from `game` (`vslit`/`fslit` and their includes, the
/// game's own copies: a builder's `inc_lighting` in `override/` draws
/// otherwise, and a check of the GPU must know what it should draw; made
/// in the shader cache `cache` where there is one), four samples a pixel,
/// offscreen. Never
/// panics. Run where the game's data is read (its programs are in it),
/// apart from the window's thread.
pub fn probe_lit(gpu: &Gpu, game: &dyn Assets, cache: Option<std::path::PathBuf>) -> Lit {
    let info = gpu.device.adapter_info();
    let mut lit = Lit {
        backend: info.backend,
        adapter: info.name,
        own: Err("not drawn".into()),
        game: None,
        fell_back: None,
        own_time: Duration::ZERO,
        game_time: Duration::ZERO,
        own_picture: None,
        game_picture: None,
        own_color: None,
        game_color: None,
        overridden: Vec::new(),
    };
    let assets = LitAssets { game, overridden: Mutex::default() };
    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let start = Instant::now();
        let mut renderer = Renderer::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
        renderer.set_shader_cache(cache);
        let targets = Targets::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 4, SIDE, SIDE);
        let (scene, camera) = lit_scene(gpu);
        let (mx, my) = pixel(&camera, [0.0, 0.0], 0.0);
        let draw = |r: &mut Renderer, picture: &mut Option<mg_image::Rgba>| {
            r.render_targets(gpu, &assets, &scene, &camera, &targets);
            let image = gpu.try_read_rgba(&targets.color)?;
            let compared = lit_compare(&image, &camera);
            *picture = Some(image);
            compared
        };
        let middle = |p: &Option<mg_image::Rgba>| {
            p.as_ref().map(|p| {
                let c = p.pixel(mx, my);
                [c[0], c[1], c[2]]
            })
        };
        lit.own = draw(&mut renderer, &mut lit.own_picture);
        lit.own_color = middle(&lit.own_picture);
        lit.own_time = start.elapsed();
        let start = Instant::now();
        renderer.game_shaders = true;
        let mut picture = None;
        let drawn = draw(&mut renderer, &mut picture);
        lit.game_time = start.elapsed();
        if renderer.drawn.game > 0 {
            lit.game = Some(drawn);
            lit.game_color = middle(&picture);
            lit.game_picture = picture;
        } else {
            let failures = renderer.game_shader_failures();
            lit.fell_back = Some(match failures.first() {
                // (wgpu's errors run over lines: "Validation Error", then
                // what.)
                Some((what, why)) => {
                    let why: Vec<&str> = why.split_whitespace().collect();
                    format!("{what}: {}", why.join(" "))
                }
                None => "drawn by Moonglow's own".into(),
            });
        }
    }));
    lit.overridden = assets.overridden.into_inner().unwrap_or_default();
    if let Err(payload) = checked {
        let why = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        lit.own = Err(format!("it panicked: {why}"));
    }
    lit
}

/// The lit check's scene: a white quad textured red, lit by the area's
/// default light, seen from above.
fn lit_scene(gpu: &Gpu) -> (Scene, Camera) {
    let mut model = quad("lit check", 6.0, 0.0, [1.0; 3], false);
    if let NodeKind::Mesh(m) = &mut model.nodes[2].kind {
        m.textures[0] = Some(LIT_TEXTURE.into());
        m.uvs[0] = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    }
    let instance = Instance {
        casts_shadows: false,
        ..Instance::new(Arc::new(GpuModel::new(gpu, Arc::new(model))), Mat4::IDENTITY)
    };
    let scene = Scene { instances: vec![instance], background: [0.0; 3], ..Default::default() };
    let camera = Camera {
        eye: Vec3::new(0.0, -0.01, 12.0),
        target: Vec3::ZERO,
        fov_y: 0.8,
        near: 0.1,
        far: 100.0,
    };
    (scene, camera)
}

/// The most a channel of the lit check's background (black) is read as.
const BACKGROUND_MAX: u8 = 50;

/// The lit picture against what it must be: the quad's middle red (lit,
/// its texture's colour), the corner the background's.
fn lit_compare(image: &mg_image::Rgba, camera: &Camera) -> Result<(), String> {
    let (x, y) = pixel(camera, [0.0, 0.0], 0.0);
    let p = image.pixel(x, y);
    let (r, g, b) = (u32::from(p[0]), u32::from(p[1]), u32::from(p[2]));
    let mut wrong = Vec::new();
    if !(r >= 40 && g * 2 <= r && b * 2 <= r) {
        wrong.push(format!("the lit mesh is {:?} at ({x}, {y}), not red", &p[..3]));
    }
    let (cx, cy) = pixel(camera, [5.0, -5.0], 0.0);
    let c = image.pixel(cx, cy);
    if c[..3].iter().any(|&v| v > BACKGROUND_MAX) {
        wrong.push(format!("the background is {:?} at ({cx}, {cy})", &c[..3]));
    }
    if wrong.is_empty() { Ok(()) } else { Err(wrong.join("; ")) }
}
