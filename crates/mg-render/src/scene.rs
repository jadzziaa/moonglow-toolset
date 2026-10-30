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
    /// `env_map` is what the object's own `default` comes to (a
    /// placeable's `Reflection` `default` is `chrome1`, whatever the
    /// area's): a mesh whose texture says `envmaptexture default` reflects
    /// the area's map instead, as it does on an object whose map is
    /// `default` or none; only a map the object names is taken over the
    /// area's (read from the client: `program_choice_in_the_client` in
    /// `client_render.rs`, `MG_LYING_ENVD=1` with `MG_REFLECTION`
    /// `default` and `plc_g_env`).
    pub env_map_default: bool,
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
    /// A tile of an area: the meshes of its model that turn their texture
    /// with them (`rotatetexture 1`, the tilesets' ground) have it turned
    /// by the tile's orientation, through the game's `m_texture` (read
    /// from the client: `program_choice_in_the_client` in
    /// `client_render.rs`). A placeable's are not, however it is turned.
    pub tile: bool,
    /// An area's grass (the tufts the editor grows on its tiles'
    /// walkmesh): the game draws it with `vsgrass`/`fsgrass`, lit at its
    /// corners (read from the client: `surfaces_in_the_client` in
    /// `client_render.rs`).
    pub grass: bool,
    /// The sun's and moon's shadows fall on it ([`Scene::shadows`]): the
    /// client darkens an area's tiles and its static placeables alone,
    /// not what moves (a dynamic placeable, a door, a creature), read
    /// from its pictures (`shadows_look` in `client_render.rs`).
    pub receives_shadows: bool,
    /// Its meshes flagged `shadow` cast shadows (with `render 0` meshes
    /// that only do): on, unless it is a marker, see-through or grass.
    pub casts_shadows: bool,
    /// It casts a shadow from the nearest light that casts them
    /// ([`Scene::light_shadows`]): a creature, as the client's do where
    /// the area has no sun or moon shadows at the time.
    pub casts_light_shadows: bool,
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
            env_map_default: false,
            plt_colors: None,
            textures: None,
            opacity: 1.0,
            unlit: false,
            tile: false,
            grass: false,
            receives_shadows: false,
            casts_shadows: true,
            casts_light_shadows: false,
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
    /// A creature's shadow can be cast from it ([`Scene::light_shadows`]):
    /// a light node flagged `shadow` of a radius of 10 m or more (the
    /// tiles' flames, radius 7, cast none in the client; a placeable's
    /// `LightColor` light, 10, does).
    pub casts_shadows: bool,
    /// How far above the light its shadows are cast from, metres (its
    /// node's `verticaldisplacement`: a placeable's light, 3, casts
    /// shorter shadows than one from where it is, as the client's).
    pub shadow_lift: f32,
}

/// The least radius of a light a creature's shadow is cast from.
pub const SHADOW_LIGHT_RADIUS: f32 = 10.0;

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
            casts_shadows: false,
            shadow_lift: 0.0,
        }
    }

    /// This light, casting creatures' shadows where `shadow` (its node's
    /// flag) and its `radius` is [`SHADOW_LIGHT_RADIUS`] or more, from
    /// `lift` metres above it.
    pub fn casting(self, shadow: bool, radius: f32, lift: f32) -> Self {
        let casts_shadows = shadow && radius >= SHADOW_LIGHT_RADIUS;
        PointLight { casts_shadows, shadow_lift: lift, ..self }
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

/// The sun's or moon's shadows over a scene (or the lights'), as the game's shadow plane
/// lays them (`docs/research/notes_shaders.md` §G): what the area light
/// ([`Scene::area`], its direction) shades of what receives them
/// ([`Instance::receives_shadows`]) is darkened, black laid over it as
/// much as the area's Shadow Opacity says, less towards the top of the
/// screen (the plane's fade) and in fog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunShadows {
    /// The area's `ShadowOpacity`, 0 to 1.
    pub opacity: f32,
    /// Where the fade of the shadows up the screen ends: the area's fog
    /// clip distance (`FogClipDist`), as the client's fog is always on.
    pub fade_limit: f32,
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
    /// Lines drawn many times over, after [`Scene::lines`] and as they are:
    /// each shape's copies (an area's Sound Ranges, a dotted sphere for
    /// each distance), its lines sent to the GPU once and only the copies
    /// each frame.
    pub line_shapes: Vec<LineShape>,
    /// Seconds, for what moves by itself: the game's water ripples, and
    /// the game's shaders' clock (`worldtimerTimeOfDay`,
    /// `sceneCurrentFrame`: wind-swayed grass, flowing water).
    pub time: f32,
    /// What the game's shaders read of the area besides its light.
    pub environment: Environment,
    /// The area light's shadows; `None`: none.
    pub shadows: Option<SunShadows>,
    /// The shadows cast from point lights (each instance that
    /// [`Instance::casts_light_shadows`] casts one from the nearest light
    /// that [`PointLight::casts_shadows`]), as dark as these say; `None`:
    /// none.
    pub light_shadows: Option<SunShadows>,
}

/// What the game's shaders read of an area besides its light
/// (`docs/research/notes_shaders.md` §B.4): its flags, its wind and its
/// weather. Nothing of it changes what Moonglow's own shader draws.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Environment {
    /// `areaFlags`, the ARE's `Flags`: 1 interior, 2 underground, 4
    /// natural.
    pub flags: u32,
    /// `areaGlobalWind`: its length squared 1 for a medium wind, 4 for a
    /// strong one.
    pub wind: Vec3,
    /// `areaWeatherType`: 0 clear, 1 rain, 2 snow.
    pub weather: u8,
    /// `areaWeatherDensity`.
    pub weather_density: f32,
    /// The module's clock (`moduleHour`, `moduleDawnHour`…).
    pub clock: ModuleClock,
}

