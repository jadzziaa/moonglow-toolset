//! The model viewer: any model from the resources, drawn with the game's
//! lighting, turned by dragging (left: orbit, right or middle: pan, wheel:
//! zoom), with its animations (and its supermodels') to play.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_core::ResType;
use mg_mdl::Model;
use mg_render::anim;
use mg_render::{AreaLight, Camera, Gpu, GpuModel, Instance, Renderer, Scene, Targets};
use mg_resman::ResKey;

use crate::Moonglow;

/// Multisampling for 3D views.
const SAMPLES: u32 = 4;

/// The GPU and renderer the 3D views share, from the window's wgpu state.
pub struct Viewport3d {
    pub render_state: egui_wgpu::RenderState,
    pub gpu: Gpu,
    pub renderer: Renderer,
}

impl std::fmt::Debug for Viewport3d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Viewport3d").finish_non_exhaustive()
    }
}

impl Viewport3d {
    pub fn new(render_state: egui_wgpu::RenderState) -> Viewport3d {
        let gpu = Gpu::new(render_state.device.clone(), render_state.queue.clone());
        let renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, SAMPLES);
        Viewport3d { render_state, gpu, renderer }
    }
}

/// An open model's view.
#[derive(Debug)]
pub struct ModelView {
    model: Result<Arc<GpuModel>, String>,
    /// Supermodels loaded so far, by name.
    supermodels: RefCell<HashMap<String, Option<Arc<Model>>>>,
    pub animation: Option<String>,
    pub playing: bool,
    pub time: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
    targets: Option<(Targets, egui::TextureId)>,
    last_frame: Option<f64>,
}

impl ModelView {
    fn open(app: &Moonglow, key: ResKey) -> ModelView {
        let model = (|| {
            let vp = app.viewport.as_ref().ok_or("No GPU: the model viewer needs one.")?;
            let game = app.game.as_ref().ok_or("No game data.")?;
            let data = app
                .ws
                .as_ref()
                .and_then(|w| w.module.get(&key).map(<[u8]>::to_vec))
                .or_else(|| game.resman.get(&key).ok().map(|d| d.into_owned()))
                .ok_or_else(|| format!("{key} not found"))?;
            let m = Model::read(&data).map_err(|e| format!("{key}: {e}"))?;
            Ok(Arc::new(GpuModel::new(&vp.gpu, Arc::new(m))))
        })();
        let mut view = ModelView {
            model,
            supermodels: RefCell::new(HashMap::new()),
            animation: None,
            playing: true,
            time: 0.0,
            yaw: -60f32.to_radians(),
            pitch: 20f32.to_radians(),
            distance: 5.0,
            target: Vec3::ZERO,
            targets: None,
            last_frame: None,
        };
        view.frame();
        view
    }

    /// Fits the camera to the model's rest pose.
    pub fn frame(&mut self) {
        let Ok(m) = &self.model else { return };
        let pose = mg_render::rest_pose(&m.model);
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for mesh in &m.meshes {
            for c in 0..8 {
                let p = Vec3::new(
                    if c & 1 == 0 { mesh.min.x } else { mesh.max.x },
                    if c & 2 == 0 { mesh.min.y } else { mesh.max.y },
                    if c & 4 == 0 { mesh.min.z } else { mesh.max.z },
                );
                let w = pose[mesh.node].transform_point3(p);
                min = min.min(w);
                max = max.max(w);
            }
        }
        if min.x > max.x {
            (min, max) = (Vec3::splat(-1.0), Vec3::splat(1.0));
        }
        self.target = (min + max) * 0.5;
        let radius = ((max - min).length() * 0.5).max(0.1);
        self.distance = radius / 20f32.to_radians().sin() * 1.1;
    }

    fn camera(&self) -> Camera {
        Camera::orbit(self.target, self.distance, self.yaw, self.pitch)
    }
}

