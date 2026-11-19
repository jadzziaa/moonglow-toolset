//! Presenting the renderer's picture to a window: the targets' picture put
//! into a surface's texture ([`Presenter`]), and the window-side chores
//! around it ([`Screen`]: the surface configured and configured again at
//! each size, its format and present mode picked, a window of no size,
//! a surface outdated or lost, the targets made again with the window).
//!
//! Moonglow Toolset and Moonglow Viewer show their picture inside an egui
//! window (the targets' colour texture registered with egui); a program
//! that draws a window of its own (Moonglow Client's `mc-scene`) presents
//! with these. A [`Screen`] takes a `wgpu::Surface` made by the caller from
//! its window, so mg-render needs no windowing crate:
//! `examples/window.rs` is the whole of it with winit.
//!
//! **Colour.** The renderer writes gamma-space values (the game's
//! shaders, and Moonglow's own, end with the gamma curve), so its targets
//! are of a format that is not sRGB, and what is in them is what the
//! screen should show. Into a surface texture of a format that is not
//! sRGB the values go as they are; into an sRGB one (a surface that
//! offers no other) they are decoded to linear first, so that the
//! hardware's encoding gives them back (within one step of 255, where
//! the round trip rounds). [`Screen`] picks a surface format that is not
//! sRGB wherever one is offered. Float and 10-bit formats are given the
//! values as they are, untried.

use std::collections::HashMap;

use crate::{Gpu, Targets};

/// How the picture goes into the surface's texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// The targets' multisampled picture resolved into it again, by a pass
    /// that draws nothing: multisampled targets of the surface's format.
    Resolve,
    /// A draw over the whole of it that reads the targets' (resolved)
    /// picture texel by texel: one sample a pixel, or another format.
    Draw,
}

impl Way {
    /// How `targets` go into a texture of `format`.
    pub fn of(targets: &Targets, format: wgpu::TextureFormat) -> Way {
        if targets.msaa_view.is_some() && targets.color.format() == format {
            Way::Resolve
        } else {
            Way::Draw
        }
    }
}

const DRAW_FS: &str = "
@group(0) @binding(0) var picture: texture_2d<f32>;

@fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let last = vec2<i32>(textureDimensions(picture)) - vec2<i32>(1);
    let c = textureLoad(picture, min(vec2<i32>(at.xy), last), 0).rgb;
    return vec4<f32>(DECODE, 1.0);
}
";

/// The draw's fragment program for a texture of `format`: an sRGB one has
/// the values decoded (the exact inverse of the sRGB curve).
pub(crate) fn draw_source(format: wgpu::TextureFormat) -> String {
    let decode = if format.is_srgb() {
        "select(pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4)), c / 12.92, \
         c <= vec3<f32>(0.04045))"
    } else {
        "c"
    };
    format!("{}{}", crate::framebuffer::FULLSCREEN_VS, DRAW_FS.replace("DECODE", decode))
}

/// Puts the targets' picture into a texture given by its view: a surface's
/// texture for this frame (or a stand-in of the same format and usage, as
/// the tests use). Only `RENDER_ATTACHMENT` is asked of the texture, which
/// a surface always gives; nothing is copied, so `COPY_DST` isn't.
///
/// After [`Renderer::render_targets`](crate::Renderer::render_targets)
/// (and whatever a caller draws over the picture into
/// [`Targets::resolve_view`], or [`Targets::render_view`] with one
/// sample): the targets keep their picture, so the view's watch
/// ([`crate::watch`]) and the framebuffer's copies read them as ever.
/// Either way costs one pass over the window ([`Way`]). Alpha: the draw
/// writes 1, a resolve keeps what the renderer left (below 1 where
/// something see-through was drawn over the background); [`Screen`] takes
/// an opaque surface where the surface offers one, which ignores it.
#[derive(Debug)]
pub struct Presenter {
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    /// The bind group on the targets drawn from last, by their copies'
    /// number (targets made again have another).
    group: Option<(u64, wgpu::BindGroup)>,
}

