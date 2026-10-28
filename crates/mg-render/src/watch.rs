//! The area view's watch: whether what it draws keeps drawing.
//!
//! The startup check ([`crate::probe`]) catches a backend that draws
//! nothing from the first frame; El Grillo's Radeon PRO drew an area's
//! tiles when it opened and lost them one by one, and the game's own
//! shaders (on by default) aren't drawn until the game's data is read. So
//! the area view looks, every second or so, at what the GPU drew against
//! what it should have:
//!
//! - **The expectation** ([`expected`]), on the CPU: of a grid of points
//!   over the picture ([`COLUMNS`] by [`ROWS`]), those whose line of sight
//!   meets the ground of a tile drawn (its 10 m square at its height). The
//!   tiles alone: Fade Geometry leaves out roofs, not the ground, and the
//!   view's object switches hide objects, never tiles. A tile whose model
//!   is missing or hidden by a brush's preview isn't in the frame, and so
//!   isn't expected; one whose model was read and failed (as it was built
//!   or posed) is: on a renderer that fails every model they are all there
//!   is ([`Observation::failed`]). A tile whose pipeline of the game's
//!   programs failed is drawn by Moonglow's own shader, and expected as
//!   any. The sky, an empty area, or an area seen from far off expects
//!   little.
//! - **The measurement** ([`Sampler`]): the depth drawn at the same points
//!   (the sky writes none), and the picture's colour there, copied into a
//!   small texture by one draw and read back a frame or more later
//!   without waiting for it, paired with the expectation of the frame it
//!   was taken from (a zoom since doesn't matter). A look the GPU doesn't
//!   give back is given up after [`LOST`] seconds. (The colour is for the
//!   debug log, [`Look::colored`]: a dark area's tiles may be drawn black.)
//! - **The verdict** ([`Watch::judge`]): a failure only when the
//!   pipelines and textures are ready (the game's programs are made on a
//!   worker and appear late) and have been for a while, or have been
//!   waited for [`PATIENCE`] seconds with none of them finished (what they
//!   wait for doesn't come); the expectation is above a floor; and fewer
//!   than a fifth of the expected points have been drawn at every look for
//!   several seconds.
//!
//! The start location's ring and the doors' arrows are not in the
//! picture: egui draws them over it. They are neither expected nor
//! measured (Heed's view showed them, and nothing else).

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, channel};

use glam::{Vec2, Vec3};

use crate::{Camera, Gpu};

/// The grid of points looked at: columns and rows over the picture.
pub const COLUMNS: u32 = 64;
pub const ROWS: u32 = 36;

/// A tile's side, metres.
const TILE: f32 = 10.0;

/// How often the view is looked at, seconds.
pub const EVERY: f64 = 1.0;

/// The points of the grid expected drawn: row by row, top first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected(pub Vec<bool>);

impl Expected {
    pub fn count(&self) -> usize {
        self.0.iter().filter(|&&e| e).count()
    }
}

/// Of the grid's points (row by row from the top, [`COLUMNS`] by
/// [`ROWS`]), those whose line of sight through the picture (`camera`'s,
/// `aspect` wide to its height) meets the ground of a tile drawn: each of
/// `tiles` (its middle: a 10 m square, level at that height). Only as far
/// as the camera sees (its far plane).
pub fn expected(tiles: &[Vec3], camera: &Camera, aspect: f32) -> Expected {
    let cell = |p: Vec3| ((p.x / TILE).floor() as i32, (p.y / TILE).floor() as i32);
    let mut drawn: HashSet<(i32, i32, u32)> = HashSet::new();
    let mut levels: Vec<f32> = Vec::new();
    for t in tiles {
        let (x, y) = cell(*t);
        drawn.insert((x, y, t.z.to_bits()));
        if !levels.iter().any(|l| l.to_bits() == t.z.to_bits()) {
            levels.push(t.z);
        }
    }
    let inverse = (camera.projection(aspect) * camera.view()).inverse();
    let reach = camera.far * 0.98;
    let mut out = Vec::with_capacity((COLUMNS * ROWS) as usize);
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let at =
                Vec2::new((column as f32 + 0.5) / COLUMNS as f32, (row as f32 + 0.5) / ROWS as f32);
            let ndc = Vec2::new(at.x * 2.0 - 1.0, 1.0 - at.y * 2.0);
            let near = inverse.project_point3(ndc.extend(0.0));
            let far = inverse.project_point3(ndc.extend(1.0));
            let dir = (far - near).normalize_or_zero();
            // (Down onto the ground, from above it: the ground's
            // underside is not drawn.)
            let seen = dir.z < -1e-6
                && levels.iter().any(|&z| {
                    let t = (z - near.z) / dir.z;
                    if t <= 0.0 || (near + dir * t - camera.eye).length() > reach {
                        return false;
                    }
                    let (x, y) = cell(near + dir * t);
                    drawn.contains(&(x, y, z.to_bits()))
                });
            out.push(seen);
        }
    }
    Expected(out)
}

