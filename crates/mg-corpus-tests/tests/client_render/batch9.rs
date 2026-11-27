//! The deferred list's "See-through meshes" entry, its last two items (the
//! ninth client batch, 2026-11-26): how the client makes the smaller
//! levels of an uncompressed texture with soft edges, and how it orders
//! static placeables' see-through meshes against each other. Each test
//! writes the client's picture (and Moonglow's beside it) to its scratch
//! directory and prints what it measured; what was seen is in its comment.
//!
//! Needs the game and the off-screen display; run by hand, one client at a
//! time:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render batch9 -- --ignored --nocapture --test-threads 1`.

use std::borrow::Cow;

use super::batch8d::{as_refs, project, reference_camera, solid, window};
use super::programs::{Quad, model};
use super::readings::{INVISIBLE, Reading, placeables_with};
use super::*;

/// A 32-bit TGA of `w` × `h` texels, `f(x, y)` each (RGBA; rows stored
/// bottom first, so y 0 is the texture's v 0).
pub(super) fn tga32(w: u16, h: u16, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let [wl, wh] = w.to_le_bytes();
    let [hl, hh] = h.to_le_bytes();
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, wl, wh, hl, hh, 32, 8];
    for y in 0..u32::from(h) {
        for x in 0..u32::from(w) {
            let [r, g, b, a] = f(x, y);
            t.extend([b, g, r, a]);
        }
    }
    t
}

// ---------------------------------------------------------------------------
// The smaller levels of an uncompressed texture.

/// The side of [`batch9_soft_edge_levels`]' textures, texels: over
/// Moonglow's `COMPRESSED_FROM`, so that its levels are made by the
/// encoder's `halve` where the device compresses, by `half` where not.
const LEVELS_SIDE: u16 = 512;

/// The transparent texels' color: green, which neither the opaque white
/// nor the ground has.
const UNDER: [u8; 3] = [0, 255, 0];

/// The soft edge: opaque white to u 0.4, its alpha falling to 0 by u 0.6,
/// transparent green from there.
fn edge_texel(x: u32, _y: u32) -> [u8; 4] {
    let u = (x as f32 + 0.5) / f32::from(LEVELS_SIDE);
    let a = ((0.6 - u) / 0.2).clamp(0.0, 1.0);
    if a > 0.0 {
        [255, 255, 255, (a * 255.0).round() as u8]
    } else {
        [UNDER[0], UNDER[1], UNDER[2], 0]
    }
}

/// A field of texels opaque white and transparent green in turn (a
/// one-texel checker): averaged by alpha, its levels are white of alpha
/// 0.5; averaged plainly, light green of alpha 0.5.
fn field_texel(x: u32, y: u32) -> [u8; 4] {
    if (x + y).is_multiple_of(2) { [255, 255, 255, 255] } else { [UNDER[0], UNDER[1], UNDER[2], 0] }
}

/// The cards of [`batch9_soft_edge_levels`]: name, texture, the middle of
/// its foot (x, y) and its side, metres. Two of each texture: one near
/// and large (its levels 2 and 3, counted from 0, read), one far and small (its
/// level 5).
const LEVEL_CARDS: [(&str, &str, [f32; 2], f32); 4] = [
    ("near_edge", "mg9_edge", [16.6, 22.5], 2.4),
    ("near_field", "mg9_field", [19.6, 22.5], 2.4),
    ("far_edge", "mg9_edge", [21.5, 32.0], 0.5),
    ("far_field", "mg9_field", [22.3, 32.0], 0.5),
];

/// How the client makes the smaller levels of an uncompressed texture
/// (a 32-bit TGA, 512 texels a side, no TXI) whose transparent texels have
/// a color of their own (green; the opaque ones white): whether a level's
/// color is the plain mean of the four texels above it (the green bleeding
/// into a soft edge seen from afar, a fringe) or the mean weighted by their
/// alpha (the edge white as near), as Moonglow makes them. Two textures on
/// standing cards (`alpha` 1, so the texture's alpha blends them): a soft
/// edge (opaque white, its alpha falling to 0 over a fifth of the width,
/// then transparent green) and a field of opaque white and transparent
/// green texels in turn (a one-texel checker), each on a large near card
/// and on a small far one (2.4 m at 2.5 m ahead of the player, 0.5 m at
/// 12 m). The field tells it in one number: averaged by alpha, its
/// levels are white of alpha 0.5; averaged plainly, light green
/// (127, 255, 127) of alpha 0.5; without levels, a noise of the two.
/// Printed: the mean color over each field, and across each edge.
///
/// Seen (2026-11-26, 89.8193.37): the client's levels are plain means.
/// The near field is light green over the ground, (124, 184, 119), and
/// the far one (111, 163, 107), where Moonglow's levels then, weighted by
/// alpha, drew them gray-white, (186, 184, 181) and (144, 137, 132). The
/// far edge seen from afar is as near, white falling to the ground over
/// the ramp, without a rim of its own (its levels' green under alpha
/// near 0). Moonglow since (`half`, `compress_plain_levels`): (125, 184,
/// 119) and (111, 160, 107), the edges' profiles as the client's
/// (`docs/findings.md`, "The client makes
/// an uncompressed texture's levels by plain means").
#[test]
#[ignore]
fn batch9_soft_edge_levels() {
    let _ = oracle_tool!("nwn_script_comp");
    let base = super::batch5::base_game();
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("mg9_edge.tga".into(), tga32(LEVELS_SIDE, LEVELS_SIDE, edge_texel)),
        ("mg9_field.tga".into(), tga32(LEVELS_SIDE, LEVELS_SIDE, field_texel)),
    ];
    let quads: Vec<Quad> = LEVEL_CARDS
        .iter()
        .map(|(name, texture, [x, y], side)| Quad {
            name: (*name).into(),
            at: [*x - 20.0, *y - 20.0, 0.0],
            corners: [
                [-side / 2.0, 0.0, 0.0],
                [side / 2.0, 0.0, 0.0],
                [side / 2.0, 0.0, *side],
                [-side / 2.0, 0.0, *side],
            ],
            bitmap: Cow::Borrowed(texture),
            extra: "",
            tverts: true,
            colors: false,
        })
        .collect();
    files.push(("mg9_levels.mdl".into(), model("mg9_levels", &quads).into_bytes()));
    let (table, first) = placeables_with(&base, &[("mg9_levels", 0)]);
    files.push(("placeables.2da".into(), table.into_bytes()));
    let light = Lighting { ambient: 0xFFFFFF, diffuse: 0x000000, main_light: 0, only_tile: None };
    let mut r = Reading::new("batch9_soft_edge_levels", "tcn01", light, &as_refs(&files));
    r.place(first, [20.0, 20.0, 0.0], 0.0, &[]);
    let dir = r.dir.clone();
    let (_, client) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    let pictures = pictures(&dir, &client);
    let camera = reference_camera();
    // (The middle of an edge card's white, opaque part, where the picture
    // shows it, against where the fitted camera puts it: the client's
    // picture is a few pixels off, which at a far card's 15 pixels puts a
    // window on the ground. The same shift is taken for the field beside
    // it.)
    let shift = |img: &Rgba, far: bool| -> (f32, f32) {
        let (_, _, [x, y], side) = LEVEL_CARDS[if far { 2 } else { 0 }];
        let p = |u: f32, v: f32| {
            project(
                &camera,
                Vec3::new(x - side / 2.0 + u * side, y, v * side),
                img.width,
                img.height,
            )
        };
        let ((x0, y1), (x1, y0)) = (p(0.0, 0.0), p(0.45, 1.0));
        let (cx, cy) = p(0.2, 0.5);
        let reach = 12.0 + (x1 - x0) * 0.5;
        let (mut sum, mut n) = ((0.0, 0.0), 0.0);
        for py in (y0 - reach) as i32..=(y1 + reach) as i32 {
            for px in (x0 - reach) as i32..=(x1 + reach) as i32 {
                let c = super::batch8d::pixel(img, px, py);
                if c[..3].iter().all(|&v| v > 215) {
                    sum = (sum.0 + px as f32, sum.1 + py as f32);
                    n += 1.0;
                }
            }
        }
        if n == 0.0 { (0.0, 0.0) } else { (sum.0 / n - cx, sum.1 / n - cy) }
    };
    for (name, _, [x, y], side) in LEVEL_CARDS {
        for (label, img) in &pictures {
            let (dx, dy) = shift(img, name.starts_with("far"));
            let at = |u: f32, v: f32| {
                let (px, py) = project(
                    &camera,
                    Vec3::new(x - side / 2.0 + u * side, y, v * side),
                    img.width,
                    img.height,
                );
                (px + dx, py + dy)
            };
            let (cx, cy) = at(0.5, 0.5);
            let (lx, _) = at(0.0, 0.5);
            let (rx, _) = at(1.0, 0.5);
            let px = rx - lx;
            if name.ends_with("field") {
                let r = ((px * 0.3) as i32).max(1);
                let m = window(img, cx, cy, r);
                eprintln!("{label:22} {name:10} ({px:5.1} px wide): mean {m:4.0?}");
            } else {
                let profile: Vec<String> = [0.3f32, 0.45, 0.5, 0.55, 0.6, 0.65, 0.75, 0.9]
                    .iter()
                    .map(|u| {
                        let (ux, uy) = at(*u, 0.5);
                        let c = window(img, ux, uy, 0);
                        format!("u {u:.2} {c:3.0?}")
                    })
                    .collect();
                eprintln!("{label:22} {name:10} ({px:5.1} px wide): {}", profile.join("  "));
            }
        }
    }
    eprintln!("{}", dir.display());
}

