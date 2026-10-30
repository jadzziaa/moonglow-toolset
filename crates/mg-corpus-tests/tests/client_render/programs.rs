//! Which of the game's programs the client draws each kind of mesh with
//! (`docs/shaders-proposal.md`, M4; `docs/research/notes_shaders.md` §A.4,
//! open questions 10 and 12), read from its pictures.
//!
//! Every stock vertex and fragment program (but the post passes' and the
//! PLT's) is put in the scratch user directory's `override`, marked: its
//! `main` wrapped, the vertex program handing its number to the fragment
//! program, which paints the pair's numbers and the preamble's defines
//! (`NO_DISCARD`, `FRAGMENT_LIGHTING`, `KEYHOLING_ENABLED`) and `m_texture`'s turn
//! over whatever it drew: red the fragment program (8 a number), green the
//! vertex program, blue the defines. What a program discards stays
//! discarded. Lit, textured fragments in a band near the bottom of the
//! picture (rows 6 and 7 of 32-pixel blocks) show uniforms instead, one a
//! block (`m_texture`, `staticLighting`, `texNoiseBound`, the module's
//! clock, the camera's angles…), as [`super::debug_value`] reads them.
//!
//! The scene: an exterior area (`tcn01`, whose tiles' ground meshes have
//! `rotatetexture`, every tile turned a quarter) with a row of cards north
//! of the player, a mesh each of every kind the choice could depend on
//! ([`CARDS`]), the same placeable static further north, the game's
//! placeables with and without a `Reflection` environment map, and
//! creatures with and without an appearance `ENVMAP`.

use super::*;

/// A card of [`CARDS`]: what it is, its mesh's `bitmap`, and the lines its
/// mesh adds (`texture1`, `materialname`, `renderhint`, `alpha`…); whether
/// it has texture coordinates and vertex colors.
struct Card {
    name: &'static str,
    bitmap: &'static str,
    extra: &'static str,
    tverts: bool,
    colors: bool,
}

const fn card(name: &'static str, bitmap: &'static str, extra: &'static str) -> Card {
    Card { name, bitmap, extra, tverts: true, colors: false }
}

/// The cards, west to east, in rows of twelve from the ground up.
const CARDS: [Card; 40] = [
    card("plain", "mgp_plain", ""),
    card("alpha", "mgp_alpha", ""),
    card("opaque32", "mgp_opq32", ""),
    card("punchthrough", "mgp_punch", ""),
    card("additive", "mgp_add", ""),
    card("decal", "mgp_decal", ""),
    card("envmap", "mgp_env", ""),
    card("envmap default", "mgp_envd", ""),
    card("bumpyshiny", "mgp_bumpy", ""),
    card("envmap+alpha", "mgp_enva", ""),
    card("mesh alpha 0.5", "mgp_plain", "  alpha 0.5\n"),
    card("transparencyhint", "mgp_plain", "  transparencyhint 1\n"),
    card("no texture", "NULL", ""),
    Card { name: "no tverts", bitmap: "mgp_plain", extra: "", tverts: false, colors: false },
    Card { name: "no texture/tverts", bitmap: "NULL", extra: "", tverts: false, colors: false },
    Card { name: "colors", bitmap: "mgp_plain", extra: "", tverts: true, colors: true },
    card("renderhint+tex1", "mgp_plain", "  texture1 mgp_nrm\n  renderhint NormalAndSpecMapped\n"),
    card("texture1 alone", "mgp_plain", "  texture1 mgp_nrm\n"),
    card("mtr texture1", "mgp_plain", "  materialname mgp_m1\n"),
    card("mtr renderhint", "mgp_plain", "  materialname mgp_mh\n"),
    card("hint+tex1+envmap", "mgp_env", "  texture1 mgp_nrm\n  renderhint NormalAndSpecMapped\n"),
    card("mtr transparency", "mgp_plain", "  materialname mgp_mt\n"),
    card("alphamean 0.5", "mgp_amean", ""),
    card("mtr texture2", "mgp_plain", "  materialname mgp_ms\n"),
    Card { name: "no tverts+envmap", bitmap: "mgp_env", extra: "", tverts: false, colors: false },
    card("bumpyshiny+alpha", "mgp_bumpa", ""),
    card("decal+alpha", "mgp_decala", ""),
    card("punchthrough opaque", "mgp_puncho", ""),
    card("envmap+mesh alpha", "mgp_env", "  alpha 0.5\n"),
    card("decal red", "mgp_decalr", ""),
    card("decal red diffuse 0.5", "mgp_decalr", "  diffuse 0.5 0.5 0.5\n"),
    card("decal red alpha 0.5", "mgp_decalr", "  alpha 0.5\n"),
    card("dds dxt1 opaque", "mgp_d1o", ""),
    card("dds dxt1 cut", "mgp_d1c", ""),
    card("dds dxt5 opaque", "mgp_d5o", ""),
    card("dds dxt5 alpha", "mgp_d5a", ""),
    // (The third client batch's: a BioWare DDS whose header's alpha mean
    // is 1 though its west half is see-through, with and without a
    // transparency hint; a material naming a geometry program that draws
    // nothing, alone and beside a pair.)
    card("bioware dds mean 1", "mgp_bw1", ""),
    card("bioware dds mean 1+hint", "mgp_bw1", "  transparencyhint 1\n"),
    card("mtr gs alone", "mgp_plain", "  materialname mgp_gs1\n"),
    card("mtr gs+pair", "mgp_plain", "  materialname mgp_gs2\n"),
];

/// Where a card is read on the client's picture, from its west edge: its
/// middle, or for a texture see-through in its west half, the middle of
/// its east half (the card straddling its own cut read the board behind
/// it).
fn read_at(c: &Card) -> f32 {
    if matches!(c.bitmap, "mgp_d1c" | "mgp_d5a" | "mgp_bw1") { 0.6 } else { 0.4 }
}

/// The cards without texture coordinates are a model of their own
/// (`mg_notv`), so that the rest show if the client refuses it.
fn risky(c: &Card) -> bool {
    !c.tverts
}

/// Where a card stands in its model (west to east, in rows of twelve):
/// its lower west corner.
fn card_corner(i: usize) -> (f32, f32) {
    ((i % 12) as f32 - 6.0 + 0.1, (i / 12) as f32 + 0.1)
}

/// A quad's mesh in a model of [`model`]: `corners` (four, a face's
/// front seen in their order), at `at`.
struct Quad {
    name: String,
    at: [f32; 3],
    corners: [[f32; 3]; 4],
    bitmap: &'static str,
    extra: &'static str,
    tverts: bool,
    colors: bool,
}

/// An ASCII model of quads.
fn model(name: &str, quads: &[Quad]) -> String {
    let mut s = format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n"
    );
    for q in quads {
        let tverts =
            if q.tverts { "  tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n" } else { "" };
        let colors =
            if q.colors { "  colors 4\n    1 0 0\n    0 1 0\n    0 0 1\n    1 1 1\n" } else { "" };
        let verts: String =
            q.corners.iter().map(|[x, y, z]| format!("    {x} {y} {z}\n")).collect();
        let [x, y, z] = q.at;
        s += &format!(
            "node trimesh {}\n  parent {name}\n  position {x} {y} {z}\n  orientation 0 0 0 0\n  \
             ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {}\n{}  \
             render 1\n  shadow 0\n  verts 4\n{verts}{tverts}{colors}  faces 2\n    \
             0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n",
            q.name, q.bitmap, q.extra
        );
    }
    s + &format!("endmodelgeom {name}\ndonemodel {name}\n")
}

/// A square of `size` facing −Y (standing, its front to the south).
fn standing(size: f32) -> [[f32; 3]; 4] {
    [[0.0, 0.0, 0.0], [size, 0.0, 0.0], [size, 0.0, size], [0.0, 0.0, size]]
}

/// A model of the cards `which` takes, each a 0.8 m square facing −Y;
/// `board`, with a decal board behind them (drawn by the unlit
/// `vsvt`/`fst`: where a card is missing, that shows).
fn cards_model(name: &str, which: &dyn Fn(&Card) -> bool, board: bool) -> String {
    let mut quads: Vec<Quad> = CARDS
        .iter()
        .enumerate()
        .filter(|(_, c)| which(c))
        .map(|(i, c)| {
            let (x, z) = card_corner(i);
            Quad {
                name: format!("p{i}"),
                at: [x, 0.0, z],
                corners: standing(0.8),
                bitmap: c.bitmap,
                extra: c.extra,
                tverts: c.tverts,
                colors: c.colors,
            }
        })
        .collect();
    if board {
        quads.push(Quad {
            name: "board".into(),
            at: [-6.0, 0.05, 0.0],
            corners: [[0.0, 0.0, 0.0], [12.0, 0.0, 0.0], [12.0, 0.0, 4.1], [0.0, 0.0, 4.1]],
            bitmap: "mgp_decal",
            extra: "",
            tverts: true,
            colors: false,
        });
    }
    model(name, &quads)
}

