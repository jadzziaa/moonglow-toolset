//! The renderer: models and textures on the GPU, lit the way the game's
//! "enhanced lighting" shaders light them (`docs/research/notes_shaders.md`),
//! drawn into a window's frame or offscreen.

pub mod anim;
pub mod assets;
mod gpu;
pub mod model;
mod renderer;
pub mod scene;
pub mod texture;

pub use assets::{Assets, LoadedTexture, NoAssets};
pub use gpu::Gpu;
pub use model::{GpuModel, rest_pose};
pub use renderer::{MAX_LIGHTS, Renderer, Targets};
pub use scene::{AreaLight, Camera, Fog, Instance, PointLight, Scene};