/// The pixels within `rect` (left, top, right, bottom), widened by
/// `margin` on each side, that have both red and blue in them (a card of
/// one drawn over one of the other where they overlap: the farther drawn
/// first, its color under the nearer's). Where the nearer was drawn first
/// the overlap is its color alone, and there are none. (Counted over the
/// pair's whole box rather than read at the projected overlap: the fitted
/// camera is a few pixels off the client's, enough to put a small window
/// outside a card.)
pub(super) fn mixed_pixels(img: &Rgba, rect: [f32; 4], margin: f32) -> usize {
    let (x0, y0) = ((rect[0] - margin).max(0.0) as u32, (rect[1] - margin).max(0.0) as u32);
    let x1 = ((rect[2] + margin) as u32).min(img.width - 1);
    let y1 = ((rect[3] + margin) as u32).min(img.height - 1);
    (y0..=y1)
        .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let [r, g, b, _] = img.pixel(x, y).map(i32::from);
            r - g > 25 && b - g > 25 && (r - b).abs() < 45
        })
        .count()
}

/// The client's picture and Moonglow's two beside it.
fn pictures(dir: &Path, client: &Rgba) -> Vec<(&'static str, Rgba)> {
    save_png(client, &dir.join("client_picture.png"));
    vec![
        ("client", client.clone()),
        ("moonglow (own shader)", common::read_png(&dir.join("moonglow.png"))),
        ("moonglow (game's)", common::read_png(&dir.join("moonglow_on.png"))),
    ]
}

// ---------------------------------------------------------------------------
// Static placeables' see-through meshes in order.

/// A set of [`batch9_see_through_statics`]: its code (in the models' and
/// textures' names), whether its placeables are static, the texture
/// names' set (another's, to share them), and whether the nearer card's
/// mesh stands 2 m before its model's origin (the origin then farther
/// than the farther card's).
struct Set {
    code: &'static str,
    statics: u8,
    textures: &'static str,
    /// The models' set (another's, to place its models).
    models: &'static str,
    offset: bool,
}

/// The sets: controls (dynamic and static, models and textures of their
/// own); the same with the nearer card's mesh before its origin; static
/// models whose textures a dynamic set's models (`td`) name as well; and
/// two pairs of sets of the same models placed static and dynamic
/// (`c1`/`c1d`, `c2`/`c2d`), whose dynamic pair stands one way round (the
/// red nearer for `c1d`, the blue for `c2d`).
const SETS: [Set; 10] = [
    Set { code: "dc", statics: 0, textures: "dc", models: "dc", offset: false },
    Set { code: "do", statics: 0, textures: "do", models: "do", offset: true },
    Set { code: "sc", statics: 1, textures: "sc", models: "sc", offset: false },
    Set { code: "so", statics: 1, textures: "so", models: "so", offset: true },
    Set { code: "ts", statics: 1, textures: "ts", models: "ts", offset: false },
    Set { code: "td", statics: 0, textures: "ts", models: "td", offset: false },
    Set { code: "c1", statics: 1, textures: "c1", models: "c1", offset: false },
    Set { code: "c2", statics: 1, textures: "c2", models: "c2", offset: false },
    Set { code: "c1d", statics: 0, textures: "c1", models: "c1", offset: false },
    Set { code: "c2d", statics: 0, textures: "c2", models: "c2", offset: false },
];

