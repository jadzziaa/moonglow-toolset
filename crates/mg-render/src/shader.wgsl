// Lit meshes, after the game's stock shaders (vslit/fslit, inc_standard,
// inc_lighting, inc_material): per-fragment lighting in view space, linear
// and unclamped ("enhanced lighting"), textures linearised with pow 2.2,
// Lambert diffuse and GGX specular without 1/pi, Schlick Fresnel (^5) and
// geometric term (k = roughness), the legacy environment-map rule (texture
// alpha = 1 - reflectivity), EE point-light attenuation, then back to gamma,
// fog in gamma space and the "legacy balanced" colour clamp.

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
    // 1 / max intensity, falloff factor, light count, unused
    light_params: vec4<f32>,
    // Minimum light (the GUI "scene colour"), linear.
    scene_color: vec4<f32>,
};

struct Light {
    // View-space position.
    pos: vec4<f32>,
    // Linear colour; a = cutoff distance squared, negative: ambient only.
    color: vec4<f32>,
};

struct Draw {
    model_view: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
    // Material diffuse, a = mesh alpha.
    diffuse: vec4<f32>,
    ambient: vec4<f32>,
    // Self-illumination, linear.
    emissive: vec4<f32>,
    // alpha discard value, environment-mapped, has texture, unlit (decal)
    params: vec4<f32>,
    // specularity, roughness, metallicness overrides (0 = derive), texture has alpha
    material: vec4<f32>,
    // x = number of lights for this draw
    light_count: vec4<u32>,
    // Up to 32 light indices.
    light_index: array<vec4<u32>, 8>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> lights: array<Light>;
@group(1) @binding(0) var<uniform> draw: Draw;
@group(2) @binding(0) var tex0: texture_2d<f32>;
@group(2) @binding(1) var tex_env: texture_2d<f32>;
@group(2) @binding(2) var samp0: sampler;
@group(2) @binding(3) var samp_env: sampler;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) pos_view: vec3<f32>,
    @location(1) normal_view: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    let p = draw.model_view * vec4<f32>(v.pos, 1.0);
    out.pos_view = p.xyz;
    out.clip = frame.proj * p;
    out.normal_view = (draw.normal_matrix * vec4<f32>(v.normal, 0.0)).xyz;
    out.uv = v.uv;
    return out;
}

fn lin(c: vec3<f32>) -> vec3<f32> {
    return sign(c) * pow(abs(c), vec3<f32>(2.2));
}

fn gam(c: vec3<f32>) -> vec3<f32> {
    return sign(c) * pow(abs(c), vec3<f32>(1.0 / 2.2));
}

fn fresnel(spec0: f32, c: f32) -> f32 {
    let f = 1.0 - c;
    let f2 = f * f;
    return mix(spec0, 1.0, f2 * f2 * f);
}

// The environment map's 2D ("sphere") coordinates, view space.
fn env_coords(n: vec3<f32>, pos_view: vec3<f32>) -> vec2<f32> {
    let e = n.xy / (1.0 + abs(n.z)) + 0.5 * pos_view.xy / (-pos_view.z + 0.2);
    return 0.5 * e + vec2<f32>(0.5, 0.25);
}

// "Legacy balanced" colour clamp: values above 1 desaturate towards white.
fn color_clamp(c: vec3<f32>) -> vec3<f32> {
    let m = max(c.r, max(c.g, c.b));
    if (m <= 1.0) {
        return c;
    }
    let k = (m - 1.0) / m;
    return clamp(vec3<f32>(m) - (vec3<f32>(m) - c) * (1.0 - k * k), vec3<f32>(0.0), vec3<f32>(1.0));
}

struct Accum {
    ambient: vec3<f32>,
    diffuse: vec3<f32>,
    specular: vec3<f32>,
};