/// A look at the view, for [`Watch::judge`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Observation {
    /// The grid's points expected drawn, and of those, the points drawn.
    pub expected: usize,
    pub drawn: usize,
    /// The frame's pipelines and textures were ready: no mesh waited for a
    /// pipeline of the game's programs, nor for its textures.
    pub ready: bool,
    /// How much of what is waited for had been finished by then (textures
    /// read and pipelines made, counted from the start: any number that
    /// grows as they are): a frame not ready whose count stays put for
    /// [`PATIENCE`] is waiting for something that doesn't come.
    pub progress: u64,
    /// Of the points expected, those where a tile was left out of the
    /// frame as it failed (its model couldn't be built or posed): expected
    /// all the same, as on a renderer that fails every tile they are all
    /// there is.
    pub failed: usize,
    /// When (seconds, of any clock that only goes forward).
    pub at: f64,
}

/// Why a look found nothing wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quiet {
    /// Pipelines or textures still being made or read (for less than
    /// [`PATIENCE`] since the last was finished).
    NotReady,
    /// Ready, but not yet for [`WARM_UP`] seconds.
    WarmingUp,
    /// Too little expected to tell (an empty area, the sky, far off).
    BelowFloor,
    /// Drawn as expected (or nearly).
    Drawn,
}

/// What [`Watch::judge`] makes of a look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    Quiet(Quiet),
    /// Too little drawn, since `since`: not yet for long enough.
    Doubt {
        since: f64,
    },
    /// Too little drawn for long enough: the renderer isn't drawing.
    Failed(Failure),
}

/// A failure of the watch: what was expected and drawn at the look that
/// decided it, and for how long it had been so.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Failure {
    pub expected: usize,
    pub drawn: usize,
    pub seconds: f64,
    /// Of the points expected, those of tiles left out as they failed
    /// ([`Observation::failed`]).
    pub failed: usize,
    /// Judged while textures or pipelines were still waited for: for how
    /// long nothing of them had been finished, seconds.
    pub waiting: Option<f64>,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} of {} points of the area's ground drawn, for {:.0} seconds",
            self.drawn, self.expected, self.seconds
        )?;
        if self.failed > 0 {
            write!(f, " ({} of them of tiles that failed and were left out)", self.failed)?;
        }
        if let Some(w) = self.waiting {
            write!(
                f,
                ", with textures or the game's programs waited for and none finished for {w:.0} \
                 seconds"
            )?;
        }
        Ok(())
    }
}

/// The least share of the grid expected for a look to count.
pub const FLOOR: f32 = 0.05;
/// Below this share of the expected points drawn, a look is a collapse.
pub const DRAWN: f32 = 0.2;
/// How long a collapse lasts, at the least, before it is a failure, and
/// how many looks it takes.
pub const SUSTAINED: f64 = 5.0;
pub const LOOKS: u32 = 4;
/// How long the pipelines must have been ready before a look counts.
pub const WARM_UP: f64 = 2.0;
/// How long a frame may wait for textures or the game's programs with
/// none of them finished before it is judged all the same, seconds. On the
/// development machine all of an area's textures are read in 1.4 s at the
/// most (The Domains of Dread's woods; area092, 784 tiles, in 0.18 s:
/// `docs/research/notes_frames.md`) and its programs made in under 0.3 s
/// with no shader cache; each of them finishes in far less. A slow
/// machine, or Direct3D 12's FXC making a program, takes several times
/// that, and a program waited for leaves nothing out (Moonglow's own
/// shader draws its mesh meanwhile): a texture is what leaves a tile out
/// until it is read. Twenty seconds with nothing finished is something
/// that doesn't come (Heed's Direct3D 12 drew nothing, and its frames may
/// never have been ready), and no wait of the frames this machine draws
/// comes near it.
pub const PATIENCE: f64 = 20.0;