/// The pairs of [`batch9_see_through_statics`]: the set, the model (its
/// code: `c2d` places `c2`'s models dynamic), the nearer color and the
/// nearer card's mesh's place (x, y); the farther's is 0.5 m east and
/// north.
const ORDER_PAIRS: [(&str, &str, char, [f32; 2]); 17] = [
    ("dc", "dc", 'R', [14.0, 23.0]),
    ("dc", "dc", 'B', [16.4, 23.0]),
    ("do", "do", 'R', [18.8, 23.0]),
    ("do", "do", 'B', [21.2, 23.0]),
    ("c1d", "c1", 'R', [23.6, 23.0]),
    ("c2d", "c2", 'B', [26.0, 23.0]),
    ("sc", "sc", 'R', [14.0, 27.0]),
    ("sc", "sc", 'B', [16.4, 27.0]),
    ("so", "so", 'R', [18.8, 27.0]),
    ("so", "so", 'B', [21.2, 27.0]),
    ("ts", "ts", 'R', [23.6, 27.0]),
    ("ts", "ts", 'B', [26.0, 27.0]),
    ("c1", "c1", 'R', [14.0, 31.0]),
    ("c1", "c1", 'B', [16.4, 31.0]),
    ("c2", "c2", 'R', [18.8, 31.0]),
    ("c2", "c2", 'B', [21.2, 31.0]),
    ("td", "td", 'R', [23.6, 31.0]),
];

/// The nearer card's mesh's place before its model's origin, metres (y).
const ORIGIN_BEHIND: f32 = 2.0;

/// A card's model name: the set's code, the color and, in an offset
/// set, `o` where its mesh stands before its origin.
fn card_model(code: &str, color: char, offset: bool) -> String {
    format!("mg9{code}_{}{}", color.to_ascii_lowercase(), if offset { "o" } else { "" })
}

/// How the client orders static placeables' see-through meshes (each a
/// card of `alpha 0.5`, 1 m square, red or blue, standing facing the eye;
/// the nearer card of a pair 0.5 m nearer and 0.5 m west of the farther,
/// the red listed first): `batch8d_see_through_two_models` was first read
/// as static placeables whose models dynamic ones use as well not drawn
/// farthest first, where dynamic ones and static ones of models and
/// textures of their own were (a reading this batch corrected). It tells
/// apart:
///
/// - the instance's origin against its mesh's place (`do`, `so`: the
///   nearer card's mesh 2 m before its model's origin, so that by the
///   origins it is the farther), for dynamic and static placeables;
/// - a texture shared (`ts`: static models naming the textures that
///   dynamic placeables of other models, `td`, name) against a model
///   shared (`c1`, `c2`: static placeables of the models that dynamic ones
///   use too);
/// - whether a shared model's static placeables take the order of its
///   dynamic ones (`c1d` stands with the red nearer, `c2d` with the blue).
///
/// Where the cards overlap the color is the nearer's alone (the farther
/// drawn after it and hidden by its depth: "nearer first") or has the
/// farther's in it ("farther first"): the pixels of both colors are counted
/// over the pair's box, put by the meshes' places ([`mixed_pixels`]).
///
/// Seen (2026-11-26, 89.8193.37): dynamic placeables go by their
/// instances' origins, not their meshes' places: the `do` pairs drawn
/// nearer first, the nearer card's origin the farther (as Moonglow); the
/// other dynamic pairs farthest first. Static placeables are not put in
/// order by either: the red drawn first in all ten static pairs (`sc`,
/// `so`, `ts`, `c1`, `c2`, whichever was nearer), sharing a texture or
/// a model with a dynamic placeable or not, whichever way the dynamic
/// sharers stood (`batch8d_see_through_two_models`, read again: the blue
/// first in all eight of its static pairs). Moonglow puts static ones in
/// order by their origins, and differs in the five pairs with the red
/// nearer. Not told apart by the batch: a fixed order of the two
/// textures (or their buckets), or an order by a texture's (bucket's)
/// static meshes together: each texture's cards stand with the same mean
/// place in both batches.
#[test]
#[ignore]
fn batch9_see_through_statics() {
    let _ = oracle_tool!("nwn_script_comp");
    let base = super::batch5::base_game();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut rows: Vec<(String, u8)> = Vec::new();
    for set in &SETS {
        // (`c1d` and `c2d` place `c1`'s and `c2`'s models: no models or
        // textures of their own.)
        let own_models = set.models == set.code;
        for (color, rgba) in [('R', [255, 0, 0, 255]), ('B', [0, 0, 255, 255])] {
            let texture = format!("mg9{}_{}", set.textures, color.to_ascii_lowercase());
            if set.textures == set.code {
                files.push((format!("{texture}.tga"), solid(rgba)));
            }
            let variants: &[bool] = if set.offset { &[false, true] } else { &[false] };
            for &offset in variants {
                let name = card_model(set.models, color, offset);
                if own_models {
                    let quad = Quad {
                        name: "card".into(),
                        at: [0.0, if offset { -ORIGIN_BEHIND } else { 0.0 }, 0.0],
                        corners: [
                            [-0.5, 0.0, 0.0],
                            [0.5, 0.0, 0.0],
                            [0.5, 0.0, 1.0],
                            [-0.5, 0.0, 1.0],
                        ],
                        bitmap: Cow::Owned(texture.clone()),
                        extra: "  alpha 0.5\n",
                        tverts: true,
                        colors: false,
                    };
                    files.push((format!("{name}.mdl"), model(&name, &[quad]).into_bytes()));
                }
                rows.push((name, set.statics));
            }
        }
    }
    let list: Vec<(&str, u8)> = rows.iter().map(|(m, s)| (m.as_str(), *s)).collect();
    let (table, first) = placeables_with(&base, &list);
    files.push(("placeables.2da".into(), table.into_bytes()));
    let appearance = |name: &str, statics: u8| -> u32 {
        first + rows.iter().position(|(m, s)| m == name && *s == statics).unwrap() as u32
    };
    let light = Lighting { ambient: 0xA0A0A0, diffuse: 0x606060, main_light: 0, only_tile: None };
    let mut r = Reading::new("batch9_see_through_statics", "tcn01", light, &as_refs(&files));
    for (set_code, models, near, at) in ORDER_PAIRS {
        let set = SETS.iter().find(|s| s.code == set_code).unwrap();
        let statics = set.statics;
        for color in ['R', 'B'] {
            let nearer = color == near;
            let offset = set.offset && nearer;
            let mesh = if nearer { at } else { [at[0] + 0.5, at[1] + 0.5] };
            let origin = [mesh[0], mesh[1] + if offset { ORIGIN_BEHIND } else { 0.0 }];
            r.place(
                appearance(&card_model(models, color, offset), statics),
                [origin[0], origin[1], 0.0],
                0.0,
                &[("Static", Value::Byte(statics))],
            );
        }
    }
    let dir = r.dir.clone();
    let (_, client) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    let pictures = pictures(&dir, &client);
    let camera = reference_camera();
    for (set_code, _, near, at) in ORDER_PAIRS {
        let set = SETS.iter().find(|s| s.code == set_code).unwrap();
        let far_at = [at[0] + 0.5, at[1] + 0.5];
        let rect = |at: [f32; 2], img: &Rgba| {
            let pts = [(-0.5, 0.0), (0.5, 0.0), (0.5, 1.0), (-0.5, 1.0)].map(|(dx, z)| {
                project(&camera, Vec3::new(at[0] + dx, at[1], z), img.width, img.height)
            });
            let xs = pts.map(|p| p.0);
            let ys = pts.map(|p| p.1);
            [
                xs.iter().copied().fold(f32::MAX, f32::min),
                ys.iter().copied().fold(f32::MAX, f32::min),
                xs.iter().copied().fold(f32::MIN, f32::max),
                ys.iter().copied().fold(f32::MIN, f32::max),
            ]
        };
        for (side, img) in &pictures {
            let (n, f) = (rect(at, img), rect(far_at, img));
            let both = [n[0].min(f[0]), n[1].min(f[1]), n[2].max(f[2]), n[3].max(f[3])];
            let mixed = mixed_pixels(img, both, 6.0);
            // (The client's pairs count 0 or over 280; Moonglow's own edges
            // up to 30.)
            let farther_first = mixed > 100;
            let first = match (farther_first, near) {
                (false, c) => c,
                (true, 'R') => 'B',
                (true, _) => 'R',
            };
            let verdict = if farther_first { "farther first" } else { "nearer first" };
            eprintln!(
                "{side:22} {set_code:3} {} offset {:5} nearer {near}: {mixed:4} pixels of both \
                 colors: {verdict} ({first} first)",
                if set.statics == 1 { "static " } else { "dynamic" },
                set.offset,
            );
        }
    }
    eprintln!("{}", dir.display());
}

