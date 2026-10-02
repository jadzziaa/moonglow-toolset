//! The renderer: models and textures on the GPU, lit the way the game's
//! "enhanced lighting" shaders light them (`docs/research/notes_shaders.md`),
//! drawn into a window's frame or offscreen.

pub mod anim;
pub mod assets;
pub mod dangly;
mod gpu;
pub mod model;
pub mod particles;
mod renderer;
pub mod scene;
pub mod texture;

pub use assets::{Assets, LoadedTexture, NoAssets, colored_name, split_colors};
pub use gpu::Gpu;
pub use model::{GpuModel, rest_pose};
pub use renderer::{
    DEPTH_FORMAT, DebugView, MAX_LIGHTS, Renderer, Targets, attenuation_params, tint_light,
};
pub use scene::{
    AreaLight, Camera, Fog, Instance, Line, MeshOverride, MeshState, PointLight, Scene,
};
