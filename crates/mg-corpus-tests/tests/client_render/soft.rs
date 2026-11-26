//! What the client does with a see-through mesh's soft texels (alpha
//! between its cut, 0.2, and opaque): whether they lay their depth, over
//! shiny water, a particle, another soft mesh and the floor (the deferred
//! list's "A see-through mesh's soft part and shiny water").

use super::batch5::{base_game, patched, program, with_skybox};
use super::readings::{INVISIBLE, Reading, placeables_with};
use super::*;

/// The alphas of the cards standing in front of the water and the
/// particle, west to east.
const LEVELS: [u8; 5] = [26, 77, 128, 179, 230];

/// A mesh of quads, each `(a, b, u0, u1)`: from `a` to `b` (standing,
/// y equal, facing south; or lying, z equal, facing up), its texture's
/// columns from `u0` to `u1`, drawn in the order given (its faces').
fn quads(
    name: &str,
    parent: &str,
    bitmap: &str,
    extra: &str,
    quads: &[([f32; 3], [f32; 3], f32, f32)],
) -> String {
    let (mut verts, mut tverts, mut faces) = (String::new(), String::new(), String::new());
    for (k, ([x0, y0, z0], [x1, y1, z1], u0, u1)) in quads.iter().enumerate() {
        if (z0 - z1).abs() < 1e-6 {
            verts += &format!(
                "    {x0} {y0} {z0}\n    {x1} {y0} {z0}\n    {x1} {y1} {z0}\n    {x0} {y1} {z0}\n"
            );
        } else {
            verts += &format!(
                "    {x0} {y0} {z0}\n    {x1} {y0} {z0}\n    {x1} {y0} {z1}\n    {x0} {y0} {z1}\n"
            );
        }
        tverts += &format!("    {u0} 0 0\n    {u1} 0 0\n    {u1} 1 0\n    {u0} 1 0\n");
        let v = 4 * k;
        faces += &format!(
            "    {} {} {} 1 {} {} {} 0\n    {} {} {} 1 {} {} {} 0\n",
            v,
            v + 1,
            v + 2,
            v,
            v + 1,
            v + 2,
            v,
            v + 2,
            v + 3,
            v,
            v + 2,
            v + 3
        );
    }
    let n = 4 * quads.len();
    format!(
        "node trimesh {name}\n  parent {parent}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n\
         {extra}  render 1\n  shadow 0\n  verts {n}\n{verts}  tverts {n}\n{tverts}  faces {}\n\
         {faces}endnode\n",
        2 * quads.len()
    )
}

/// How a reading's soft meshes are made see-through
/// (`MG_SOFT_KIND`): their texture's kind, the mesh's lines and whether
/// the soft alpha is the mesh's (`alpha`) rather than the texels'.
struct Kind {
    dds: bool,
    extra: &'static str,
    txi: Option<&'static str>,
    mtr: bool,
    mesh_alpha: bool,
}

fn kind(name: &str) -> Kind {
    let k = Kind {
        dds: true,
        extra: "  transparencyhint 1\n",
        txi: None,
        mtr: false,
        mesh_alpha: false,
    };
    match name {
        "dxt5" => Kind { extra: "", ..k },
        "tga" => Kind { dds: false, ..k },
        "punch" => Kind { txi: Some("blending punchthrough\n"), ..k },
        "mtr" => Kind { extra: "", mtr: true, ..k },
        "alpha" => Kind { extra: "", mesh_alpha: true, ..k },
        _ => k,
    }
}