// ---------------------------------------------------------------------------
// What decides which static see-through mesh is drawn first.

/// A set of [`batch9_static_order`]: its code; the red card's and the blue
/// card's model and texture names (a set's own); the lone card (its color
/// and how far north of the pairs, metres; south when negative), if any;
/// whether the lone card is placed first in the area's list (else last);
/// whether each pair lists its blue card first (else its red); the place
/// of the set (its first pair's nearer card, x and y).
struct OrderSet {
    code: &'static str,
    red: (&'static str, &'static str),
    blue: (&'static str, &'static str),
    lone: Option<(char, f32)>,
    lone_first: bool,
    blue_listed_first: bool,
    at: [f32; 2],
}

/// The sets of [`batch9_static_order`]. Each has two pairs (the red nearer
/// in the first, the blue in the second, 2.2 m east), so that a texture's
/// cards tie in their mean place but for a lone card.
const ORDER_SETS: [OrderSet; 7] = [
    // Row 1 (y 23).
    OrderSet {
        code: "u4",
        red: ("mg9u4_r", "mg9u4_r"),
        blue: ("mg9u4_b", "mg9u4_b"),
        lone: Some(('R', -2.5)),
        lone_first: false,
        blue_listed_first: false,
        at: [13.0, 23.0],
    },
    OrderSet {
        code: "u1",
        red: ("mg9u1_r", "mg9u1_r"),
        blue: ("mg9u1_b", "mg9u1_b"),
        lone: Some(('R', 4.0)),
        lone_first: false,
        blue_listed_first: false,
        at: [17.5, 23.0],
    },
    OrderSet {
        code: "u3",
        red: ("mg9u3_r", "mg9u3_r"),
        blue: ("mg9u3_b", "mg9u3_b"),
        lone: Some(('R', 4.0)),
        lone_first: false,
        blue_listed_first: true,
        at: [22.0, 23.0],
    },
    // Row 2 (y 29.5).
    OrderSet {
        code: "c",
        red: ("mg9oc_r", "mg9oc_r"),
        blue: ("mg9oc_b", "mg9oc_b"),
        lone: None,
        lone_first: false,
        blue_listed_first: false,
        at: [12.5, 29.5],
    },
    OrderSet {
        code: "u2",
        red: ("mg9u2_r", "mg9u2_r"),
        blue: ("mg9u2_b", "mg9u2_b"),
        lone: Some(('B', 4.0)),
        lone_first: true,
        blue_listed_first: false,
        at: [16.7, 29.5],
    },
    // (The names swapped: the red card's model and texture named `_b`.)
    OrderSet {
        code: "n",
        red: ("mg9nn_b", "mg9nn_b"),
        blue: ("mg9nn_r", "mg9nn_r"),
        lone: Some(('R', 4.0)),
        lone_first: false,
        blue_listed_first: false,
        at: [20.9, 29.5],
    },
    // (The textures swapped between the models: the red card's model
    // named `_r`, its texture `_b`.)
    OrderSet {
        code: "x",
        red: ("mg9xs_r", "mg9xs_b"),
        blue: ("mg9xs_b", "mg9xs_r"),
        lone: None,
        lone_first: false,
        blue_listed_first: false,
        at: [25.1, 29.5],
    },
];