fn load_super(
    app: &Moonglow,
    cache: &RefCell<HashMap<String, Option<Arc<Model>>>>,
    name: &str,
) -> Option<Arc<Model>> {
    let name = name.to_ascii_lowercase();
    if let Some(m) = cache.borrow().get(&name) {
        return m.clone();
    }
    let m = app
        .game
        .as_ref()
        .and_then(|g| g.resman.get_named(&name, ResType::MDL).ok())
        .and_then(|d| Model::read(&d).ok())
        .map(Arc::new);
    cache.borrow_mut().insert(name, m.clone());
    m
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui, key: ResKey) {
    if !app.model_views.contains_key(&key) {
        let view = ModelView::open(app, key);
        app.model_views.insert(key, view);
    }
    let mut view = app.model_views.remove(&key).expect("just inserted");
    let model = match &view.model {
        Ok(m) => m.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
            app.model_views.insert(key, view);
            return;
        }
    };

    // Animations: the model's own, then its supermodels'.
    let anims = anim::animations(&model.model, &|n| load_super(app, &view.supermodels, n));
    ui.horizontal(|ui| {
        ui.label(format!(
            "{}: {} nodes, {} meshes",
            model.model.name,
            model.model.nodes.len(),
            model.meshes.len()
        ));
        if let Some(s) = &model.model.supermodel {
            ui.weak(format!("supermodel {s}"));
        }
        ui.separator();
        let current = view.animation.clone().unwrap_or_else(|| "(rest pose)".into());
        egui::ComboBox::new(("anim", key), "Animation").selected_text(current).show_ui(ui, |ui| {
            if ui.selectable_label(view.animation.is_none(), "(rest pose)").clicked() {
                view.animation = None;
            }
            for (name, _) in &anims {
                if ui.selectable_label(view.animation.as_deref() == Some(name), name).clicked() {
                    view.animation = Some(name.clone());
                    view.time = 0.0;
                }
            }
        });
        if ui.button(if view.playing { "Pause" } else { "Play" }).clicked() {
            view.playing = !view.playing;
        }
        if ui.button("Frame").on_hover_text("Fit the model in the view").clicked() {
            view.frame();
        }
    });

    let Some(vp) = app.viewport.as_mut() else {
        ui.label("No GPU.");
        app.model_views.insert(key, view);
        return;
    };
    let size = ui.available_size().max(egui::vec2(32.0, 32.0));
    let ppp = ui.ctx().pixels_per_point();
    let (w, h) = ((size.x * ppp).round() as u32, (size.y * ppp).round() as u32);
    let (w, h) = (w.clamp(16, 8192), h.clamp(16, 8192));

    // Animation time.
    let now = ui.input(|i| i.time);
    if let (Some(last), true) = (view.last_frame, view.playing) {
        view.time += (now - last) as f32;
    }
    view.last_frame = Some(now);
    let pose = view.animation.as_ref().and_then(|name| {
        let (_, owner) = anims.iter().find(|(n, _)| n == name)?;
        let a = owner.animation(name)?;
        Some(Arc::new(anim::pose(&model.model, a, view.time)))
    });
    if pose.is_some() && view.playing {
        ui.ctx().request_repaint();
    }

    // Render into our own texture, shown as an image.
    if view.targets.as_ref().is_none_or(|(t, _)| t.size != (w, h)) {
        let targets = Targets::new(&vp.gpu, wgpu::TextureFormat::Rgba8Unorm, SAMPLES, w, h);
        let mut egui_renderer = vp.render_state.renderer.write();
        let id = match view.targets.take() {
            Some((_, id)) => {
                egui_renderer.update_egui_texture_from_wgpu_texture(
                    &vp.gpu.device,
                    &targets.color_view,
                    wgpu::FilterMode::Linear,
                    id,
                );
                id
            }
            None => egui_renderer.register_native_texture(
                &vp.gpu.device,
                &targets.color_view,
                wgpu::FilterMode::Linear,
            ),
        };
        view.targets = Some((targets, id));
    }
    let (targets, id) = view.targets.as_ref().expect("made above");
    let scene = Scene {
        instances: vec![Instance { model: model.clone(), transform: Mat4::IDENTITY, pose }],
        area: AreaLight::default(),
        background: [0.16, 0.18, 0.21],
        ..Default::default()
    };
    let resman = app.game.as_ref().map(|g| &g.resman);
    let assets: &dyn mg_render::Assets = match resman {
        Some(rm) => rm,
        None => &mg_render::NoAssets,
    };
    vp.renderer.render(
        &vp.gpu,
        assets,
        &scene,
        &view.camera(),
        targets.render_view(),
        targets.resolve_view(),
        &targets.depth,
        (w, h),
    );
    let response = ui.add(
        egui::Image::new(egui::load::SizedTexture::new(*id, size))
            .sense(egui::Sense::click_and_drag()),
    );

    // Orbit, pan, zoom.
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        view.yaw -= d.x * 0.01;
        view.pitch = (view.pitch + d.y * 0.01).clamp(-1.45, 1.45);
    }
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let d = response.drag_delta();
        let cam = view.camera();
        let fwd = (cam.target - cam.eye).normalize();
        let right = fwd.cross(Vec3::Z).normalize_or_zero();
        let up = right.cross(fwd);
        let scale = view.distance * 0.0015;
        view.target += (-right * d.x + up * d.y) * scale;
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            view.distance = (view.distance * (-scroll * 0.002).exp()).clamp(0.05, 5000.0);
        }
    }
    app.model_views.insert(key, view);
}