/// The watch's state across looks: when the pipelines became ready, when
/// a frame not ready last saw something finished, when a collapse began
/// and how many looks it has lasted, and the failure once there is one
/// (said once: it stays).
#[derive(Debug, Clone, Default)]
pub struct Watch {
    ready_since: Option<f64>,
    /// Waiting: since when, and the progress then.
    waiting: Option<(f64, u64)>,
    collapse: Option<(f64, u32)>,
    failed: Option<Failure>,
}

impl Watch {
    /// The verdict of a look (a pure function of the looks so far).
    pub fn judge(&mut self, o: Observation) -> Verdict {
        if let Some(f) = self.failed {
            return Verdict::Failed(f);
        }
        let quiet = |w: &mut Watch, why| {
            w.collapse = None;
            Verdict::Quiet(why)
        };
        // Not ready: nothing counts, unless nothing it waits for has been
        // finished for long enough (then it is judged as it is: what it
        // waits for doesn't come).
        let mut waiting = None;
        if o.ready {
            self.waiting = None;
            let since = *self.ready_since.get_or_insert(o.at);
            if o.at - since < WARM_UP {
                return quiet(self, Quiet::WarmingUp);
            }
        } else {
            self.ready_since = None;
            let (since, progress) = *self.waiting.get_or_insert((o.at, o.progress));
            let since = if progress == o.progress {
                since
            } else {
                self.waiting = Some((o.at, o.progress));
                o.at
            };
            if o.at - since < PATIENCE {
                return quiet(self, Quiet::NotReady);
            }
            waiting = Some(o.at - since);
        }
        let grid = (COLUMNS * ROWS) as f32;
        if (o.expected as f32) < FLOOR * grid {
            return quiet(self, Quiet::BelowFloor);
        }
        if o.drawn as f32 >= DRAWN * o.expected as f32 {
            return quiet(self, Quiet::Drawn);
        }
        let (began, looks) = self.collapse.get_or_insert((o.at, 0));
        *looks += 1;
        let (began, looks) = (*began, *looks);
        if o.at - began >= SUSTAINED && looks >= LOOKS {
            let f = Failure {
                expected: o.expected,
                drawn: o.drawn,
                seconds: o.at - began,
                failed: o.failed,
                waiting,
            };
            self.failed = Some(f);
            return Verdict::Failed(f);
        }
        Verdict::Doubt { since: began }
    }

    /// The failure, once there is one.
    pub fn failure(&self) -> Option<Failure> {
        self.failed
    }

    /// Forgets the looks (another area, the switch of the game's shaders):
    /// the next look starts again, warming up. A failure stays.
    pub fn restart(&mut self) {
        self.ready_since = None;
        self.waiting = None;
        self.collapse = None;
    }
}

/// The look's format: the depth, and the picture's colour (as drawn,
/// resolved) at each point.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// How long a look may wait for the GPU before it is given up, seconds.
pub const LOST: f64 = 5.0;

fn source(samples: u32) -> String {
    let depth = if samples > 1 { "texture_depth_multisampled_2d" } else { "texture_depth_2d" };
    format!(
        "@group(0) @binding(0) var depth: {depth};
@group(0) @binding(1) var color: texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}}
@fragment fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {{
    let size = vec2<f32>(textureDimensions(depth));
    let at = vec2<i32>(p.xy / vec2<f32>({COLUMNS}.0, {ROWS}.0) * size);
    return vec4<f32>(textureLoad(depth, at, 0), textureLoad(color, at, 0).rgb);
}}
"
    )
}

/// A look under way: its buffer, its mapping's answer, what was expected
/// of its frame, and when it was taken.
#[derive(Debug)]
struct Pending {
    buffer: wgpu::Buffer,
    mapped: Receiver<Result<(), wgpu::BufferAsyncError>>,
    expected: Expected,
    at: f64,
}

