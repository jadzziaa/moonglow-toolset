//! The GPU: a wgpu device and queue, either shared with the window (egui's)
//! or made headless for offscreen rendering and tests.

use std::sync::mpsc::channel;
use std::time::Duration;

use mg_image::Rgba;

/// The features a device is asked for where its adapter has them: BC
/// textures (the game's are mostly BC-compressed: uploaded as they are)
/// and 32-bit float depth with a stencil (the shadow volumes' depth copy
/// as exact as the scene's; 24 bits without).
pub const OPTIONAL_FEATURES: wgpu::Features =
    wgpu::Features::TEXTURE_COMPRESSION_BC.union(wgpu::Features::DEPTH32FLOAT_STENCIL8);

/// A device and queue.
#[derive(Debug, Clone)]
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Where the meshes uploaded through it lie (`pool.rs`).
    pub(crate) pool: std::sync::Arc<crate::pool::MeshPool>,
}

impl Gpu {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Gpu {
        Gpu { device, queue, pool: Default::default() }
    }

    /// A headless device (BC textures when available), or `None` without
    /// a usable adapter: on [`headless_adapter`].
    pub fn headless() -> Option<Gpu> {
        let adapter = headless_adapter()?;
        let features = adapter.features() & OPTIONAL_FEATURES;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("moonglow"),
            required_features: features,
            required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .ok()?;
        Some(Gpu::new(device, queue))
    }

    /// Whether BC1–BC5 textures can be uploaded as they are.
    pub fn bc_textures(&self) -> bool {
        self.device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
    }

    /// Reads an RGBA8 texture back (rows top first, as rendered).
    pub fn read_rgba(&self, texture: &wgpu::Texture) -> Rgba {
        self.try_read_rgba(texture).expect("texture read back")
    }

    /// [`Gpu::read_rgba`], saying what failed rather than panicking (a GPU
    /// that doesn't finish within 10 seconds, a buffer that can't be
    /// mapped): for the startup check ([`crate::probe`]), on a driver that
    /// may be what is wrong.
    pub fn try_read_rgba(&self, texture: &wgpu::Texture) -> Result<Rgba, String> {
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
            .poll(wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: Some(Duration::from_secs(10)),
            })
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
    let from_environment = wgpu::Backends::from_env();
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