/// A see-through mesh's soft texels in the client (`MG_SOFT_KIND`:
/// `hint`, the default, a standard DXT5 with `transparencyhint 1`, as
/// ttr01's grass and decals; `dxt5` without the hint; `tga`, a 32-bit TGA
/// with it; `punch`, the hint and a TXI `blending punchthrough`; `mtr`,
/// an MTR's `transparency 1`; `alpha`, opaque texels and the mesh's
/// `alpha` the level): ttr01 (a skybox), a placeable north of the
/// player, in four rows from the far one:
///
/// 1. west, a pool of `ttr01_water01` (as `copies::reading_copy_order`'s,
///    `fswater` painting its copy's red and blue with a green of 255) and
///    five magenta cards standing in front of it of alpha [`LEVELS`];
///    east, a white still particle (`Billboard_to_World_Z`, which no
///    softness fades), 4.4 m, with the same five cards in front of it;
/// 2. four pairs of standing cards of alpha 0.5, a magenta one in front
///    and a cyan one 0.4 m behind it, overlapping: one mesh with the front
///    quad's faces first, one with the back quad's first, two meshes with
///    the front one's node first, and with the back one's first;
/// 3. lying on the floor, two pairs of decals of alpha 0.5 overlapping,
///    the upper (4 cm) one's faces first and last, and three single ones
///    of alpha 0.3, 0.5 and 0.7.
///
/// Moonglow's area view is drawn beside it. Pictures in
/// `target/test-output/client_reading_soft_<kind>/`.
#[test]
#[ignore]
fn reading_soft_depth() {
    let kind_name = std::env::var("MG_SOFT_KIND").unwrap_or_else(|_| "hint".into());
    let k = kind(&kind_name);
    let base = base_game();
    let water_fs = program(&base, "fswater");
    let water_fs = patched(
        &water_fs,
        "\tgl_FragColor = FragmentColor;\n}",
        "\tvec3 copy = texture2D(texFBColor, vScreenCoords).rgb;\n\
         \tgl_FragColor = vec4(copy.r, 1.0, copy.b, 1.0);\n}",
    );
    let mut overrides: Vec<(String, Vec<u8>)> = vec![
        ("fswater.shd".into(), water_fs.into_bytes()),
        ("mgwhite.tga".into(), super::white_tga()),
    ];
    // A texture of `width` 8-texel columns, each a color of alpha `a`
    // (opaque with the mesh's alpha), as the kind says; its name to bind
    // (an MTR's for `mtr`).
    let mut texture = |name: &str, colors: &[[u8; 3]], a: u8| -> String {
        let a = if k.mesh_alpha { 255 } else { a };
        let w = 8 * colors.len() as u32;
        let mut data = Vec::new();
        for _ in 0..8 {
            for c in colors {
                data.extend([c[0], c[1], c[2], a].repeat(8));
            }
        }
        let picture = mg_image::Rgba { width: w, height: 8, data };
        if k.dds {
            let t = mg_image::encode::compress(&picture, mg_image::Format::Bc3, true, false);
            overrides.push((format!("{name}.dds"), mg_image::dds::write_standard(&t).unwrap()));
        } else {
            overrides.push((format!("{name}.tga"), mg_image::write_tga(&picture)));
        }
        if let Some(txi) = k.txi {
            overrides.push((format!("{name}.txi"), txi.as_bytes().to_vec()));
        }
        if k.mtr {
            let m = format!("{name}m");
            overrides.push((
                format!("{m}.mtr"),
                format!("texture0 {name}\ntransparency 1\n").into_bytes(),
            ));
            m
        } else {
            name.to_string()
        }
    };
    let extra = |a: u8| -> String {
        let alpha = if k.mesh_alpha {
            format!("  alpha {}\n", f32::from(a) / 255.0)
        } else {
            String::new()
        };
        format!("{}{alpha}", k.extra)
    };
    const MAGENTA: [u8; 3] = [255, 0, 255];
    const CYAN: [u8; 3] = [0, 255, 255];
    let parent = "mg_softd";
    let mut nodes = Vec::new();
    // 1. The pool and the particle, the cards in front of them.
    nodes.push(super::copies::quad(
        "pool",
        parent,
        "ttr01_water01",
        "",
        [-4.6, -1.6, 1.0],
        [-0.1, 0.3, 1.0],
    ));
    let particle = super::soft_particles::one(
        "light",
        [2.35, 0.0, 1.2],
        (4.4, 4.4),
        "Normal",
        "Billboard_to_World_Z",
        [1.0, 1.0, 1.0],
    );
    nodes.push(particle.replace("parent mg_soft", &format!("parent {parent}")));
    for (i, a) in LEVELS.into_iter().enumerate() {
        let bitmap = texture(&format!("mgsd_l{i}"), &[MAGENTA], a);
        for (half, x0) in [("w", -4.5f32), ("e", 0.2)] {
            let x = x0 + i as f32 * 0.9;
            let q = [([x, -2.0, 0.8], [x + 0.7, -2.0, 2.4], 0.0, 1.0)];
            nodes.push(quads(&format!("card_{half}{i}"), parent, &bitmap, &extra(a), &q));
        }
    }
    // 2. Pairs of standing cards; 3. pairs of decals and single ones.
    let pair = texture("mgsd_pair", &[MAGENTA, CYAN], 128);
    let (m, c) = ((0.0, 0.5), (0.5, 1.0));
    for (i, (front_first, one_mesh)) in
        [(true, true), (false, true), (true, false), (false, false)].into_iter().enumerate()
    {
        let x = -4.5 + i as f32 * 2.3;
        let front = ([x, -3.8, 0.0], [x + 1.4, -3.8, 1.4], m.0, m.1);
        let back = ([x + 0.5, -3.4, 0.4], [x + 1.9, -3.4, 1.8], c.0, c.1);
        let order = if front_first { [front, back] } else { [back, front] };
        if one_mesh {
            nodes.push(quads(&format!("pair{i}"), parent, &pair, &extra(128), &order));
        } else {
            for (j, q) in order.into_iter().enumerate() {
                nodes.push(quads(&format!("pair{i}_{j}"), parent, &pair, &extra(128), &[q]));
            }
        }
    }
    for (i, upper_first) in [true, false].into_iter().enumerate() {
        let x = -4.5 + i as f32 * 2.3;
        let upper = ([x, -6.0, 0.04], [x + 1.4, -5.0, 0.04], m.0, m.1);
        let lower = ([x + 0.5, -5.8, 0.02], [x + 1.9, -4.8, 0.02], c.0, c.1);
        let order = if upper_first { [upper, lower] } else { [lower, upper] };
        nodes.push(quads(&format!("decals{i}"), parent, &pair, &extra(128), &order));
    }
    for (i, a) in [77u8, 128, 179].into_iter().enumerate() {
        let bitmap = texture(&format!("mgsd_d{i}"), &[MAGENTA], a);
        let x = 0.2 + i as f32 * 1.5;
        let q = [([x, -6.0, 0.02], [x + 1.2, -5.0, 0.02], 0.0, 1.0)];
        nodes.push(quads(&format!("decal{i}"), parent, &bitmap, &extra(a), &q));
    }
    let model = super::batch4::probe_model(parent, &nodes);
    let (table, first) = placeables_with(&base, &[(parent, 0)]);
    overrides.push((format!("{parent}.mdl"), model.into_bytes()));
    overrides.push(("placeables.2da".into(), table.into_bytes()));
    let overrides: Vec<(&str, Vec<u8>)> =
        overrides.iter().map(|(f, d)| (f.as_str(), d.clone())).collect();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new(&format!("soft_{kind_name}"), "ttr01", light, &overrides);
    r.are.root.set("SkyBox", Value::Byte(1));
    r.place(first, [20.0, 28.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = r.shoot_beside(INVISIBLE, &with_skybox(SETTINGS), &|_| {});
    eprintln!("{}", dir.display());
}
