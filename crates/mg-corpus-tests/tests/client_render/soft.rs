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
    // (A run's own folder: `MG_SOFT_TAG` is added to its name.)
    let tag = std::env::var("MG_SOFT_TAG").map(|t| format!("_{t}")).unwrap_or_default();
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
    let mut r = Reading::new(&format!("soft_{kind_name}{tag}"), "ttr01", light, &overrides);
    r.are.root.set("SkyBox", Value::Byte(1));
    r.place(first, [20.0, 28.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    // (`MG_SOFT_GRASS=1`: the client's grass grown too, as Moonglow's view
    // grows it, over and around the decals.)
    let mut settings = with_skybox(SETTINGS);
    if std::env::var_os("MG_SOFT_GRASS").is_some() {
        settings =
            settings.replace("[graphics.grass]\n\t\tmode = 0", "[graphics.grass]\n\t\tmode = 2");
        assert!(settings.contains("mode = 2"), "the grass's setting");
    }
    let (dir, _) = r.shoot_beside(INVISIBLE, &settings, &|_| {});
    eprintln!("{}", dir.display());
}

/// The alphas of [`reading_punch_cut`]'s bands, west to east: around the
/// client's cut (0.2: 51) closely, then in tenths.
const PUNCH_LEVELS: [u8; 16] =
    [0, 13, 26, 38, 46, 50, 52, 56, 64, 77, 102, 128, 153, 179, 230, 255];

/// Where the client cuts a texture of TXI `blending punchthrough` on a
/// mesh made see-through, and which TXI makes a mesh a cut-out: ttr01
/// (a skybox), a placeable north of the player, ten standing cards side
/// by side, 1.7 m wide, textured with 16 bands of alpha [`PUNCH_LEVELS`],
/// magenta and orange in turn (a DXT5), a white bar 0.8 m behind each,
/// lower, seen through its middle ([`PUNCH_CARDS`] has what each is).
/// Moonglow's area view is drawn beside it, and each card read as solid
/// or blended in the three pictures (a window across its middle, above
/// the bar). Pictures in `target/test-output/client_reading_punch_cut/`.
///
/// Read 2026-11-27 (game 89.8193.37): every card is cut where its
/// alpha is 50/255 or less and kept from 52/255 (the 0.2 of
/// `fAlphaDiscardValue`); a cut-out's kept bands are drawn opaque,
/// hiding the bar, darker as their alpha rises (the light through it
/// weighed by its alpha), and a blended card's are blended. The client
/// takes a mesh to be a cut-out by the TXI of the name it binds (its
/// bitmap): the texture's own TXI with a bitmap naming the texture
/// (with `materialname` too), the MTR's (`name.txi` beside `name.mtr`)
/// with a bitmap naming an MTR, whose texture's TXI it does not read for
/// it; `transparencyhint 1` and the MTR's `transparency 1` change nothing.
/// Before, Moonglow blended the hint's card and the MTR-named TXI's, and
/// drew solid those whose MTR's texture had the TXI.
#[test]
#[ignore]
fn reading_punch_cut() {
    let base = base_game();
    let mut overrides: Vec<(String, Vec<u8>)> = vec![("mgwhite.tga".into(), super::white_tga())];
    let mut data = Vec::new();
    for _ in 0..8 {
        for (i, a) in PUNCH_LEVELS.into_iter().enumerate() {
            let [r, g, b] = if i % 2 == 0 { [255, 0, 255] } else { [255, 160, 0] };
            data.extend([r, g, b, a].repeat(8));
        }
    }
    let picture = mg_image::Rgba { width: 8 * PUNCH_LEVELS.len() as u32, height: 8, data };
    let t = mg_image::encode::compress(&picture, mg_image::Format::Bc3, true, false);
    let dds = mg_image::dds::write_standard(&t).unwrap();
    const PUNCH: &str = "blending punchthrough\n";
    const HINT: &str = "  transparencyhint 1\n";
    // (name, the TXI's under the texture's name and under the MTR's, an
    // MTR's lines after its texture, the mesh's lines)
    let kinds = [
        ("mgpc_a", (true, false), None, ""),
        ("mgpc_b", (true, false), None, HINT),
        ("mgpc_c", (true, false), Some("transparency 1\n"), ""),
        ("mgpc_d", (false, false), None, HINT),
        ("mgpc_e", (true, false), Some(""), ""),
        ("mgpc_f", (true, false), Some("transparency 1\n"), HINT),
        ("mgpc_g", (false, true), Some(""), ""),
        ("mgpc_h", (true, true), Some(""), ""),
        ("mgpc_i", (false, true), Some("transparency 1\n"), ""),
        ("mgpc_j", (true, false), Some(""), "  materialname mgpc_jm\n"),
    ];
    let parent = "mg_punch";
    let mut nodes = Vec::new();
    for (i, (name, (txi, mtr_txi), mtr, extra)) in kinds.into_iter().enumerate() {
        overrides.push((format!("{name}.dds"), dds.clone()));
        if txi {
            overrides.push((format!("{name}.txi"), PUNCH.as_bytes().to_vec()));
        }
        if mtr_txi {
            overrides.push((format!("{name}m.txi"), PUNCH.as_bytes().to_vec()));
        }
        // (The MTR named by the bitmap, or by `materialname` beside the
        // texture's.)
        if let Some(lines) = mtr {
            let m = format!("{name}m");
            overrides.push((format!("{m}.mtr"), format!("texture0 {name}\n{lines}").into_bytes()));
        }
        let bitmap = match mtr {
            Some(_) if !extra.contains("materialname") => format!("{name}m"),
            _ => name.to_string(),
        };
        let x = -9.5 + i as f32 * 1.9;
        let card = [([x, 0.0, 0.4], [x + 1.7, 0.0, 2.0], 0.0, 1.0)];
        nodes.push(quads(&format!("card{i}"), parent, &bitmap, extra, &card));
        let bar = [([x - 0.1, 0.8, 0.3], [x + 1.8, 0.8, 0.9], 0.0, 1.0)];
        nodes.push(quads(&format!("bar{i}"), parent, "mgwhite", "", &bar));
    }
    let model = super::batch4::probe_model(parent, &nodes);
    let (table, first) = placeables_with(&base, &[(parent, 0)]);
    overrides.push((format!("{parent}.mdl"), model.into_bytes()));
    overrides.push(("placeables.2da".into(), table.into_bytes()));
    let overrides: Vec<(&str, Vec<u8>)> =
        overrides.iter().map(|(f, d)| (f.as_str(), d.clone())).collect();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("punch_cut", "ttr01", light, &overrides);
    r.are.root.set("SkyBox", Value::Byte(1));
    r.place(first, [20.0, 22.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, client) = r.shoot_beside(INVISIBLE, &with_skybox(SETTINGS), &|_| {});
    // (Each card's middle, above the bar, in a 1024 by 768 window: its
    // saturated texels, drawn opaque, against its blended ones.)
    let solid = |p: &Rgba| -> Vec<bool> {
        (0..kinds.len())
            .map(|i| {
                let centre = 150.0 + 82.8 * i as f32;
                let (mut opaque, mut blended) = (0, 0);
                for x in (centre as u32 - 12)..(centre as u32 + 12) {
                    let at = 4 * (310 * p.width + x) as usize;
                    let [r, g, b] = [p.data[at], p.data[at + 1], p.data[at + 2]];
                    if (r > 100 && b > 100 && g < 80) || (r > 100 && g > 50 && b < 60) {
                        opaque += 1;
                    } else if r > b && b > g {
                        blended += 1;
                    }
                }
                opaque > 2 * blended
            })
            .collect()
    };
    let read = solid(&client);
    eprintln!("{}\nthe client's cards solid: {read:?}", dir.display());
    let want: Vec<bool> = PUNCH_CARDS.iter().map(|c| c.1).collect();
    assert_eq!(read, want, "the client's cut-outs");
    for name in ["moonglow.png", "moonglow_on.png"] {
        let path = dir.join(name);
        if path.exists() {
            let ours = solid(&crate::common::read_png(&path));
            assert_eq!(ours, read, "{name}'s cut-outs against the client's");
        }
    }
}

/// [`reading_punch_cut`]'s cards, west to east: what each is, and
/// whether the client draws it solid (a cut-out).
const PUNCH_CARDS: [(&str, bool); 10] = [
    ("a texture of TXI `blending punchthrough`", true),
    ("the same, `transparencyhint 1`", true),
    ("an MTR of `transparency 1` naming it", false),
    ("`transparencyhint 1`, no TXI", false),
    ("an MTR naming it", false),
    ("an MTR of `transparency 1` naming it, `transparencyhint 1`", false),
    ("an MTR of the TXI naming a texture of none", true),
    ("an MTR of the TXI naming a texture of it", true),
    ("an MTR of the TXI and `transparency 1`, a texture of none", true),
    ("the texture of the TXI, an MTR by `materialname`", true),
];