impl Presenter {
    pub fn new(gpu: &Gpu) -> Presenter {
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("present"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("present"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        Presenter { layout, pipeline_layout, pipelines: HashMap::new(), group: None }
    }

    /// Puts `targets`' picture into `view`, a texture of `format` of the
    /// targets' size, and submits it: how ([`Way`]) said.
    pub fn present(
        &mut self,
        gpu: &Gpu,
        targets: &Targets,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
    ) -> Way {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("present") });
        let way = self.encode(gpu, &mut encoder, targets, view, format);
        gpu.queue.submit([encoder.finish()]);
        way
    }

    /// [`Presenter::present`] into a caller's encoder, not submitted.
    pub fn encode(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        targets: &Targets,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
    ) -> Way {
        let way = Way::of(targets, format);
        if let (Way::Resolve, Some(msaa)) = (way, &targets.msaa_view) {
            // (A pass that draws nothing still resolves at its end.)
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present resolve"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa,
                    resolve_target: Some(view),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            return way;
        }
        let pipeline = self.pipelines.entry(format).or_insert_with(|| {
            let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("present"),
                source: wgpu::ShaderSource::Wgsl(draw_source(format).into()),
            });
            gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("present"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        });
        let id = targets.copies.id;
        if self.group.as_ref().is_none_or(|(of, _)| *of != id) {
            let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("present"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&targets.color_view),
                }],
            });
            self.group = Some((id, group));
        }
        let Some((_, group)) = &self.group else { unreachable!("made above") };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("present draw"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
        way
    }
}

/// The surface format a [`Screen`] takes of those a surface offers: 8-bit
/// BGRA or RGBA that is not sRGB, else any 8-bit one not sRGB, else an
/// sRGB one (decoded into, [`Presenter`]), else the first offered.
pub fn pick_format(offered: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    use wgpu::TextureFormat as F;
    [F::Bgra8Unorm, F::Rgba8Unorm]
        .into_iter()
        .find(|f| offered.contains(f))
        .or_else(|| offered.iter().copied().find(|f| !f.is_srgb() && targets_format(*f) == *f))
        .or_else(|| offered.iter().copied().find(|f| f.is_srgb()))
        .or_else(|| offered.first().copied())
}

/// The format of the targets drawn for a surface of `surface`: the same
/// where the renderer draws into it (8-bit RGBA or BGRA, so the picture
/// is resolved straight into the surface's texture), its counterpart that
/// is not sRGB for an sRGB one, else 8-bit RGBA.
pub fn targets_format(surface: wgpu::TextureFormat) -> wgpu::TextureFormat {
    use wgpu::TextureFormat as F;
    match surface.remove_srgb_suffix() {
        f @ (F::Rgba8Unorm | F::Bgra8Unorm) => f,
        _ => F::Rgba8Unorm,
    }
}

/// The present mode a [`Screen`] takes for `wanted`: it, where the
/// surface offers it (wgpu's two automatic ones always are), else `Fifo`
/// (vertical sync), which every surface offers.
pub fn pick_present_mode(
    wanted: wgpu::PresentMode,
    offered: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    use wgpu::PresentMode as P;
    if matches!(wanted, P::AutoVsync | P::AutoNoVsync) || offered.contains(&wanted) {
        wanted
    } else {
        P::Fifo
    }
}

/// What a [`Screen`] is made with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenOptions {
    /// Samples a pixel the targets are asked for (as the device gives
    /// them, [`Gpu::samples`]: one on OpenGL).
    pub samples: u32,
    /// `AutoVsync` (the default) waits for the display, `AutoNoVsync`
    /// doesn't; another the surface doesn't offer is `Fifo`.
    pub present_mode: wgpu::PresentMode,
    /// Frames the GPU may be given before the one shown (wgpu's
    /// `desired_maximum_frame_latency`).
    pub frame_latency: u32,
}

impl Default for ScreenOptions {
    fn default() -> ScreenOptions {
        ScreenOptions { samples: 4, present_mode: wgpu::PresentMode::AutoVsync, frame_latency: 2 }
    }
}

/// Why a [`Screen`] can't go on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenError {
    /// No adapter can present to the surface (or no device was given).
    NoAdapter(String),
    /// The surface offers no format to present in.
    NoFormat,
    /// The surface is lost and stays lost when configured again: the
    /// window's owner makes another from the window
    /// (`wgpu::Instance::create_surface`) and gives it to
    /// [`Screen::set_surface`].
    Lost,
    /// wgpu refused the surface's texture (a validation error).
    Refused,
}

