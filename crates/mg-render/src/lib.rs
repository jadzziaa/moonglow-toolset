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
pub mod probe;
mod reach;
mod renderer;
pub mod scene;
mod shadow;
pub mod texture;
pub mod watch;

pub use assets::{Assets, LoadedTexture, NoAssets, colored_name, split_colors};
pub use game::ShaderStats;
#[doc(hidden)]
pub use game::{Captured, CapturedPipeline};
pub use gpu::{Gpu, headless_adapter};
pub use model::{GpuModel, rest_pose};
pub use reach::{Frustum, box_corners};
pub use renderer::{
    DEPTH_FORMAT, Drawn, GAME_WATER, GameProgram, MAX_LIGHTS, Renderer, Targets,
    attenuation_params, tint_light,
};
/// Moonglow's own shaders, each as wgpu is given it (a name, and its
/// WGSL), at the samples a pixel the renderer draws with (1, and the
/// area view's 4): what a backend makes of them (Direct3D 12's HLSL) can
/// be tried on a machine without that backend.
#[doc(hidden)]
pub fn wgsl_sources() -> Vec<(String, String)> {
    let mut out = vec![
        ("shader".to_string(), include_str!("shader.wgsl").to_string()),
        ("particle".into(), include_str!("particle.wgsl").into()),
        ("line".into(), include_str!("line.wgsl").into()),
        ("shadow_cast".into(), shadow::CAST.into()),
    ];
    for samples in [1, 4] {
        out.push((format!("framebuffer_{samples}"), framebuffer::source(samples)));
        out.push((format!("watch_{samples}"), watch::source(samples)));
        for lights in [false, true] {
            let name = format!("shadow_darken_{samples}{}", if lights { "_lights" } else { "" });
            out.push((name, shadow::darken_source(samples, lights)));
        }
    }
    out
}

pub use scene::{
    AreaLight, Camera, Environment, Fog, Instance, Line, LineCopy, LineShape, MeshOverride,
    MeshState, ModuleClock, PointLight, Scene, SunShadows,
};