/// Which card of each pair the client draws first, where nothing but the
/// order of each texture's static meshes decides it (each a card of
/// `alpha 0.5`, 1 m square, static, of its set's own models and textures;
/// the nearer card of a pair 0.5 m nearer and 0.5 m west of the other).
/// [`batch9_see_through_statics`] read that two static placeables are not
/// put in order by distance, the same texture first in every pair of a
/// scene; its layouts tied each texture's cards in their mean place, so a
/// fixed order and an order of each texture's meshes together were not
/// told apart. Here a lone card of one texture, standing apart from the
/// pairs, unties them, and the area's list is varied:
///
/// | set | lone card     | listed           | first listed | last listed | mean place |
/// |-----|---------------|------------------|--------------|-------------|------------|
/// | c   | none          | R B, R B         | R            | B           | tie        |
/// | u1  | red, far      | R B, R B, lone   | R            | R           | R farther  |
/// | u3  | red, far      | B R, B R, lone   | B            | R           | R farther  |
/// | u2  | blue, far     | lone, R B, R B   | B            | B           | B farther  |
/// | u4  | red, near     | R B, R B, lone   | R            | R           | R nearer   |
/// | n   | red, far      | as u1, the red named `_b` and the blue `_r`             |
/// | x   | none          | as c, the red card's model `_r` with the texture `_b`   |
///
/// The columns are what each reading predicts drawn first: the texture
/// first in the area's list, the texture listed last (its last card) last
/// drawn, or the farther texture by its cards' mean place (or nearest, or
/// farthest card). `n` and `x` tell an order by name (model or texture)
/// from the list's.
///
/// Seen (2026-11-26, 89.8193.37): the texture drawn first in both pairs
/// of each set (`x`, beside the tree, read in its first pair only): `c`
/// blue, `u1` red, `u3` blue, `u2` red, `u4` red, `n` red (as `u1`,
/// whose names it swaps), `x` red. None of the columns predicts it: `c`
/// rules out the first listed and the names (blue, `_b`, first; but `n`
/// red, its red named `_b`), `u1` the last listed undrawn, `u3` the
/// mean, farthest card and last listed (its red farther and last, the
/// blue first), `u4` the nearest card. Not separated: where a set stands
/// from the eye (`u1` and `u3`, alike but for their listing, stand
/// 4.5 m apart, either side of the camera), and the tiles its cards
/// stand on (`u1` straddles x 20, the second row y 30). Moonglow (by the
/// origins) differs in half the pairs.
#[test]
#[ignore]
fn batch9_static_order() {
    let _ = oracle_tool!("nwn_script_comp");
    let base = super::batch5::base_game();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut rows: Vec<String> = Vec::new();
    for set in &ORDER_SETS {
        for ((model_name, texture), rgba) in
            [(set.red, [255, 0, 0, 255]), (set.blue, [0, 0, 255, 255])]
        {
            files.push((format!("{texture}.tga"), solid(rgba)));
            let quad = Quad {
                name: "card".into(),
                at: [0.0, 0.0, 0.0],
                corners: [[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0], [0.5, 0.0, 1.0], [-0.5, 0.0, 1.0]],
                bitmap: Cow::Owned(texture.to_string()),
                extra: "  alpha 0.5\n",
                tverts: true,
                colors: false,
            };
            files.push((format!("{model_name}.mdl"), model(model_name, &[quad]).into_bytes()));
            rows.push(model_name.to_string());
        }
    }
    let list: Vec<(&str, u8)> = rows.iter().map(|m| (m.as_str(), 1)).collect();
    let (table, first) = placeables_with(&base, &list);
    files.push(("placeables.2da".into(), table.into_bytes()));
    let appearance =
        |name: &str| -> u32 { first + rows.iter().position(|m| m == name).unwrap() as u32 };
    let light = Lighting { ambient: 0xA0A0A0, diffuse: 0x606060, main_light: 0, only_tile: None };
    let mut r = Reading::new("batch9_static_order", "tcn01", light, &as_refs(&files));
    // Each set's pairs: the nearer color and the nearer card's place.
    let pairs = |set: &OrderSet| [('R', set.at), ('B', [set.at[0] + 2.2, set.at[1]])];
    let model_of = |set: &OrderSet, color: char| if color == 'R' { set.red.0 } else { set.blue.0 };
    let static_field = [("Static", Value::Byte(1))];
    for set in &ORDER_SETS {
        let lone_at = |north: f32| [set.at[0] + 1.35, set.at[1] + north];
        if let (Some((color, north)), true) = (set.lone, set.lone_first) {
            let [x, y] = lone_at(north);
            r.place(appearance(model_of(set, color)), [x, y, 0.0], 0.0, &static_field);
        }
        for (near, at) in pairs(set) {
            let order = if set.blue_listed_first { ['B', 'R'] } else { ['R', 'B'] };
            for color in order {
                let [x, y] = if color == near { at } else { [at[0] + 0.5, at[1] + 0.5] };
                r.place(appearance(model_of(set, color)), [x, y, 0.0], 0.0, &static_field);
            }
        }
        if let (Some((color, north)), false) = (set.lone, set.lone_first) {
            let [x, y] = lone_at(north);
            r.place(appearance(model_of(set, color)), [x, y, 0.0], 0.0, &static_field);
        }
    }
    let dir = r.dir.clone();
    let (_, client) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    let pictures = pictures(&dir, &client);
    let camera = reference_camera();
    for set in &ORDER_SETS {
        for (side, img) in &pictures {
            let firsts: Vec<String> = pairs(set)
                .iter()
                .map(|(near, at)| {
                    let rect = |at: [f32; 2]| {
                        let pts =
                            [(-0.5, 0.0), (0.5, 0.0), (0.5, 1.0), (-0.5, 1.0)].map(|(dx, z)| {
                                project(
                                    &camera,
                                    Vec3::new(at[0] + dx, at[1], z),
                                    img.width,
                                    img.height,
                                )
                            });
                        let xs = pts.map(|p| p.0);
                        let ys = pts.map(|p| p.1);
                        [
                            xs.iter().copied().fold(f32::MAX, f32::min),
                            ys.iter().copied().fold(f32::MAX, f32::min),
                            xs.iter().copied().fold(f32::MIN, f32::max),
                            ys.iter().copied().fold(f32::MIN, f32::max),
                        ]
                    };
                    let (n, f) = (rect(*at), rect([at[0] + 0.5, at[1] + 0.5]));
                    let both = [n[0].min(f[0]), n[1].min(f[1]), n[2].max(f[2]), n[3].max(f[3])];
                    let mixed = mixed_pixels(img, both, 6.0);
                    let first = match (mixed > 100, *near) {
                        (false, c) => c,
                        (true, 'R') => 'B',
                        (true, _) => 'R',
                    };
                    format!("nearer {near}: {mixed:4} mixed, {first} first")
                })
                .collect();
            eprintln!("{side:22} set {:3} {}", set.code, firsts.join(" | "));
        }
    }
    eprintln!("{}", dir.display());
}

// ---------------------------------------------------------------------------
// Static see-through meshes and the tiles they stand on.