/// Two squares lying on the ground, the west one's texture turned with it
/// (`rotatetexture 1`): what `m_texture` is for a placeable that is
/// turned.
fn turned_model() -> String {
    let flat = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]];
    let quad = |name: &str, x: f32, extra| Quad {
        name: name.into(),
        at: [x, 0.0, 0.05],
        corners: flat,
        bitmap: "mgp_plain",
        extra,
        tverts: true,
        colors: false,
    };
    model("mg_turned", &[quad("rotated", -1.1, "  rotatetexture 1\n"), quad("kept", 0.1, "")])
}

/// A plain square of 8 m lying on the ground: under the band's east half,
/// for a placeable's uniforms there. (`MG_LYING_BUMPY=1`: its texture's
/// TXI names a `bumpyshinytexture`.)
fn lying_model() -> String {
    // (`MG_LYING_ENVD=1`: its TXI names `envmaptexture default`.)
    let bitmap = if std::env::var_os("MG_LYING_BUMPY").is_some() {
        "mgp_bumpy"
    } else if std::env::var_os("MG_LYING_ENVD").is_some() {
        "mgp_envd"
    } else {
        "mgp_plain"
    };
    let flat = [[0.0, 0.0, 0.0], [8.0, 0.0, 0.0], [8.0, 8.0, 0.0], [0.0, 8.0, 0.0]];
    let q = Quad {
        name: "lying".into(),
        at: [0.0, 0.0, 0.03],
        corners: flat,
        bitmap,
        extra: "",
        tverts: true,
        colors: false,
    };
    model("mg_lying", &[q])
}

/// A TGA, `alpha` with an alpha channel, of `f(x, y)` (RGBA) for 64×64
/// texels.
fn tga(alpha: bool, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let depth = if alpha { 32 } else { 24 };
    let mut t =
        vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 64, 0, depth, if alpha { 8 } else { 0 }];
    for y in 0..64 {
        for x in 0..64 {
            let [r, g, b, a] = f(x, y);
            t.extend([b, g, r]);
            if alpha {
                t.push(a);
            }
        }
    }
    t
}

/// A checker of greys, opaque.
fn checker(x: u32, y: u32) -> [u8; 4] {
    if ((x / 8) + (y / 8)).is_multiple_of(2) { [200, 190, 170, 255] } else { [90, 100, 110, 255] }
}

/// A standard DDS of 64 by 64 texels with all its levels (the client reads
/// past a single level: what it drew was noise), in `fourcc`'s blocks:
/// `block(x, of)` for each block, `x` of the level's `of` blocks a row.
fn dds(fourcc: &[u8; 4], block: impl Fn(u32, u32) -> Vec<u8>) -> Vec<u8> {
    let block = &block;
    let mut blocks = Vec::new();
    for level in 0..7u32 {
        let of = (16 >> level).max(1);
        blocks.extend((0..of).flat_map(|_| (0..of).flat_map(move |x| block(x, of))));
    }
    let top = block(0, 16).len() as u32 * 256;
    let mut h = b"DDS ".to_vec();
    let mut u = |v: u32| h.extend(v.to_le_bytes());
    // (Size; flags: caps, height, width, pixel format, levels, linear
    // size; the first level's size; levels.)
    u(124);
    u(0x1 | 0x2 | 0x4 | 0x1000 | 0x2_0000 | 0x8_0000);
    u(64);
    u(64);
    u(top);
    u(0);
    u(7);
    for _ in 0..11 {
        u(0);
    }
    // The pixel format: a four-character code.
    u(32);
    u(0x4);
    u(u32::from_le_bytes(*fourcc));
    for _ in 0..5 {
        u(0);
    }
    // (A texture with levels.)
    u(0x1000 | 0x8 | 0x40_0000);
    for _ in 0..4 {
        u(0);
    }
    h.extend(blocks);
    h
}

/// A BioWare DDS (no magic: width, height, 4 channels, the first level's
/// size and the alpha mean, then DXT5 levels) of 64 by 64 of `block`s.
fn bioware_dds(alpha_mean: f32, block: impl Fn(u32, u32) -> Vec<u8>) -> Vec<u8> {
    let standard = dds(b"DXT5", block);
    // (The standard file's levels, after its 128 bytes of header.)
    let levels = &standard[128..];
    let mut h = Vec::new();
    for v in [64u32, 64, 4, 64 * 64] {
        h.extend(v.to_le_bytes());
    }
    h.extend(alpha_mean.to_le_bytes());
    h.extend(levels);
    h
}

/// A DXT1 block: red, opaque (four colors); or wholly see-through (three
/// colors and transparent, every texel the transparent one).
fn dxt1(see_through: bool) -> Vec<u8> {
    let (red, blue) = (0xF800u16, 0x001Fu16);
    let (c0, c1, bits) = if see_through { (blue, red, u32::MAX) } else { (red, blue, 0) };
    [c0.to_le_bytes(), c1.to_le_bytes()].concat().into_iter().chain(bits.to_le_bytes()).collect()
}

/// A DXT5 block: alpha 255 or 0 throughout, over an opaque DXT1 block.
fn dxt5(alpha: u8) -> Vec<u8> {
    let mut b = vec![alpha, alpha, 0, 0, 0, 0, 0, 0];
    b.extend(dxt1(false));
    b
}

/// The textures, their TXIs, and the materials.
fn write_textures(over: &Path) {
    let w = |name: &str, bytes: Vec<u8>| std::fs::write(over.join(name), bytes).unwrap();
    for name in ["mgp_plain", "mgp_add", "mgp_decal", "mgp_env", "mgp_envd", "mgp_bumpy"] {
        w(&format!("{name}.tga"), tga(false, checker));
    }
    w("mgp_opq32.tga", tga(true, checker));
    w("mgp_amean.tga", tga(true, checker));
    w("mgp_puncho.tga", tga(false, checker));
    w("mgp_decalr.tga", tga(false, |_, _| [200, 40, 40, 255]));
    // DDS: opaque and see-through in the west half, DXT1 and DXT5.
    w("mgp_d1o.dds", dds(b"DXT1", |_, _| dxt1(false)));
    w("mgp_d1c.dds", dds(b"DXT1", |x, of| dxt1(x * 2 < of)));
    w("mgp_d5o.dds", dds(b"DXT5", |_, _| dxt5(255)));
    w("mgp_d5a.dds", dds(b"DXT5", |x, of| dxt5(if x * 2 < of { 0 } else { 255 })));
    w("mgp_bw1.dds", bioware_dds(1.0, |x, of| dxt5(if x * 2 < of { 0 } else { 255 })));
    for name in ["mgp_alpha", "mgp_punch", "mgp_enva", "mgp_bumpa", "mgp_decala"] {
        w(&format!("{name}.tga"), disc_tga());
    }
    w("mgp_nrm.tga", tga(false, |x, _| [128 + ((x % 16) as u8) * 4, 128, 255, 255]));
    for (name, txi) in [
        ("mgp_punch", "blending punchthrough\n"),
        ("mgp_add", "blending additive\n"),
        ("mgp_decal", "decal 1\n"),
        ("mgp_env", "envmaptexture chrome1\n"),
        ("mgp_envd", "envmaptexture default\n"),
        ("mgp_bumpy", "bumpyshinytexture chrome1\n"),
        ("mgp_enva", "envmaptexture chrome1\n"),
        ("mgp_amean", "alphamean 0.5\n"),
        ("mgp_bumpa", "bumpyshinytexture chrome1\n"),
        ("mgp_decala", "decal 1\n"),
        ("mgp_decalr", "decal 1\n"),
        ("mgp_puncho", "blending punchthrough\n"),
    ] {
        w(&format!("{name}.txi"), txi.as_bytes().to_vec());
    }
    for (name, mtr) in [
        ("mgp_m1", "texture0 mgp_plain\ntexture1 mgp_nrm\n"),
        ("mgp_mh", "renderhint NormalAndSpecMapped\ntexture0 mgp_plain\n"),
        ("mgp_mt", "texture0 mgp_plain\ntransparency 1\n"),
        ("mgp_ms", "texture0 mgp_plain\ntexture2 mgp_plain\n"),
        ("mgp_gs1", "texture0 mgp_plain\ncustomshaderGS mgp_gs\n"),
        (
            "mgp_gs2",
            "texture0 mgp_plain\ncustomshaderVS vslit\ncustomshaderFS fslit\n\
             customshaderGS mgp_gs\n",
        ),
    ] {
        w(&format!("{name}.mtr"), mtr.as_bytes().to_vec());
    }
    // A geometry program that emits nothing: a mesh drawn through it is
    // not seen.
    w(
        "mgp_gs.shd",
        b"layout(triangles) in;\nlayout(triangle_strip, max_vertices = 3) out;\nvoid main()\n{\n}\n"
            .to_vec(),
    );
}