/// A look the GPU has done ([`Sampler::take`]): the expectation of its
/// frame, and at each point of the grid (row by row), whether something
/// was drawn there (a depth nearer than the far plane's) and the
/// picture's colour (0 to 1, as the target holds it).
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub expected: Expected,
    pub drawn: Vec<bool>,
    pub colors: Vec<[f32; 3]>,
}

impl Look {
    /// The points expected, and of those the points drawn.
    pub fn counted(&self) -> (usize, usize) {
        counted(&self.expected, &self.drawn)
    }

    /// Of the points expected and drawn, those whose colour isn't the
    /// frame's `background` (what its picture was cleared to, 0 to 1):
    /// a renderer that lays the depth of what it draws and none of its
    /// colour (or only the background's) is seen drawing nothing, though
    /// the depth says it drew. (For the debug log: a dark area's tiles may
    /// be drawn black on a black background.)
    pub fn colored(&self, background: [f32; 3]) -> usize {
        let apart =
            |c: &[f32; 3]| c.iter().zip(background).any(|(a, b)| (a - b).abs() > 2.5 / 255.0);
        self.expected
            .0
            .iter()
            .zip(&self.drawn)
            .zip(&self.colors)
            .filter(|((e, d), c)| **e && **d && apart(c))
            .count()
    }
}

/// What looks at a target: one draw into a small texture (a texel for
/// each point of the grid: the depth and the colour there), copied into a
/// buffer read when the GPU is done, a look at a time.
#[derive(Debug)]
pub struct Sampler {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    grid: wgpu::TextureView,
    texture: wgpu::Texture,
    samples: u32,
    pending: Option<Pending>,
}

impl Sampler {
    /// For targets of `samples` samples a pixel.
    pub fn new(gpu: &Gpu, samples: u32) -> Sampler {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("watch"),
            source: wgpu::ShaderSource::Wgsl(source(samples).into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("watch"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: samples > 1,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("watch"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("watch"),
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
                targets: &[Some(FORMAT.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("watch"),
            size: wgpu::Extent3d { width: COLUMNS, height: ROWS, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let grid = texture.create_view(&Default::default());
        Sampler { pipeline, layout, grid, texture, samples, pending: None }
    }

    /// The samples a pixel of the targets it is for.
    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// Whether a look is under way (the next is taken once it is read).
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Takes a look at `targets` as just drawn (their depth, and the
    /// colour of their picture), what `expected` says of its frame kept
    /// with it, at `now` (seconds). Nothing while a look is under way.
    pub fn look(&mut self, gpu: &Gpu, targets: &crate::Targets, expected: Expected, now: f64) {
        if self.pending.is_some() {
            return;
        }
        let device = &gpu.device;
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("watch"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&targets.depth),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.color_view),
                },
            ],
        });
        // (A row of 64 sixteen-byte texels is 1024 bytes: of the copy's
        // alignment.)
        let row = COLUMNS * 16;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("watch"),
            size: u64::from(row * ROWS),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("watch"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.grid,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d { width: COLUMNS, height: ROWS, depth_or_array_layers: 1 },
        );
        gpu.queue.submit([encoder.finish()]);
        let (tx, rx) = channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| drop(tx.send(r)));
        self.pending = Some(Pending { buffer, mapped: rx, expected, at: now });
    }

    /// The look under way, once the GPU has done it (never waiting).
    pub fn take(&mut self, gpu: &Gpu) -> Option<Look> {
        let pending = self.pending.as_ref()?;
        let _ = gpu.device.poll(wgpu::PollType::Poll);
        match pending.mapped.try_recv() {
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) | Ok(Err(_)) => {
                // (Lost: the next look is another.)
                self.pending = None;
                return None;
            }
            Ok(Ok(())) => {}
        }
        let pending = self.pending.take()?;
        let texels: Vec<[f32; 4]> = {
            let data = pending.buffer.slice(..).get_mapped_range().ok()?;
            data.as_chunks::<16>()
                .0
                .iter()
                .map(|t| {
                    std::array::from_fn(|i| {
                        f32::from_le_bytes([t[4 * i], t[4 * i + 1], t[4 * i + 2], t[4 * i + 3]])
                    })
                })
                .collect()
        };
        pending.buffer.unmap();
        Some(Look {
            expected: pending.expected,
            drawn: texels.iter().map(|t| t[0] < 1.0).collect(),
            colors: texels.iter().map(|t| [t[1], t[2], t[3]]).collect(),
        })
    }

