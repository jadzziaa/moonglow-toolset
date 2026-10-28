//! The GPU: a wgpu device and queue, either shared with the window (egui's)
//! or made headless for offscreen rendering and tests.

use std::sync::mpsc::channel;
use std::time::Duration;

use mg_image::Rgba;

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
    /// a usable adapter. The adapter is the window's on Automatic
    /// ([`crate::backend::pick_adapter`]: wgpu's, but an AMD GPU on
    /// Windows through Direct3D 12), of the backends `WGPU_BACKEND` names
    /// if it is set.
    pub fn headless() -> Option<Gpu> {
        let from_environment = wgpu::Backends::from_env();
        let backends = from_environment.unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
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
        )?;
        let adapter = adapters.into_iter().nth(pick.index)?;
        let features = adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC;
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
            .expect("GPU did not finish");
        rx.recv().expect("map callback").expect("buffer mapped");
        let data = slice.get_mapped_range().expect("mapped range");
        let mut out = Vec::with_capacity(unpadded * h as usize);
        for row in data.chunks_exact(padded) {
            out.extend_from_slice(&row[..unpadded]);
        }
        Rgba { width: w, height: h, data: out }
    }
}