/// The module's clock as the game's shaders read it (`moduleHour`,
/// `moduleMinutesPerHour`, `moduleDawnHour`, `moduleDuskHour`,
/// `moduleDay`, `moduleMonth`, `moduleYear`): the module.ifo's starting
/// hour, day, month and year, its minutes an hour and its dawn and dusk
/// (read back from the client as it starts a module:
/// `program_choice_in_the_client` in `client_render.rs`). The editor's
/// clock doesn't run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleClock {
    pub hour: u32,
    pub minutes_per_hour: u32,
    pub dawn: u32,
    pub dusk: u32,
    pub day: u32,
    pub month: u32,
    pub year: u32,
}

impl Default for ModuleClock {
    /// No module's: noon of a day of two minutes an hour, dawn at 6, dusk
    /// at 18 (the toolset's new module's).
    fn default() -> Self {
        ModuleClock {
            hour: 12,
            minutes_per_hour: 2,
            dawn: 6,
            dusk: 18,
            day: 1,
            month: 1,
            year: 1372,
        }
    }
}

impl Environment {
    /// An area's, from its ARE's `Flags` and `WindPower` (0 none, 1
    /// medium, 2 strong). Its weather is clear: the game rolls it from the
    /// ARE's chances as it runs, and the editor runs none.
    pub fn from_are(flags: u32, wind_power: i64) -> Environment {
        Environment {
            flags,
            wind: wind(wind_power),
            weather: 0,
            weather_density: 0.0,
            clock: ModuleClock::default(),
        }
    }
}

/// `areaGlobalWind` for an ARE's `WindPower`: none, (0.707, 0.707, 0) and
/// (1.414, 1.414, 0), as The Domains of Dread's wind shaders give the
/// toolset's three (their comments, `shd_windgrass_vs`; a length squared of
/// 1 and 4, as notes §B.4 has it), and as the client uploads it (a strong
/// wind read back: `program_choice_in_the_client` in `client_render.rs`).
pub fn wind(power: i64) -> Vec3 {
    let along = std::f32::consts::FRAC_1_SQRT_2 * power.clamp(0, 2) as f32;
    Vec3::new(along, along, 0.0)
}

/// A line in the scene ([`Scene::lines`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub from: Vec3,
    pub to: Vec3,
    /// Gamma-space colour and alpha.
    pub color: [f32; 4],
}

/// Lines drawn many times over ([`Scene::line_shapes`]): the same lines,
/// moved and scaled, in each copy's colour. The renderer keeps a shape's
/// lines on the GPU while the shape is drawn, known by its `lines` (the
/// same `Arc` from frame to frame): only the copies are sent each frame.
#[derive(Debug, Clone)]
pub struct LineShape {
    /// The lines' ends, about the origin.
    pub lines: Arc<[[Vec3; 2]]>,
    pub copies: Vec<LineCopy>,
}

/// One of a [`LineShape`]'s copies: each end at `offset + end * scale`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineCopy {
    pub offset: Vec3,
    pub scale: f32,
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

    /// How far a point of a view `height` points tall is at the target
    /// (across the view, as up it): what a drag of a point moves there, so
    /// that what is dragged follows the pointer. The area view's pan, its
    /// lift and its arrows seen end on, and the model viewer's pan.
    pub fn per_point(&self, height: f32) -> f32 {
        2.0 * self.eye.distance(self.target) * (self.fov_y / 2.0).tan() / height.max(1.0)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect.max(1e-3), self.near, self.far)
    }

    /// The projection as the game's shaders take it: OpenGL's, depth from
    /// -1 to 1 (`m_proj`).
    pub fn projection_gl(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov_y, aspect.max(1e-3), self.near, self.far)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A step of `per_point` at the target, across the view, is a point
    /// on the screen: what is dragged follows the pointer.
    #[test]
    fn a_point_at_the_target_is_per_point_there() {
        let target = Vec3::new(10.0, 20.0, 0.0);
        for distance in [5.0, 40.0, 300.0] {
            let c = Camera::orbit(target, distance, 0.7, 0.9);
            let (w, h) = (1200.0, 800.0);
            let m = c.projection(w / h) * c.view();
            let on_screen = |p: Vec3| {
                let v = m.project_point3(p);
                glam::Vec2::new((v.x + 1.0) / 2.0 * w, (1.0 - v.y) / 2.0 * h)
            };
            let right = (c.target - c.eye).cross(Vec3::Z).normalize();
            let step = on_screen(target + right * c.per_point(h)) - on_screen(target);
            assert!((step.length() - 1.0).abs() < 1e-3, "{distance}: {step}");
        }
    }

    /// The client's uniforms for these lights (read back through a debug
    /// shader): colour and cutoff.
    #[test]
    fn wind_is_the_ares_power() {
        assert_eq!(wind(0), Vec3::ZERO);
        assert!((wind(1).length_squared() - 1.0).abs() < 1e-6);
        assert!((wind(2).length_squared() - 4.0).abs() < 1e-5);
        assert_eq!(wind(7), wind(2));
        let e = Environment::from_are(5, 1);
        assert_eq!((e.flags, e.weather), (5, 0));
        assert!(e.wind.x == e.wind.y && e.wind.z == 0.0 && e.wind.x > 0.7);
    }

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