/// A pair of [`batch9_static_tiles`]: its label, the nearer card's color,
/// its place (x, y) and the farther card's offset from it (x, y).
type TilePair = (&'static str, char, [f32; 2], [f32; 2]);

/// The pairs of [`batch9_static_tiles`] (the area's tiles are 10 m; tile
/// 9 is x 10 to 20, y 20 to 30, 10 east of it, 13 and 14 north of them).
const TILE_PAIRS: [TilePair; 16] = [
    // Wholly inside one tile: two pairs a tile, the red nearer in the
    // first; tiles 9 and 13 at the same x (near and far), 10 and 14 their
    // mirror east of the camera.
    ("in 9", 'R', [14.0, 23.0], [0.5, 0.5]),
    ("in 9", 'B', [16.5, 23.0], [0.5, 0.5]),
    ("in 10", 'R', [23.0, 23.0], [0.5, 0.5]),
    ("in 10", 'B', [25.5, 23.0], [0.5, 0.5]),
    ("in 13", 'R', [14.0, 32.0], [0.5, 0.5]),
    ("in 13", 'B', [16.5, 32.0], [0.5, 0.5]),
    ("in 14", 'R', [23.0, 32.0], [0.5, 0.5]),
    ("in 14", 'B', [25.5, 32.0], [0.5, 0.5]),
    // Across x 20 (tiles 9 and 10, as far from the camera): the nearer
    // card on 9 (the earlier) or on 10, either color.
    ("near on 9 | far on 10", 'R', [19.55, 23.0], [0.5, 0.5]),
    ("near on 10 | far on 9", 'R', [20.45, 25.0], [-0.5, 0.5]),
    ("near on 9 | far on 10", 'B', [19.55, 27.0], [0.5, 0.5]),
    ("near on 10 | far on 9", 'B', [20.45, 33.0], [-0.5, 0.5]),
    // Across y 30 (tiles 9 and 13, 10 and 14): the nearer card on the
    // nearer, earlier tile.
    ("near on 9 | far on 13", 'R', [14.0, 29.6], [0.5, 0.5]),
    ("near on 9 | far on 13", 'B', [16.5, 29.6], [0.5, 0.5]),
    ("near on 10 | far on 14", 'R', [23.0, 29.6], [0.5, 0.5]),
    ("near on 10 | far on 14", 'B', [25.5, 29.6], [0.5, 0.5]),
];

/// Whether the client draws static placeables' see-through meshes by the
/// tile they stand on (a static placeable is merged into its tile): by
/// the tiles' order in the area, or the farther tile first, and within a
/// tile in a fixed order. Every card is one of two static placeables'
/// models (red and blue, `alpha 0.5`, 1 m square), the red listed first
/// in each pair; [`TILE_PAIRS`] lays pairs wholly inside four tiles (two
/// near the camera, two far, at the same x; two west of it and two east),
/// across the boundary between two tiles as far from the camera (x 20:
/// the nearer card on the earlier tile or the later), and across the
/// boundary between a near tile and a far one (y 30).
///
/// What each reading predicts: by the tiles' order, a pair across x 20 is
/// drawn nearer first with its nearer card on tile 9 and farther first on
/// 10, and one across y 30 nearer first; by the tiles' distance, one across
/// y 30 farther first; within a tile, one color first in both its pairs,
/// the same in every tile for a fixed order.
///
/// Seen (2026-11-26, 89.8193.37): the blue first in all sixteen pairs,
/// inside each of the four tiles, across x 20 whichever tile the nearer
/// card stands on, and across y 30. Neither the tiles' order nor their
/// distance decides it: one order for the two textures over the whole
/// area (Moonglow, by the origins, differs in the eight with the blue
/// nearer).
#[test]
#[ignore]
fn batch9_static_tiles() {
    let _ = oracle_tool!("nwn_script_comp");
    let base = super::batch5::base_game();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, rgba) in [("mg9t_r", [255, 0, 0, 255]), ("mg9t_b", [0, 0, 255, 255])] {
        files.push((format!("{name}.tga"), solid(rgba)));
        let quad = Quad {
            name: "card".into(),
            at: [0.0, 0.0, 0.0],
            corners: [[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0], [0.5, 0.0, 1.0], [-0.5, 0.0, 1.0]],
            bitmap: Cow::Owned(name.to_string()),
            extra: "  alpha 0.5\n",
            tverts: true,
            colors: false,
        };
        files.push((format!("{name}.mdl"), model(name, &[quad]).into_bytes()));
    }
    let (table, first) = placeables_with(&base, &[("mg9t_r", 1), ("mg9t_b", 1)]);
    files.push(("placeables.2da".into(), table.into_bytes()));
    let light = Lighting { ambient: 0xA0A0A0, diffuse: 0x606060, main_light: 0, only_tile: None };
    let mut r = Reading::new("batch9_static_tiles", "tcn01", light, &as_refs(&files));
    let static_field = [("Static", Value::Byte(1))];
    let far_of = |at: [f32; 2], off: [f32; 2]| [at[0] + off[0], at[1] + off[1]];
    for (_, near, at, off) in TILE_PAIRS {
        for color in ['R', 'B'] {
            let [x, y] = if color == near { at } else { far_of(at, off) };
            r.place(first + u32::from(color == 'B'), [x, y, 0.0], 0.0, &static_field);
        }
    }
    let dir = r.dir.clone();
    let (_, client) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    let pictures = pictures(&dir, &client);
    let camera = reference_camera();
    for (label, near, at, off) in TILE_PAIRS {
        for (side, img) in &pictures {
            let rect = |at: [f32; 2]| {
                let pts = [(-0.5, 0.0), (0.5, 0.0), (0.5, 1.0), (-0.5, 1.0)].map(|(dx, z)| {
                    project(&camera, Vec3::new(at[0] + dx, at[1], z), img.width, img.height)
                });
                let xs = pts.map(|p| p.0);
                let ys = pts.map(|p| p.1);
                [
                    xs.iter().copied().fold(f32::MAX, f32::min),
                    ys.iter().copied().fold(f32::MAX, f32::min),
                    xs.iter().copied().fold(f32::MIN, f32::max),
                    ys.iter().copied().fold(f32::MIN, f32::max),
                ]
            };
            let (n, f) = (rect(at), rect(far_of(at, off)));
            let both = [n[0].min(f[0]), n[1].min(f[1]), n[2].max(f[2]), n[3].max(f[3])];
            let mixed = mixed_pixels(img, both, 4.0);
            let (first, verdict) = match (mixed > 100, near) {
                (false, c) => (c, "nearer first"),
                (true, 'R') => ('B', "farther first"),
                (true, _) => ('R', "farther first"),
            };
            eprintln!(
                "{side:22} {label:22} nearer {near}: {mixed:4} mixed: {verdict} ({first} first)"
            );
        }
    }
    eprintln!("{}", dir.display());
}

// ---------------------------------------------------------------------------
// Static see-through meshes in their textures' order (read from the
// client's code, confirmed by running it).

/// Where a rung's texture gets its alpha mean, as the client's
/// `CAurTextureBasic` (+0x30) does: a 32-bit TGA's computed from its
/// texels (`ImageGetAlphaMean`), a TXI's `alphamean`, a BioWare DDS's
/// header (read with the image, after the TXI: the header's wins), a
/// standard DDS's none (a compressed texture without one: 1, or 0.5 of
/// four channels).
#[derive(Clone, Copy)]
enum MeanFrom {
    /// A 32-bit TGA, every texel of this alpha.
    Tga(u8),
    /// An opaque 32-bit TGA with a TXI `alphamean`.
    Txi(f32),
    /// A BioWare DDS (DXT5) with this header alpha mean, and a TXI
    /// `alphamean` beside it where given.
    Bioware(f32, Option<f32>),
    /// A standard DDS (DXT5).
    Standard,
}

/// The rungs of [`batch9_static_alpha_mean`]: the texture's code (its
/// names scrambled against the means), where its alpha mean comes from,
/// and the mean the reading gives it.
const RUNGS: [(&str, MeanFrom, f32); 7] = [
    ("q", MeanFrom::Tga(255), 1.0),
    ("d", MeanFrom::Tga(245), 245.0 / 255.0),
    ("w", MeanFrom::Bioware(0.9, None), 0.9),
    ("h", MeanFrom::Txi(0.8), 0.8),
    ("b", MeanFrom::Bioware(0.7, Some(0.95)), 0.7),
    ("t", MeanFrom::Standard, 0.5),
    ("m", MeanFrom::Bioware(0.3, None), 0.3),
];

