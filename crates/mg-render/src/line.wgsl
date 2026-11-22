// Lines (the area's tile grid, and Sound Ranges' spheres as copies of a
// shape's lines): unlit, in gamma space, hidden behind the meshes in front
// of them but drawn over the ground they lie on. The grid fades with the
// fog, as the ground does, down to `grid_least`; the spheres don't.

// (`Frame`: `frame.wgsl`, put before this source.)

@group(0) @binding(0) var<uniform> frame: Frame;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    // How far in front of the camera (view space), metres: the fog's
    // measure, as the lit shader's.
    @location(1) depth: f32,
};

// What is left of a grid line's opacity in the thickest fog (its end and
// past it): the renderer's `GRID_LEAST_IN_FOG` (a test holds them equal),
// whose comment says why it is what it is.
const grid_least: f32 = 0.3;

// A line's end where it is drawn.
fn placed(pos: vec3<f32>, color: vec4<f32>) -> VertexOut {
    var out: VertexOut;
    // A little toward the camera along its ray (the same place on screen),
    // in proportion to the distance as depth precision goes, so the ground
    // a line lies on doesn't hide it.
    let p = frame.view * vec4<f32>(pos, 1.0);
    out.clip = frame.proj * vec4<f32>(p.xyz * 0.995, 1.0);
    out.color = color;
    out.depth = -p.z;
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

// The grid: as faint as the fog makes the ground there, but never fainter
// than `grid_least` of itself (still to be counted by, no longer standing
// out of the fog as if hung in the sky). The same measure of the fog as
// the lit shader's (linear from its start to its end).
@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var alpha = in.color.a;
    if (frame.fog.x > 0.5) {
        let f = clamp((in.depth - frame.fog.y) * frame.fog.w, 0.0, 1.0);
        alpha = alpha * mix(1.0, grid_least, f);
    }
    return vec4<f32>(in.color.rgb, alpha);
}

// Sound Ranges' spheres: an aid to what is heard, not ground: as they are,
// whatever the fog.
@fragment
fn fs_plain(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