/// The programs marked, vertex and fragment, by name (their numbers are
/// their places, from 1): every stock program but the post passes'
/// (`vsfb*`, `fsfb*`) and the PLT's.
fn programs(game: &GameData) -> (Vec<String>, Vec<String>) {
    let mut names: Vec<String> = game
        .resman
        .entries()
        .iter()
        .filter(|(k, _)| k.restype == ResType::SHD)
        .map(|(k, _)| k.resref.to_string().to_ascii_lowercase())
        .filter(|n| !n.starts_with("vsfb") && !n.starts_with("fsfb") && !n.contains("pltgen"))
        .collect();
    names.sort();
    names.dedup();
    let of = |p: &str| names.iter().filter(|n| n.starts_with(p)).cloned().collect::<Vec<_>>();
    (of("vs"), of("fs"))
}

/// What a marked fragment program paints in the band: lit, textured
/// programs only.
const MARK_BAND: &str = r#"
#if LIGHTING == 1 && SPECULAR_LIGHT == 1 && NO_TEXTURE_COORDS != 1 && NO_TEXTURE != 1 && POSTPROCESSING == 0
float MgB(int v) { return (float(v & 255) + 0.5) / 256.0; }
float MgS(float v) { return v * 0.25 + 0.5; }
float MgA(float v) { return v / 720.0 + 0.5; }
float MgBand(int k)
{
    if (k == 0) return 0.5;
    if (k == 1) return MgS(m_texture[0][0]);
    if (k == 2) return MgS(m_texture[0][1]);
    if (k == 3) return MgS(m_texture[1][0]);
    if (k == 4) return MgS(m_texture[1][1]);
    if (k == 5) return MgS(m_texture[3][0]);
    if (k == 6) return MgS(m_texture[3][1]);
    if (k == 7) return MgB(staticLighting);
#ifdef INC_RANDOM
    if (k == 8) return MgB(texNoiseBound);
#endif
    if (k == 9) return MgB(moduleHour);
    if (k == 10) return MgB(moduleMinutesPerHour);
    if (k == 11) return MgB(moduleDawnHour);
    if (k == 12) return MgB(moduleDuskHour);
    if (k == 13) return MgB(moduleDay);
    if (k == 14) return MgB(moduleMonth);
    if (k == 15) return MgB(moduleYear);
#if NO_DISCARD != 1
    if (k == 16) return fAlphaDiscardValue;
#endif
    if (k == 17) return MgB(texture0Bound);
    if (k == 18) return MgB(envMapCube);
    if (k == 19) return MgB(texEnvBound);
    if (k == 20) return MgB(texEnvCubeBound);
    if (k == 21) return MgB(areaFlags);
    if (k == 22) return MgB(areaWeatherType);
    if (k >= 23 && k < 26) return MgS(areaGlobalWind[k - 23]);
    if (k == 26) return MgA(cameraPitch);
    if (k == 27) return MgA(cameraYaw);
    if (k == 28) return MgA(cameraViewAngle);
    if (k == 29) return playerCameraDist / 64.0;
    if (k >= 30 && k < 33) return playerPosition[k - 30] / 64.0;
    if (k >= 33 && k < 36) return cameraPosition[k - 33] / 64.0;
    if (k >= 36 && k < 39) return cameraFocus[k - 36] / 64.0;
    if (k == 39) return MgB(subvariant);
    if (k == 40) return areaWeatherDensity;
    if (k >= 41 && k < 44) return MgS(m_texture[2][k - 41]);
    // The 2D environment map bound, by its mean colour (its smallest
    // level); here and under the lying square (k 56 on).
    if ((k >= 44 && k < 47) || (k >= 56 && k < 59)) return textureLod(texUnitEnv, vec2(0.5), 20.0)[(k - 44) % 4];
    // The cube map bound, by the mean colours of its +Z and +X faces.
    if (k >= 47 && k < 50) return textureLod(texUnitEnvCube, vec3(0.0, 0.0, 1.0), 20.0)[k - 47];
    if (k >= 50 && k < 53) return textureLod(texUnitEnvCube, vec3(1.0, 0.0, 0.0), 20.0)[k - 50];
    return 0.0;
}
#define MG_BAND 1
#endif
"#;

/// A program's text marked with its number: `main` wrapped.
fn marked(src: &str, vertex: bool, id: usize) -> String {
    marked_as(src, vertex, id, false)
}

/// [`marked`]; `black`: a fragment program that paints black instead of
/// its mark (the decal board's `fst`, for reading what is added over it).
/// `fsparticle` paints its `blendMode` where the others paint `m_texture`'s
/// turn, and whether its `particleSoftness` is above 0 where they paint
/// `KEYHOLING_ENABLED`.
fn marked_as(src: &str, vertex: bool, id: usize, black: bool) -> String {
    let src = src.replace("\r\n", "\n");
    if vertex {
        return format!(
            "#define main mg_main\n{src}\n#undef main\nout float mgVs;\nvoid main()\n{{\n    \
             mg_main();\n    mgVs = {id}.0;\n}}\n"
        );
    }
    // (The noise's uniforms, which only keyholing and water include, for
    // the band.)
    let noise = if src.contains("inc_standard") { "#include \"inc_random\"\n" } else { "" };
    format!(
        "#define main mg_main\n{src}\n#undef main\n{noise}in float mgVs;\n{MARK_BAND}\nvoid main()\n{{\n    \
         mg_main();\n    float rot = 0.0;\n\
         #if defined(INC_COMMON) && NO_TEXTURE_COORDS != 1 && NO_TEXTURE != 1 && POSTPROCESSING == 0\n    \
         vec2 mgc = vec2(m_texture[0][0], m_texture[0][1]);\n    \
         rot = mgc.x > 0.5 ? 0.0 : (mgc.y > 0.5 ? 1.0 : (mgc.x < -0.5 ? 2.0 : 3.0));\n\
         #endif\n    float flags = 4.0 + 8.0 * rot\n        \
         + (KEYHOLING_ENABLED == 1 ? 32.0 : 0.0)\n        \
         + (FRAGMENT_LIGHTING == 1 ? 64.0 : 0.0)\n        \
         + (NO_DISCARD == 1 ? 128.0 : 0.0);\n\
         #if defined(BLEND_MODE_NORMAL) && SHADER_QUALITY_MODE > 0\n    \
         flags = 4.0 + 8.0 * float(blendMode) + (particleSoftness > 0.0 ? 32.0 : 0.0)\n        \
         + (FRAGMENT_LIGHTING == 1 ? 64.0 : 0.0) + (NO_DISCARD == 1 ? 128.0 : 0.0);\n\
         #endif\n    \
         vec4 c = vec4(float({id}) * 8.0 / 255.0, floor(mgVs + 0.5) * 8.0 / 255.0, flags / 255.0, 1.0);\n\
         #ifdef MG_BAND\n    int row = int(gl_FragCoord.y / 32.0) - 6;\n    \
         if (row >= 0 && row < 2)\n    {{\n        \
         float x = clamp(MgBand(row * 32 + int(gl_FragCoord.x / 32.0)), 0.0, 0.99999);\n        \
         c = vec4(floor(x * 255.0) / 255.0, fract(x * 255.0), 0.0, 1.0);\n    }}\n#endif\n    \
         {}gl_FragColor = c;\n}}\n",
        if black { "c = vec4(0.0, 0.0, 0.0, 1.0);\n    " } else { "" }
    )
}

/// A pixel's mark: the pair's numbers and the defines, if it is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Mark {
    fs: u8,
    vs: u8,
    no_discard: bool,
    fragment_lighting: bool,
    keyholing: bool,
    /// `m_texture`'s turn, in quarters (3 also: no texture matrix).
    rot: u8,
}

fn mark(p: &[u8]) -> Option<Mark> {
    let near = |v: u8, step: f32, offset: f32| {
        let n = ((f32::from(v) - offset) / step).round();
        ((f32::from(v) - offset - n * step).abs() <= 2.0 && n >= 0.0).then_some(n as u8)
    };
    let fs = near(p[0], 8.0, 0.0)?;
    let vs = near(p[1], 8.0, 0.0)?;
    let b = near(p[2], 8.0, 4.0)?;
    (fs > 0 && vs > 0).then_some(Mark {
        fs,
        vs,
        no_discard: b & 16 != 0,
        fragment_lighting: b & 8 != 0,
        keyholing: b & 4 != 0,
        rot: b & 3,
    })
}

