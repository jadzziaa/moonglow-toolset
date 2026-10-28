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
//! read yet): the area view's watch ([`crate::watch`]) looks at them as
//! they draw.

use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use glam::{Mat4, Vec3};
use mg_mdl::{Face, Mesh, MeshExtra, Model, Node, NodeKind, Skin};

use crate::{Camera, Gpu, GpuModel, Instance, NoAssets, Renderer, Scene, Targets};

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
