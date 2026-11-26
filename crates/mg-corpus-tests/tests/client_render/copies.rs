//! What the framebuffer's copies hold in the client, and when they are
//! made (the deferred list's fountain and "The framebuffer's copies").

use super::batch5::{base_game, patched, program, with_skybox};
use super::readings::{INVISIBLE, Reading, placeables_with};
use super::*;

/// A card of [`ORDER_CARDS`].
type Card = (&'static str, [u8; 4], &'static str, Option<&'static str>, &'static str);

/// The cards standing through [`reading_copy_order`]'s water, west to
/// east in each half: their name, color (opaque but for `alpha`), what
/// their texture is (`tga`, a 32-bit TGA; `dds`, a standard DDS, DXT5,
/// every texel opaque) and their MTR's lines (`None`: no MTR, the model's
/// `bitmap` the texture) and their mesh's other lines.
const ORDER_CARDS: [Card; 6] = [
    ("solid", [255, 0, 0, 255], "tga", None, ""),
    ("alpha", [0, 255, 0, 128], "tga", None, ""),
    ("dxt5", [0, 0, 255, 255], "dds", None, ""),
    ("hint", [255, 255, 0, 255], "tga", None, "  transparencyhint 1\n"),
    ("mtrtr", [0, 255, 255, 255], "tga", Some("transparency 1\n"), ""),
    ("sf2", [255, 0, 255, 255], "tga", Some("transparency 1\nsample_framebuffer 2\n"), ""),
];

/// A card's fragment program that paints the framebuffer's colour copy at
/// its own pixel, halved, plus half of red: where the copy is the frame
/// before's picture (the card's own last drawing in it), it grows to pure
/// red over the frames; where it is taken before the card in the same
/// frame, it is half the picture behind the card, reddened.
const FEEDBACK_FS: &str = r#"
#define SHADER_TYPE 2
#define FOG 0
#define KEYHOLING 0
#define LIGHTING 0
#define NORMAL_MAP 0
#define SPECULAR_MAP 0
#define ROUGHNESS_MAP 0
#define HEIGHT_MAP 0
#define SELF_ILLUMINATION_MAP 0
#define ENVIRONMENT_MAP 0
#include "inc_framebuffer"
#include "inc_standard"
void main ()
{
    FragmentColor = vec4(1.0);
    SetupStandardShaderInputs();
    vec2 uv = vec2(gl_FragCoord.x * invScreenWidth, gl_FragCoord.y * invScreenHeight);
    vec3 c = texture2D(texFBColor, uv).rgb;
    gl_FragColor = vec4(c * 0.5 + vec3(0.5, 0.0, 0.0), 1.0);
}
"#;

/// The feedback cards' MTR lines ([`FEEDBACK_FS`]), west to east.
const FEEDBACK_CARDS: [(&str, &str); 3] = [
    ("mgfd_solid", ""),
    ("mgfd_tr", "transparency 1\n"),
    ("mgfd_sf2", "transparency 1\nsample_framebuffer 2\n"),
];

/// A quad from `(x0, y0, z0)` to `(x1, y1, z1)` as a node of `parent`:
/// standing (y0 = y1, facing south) or lying (z0 = z1, facing up).
pub(super) fn quad(
    name: &str,
    parent: &str,
    bitmap: &str,
    extra: &str,
    a: [f32; 3],
    b: [f32; 3],
) -> String {
    let [x0, y0, z0] = a;
    let [x1, y1, z1] = b;
    let verts = if (z0 - z1).abs() < 1e-6 {
        format!("    {x0} {y0} {z0}\n    {x1} {y0} {z0}\n    {x1} {y1} {z0}\n    {x0} {y1} {z0}\n")
    } else {
        format!("    {x0} {y0} {z0}\n    {x1} {y0} {z0}\n    {x1} {y0} {z1}\n    {x0} {y0} {z1}\n")
    };
    let _ = y1;
    format!(
        "node trimesh {name}\n  parent {parent}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n\
         {extra}  render 1\n  shadow 0\n  verts 4\n{verts}  tverts 4\n    0 0 0\n    1 0 0\n    \
         1 1 0\n    0 1 0\n  faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n"
    )
}

/// An 8×8 picture of one colour.
fn flat(c: [u8; 4]) -> mg_image::Rgba {
    mg_image::Rgba { width: 8, height: 8, data: c.repeat(64) }
}