    /// Gives up the look under way if the GPU hasn't done it in [`LOST`]
    /// seconds since it was taken (at `now`): whether it did. (A look that
    /// never came back would keep the next from being taken: the watch
    /// would look no more.)
    pub fn give_up(&mut self, now: f64) -> bool {
        if self.pending.as_ref().is_some_and(|p| now - p.at >= LOST) {
            self.pending = None;
            return true;
        }
        false
    }
}

/// Of the points `expected`, how many `drawn` has: an [`Observation`]'s
/// numbers.
pub fn counted(expected: &Expected, drawn: &[bool]) -> (usize, usize) {
    let of = expected.0.iter().zip(drawn).filter(|&(&e, &d)| e && d).count();
    (expected.count(), of)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRID: usize = (COLUMNS * ROWS) as usize;

    fn look(expected: usize, drawn: usize, at: f64) -> Observation {
        Observation { expected, drawn, ready: true, at, ..Default::default() }
    }

    /// Looks every second from `from` to `to`, all the same.
    fn looks(w: &mut Watch, o: Observation, from: f64, to: f64) -> Verdict {
        let mut v = Verdict::Quiet(Quiet::Drawn);
        let mut at = from;
        while at <= to {
            v = w.judge(Observation { at, ..o });
            at += EVERY;
        }
        v
    }

    /// Drawing nothing of what is expected, for long enough, is a
    /// failure, said once and kept.
    #[test]
    fn a_sustained_collapse_fails() {
        let mut w = Watch::default();
        assert_eq!(looks(&mut w, look(1000, 990, 0.0), 0.0, 4.0), Verdict::Quiet(Quiet::Drawn));
        assert_eq!(w.judge(look(1000, 0, 5.0)), Verdict::Doubt { since: 5.0 });
        let v = looks(&mut w, look(1000, 0, 0.0), 6.0, 10.0);
        let f = Failure { expected: 1000, drawn: 0, seconds: 5.0, failed: 0, waiting: None };
        assert_eq!(v, Verdict::Failed(f));
        assert_eq!(w.judge(look(1000, 1000, 11.0)), Verdict::Failed(f), "kept");
        assert_eq!(f.to_string(), "0 of 1000 points of the area's ground drawn, for 5 seconds");
        // (Fewer than a fifth drawn: tiles vanishing one by one.)
        let mut w = Watch::default();
        assert!(matches!(looks(&mut w, look(1000, 150, 0.0), 0.0, 10.0), Verdict::Failed(_)));
    }

    /// Guard: the pipelines ready. While the game's programs are made
    /// (or textures read), one after another, nothing counts, however
    /// long, and once ready, not for the first seconds.
    #[test]
    fn nothing_counts_before_the_pipelines_are_ready_and_warm() {
        let mut w = Watch::default();
        for second in 0..=30 {
            // (A texture or a program finished every few seconds.)
            let waiting = Observation {
                ready: false,
                progress: second / 5,
                ..look(1000, 0, f64::from(second as u32))
            };
            assert_eq!(w.judge(waiting), Verdict::Quiet(Quiet::NotReady), "second {second}");
        }
        assert_eq!(w.judge(look(1000, 0, 31.0)), Verdict::Quiet(Quiet::WarmingUp));
        assert_eq!(w.judge(look(1000, 0, 32.0)), Verdict::Quiet(Quiet::WarmingUp));
        assert_eq!(w.judge(look(1000, 0, 33.0)), Verdict::Doubt { since: 33.0 });
        // Not ready again (the Shaders switch turned on): warming up again.
        let again = Observation { ready: false, ..look(1000, 0, 34.0) };
        assert_eq!(w.judge(again), Verdict::Quiet(Quiet::NotReady));
        assert_eq!(w.judge(look(1000, 0, 35.0)), Verdict::Quiet(Quiet::WarmingUp));
        assert!(matches!(looks(&mut w, look(1000, 0, 0.0), 36.0, 50.0), Verdict::Failed(_)));
    }

    /// A frame that stays not ready with nothing it waits for finished
    /// (a texture never read, a program never made: Heed's Direct3D 12
    /// may never have been ready) is judged after [`PATIENCE`]: drawn,
    /// quiet; drawing nothing, a failure that says it was waiting. A wait
    /// that finishes something now and then starts again each time.
    #[test]
    fn a_frame_that_never_gets_ready_is_judged_after_a_while() {
        let stuck = |at: f64| Observation { ready: false, progress: 7, ..look(1000, 0, at) };
        let mut w = Watch::default();
        assert_eq!(looks(&mut w, stuck(0.0), 0.0, PATIENCE - 1.0), Verdict::Quiet(Quiet::NotReady));
        assert_eq!(w.judge(stuck(PATIENCE)), Verdict::Doubt { since: PATIENCE });
        let v = looks(&mut w, stuck(0.0), PATIENCE + 1.0, PATIENCE + 10.0);
        let Verdict::Failed(f) = v else { panic!("{v:?}") };
        assert_eq!(f.waiting, Some(PATIENCE + SUSTAINED));
        assert!(f.to_string().ends_with("none finished for 25 seconds"), "{f}");
        // Drawn, not ready all the while: quiet.
        let mut w = Watch::default();
        let drawn = |at: f64| Observation { drawn: 900, ..stuck(at) };
        assert_eq!(looks(&mut w, drawn(0.0), 0.0, 60.0), Verdict::Quiet(Quiet::Drawn));
        // Something finished every 15 seconds: the wait starts again.
        let mut w = Watch::default();
        for second in 0..90u32 {
            let o = Observation { progress: u64::from(second / 15), ..stuck(f64::from(second)) };
            assert_eq!(w.judge(o), Verdict::Quiet(Quiet::NotReady), "second {second}");
        }
        // Ready a moment between waits (a frame ready, then a texture met):
        // each wait starts again, and a stuck one is judged in its turn.
        let mut w = Watch::default();
        assert_eq!(looks(&mut w, stuck(0.0), 0.0, 15.0), Verdict::Quiet(Quiet::NotReady));
        assert_eq!(w.judge(look(1000, 0, 16.0)), Verdict::Quiet(Quiet::WarmingUp));
        assert_eq!(looks(&mut w, stuck(0.0), 17.0, 36.0), Verdict::Quiet(Quiet::NotReady));
        assert_eq!(w.judge(stuck(37.0)), Verdict::Doubt { since: 37.0 });
    }

    /// Tiles left out as they failed are expected: a failure counts them,
    /// and says so.
    #[test]
    fn a_failure_names_the_tiles_that_failed() {
        let mut w = Watch::default();
        let o = |at| Observation { failed: 1000, ..look(1000, 0, at) };
        let Verdict::Failed(f) = looks(&mut w, o(0.0), 0.0, 10.0) else { panic!() };
        assert_eq!(f.failed, 1000);
        assert_eq!(
            f.to_string(),
            "0 of 1000 points of the area's ground drawn, for 5 seconds (1000 of them of tiles \
             that failed and were left out)"
        );
    }

    /// Guard: the expectation above a floor. An empty area, the sky, or an
    /// area far off expects too little to tell, however little is drawn.
    #[test]
    fn too_little_expected_never_fails() {
        let mut w = Watch::default();
        let floor = (FLOOR * GRID as f32) as usize;
        let v = looks(&mut w, look(floor - 1, 0, 0.0), 0.0, 60.0);
        assert_eq!(v, Verdict::Quiet(Quiet::BelowFloor));
        assert_eq!(looks(&mut w, look(0, 0, 0.0), 61.0, 90.0), Verdict::Quiet(Quiet::BelowFloor));
    }

    /// Guard: the collapse sustained. Looks that collapse and recover
    /// (zooming through, a frame drawn while a model loads) start again
    /// each time; nor do fewer looks than [`LOOKS`] decide, however long.
    #[test]
    fn a_collapse_that_recovers_or_is_not_seen_enough_is_not_a_failure() {
        let mut w = Watch::default();
        w.judge(look(1000, 1000, 0.0));
        for second in 3..60 {
            let drawn = if second % 4 == 0 { 900 } else { 0 };
            let v = w.judge(look(1000, drawn, f64::from(second)));
            assert!(!matches!(v, Verdict::Failed(_)), "second {second}: {v:?}");
        }
        // Two looks ten seconds apart: not enough looks.
        let mut w = Watch::default();
        w.judge(look(1000, 1000, 0.0));
        w.judge(look(1000, 1000, 3.0));
        assert_eq!(w.judge(look(1000, 0, 4.0)), Verdict::Doubt { since: 4.0 });
        assert_eq!(w.judge(look(1000, 0, 14.0)), Verdict::Doubt { since: 4.0 });
        // A look below the floor between collapses (the camera to the sky
        // and back) starts it again too.
        let mut w = Watch::default();
        looks(&mut w, look(1000, 1000, 0.0), 0.0, 3.0);
        looks(&mut w, look(1000, 0, 0.0), 4.0, 7.0);
        assert_eq!(w.judge(look(10, 0, 8.0)), Verdict::Quiet(Quiet::BelowFloor));
        assert_eq!(w.judge(look(1000, 0, 9.0)), Verdict::Doubt { since: 9.0 });
    }

    /// Guard: what is drawn short of the expectation, but more than a
    /// fifth of it, is drawn (Fade Geometry's holes, hidden objects never
    /// expected, a tile with a hole in its ground).
    #[test]
    fn partly_drawn_is_drawn() {
        let mut w = Watch::default();
        let v = looks(&mut w, look(1000, 200, 0.0), 0.0, 60.0);
        assert_eq!(v, Verdict::Quiet(Quiet::Drawn));
        w.restart();
        assert_eq!(w.judge(look(1000, 1000, 61.0)), Verdict::Quiet(Quiet::WarmingUp));
    }

    fn camera(eye: Vec3, target: Vec3) -> Camera {
        Camera { eye, target, fov_y: 40f32.to_radians(), near: 0.1, far: 500.0 }
    }

    /// A 4 by 4 area's tiles, at height 0.
    fn area(side: i32) -> Vec<Vec3> {
        (0..side * side)
            .map(|i| {
                Vec3::new(((i % side) as f32 + 0.5) * TILE, ((i / side) as f32 + 0.5) * TILE, 0.0)
            })
            .collect()
    }

    /// The expectation: an area seen from above fills the picture; turned
    /// up to the sky, the top expects nothing; far off, little; an empty
    /// area, nothing; a tile at another height is met at its own.
    #[test]
    fn the_expectation_is_the_ground_seen() {
        let tiles = area(4);
        let middle = Vec3::new(20.0, 20.0, 0.0);
        let above = expected(&tiles, &camera(middle + Vec3::new(0.0, -0.01, 30.0), middle), 1.6);
        assert_eq!(above.count(), GRID, "from 30 m up, the 40 m area fills the view");
        let low = camera(middle + Vec3::new(0.0, -30.0, 2.0), middle + Vec3::new(0.0, 0.0, 6.0));
        let sky = expected(&tiles, &low, 1.6);
        let top_row = &sky.0[..COLUMNS as usize];
        assert!(top_row.iter().all(|&e| !e), "the sky expects nothing");
        assert!(sky.count() > 0 && sky.count() < GRID / 2, "{}", sky.count());
        let far = expected(&tiles, &camera(middle + Vec3::new(0.0, -300.0, 300.0), middle), 1.6);
        assert!((far.count() as f32) < FLOOR * GRID as f32, "{}", far.count());
        assert_eq!(expected(&[], &camera(Vec3::new(0.0, -10.0, 10.0), Vec3::ZERO), 1.6).count(), 0);
        // One tile raised 5 m, seen from above: met at its height.
        let raised = [Vec3::new(5.0, 5.0, 5.0)];
        let over = camera(Vec3::new(5.0, 4.99, 20.0), Vec3::new(5.0, 5.0, 5.0));
        assert!(expected(&raised, &over, 1.0).0[(ROWS / 2 * COLUMNS + COLUMNS / 2) as usize]);
        // Under the ground, looking up: its underside isn't drawn, nor
        // expected.
        let under = camera(Vec3::new(20.0, -10.0, -5.0), middle + Vec3::new(0.0, 0.0, 10.0));
        assert_eq!(expected(&tiles, &under, 1.6).count(), 0);
    }

    #[test]
    fn counted_is_the_expected_points_drawn() {
        let e = Expected(vec![true, true, false, true]);
        assert_eq!(counted(&e, &[true, false, true, true]), (3, 2));
    }
}
