// Particles: unlit textured quads in world space, coloured and faded over
// their life, fogged like everything else (in gamma space), and faded where
// they meet what is behind them (soft) as the game's `fsparticle` fades
// them: through the framebuffer's depth copy, by their draw's softness.

struct Frame {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    area_ambient: vec4<f32>,
    area_diffuse: vec4<f32>,
    area_dir: vec4<f32>,
    fog: vec4<f32>,
    fog_color: vec4<f32>,
    light_params: vec4<f32>,
    scene_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var tex0: texture_2d<f32>;
@group(1) @binding(2) var samp0: sampler;
// The framebuffer's depth copy (`R32Float`, the depth buffer's values, its
// rows bottom first as OpenGL's); 1x1 where the target has none.
@group(2) @binding(0) var depth_copy: texture_2d<f32>;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) softness: f32,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) depth: f32,
    @location(3) @interpolate(flat) softness: f32,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    let p = frame.view * vec4<f32>(v.pos, 1.0);
    out.clip = frame.proj * p;
    out.uv = v.uv;
    out.color = v.color;
    out.depth = -p.z;
    out.softness = v.softness;
    return out;
}

// How much of a soft particle shows where it is: `fsparticle`'s
// `sqr(clamp((scene's depth - its own) * particleSoftness, 0, 1))`, the
// scene's distance from the depth copy at the fragment's pixel.
fn soft(in: VertexOut) -> f32 {
    if (in.softness <= 0.0) {
        return 1.0;
    }
    let size = textureDimensions(depth_copy);
    let x = min(u32(in.clip.x), size.x - 1u);
    let y = min(u32(in.clip.y), size.y - 1u);
    let d = textureLoad(depth_copy, vec2<u32>(x, size.y - 1u - y), 0).r;
    // (A perspective projection's depth back to a distance.)
    let scene = frame.proj[3][2] / (d + frame.proj[2][2]);
    let f = clamp((scene - in.depth) * in.softness, 0.0, 1.0);
    return f * f;
}

fn fog_amount(depth: f32) -> f32 {
    if (frame.fog.x < 0.5) {
        return 0.0;
    }
    return clamp((depth - frame.fog.y) * frame.fog.w, 0.0, 1.0);
}

// Alpha-blended and punch-through particles.
@fragment
fn fs_blend(in: VertexOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex0, samp0, in.uv);
    var c = t * in.color;
    c = vec4<f32>(mix(c.rgb, frame.fog_color.rgb, fog_amount(in.depth)), c.a * soft(in));
    return c;
}

@fragment
fn fs_punch(in: VertexOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex0, samp0, in.uv);
    let c = t * in.color;
    // (Cut at 0.2, as the game cuts everything: a ramp of alpha on a
    // `Punch-Through` particle is there from a fifth of its width on, seen
    // in the client: `particles_look`, `MG_PARTICLES=punch`.)
    if (c.a <= 0.2) {
        // (And nothing more of it: see the lit meshes' shader.)
        discard;
        return vec4<f32>(0.0);
    }
    return vec4<f32>(mix(c.rgb, frame.fog_color.rgb, fog_amount(in.depth)), 1.0);
}

// "Lighten": added to what is behind, fading into fog.
@fragment
fn fs_add(in: VertexOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex0, samp0, in.uv);
    let a = t.a * in.color.a * soft(in) * (1.0 - fog_amount(in.depth));
    return vec4<f32>(t.rgb * in.color.rgb * a, a);
}