/// The commonest marks in a square of `r` pixels around (x, y), each with
/// the share of the square's pixels it is, most first.
fn marks_at(img: &Rgba, x: f32, y: f32, r: i32) -> Vec<(Mark, f32)> {
    let mut counts: HashMap<Mark, usize> = HashMap::new();
    let mut n = 0;
    for dy in -r..=r {
        for dx in -r..=r {
            let (px, py) = (x as i32 + dx, y as i32 + dy);
            if px < 0 || py < 0 || px >= img.width as i32 || py >= img.height as i32 {
                continue;
            }
            n += 1;
            let i = (py as usize * img.width as usize + px as usize) * 4;
            if let Some(m) = mark(&img.data[i..i + 4]) {
                *counts.entry(m).or_default() += 1;
            }
        }
    }
    let mut counts: Vec<(Mark, usize)> = counts.into_iter().collect();
    counts.sort_by_key(|(m, c)| (std::cmp::Reverse(*c), *m));
    counts.into_iter().map(|(m, c)| (m, c as f32 / n.max(1) as f32)).collect()
}

/// Exploration: which programs the client draws each kind of mesh with
/// (the module's description above). Prints, per card and object, the pair
/// and the defines; every mark in the picture with its pixels and where
/// they lie; and the band's uniforms. Pictures (the client's, and the same
/// with the places read marked) in `target/test-output/client_programs/`.
/// Needs the game and the off-screen display; run by hand (`--ignored`).
#[test]
#[ignore]
fn program_choice_in_the_client() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("client_programs");
    let over = dir.join("user/override");
    std::fs::create_dir_all(&over).unwrap();
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();

    let (vs, fs) = programs(&base);
    assert!(vs.len() < 32 && fs.len() < 32, "{vs:?} {fs:?}");
    // (`MG_UNMARKED=1`: the scene as the client draws it, to see where
    // things are.)
    let unmarked = std::env::var_os("MG_UNMARKED").is_some();
    for (list, vertex) in [(&vs, true), (&fs, false)].into_iter().filter(|_| !unmarked) {
        for (i, name) in list.iter().enumerate() {
            let src = base.resman.get_named(name, ResType::SHD).unwrap();
            let text = marked(&String::from_utf8_lossy(&src), vertex, i + 1);
            std::fs::write(over.join(format!("{name}.shd")), text).unwrap();
        }
    }
    write_textures(&over);
    std::fs::write(over.join("mg_progs.mdl"), cards_model("mg_progs", &|c| !risky(c), true))
        .unwrap();
    std::fs::write(over.join("mg_notv.mdl"), cards_model("mg_notv", &risky, false)).unwrap();
    std::fs::write(over.join("mg_turned.mdl"), turned_model()).unwrap();
    std::fs::write(over.join("mg_lying.mdl"), lying_model()).unwrap();
    // (`MG_STATIC=1`: the cards' placeable static, as the game bakes it
    // into the area's geometry.)
    let cards_static = u8::from(std::env::var_os("MG_STATIC").is_some());

    // placeables.2da with rows for the models (the armoire's row).
    let table = base.resman.get_named("placeables", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).into_owned();
    let header: Vec<&str> = table.lines().nth(2).unwrap().split_whitespace().collect();
    let column = |name: &str| header.iter().position(|c| *c == name).unwrap() + 1;
    let (model_col, static_col) = (column("ModelName"), column("Static"));
    // (`MG_REFLECTION`: the lying square's `Reflection`, whose map the band
    // then shows.)
    let reflection = std::env::var("MG_REFLECTION").ok();
    let first = table.lines().count() - 3;
    let armoire: Vec<String> =
        table.lines().nth(3).unwrap().split_whitespace().map(str::to_owned).collect();
    let mut rows = Vec::new();
    for (model, is_static) in
        [("mg_progs", cards_static), ("mg_notv", 0), ("mg_turned", 0), ("mg_lying", 0)]
    {
        let mut row = armoire.clone();
        row[0] = (first + rows.len()).to_string();
        row[model_col] = model.to_owned();
        row[static_col] = is_static.to_string();
        if let (Some(r), "mg_lying") = (&reflection, model) {
            row[column("Reflection")] = r.clone();
        }
        rows.push(row.join(" "));
    }
    if !table.ends_with('\n') {
        table.push('\n');
    }
    table += &(rows.join("\n") + "\n");
    std::fs::write(over.join("placeables.2da"), table).unwrap();

    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let tileset = std::env::var("MG_TILESET").unwrap_or_else(|_| "tcn01".into());
    let (mut m, mut are) = build_module_on(&game, &dir, light, &tileset, 0);
    std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
    let are_key = *m.keys_of(ResType::ARE).next().unwrap();
    // (`MG_WIND`: the area's wind power, 0 to 2; 2 by default.)
    let wind = std::env::var("MG_WIND").ok().and_then(|w| w.parse().ok()).unwrap_or(2);
    are.root.set("WindPower", Value::Int(wind));
    for t in are.root.list_mut("Tile_List").unwrap().iter_mut() {
        t.set("Tile_Orientation", Value::Int(1));
    }
    m.set_gff(are_key, &are).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_START_HOUR, 13);
    info.root.write(&ifo::MOD_MIN_PER_HOUR, 3);
    info.root.write(&ifo::MOD_DAWN_HOUR, 5);
    info.root.write(&ifo::MOD_DUSK_HOUR, 19);
    info.root.write(&ifo::MOD_START_DAY, 7);
    info.root.write(&ifo::MOD_START_MONTH, 4);
    info.root.write(&ifo::MOD_START_YEAR, 1372);
    m.set_info(&info).unwrap();
    // (`MG_FACING`: the player's facing as it enters, degrees anticlockwise
    // from east, which the camera takes (the script's `SetCameraFacing`
    // doesn't move it): for `cameraYaw`. North by default, which the
    // places read are worked out for.)
    if let Some(facing) = std::env::var("MG_FACING").ok().and_then(|f| f.parse::<f32>().ok()) {
        let mut info = m.info().unwrap();
        info.root.set("Mod_Entry_Dir_X", Value::Float(facing.to_radians().cos()));
        info.root.set("Mod_Entry_Dir_Y", Value::Float(facing.to_radians().sin()));
        m.set_info(&info).unwrap();
    }
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));

    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    // (What, placeables.2da row, where, facing, static.) Where the cards'
    // model stands, its cards' corners are [`card_corner`] from.
    let cards_at = [18.0f32, 23.0];
    let quarter = std::f32::consts::FRAC_PI_2;
    let objects: Vec<(&str, u32, [f32; 2], f32, u8)> = vec![
        ("cards", first as u32, cards_at, 0.0, cards_static),
        ("cards (no tverts)", first as u32 + 1, cards_at, 0.0, 0),
        ("turned a quarter", first as u32 + 2, [18.5, 19.0], quarter, 0),
        ("lying square", first as u32 + 3, [21.0, 10.0], 0.0, 0),
        ("armoire (no Reflection)", 0, [11.0, 27.0], 0.0, 0),
        ("treasure (Reflection PLC_G_ENV)", 191, [14.0, 19.0], 0.0, 0),
        ("mirror (Reflection default)", 428, [27.5, 28.0], 0.0, 0),
    ];
    let mut placed = Vec::new();
    for (_, row, [x, y], rotation, is_static) in &objects {
        let at = Placement { position: [*x, *y, 0.0], rotation: *rotation };
        let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
        p.set("Appearance", Value::Dword(*row));
        p.set("Static", Value::Byte(*is_static));
        placed.push(p);
    }
    git.root.set("Placeable List", Value::List(placed));
    // Creatures: a human (appearance ENVMAP `default`), a badger (none), a
    // diamond golem (`evmap_irrid`), facing the camera.
    let human = {
        let mut keys: Vec<ResKey> = game
            .resman
            .entries()
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| k.restype == ResType::UTC)
            .collect();
        keys.sort_by_key(|k| k.to_string());
        keys.iter()
            .filter_map(|k| Gff::read(&game.resman.get(k).ok()?).ok())
            .find(|g| g.root.integer("Appearance_Type") == Some(6))
            .expect("a human blueprint")
    };
    let creatures: [(&str, u16, [f32; 2]); 3] = [
        ("human (ENVMAP default)", 6, [9.5, 23.5]),
        ("badger (no ENVMAP)", 8, [30.5, 23.5]),
        ("diamond golem (ENVMAP evmap_irrid)", 149, [10.5, 20.5]),
    ];
    let mut placed = Vec::new();
    for (_, appearance, [x, y]) in creatures {
        let mut c = human.root.clone();
        c.set("Appearance_Type", Value::Word(appearance));
        c.set("Equip_ItemList", Value::List(Vec::new()));
        let scripts: Vec<String> = c
            .fields
            .iter()
            .map(|f| f.label.to_string_lossy())
            .filter(|l| l.starts_with("Script"))
            .collect();
        for label in scripts {
            c.set(&label, Value::resref(ResRef::EMPTY));
        }
        let at = Placement { position: [x, y, 0.0], rotation: -std::f32::consts::FRAC_PI_2 };
        placed.push(instance(&placing, ResType::UTC, &c, at, &[]).unwrap());
    }
    git.root.set("Creature List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let Some(client) = client_screenshot(&dir, "MgScene") else {
        panic!("the client did not reach the scene (see {})", dir.display());
    };
    save_png(
        &client,
        &dir.join(if unmarked { "client_unmarked.png" } else { "client_marked.png" }),
    );

    let (w, h) = (client.width as f32, client.height as f32);
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let vp = camera.projection(w / h) * camera.view();
    let screen = |p: Vec3| {
        let c = vp * p.extend(1.0);
        let n = c.truncate() / c.w;
        ((n.x * 0.5 + 0.5) * w, (0.5 - n.y * 0.5) * h)
    };
    let name = |m: &Mark| {
        let v = vs.get(usize::from(m.vs).wrapping_sub(1)).map_or("?", String::as_str);
        let f = fs.get(usize::from(m.fs).wrapping_sub(1)).map_or("?", String::as_str);
        format!(
            "{v}/{f} NO_DISCARD {} FRAGMENT_LIGHTING {} KEYHOLING_ENABLED {} turn {}",
            u8::from(m.no_discard),
            u8::from(m.fragment_lighting),
            u8::from(m.keyholing),
            m.rot
        )
    };
    let mut shown = client.clone();
    let mut spot = |x: f32, y: f32| {
        for d in -4i32..=4 {
            for (px, py) in [(x as i32 + d, y as i32), (x as i32, y as i32 + d)] {
                if px >= 0 && py >= 0 && px < shown.width as i32 && py < shown.height as i32 {
                    let i = (py as usize * shown.width as usize + px as usize) * 4;
                    shown.data[i..i + 4].copy_from_slice(&[255, 255, 0, 255]);
                }
            }
        }
    };
    let mut report = format!("cards static: {cards_static}\n");
    let listed = |marks: Vec<(Mark, f32)>, most: usize| {
        let shown: Vec<String> = marks
            .iter()
            .take(most)
            .map(|(m, s)| format!("{} ({:.0}%)", name(m), s * 100.0))
            .collect();
        if shown.is_empty() { "(no mark)".to_string() } else { shown.join("; ") }
    };
    let [x0, y0] = cards_at;
    // (Each card's mark, by its mesh's name.)
    let mut seen: Vec<(String, Mark)> = Vec::new();
    for (i, c) in CARDS.iter().enumerate() {
        let (x, z) = card_corner(i);
        let (sx, sy) = screen(Vec3::new(x0 + x + read_at(c), y0, z + 0.4));
        spot(sx, sy);
        let marks = marks_at(&client, sx, sy, 4);
        if let Some((m, share)) = marks.first().filter(|(_, share)| *share > 0.9) {
            let _ = share;
            seen.push((format!("p{i}"), *m));
        }
        report += &format!("{:20} {}\n", c.name, listed(marks, 1));
    }
    // (The turned squares lie west and east of the placeable's place, half a
    // metre north of it: south and north of it, half a metre west, once
    // turned a quarter anticlockwise.)
    for (what, at) in [("turned, rotatetexture 1", -0.6), ("turned, rotatetexture 0", 0.6)] {
        let (sx, sy) = screen(Vec3::new(18.0, 19.0 + at, 0.05));
        spot(sx, sy);
        report += &format!("{what:34} {}\n", listed(marks_at(&client, sx, sy, 3), 2));
    }
    for (what, _, [x, y], ..) in objects.iter().skip(4) {
        let (sx, sy) = screen(Vec3::new(*x, *y, 0.8));
        spot(sx, sy);
        report += &format!("{what:34} {}\n", listed(marks_at(&client, sx, sy, 16), 3));
    }
    for (what, _, [x, y]) in creatures {
        let (sx, sy) = screen(Vec3::new(x, y, 1.0));
        spot(sx, sy);
        report += &format!("{what:34} {}\n", listed(marks_at(&client, sx, sy, 16), 3));
    }
    save_png(&shown, &dir.join("client_places.png"));
    // Every mark in the picture.
    let mut all: HashMap<Mark, (usize, f64, f64)> = HashMap::new();
    for (k, p) in client.data.chunks(4).enumerate() {
        if let Some(m) = mark(p) {
            let e = all.entry(m).or_default();
            e.0 += 1;
            e.1 += (k % client.width as usize) as f64;
            e.2 += (k / client.width as usize) as f64;
        }
    }
    let mut all: Vec<_> = all.into_iter().filter(|(_, (n, ..))| *n >= 50).collect();
    all.sort_by_key(|(_, (n, ..))| std::cmp::Reverse(*n));
    for (m, (n, sx, sy)) in &all {
        let n = *n as f64;
        report +=
            &format!("mark {:70} {n:7} pixels about ({:.0}, {:.0})\n", name(m), sx / n, sy / n);
    }
    // The band.
    let b = |k| debug_value(&client, k, 1.0);
    let s = |k| (b(k) - 0.5) * 4.0;
    let byte = |k| debug_byte(&client, k);
    let a = |k| (b(k) - 0.5) * 720.0;
    report += &format!(
        "band sentinel {:.3} (0.5)\nm_texture [[{:.3}, {:.3}], [{:.3}, {:.3}]], translation ({:.3}, {:.3}), \
         column 2 ({:.3}, {:.3}, {:.3})\nstaticLighting {} texNoiseBound {}\nmodule hour {} min/h {} \
         dawn {} dusk {} day {} month {} year&255 {}\nfAlphaDiscardValue {:.3}\ntexture0Bound {} \
         envMapCube {} texEnvBound {} texEnvCubeBound {}\nareaFlags {} areaWeatherType {} \
         areaGlobalWind ({:.3}, {:.3}, {:.3}) density {:.3}\ncameraPitch {:.1} cameraYaw {:.1} \
         cameraViewAngle {:.1} playerCameraDist {:.2}\nplayerPosition ({:.2}, {:.2}, {:.2}) \
         cameraPosition ({:.2}, {:.2}, {:.2}) cameraFocus ({:.2}, {:.2}, {:.2}) subvariant {}\n",
        b(0),
        s(1),
        s(2),
        s(3),
        s(4),
        s(5),
        s(6),
        s(41),
        s(42),
        s(43),
        byte(7),
        byte(8),
        byte(9),
        byte(10),
        byte(11),
        byte(12),
        byte(13),
        byte(14),
        byte(15),
        b(16),
        byte(17),
        byte(18),
        byte(19),
        byte(20),
        byte(21),
        byte(22),
        s(23),
        s(24),
        s(25),
        b(40),
        a(26),
        a(27),
        a(28),
        b(29) * 64.0,
        b(30) * 64.0,
        b(31) * 64.0,
        b(32) * 64.0,
        b(33) * 64.0,
        b(34) * 64.0,
        b(35) * 64.0,
        b(36) * 64.0,
        b(37) * 64.0,
        b(38) * 64.0,
        byte(39),
    );
    // The environment maps' means, to tell which is bound.
    let mean_of = |name: &str| {
        let t = <mg_resman::ResMan as mg_render::Assets>::texture(&game.resman, name)?;
        let img = t.texture.to_rgba();
        let n = (img.width * img.height) as f64;
        let mut sum = [0f64; 3];
        for p in img.data.chunks(4) {
            for c in 0..3 {
                sum[c] += f64::from(p[c]) / 255.0;
            }
        }
        Some(sum.map(|v| v / n))
    };
    report += &format!(
        "texUnitEnv's mean: the floor's ({:.3}, {:.3}, {:.3}), the lying square's ({:.3}, {:.3}, {:.3}); \
         chrome1 {:.3?}, {tileset}__ref01 {:.3?}, the set's EnvMap\n",
        b(44),
        b(45),
        b(46),
        b(56),
        b(57),
        b(58),
        mean_of("chrome1"),
        mean_of(&format!("{tileset}__ref01")),
    );
    std::fs::write(dir.join("report.txt"), &report).unwrap();
    eprintln!("{report}\n{}", dir.display());
    if unmarked || std::env::var_os("MG_FACING").is_some() {
        return;
    }

    // Moonglow's choice for the same cards, with the game's own programs
    // (the marked ones taken out of the override first).
    for name in vs.iter().chain(&fs) {
        let _ = std::fs::remove_file(over.join(format!("{name}.shd")));
    }
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped Moonglow's side: no GPU");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let model = mg_area::AreaModel::read(
        &game,
        &are.root,
        &git.root,
        mg_area::tileset(&game, ResRef::from_str(&tileset).unwrap()).ok().as_ref(),
    );
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    r.game_shaders = true;
    r.game_programs = Some(Vec::new());
    let ours = r.render_image(&gpu, &game.resman, &scene, &camera, client.width, client.height);
    save_png(&ours, &dir.join("moonglow_on.png"));
    let drawn = r.game_programs.take().unwrap_or_default();
    let mut wrong = Vec::new();
    for (node, m) in &seen {
        let card = &CARDS[node[1..].parse::<usize>().unwrap()];
        if card.name == "mtr gs+pair" {
            // (The client runs the geometry program, which emits nothing:
            // the board behind shows. wgpu has no geometry stage, so
            // Moonglow draws its own shader there: settled.)
            continue;
        }
        let theirs = (
            vs.get(usize::from(m.vs) - 1).cloned().unwrap_or_default(),
            fs.get(usize::from(m.fs) - 1).cloned().unwrap_or_default(),
            m.no_discard,
        );
        let ours = drawn
            .iter()
            .find(|d| d.model.starts_with("mg_") && d.node.eq_ignore_ascii_case(node))
            .map(|d| (d.vs.clone(), d.fs.clone(), d.no_discard));
        eprintln!("{:20} client {theirs:?}, Moonglow {ours:?}", card.name);
        if ours.as_ref() != Some(&theirs) {
            wrong.push(format!("{}: the client {theirs:?}, Moonglow {ours:?}", card.name));
        }
    }
    assert!(seen.len() >= 25, "{} cards read", seen.len());
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The cards of [`surfaces_in_the_client`], west to east on the ground row:
/// what each is, its bitmap.
const SURFACE_CARDS: [(&str, &str); 6] = [
    ("water (TXI shinywater)", "mgp_water"),
    ("additive", "mgp_add"),
    ("additive decal", "mgp_adddc"),
    ("decal envmap", "mgp_decenv"),
    ("additive envmap", "mgp_addenv"),
    ("plain", "mgp_plain"),
];