/// A rung's card: its model's and texture's name.
fn rung_card(code: &str, color: char) -> String {
    format!("mg9l{code}_{}", color.to_ascii_lowercase())
}

/// A rung's texture files in `color`.
fn rung_files(code: &str, from: MeanFrom, color: char) -> Vec<(String, Vec<u8>)> {
    let name = rung_card(code, color);
    let rgb = if color == 'R' { [255, 0, 0] } else { [0, 0, 255] };
    let opaque = Rgba { width: 16, height: 16, data: [rgb[0], rgb[1], rgb[2], 255].repeat(256) };
    let dxt5 = || mg_image::encode::compress(&opaque, mg_image::Format::Bc3, true, false);
    let txi = |mean: f32| (format!("{name}.txi"), format!("alphamean {mean}\n").into_bytes());
    match from {
        MeanFrom::Tga(a) => vec![(format!("{name}.tga"), solid([rgb[0], rgb[1], rgb[2], a]))],
        MeanFrom::Txi(mean) => {
            vec![(format!("{name}.tga"), solid([rgb[0], rgb[1], rgb[2], 255])), txi(mean)]
        }
        MeanFrom::Bioware(header, beside) => {
            let mut t = dxt5();
            t.alpha_mean = Some(header);
            let mut files =
                vec![(format!("{name}.dds"), mg_image::dds::write_bioware(&t).unwrap())];
            files.extend(beside.map(txi));
            files
        }
        MeanFrom::Standard => {
            vec![(format!("{name}.dds"), mg_image::dds::write_standard(&dxt5()).unwrap())]
        }
    }
}

/// A card model of `alpha 0.5`, 1 m square, standing facing the eye.
fn card_mdl(name: &str, texture: &str) -> Vec<u8> {
    let quad = Quad {
        name: "card".into(),
        at: [0.0, 0.0, 0.0],
        corners: [[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0], [0.5, 0.0, 1.0], [-0.5, 0.0, 1.0]],
        bitmap: Cow::Owned(texture.to_string()),
        extra: "  alpha 0.5\n",
        tverts: true,
        colors: false,
    };
    model(name, &[quad]).into_bytes()
}

/// The ladder's pairs: the red card's rung and the blue card's (codes).
/// Adjacent rungs, both colors of the top two, and a few across the
/// ladder; `b` against `w` tells the DDS header from its TXI.
const LADDER: [(&str, &str); 12] = [
    ("q", "d"),
    ("d", "q"),
    ("d", "w"),
    ("w", "h"),
    ("h", "b"),
    ("b", "t"),
    ("t", "m"),
    ("m", "q"),
    ("w", "t"),
    ("b", "w"),
    ("m", "h"),
    ("t", "d"),
];

/// The slots of the pairs, nearer card's place: eight columns 2 m apart,
/// three rows 4 m apart (the farther card 0.5 m east and north of it).
fn slot(i: usize) -> [f32; 2] {
    [13.0 + 2.0 * (i % 8) as f32, 23.0 + 4.0 * (i / 8) as f32]
}

/// The farther card's place of a pair whose nearer stands at `at`.
fn far_of_near(at: [f32; 2]) -> [f32; 2] {
    [at[0] + 0.5, at[1] + 0.5]
}

/// Which color of a pair is drawn first in `img`: its count of mixed
/// pixels ([`mixed_pixels`]) and the color.
fn first_drawn(img: &Rgba, near: char, at: [f32; 2]) -> (usize, char) {
    let camera = reference_camera();
    let rect = |at: [f32; 2]| {
        let pts = [(-0.5, 0.0), (0.5, 0.0), (0.5, 1.0), (-0.5, 1.0)].map(|(dx, z)| {
            project(&camera, Vec3::new(at[0] + dx, at[1], z), img.width, img.height)
        });
        let xs = pts.map(|p| p.0);
        let ys = pts.map(|p| p.1);
        [
            xs.iter().copied().fold(f32::MAX, f32::min),
            ys.iter().copied().fold(f32::MAX, f32::min),
            xs.iter().copied().fold(f32::MIN, f32::max),
            ys.iter().copied().fold(f32::MIN, f32::max),
        ]
    };
    let (n, f) = (rect(at), rect(far_of_near(at)));
    let both = [n[0].min(f[0]), n[1].min(f[1]), n[2].max(f[2]), n[3].max(f[3])];
    let mixed = mixed_pixels(img, both, 6.0);
    let far = if near == 'R' { 'B' } else { 'R' };
    (mixed, if mixed > 100 { far } else { near })
}

/// A pair placed and its prediction: label, the red card's appearance
/// and whether static, the blue's, the nearer color, its place, and the
/// color predicted drawn first.
struct Placed {
    label: String,
    red: (u32, bool),
    blue: (u32, bool),
    near: char,
    at: [f32; 2],
    predicted: char,
}

/// Places `pairs` (red listed first in even slots, blue in odd ones),
/// shoots the client and Moonglow, prints each pair's reading and
/// panics where the client's differs from the prediction.
fn shoot_pairs(mut r: Reading, pairs: &[Placed]) {
    for (i, p) in pairs.iter().enumerate() {
        let cards = [('R', p.red), ('B', p.blue)];
        let order = if i % 2 == 0 { [cards[0], cards[1]] } else { [cards[1], cards[0]] };
        for (color, (appearance, statics)) in order {
            let [x, y] = if color == p.near { p.at } else { far_of_near(p.at) };
            r.place(appearance, [x, y, 0.0], 0.0, &[("Static", Value::Byte(u8::from(statics)))]);
        }
    }
    let dir = r.dir.clone();
    let (_, client) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    let pictures = pictures(&dir, &client);
    let mut failed = Vec::new();
    for p in pairs {
        for (side, img) in &pictures {
            let (mixed, first) = first_drawn(img, p.near, p.at);
            let verdict = if first == p.predicted { "as predicted" } else { "NOT as predicted" };
            eprintln!(
                "{side:22} {:28} nearer {}: {mixed:4} mixed, {first} first ({verdict})",
                p.label, p.near
            );
            if *side == "client" && first != p.predicted {
                failed.push(format!("{} nearer {}", p.label, p.near));
            }
        }
    }
    eprintln!("{}", dir.display());
    assert!(failed.is_empty(), "the client differs from the prediction: {failed:?}");
}

