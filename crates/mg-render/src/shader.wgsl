// Lit meshes, after the game's stock shaders (vslit/fslit, inc_standard,
// inc_lighting, inc_material): per-fragment lighting in view space, linear
// and unclamped ("enhanced lighting"), textures linearised with pow 2.2,
// Lambert diffuse and GGX specular without 1/pi, Schlick Fresnel (^5) and
// geometric term (k = roughness), light softening and translucence (the
// "High Quality" setting), the legacy environment-map rule (texture alpha =
// 1 - reflectivity), EE point-light attenuation, then back to gamma, fog in
// gamma space and the "legacy balanced" colour clamp. Normal, specular,
// roughness, height (parallax and occlusion) and self-illumination maps as
// the normal-mapped variants (fslit_nm); their tangent frame is the stock
// shaders' (the vertex tangent, the bitangent N × T by its sign) where the
// model has tangents, else from screen-space derivatives.

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
    // 1 / max intensity, falloff factor, light count, target height (pixels)
    light_params: vec4<f32>,
    // Minimum light (the GUI "scene colour"), linear; w = debug view
    // (1: material diffuse).
    scene_color: vec4<f32>,
    // x = seconds, for what moves by itself (water).
    time: vec4<f32>,
};

struct Light {
    // View-space position.
    pos: vec4<f32>,
    // Linear colour; a = cutoff distance squared, negative: ambient only.
    color: vec4<f32>,
};

struct Draw {
    // Model to world; the view matrix is the frame's, so coplanar meshes
    // (tile floors) get equal depths and the first drawn stays.
    world: mat4x4<f32>,
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
    // normal, specular, roughness, height map bound
    maps: vec4<f32>,
    // self-illumination map bound, displacement offset, displacement
    // multiplier, normal-mapped variant
    maps2: vec4<f32>,
    // Custom specular colour (linear), w = 1 when set.
    spec_color: vec4<f32>,
    // x = the environment map is a cube map; w = the part of a see-through
    // mesh drawn: 1 its solid part, 2 the rest (0: all of it)
    extra: vec4<f32>,
    // A rippling texture (the game's procedural water): x = how far its
    // picture is pushed about (a part of its width), y = how fast; 0: still.
    // z = 1 for water (`bumpmaptexture shinywater`): waves cross it.
    water: vec4<f32>,
    // A known custom shader's doings (the `vertexalpha` pair): x = which
    // (1 vertex colors, 2 an alpha mask on the second coordinates, 4 a
    // lightmap there, 8 a flow map slides the texture), y = slide speed,
    // z = shadow reduction, w = shadow brightening.
    effect: vec4<f32>,
    // x = number of lights for this draw, y = 1 if skinned, z = first bone
    light_count: vec4<u32>,
    // Up to 32 light indices.
    light_index: array<vec4<u32>, 8>,
};

// From this alpha a see-through mesh's surface counts as solid.
const SOLID_ALPHA: f32 = 0.95;

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> lights: array<Light>;
// Skin bone matrices (bind pose to current pose, in the skin node's space).
@group(0) @binding(2) var<storage, read> bones: array<mat4x4<f32>>;
@group(1) @binding(0) var<uniform> draw: Draw;
@group(2) @binding(0) var tex0: texture_2d<f32>;
@group(2) @binding(1) var tex_env: texture_2d<f32>;
@group(2) @binding(2) var samp0: sampler;
@group(2) @binding(3) var samp_env: sampler;
@group(2) @binding(4) var tex_normal: texture_2d<f32>;
@group(2) @binding(5) var tex_spec: texture_2d<f32>;
@group(2) @binding(6) var tex_rough: texture_2d<f32>;
@group(2) @binding(7) var tex_height: texture_2d<f32>;
@group(2) @binding(8) var tex_illum: texture_2d<f32>;
@group(2) @binding(9) var tex_env_cube: texture_cube<f32>;
@group(2) @binding(10) var tex_mask: texture_2d<f32>;
@group(2) @binding(11) var tex_flow: texture_2d<f32>;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    // Tangent and the bitangent's sign; zero without tangents.
    @location(5) tangent: vec4<f32>,
    // The model's vertex color and second texture coordinates.
    @location(6) color: vec4<f32>,
    @location(7) uv1: vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) pos_view: vec3<f32>,
    @location(1) normal_view: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent_view: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) uv1: vec2<f32>,
};