/// The particles of [`surfaces_in_the_client`]: what, the emitter's blend,
/// its column on the cards' second row.
const SURFACE_PARTICLES: [(&str, &str, usize); 3] = [
    ("particle Normal", "Normal", 7),
    ("particle Punch-Through", "Punch-Through", 9),
    ("particle Lighten", "Lighten", 11),
];

/// A placeable's model of still, large particles, one an emitter of
/// [`SURFACE_PARTICLES`], in front of the cards' board.
fn particles_model() -> String {
    let emitters: String = SURFACE_PARTICLES
        .iter()
        .map(|(_, blend, column)| {
            let (x, _) = card_corner(*column);
            // (One particle at a time where they are added up: a few
            // in one place, added, are white.)
            let life = if *blend == "Lighten" { 0.95 } else { 30.0 };
            let one = format!(
                "birthrate 1\n  lifeExp {life}\n  velocity 0\n  sizeStart 0.7\n  sizeEnd 0.7\n  \
                 blend {blend}"
            );
            super::probe_emitter(&format!("e{column}"), [x + 0.4, -0.4, 1.5], &one)
                .replace("parent plc_a01", "parent mg_parts")
        })
        .collect();
    format!(
        "newmodel mg_parts\nsetsupermodel mg_parts NULL\nclassification Character\n\
         setanimationscale 1\nbeginmodelgeom mg_parts\nnode dummy mg_parts\n  parent NULL\n\
         endnode\n{emitters}endmodelgeom mg_parts\ndonemodel mg_parts\n"
    )
}

