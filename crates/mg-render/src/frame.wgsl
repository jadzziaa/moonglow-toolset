// The frame's uniforms, as every one of Moonglow's own shaders takes them
// (the renderer's `FrameUniform`, field for field): put before the lit
// shader's, the particles' and the lines' sources, so that there is one of
// it. (Each had its own, the particles' and the lines' a field short: read
// right as a prefix, until a field came before the last.)

struct Frame {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // Area (sun/moon) light, linear.
    area_ambient: vec4<f32>,
    area_diffuse: vec4<f32>,
    // View-space direction towards the sun/moon.
    area_dir: vec4<f32>,
    // enabled, start, end, 1 / (end - start)
    fog: vec4<f32>,
    // Gamma space.
    fog_color: vec4<f32>,
    // 1 / max intensity, falloff factor, nothing, target height (pixels)
    light_params: vec4<f32>,
    // x = seconds, for what moves by itself (water).
    time: vec4<f32>,
};

