// Lines (the area's tile grid): unlit, in gamma space, hidden behind the
// meshes in front of them but drawn over the ground they lie on.

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

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// A line's end where it is drawn.
fn placed(pos: vec3<f32>, color: vec4<f32>) -> VertexOut {
    var out: VertexOut;
    // A little toward the camera along its ray (the same place on screen),
    // in proportion to the distance as depth precision goes, so the ground
    // a line lies on doesn't hide it.
    let p = frame.view * vec4<f32>(pos, 1.0);
    out.clip = frame.proj * vec4<f32>(p.xyz * 0.995, 1.0);
    out.color = color;
    return out;
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    return placed(v.pos, v.color);
}

// A copy of a shape's lines (`Scene::line_shapes`): its ends moved and
// scaled, then as the lines above.
struct CopyIn {
    @location(0) pos: vec3<f32>,
    @location(1) offset_scale: vec4<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_copy(v: CopyIn) -> VertexOut {
    return placed(v.offset_scale.xyz + v.pos * v.offset_scale.w, v.color);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
