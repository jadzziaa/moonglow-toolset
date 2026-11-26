// Lines (the area's tile grid, Sound Ranges' spheres as copies of a
// shape's lines, and the boxes at the end): unlit, in gamma space, hidden behind the meshes in front
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

// Boxes (`Scene::boxes`, the area view's object and selection boxes): the
// unit cube's twelve edges, each a quad of two triangles, drawn for every
// copy, over everything (no depth test). Each edge's quad is widened on
// screen to the copy's width in pixels, with a soft edge a pixel wide (a
// pixel's coverage of the line: egui's feathering), and cut where it
// passes behind the near plane.

struct BoxIn {
    // The edge's two ends on the unit cube; `a.w` which of them this
    // corner of its quad is at (0 or 1), `b.w` its side (-1 or 1).
    @location(0) a: vec4<f32>,
    @location(1) b: vec4<f32>,
    // From the unit cube to the box as posed.
    @location(2) t0: vec4<f32>,
    @location(3) t1: vec4<f32>,
    @location(4) t2: vec4<f32>,
    @location(5) t3: vec4<f32>,
    @location(6) line_color: vec4<f32>,
    @location(7) edge_color: vec4<f32>,
    // The line's width and the edge's, pixels (0: none).
    @location(8) widths: vec2<f32>,
};

struct BoxOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    // Pixels across the line from its middle, and along it from its first
    // end; the line's length on screen and the half of its width that is
    // solid (all four in pixels).
    @location(1) at: vec4<f32>,
};

// Nothing drawn: the quad's corners all in one place, off the picture.
fn no_box() -> BoxOut {
    var out: BoxOut;
    out.clip = vec4<f32>(2.0, 2.0, 0.0, 1.0);
    out.color = vec4<f32>(0.0);
    out.at = vec4<f32>(0.0);
    return out;
}

fn box_corner(v: BoxIn, width: f32, color: vec4<f32>) -> BoxOut {
    if (!(width > 0.0)) {
        return no_box();
    }
    let t = mat4x4<f32>(v.t0, v.t1, v.t2, v.t3);
    let to_clip = frame.proj * frame.view * t;
    var a = to_clip * vec4<f32>(v.a.xyz, 1.0);
    var b = to_clip * vec4<f32>(v.b.xyz, 1.0);
    // Cut at the near plane (wgpu's clip space: depth 0 there, below 0
    // nearer and behind the eye), where the edge crosses it.
    if (a.z < 0.0 && b.z < 0.0) {
        return no_box();
    }
    if (a.z < 0.0) {
        a = mix(a, b, a.z / (a.z - b.z));
    } else if (b.z < 0.0) {
        b = mix(b, a, b.z / (b.z - a.z));
    }
    // On screen, in pixels (y up, as clip space has it).
    let size = vec2<f32>(frame.light_params.z, frame.light_params.w);
    let sa = a.xy / max(a.w, 1e-6) * 0.5 * size;
    let sb = b.xy / max(b.w, 1e-6) * 0.5 * size;
    let d = sb - sa;
    let len = sqrt(dot(d, d));
    // (An edge seen end on: a dot as wide as the line.)
    var along = vec2<f32>(1.0, 0.0);
    if (len > 1e-4) {
        along = d / len;
    }
    let across = vec2<f32>(-along.y, along.x);
    // Under a pixel wide: a pixel wide, as faint as it is narrower.
    let w = max(width, 1.0);
    let reach = w * 0.5 + 0.5;
    let end = v.a.w;
    let side = v.b.w;
    let ahead = (end * 2.0 - 1.0) * 0.5;
    let p = mix(sa, sb, end) + along * ahead + across * side * reach;
    var out: BoxOut;
    out.clip = vec4<f32>(p / (0.5 * size), 0.0, 1.0);
    out.color = vec4<f32>(color.rgb, color.a * min(width, 1.0));
    out.at = vec4<f32>(side * reach, end * len + ahead, len, w * 0.5);
    return out;
}

// The copies' edges (where they have one), drawn first.
@vertex
fn vs_box_edge(v: BoxIn) -> BoxOut {
    return box_corner(v, v.widths.y, v.edge_color);
}

// The copies' lines, over every edge.
@vertex
fn vs_box_line(v: BoxIn) -> BoxOut {
    return box_corner(v, v.widths.x, v.line_color);
}

// How much of the pixel the line covers: across it and at its two ends.
@fragment
fn fs_box(in: BoxOut) -> @location(0) vec4<f32> {
    let across = clamp(in.at.w + 0.5 - abs(in.at.x), 0.0, 1.0);
    let ends = clamp(0.5 + in.at.y, 0.0, 1.0) * clamp(0.5 + in.at.z - in.at.y, 0.0, 1.0);
    return vec4<f32>(in.color.rgb, in.color.a * across * ends);
}
