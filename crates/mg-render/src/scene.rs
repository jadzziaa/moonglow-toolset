//! What to draw: model instances, lights, the area light and fog, and the
//! camera looking at them. Z is up, as in the game.

use std::sync::Arc;

use glam::{Mat4, Vec3};

use crate::model::GpuModel;

/// A placed model.
#[derive(Debug, Clone)]
pub struct Instance {
    pub model: Arc<GpuModel>,
    pub transform: Mat4,
    /// Model-space node transforms (a pose); `None` for the rest pose.
    pub pose: Option<Arc<Vec<Mat4>>>,
    /// What animations and dangly meshes change in the meshes; `None` for
    /// the model's own values.
    pub state: Option<Arc<MeshState>>,
    /// The object's environment map (appearance.2da `ENVMAP`, resolved):
    /// meshes whose texture names none reflect it, their texture alpha
    /// being the reflectivity. `None`: only what textures ask for.
    pub env_map: Option<String>,
    /// Colours for PLT textures by layer (skin, hair, metal 1 and 2, cloth
    /// 1 and 2, leather 1 and 2, tattoo 1 and 2); `None`: colour 0.
    pub plt_colors: Option<[u8; 10]>,
    /// Texture names to use instead of the model's (lower case); under the
    /// empty name, the texture of meshes that name none.
    pub textures: Option<Arc<std::collections::HashMap<String, String>>>,
    /// How opaque the whole instance is (each mesh's alpha times this):
    /// below 1, see-through, as a blueprint about to be placed.
    pub opacity: f32,
    /// Drawn in its own colours (its texture's and its material's),
    /// whatever the scene's light: an editor's marker (a waypoint's flag),
    /// which must tell apart in a dark area.
    pub unlit: bool,
}

impl Instance {
    /// An instance in its rest pose.
    pub fn new(model: Arc<GpuModel>, transform: Mat4) -> Instance {
        Instance {
            model,
            transform,
            pose: None,
            state: None,
            env_map: None,
            plt_colors: None,
            textures: None,
            opacity: 1.0,
            unlit: false,
        }
    }
}

/// Per mesh of a [`GpuModel`] (same order): values that replace the
/// model's own at a moment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshState {
    pub meshes: Vec<MeshOverride>,
}

/// A mesh's replaced values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshOverride {
    pub alpha: Option<f32>,
    pub selfillum: Option<Vec3>,
    /// Vertices (animated or dangly meshes), in the mesh's own order.
    pub vertices: Option<Vec<crate::model::Vertex>>,
}

impl MeshState {
    /// Nothing replaced, for `model`.
    pub fn new(model: &GpuModel) -> MeshState {
        MeshState { meshes: vec![MeshOverride::default(); model.meshes.len()] }
    }
}

/// A point light, as the game uploads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub position: Vec3,
    /// Linear colour, at most 1 per channel.
    pub color: Vec3,
    /// The distance where the light ends, metres.
    pub cutoff: f32,
    pub ambient_only: bool,
    /// 1 (highest) to 5: which lights are kept past 32.
    pub priority: u32,
}

/// The game's `cutoff-range-multiplier` setting.
pub const CUTOFF_RANGE_MULTIPLIER: f32 = 2.0;

impl PointLight {
    /// A light from a model's (or lightcolor.2da's) colour and radius, the
    /// way the game converts them (checked against its uniforms): a colour
    /// brighter than 1 is scaled down to a largest channel of 1 and reaches
    /// that much further instead; the colour is then linearised.
    pub fn new(
        position: Vec3,
        color: Vec3,
        radius: f32,
        ambient_only: bool,
        priority: u32,
    ) -> Self {
        let intensity = color.max_element().max(1.0);
        PointLight {
            position,
            color: (color / intensity).max(Vec3::ZERO).powf(2.2),
            cutoff: radius * CUTOFF_RANGE_MULTIPLIER * intensity,
            ambient_only,
            priority,
        }
    }
}

/// The sun or moon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaLight {
    /// Linear colours.
    pub ambient: Vec3,
    pub diffuse: Vec3,
    /// World-space direction towards the light.
    pub direction: Vec3,
}

impl AreaLight {
    /// An area light from ARE colours (0xBBGGRR, gamma space).
    pub fn from_are(ambient: u32, diffuse: u32, direction: Vec3) -> AreaLight {
        AreaLight { ambient: bgr_linear(ambient), diffuse: bgr_linear(diffuse), direction }
    }
}

