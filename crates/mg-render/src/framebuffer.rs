//! The framebuffer's copies (`docs/framebuffer-proposal.md`): the picture
//! and its depth as drawn after the solid meshes, which the game's
//! programs read as `texFBColor` and `texFBDepth` (`fswater`'s refraction
//! and reflections). Made with the targets they copy ([`Copies`], in
//! [`crate::Targets`]: made late, in a full video memory, textures land in
//! the system's), and filled only in a frame that has a draw reading them.
//!
//! One full-screen draw resolves the multisampled colour (the mean of its
//! samples, as the hardware's resolve) and takes the depth's first sample
//! into an `R32Float` texture (WebGPU resolves no depth, and a depth
//! format read as a float texture is not one to lean on across backends).
//! Its rows are written as OpenGL's copy of its framebuffer has them, the
//! picture's bottom first: a program's screen coordinates made from the
//! projection (y up) then find their pixel, and so do those made from
//! `gl_FragCoord`, which the programs that read the copies count from the
//! bottom (`mg_shader::Program::frag_coord_from_bottom`).

use std::sync::atomic::{AtomicU64, Ordering};

/// Copies of a target's picture and depth, of its size.
#[derive(Debug)]
pub struct Copies {
    pub(crate) color: wgpu::TextureView,
    pub(crate) depth: wgpu::TextureView,
    /// Tells one target's copies from another's (the game's bind groups
    /// are made for one).
    pub(crate) id: u64,
}

/// The depth copy's format.
pub(crate) const DEPTH_COPY: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

impl Copies {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        (width, height): (u32, u32),
    ) -> Copies {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let make = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        Copies {
            color: make("framebuffer colour", format),
            depth: make("framebuffer depth", DEPTH_COPY),
            id: NEXT.fetch_add(1, Ordering::Relaxed),
        }
    }
}

/// The draw that fills the copies, for a target's format and samples.
#[derive(Debug)]
pub(crate) struct Copier {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

fn source(samples: u32) -> String {
    let (color, depth, sum) = if samples > 1 {
        (
            "texture_multisampled_2d<f32>",
            "texture_depth_multisampled_2d",
            format!(
                "var c = vec4<f32>(0.0);\n    for (var s = 0; s < {samples}; s++) {{ c += textureLoad(color, at, s); }}\n    c = c / {samples}.0;"
            ),
        )
    } else {
        ("texture_2d<f32>", "texture_depth_2d", "let c = textureLoad(color, at, 0);".into())
    };
    format!(
        "@group(0) @binding(0) var color: {color};
@group(0) @binding(1) var depth: {depth};
struct Out {{ @location(0) color: vec4<f32>, @location(1) depth: vec4<f32> }}
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}}
@fragment fn fs(@builtin(position) p: vec4<f32>) -> Out {{
    // (OpenGL's rows: the copy's first row is the picture's last.)
    var at = vec2<i32>(p.xy);
    at.y = i32(textureDimensions(color).y) - 1 - at.y;
    {sum}
    let d = textureLoad(depth, at, 0);
    return Out(c, vec4<f32>(d, 0.0, 0.0, 1.0));
}}
"
    )
}

impl Copier {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Copier {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("framebuffer copy"),
            source: wgpu::ShaderSource::Wgsl(source(samples).into()),
        });
        let multisampled = samples > 1;
        let texture = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("framebuffer copy"),
            entries: &[
                texture(0, wgpu::TextureSampleType::Float { filterable: false }),
                texture(1, wgpu::TextureSampleType::Depth),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("framebuffer copy"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("framebuffer copy"),
            layout: Some(&pipeline_layout),
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
                targets: &[Some(format.into()), Some(DEPTH_COPY.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Copier { pipeline, layout }
    }

    /// Fills `copies` from the target drawn into (`color`: multisampled or
    /// not) and its `depth`.
    pub(crate) fn copy(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        copies: &Copies,
    ) {
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("framebuffer copy"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
            ],
        });
        let attach = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("framebuffer copy"),
            color_attachments: &[attach(&copies.color), attach(&copies.depth)],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}