impl std::fmt::Display for ScreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenError::NoAdapter(e) => write!(f, "no graphics adapter draws to this window: {e}"),
            ScreenError::NoFormat => f.write_str("the window's surface offers no format"),
            ScreenError::Lost => f.write_str("the window's surface was lost"),
            ScreenError::Refused => f.write_str("the window's surface refused a frame"),
        }
    }
}

impl std::error::Error for ScreenError {}

/// An instance of the backends Moonglow draws with, for a window's
/// surface: those `WGPU_BACKEND` names, else Vulkan, Direct3D 12 or
/// Metal, with OpenGL where nothing else draws (as [`Gpu::headless`]
/// takes them; [`open`] prefers the others). `display` is the window
/// system's display (winit's `OwnedDisplayHandle`), which wgpu's OpenGL
/// back end needs on Wayland; the others don't use it.
pub fn instance(display: Option<Box<dyn wgpu::wgt::WgpuHasDisplayHandle>>) -> wgpu::Instance {
    let (from_environment, unknown) = crate::backend::backends_from_env();
    if let Some(said) = crate::backend::unknown_backends_said(&unknown, from_environment.is_some())
    {
        log::warn!("{said}");
    }
    let mut descriptor = wgpu::InstanceDescriptor {
        backends: from_environment.unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL),
        backend_options: crate::dx12::backend_options(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    };
    descriptor.display = display;
    wgpu::Instance::new(descriptor)
}

/// A device that presents to `surface`, on the adapter Moonglow would take
/// among those that can ([`crate::backend::pick_adapter`]: wgpu's order,
/// but an AMD GPU on Windows through Direct3D 12, and OpenGL or a software
/// adapter only where there is no other), with the features and limits
/// the renderer asks for. The adapter is for [`Screen::new`].
pub fn open(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> Result<(Gpu, wgpu::Adapter), ScreenError> {
    let (from_environment, _) = crate::backend::backends_from_env();
    let adapters: Vec<wgpu::Adapter> =
        pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
            .into_iter()
            .filter(|a| a.is_surface_supported(surface))
            .collect();
    let infos: Vec<wgpu::AdapterInfo> = adapters.iter().map(|a| a.get_info()).collect();
    let pick = crate::backend::pick_adapter(
        &infos,
        None,
        from_environment.is_some(),
        wgpu::PowerPreference::HighPerformance,
        crate::backend::Platform::current(),
        &[],
    )
    .ok_or_else(|| ScreenError::NoAdapter(crate::backend::describe_adapters(&infos)))?;
    let adapter = adapters.into_iter().nth(pick.index).expect("picked among them");
    let limits = wgpu::Limits::default().using_resolution(adapter.limits());
    let descriptor = crate::backend::device_descriptor("moonglow", adapter.features(), limits);
    let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))
        .map_err(|e| ScreenError::NoAdapter(e.to_string()))?;
    Ok((Gpu::new(device, queue), adapter))
}

/// The targets of a window's size and what presents them: the part of a
/// [`Screen`] that needs no surface (tested without one).
#[derive(Debug)]
struct Fit {
    format: wgpu::TextureFormat,
    samples: u32,
    size: (u32, u32),
    /// `None` while the window has no size.
    targets: Option<Targets>,
    presenter: Presenter,
}

impl Fit {
    fn new(gpu: &Gpu, format: wgpu::TextureFormat, samples: u32, size: (u32, u32)) -> Fit {
        let samples = gpu.samples(samples);
        let mut fit =
            Fit { format, samples, size: (0, 0), targets: None, presenter: Presenter::new(gpu) };
        fit.resize(gpu, size);
        fit
    }

    /// Whether the size changed (and the surface is to be configured).
    fn resize(&mut self, gpu: &Gpu, size: (u32, u32)) -> bool {
        if size == self.size && (self.targets.is_some() || size.0 == 0 || size.1 == 0) {
            return false;
        }
        self.size = size;
        self.targets = (size.0 > 0 && size.1 > 0)
            .then(|| Targets::new(gpu, self.format, self.samples, size.0, size.1));
        true
    }
}