impl Default for AreaLight {
    /// A neutral daylight for previews: the game's default sun direction
    /// (towards 4000, 4500, 7000).
    fn default() -> AreaLight {
        AreaLight {
            ambient: Vec3::splat(0.35_f32.powf(2.2)),
            diffuse: Vec3::splat(0.85_f32.powf(2.2)),
            direction: Vec3::new(4000.0, 4500.0, 7000.0).normalize(),
        }
    }
}

/// An ARE colour (0x00BBGGRR) in linear space.
pub fn bgr_linear(c: u32) -> Vec3 {
    let ch = |shift: u32| (((c >> shift) & 0xFF) as f32 / 255.0).powf(2.2);
    Vec3::new(ch(0), ch(8), ch(16))
}

/// Linear fog by view depth, blended in gamma space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fog {
    pub start: f32,
    pub end: f32,
    /// Gamma space.
    pub color: Vec3,
}

/// Everything drawn in one frame.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub instances: Vec<Instance>,
    pub lights: Vec<PointLight>,
    pub area: AreaLight,
    pub fog: Option<Fog>,
    /// Clear colour, gamma space.
    pub background: [f32; 3],
    /// Particle quads, drawn after the meshes.
    pub particles: Vec<crate::particles::ParticleBatch>,
    /// The area's environment map (the tileset's `EnvMap`), for textures
    /// asking for `default`; `chrome1` when `None`.
    pub env_map: Option<String>,
    /// The skybox: drawn around the camera (its transform taken from the
    /// camera's position, at ground level), behind everything, unlit and
    /// without fog.
    pub sky: Option<Instance>,
    /// The game's `skyfade1` over the sky's lower part, with the colour it
    /// lays over it (gamma: the area's fog colour) as much as its texture
    /// is white.
    pub sky_fade: Option<(Instance, Vec3)>,
    /// Lines drawn after the meshes, hidden behind them (the area's tile
    /// grid), a pixel wide.
    pub lines: Vec<Line>,
}

/// A line in the scene ([`Scene::lines`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub from: Vec3,
    pub to: Vec3,
    /// Gamma-space colour and alpha.
    pub color: [f32; 4],
}

/// A perspective camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    /// Vertical field of view, radians.
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// Looking at `target` from `distance` away, at `yaw` (around Z from +X)
    /// and `pitch` (up from the ground plane), in radians.
    pub fn orbit(target: Vec3, distance: f32, yaw: f32, pitch: f32) -> Camera {
        let dir = Vec3::new(pitch.cos() * yaw.cos(), pitch.cos() * yaw.sin(), pitch.sin());
        Camera {
            eye: target + dir * distance,
            target,
            fov_y: 40f32.to_radians(),
            near: (distance * 0.01).max(0.01),
            far: distance * 20.0 + 100.0,
        }
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, Vec3::Z)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect.max(1e-3), self.near, self.far)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The client's uniforms for these lights (read back through a debug
    /// shader): colour and cutoff.
    #[test]
    fn lights_convert_like_the_game() {
        let close = |a: Vec3, b: Vec3| (a - b).abs().max_element() < 1e-3;
        let cases = [
            // Tile main light 1, lightcolor White, BrightWhite, DimWhite, Yellow.
            (Vec3::splat(1.2), 10.0, Vec3::ONE, 24.0),
            (Vec3::splat(2.0), 10.0, Vec3::ONE, 40.0),
            (Vec3::splat(0.6), 10.0, Vec3::splat(0.3250), 20.0),
            (Vec3::new(1.9, 1.7, 0.06), 10.0, Vec3::new(1.0, 0.7830, 0.0005), 38.0),
            // Main light 2, White.
            (Vec3::splat(1.2), 5.0, Vec3::ONE, 12.0),
            // Source lights (fx_flame01, radius 7): animation 1, PaleDarkBlue.
            (Vec3::splat(2.0), 7.0, Vec3::ONE, 28.0),
            (Vec3::new(0.7, 0.7, 1.2), 7.0, Vec3::new(0.3055, 0.3055, 1.0), 16.8),
        ];
        for (color, radius, want_color, want_cutoff) in cases {
            let l = PointLight::new(Vec3::ZERO, color, radius, false, 4);
            assert!(close(l.color, want_color), "{color}: {}", l.color);
            assert!((l.cutoff - want_cutoff).abs() < 1e-3, "{color}: {}", l.cutoff);
        }
    }
}