/// Which picture each framebuffer copy holds in the client (the deferred
/// list's fountain entry and "The framebuffer's copies"): ttr01 (a
/// skybox) with a placeable north of the player of two pools, each 4.5 m
/// by 1.9 m at 1 m above the ground, of `ttr01_water01`: the west one a
/// solid mesh (as The Domains of Dread's fountain's), the east one with
/// `transparencyhint 1` (as ttr01's water tiles'). Six cards stand through
/// each pool, 2 m high ([`ORDER_CARDS`]: opaque, with alpha 0.5 texels, a
/// standard DXT5 of opaque texels, `transparencyhint 1`, an MTR's
/// `transparency 1`, and its `sample_framebuffer 2`), so that their lower
/// halves are under the water. `fswater` is the game's, patched to paint
/// the copy it reads at its own pixel (`MG_COPY_ORDER=color`, the default)
/// or the depth under the water it reads from the depth copy
/// (`MG_COPY_ORDER=depth`: `fDepthOffset` / 2 m as gray): a card drawn
/// before the water's copy shows in its color there, and is near in its
/// depth. South of the pools three cards of [`FEEDBACK_FS`] (solid, MTR
/// `transparency 1`, `sample_framebuffer 2`), each with an additive green
/// card (64 of 255) over its east half: whether a card's copy is the frame
/// before's (red over its west half, (255, 128 or 64, 0) over its east,
/// as the copy was taken after the additive card or before it) or taken
/// in the frame before it is drawn. `MG_COPY_ORDER=particles`: the stock
/// `fswater`, no cards, and a white still particle 4.5 m wide (its softness
/// 2.2) over each pool, its middle at the water's surface: whether a soft
/// particle fades where it meets the water (its depth copy holding the
/// water) or not; `front`, see below. Moonglow's area view is drawn beside
/// with the same program. Pictures in
/// `target/test-output/client_reading_copy_order_<paint>/`.
///
/// Seen (2026-11-26, 89.8193.37): under both pools, solid and
/// `transparencyhint 1` alike, the water's colour copy and depth copy hold
/// the solid card, the card of alpha 0.5 texels, the standard DXT5, the
/// `transparencyhint 1` card and the MTR's `transparency 1` card; not the
/// `sample_framebuffer 2` card. So the client copies for its water after
/// the see-through meshes, before the water and the `sample_framebuffer`
/// meshes. Moonglow copied after the solid meshes only: the DXT5 and
/// `transparencyhint` cards (see-through to it) were missing from its
/// copies, as The Domains of Dread's fountain's basin was (fixed: the copy
/// after the see-through meshes' solid parts). The solid and
/// `transparency 1` feedback cards are pure red on their west halves
/// (254, 0, 0) and (255, 47, 3) on their east, under the additive card:
/// the frame before's picture, the additive card in it (it adds 24 of
/// green, the `sample_framebuffer` card's halves apart; twice that is the
/// fixed point of a copy that holds it). The `sample_framebuffer 2` card
/// reads this frame's copy, (159, 30, 17), half the ground and red, the
/// additive card not in it. Moonglow's three read this frame's copy. With
/// `particles`, the particles end at the water's surface without fading
/// in the client as in Moonglow: their depth copy holds no water.
/// The "alpha" card, a 32-bit TGA of alpha 0.5 texels, is drawn opaque
/// in both. `MG_COPY_ORDER=front`: magenta cards with `transparencyhint
/// 1` standing in front of each pool, a standard DXT5 of alpha 0.5
/// texels (soft) and an opaque one, `fswater` painting its copy's red and
/// blue with a green of 255. In the client the water is not drawn behind
/// the soft card (green 39 there), as though it wrote its depth; Moonglow
/// lays its soft part over the water (green 132): the deferred list's
/// "A see-through mesh's soft part and shiny water".
#[test]
#[ignore]
fn reading_copy_order() {
    let paint = std::env::var("MG_COPY_ORDER").unwrap_or_else(|_| "color".into());
    let base = base_game();
    let water_fs = program(&base, "fswater");
    let find = "\tgl_FragColor = FragmentColor;\n}";
    let with = match paint.as_str() {
        "depth" => "\tgl_FragColor = vec4(vec3(clamp(fDepthOffset * 0.5, 0.0, 1.0)), 1.0);\n}",
        "front" => {
            "\tvec3 copy = texture2D(texFBColor, vScreenCoords).rgb;\n\
             \tgl_FragColor = vec4(copy.r, 1.0, copy.b, 1.0);\n}"
        }
        _ => "\tgl_FragColor = vec4(texture2D(texFBColor, vScreenCoords).rgb, 1.0);\n}",
    };
    let water_fs = patched(&water_fs, find, with);
    let particles = paint == "particles";
    let mut overrides: Vec<(String, Vec<u8>)> = vec![
        ("mgfdvs.shd".into(), program(&base, "vslit").into_bytes()),
        ("mgfdfs.shd".into(), FEEDBACK_FS.as_bytes().to_vec()),
        ("mgwhite.tga".into(), super::white_tga()),
        ("mgco_add.tga".into(), mg_image::write_tga(&flat([0, 64, 0, 255]))),
        ("mgco_add.txi".into(), b"blending additive\n".to_vec()),
    ];
    if !particles {
        overrides.push(("fswater.shd".into(), water_fs.into_bytes()));
    }
    let mut nodes = Vec::new();
    let parent = "mg_copyord";
    for (half, x0, hint) in [("w", -4.6f32, ""), ("e", 0.1, "  transparencyhint 1\n")] {
        let name = format!("pool_{half}");
        nodes.push(quad(
            &name,
            parent,
            "ttr01_water01",
            hint,
            [x0, -1.6, 1.0],
            [x0 + 4.5, 0.3, 1.0],
        ));
        if particles {
            // (A white particle 4.5 m wide, its softness 2.2, half under the
            // water: does it fade where it meets the water's surface?)
            let x = x0 + 2.25;
            let one = super::soft_particles::one(
                &format!("soft_{half}"),
                [x, -0.6, 1.0],
                (4.5, 4.5),
                "Normal",
                "Normal",
                [1.0, 1.0, 1.0],
            );
            nodes.push(one.replace("parent mg_soft", &format!("parent {parent}")));
            continue;
        }
        if paint == "front" {
            // (Magenta cards with `transparencyhint 1`, of alpha 0.5
            // texels (soft, as a road decal's) and opaque ones, standing
            // in front of the pool, seen over it: is the water drawn over
            // them?)
            for (i, (card, alpha)) in [("soft", 128), ("hint", 255)].into_iter().enumerate() {
                let hint = "  transparencyhint 1\n";
                let texture = format!("mgcf_{card}");
                let picture = flat([255, 0, 255, alpha]);
                // (A standard DXT5, as ttr01's grass and decals: a TGA's
                // alpha the client draws opaque here.)
                let t = mg_image::encode::compress(&picture, mg_image::Format::Bc3, true, false);
                let dds = mg_image::dds::write_standard(&t).unwrap();
                overrides.push((format!("{texture}.dds"), dds));
                let x = x0 + 0.5 + i as f32 * 2.0;
                let node = format!("front_{half}_{card}");
                nodes.push(quad(
                    &node,
                    parent,
                    &texture,
                    hint,
                    [x, -2.0, 0.8],
                    [x + 1.2, -2.0, 2.4],
                ));
            }
            continue;
        }
        for (i, (card, color, kind, mtr, extra)) in ORDER_CARDS.iter().enumerate() {
            let x = x0 + 0.1 + i as f32 * 0.75;
            let texture = format!("mgco_{card}");
            let picture = flat(*color);
            match *kind {
                "dds" => {
                    let t =
                        mg_image::encode::compress(&picture, mg_image::Format::Bc3, true, false);
                    let dds = mg_image::dds::write_standard(&t).unwrap();
                    overrides.push((format!("{texture}.dds"), dds));
                }
                _ => overrides.push((format!("{texture}.tga"), mg_image::write_tga(&picture))),
            }
            let bitmap = match mtr {
                Some(lines) => {
                    let m = format!("mgcm_{card}");
                    overrides.push((
                        format!("{m}.mtr"),
                        format!("texture0 {texture}\n{lines}").into_bytes(),
                    ));
                    m
                }
                None => texture,
            };
            let node = format!("card_{half}_{card}");
            nodes.push(quad(&node, parent, &bitmap, extra, [x, 0.0, 0.0], [x + 0.6, 0.0, 2.0]));
        }
    }
    let feedback = !particles && paint != "front";
    for (i, (name, lines)) in FEEDBACK_CARDS.iter().enumerate().filter(|_| feedback) {
        let x = -3.0 + i as f32 * 2.0;
        let mtr =
            format!("texture0 mgwhite\ncustomshadervs mgfdvs\ncustomshaderfs mgfdfs\n{lines}");
        overrides.push((format!("{name}.mtr"), mtr.into_bytes()));
        nodes.push(quad(name, parent, name, "", [x, -3.0, 0.0], [x + 1.2, -3.0, 1.2]));
        let add = format!("{name}_add");
        nodes.push(quad(&add, parent, "mgco_add", "", [x + 0.6, -3.3, 0.0], [x + 1.2, -3.3, 1.2]));
    }
    let model = super::batch4::probe_model(parent, &nodes);
    let (table, first) = placeables_with(&base, &[(parent, 0)]);
    overrides.push((format!("{parent}.mdl"), model.into_bytes()));
    overrides.push(("placeables.2da".into(), table.into_bytes()));
    let overrides: Vec<(&str, Vec<u8>)> =
        overrides.iter().map(|(f, d)| (f.as_str(), d.clone())).collect();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new(&format!("copy_order_{paint}"), "ttr01", light, &overrides);
    r.are.root.set("SkyBox", Value::Byte(1));
    r.place(first, [20.0, 28.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = r.shoot_beside(INVISIBLE, &with_skybox(SETTINGS), &|_| {});
    eprintln!("{}", dir.display());
}
