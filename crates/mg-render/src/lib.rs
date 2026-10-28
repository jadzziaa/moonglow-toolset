//! The renderer: models and textures on the GPU, lit the way the game's
//! "enhanced lighting" shaders light them (`docs/research/notes_shaders.md`),
//! drawn into a window's frame or offscreen.

pub mod anim;
pub mod assets;
pub mod backend;
mod batch;
pub mod dangly;
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
pub use gpu::{Gpu, headless_adapter};
pub use model::{GpuModel, rest_pose};
pub use reach::Frustum;
pub use renderer::{
    DEPTH_FORMAT, DebugView, Drawn, GAME_WATER, GameProgram, MAX_LIGHTS, Renderer, Targets,
    attenuation_params, tint_light,
};
pub use scene::{
    AreaLight, Camera, Environment, Fog, Instance, Line, MeshOverride, MeshState, ModuleClock,
    PointLight, Scene, SunShadows,
};
