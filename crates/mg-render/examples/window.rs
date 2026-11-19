//! The renderer presenting to a window of its own, with winit: a
//! [`Screen`] on the window's surface does the rest.
//! `cargo run -p mg-render --example window [-- SECONDS]`: closes itself
//! after SECONDS (default 10; 0 stays open until closed). It makes the
//! window larger once after a second, so the surface is configured again
//! while it runs, and says on standard output what it drew with.
//!
//! The scene is authored here, no game data: a floor and three squares
//! of colour standing on it, turning.

use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::{Mat4, Vec3};
use mg_mdl::{Face, Mesh, Model, Node, NodeKind};
use mg_render::present::{Screen, ScreenOptions};
use mg_render::{AreaLight, Camera, Gpu, GpuModel, Instance, NoAssets, Renderer, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// A square of `half` metres to each side, facing +Z, of one colour.
fn square(half: f32, color: [f32; 3]) -> Model {
    let mesh = Mesh {
        diffuse: color,
        ambient: color,
        render: true,
        vertices: vec![
            [-half, -half, 0.0],
            [half, -half, 0.0],
            [half, half, 0.0],
            [-half, half, 0.0],
        ],
        normals: vec![[0.0, 0.0, 1.0]; 4],
        uvs: [vec![[0.0, 0.0]; 4], vec![], vec![], vec![]],
        faces: vec![
            Face { vertices: [0, 1, 2], material: 0 },
            Face { vertices: [0, 2, 3], material: 0 },
            // (Its back too: it turns.)
            Face { vertices: [0, 2, 1], material: 0 },
            Face { vertices: [0, 3, 2], material: 0 },
        ],
        source: vec![0, 1, 2, 3],
        ..Default::default()
    };
    let mut root = Node::new("square", NodeKind::Dummy);
    root.children = vec![1];
    let mut node = Node::new("face", NodeKind::Mesh(Box::new(mesh)));
    node.parent = Some(0);
    Model {
        name: "square".into(),
        animation_scale: 1.0,
        nodes: vec![root, node],
        ..Default::default()
    }
}

/// What draws once the window is up.
struct Drawing {
    window: Arc<Window>,
    gpu: Gpu,
    screen: Screen,
    renderer: Renderer,
    floor: Arc<GpuModel>,
    squares: Vec<(Arc<GpuModel>, Vec3)>,
}

impl Drawing {
    fn scene(&self, seconds: f32) -> Scene {
        let mut instances = vec![Instance::new(self.floor.clone(), Mat4::IDENTITY)];
        for (i, (model, at)) in self.squares.iter().enumerate() {
            let turn = Mat4::from_rotation_z(seconds * (0.5 + 0.3 * i as f32));
            let stand = Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
            let place = Mat4::from_translation(*at + Vec3::Z) * turn * stand;
            instances.push(Instance::new(model.clone(), place));
        }
        Scene {
            instances,
            area: AreaLight {
                ambient: Vec3::splat(0.25),
                diffuse: Vec3::new(0.9, 0.85, 0.75),
                direction: Vec3::new(0.4, -0.6, 0.7).normalize(),
            },
            background: [0.12, 0.16, 0.24],
            time: seconds,
            ..Default::default()
        }
    }
}

struct App {
    close_after: Option<Duration>,
    began: Instant,
    drawing: Option<Drawing>,
    frames: u64,
    resized: bool,
    failed: Option<String>,
}

impl App {
    fn open(&self, event_loop: &ActiveEventLoop) -> Result<Drawing, String> {
        let attributes = Window::default_attributes()
            .with_title("Moonglow renderer")
            .with_inner_size(PhysicalSize::new(640, 400));
        let window = Arc::new(event_loop.create_window(attributes).map_err(|e| e.to_string())?);
        let instance =
            mg_render::present::instance(Some(Box::new(event_loop.owned_display_handle())));
        let surface = instance.create_surface(window.clone()).map_err(|e| e.to_string())?;
        let (gpu, adapter) =
            mg_render::present::open(&instance, &surface).map_err(|e| e.to_string())?;
        let size = window.inner_size();
        let screen = Screen::new(
            &gpu,
            &adapter,
            surface,
            (size.width, size.height),
            ScreenOptions::default(),
        )
        .map_err(|e| e.to_string())?;
        let renderer = Renderer::new(&gpu, screen.targets_format(), screen.samples());
        let info = adapter.get_info();
        println!(
            "{:?} ({}): surface {:?}, targets {:?}, {} samples, {:?}",
            info.backend,
            info.name,
            screen.format(),
            screen.targets_format(),
            screen.samples(),
            screen.present_mode()
        );
        let model = |m: Model| Arc::new(GpuModel::new(&gpu, Arc::new(m)));
        let floor = model(square(4.0, [0.55, 0.5, 0.45]));
        let squares = vec![
            (model(square(0.8, [0.9, 0.2, 0.15])), Vec3::new(-2.0, 0.0, 0.0)),
            (model(square(0.8, [0.2, 0.8, 0.25])), Vec3::new(0.0, 1.0, 0.0)),
            (model(square(0.8, [0.2, 0.35, 0.95])), Vec3::new(2.0, 0.0, 0.0)),
        ];
        Ok(Drawing { window, gpu, screen, renderer, floor, squares })
    }

    fn draw(&mut self) -> Result<(), String> {
        let Some(d) = &mut self.drawing else { return Ok(()) };
        let seconds = self.began.elapsed().as_secs_f32();
        let scene = d.scene(seconds);
        let camera = Camera::orbit(Vec3::new(0.0, 0.0, 0.8), 9.0, -1.2, 0.5);
        let Some(frame) = d.screen.frame(&d.gpu).map_err(|e| e.to_string())? else {
            return Ok(());
        };
        d.renderer.render_targets(&d.gpu, &NoAssets, &scene, &camera, frame.targets());
        d.window.pre_present_notify();
        let way = frame.present(&d.gpu);
        if self.frames == 0 {
            println!("presented by {way:?}");
        }
        self.frames += 1;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.drawing.is_some() {
            return;
        }
        match self.open(event_loop) {
            Ok(drawing) => {
                drawing.window.request_redraw();
                self.drawing = Some(drawing);
            }
            Err(e) => {
                self.failed = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(d) = &mut self.drawing {
                    d.screen.resize(&d.gpu, (size.width, size.height));
                    println!("resized to {} by {}", size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.draw() {
                    self.failed = Some(e);
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let elapsed = self.began.elapsed();
        if self.close_after.is_some_and(|limit| elapsed >= limit) {
            event_loop.exit();
            return;
        }
        if let Some(d) = &self.drawing {
            // (Once, after a second: the surface configured again while
            // it draws.)
            if !self.resized && elapsed >= Duration::from_secs(1) {
                self.resized = true;
                let _ = d.window.request_inner_size(PhysicalSize::new(800, 500));
            }
            d.window.request_redraw();
        }
    }
}

fn main() {
    let seconds: f32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10.0);
    let event_loop = EventLoop::new().expect("an event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        close_after: (seconds > 0.0).then(|| Duration::from_secs_f32(seconds)),
        began: Instant::now(),
        drawing: None,
        frames: 0,
        resized: false,
        failed: None,
    };
    event_loop.run_app(&mut app).expect("the event loop");
    if let Some(failure) = app.failed {
        eprintln!("the window: {failure}");
        std::process::exit(1);
    }
    let seconds = app.began.elapsed().as_secs_f32();
    println!("{} frames in {seconds:.1} s", app.frames);
}