/// Exploration (M5 of `docs/shaders-proposal.md`): which programs the
/// client draws the special surfaces with, marked as
/// [`program_choice_in_the_client`] marks them. A rural area (`ttr01`, its
/// grass on, its north row of tiles water, a skybox) with cards of water,
/// additive and decal textures in front of a decal board, still particles
/// of the three blends, a sword and a shield on the ground. `MG_RUNS`
/// (comma-separated) picks the runs: `marked` (default), `black` (the
/// board's `fst` painting black: what is added over it reads as its own
/// mark), `sky` (the camera raised to the horizon) and `unmarked` (the
/// scene as it looks, for comparing Moonglow's pictures). Reports and
/// pictures in `target/test-output/client_surfaces_<run>/`.
#[test]
#[ignore]
fn surfaces_in_the_client() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let (vs, fs) = programs(&base);
    let runs = std::env::var("MG_RUNS").unwrap_or_else(|_| "marked,black".into());
    for run in runs.split(',').map(str::trim) {
        let dir = scratch_dir(&format!("client_surfaces_{run}"));
        let over = dir.join("user/override");
        std::fs::create_dir_all(&over).unwrap();
        if run != "unmarked" {
            for (list, vertex) in [(&vs, true), (&fs, false)] {
                for (i, name) in list.iter().enumerate() {
                    let src = base.resman.get_named(name, ResType::SHD).unwrap();
                    let black = run == "black" && name == "fst";
                    let text = marked_as(&String::from_utf8_lossy(&src), vertex, i + 1, black);
                    std::fs::write(over.join(format!("{name}.shd")), text).unwrap();
                }
            }
        }
        write_textures(&over);
        let w = |name: &str, bytes: Vec<u8>| std::fs::write(over.join(name), bytes).unwrap();
        // Water: a see-through blue, with the TXI of the rural tileset's
        // water (`ttr01_water01`).
        w("mgp_water.tga", tga(true, |x, y| [40, 90 + ((x + y) % 16) as u8, 160, 140]));
        let water_txi = base.resman.get_named("ttr01_water01", ResType::TXI).unwrap();
        w("mgp_water.txi", water_txi.to_vec());
        for name in ["mgp_adddc", "mgp_decenv", "mgp_addenv"] {
            w(&format!("{name}.tga"), tga(false, checker));
        }
        w("mgp_adddc.txi", b"blending additive\ndecal 1\n".to_vec());
        w("mgp_decenv.txi", b"decal 1\nenvmaptexture chrome1\n".to_vec());
        w("mgp_addenv.txi", b"blending additive\nenvmaptexture chrome1\n".to_vec());
        w("mgwhite.tga", super::white_tga());
        let quads: Vec<Quad> = SURFACE_CARDS
            .iter()
            .enumerate()
            .map(|(i, (_, bitmap))| {
                let (x, z) = card_corner(i);
                Quad {
                    name: format!("s{i}"),
                    at: [x, 0.0, z],
                    corners: standing(0.8),
                    bitmap,
                    extra: "",
                    tverts: true,
                    colors: false,
                }
            })
            .chain(std::iter::once(Quad {
                name: "board".into(),
                at: [-6.0, 0.05, 0.0],
                corners: [[0.0, 0.0, 0.0], [12.0, 0.0, 0.0], [12.0, 0.0, 3.1], [0.0, 0.0, 3.1]],
                bitmap: "mgp_decal",
                extra: "",
                tverts: true,
                colors: false,
            }))
            .collect();
        std::fs::write(over.join("mg_surf.mdl"), model("mg_surf", &quads)).unwrap();
        std::fs::write(over.join("mg_parts.mdl"), particles_model()).unwrap();
        let table = base.resman.get_named("placeables", ResType::TWODA).unwrap();
        let mut table = String::from_utf8_lossy(&table).into_owned();
        let header: Vec<&str> = table.lines().nth(2).unwrap().split_whitespace().collect();
        let model_col = header.iter().position(|c| *c == "ModelName").unwrap() + 1;
        let first = table.lines().count() - 3;
        let armoire: Vec<String> =
            table.lines().nth(3).unwrap().split_whitespace().map(str::to_owned).collect();
        let rows: Vec<String> = ["mg_surf", "mg_parts"]
            .iter()
            .enumerate()
            .map(|(k, model)| {
                let mut row = armoire.clone();
                row[0] = (first + k).to_string();
                row[model_col] = (*model).to_owned();
                row.join(" ")
            })
            .collect();
        if !table.ends_with('\n') {
            table.push('\n');
        }
        table += &(rows.join("\n") + "\n");
        std::fs::write(over.join("placeables.2da"), table).unwrap();

        let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
        let light =
            Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
        let (mut m, mut are) = build_module_on(&game, &dir, light, "ttr01", 1);
        let settings = SETTINGS
            .replace("[graphics.grass]\n\t\tmode = 0", "[graphics.grass]\n\t\tmode = 2")
            .replace(
                "[graphics.skyboxes]\n\t\tenabled = false",
                "[graphics.skyboxes]\n\t\tenabled = true",
            );
        assert!(settings.contains("mode = 2") && settings.contains("enabled = true"));
        std::fs::write(dir.join("user/settings.tml"), settings).unwrap();
        let are_key = *m.keys_of(ResType::ARE).next().unwrap();
        are.root.set("WindPower", Value::Int(2));
        // (The north row water: ttr01's tile 20, `ttr01_b20_01`, all water.)
        // (`waterband`: the row south of the player's water too, under
        // the band of uniforms.)
        let width = are.root.int("Width").unwrap_or(4) as usize;
        let water_rows: &[usize] = if run == "waterband" { &[1, 3] } else { &[3] };
        for (i, t) in are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
            if water_rows.contains(&(i / width)) {
                t.set("Tile_ID", Value::Int(20));
                t.set("Tile_Height", Value::Int(0));
            }
        }
        m.set_gff(are_key, &are).unwrap();
        let mut enter: String =
            ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
        if run == "sky" {
            // (Once the player is in the area: from its OnClientEnter, the
            // camera doesn't move.)
            enter = enter.replace(
                "AssignCommand(pc, SetCameraFacing(90.0, 12.0, 45.0, CAMERA_TRANSITION_TYPE_SNAP));",
                "DelayCommand(1.0, AssignCommand(pc, SetCameraFacing(90.0, 12.0, 89.0, \
                 CAMERA_TRANSITION_TYPE_SNAP)));",
            );
        }
        m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
        let git_key = *m.keys_of(ResType::GIT).next().unwrap();
        let mut git = m.gff(&git_key).unwrap().unwrap();
        let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let cards_at = [18.0f32, 23.0];
        let placed: Vec<_> = (0..2)
            .map(|k| {
                let at = Placement { position: [cards_at[0], cards_at[1], 0.0], rotation: 0.0 };
                let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
                p.set("Appearance", Value::Dword((first + k) as u32));
                p.set("Static", Value::Byte(0));
                p
            })
            .collect();
        git.root.set("Placeable List", Value::List(placed));
        let items = [
            ("short sword", "nw_wswss001", [16.5f32, 21.0]),
            ("small shield", "nw_ashsw001", [19.5, 21.0]),
        ];
        let placed: Vec<_> = items
            .iter()
            .map(|(_, name, [x, y])| {
                let bp = Gff::read(&game.resman.get_named(name, ResType::UTI).unwrap()).unwrap();
                let at = Placement { position: [*x, *y, 0.0], rotation: 0.0 };
                instance(&placing, ResType::UTI, &bp.root, at, &[]).unwrap()
            })
            .collect();
        git.root.set(mg_area::ObjectKind::Item.list(), Value::List(placed));
        m.set_gff(git_key, &git).unwrap();
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let client = if run == "unmarked" {
            client_capture(&dir, "MgScene", "surfaces_unmarked")
        } else {
            client_screenshot(&dir, "MgScene")
        };
        let Some(client) = client else {
            panic!("{run}: the client did not reach the scene (see {})", dir.display());
        };
        save_png(&client, &dir.join("client.png"));
        if run == "unmarked" {
            surfaces_in_moonglow(&game, &are, &git, &client, &dir);
            continue;
        }

        let (wd, ht) = (client.width as f32, client.height as f32);
        let camera =
            fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
        let vp = camera.projection(wd / ht) * camera.view();
        let screen = |p: Vec3| {
            let c = vp * p.extend(1.0);
            let n = c.truncate() / c.w;
            ((n.x * 0.5 + 0.5) * wd, (0.5 - n.y * 0.5) * ht)
        };
        let name = |m: &Mark| {
            let v = vs.get(usize::from(m.vs).wrapping_sub(1)).map_or("?", String::as_str);
            let f = fs.get(usize::from(m.fs).wrapping_sub(1)).map_or("?", String::as_str);
            format!(
                "{v}/{f} NO_DISCARD {} FRAGMENT_LIGHTING {} KEYHOLING_ENABLED {} turn {}",
                u8::from(m.no_discard),
                u8::from(m.fragment_lighting),
                u8::from(m.keyholing),
                m.rot
            )
        };
        let listed = |marks: Vec<(Mark, f32)>, most: usize| {
            let shown: Vec<String> = marks
                .iter()
                .take(most)
                .map(|(m, s)| format!("{} ({:.0}%)", name(m), s * 100.0))
                .collect();
            if shown.is_empty() { "(no mark)".to_string() } else { shown.join("; ") }
        };
        let mut report = format!("run {run} ({}x{})\n", client.width, client.height);
        let [x0, y0] = cards_at;
        let mut places = Vec::new();
        for (i, (what, _)) in SURFACE_CARDS.iter().enumerate() {
            let (x, z) = card_corner(i);
            places.push((what.to_string(), screen(Vec3::new(x0 + x + 0.4, y0, z + 0.4)), 4));
        }
        for (what, _, column) in SURFACE_PARTICLES {
            let (x, _) = card_corner(column);
            places.push((what.to_string(), screen(Vec3::new(x0 + x + 0.4, y0 - 0.4, 1.5)), 4));
        }
        for (what, _, [x, y]) in items {
            places.push((what.to_string(), screen(Vec3::new(x, y, 0.05)), 8));
        }
        places.push(("water tile".into(), screen(Vec3::new(25.0, 35.0, 0.0)), 8));
        places.push(("grass tile".into(), screen(Vec3::new(22.0, 16.0, 0.0)), 8));
        let mut shown = client.clone();
        for (what, (sx, sy), r) in &places {
            report += &format!("{what:26} {}\n", listed(marks_at(&client, *sx, *sy, *r), 3));
            for d in -4i32..=4 {
                for (px, py) in [(*sx as i32 + d, *sy as i32), (*sx as i32, *sy as i32 + d)] {
                    if px >= 0 && py >= 0 && px < shown.width as i32 && py < shown.height as i32 {
                        let i = (py as usize * shown.width as usize + px as usize) * 4;
                        shown.data[i..i + 4].copy_from_slice(&[255, 255, 0, 255]);
                    }
                }
            }
        }
        save_png(&shown, &dir.join("client_places.png"));
        // Every mark in the picture, with where it lies.
        let mut all: HashMap<Mark, (usize, f64, f64, f64, f64)> = HashMap::new();
        for (k, p) in client.data.chunks(4).enumerate() {
            if let Some(m) = mark(p) {
                let (x, y) =
                    ((k % client.width as usize) as f64, (k / client.width as usize) as f64);
                let e = all.entry(m).or_insert((0, 0.0, 0.0, f64::MAX, f64::MIN));
                e.0 += 1;
                e.1 += x;
                e.2 += y;
                e.3 = e.3.min(y);
                e.4 = e.4.max(y);
            }
        }
        let mut all: Vec<_> = all.into_iter().filter(|(_, (n, ..))| *n >= 30).collect();
        all.sort_by_key(|(_, (n, ..))| std::cmp::Reverse(*n));
        for (m, (n, sx, sy, top, bottom)) in &all {
            let n = *n as f64;
            report += &format!(
                "mark {:70} {n:7} pixels about ({:.0}, {:.0}), rows {top:.0}..{bottom:.0}\n",
                name(m),
                sx / n,
                sy / n
            );
        }
        if run == "waterband" {
            // The band, over the near water: what `fswater` is given.
            let b = |k| debug_value(&client, k, 1.0);
            let byte = |k| debug_byte(&client, k);
            let s = |k| (b(k) - 0.5) * 4.0;
            report += &format!(
                "band sentinel {:.3} (0.5)\nenvMapCube {} texEnvBound {} texEnvCubeBound {} \
                 texture0Bound {}\nareaGlobalWind ({:.3}, {:.3}, {:.3}) areaFlags {}\n\
                 texUnitEnv's mean ({:.3}, {:.3}, {:.3}); texUnitEnvCube's +Z ({:.3}, {:.3}, {:.3}), \
                 +X ({:.3}, {:.3}, {:.3})\n",
                b(0),
                byte(18),
                byte(19),
                byte(20),
                byte(17),
                s(23),
                s(24),
                s(25),
                byte(21),
                b(44),
                b(45),
                b(46),
                b(47),
                b(48),
                b(49),
                b(50),
                b(51),
                b(52),
            );
            let mean_of = |name: &str| {
                let t = <mg_resman::ResMan as mg_render::Assets>::texture(&game.resman, name)?;
                let img = t.texture.to_rgba();
                let n = (img.width * img.height) as f64;
                let mut sum = [0f64; 3];
                for p in img.data.chunks(4) {
                    for c in 0..3 {
                        sum[c] += f64::from(p[c]) / 255.0;
                    }
                }
                Some(sum.map(|v| v / n))
            };
            for name in ["chrome1", "ttr01__ref01", "ttr01__env4", "ttr01__env0", "ttr01__env"] {
                report += &format!("{name}'s mean {:.3?}\n", mean_of(name));
            }
        }
        std::fs::write(dir.join("report.txt"), &report).unwrap();
        eprintln!("{report}\n{}", dir.display());
    }
}

