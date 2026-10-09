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
}

impl Gpu {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Gpu {
        Gpu { device, queue }
    }

    /// A headless device (any adapter, BC textures when available), or
    /// `None` without a usable adapter.
    pub fn headless() -> Option<Gpu> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .ok()?;
        let features = adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("moonglow"),
            required_features: features,
            required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .ok()?;
        Some(Gpu { device, queue })
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