/// What to do about what the surface gave for a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Acquire {
    /// Draw it (and configure the surface again after presenting it:
    /// suboptimal).
    Draw {
        configure_after: bool,
    },
    /// Configure the surface again and ask once more (outdated, lost).
    Configure {
        lost: bool,
    },
    /// Nothing this frame (timed out, occluded).
    Skip,
    Refused,
}

fn acquire(status: &wgpu::CurrentSurfaceTexture) -> Acquire {
    use wgpu::CurrentSurfaceTexture as C;
    match status {
        C::Success(_) => Acquire::Draw { configure_after: false },
        C::Suboptimal(_) => Acquire::Draw { configure_after: true },
        C::Outdated => Acquire::Configure { lost: false },
        C::Lost => Acquire::Configure { lost: true },
        C::Timeout | C::Occluded => Acquire::Skip,
        C::Validation => Acquire::Refused,
    }
}

/// A window's surface and the renderer's targets of its size: what a
/// program that presents to a window would otherwise write itself.
///
/// ```ignore
/// let instance = mg_render::present::instance(Some(Box::new(event_loop.owned_display_handle())));
/// let surface = instance.create_surface(window.clone())?;
/// let (gpu, adapter) = mg_render::present::open(&instance, &surface)?;
/// let mut screen = Screen::new(&gpu, &adapter, surface, (w, h), ScreenOptions::default())?;
/// let mut renderer = Renderer::new(&gpu, screen.targets_format(), screen.samples());
/// // On a resize:
/// screen.resize(&gpu, (w, h));
/// // On a redraw:
/// if let Some(frame) = screen.frame(&gpu)? {
///     renderer.render_targets(&gpu, assets, &scene, &camera, frame.targets());
///     window.pre_present_notify();
///     frame.present(&gpu);
/// }
/// ```
#[derive(Debug)]
pub struct Screen {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    fit: Fit,
    /// Whether the surface is configured at the window's size (not while it
    /// has none).
    configured: bool,
}

impl Screen {
    /// A screen on `surface` (made from a window on `adapter`'s instance)
    /// of the window's `size` in pixels, which may be none (a minimized
    /// window): nothing is drawn until it has one.
    pub fn new(
        gpu: &Gpu,
        adapter: &wgpu::Adapter,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
        options: ScreenOptions,
    ) -> Result<Screen, ScreenError> {
        let capabilities = surface.get_capabilities(adapter);
        let format = pick_format(&capabilities.formats).ok_or(ScreenError::NoFormat)?;
        let alpha_mode = if capabilities.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            capabilities.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Auto)
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: pick_present_mode(options.present_mode, &capabilities.present_modes),
            desired_maximum_frame_latency: options.frame_latency,
            alpha_mode,
            view_formats: Vec::new(),
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        let fit = Fit::new(gpu, targets_format(format), options.samples, (0, 0));
        let mut screen = Screen { surface, config, fit, configured: false };
        screen.resize(gpu, size);
        Ok(screen)
    }

    /// The window is now `size` pixels (none: nothing is drawn until it
    /// has a size again): the surface configured for it, and the targets
    /// made again.
    pub fn resize(&mut self, gpu: &Gpu, size: (u32, u32)) {
        self.fit.resize(gpu, size);
        self.configured = false;
        if size.0 > 0 && size.1 > 0 {
            (self.config.width, self.config.height) = size;
            self.configure(gpu);
        }
    }

    fn configure(&mut self, gpu: &Gpu) {
        self.surface.configure(&gpu.device, &self.config);
        self.configured = true;
    }

    /// A surface made anew from the window after [`ScreenError::Lost`],
    /// configured as the one before.
    pub fn set_surface(&mut self, gpu: &Gpu, surface: wgpu::Surface<'static>) {
        self.surface = surface;
        self.configured = false;
        if self.fit.targets.is_some() {
            self.configure(gpu);
        }
    }

    /// The window's size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.fit.size
    }

    /// The surface's format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// The present mode in use.
    pub fn present_mode(&self) -> wgpu::PresentMode {
        self.config.present_mode
    }

    /// The format of the targets: a [`Renderer`](crate::Renderer) drawing
    /// into them is made with it ([`targets_format`]).
    pub fn targets_format(&self) -> wgpu::TextureFormat {
        self.fit.format
    }

    /// The samples a pixel the targets have: a renderer drawing into them
    /// is made with it.
    pub fn samples(&self) -> u32 {
        self.fit.samples
    }

    /// The targets of the window's size (`None` while it has none).
    pub fn targets(&self) -> Option<&Targets> {
        self.fit.targets.as_ref()
    }

    /// The surface's texture for a frame, with the targets to draw into:
    /// `None` when there is nothing to draw to this time (the window has no
    /// size, is hidden, or the surface timed out or stays outdated); the
    /// next frame asks again. An outdated or lost surface is configured
    /// again and asked once more.
    pub fn frame(&mut self, gpu: &Gpu) -> Result<Option<Frame<'_>>, ScreenError> {
        if self.fit.targets.is_none() {
            return Ok(None);
        }
        if !self.configured {
            self.configure(gpu);
        }
        let mut again = true;
        let (texture, configure_after) = loop {
            let status = self.surface.get_current_texture();
            match acquire(&status) {
                Acquire::Draw { configure_after } => {
                    let (wgpu::CurrentSurfaceTexture::Success(texture)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(texture)) = status
                    else {
                        unreachable!("a texture to draw")
                    };
                    break (texture, configure_after);
                }
                Acquire::Configure { lost } => {
                    self.configure(gpu);
                    if !std::mem::take(&mut again) {
                        return if lost { Err(ScreenError::Lost) } else { Ok(None) };
                    }
                }
                Acquire::Skip => return Ok(None),
                Acquire::Refused => return Err(ScreenError::Refused),
            }
        };
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Some(Frame { screen: self, texture, view, configure_after }))
    }
}