/// The regions of [`surfaces_in_moonglow`]'s comparison: what, and where
/// (fractions of the picture, top-left origin), clear of the GUI.
const SURFACE_REGIONS: [(&str, [f32; 4]); 7] = [
    ("sky", [0.02, 0.01, 0.90, 0.12]),
    ("water, far", [0.02, 0.15, 0.90, 0.26]),
    ("grass field", [0.10, 0.50, 0.68, 0.78]),
    ("decal board", [0.20, 0.32, 0.65, 0.37]),
    ("particle Normal", [0.465, 0.380, 0.490, 0.410]),
    ("particle Punch-Through", [0.550, 0.380, 0.575, 0.410]),
    ("particle Lighten", [0.632, 0.380, 0.657, 0.410]),
];

/// Moonglow's side of [`surfaces_in_the_client`]'s `unmarked` run: the
/// same area drawn with the Shaders switch off and on (`moonglow_off.png`,
/// `moonglow_on.png`), each region's mean colour against the client's, and
/// the programs the game path chose for the surfaces, held to the client's
/// (as the marked runs read them).
fn surfaces_in_moonglow(game: &GameData, are: &Gff, git: &Gff, client: &Rgba, dir: &Path) {
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped Moonglow's side: no GPU");
        return;
    };
    let tileset = mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).ok();
    let model = mg_area::AreaModel::read(game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, game, &model);
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let view = mg_area::View {
        fog: false,
        time: 6.0,
        grass: Some(mg_area::Sight::of(&camera, client.width as f32 / client.height as f32)),
        ..mg_area::View::of(&model)
    };
    let mut scene = area_scene.scene(&model, &view);
    let mut sims = HashMap::new();
    scene.particles = area_scene.particles(&model, &view, &mut sims, 0.0, camera.view());
    let (w, h) = (client.width, client.height);
    let mut pictures = Vec::new();
    let mut chosen = Vec::new();
    // (Off, on (shiny water by `fswater` with the framebuffer's copies,
    // `GAME_WATER`), and on with Moonglow's own water, as before.)
    for (on, water, name) in [
        (false, false, "moonglow_off"),
        (true, mg_render::GAME_WATER, "moonglow_on"),
        (true, false, "moonglow_ownwater"),
    ] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
        r.game_shaders = on;
        r.game_water = water;
        r.game_programs = Some(Vec::new());
        let img = r.render_image(&gpu, &game.resman, &scene, &camera, w, h);
        save_png(&img, &dir.join(format!("{name}.png")));
        if on && water == mg_render::GAME_WATER {
            chosen = r.game_programs.take().unwrap_or_default();
            eprintln!("failures: {:?}", r.game_shader_failures());
        }
        pictures.push(img);
    }
    // A close look, low over the grass towards the cards, the water and
    // the sky (for looking at, not compared: the client's camera can't be
    // put there).
    let close = Camera {
        eye: Vec3::new(20.0, 17.0, 1.4),
        target: Vec3::new(20.0, 30.0, 1.6),
        fov_y: 60f32.to_radians(),
        near: 0.1,
        far: 200.0,
    };
    for on in [false, true] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
        r.game_shaders = on;
        let img = r.render_image(&gpu, &game.resman, &scene, &close, w, h);
        save_png(&img, &dir.join(if on { "close_on.png" } else { "close_off.png" }));
    }
    let mut report = String::new();
    // (And each card's upper half, above the grass, a little in from its
    // edges.)
    let vp = camera.projection(w as f32 / h as f32) * camera.view();
    let fraction = |p: Vec3| {
        let c = vp * p.extend(1.0);
        let n = c.truncate() / c.w;
        (n.x * 0.5 + 0.5, 0.5 - n.y * 0.5)
    };
    let cards = SURFACE_CARDS.iter().enumerate().map(|(i, (what, _))| {
        let (x, z) = card_corner(i);
        let (x0, y0) = fraction(Vec3::new(18.0 + x + 0.15, 23.0, z + 0.75));
        let (x1, y1) = fraction(Vec3::new(18.0 + x + 0.65, 23.0, z + 0.45));
        (*what, [x0, y0, x1, y1])
    });
    for (what, rect) in SURFACE_REGIONS.into_iter().chain(cards) {
        let c = mean(client, rect);
        let [off, on, water] = [&pictures[0], &pictures[1], &pictures[2]].map(|p| mean(p, rect));
        let away = |m: [f32; 3]| (0..3).map(|i| (m[i] - c[i]).abs()).sum::<f32>() / 3.0;
        report += &format!(
            "{what:24} client {c:5.1?}  off {off:5.1?} ({:.1})  on {on:5.1?} ({:.1})  \
             on, Moonglow's water {water:5.1?} ({:.1})\n",
            away(off),
            away(on),
            away(water)
        );
    }
    report += &format!(
        "the picture: off {:.2}, on {:.2}, on with Moonglow's water {:.2} of 255 from the \
         client's\n",
        super::picture_difference(client, &pictures[0]),
        super::picture_difference(client, &pictures[1]),
        super::picture_difference(client, &pictures[2])
    );
    // What the client chose (the marked runs), and Moonglow.
    let pair = |model: &dyn Fn(&str) -> bool, node: &dyn Fn(&str) -> bool| {
        chosen
            .iter()
            .find(|g| model(&g.model) && node(&g.node))
            .map(|g| (g.vs.as_str(), g.fs.as_str(), g.no_discard))
    };
    let card = |i: usize| {
        let node = format!("s{i}");
        pair(&|m: &str| m == "mg_surf", &move |n: &str| n.eq_ignore_ascii_case(&node))
    };
    // (Shiny water: the client's pair, `vswater`/`fswater` (read in the
    // marked runs), with the framebuffer's copies while `GAME_WATER`.)
    let water = mg_render::GAME_WATER.then_some(("vswater", "fswater", true));
    let expected: Vec<(&str, Option<Chosen>, Option<Chosen>)> = vec![
        ("water card", card(0), water),
        ("additive card", card(1), Some(("vslit", "fslit", true))),
        ("additive decal card", card(2), Some(("vsvt", "fst", true))),
        ("decal envmap card", card(3), Some(("vsvt_sm", "fst_sm", true))),
        ("additive envmap card", card(4), Some(("vslit_sm", "fslit_sm", true))),
        (
            "water tile's water",
            chosen
                .iter()
                .find(|g| g.model.eq_ignore_ascii_case("ttr01_b20_01") && g.fs == "fswater")
                .map(|g| (g.vs.as_str(), g.fs.as_str(), g.no_discard)),
            water,
        ),
        (
            "grass",
            pair(&|m: &str| m.ends_with("_grass"), &|_| true),
            Some(("vsgrass", "fsgrass", false)),
        ),
        (
            "particle Normal",
            pair(&|m: &str| m == "(particles)", &|n: &str| n == "Normal"),
            Some(("vsparticle", "fsparticle", true)),
        ),
        (
            "particle Lighten",
            pair(&|m: &str| m == "(particles)", &|n: &str| n == "Lighten"),
            Some(("vsparticle", "fsparticle", true)),
        ),
        (
            "particle Punch-Through",
            pair(&|m: &str| m == "(particles)", &|n: &str| n == "PunchThrough"),
            Some(("vsparticle", "fsparticle", false)),
        ),
    ];
    let sky_model = area_scene_sky_name(&scene);
    let sky = sky_model
        .as_deref()
        .and_then(|name| pair(&|m: &str| m.eq_ignore_ascii_case(name), &|_| true));
    report +=
        &format!("skybox {sky_model:?}: Moonglow {sky:?} (the client: vsvt/fst, NO_DISCARD 1)\n");
    let mut wrong = Vec::new();
    for (what, ours, theirs) in &expected {
        report += &format!("{what:24} Moonglow should {theirs:?}, draws {ours:?}\n");
        if ours != theirs {
            wrong.push(format!("{what}: the client {theirs:?}, Moonglow {ours:?}"));
        }
    }
    if sky_model.is_some() && sky != Some(("vsvt", "fst", true)) {
        wrong.push(format!("skybox: Moonglow {sky:?}"));
    }
    std::fs::write(dir.join("report.txt"), &report).unwrap();
    eprintln!("{report}{}", dir.display());
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A program pair and its `NO_DISCARD`, as chosen for a mesh.
type Chosen<'a> = (&'a str, &'a str, bool);

/// The skybox's model name in a scene, if it has one.
fn area_scene_sky_name(scene: &Scene) -> Option<String> {
    scene.sky.as_ref().map(|s| s.model.model.name.clone())
}
