//! The GPU: a wgpu device and queue, either shared with the window (egui's)
//! or made headless for offscreen rendering and tests.

use std::sync::mpsc::channel;
use std::time::Duration;

use mg_image::Rgba;

/// The features a device is asked for where its adapter has them: BC
/// textures (the game's are mostly BC-compressed: uploaded as they are),
/// 32-bit float depth with a stencil (the shadow volumes' depth copy as
/// exact as the scene's; 24 bits without), and the adapter's own format
/// features (without them a target takes 1 or 4 samples a pixel, never
/// 2: Options › Area › Anti-aliasing, [`Gpu::sample_counts`]).
pub const OPTIONAL_FEATURES: wgpu::Features = wgpu::Features::TEXTURE_COMPRESSION_BC
    .union(wgpu::Features::DEPTH32FLOAT_STENCIL8)
    .union(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);

/// The samples a pixel a 3D view may be drawn with: none (1), 2 and 4.
pub const SAMPLE_COUNTS: [u32; 3] = [1, 2, 4];

/// A device and queue.
#[derive(Debug, Clone)]
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Where the meshes uploaded through it lie (`pool.rs`).
    pub(crate) pool: std::sync::Arc<crate::pool::MeshPool>,
    /// The largest side a texture is uploaded at, pixels (0: as it is):
    /// [`Gpu::set_texture_limit`].
    texture_limit: std::sync::Arc<std::sync::atomic::AtomicU32>,
    /// Whether the textures read as RGBA (TGA, PLT) are kept so, not
    /// compressed as they are read: [`Gpu::set_compress_read_textures`].
    keep_rgba: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Gpu {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Gpu {
        Gpu {
            device,
            queue,
            pool: Default::default(),
            texture_limit: Default::default(),
            keep_rgba: Default::default(),
        }
    }

    /// Textures uploaded from now on through this device (and its
    /// clones) are at most `limit` pixels a side: the levels of their
    /// mip chain larger than that are left out, as a game's lower texture
    /// quality leaves them (a quarter of the memory a level). `None`: as
    /// they are. A BC texture without the smaller levels is uploaded as
    /// it is (made smaller, as RGBA8, it would take more). Those uploaded
    /// before stay as they are: read them again (`Renderer::clear_textures`).
    pub fn set_texture_limit(&self, limit: Option<u32>) {
        let limit = limit.unwrap_or(0);
        self.texture_limit.store(limit, std::sync::atomic::Ordering::Relaxed);
    }

    /// Whether the textures the 3D views read as RGBA (TGA and PLT) are
    /// compressed to BC as they are read from now on, through this device
    /// and its clones (`texture::compressed`): on unless switched off,
    /// where the device reads BC ([`Gpu::bc_textures`]). Those uploaded
    /// before stay as they are.
    pub fn set_compress_read_textures(&self, on: bool) {
        self.keep_rgba.store(!on, std::sync::atomic::Ordering::Relaxed);
    }

    /// Whether textures read as RGBA are compressed as they are read
    /// ([`Gpu::set_compress_read_textures`]): never without BC.
    pub fn compresses_read_textures(&self) -> bool {
        !self.keep_rgba.load(std::sync::atomic::Ordering::Relaxed) && self.bc_textures()
    }

    /// What the meshes uploaded through this device take of its memory
    /// (their chunks, `pool.rs`), and what of it they use.
    pub fn mesh_memory(&self) -> crate::MeshMemory {
        self.pool.memory()
    }

    /// The largest side a texture is uploaded at ([`Gpu::set_texture_limit`]).
    pub fn texture_limit(&self) -> Option<u32> {
        Some(self.texture_limit.load(std::sync::atomic::Ordering::Relaxed)).filter(|&l| l > 0)
    }

    /// A headless device (BC textures when available), or `None` without
    /// a usable adapter: on [`headless_adapter`].
    pub fn headless() -> Option<Gpu> {
        let adapter = headless_adapter()?;
        let limits = wgpu::Limits::default().using_resolution(adapter.limits());
        let descriptor = crate::backend::device_descriptor("moonglow", adapter.features(), limits);
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).ok()?;
        Some(Gpu::new(device, queue))
    }

    /// The samples a pixel a target asked for with `wanted` gets on this
    /// device: `wanted`, but one on wgpu's OpenGL back end.
    /// [`Renderer::new`](crate::Renderer::new), [`Targets::new`](crate::Targets::new)
    /// and [`watch::Sampler::new`](crate::watch::Sampler::new) ask it, and
    /// so must a caller's own pipeline that draws into the targets.
    ///
    /// Why: the renderer's multisampled targets are read as textures too
    /// (the framebuffer's copies, the shadows' darkening, the shadow
    /// volumes' depth copy, the watch), and wgpu-hal 30's OpenGL back end
    /// never makes a `GL_TEXTURE_2D_MULTISAMPLE` texture: it gives such a
    /// texture `glTexStorage2DMultisample` on `GL_TEXTURE_2D`, which the
    /// driver refuses (`GL_INVALID_ENUM`, reported by nothing but
    /// `MESA_DEBUG`), so nothing drawn into it arrives. A multisampled
    /// attachment that is never read as a texture works there; anti-aliasing
    /// on OpenGL is what is given up.
    pub fn samples(&self, wanted: u32) -> u32 {
        if self.device.adapter_info().backend == wgpu::Backend::Gl { 1 } else { wanted }
    }

    /// Of [`SAMPLE_COUNTS`], those a 3D view drawn into `color` targets
    /// can take on this device (made on `adapter`): every multisampled
    /// target's format must take the count (the picture, its depth, the
    /// shadows' mask, the shadow volumes' depth and stencil). One always;
    /// on OpenGL only one ([`Gpu::samples`]); without the adapter's own
    /// format features, one and four, as WebGPU guarantees.
    pub fn sample_counts(&self, adapter: &wgpu::Adapter, color: wgpu::TextureFormat) -> Vec<u32> {
        if self.samples(4) == 1 {
            return vec![1];
        }
        let specific = self
            .device
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        let formats = [
            color,
            crate::renderer::DEPTH_FORMAT,
            crate::shadow::MASK_FORMAT,
            crate::volume::stencil_format(&self.device),
        ];
        SAMPLE_COUNTS
            .into_iter()
            .filter(|&n| {
                n == 1
                    || match specific {
                        true => formats.iter().all(|f| {
                            adapter.get_texture_format_features(*f).flags.sample_count_supported(n)
                        }),
                        false => n == 4,
                    }
            })
            .collect()
    }

    /// Whether BC1–BC5 textures can be uploaded as they are.
    pub fn bc_textures(&self) -> bool {
        self.device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
    }

    /// Reads an RGBA8 texture back (rows top first, as rendered), waiting
    /// up to two minutes for the GPU: a software rasterizer (Windows' WARP
    /// on a busy CI runner) can take longer than the startup check's 10
    /// seconds over a large scene.
    pub fn read_rgba(&self, texture: &wgpu::Texture) -> Rgba {
        self.read_rgba_within(texture, Duration::from_secs(120)).expect("texture read back")
    }

    /// [`Gpu::read_rgba`], saying what failed rather than panicking (a GPU
    /// that doesn't finish within 10 seconds, a buffer that can't be
    /// mapped): for the startup check ([`crate::probe`]), on a driver that
    /// may be what is wrong.
    pub fn try_read_rgba(&self, texture: &wgpu::Texture) -> Result<Rgba, String> {
        self.read_rgba_within(texture, Duration::from_secs(10))
    }

    fn read_rgba_within(&self, texture: &wgpu::Texture, wait: Duration) -> Result<Rgba, String> {
        let (w, h) = (texture.width(), texture.height());
        let unpadded = w as usize * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * h as usize) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let index = self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        let (tx, rx) = channel();
        slice.map_async(wgpu::MapMode::Read, move |r| drop(tx.send(r)));
        self.device
            .poll(wgpu::PollType::Wait { submission_index: Some(index), timeout: Some(wait) })
            .map_err(|e| format!("the GPU did not finish: {e}"))?;
        rx.recv_timeout(Duration::from_secs(1))
            .map_err(|_| "the read-back buffer was not mapped".to_string())?
            .map_err(|e| format!("the read-back buffer could not be mapped: {e}"))?;
        let data = slice.get_mapped_range().map_err(|e| format!("its mapped range: {e}"))?;
        let mut out = Vec::with_capacity(unpadded * h as usize);
        for row in data.chunks_exact(padded) {
            out.extend_from_slice(&row[..unpadded]);
        }
        Ok(Rgba { width: w, height: h, data: out })
    }
}

/// The adapter a headless device is opened on ([`Gpu::headless`]): the
/// window's on Automatic ([`crate::backend::pick_adapter`]: wgpu's, but an
/// AMD GPU on Windows through Direct3D 12), of the backends `WGPU_BACKEND`
/// names if it is set; `None` without one. Direct3D 12 compiles with the
/// compiler [`crate::dx12`] chose.
pub fn headless_adapter() -> Option<wgpu::Adapter> {
    // (Set but naming no backend: as not set, and said.)
    let (from_environment, unknown) = crate::backend::backends_from_env();
    if let Some(said) = crate::backend::unknown_backends_said(&unknown, from_environment.is_some())
    {
        eprintln!("{said}");
    }
    let backends = from_environment.unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        backend_options: crate::dx12::backend_options(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(backends));
    let infos: Vec<wgpu::AdapterInfo> = adapters.iter().map(|a| a.get_info()).collect();
    let pick = crate::backend::pick_adapter(
        &infos,
        None,
        from_environment.is_some(),
        wgpu::PowerPreference::HighPerformance,
        crate::backend::Platform::current(),
        &[],
    )?;
    adapters.into_iter().nth(pick.index)
}