@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let env_mapped = draw.params.y > 0.5;
    let has_texture = draw.params.z > 0.5;
    var color = vec4<f32>(1.0, 1.0, 1.0, clamp(draw.diffuse.a, 0.0, 1.0));
    if (env_mapped && color.a <= draw.params.x) {
        discard;
    }

    // Base texture.
    var tex = vec4<f32>(1.0);
    var env_level = 0.0;
    if (has_texture) {
        tex = textureSample(tex0, samp0, in.uv);
        if (draw.material.w < 0.5) {
            tex.a = 1.0;
        }
    }
    if (env_mapped) {
        if (has_texture) {
            env_level = 1.0 - tex.a;
            tex = vec4<f32>(mix(tex.rgb, vec3<f32>(0.666667), env_level), tex.a);
        } else {
            tex = vec4<f32>(0.666667, 0.666667, 0.666667, 1.0);
            env_level = 1.0;
        }
        color = vec4<f32>(color.rgb * lin(tex.rgb), color.a);
    } else {
        color = color * vec4<f32>(lin(tex.rgb), tex.a);
        if (color.a <= draw.params.x) {
            discard;
        }
    }

    // Unlit (TXI decal).
    if (draw.params.w > 0.5) {
        return vec4<f32>(color_clamp(gam(color.rgb)), color.a);
    }

    var n = normalize(select(-in.normal_view, in.normal_view, front));
    let v = -normalize(in.pos_view);

    // Material values (SetupSpecularity).
    let albedo = color.rgb * draw.diffuse.rgb;
    var spec0 = 0.04;
    if (draw.material.x > 0.0) {
        spec0 = draw.material.x;
    } else if (env_mapped) {
        spec0 = mix(0.04, 0.98, min(env_level * 8.0, 1.0));
    }
    var rough = 0.55;
    if (draw.material.y > 0.0) {
        rough = draw.material.y;
    } else if (env_mapped) {
        rough = mix(0.55, 0.125, min(env_level * 2.5, 1.0));
    }
    var metal = clamp(10.0 * spec0 - 0.4, 0.0, 1.0);
    if (draw.material.z > 0.0) {
        metal = draw.material.z;
    }
    let max_albedo = max(albedo.r, max(albedo.g, albedo.b));
    let spec_color = mix(vec3<f32>(min(1.0, max_albedo / 0.217638)), albedo, metal);

    // Bend normals facing away from the eye.
    var n_dot_v = dot(n, v);
    if (n_dot_v < 0.0) {
        n = normalize(n - n_dot_v * v);
        n_dot_v = 0.0;
    }
    let r2 = rough * rough;
    let k = rough;

    var acc: Accum;
    acc.ambient = frame.area_ambient.rgb;
    acc.diffuse = vec3<f32>(0.0);
    acc.specular = vec3<f32>(0.0);

    // The area light, then point lights.
    let count = draw.light_count.x;
    for (var i = 0u; i <= count; i = i + 1u) {
        var l: vec3<f32>;
        var c: vec3<f32>;
        var att: f32;
        if (i == count) {
            l = normalize(frame.area_dir.xyz);
            c = frame.area_diffuse.rgb;
            att = 1.0;
        } else {
            let light = lights[draw.light_index[i / 4u][i % 4u]];
            let to_light = light.pos.xyz - in.pos_view;
            let d2 = dot(to_light, to_light);
            let r_cut2 = abs(light.color.a);
            if (d2 > r_cut2) {
                continue;
            }
            let f = d2 / r_cut2;
            att = (1.0 - f) / (frame.light_params.x + frame.light_params.y * f);
            if (light.color.a < 0.0) {
                acc.ambient = acc.ambient + light.color.rgb * att;
                continue;
            }
            l = normalize(to_light);
            c = light.color.rgb;
        }
        let n_dot_l = dot(n, l);
        if (n_dot_l <= 0.0) {
            continue;
        }
        let a = att * n_dot_l;
        acc.diffuse = acc.diffuse + c * a;
        let v_dot_h = sqrt(dot(v, l) * 0.5 + 0.5);
        let n_dot_h = clamp((n_dot_l + n_dot_v) * 0.5 / max(v_dot_h, 1e-4), 0.0, 1.0);
        let den = n_dot_h * n_dot_h * (r2 - 1.0) + 1.0;
        let g_l = 1.0 / (n_dot_l * (1.0 - k) + k);
        acc.specular = acc.specular + c * a * (1.0 / (den * den)) * g_l * fresnel(spec0, v_dot_h);
    }

    let ambient = acc.ambient * draw.ambient.rgb;
    let diffuse = acc.diffuse * draw.diffuse.rgb;
    var specular = acc.specular * (r2 * 0.25) / (n_dot_v * (1.0 - k) + k);
    let env_spec = mix(fresnel(spec0, n_dot_v), spec0, sqrt(rough));
    let lod = clamp(rough * 30.0 - 1.0, 0.0, 10.0);
    let env_sample = lin(textureSampleLevel(tex_env, samp_env, env_coords(n, in.pos_view), lod).rgb);
    specular = specular + (ambient + diffuse) * env_sample * env_spec;
    var total = draw.emissive.rgb + (1.0 - env_spec) * (ambient + diffuse);
    total = max(total, min(frame.scene_color.rgb * draw.diffuse.rgb, vec3<f32>(1.0)));
    var rgb = color.rgb * total;
    if (color.a > 0.001 && color.a < 1.0) {
        specular = specular / max(color.a, 0.1);
    }
    rgb = rgb + spec_color * specular;

    // Back to gamma; fog in gamma space.
    rgb = gam(rgb);
    if (frame.fog.x > 0.5) {
        let f = clamp((-in.pos_view.z - frame.fog.y) * frame.fog.w, 0.0, 1.0);
        rgb = mix(rgb, frame.fog_color.rgb, f);
    }
    return vec4<f32>(color_clamp(rgb), color.a);
}