/// The client's order of static placeables' see-through meshes, as read
/// from its code (`docs/findings.md`, "Static placeables' see-through
/// meshes go by their textures' alpha means"): static geometry's meshes
/// are drawn by texture (`Scene::AddPartsToDrawBuckets` walks
/// `meshbuckets` in `AurTextureGetOrdering`'s `TextureIndices` order),
/// the textures sorted by their alpha mean (`CAurTextureBasic` +0x30), the
/// highest first; within a texture, in the order its meshes were listed.
/// Ties (every earlier batch's textures were opaque TGAs, all of mean 1)
/// are left to `std::sort` over the whole static texture list.
///
/// Seven textures of distinct means from every source the code reads
/// ([`MeanFrom`]; each in red and in blue, names scrambled against the
/// means), on static cards of `alpha 0.5` (1 m, own models), in twelve
/// pairs of a red and a blue rung, each pair twice (the red nearer, the
/// blue nearer), listed red first and blue first in turn.
///
/// Predicted before the run, every pair, whichever is nearer: the card
/// of the higher alpha mean drawn first. By pair (red rung, blue rung:
/// first): q d R, d q B, d w R, w h R, h b R, b t R, t m R, m q B, w t R,
/// b w B (the DDS header's 0.7 over its TXI's 0.95, which would put `b`
/// first), m h B, t d B.
#[test]
#[ignore]
fn batch9_static_alpha_mean() {
    let _ = oracle_tool!("nwn_script_comp");
    let base = super::batch5::base_game();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut rows: Vec<String> = Vec::new();
    for (code, from, _) in RUNGS {
        for color in ['R', 'B'] {
            let name = rung_card(code, color);
            files.extend(rung_files(code, from, color));
            files.push((format!("{name}.mdl"), card_mdl(&name, &name)));
            rows.push(name);
        }
    }
    let list: Vec<(&str, u8)> = rows.iter().map(|m| (m.as_str(), 1)).collect();
    let (table, first) = placeables_with(&base, &list);
    files.push(("placeables.2da".into(), table.into_bytes()));
    let appearance = |code: &str, color: char| {
        first + rows.iter().position(|m| *m == rung_card(code, color)).unwrap() as u32
    };
    let mean = |code: &str| RUNGS.iter().find(|r| r.0 == code).unwrap().2;
    let mut pairs = Vec::new();
    for (k, (red, blue)) in LADDER.iter().enumerate() {
        for (j, near) in ['R', 'B'].into_iter().enumerate() {
            pairs.push(Placed {
                label: format!("red {red} ({:.3}), blue {blue} ({:.3})", mean(red), mean(blue)),
                red: (appearance(red, 'R'), true),
                blue: (appearance(blue, 'B'), true),
                near,
                at: slot(k * 2 + j),
                predicted: if mean(red) > mean(blue) { 'R' } else { 'B' },
            });
        }
    }
    // (Listed in a scrambled order: a pair's slot is its place, not its turn.)
    pairs.reverse();
    pairs.rotate_left(5);
    let light = Lighting { ambient: 0xA0A0A0, diffuse: 0x606060, main_light: 0, only_tile: None };
    let r = Reading::new("batch9_static_alpha_mean", "tcn01", light, &as_refs(&files));
    shoot_pairs(r, &pairs);
}

/// [`batch9_static_alpha_mean`]'s check of its reading and the order of
/// static against dynamic see-through meshes. The reading counts mixed
/// pixels with textures whose alpha means are below 1; dynamic
/// placeables of the same models (drawn by their origins, farther first,
/// `batch9_see_through_statics`) must read "farther first" in every pair.
/// And a static card against a dynamic one (opaque TGAs of their own):
/// the client fills its draw buckets with the static geometry's meshes
/// before the sorted dynamic objects' (`AddPartsToDrawBuckets` calls
/// `DrawBucketManager::AddPartToBucket` for `meshbuckets` before it sorts
/// the `Gob`s, and `DrawBucket::Add` only appends).
///
/// Predicted before the run: the dynamic pairs farther first (six); the
/// static card first in all four static-against-dynamic pairs, whichever
/// is nearer and whichever color is static.
#[test]
#[ignore]
fn batch9_static_alpha_mean_checks() {
    let _ = oracle_tool!("nwn_script_comp");
    let base = super::batch5::base_game();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut rows: Vec<(String, u8)> = Vec::new();
    for (code, from, _) in RUNGS.iter().filter(|r| ["q", "d", "t", "m", "b", "w"].contains(&r.0)) {
        for color in ['R', 'B'] {
            let name = rung_card(code, color);
            files.extend(rung_files(code, *from, color));
            files.push((format!("{name}.mdl"), card_mdl(&name, &name)));
            rows.push((name, 0));
        }
    }
    // (Static and dynamic cards of opaque TGAs of their own.)
    for (kind, statics) in [("s", 1), ("y", 0)] {
        for color in ['R', 'B'] {
            let name = rung_card(kind, color);
            files.extend(rung_files(kind, MeanFrom::Tga(255), color));
            files.push((format!("{name}.mdl"), card_mdl(&name, &name)));
            rows.push((name, statics));
        }
    }
    let list: Vec<(&str, u8)> = rows.iter().map(|(m, s)| (m.as_str(), *s)).collect();
    let (table, first) = placeables_with(&base, &list);
    files.push(("placeables.2da".into(), table.into_bytes()));
    let appearance = |code: &str, color: char| {
        first + rows.iter().position(|(m, _)| *m == rung_card(code, color)).unwrap() as u32
    };
    let mut pairs = Vec::new();
    for (k, (red, blue)) in [("q", "d"), ("t", "m"), ("b", "w")].iter().enumerate() {
        for (j, near) in ['R', 'B'].into_iter().enumerate() {
            pairs.push(Placed {
                label: format!("dynamic: red {red}, blue {blue}"),
                red: (appearance(red, 'R'), false),
                blue: (appearance(blue, 'B'), false),
                near,
                at: slot(k * 2 + j),
                predicted: if near == 'R' { 'B' } else { 'R' },
            });
        }
    }
    for (k, static_color) in ['R', 'B'].into_iter().enumerate() {
        for (j, near) in ['R', 'B'].into_iter().enumerate() {
            let card = |color: char| {
                let kind = if color == static_color { "s" } else { "y" };
                (appearance(kind, color), color == static_color)
            };
            pairs.push(Placed {
                label: format!("static {static_color} against dynamic"),
                red: card('R'),
                blue: card('B'),
                near,
                at: slot(8 + k * 2 + j),
                predicted: static_color,
            });
        }
    }
    let light = Lighting { ambient: 0xA0A0A0, diffuse: 0x606060, main_light: 0, only_tile: None };
    let r = Reading::new("batch9_static_alpha_mean_checks", "tcn01", light, &as_refs(&files));
    shoot_pairs(r, &pairs);
}