fn finish(v: VertexIn, pos: vec3<f32>, normal: vec3<f32>, tangent: vec4<f32>) -> VertexOut {
    var out: VertexOut;
    let uv = v.uv;
    out.color = v.color;
    out.uv1 = v.uv1;
    let p = frame.view * (draw.world * vec4<f32>(pos, 1.0));
    out.pos_view = p.xyz;
    out.clip = frame.proj * p;
    // The sky: at the far plane, behind everything and never cut off by it.
    if (draw.extra.y > 0.5) {
        out.clip.z = out.clip.w * 0.99999;
    }
    out.normal_view = (draw.normal_matrix * vec4<f32>(normal, 0.0)).xyz;
    out.uv = uv;
    let t = frame.view * (draw.world * vec4<f32>(tangent.xyz, 0.0));
    out.tangent_view = vec4<f32>(t.xyz, tangent.w);
    return out;
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    return finish(v, v.pos, v.normal, v.tangent);
}

struct SkinIn {
    @location(3) bones: vec4<u32>,
    @location(4) weights: vec4<f32>,
};

// Up to four bone influences, as the game's skinned shaders (vslit_sk).
@vertex
fn vs_skinned(v: VertexIn, s: SkinIn) -> VertexOut {
    var pos = vec3<f32>(0.0);
    var normal = vec3<f32>(0.0);
    var tangent = vec3<f32>(0.0);
    var total = 0.0;
    for (var i = 0u; i < 4u; i = i + 1u) {
        let w = s.weights[i];
        if (w > 0.0) {
            let m = bones[draw.light_count.z + s.bones[i]];
            pos = pos + w * (m * vec4<f32>(v.pos, 1.0)).xyz;
            normal = normal + w * (m * vec4<f32>(v.normal, 0.0)).xyz;
            tangent = tangent + w * (m * vec4<f32>(v.tangent.xyz, 0.0)).xyz;
            total = total + w;
        }
    }
    if (total <= 0.0) {
        return finish(v, v.pos, v.normal, v.tangent);
    }
    return finish(v, pos / total, normal, vec4<f32>(tangent, v.tangent.w));
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

fn normalize_or_zero(x: vec3<f32>) -> vec3<f32> {
    let l2 = dot(x, x);
    if (l2 > 1e-30) {
        return x * inverseSqrt(l2);
    }
    return vec3<f32>(0.0);
}

// The tangent frame (columns +u, +v, the surface normal; view space) from
// screen-space derivatives of the position and texture coordinates
// (Schüler's cotangent frame; negated because framebuffer y points down).
fn tangent_frame(
    n: vec3<f32>,
    dp1: vec3<f32>,
    dp2: vec3<f32>,
    duv1: vec2<f32>,
    duv2: vec2<f32>,
) -> mat3x3<f32> {
    let dp2perp = cross(dp2, n);
    let dp1perp = cross(n, dp1);
    let t = -(dp2perp * duv1.x + dp1perp * duv2.x);
    let b = -(dp2perp * duv1.y + dp1perp * duv2.y);
    return mat3x3<f32>(normalize_or_zero(t), normalize_or_zero(b), n);
}

fn height_at(uv: vec2<f32>, duv1: vec2<f32>, duv2: vec2<f32>) -> f32 {
    return textureSampleGrad(tex_height, samp0, uv, duv1, duv2).r;
}

// Parallax: the game's iterative search (inc_displacement), height read as
// depth 1 - h. `v` points towards the eye.
fn displace(
    uv0: vec2<f32>,
    tsb: mat3x3<f32>,
    v: vec3<f32>,
    surface_n: vec3<f32>,
    pos_view: vec3<f32>,
    duv1: vec2<f32>,
    duv2: vec2<f32>,
) -> vec2<f32> {
    var multiplier = draw.maps2.z;
    if (multiplier == 0.0) {
        multiplier = 1.0;
    }
    var modifier = 1.0 + 0.5 * dot(-v, surface_n);
    modifier = modifier * modifier / (max(-pos_view.z, 1e-3) * 0.125);
    var iterations =
        32.0 * multiplier * clamp(modifier * frame.light_params.w / 1080.0, 0.0, 1.0);
    if (iterations <= 0.5) {
        return uv0;
    }
    let vd = transpose(tsb) * v;
    iterations = floor(iterations + 0.5);
    var segment = 1.0 / iterations;
    let h = 0.5 * vd.z + 0.5;
    var step = vd.xy * 0.05 * ((1.0 - vd.z) / (h * h) + 1.0);
    let start = uv0 + draw.maps2.y * step;
    step = step * multiplier;
    var delta = 1.0 - height_at(start, duv1, duv2);
    var current = delta * segment;
    for (var i = i32(iterations); i > 1; i = i - 1) {
        delta = 1.0 - height_at(start - step * current, duv1, duv2) - current;
        if (delta < 0.0) {
            delta = delta / max(current, 1e-4);
            segment = segment * 0.5;
        } else {
            delta = delta / max(1.0 - current, 1e-4);
        }
        current = current + delta * segment;
    }
    return start - step * current;
}

@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    // Derivatives first, while every fragment of the quad is running.
    let dp1 = dpdx(in.pos_view);
    let dp2 = dpdy(in.pos_view);
    let duv1 = dpdx(in.uv);
    let duv2 = dpdy(in.uv);
    let env_mapped = draw.params.y > 0.5;
    let has_texture = draw.params.z > 0.5;
    let normal_mapped = draw.maps.x > 0.5;
    let spec_mapped = draw.maps.y > 0.5;
    let rough_mapped = draw.maps.z > 0.5;
    let height_mapped = draw.maps.w > 0.5;
    let nm_variant = draw.maps2.w > 0.5;
    let surface_n = normalize(select(-in.normal_view, in.normal_view, front));
    let v = -normalize(in.pos_view);

    // Tangent frame and parallax.
    var uv = in.uv;
    // Water: the picture pushed about by two slow waves each way (an
    // approximation of the game's procedure, which is its own).
    if (draw.water.x > 0.0) {
        let t = frame.time.x * draw.water.y * 0.06;
        let k = 6.2831853;
        let wave = vec2<f32>(
            sin(k * uv.y * 1.0 + t) + sin(k * uv.x * 0.7 + t * 0.63 + 1.3),
            cos(k * uv.x * 1.0 + t * 0.81) + cos(k * uv.y * 0.6 + t * 0.47 + 2.1),
        );
        // (Under the waves of Enhanced Edition's water, half as far.)
        uv = uv + wave * (draw.water.x * select(0.25, 0.12, draw.water.z > 0.5));
    }
    // A known custom shader's doings (see `Draw.effect`).
    let fx = u32(draw.effect.x + 0.5);
    let fx_colors = (fx & 1u) != 0u;
    let fx_mask = (fx & 2u) != 0u;
    let fx_lightmap = (fx & 4u) != 0u;
    // The texture slid along its flow map (lava, water), as fast as the
    // material says.
    if ((fx & 8u) != 0u) {
        let way = textureSampleLevel(tex_flow, samp0, in.uv, 0.0).rg * 2.0 - 1.0;
        uv = uv + way * (draw.effect.y * frame.time.x);
    }
    var tsb = mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), surface_n);
    if (normal_mapped || height_mapped) {
        let t = in.tangent_view.xyz;
        if (in.tangent_view.w != 0.0 && dot(t, t) > 1e-12) {
            let tn = normalize(t);
            let handed = select(-1.0, 1.0, in.tangent_view.w >= 0.0);
            tsb = mat3x3<f32>(tn, cross(surface_n, tn) * handed, surface_n);
        } else {
            tsb = tangent_frame(surface_n, dp1, dp2, duv1, duv2);
        }
        if (height_mapped) {
            uv = displace(uv, tsb, v, surface_n, in.pos_view, duv1, duv2);
        }
    }

    var color = vec4<f32>(1.0, 1.0, 1.0, clamp(draw.diffuse.a, 0.0, 1.0));
    if (env_mapped && color.a <= draw.params.x) {
        discard;
    }

    // The sky fade: its colour over the sky as much as its texture is white.
    if (draw.extra.z > 0.5) {
        let t = textureSampleGrad(tex0, samp0, uv, duv1, duv2);
        return vec4<f32>(draw.diffuse.rgb, t.r);
    }

    // Base texture.
    var tex = vec4<f32>(1.0);
    var env_level = 0.0;
    if (has_texture) {
        tex = textureSampleGrad(tex0, samp0, uv, duv1, duv2);
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
        // The alpha of a layer blended into what is under it: a mask's, on
        // the second coordinates, else the texture's by the vertex color's.
        if (fx_mask) {
            color.a = clamp(draw.diffuse.a, 0.0, 1.0)
                * textureSampleLevel(tex_mask, samp0, in.uv1, 0.0).r;
        } else if (fx_lightmap) {
            color.a = 1.0;
        } else if (fx_colors) {
            color.a = color.a * in.color.a;
        }
        if (!fx_mask && color.a <= draw.params.x) {
            discard;
        }
        if (fx_mask && color.a <= 0.004) {
            discard;
        }
    }

    // A see-through mesh in two parts: what is nearly opaque of it first
    // (hiding what is behind), the rest afterwards (hiding nothing).
    if (draw.extra.w > 1.5) {
        if (color.a >= SOLID_ALPHA) {
            discard;
        }
    } else if (draw.extra.w > 0.5) {
        if (color.a < SOLID_ALPHA) {
            discard;
        }
    }

    // Debug view: the material colour as uploaded.
    if (frame.scene_color.w == 1.0) {
        return vec4<f32>(draw.diffuse.rgb, 1.0);
    }

    // A marker: unlit, in its material's colour.
    if (draw.params.w > 1.5) {
        return vec4<f32>(color_clamp(gam(color.rgb * draw.diffuse.rgb)), color.a);
    }
    // Unlit (TXI decal).
    if (draw.params.w > 0.5) {
        return vec4<f32>(color_clamp(gam(color.rgb)), color.a);
    }

    // The normal map: RG only, Z rebuilt.
    var n = surface_n;
    if (normal_mapped) {
        let t = textureSampleGrad(tex_normal, samp0, uv, duv1, duv2).rg * 2.0 - 1.0;
        n = normalize(tsb * vec3<f32>(t, sqrt(max(1.0 - dot(t, t), 0.0))));
    }

    // Water: small waves crossing it tip its surface this way and that, so
    // what it reflects (the environment, the lights) moves over it. Waves
    // of Moonglow's own, standing in for Enhanced Edition's water shader.
    let watery = draw.water.z > 0.5;
    if (watery) {
        // Where the fragment is in the world: waves there run on from tile
        // to tile (by its texture coordinates, each tile had its own).
        let view_rot = mat3x3<f32>(frame.view[0].xyz, frame.view[1].xyz, frame.view[2].xyz);
        let world = transpose(view_rot) * (in.pos_view - frame.view[3].xyz);
        let t = frame.time.x;
        // Six trains of waves, of lengths and headings that don't repeat
        // together, over ground bent a little so their crests aren't
        // straight: (heading, waves a metre, speed, height).
        let bent = world.xy
            + 0.45 * vec2<f32>(
                sin(world.y * 0.83 + t * 0.31) + sin(world.x * 0.41 - t * 0.17),
                cos(world.x * 0.71 - t * 0.27) + cos(world.y * 0.37 + t * 0.21),
            );
        var slope = vec2<f32>(0.0);
        let trains = array<vec4<f32>, 6>(
            vec4<f32>(0.35, 3.1, 1.3, 0.034),
            vec4<f32>(1.95, 4.3, 1.7, 0.030),
            vec4<f32>(3.60, 5.9, 2.2, 0.024),
            vec4<f32>(5.10, 7.7, 2.8, 0.020),
            vec4<f32>(2.75, 10.3, 3.4, 0.014),
            vec4<f32>(4.40, 13.1, 4.1, 0.010),
        );
        for (var i = 0; i < 6; i = i + 1) {
            let w = trains[i];
            let heading = vec2<f32>(cos(w.x), sin(w.x));
            slope = slope + heading * (cos(dot(bent, heading) * w.y + t * w.z) * w.w);
        }
        n = normalize(n + view_rot * vec3<f32>(-slope, 0.0));
    }

    // Occlusion from the height map's local depth.
    var ao = 1.0;
    var surface_fade = 10.0;
    if (height_mapped) {
        let blur = exp2(max(0.0, 7.0 - log2(max(-in.pos_view.z, 1e-3))));
        let around = height_at(uv, duv1 * blur, duv2 * blur);
        let occlusion = clamp(around - height_at(uv, duv1, duv2), 0.0, 0.9);
        ao = (1.0 - occlusion) * (1.0 - occlusion);
        surface_fade = 1.0 / mix(1.0, 0.1, ao * ao);
    }

    // Material values (SetupSpecularity).
    let albedo = color.rgb * draw.diffuse.rgb;
    var spec0 = 0.04;
    if (draw.material.x > 0.0) {
        spec0 = draw.material.x;
    } else if (spec_mapped) {
        spec0 = textureSampleGrad(tex_spec, samp0, uv, duv1, duv2).r;
    } else if (env_mapped) {
        spec0 = mix(0.04, 0.98, min(env_level * 8.0, 1.0));
    }
    let env_rough = mix(0.55, 0.125, min(env_level * 2.5, 1.0));
    var rough = 0.55;
    if (draw.material.y > 0.0) {
        rough = draw.material.y;
    } else if (rough_mapped) {
        rough = textureSampleGrad(tex_rough, samp0, uv, duv1, duv2).r;
    } else if (spec_mapped) {
        rough = mix(0.125, 0.55, (1.0 - spec0) * (1.0 - spec0));
        if (env_mapped) {
            rough = min(rough, env_rough);
        }
    } else if (env_mapped) {
        rough = env_rough;
    }
    // (Water shines: smooth, and mirror enough to show the sky.)
    if (watery) {
        spec0 = max(spec0, 0.25);
        rough = min(rough, 0.15);
    }
    var metal = clamp(10.0 * spec0 - 0.4, 0.0, 1.0);
    if (draw.material.z > 0.0) {
        metal = draw.material.z;
    } else if (nm_variant) {
        let m = spec0 * spec0;
        metal = clamp(m / (0.04 + 0.96 * m), 0.0, 1.0);
    }
    var spec_color: vec3<f32>;
    if (draw.spec_color.w > 0.5) {
        spec_color = draw.spec_color.rgb;
    } else if (height_mapped) {
        spec_color = mix(vec3<f32>(1.0), albedo, metal);
    } else {
        let max_albedo = max(albedo.r, max(albedo.g, albedo.b));
        spec_color = mix(vec3<f32>(min(1.0, max_albedo / 0.217638)), albedo, metal);
    }

    // Bend normals facing away from the eye.
    var n_dot_v = dot(n, v);
    if (n_dot_v < 0.0) {
        n = normalize(n - n_dot_v * v);
        n_dot_v = 0.0;
    }
    let r2 = rough * rough;
    let k = rough;
    // Light through transparent surfaces from behind.
    let translucence = 1.0 - color.a;

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
        var n_dot_l = dot(n, l);
        if (translucence > 0.0 && n_dot_l < 0.0) {
            acc.diffuse = acc.diffuse + c * (att * -n_dot_l * translucence);
        }
        // Softened: light reaches a little past the terminator.
        if (n_dot_l <= select(-0.25, 0.0, nm_variant)) {
            continue;
        }
        var a = att;
        if (nm_variant) {
            // No light around corners of the surface itself.
            a = a * clamp(dot(surface_n, l) * surface_fade, 0.0, 1.0);
        }
        a = a * 2.0 * smoothstep(0.0, 2.0, n_dot_l * 0.8 + 0.2);
        n_dot_l = max(n_dot_l, 0.0);
        acc.diffuse = acc.diffuse + c * a;
        let v_dot_h = sqrt(dot(v, l) * 0.5 + 0.5);
        let n_dot_h = clamp((n_dot_l + n_dot_v) * 0.5 / max(v_dot_h, 1e-4), 0.0, 1.0);
        let den = n_dot_h * n_dot_h * (r2 - 1.0) + 1.0;
        let g_l = 1.0 / (n_dot_l * (1.0 - k) + k);
        acc.specular = acc.specular + c * a * (1.0 / (den * den)) * g_l * fresnel(spec0, v_dot_h);
    }

    var ambient = acc.ambient * draw.ambient.rgb;
    let diffuse = acc.diffuse * draw.diffuse.rgb;
    var specular = acc.specular * (r2 * 0.25) / (n_dot_v * (1.0 - k) + k);
    let env_spec = mix(fresnel(spec0, n_dot_v), spec0, sqrt(rough));
    let lod = clamp(rough * 30.0 - 1.0, 0.0, 10.0);
    var env_sample: vec3<f32>;
    if (draw.extra.x > 0.5) {
        // Cube maps: the reflected view in world space, level 0.
        let r = reflect(-v, n);
        let to_world = transpose(mat3x3<f32>(frame.view[0].xyz, frame.view[1].xyz, frame.view[2].xyz));
        env_sample = lin(textureSampleLevel(tex_env_cube, samp_env, to_world * r, 0.0).rgb);
    } else {
        env_sample = lin(textureSampleLevel(tex_env, samp_env, env_coords(n, in.pos_view), lod).rgb);
    }
    specular = specular + (ambient + diffuse) * env_sample * ao * env_spec;
    ambient = ambient * ao;
    var total = draw.emissive.rgb + (1.0 - env_spec) * (ambient + diffuse);
    total = max(total, min(frame.scene_color.rgb * draw.diffuse.rgb, vec3<f32>(1.0)));
    var rgb = color.rgb * total;
    if (draw.maps2.x > 0.5) {
        rgb = rgb + lin(textureSampleGrad(tex_illum, samp0, uv, duv1, duv2).rgb);
    }
    if (color.a > 0.001 && color.a < 1.0) {
        specular = specular / max(color.a, 0.1);
    }
    rgb = rgb + spec_color * specular;

    // Back to gamma; fog in gamma space.
    rgb = gam(rgb);
    // Baked shading: a lightmap on the second coordinates, else the vertex
    // colors, their shadows eased as the material says.
    if (fx_lightmap) {
        rgb = rgb * textureSampleLevel(tex_mask, samp0, in.uv1, 0.0).rgb;
    } else if (fx_colors) {
        let lit = min(in.color.rgb * (max(draw.effect.z, 0.0) + 1.0), vec3<f32>(1.0));
        rgb = rgb * min(lit + max(draw.effect.w, 0.0), vec3<f32>(1.0));
    }
    if (frame.fog.x > 0.5) {
        let f = clamp((-in.pos_view.z - frame.fog.y) * frame.fog.w, 0.0, 1.0);
        rgb = mix(rgb, frame.fog_color.rgb, f);
    }
    return vec4<f32>(color_clamp(rgb), color.a);
}
