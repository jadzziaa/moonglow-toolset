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

/// A point light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub position: Vec3,
    /// Linear colour (lightcolor.2da values go straight in; above 1 is
    /// brighter).
    pub color: Vec3,
    /// The light's radius; the cutoff is twice that.
    pub radius: f32,
    pub ambient_only: bool,
    /// 1 (highest) to 5: which lights are kept past 32.
    pub priority: u32,
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
