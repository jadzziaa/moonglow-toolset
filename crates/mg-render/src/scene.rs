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