/// A frame of a [`Screen`]: draw into [`Frame::targets`], then
/// [`Frame::present`]. Dropped unpresented, the surface's texture is
/// given back unshown.
#[derive(Debug)]
pub struct Frame<'a> {
    screen: &'a mut Screen,
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    configure_after: bool,
}

impl Frame<'_> {
    /// The targets to draw the frame into
    /// ([`Renderer::render_targets`](crate::Renderer::render_targets)).
    pub fn targets(&self) -> &Targets {
        self.screen.fit.targets.as_ref().expect("a frame only with targets")
    }

    /// The frame's size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.screen.fit.size
    }

    /// The surface's texture, for a caller that draws over the picture
    /// after it is put there (before [`Frame::present`] is not possible:
    /// it puts it there; draw over the targets instead).
    pub fn surface_texture(&self) -> &wgpu::Texture {
        &self.texture.texture
    }

    /// Puts the targets' picture into the surface's texture and shows it
    /// (winit: call the window's `pre_present_notify` just before).
    pub fn present(self, gpu: &Gpu) -> Way {
        let Frame { screen, texture, view, configure_after } = self;
        let fit = &mut screen.fit;
        let targets = fit.targets.as_ref().expect("a frame only with targets");
        let way = fit.presenter.present(gpu, targets, &view, screen.config.format);
        drop(view);
        gpu.queue.present(texture);
        if configure_after {
            screen.configure(gpu);
        }
        way
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use glam::{Mat4, Vec3};
    use wgpu::TextureFormat as F;

    use super::{Acquire, Fit, Presenter, Way};
    use crate::{AreaLight, Camera, Gpu, GpuModel, Instance, NoAssets, Renderer, Scene, Targets};

    fn gpu() -> Option<Gpu> {
        mg_testkit::gpu::open(Gpu::headless)
    }

    /// A lit square turned against the camera (edges across pixels, where
    /// the samples differ) on a coloured background.
    fn scene(gpu: &Gpu) -> (Scene, Camera) {
        use mg_mdl::{Face, Mesh, Model, Node, NodeKind};
        let mesh = Mesh {
            diffuse: [1.0, 0.6, 0.3],
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
        let mut node = Node::new("plane", NodeKind::Mesh(Box::new(mesh)));
        node.parent = Some(0);
        let nodes = vec![root, node];
        let model =
            Model { name: "quad".into(), animation_scale: 1.0, nodes, ..Default::default() };
        let model = Arc::new(GpuModel::new(gpu, Arc::new(model)));
        let scene = Scene {
            instances: vec![Instance::new(model, Mat4::from_rotation_z(0.5))],
            area: AreaLight {
                ambient: Vec3::splat(0.2),
                diffuse: Vec3::new(0.8, 0.7, 0.6),
                direction: Vec3::new(0.3, -0.5, 0.8).normalize(),
            },
            background: [0.1, 0.3, 0.6],
            ..Default::default()
        };
        (scene, Camera::orbit(Vec3::ZERO, 4.0, 0.4, 0.9))
    }

    /// A texture standing in for a surface's: its format, and the usage a
    /// surface is configured with, and copying out for the test to read.
    fn stand_in(gpu: &Gpu, format: F, (width, height): (u32, u32)) -> wgpu::Texture {
        gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("surface stand-in"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// A texture of 8-bit texels read back as RGBA.
    fn read(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
        let mut data = gpu.read_rgba(texture).data;
        if matches!(texture.format().remove_srgb_suffix(), F::Bgra8Unorm) {
            data.as_chunks_mut::<4>().0.iter_mut().for_each(|t| t.swap(0, 2));
        }
        data
    }

    /// What is presented is what the targets read back give: resolved
    /// again or drawn, into each format a surface offers here (BGRA, RGBA
    /// and sRGB), from targets of either order, at one sample and four.
    #[test]
    fn what_is_presented_is_what_the_targets_hold() {
        let Some(gpu) = gpu() else { return };
        let (scene, camera) = scene(&gpu);
        let size = (96, 64);
        for samples in [1, 4] {
            for targets_format in [F::Rgba8Unorm, F::Bgra8Unorm] {
                let mut renderer = Renderer::new(&gpu, targets_format, samples);
                let targets = Targets::new(&gpu, targets_format, samples, size.0, size.1);
                renderer.render_targets(&gpu, &NoAssets, &scene, &camera, &targets);
                let want = read(&gpu, &targets.color);
                let colors: std::collections::HashSet<&[u8; 4]> =
                    want.as_chunks().0.iter().collect();
                assert!(colors.len() > 3, "a picture: {} colours", colors.len());
                let mut presenter = Presenter::new(&gpu);
                // (Each twice: the targets keep their picture, the
                // multisampled one too.)
                for format in [F::Bgra8Unorm, F::Rgba8Unorm, F::Bgra8UnormSrgb].repeat(2) {
                    let surface = stand_in(&gpu, format, size);
                    let view = surface.create_view(&Default::default());
                    let way = presenter.present(&gpu, &targets, &view, format);
                    let what = format!("{samples} samples, {targets_format:?} into {format:?}");
                    let resolves = targets.samples > 1 && format == targets_format;
                    assert_eq!(way, if resolves { Way::Resolve } else { Way::Draw }, "{what}");
                    let got = read(&gpu, &surface);
                    // (sRGB: decoded and encoded again, within a step.)
                    let allowed = if format.is_srgb() { 1 } else { 0 };
                    for (i, (g, w)) in
                        got.as_chunks::<4>().0.iter().zip(want.as_chunks::<4>().0).enumerate()
                    {
                        let near = (0..3).all(|c| g[c].abs_diff(w[c]) <= allowed);
                        assert!(near && g[3] == 255, "{what}: pixel {i}: {g:?}, read back {w:?}");
                    }
                    assert!(read(&gpu, &targets.color) == want, "{what}: the targets kept");
                }
                // The view's watch reads them as ever after a present.
                let mut sampler = crate::watch::Sampler::new(&gpu, samples);
                let points = (crate::watch::COLUMNS * crate::watch::ROWS) as usize;
                let expected = crate::watch::Expected(vec![true; points]);
                sampler.look(&gpu, &targets, expected, 0.0);
                let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
                let look = sampler.take(&gpu).expect("the look, once the GPU is done");
                let (_, drawn) = look.counted();
                assert!(drawn > 50 && drawn < points, "the square seen: {drawn} of {points}");
                assert!(look.colored(scene.background) >= drawn / 2, "in colour");
                renderer.let_go_of(&targets);
            }
        }
    }

    /// A window's targets follow its size: made again at another (and
    /// what presents them binds the new ones), none while it has no size,
    /// and the same ones kept at the same size.
    #[test]
    fn the_targets_follow_the_window_s_size() {
        let Some(gpu) = gpu() else { return };
        let (scene, camera) = scene(&gpu);
        let mut fit = Fit::new(&gpu, F::Bgra8Unorm, 4, (64, 48));
        assert_eq!(fit.samples, gpu.samples(4));
        let mut renderer = Renderer::new(&gpu, F::Bgra8Unorm, fit.samples);
        let mut presented = |fit: &mut Fit| {
            let targets = fit.targets.as_ref().expect("targets");
            assert_eq!(targets.size, fit.size);
            renderer.render_targets(&gpu, &NoAssets, &scene, &camera, targets);
            let surface = stand_in(&gpu, F::Bgra8Unorm, fit.size);
            let view = surface.create_view(&Default::default());
            fit.presenter.present(&gpu, targets, &view, F::Bgra8Unorm);
            assert_eq!(read(&gpu, &surface), read(&gpu, &targets.color), "{:?}", fit.size);
        };
        presented(&mut fit);
        let id = fit.targets.as_ref().map(|t| t.copies.id);
        assert!(!fit.resize(&gpu, (64, 48)), "the same size");
        assert_eq!(fit.targets.as_ref().map(|t| t.copies.id), id, "the same targets");
        assert!(fit.resize(&gpu, (80, 60)));
        presented(&mut fit);
        // No size: no targets, until it has one again.
        for none in [(0, 60), (80, 0), (0, 0)] {
            fit.resize(&gpu, none);
            assert!(fit.targets.is_none(), "{none:?}");
        }
        assert!(!fit.resize(&gpu, (0, 0)), "still none");
        assert!(fit.resize(&gpu, (33, 17)));
        presented(&mut fit);
    }

    /// The surface formats taken, the targets' for each, and the present
    /// modes.
    #[test]
    fn a_surface_s_format_and_present_mode_are_picked() {
        use super::{pick_format, pick_present_mode, targets_format};
        use wgpu::PresentMode as P;
        // (Vulkan on Mesa offers these on X11 and Wayland.)
        let offered = [F::Bgra8UnormSrgb, F::Bgra8Unorm, F::Rgba8UnormSrgb, F::Rgba8Unorm];
        assert_eq!(pick_format(&offered), Some(F::Bgra8Unorm));
        assert_eq!(pick_format(&[F::Rgba8UnormSrgb, F::Rgba8Unorm]), Some(F::Rgba8Unorm));
        assert_eq!(pick_format(&[F::Rgb10a2Unorm, F::Bgra8UnormSrgb]), Some(F::Bgra8UnormSrgb));
        assert_eq!(pick_format(&[F::Rgba16Float]), Some(F::Rgba16Float));
        assert_eq!(pick_format(&[]), None);
        assert_eq!(targets_format(F::Bgra8Unorm), F::Bgra8Unorm);
        assert_eq!(targets_format(F::Bgra8UnormSrgb), F::Bgra8Unorm);
        assert_eq!(targets_format(F::Rgba8UnormSrgb), F::Rgba8Unorm);
        assert_eq!(targets_format(F::Rgba16Float), F::Rgba8Unorm);
        let offered = [P::Fifo, P::Mailbox];
        assert_eq!(pick_present_mode(P::AutoVsync, &offered), P::AutoVsync);
        assert_eq!(pick_present_mode(P::Mailbox, &offered), P::Mailbox);
        assert_eq!(pick_present_mode(P::Immediate, &offered), P::Fifo);
    }

    /// What is done about a surface that gives no texture.
    #[test]
    fn a_surface_that_gives_no_texture_is_configured_again_or_skipped() {
        use wgpu::CurrentSurfaceTexture as C;
        assert_eq!(super::acquire(&C::Outdated), Acquire::Configure { lost: false });
        assert_eq!(super::acquire(&C::Lost), Acquire::Configure { lost: true });
        assert_eq!(super::acquire(&C::Timeout), Acquire::Skip);
        assert_eq!(super::acquire(&C::Occluded), Acquire::Skip);
        assert_eq!(super::acquire(&C::Validation), Acquire::Refused);
    }
}
