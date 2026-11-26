//! The renderer: models and textures on the GPU, lit the way the game's
//! "enhanced lighting" shaders light them (`docs/research/notes_shaders.md`),
//! drawn into a window's frame or offscreen.

pub mod anim;
pub mod assets;
pub mod backend;
mod batch;
pub mod dangly;
pub mod dx12;
pub mod framebuffer;
mod game;
mod gpu;
pub mod guard;
pub mod model;
pub mod particles;
mod pool;
pub mod present;
pub mod probe;
mod reach;
mod renderer;
pub mod scene;
mod shadow;
pub mod texture;
mod views;
mod volume;
pub mod watch;

pub use assets::{Assets, LoadedTexture, NoAssets, Palettes, colored_name, split_colors};
pub use game::ShaderStats;
/// (For the corpus tests: the game's programs split as the renderer
/// splits them.)
#[doc(hidden)]
pub use game::part as uniform_part;
#[doc(hidden)]
pub use game::{Captured, CapturedPipeline};
pub use gpu::{Gpu, OPTIONAL_FEATURES, SAMPLE_COUNTS, headless_adapter};
pub use model::{GpuModel, rest_pose};
pub use pool::MeshMemory;
pub use reach::{Frustum, box_corners, box_of};
#[doc(hidden)]
pub use renderer::ViewKept;
pub use renderer::{
    DEPTH_FORMAT, Drawn, GAME_WATER, GRID_LEAST_IN_FOG, GameProgram, MAX_LIGHTS, Renderer,
    STALE_TEXTURES_KEPT, STALE_TEXTURES_KEPT_INTEGRATED, Targets, TextureMemory,
    attenuation_params, tint_light,
};
/// Moonglow's own shaders, each as wgpu is given it (a name, and its
/// WGSL), at the samples a pixel the renderer draws with (1, and the
/// area view's 4): what a backend makes of them (Direct3D 12's HLSL) can
/// be tried on a machine without that backend.
#[doc(hidden)]
pub fn wgsl_sources() -> Vec<(String, String)> {
    let mut out = vec![
        ("shader".to_string(), renderer::LIT_WGSL.to_string()),
        ("particle".into(), renderer::PARTICLE_WGSL.into()),
        ("line".into(), renderer::LINE_WGSL.into()),
        ("shadow_cast".into(), shadow::CAST.into()),
        ("shadow_volume".into(), volume::VOLUME.into()),
        ("present".into(), present::draw_source(wgpu::TextureFormat::Bgra8Unorm)),
        ("present_srgb".into(), present::draw_source(wgpu::TextureFormat::Bgra8UnormSrgb)),
    ];
    for samples in [1, 4] {
        out.push((format!("framebuffer_{samples}"), framebuffer::source(samples)));
        out.push((format!("watch_{samples}"), watch::source(samples)));
        out.push((format!("shadow_volume_depth_{samples}"), volume::copy_source(samples)));
        for lights in [false, true] {
            for stencil in [false, true] {
                let name = format!(
                    "shadow_darken_{samples}{}{}",
                    if lights { "_lights" } else { "" },
                    if stencil { "_stencil" } else { "" }
                );
                out.push((name, shadow::darken_source(samples, lights, stencil)));
            }
        }
    }
    out
}

/// What an object sets on its materials ([`Instance::material_values`]).
pub use mg_image::mtr::MaterialValue;
pub use scene::{
    AreaLight, Camera, Environment, Fog, Instance, Line, LineCopy, LineShape, MeshOverride,
    MeshState, ModuleClock, PointLight, Scene, SunShadows,
};
