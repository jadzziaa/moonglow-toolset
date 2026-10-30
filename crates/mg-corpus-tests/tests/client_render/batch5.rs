//! The deferred sprint's fifth client batch (2026-10-30): readings of the
//! client that answer the deferred list's questions, beside the first
//! four batches' (`readings.rs`, `batch2.rs`, `batch3.rs`, `batch4.rs`,
//! whose scaffolding they use). Each writes the client's pictures to its
//! scratch directory (`target/test-output/client_reading_<name>/`) and
//! prints where; what was seen is in its comment.
//!
//! Most patch one of the game's programs in the scratch user directory's
//! `override`, which the client and Moonglow's game path both read (the
//! game is opened with that user directory): a part of what a program
//! computes is painted into the picture of both.
//!
//! Needs the game and the off-screen display; run by hand, one client at a
//! time:
//! `DISPLAY=:2 cargo test -p mg-corpus-tests --release --test client_render batch5 -- --ignored --nocapture --test-threads 1`.

use super::readings::{INVISIBLE, Reading, placeables_with};
use super::*;

/// The game's program `name` (a `.shd`), its line ends Unix's.
fn program(game: &GameData, name: &str) -> String {
    let src =
        game.resman.get_named(name, ResType::SHD).unwrap_or_else(|e| panic!("{name}.shd: {e}"));
    String::from_utf8_lossy(&src).replace("\r\n", "\n")
}

/// `src` with `find` (which must be there) replaced by `with`.
fn patched(src: &str, find: &str, with: &str) -> String {
    assert!(src.contains(find), "{find:?} not in the program");
    src.replacen(find, with, 1)
}

/// The game read with no user directory of ours.
fn base_game() -> GameData {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    GameData::open(&GameInstall::new(&root, None, "en")).unwrap()
}

/// The client's settings with skyboxes on.
fn with_skybox(settings: &str) -> String {
    let s = settings.replace(
        "[graphics.skyboxes]\n\t\tenabled = false",
        "[graphics.skyboxes]\n\t\tenabled = true",
    );
    assert!(s.contains("[graphics.skyboxes]\n\t\tenabled = true"));
    s
}

/// The mean colour of `r` (fractions, top-left origin) in each picture of
/// `dir` named in `names` (those there), printed after `what`.
fn region_means(dir: &Path, what: &str, r: [f32; 4], names: &[&str]) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    let mut line = format!("{what:28}");
    for name in names {
        let path = dir.join(format!("{name}.png"));
        if !path.exists() {
            continue;
        }
        let m = mean(&common::read_png(&path), r);
        line += &format!("  {name} {m:5.1?}");
        out.push(m);
    }
    eprintln!("{line}");
    out
}

/// The parts of `fswater`'s far water, one a run: `whole` (the program as
/// it is), and with the fog left out, `nofog` (all of it), `reflection`
/// (the reflection times its Fresnel weight), `refraction` (the
/// framebuffer copy's colour as far as the water is clear) and `liquid`
/// (the water's own colour, lit beneath, as far as it isn't clear).
const WATER_PARTS: [(&str, Option<&str>); 5] = [
    ("whole", None),
    ("nofog", Some("ColorLiquid.rgb + ColorReflection.rgb * fFresnelSpecularity")),
    ("reflection", Some("ColorReflection.rgb * fFresnelSpecularity")),
    ("refraction", Some("ColorRefraction * fRefractionTransparency")),
    ("liquid", Some("WaterBaseColor.rgb * SubsurfaceLight * (1.0 - fRefractionTransparency)")),
];

/// The far water by its parts (the deferred list's "The special surfaces",
/// shiny water: a rural area's far water 10.9 of 255 from the client with
/// the game's `fswater`, not looked into): ttr01 with its north row of
/// tiles water (tile 20), a skybox, wind 2, seen from the reference
/// scenes' camera, `fswater` patched in the override to paint one part
/// of what it adds up ([`WATER_PARTS`]), in the client and in Moonglow's
/// area view with the Shaders switch on (which reads the same override).
/// `MG_PARTS` (comma-separated) picks the parts. Each part's far water
/// (the region `surfaces_in_moonglow` compares) is printed for the
/// client, Moonglow on (`moonglow_on`) and Moonglow's own water
/// (`moonglow`, the patch not read).
///
/// Seen (2026-10-30, 89.8193.37), the far water's mean (R, G, B) in the
/// client against Moonglow's with the Shaders switch on: `whole` 63.1,
/// 76.0, 82.4 against 63.4, 76.5, 83.0 (Moonglow's own water 62.0, 74.4,
/// 83.9) with Moonglow's fog on; with it off, as `surfaces_in_moonglow`
/// and the readings' `moonglow_beside` draw, 54.3, 65.4, 69.1: the 10.9
/// of 255 was the comparison's fog, which the client lays over the far
/// water at a fog amount of 0 too (its start 30 m before the fog clip
/// distance, as Moonglow's, `fog_uniforms_match_the_client`). By part,
/// fog left out: `liquid` 43.0, 53.7, 57.8 against 42.4, 53.4, 57.5;
/// `reflection` 22.9, 23.9, 19.5 against 25.0, 26.1, 21.3; `refraction`
/// 20.0, 23.5, 26.8 against 23.5, 27.5, 31.0 (Moonglow's copy shows the
/// bed's dark shapes where the client's is an even dark blue); `nofog`
/// 52.1, 62.5, 65.4 against 54.3, 65.4, 69.1.
#[test]
#[ignore]
fn reading_far_water_by_part() {
    let base = base_game();
    let fswater = program(&base, "fswater");
    let parts = std::env::var("MG_PARTS").unwrap_or_default();
    let far = [0.02, 0.15, 0.90, 0.26];
    for (part, sum) in WATER_PARTS {
        if !parts.is_empty() && !parts.split(',').any(|p| p == part) {
            continue;
        }
        let mut overrides = Vec::new();
        if let Some(sum) = sum {
            let src = patched(
                &fswater,
                "FragmentColor.rgb = ColorLiquid.rgb + ColorReflection.rgb * fFresnelSpecularity;",
                &format!("FragmentColor.rgb = {sum};"),
            );
            let src = patched(&src, "\t\tApplyFog(FragmentColor);", "");
            overrides.push(("fswater.shd", src.into_bytes()));
        }
        let light =
            Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
        let mut r = Reading::new(&format!("far_water_{part}"), "ttr01", light, &overrides);
        r.are.root.set("SkyBox", Value::Byte(1));
        r.are.root.set("WindPower", Value::Int(2));
        let width = r.are.root.int("Width").unwrap_or(4) as usize;
        for (i, t) in r.are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
            if i / width == 3 {
                t.set("Tile_ID", Value::Int(20));
                t.set("Tile_Height", Value::Int(0));
            }
        }
        // (Moonglow with its fog, as the client fogs the far water.)
        let (dir, _) = r.shoot_beside(INVISIBLE, &with_skybox(SETTINGS), &|v| v.fog = true);
        let names = ["client", "moonglow_on", "moonglow"];
        region_means(&dir, &format!("{part}: water, far"), far, &names);
    }
}

/// What the game's `fst` paints, one a run: `look` (as it is), the
/// draw's `globalColor` and its alpha, its texture and its alpha, and
/// what the program writes before blending (`out`, `out_a`).
const SKY_PARTS: [(&str, Option<&str>); 7] = [
    ("look", None),
    ("global", Some("vec4(globalColor.rgb, 1.0)")),
    ("global_a", Some("vec4(vec3(globalColor.a), 1.0)")),
    ("tex", Some("vec4(texture2D(texUnit0, vTexCoords).rgb, 1.0)")),
    ("tex_a", Some("vec4(vec3(texture2D(texUnit0, vTexCoords).a), 1.0)")),
    ("out", Some("vec4(FragmentColor.rgb, 1.0)")),
    ("out_a", Some("vec4(vec3(FragmentColor.a), 1.0)")),
];

/// The sky's fade (the deferred list's "The special surfaces": the
/// horizon's band, `skyfade1` in the fog's colour, stays Moonglow's; what
/// the engine draws it with and gives it wasn't read): ttr01 with a
/// skybox and a red fog colour (fog amount 0), no grass, the camera low
/// over the field towards the horizon (`SetCameraFacing(90, 12, 89)`, a
/// second after entry), `fst` (which draws the skybox and the band:
/// `surfaces_in_the_client`'s `sky` run marks every pixel of both
/// `vsvt`/`fst`) patched to paint one of [`SKY_PARTS`]. `MG_PARTS` picks.
/// Each run prints the mean colour of strips of the picture's rows (the
/// sky's top down to the horizon), Moonglow's (`moonglow_on`, which reads
/// the patched `fst` for the skybox only; its fade is its own) beside.
///
/// Seen (2026-10-30, 89.8193.37): the band is `fst` with `globalColor` the
/// fog's colour (made linear: 149, 11, 2 painted raw for 200, 60, 30) and
/// alpha 1, times the texture, which is white there (`tex` 255) with its
/// alpha falling off (`tex_a` 0.50, 0.16, 0.40 down the band); what it
/// writes (`out`) is the fog's colour, its alpha the texture's, blended
/// over the sky. Moonglow took the texture's red (1 across the band) for
/// its alpha: a solid red band where the client's is a soft one (fixed:
/// `the_sky_fade_is_laid_over_the_sky_by_its_alpha`; after, the band's
/// rows within 10 of the client's).
#[test]
#[ignore]
fn reading_sky_fade() {
    let base = base_game();
    let fst = program(&base, "fst");
    let parts = std::env::var("MG_PARTS").unwrap_or_default();
    for (part, paint) in SKY_PARTS {
        if !parts.is_empty() && !parts.split(',').any(|p| p == part) {
            continue;
        }
        let mut overrides = Vec::new();
        if let Some(paint) = paint {
            let src =
                patched(&fst, "gl_FragColor = FragmentColor;", &format!("gl_FragColor = {paint};"));
            overrides.push(("fst.shd", src.into_bytes()));
        }
        let light =
            Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
        let mut r = Reading::new(&format!("sky_fade_{part}"), "ttr01", light, &overrides);
        r.are.root.set("SkyBox", Value::Byte(1));
        // (Red: R 200, G 60, B 30; a colour's bytes are 0x00BBGGRR.)
        r.are.root.set("SunFogColor", Value::Dword(0x001E_3CC8));
        r.are.root.set("MoonFogColor", Value::Dword(0x001E_3CC8));
        let (dir, _) = super::batch4::shoot_close(
            r,
            INVISIBLE,
            &with_skybox(SETTINGS),
            (90.0, 12.0, 89.0),
            &|v| v.fog = true,
        );
        eprintln!("{part}: {}", dir.display());
        for k in 0..20 {
            let y = k as f32 * 0.03;
            region_means(
                &dir,
                &format!("{part}: rows {y:.2}..{:.2}", y + 0.03),
                [0.05, y, 0.60, y + 0.03],
                &["client", "moonglow_on"],
            );
        }
    }
}

/// A card's fragment program that paints the framebuffer's colour copy
/// where the card is (`texFBColor` at its own pixel), a magenta band down
/// each side to find it by.
const COPY_FS: &str = r#"
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
    if (vVertexTexCoords.x < 0.08 || vVertexTexCoords.x > 0.92) c = vec3(1.0, 0.0, 1.0);
    gl_FragColor = vec4(c, 1.0);
}
"#;

/// [`COPY_FS`], or with `MG_COPY_PAINT=uv` the coordinates it samples the
/// copy at painted as red and green.
fn copy_fs() -> String {
    let find = "vec3 c = texture2D(texFBColor, uv).rgb;";
    match std::env::var("MG_COPY_PAINT").as_deref() {
        Ok("uv") => patched(COPY_FS, find, "vec3 c = vec3(uv, 0.0);"),
        // (The copy 0.3 of the picture lower: the near field.)
        Ok("down") => {
            patched(COPY_FS, find, "vec3 c = texture2D(texFBColor, uv - vec2(0.0, 0.3)).rgb;")
        }
        // (0.08 higher: the water above the cards.)
        Ok("up") => {
            patched(COPY_FS, find, "vec3 c = texture2D(texFBColor, uv + vec2(0.0, 0.08)).rgb;")
        }
        _ => COPY_FS.to_string(),
    }
}

/// The cards of [`reading_framebuffer_copy_over_water`], west to east:
/// their MTRs' lines beside the shader pair.
const COPY_CARDS: [(&str, &str); 4] = [
    ("mgfb_solid", ""),
    ("mgfb_tr", "transparency 1\n"),
    ("mgfb_sf1", "transparency 1\nsample_framebuffer 1\n"),
    ("mgfb_sf2", "transparency 1\nsample_framebuffer 2\n"),
];

/// Whether the framebuffer's copy a see-through mesh reads holds the
/// water (the deferred list's "Tier A of the game's shaders": Moonglow
/// copies after the solid meshes only): ttr01 with its north row of
/// tiles water, and standing south of it four cards 1.6 m wide and 2 m
/// high, each painting the copy where it stands ([`COPY_FS`], magenta
/// edges), their MTRs [`COPY_CARDS`]: solid, `transparency 1`, and with
/// `sample_framebuffer` 1 and 2 (the wiki: drawn "near the end of the
/// frame", still before shiny water). Behind a card's upper half, from
/// the camera, is the water: where the copy was taken after the water,
/// the card shows it there; before, the ground under it.
///
/// Seen (2026-10-30, 89.8193.37), with `MG_COPY_PAINT=up` (the copy 0.08
/// of the picture above each card, the water there in the picture): the
/// solid card and the `transparency 1` one show the water (77, 89, 97
/// against the water's 77, 90, 97 above them), the `sample_framebuffer`
/// 1 and 2 cards a lighter blue (92, 111, 128), the copy without the
/// water; all four of Moonglow's show the copy without the water (104,
/// 126, 145). So the client's copy that a solid or `transparency 1` mesh
/// reads holds the water (likely the last copy of the frame before: the
/// camera is still), and a `sample_framebuffer` mesh reads one taken
/// before the water, as Moonglow's are. `MG_COPY_PAINT=uv` showed the
/// client's coordinates right (0.385 at 39% across, from the bottom).
#[test]
#[ignore]
fn reading_framebuffer_copy_over_water() {
    let base = base_game();
    let mut nodes = Vec::new();
    let mut overrides: Vec<(&str, Vec<u8>)> = vec![
        ("mgfbvs.shd", program(&base, "vslit").into_bytes()),
        // (`MG_COPY_PAINT=uv`: the copy's coordinates painted instead.)
        ("mgfbfs.shd", copy_fs().into_bytes()),
        ("mgwhite.tga", super::white_tga()),
    ];
    let mtrs: Vec<(String, String)> = COPY_CARDS
        .iter()
        .map(|(name, lines)| {
            let mtr =
                format!("texture0 mgwhite\ncustomshadervs mgfbvs\ncustomshaderfs mgfbfs\n{lines}");
            (format!("{name}.mtr"), mtr)
        })
        .collect();
    for (i, (name, _)) in COPY_CARDS.iter().enumerate() {
        let x = -4.2 + i as f32 * 2.2;
        nodes.push(format!(
            "node trimesh {name}\n  parent mg_fbcards\n  position {x} 0 0\n  \
             orientation 0 0 0 0\n  ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  \
             shininess 1\n  bitmap {name}\n  render 1\n  shadow 0\n  verts 4\n    \
             0 0 0\n    1.6 0 0\n    1.6 0 2\n    0 0 2\n  tverts 4\n    0 0 0\n    1 0 0\n    \
             1 1 0\n    0 1 0\n  faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n"
        ));
    }
    for (file, mtr) in &mtrs {
        overrides.push((file, mtr.clone().into_bytes()));
    }
    let model = super::batch4::probe_model("mg_fbcards", &nodes);
    let (table, first) = placeables_with(&base, &[("mg_fbcards", 0)]);
    overrides.push(("mg_fbcards.mdl", model.into_bytes()));
    overrides.push(("placeables.2da", table.into_bytes()));
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("framebuffer_copy_over_water", "ttr01", light, &overrides);
    r.are.root.set("SkyBox", Value::Byte(1));
    let width = r.are.root.int("Width").unwrap_or(4) as usize;
    for (i, t) in r.are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
        if i / width == 3 {
            t.set("Tile_ID", Value::Int(20));
            t.set("Tile_Height", Value::Int(0));
        }
    }
    r.place(first, [20.0, 28.5, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = r.shoot_beside(INVISIBLE, &with_skybox(SETTINGS), &|v| v.fog = true);
    eprintln!("{}", dir.display());
}

/// A custom vertex program: the stock lit one, normal-mapped (it passes
/// the mesh's tangent and handedness on).
const TANGENT_VS: &str = "\
#define SHADER_TYPE 1
#define FOG 0
#define KEYHOLING 0
#define NORMAL_MAP 1
#define LIGHTING 1
#include \"inc_standard\"
void main()
{
    ApplyStandardShader();
}
";

/// A custom fragment program painting the tangent it is given (in the
/// world, as the stock debug mode `SHADER_DEBUG_MODE_VERTEX_TANGENTS`
/// paints it: × 0.5 + 0.5), or with `MG_TAN_PAINT=handedness` its
/// handedness as that mode paints it (red none, blue +1, green −1).
fn tangent_fs() -> String {
    let paint = match std::env::var("MG_TAN_PAINT").as_deref() {
        Ok("handedness") => {
            "vec3(1.0 - abs(fTextureHandedness), -fTextureHandedness, fTextureHandedness)"
        }
        _ => "mat3(m_view_inv) * vVertexTangent * 0.5 + 0.5",
    };
    format!(
        "#define SHADER_TYPE 2\n#define FOG 0\n#define KEYHOLING 0\n#define LIGHTING 1\n\
         #define NORMAL_MAP 1\n#define SPECULAR_MAP 0\n#define ROUGHNESS_MAP 0\n\
         #define HEIGHT_MAP 0\n#define SELF_ILLUMINATION_MAP 0\n#define ENVIRONMENT_MAP 0\n\
         #include \"inc_standard\"\nvoid main()\n{{\n    FragmentColor = vec4(1.0);\n    \
         SetupStandardShaderInputs();\n    gl_FragColor = vec4({paint}, 1.0);\n}}\n"
    )
}

/// The cards of [`reading_tangents`], west to east: their name, their
/// bitmap (the custom pair's MTR: `mgtan` without a render hint, `mgtanh`
/// with one) and their mesh's other lines.
const TANGENT_CARDS: [(&str, &str, &str); 6] = [
    ("hint", "mgtan", "  renderhint NormalAndSpecMapped\n"),
    ("hintmesh", "mgtan", "  renderhint NormalAndSpecMappedMesh\n"),
    ("tangents", "mgtan", "  renderhint NormalTangents\n"),
    ("texture1", "mgtan", "  texture1 mgflatnm\n"),
    ("plain", "mgtan", ""),
    ("mtrhint", "mgtanh", ""),
];

/// A model of [`TANGENT_CARDS`], each a standing card 1.2 m wide and 1.2
/// m high with a mirrored seam down its middle (its texture's u from 0 at
/// the west edge to 1 at the middle and back to 0 at the east edge, the
/// middle's vertices shared): the tangent turns about there.
fn tangent_cards(name: &str) -> String {
    let nodes: Vec<String> = TANGENT_CARDS
        .iter()
        .enumerate()
        .map(|(i, (card, bitmap, lines))| {
            let x = -4.35 + i as f32 * 1.5;
            format!(
                "node trimesh {card}\n  parent {name}\n  position {x} 0 0\n  orientation 0 0 0 0\n  \
                 ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n\
                 {lines}  render 1\n  shadow 0\n  verts 6\n    0 0 0\n    0.6 0 0\n    1.2 0 0\n    \
                 0 0 1.2\n    0.6 0 1.2\n    1.2 0 1.2\n  tverts 6\n    0 0 0\n    1 0 0\n    \
                 0 0 0\n    0 1 0\n    1 1 0\n    0 1 0\n  faces 4\n    0 1 4 1 0 1 4 0\n    \
                 0 4 3 1 0 4 3 0\n    1 2 5 1 1 2 5 0\n    1 5 4 1 1 5 4 0\nendnode\n"
            )
        })
        .collect();
    super::batch4::probe_model(name, &nodes)
}

/// The tangents a custom pair is given (the deferred list's "A custom
/// shader's tangents" and the shaders' Tier A, "The tangents the renderer
/// makes"): north of the player, two rows of six cards
/// ([`tangent_cards`]: the render hints `NormalAndSpecMapped`,
/// `NormalAndSpecMappedMesh` and `NormalTangents` in the model, a
/// `texture1` normal map named in the model, neither, and
/// `NormalAndSpecMapped` in the MTR), each with a mirrored seam, drawn by
/// a custom pair ([`TANGENT_VS`], [`tangent_fs`]) that paints the
/// tangent (`MG_TAN_PAINT=handedness`: the handedness); the back row
/// ASCII (`mg_tana`), the front row the same compiled by nwnmdlcomp
/// (`mg_tanc`). Beside it, Moonglow's area view with the Shaders switch
/// on (the same programs).
///
/// Seen (2026-10-30, 89.8193.37): in the client the ASCII cards with
/// `NormalAndSpecMapped` and `NormalTangents` in the model, and the one
/// with `NormalAndSpecMapped` in its MTR (ASCII or compiled), have
/// tangents; `NormalAndSpecMappedMesh`, `texture1` alone and nothing give
/// none (handedness 0), nor the cards compiled by nwnmdlcomp with the
/// model's hint (it keeps no hint). The seam is split in the client: the
/// tangent +x on the west half and −x on the east, handedness +1 and −1,
/// sharp down the middle, its vertices copied as the game's compiler
/// copies them (MDL-MESH-043); Moonglow's made tangents run smoothly
/// across it (summed per face on the shared vertices). Moonglow gave
/// `NormalAndSpecMappedMesh` tangents too (fixed: `hint_wants_tangents`).
#[test]
#[ignore]
fn reading_tangents() {
    let base = base_game();
    let dir = scratch_dir("client_reading_tangents_compile");
    let compiled = super::batch4::nwnmdlcomp(&dir, "mg_tanc", &tangent_cards("mg_tanc"));
    let (table, first) = placeables_with(&base, &[("mg_tana", 0), ("mg_tanc", 0)]);
    // (A flat normal map: the tangent painted, not the map.)
    let flat = Rgba { width: 4, height: 4, data: [128, 128, 255, 255].repeat(16) };
    let overrides = [
        ("mg_tana.mdl", tangent_cards("mg_tana").into_bytes()),
        ("mg_tanc.mdl", compiled),
        ("mgtanvs.shd", TANGENT_VS.as_bytes().to_vec()),
        ("mgtanfs.shd", tangent_fs().into_bytes()),
        (
            "mgtan.mtr",
            b"texture0 mgwhite\ncustomshadervs mgtanvs\ncustomshaderfs mgtanfs\n".to_vec(),
        ),
        (
            "mgtanh.mtr",
            b"renderhint NormalAndSpecMapped\ntexture0 mgwhite\ncustomshadervs mgtanvs\n\
              customshaderfs mgtanfs\n"
                .to_vec(),
        ),
        ("mgwhite.tga", super::white_tga()),
        ("mgflatnm.tga", mg_image::write_tga(&flat)),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("tangents", "tic01", light, &overrides);
    r.place(first, [20.0, 25.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    r.place(first + 1, [20.0, 23.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// Moonglow's side of `batch3.rs`'s `reading_light_shadows` (no client):
/// the same dim tic01 room, the ogre at (20, 22) and the lamp posts of
/// each case, drawn by the area view from the reference camera with the
/// lights' shadows on and off; the shadow's pixels (where on is darker by
/// 6 of 765 or more), which side of the middle they lie, and the floor's
/// darkness below the feet (the median), printed as the client's reading
/// prints them. `MG_CASES` picks.
///
/// Seen (2026-10-30): the floor darker by 0.53, 0.54, 0.57, 0.59 and 0.60
/// at 1.5, 3, 5, 7 and 9 m (the client's 0.50, 0.53, 0.38, 0.21, 0.11),
/// the shadows as long as the client's; of the equal pairs, before the
/// fix, the east lamp's shadow (falling west) where the client casts the
/// west one's, and with the ogre facing east the north one's; after
/// (`casting_light` measuring from the creature's place), the client's
/// lamps in every case.
#[test]
#[ignore]
fn moonglow_light_shadows() {
    let cases: [super::batch3::LampCase; 8] = [
        ("near_1.5", &[(20.0, 23.5)], 270.0),
        ("near_3", &[(20.0, 25.0)], 270.0),
        ("near_5", &[(20.0, 27.0)], 270.0),
        ("near_7", &[(20.0, 29.0)], 270.0),
        ("near_9", &[(20.0, 31.0)], 270.0),
        ("equal_we", &[(17.5, 22.0), (22.5, 22.0)], 270.0),
        ("equal_ns", &[(20.0, 24.5), (20.0, 19.5)], 270.0),
        ("equal_ns_east", &[(20.0, 24.5), (20.0, 19.5)], 0.0),
    ];
    let only = std::env::var("MG_CASES").ok();
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    for (case, lamps, facing) in cases {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|c| c == case)) {
            continue;
        }
        let light =
            Lighting { ambient: 0x303030, diffuse: 0x303030, main_light: 0, only_tile: None };
        let mut r = Reading::new(&format!("moonglow_light_shadows_{case}"), "tic01", light, &[]);
        for (label, v) in [("SunShadows", 0u8), ("MoonShadows", 0), ("ShadowOpacity", 100)] {
            r.are.root.set(label, Value::Byte(v));
        }
        super::batch3::put_creature(&mut r, "nw_ogre01", [20.0, 22.0, 0.0], facing);
        for &(x, y) in lamps {
            r.place(73, [x, y, 0.0], 0.0, &[("Static", Value::Byte(0))]);
        }
        let tileset = mg_area::tileset(&r.game, r.are.root.resref("Tileset").unwrap()).ok();
        let model = mg_area::AreaModel::read(&r.game, &r.are.root, &r.git.root, tileset.as_ref());
        let area_scene = mg_area::AreaScene::new(&gpu, &r.game, &model);
        let camera =
            fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
        let shot = |on: bool| {
            let v = mg_area::View {
                fog: false,
                creature_shadows: on,
                light_shadows: on,
                ..mg_area::View::of(&model)
            };
            let scene = area_scene.scene(&model, &v);
            let mut renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
            renderer.render_image(&gpu, &r.game.resman, &scene, &camera, 1024, 768)
        };
        let (best, off) = (shot(true), shot(false));
        save_png(&best, &r.dir.join("on.png"));
        save_png(&off, &r.dir.join("off.png"));
        let (w, h) = (best.width as usize, best.height as usize);
        let (mut n, mut bbox, mut halves, mut floor) =
            (0usize, [usize::MAX, usize::MAX, 0, 0], [0usize; 2], Vec::new());
        for y in h * 35 / 100..h * 75 / 100 {
            for x in w * 30 / 100..w * 70 / 100 {
                let i = (y * w + x) * 4;
                let sum = |p: &[u8]| p[0] as f64 + p[1] as f64 + p[2] as f64;
                let (b, o) = (sum(&best.data[i..i + 3]), sum(&off.data[i..i + 3]));
                if o - b >= 6.0 {
                    if y * 1000 >= h * 505 {
                        floor.push(1.0 - b / o);
                    }
                    n += 1;
                    bbox = [bbox[0].min(x), bbox[1].min(y), bbox[2].max(x), bbox[3].max(y)];
                    halves[usize::from(x >= w / 2)] += 1;
                }
            }
        }
        floor.sort_by(f64::total_cmp);
        eprintln!(
            "{case}: Moonglow's shadow: {n} pixels from ({}, {}) to ({}, {}); west of the middle \
             {}, east {}; the floor below the feet darker by {:.3} (the median)",
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            halves[0],
            halves[1],
            floor.get(floor.len() / 2).copied().unwrap_or(0.0)
        );
    }
}

/// A city building's shadow under a sinking sun (the deferred list's
/// shadows entry, the low sun: tcn01's buildings cast none over the street
/// at 14 degrees in the client, half of it at 27 and 45): tcn01 all plain
/// cobbles (tile 44) but for one house (tile 54, `tcn01_s05_01`) west of
/// the middle, north of the player, Sun Shadows on at 100, a GIT sun from
/// the west at each height of `MG_SUN_Z` (default 0.25, 0.35, 0.5, 0.75
/// and 1: 14, 19, 27, 37 and 45 degrees up), beside Moonglow's area view
/// with its shadows.
///
/// `MG_HOUSE_X` puts the house that many tiles from the west edge (0: at
/// the view's edge), `MG_HOUSE_TURN` turns it, `MG_LOOK` gives another
/// camera (`facing,distance,pitch`).
///
/// Seen (2026-10-30, 89.8193.37): the house's shadow is as long in the
/// client as in Moonglow at every height, 14 degrees too (out to the
/// area's end, `MG_LOOK=30,20,50`), turned each of the four ways, and at
/// the view's edge: the city street the client leaves lit at 14 degrees
/// (`reading_low_sun_shadow_length` with `MG_TILESET=tcn01`) isn't a
/// building tile's own doing.
#[test]
#[ignore]
fn reading_low_sun_building() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let heights = std::env::var("MG_SUN_Z").unwrap_or_else(|_| "0.25,0.35,0.5,0.75,1".into());
    for z in heights.split(',').filter_map(|z| z.trim().parse::<f32>().ok()) {
        let x = std::env::var("MG_HOUSE_X").unwrap_or_else(|_| "1".into())
            + &std::env::var("MG_HOUSE_TURN").map(|t| format!("_turn{t}")).unwrap_or_default();
        let mut r = Reading::new(&format!("low_sun_building_{x}_{z}"), "tcn01", light, &[]);
        for (label, v) in [("SunShadows", 1u8), ("MoonShadows", 0), ("ShadowOpacity", 100)] {
            r.are.root.set(label, Value::Byte(v));
        }
        let width = r.are.root.int("Width").unwrap_or(4) as usize;
        for (i, t) in r.are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
            // (`MG_HOUSE_X`: the house that many tiles from the west edge, 1 by
            // default; 0 puts it out of the camera's view.)
            let column = std::env::var("MG_HOUSE_X").ok().and_then(|x| x.parse().ok()).unwrap_or(1);
            let house = i == 2 * width + column;
            t.set("Tile_ID", Value::Int(if house { 54 } else { 44 }));
            t.set("Tile_Height", Value::Int(0));
            // (`MG_HOUSE_TURN`: the house turned so many quarters.)
            let turn = std::env::var("MG_HOUSE_TURN").ok().and_then(|t| t.parse().ok());
            t.set("Tile_Orientation", Value::Int(if house { turn.unwrap_or(0) } else { 0 }));
        }
        let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else {
            panic!("no AreaProperties")
        };
        for (axis, v) in [("X", -1.0f32), ("Y", 0.0), ("Z", z)] {
            props.set(&format!("SunDirection{axis}"), Value::Float(v));
            props.set(&format!("MoonDirection{axis}"), Value::Float(v));
        }
        let settings = super::shadows::settings();
        // (`MG_LOOK=facing,distance,pitch`: another camera, as `shoot_close`.)
        let look: Vec<f32> = std::env::var("MG_LOOK")
            .map(|l| l.split(',').filter_map(|v| v.parse().ok()).collect())
            .unwrap_or_default();
        let (dir, _) = if let [f, d, p] = look[..] {
            super::batch4::shoot_close(r, INVISIBLE, &settings, (f, d, p), &|v| v.shadows = true)
        } else {
            r.shoot_beside(INVISIBLE, &settings, &|v| v.shadows = true)
        };
        eprintln!("{}", dir.display());
    }
}

/// The game's `vsgrass` and `fsgrass` patched to paint each tuft in three
/// bands of its texture's height (the quarter's v, the tuft's own): the
/// lower and upper quarter the fragment's place in the world (red and
/// green the fractions of its x and y metres, blue 0.2 below and 0.8
/// above), the middle half the client's clock (`worldtimerTimeOfDay`'s
/// two low bytes as red and green, blue 0.5).
fn sway_programs(game: &GameData) -> [(&'static str, Vec<u8>); 2] {
    let world = "#define POSITION_WORLD 1\n";
    let vs = format!("{world}{}", program(game, "vsgrass"));
    let fs = patched(
        &format!("{world}#include \"inc_uniforms\"\n{}", program(game, "fsgrass")),
        "gl_FragColor = FragmentColor;",
        "float v = fract(vVertexTexCoords.y * 2.0);\n    \
         int t = worldtimerTimeOfDay;\n    \
         if (v < 0.25 || v > 0.75)\n        \
         gl_FragColor = vec4(fract(vPosWorld.x), fract(vPosWorld.y), v < 0.25 ? 0.2 : 0.8, 1.0);\n    \
         else\n        \
         gl_FragColor = vec4(float(t - (t / 256) * 256) / 255.0, \
         float((t / 256) - (t / 65536) * 256) / 255.0, 0.5, 1.0);",
    );
    [("vsgrass.shd", vs.into_bytes()), ("fsgrass.shd", fs.into_bytes())]
}

/// How far and how fast a grass tuft's top sways in the client (the
/// deferred list's "A grass tuft's sway"): sparse white tufts (0.3 a
/// square metre) on a rural field in the wind (`MG_WIND`, 1 and 2 by
/// default), seen from the side and close (`SetCameraFacing(90, 4, 85)`),
/// the grass programs patched ([`sway_programs`]) so that each picture
/// carries the client's clock and each tuft its feet's and top's places
/// in the world; `MG_SHOTS` pictures (30 by default) as fast as they come.
/// For each picture, the clock (the middle bands' most common value) and,
/// for each tuft (a run of rows of the same column whose lower band and
/// upper band both show), its top's place less its feet's (metres, the
/// fractions unwrapped), the mean over the picture's tufts, are printed.
///
/// Seen (2026-10-30, 89.8193.37; 80 pictures each, about 5.5 s of the
/// clock, a new frame every 0.05 to 0.5 s): the tops of all the tufts
/// (500 to 590 read a picture) sway together, north and south (y; x stays
/// within 0.03 m): at `WindPower` 2 between −0.055 and +0.098 m of their
/// feet (1.2 m tufts), from a peak at 31.8 s through a trough at 33.8 s to
/// the next peak at 35.8 to 36.0 s, about 4 s a swing; at 1 between
/// −0.027 and +0.045 m, slower (a peak at 33.1 s, falling to the end at
/// 36.5 s). Moonglow's tufts stand still (its own programs and the game's).
#[test]
#[ignore]
fn reading_grass_sway_by_clock() {
    let base = base_game();
    let shots = std::env::var("MG_SHOTS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    let winds = std::env::var("MG_WIND").unwrap_or_else(|_| "1,2".into());
    for wind in winds.split(',').filter_map(|w| w.trim().parse::<i32>().ok()) {
        let mut overrides = vec![("mgwhite.tga", super::white_tga())];
        overrides.extend(sway_programs(&base));
        let light =
            Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
        let mut r = Reading::new(&format!("grass_sway_clock_{wind}"), "ttr01", light, &overrides);
        r.are.root.set("WindPower", Value::Int(wind));
        let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else { panic!() };
        let mut ovr = mg_gff::Struct::new(101);
        let white = super::grass_override(3, "mgwhite", 0.3, 1.2, [1.0, 1.0, 1.0]);
        ovr.set("GrassOvrList", Value::List(vec![white]));
        props.set("GrassOverrides", Value::Struct(ovr));
        r.save(
            &super::readings::enter(INVISIBLE, (90.0, 4.0, 85.0)),
            &super::readings::with_grass(),
        );
        let pictures =
            common::client_screenshots(&r.dir, "MgScene", shots, Duration::from_millis(0))
                .expect("the client's pictures");
        eprintln!("wind {wind}: {}", r.dir.display());
        for (i, (_, img)) in pictures.iter().enumerate() {
            save_png(img, &r.dir.join(format!("sway_{i}.png")));
            let (clock, sway, tufts) = sway_of(img);
            eprintln!(
                "wind {wind} picture {i}: clock {clock:?} ms (low 16 bits), {tufts} tufts, \
                 top less feet ({:.3}, {:.3}) m",
                sway[0], sway[1]
            );
        }
    }
}

/// What a picture of [`reading_grass_sway_by_clock`] shows: the clock's
/// two low bytes (the most common middle-band value), the mean of the
/// tufts' top less feet (x, y metres), how many tufts were read.
fn sway_of(img: &Rgba) -> (Option<u32>, [f32; 2], usize) {
    let (w, h) = (img.width as usize, img.height as usize);
    let px = |x: usize, y: usize| {
        let i = (y * w + x) * 4;
        [img.data[i], img.data[i + 1], img.data[i + 2]]
    };
    let band = |p: [u8; 3]| match p[2] {
        46..=56 => Some(0u8),
        122..=133 => Some(1),
        199..=209 => Some(2),
        _ => None,
    };
    let mut clocks: HashMap<u32, usize> = HashMap::new();
    let (mut sum, mut tufts) = ([0.0f32; 2], 0usize);
    // (The game's GUI covers the bottom; the tufts stand in the middle.)
    // One tuft down a column: its top band, then its clock band, then its
    // other band down to its foot, without a break; the first such run in
    // each column, its top's first pixel less its foot's last.
    let d = |a: u8, c: u8| {
        let v = (f32::from(a) - f32::from(c)) / 255.0;
        v - v.round()
    };
    for x in 0..w {
        let (mut top, mut clocked, mut feet): (Option<[u8; 3]>, bool, Option<[u8; 3]>) =
            (None, false, None);
        for y in h / 8..h * 3 / 4 {
            let p = px(x, y);
            let b = band(p);
            if let (Some(t), Some(f)) = (top, feet) {
                if b == band(f) {
                    feet = Some(p);
                    continue;
                }
                sum[0] += d(t[0], f[0]);
                sum[1] += d(t[1], f[1]);
                tufts += 1;
                break;
            }
            match b {
                Some(1) => {
                    *clocks.entry(u32::from(p[0]) | u32::from(p[1]) << 8).or_default() += 1;
                    clocked = top.is_some();
                }
                Some(b) => match top {
                    Some(t) if clocked && band(t) != Some(b) => feet = Some(p),
                    Some(t) if band(t) == Some(b) && !clocked => {}
                    _ => (top, clocked) = (Some(p), false),
                },
                None => (top, clocked) = (None, false),
            }
        }
    }
    let clock = clocks.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c);
    let n = tufts.max(1) as f32;
    (clock, [sum[0] / n, sum[1] / n], tufts)
}

/// A texture 64 square, blue, its green the texture's v (0 at the bottom,
/// 252 at the top) and its red the u in four steps: what part of it a
/// pixel shows.
fn gradient_tga() -> Vec<u8> {
    // (Bottom row first, BGRA.)
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 64, 0, 32, 0];
    for y in 0..64u8 {
        for x in 0..64u8 {
            t.extend([255, y * 4, (x / 16) * 40, 255]);
        }
    }
    t
}

/// A `Linked` ribbon's pieces and its order among see-through meshes (the
/// deferred list's "A `Linked` ribbon and a flip-book"): north of the
/// player, seen from the side (`SetCameraFacing(90, 6, 80)`, a second
/// after entry), west to east, ribbons of 30 cm particles of
/// [`gradient_tga`] rising 1 m/s (one a second: pieces a metre long) and
/// 0.5 m/s (half a metre), then the same particles not linked; before them
/// (nearer the camera) a card of mesh `alpha 0.5`, red, across their
/// middle (1 to 1.6 m up), and behind them another, from 2 to 2.6 m.
/// Where a ribbon shows through the near card, or over the far one, says
/// which is drawn first. `MG_RIBBON_SIZE` gives the particles' size.
///
/// Seen (2026-10-30, 89.8193.37), down each ribbon's middle (the
/// texture's v as green): each piece starts at its older particle with v
/// near 0 and is stretched past the next one, which starts again from 0:
/// at 30 cm particles v reaches 0.89 where the next piece starts a metre
/// on (its whole cell over 1.10 m) and 0.85 half a metre on (over 0.59
/// m); at 60 cm 0.78 and 0.67 (over 1.23 and 0.75 m): the cell runs over
/// the gap and about a tenth of a metre more at 30 cm, a quarter at 60.
/// Moonglow's pieces show the whole cell from one particle to the next.
/// The near card hides the ribbon behind it entirely in the client (the
/// card's red over the wall, as where no ribbon is: it writes the depth
/// the particles are drawn against), where Moonglow draws the ribbon over
/// it, whole; over the far card the ribbon shows whole in both.
#[test]
#[ignore]
fn reading_ribbon_pieces_and_order() {
    let base = base_game();
    let m = "mg_ribbon";
    // (`MG_RIBBON_SIZE`: the particles' size, 0.3 m by default.)
    let size = std::env::var("MG_RIBBON_SIZE").unwrap_or_else(|_| "0.3".into());
    let p =
        format!("texture mggrad\n  sizeStart {size}\n  sizeEnd {size}\n  birthrate 1\n  lifeExp 4");
    let card = |name: &str, y: f32, z: f32| {
        format!(
            "node trimesh {name}\n  parent {m}\n  position 0 {y} {z}\n  orientation 0 0 0 0\n  \
             ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap mgred\n  \
             alpha 0.5\n  render 1\n  shadow 0\n  verts 4\n    -1.4 0 0\n    1.4 0 0\n    \
             1.4 0 0.6\n    -1.4 0 0.6\n  tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  \
             faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n"
        )
    };
    let e = super::batch4::emitter;
    let model = super::batch4::probe_model(
        m,
        &[
            e(m, "link1", [-0.9, 0.0, 0.0], &format!("{p}\n  velocity 1\n  render Linked")),
            e(m, "link05", [-0.3, 0.0, 0.0], &format!("{p}\n  velocity 0.5\n  render Linked")),
            e(m, "plain1", [0.3, 0.0, 0.0], &format!("{p}\n  velocity 1")),
            e(m, "plain05", [0.9, 0.0, 0.0], &format!("{p}\n  velocity 0.5")),
            card("near", -0.6, 1.0),
            card("far", 0.6, 2.0),
        ],
    );
    let (table, first) = placeables_with(&base, &[(m, 0)]);
    let red = Rgba { width: 4, height: 4, data: [255, 0, 0, 255].repeat(16) };
    let overrides = [
        ("mg_ribbon.mdl", model.into_bytes()),
        ("mggrad.tga", gradient_tga()),
        ("mgred.tga", mg_image::write_tga(&red)),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x606060, diffuse: 0x606060, main_light: 0, only_tile: None };
    let mut r =
        Reading::new(&format!("ribbon_pieces_and_order_{size}"), "tic01", light, &overrides);
    r.place(first, [20.0, 24.0, 0.0], 0.0, &[]);
    let (dir, _) = super::batch4::shoot_close(r, INVISIBLE, SETTINGS, (90.0, 6.0, 80.0), &|v| {
        v.animate = true
    });
    eprintln!("{}", dir.display());
}

/// A standing card of `model` named `name`, 1.6 m wide and 1.6 m high,
/// its middle at `at`, turned `turn` degrees about Z, of `bitmap`, with
/// the mesh's `alpha`.
fn turned_card(
    model: &str,
    name: &str,
    at: [f32; 3],
    turn: f32,
    bitmap: &str,
    alpha: f32,
) -> String {
    let (s, c) = turn.to_radians().sin_cos();
    let [x, y, z] = at;
    let corner = |u: f32, h: f32| format!("    {} {} {}\n", u * c, u * s, h);
    format!(
        "node trimesh {name}\n  parent {model}\n  position {x} {y} {z}\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n  \
         alpha {alpha}\n  render 1\n  shadow 0\n  verts 4\n{}{}{}{}  tverts 4\n    0 0 0\n    \
         1 0 0\n    1 1 0\n    0 1 0\n  faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\n\
         endnode\n",
        corner(-0.8, 0.0),
        corner(0.8, 0.0),
        corner(0.8, 1.6),
        corner(-0.8, 1.6)
    )
}

/// How the client draws see-through meshes against each other (the
/// deferred list's "See-through meshes": two crossing ones sort by their
/// middles in Moonglow, the see-through part writing no depth): north of
/// the player, west to east, two crossing pairs of cards (red and blue,
/// turned +35 and −35 degrees about Z, crossing at their middles): of
/// mesh `alpha 0.5` and a plain texture, and of mesh alpha 1 and a
/// texture of alpha 128; then a red card of mesh `alpha 0.5` a metre in
/// front of a blue one of `alpha 0.5`, and a blue one in front of a red
/// one, the far one listed first in the model (which is drawn over which
/// where they overlap). Seen from the reference camera, Moonglow's area
/// view beside.
///
/// Seen (2026-10-30, 89.8193.37): the `alpha 0.5` cards are drawn in the
/// model's order, each writing its depth: where a card listed later lies
/// behind one listed earlier it is hidden (the crossing pair's blue,
/// listed second, shows only where it is in front of the red; the front
/// card listed first hides the one behind it), and where the far card is
/// listed first the near one is laid over it, both showing (as when the
/// fourth pair was listed near first: blue alone where they overlap).
/// Moonglow sorts them back to front writing no depth: both show
/// everywhere they overlap. A texture of alpha 128 with mesh alpha 1 (no
/// TXI) is opaque in both.
#[test]
#[ignore]
fn reading_see_through_crossing() {
    let base = base_game();
    let m = "mg_cross";
    let model = super::batch4::probe_model(
        m,
        &[
            turned_card(m, "ar", [-3.0, 0.0, 0.0], 35.0, "mgred", 0.5),
            turned_card(m, "ab", [-3.0, 0.0, 0.0], -35.0, "mgblue", 0.5),
            turned_card(m, "br", [-1.0, 0.0, 0.0], 35.0, "mgredt", 1.0),
            turned_card(m, "bb", [-1.0, 0.0, 0.0], -35.0, "mgbluet", 1.0),
            turned_card(m, "cr", [1.0, -0.5, 0.0], 0.0, "mgred", 0.5),
            turned_card(m, "cb", [1.4, 0.5, 0.3], 0.0, "mgblue", 0.5),
            turned_card(m, "dr", [3.4, 0.5, 0.3], 0.0, "mgred", 0.5),
            turned_card(m, "db", [3.0, -0.5, 0.0], 0.0, "mgblue", 0.5),
        ],
    );
    let (table, first) = placeables_with(&base, &[(m, 0)]);
    let solid = |c: [u8; 4]| mg_image::write_tga(&Rgba { width: 4, height: 4, data: c.repeat(16) });
    let overrides = [
        ("mg_cross.mdl", model.into_bytes()),
        ("mgred.tga", solid([255, 0, 0, 255])),
        ("mgblue.tga", solid([0, 0, 255, 255])),
        ("mgredt.tga", solid([255, 0, 0, 128])),
        ("mgbluet.tga", solid([0, 0, 255, 128])),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0xA0A0A0, diffuse: 0x606060, main_light: 0, only_tile: None };
    let mut r = Reading::new("see_through_crossing", "tic01", light, &overrides);
    r.place(first, [20.0, 24.0, 0.0], 0.0, &[]);
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// The entry script of [`joined_pc_faction`]: after a while, every
/// creature and placeable of the area is written to the server's log with
/// how it and the player regard each other.
const PC_FACTION_ENTER: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
void Regards(object pc)
{
    object area = GetArea(pc);
    object o = GetFirstObjectInArea(area);
    while (GetIsObjectValid(o))
    {
        int t = GetObjectType(o);
        if ((t == OBJECT_TYPE_CREATURE || t == OBJECT_TYPE_PLACEABLE) && o != pc)
            Log("MG_REGARDS " + GetTag(o) + " " + GetResRef(o)
                + " same_faction " + IntToString(GetFactionEqual(o, pc))
                + " it_of_pc " + IntToString(GetReputation(o, pc))
                + " pc_of_it " + IntToString(GetReputation(pc, o))
                + " it_enemy " + IntToString(GetIsEnemy(pc, o))
                + " pc_enemy " + IntToString(GetIsEnemy(o, pc))
                + " as_pc0 " + IntToString(GetFactionEqual(o, GetObjectByTag("MG_PC0_COMMONER")))
                + " as_hostile " + IntToString(GetFactionEqual(o, GetObjectByTag("MG_HOSTILE_REF")))
                + " as_commoner " + IntToString(GetFactionEqual(o, GetObjectByTag("MG_COMMONER_REF"))));
        o = GetNextObjectInArea(area);
    }
}
// (Read as it runs: a DelayCommand's arguments are taken when it is
// called.)
void LogAt(object pc)
{
    Log("MG_AT " + FloatToString(GetPosition(pc).x) + " " + FloatToString(GetPosition(pc).y));
}
void main()
{
    object pc = GetEnteringObject();
    if (!GetIsPC(pc)) return;
    Log("MG_ENTERED " + GetName(pc));
    object area = GetArea(pc);
    DelayCommand(8.0, LogAt(pc));
    // (The player kept out of the fight once the encounters have spawned.)
    DelayCommand(1.5, SetPlotFlag(pc, TRUE));
    DelayCommand(1.5, ApplyEffectToObject(DURATION_TYPE_PERMANENT, EffectVisualEffect(VFX_DUR_CUTSCENE_INVISIBILITY), pc));
    DelayCommand(3.0, Log("MG_AT_3"));
    DelayCommand(3.0, Regards(pc));
    DelayCommand(12.0, Log("MG_AT_12"));
    DelayCommand(12.0, Regards(pc));
    DelayCommand(13.0, Log("MG_DONE"));
}
"#;

/// The PC faction with a player (the deferred list's "The PC faction,
/// what's left"): a player joins a server (`mg_testkit::joined`, the owner's
/// CD key) whose tic01 area holds, about the start (20, 20): a commoner of
/// faction 0 (`MG_PC0_COMMONER`) and a chest of faction 0 (`MG_PC0_CHEST`),
/// and two encounters about the start, where the player enters: kobolds
/// (`x2_kobold001`) of faction 0 and goblins (`nw_goblingrp`) without a
/// `Faction` field. Each creature's and placeable's regard of the player
/// (and the player's of it) is written to the server's log
/// (`MG_REGARDS`), printed. The character is chosen
/// by hand (`MG_JOINED_HOLD`, `tools/aurora/xdrive.py`: Play clicked, the
/// pointer moved off the game at once): the test waits for `MG_DONE` in
/// the log. Beside them, far off, a goblin of Hostile (`MG_HOSTILE_REF`)
/// and a commoner of Commoner (`MG_COMMONER_REF`), whose factions each is
/// compared with. `MG_ENCOUNTERS` picks the encounters.
///
/// Seen (2026-10-30, 89.8193.37): the commoner of faction 0 is of the
/// goblin's faction (Hostile) and regards the player as hostile (0), the
/// player it too (0); with its scripts it fought the player and died. The
/// chest of faction 0 regards the player as hostile (0), the player it as
/// neutral (50), of no creature's faction. The encounter of faction 0
/// spawns nothing (`faction0` alone too), the same of the kobolds' own
/// faction (`kobolds`) four kobolds of Hostile, and the one without a
/// `Faction` field its goblins, of Hostile. (A `DelayCommand`'s arguments
/// are taken when it is called: `LogAt`.)
#[test]
#[ignore]
fn joined_pc_faction() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_testkit::joined::{self, Join};
    let Some(_) = joined::cd_key() else {
        eprintln!("skipped: the sandboxed client has no CD key (MG_CLIENT_CDKEY)");
        return;
    };
    let game = base_game();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let dir = scratch_dir("client_joined_pc_faction");
    let (mut m, _) = build_module(&game, &dir, light);
    m.set(
        ResKey::parse("mg_enter", ResType::NCS).unwrap(),
        compile(&dir, "mg_enter", PC_FACTION_ENTER),
    );
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let bp = |name: &str, t: ResType| Gff::read(&game.resman.get_named(name, t).unwrap()).unwrap();
    let at = |x: f32, y: f32| Placement { position: [x, y, 0.0], rotation: 0.0 };
    let mut commoner =
        instance(&placing, ResType::UTC, &bp("nw_commale", ResType::UTC).root, at(20.0, 24.0), &[])
            .unwrap();
    commoner.set("FactionID", Value::Word(0));
    // (Standing still, unkillable: its scripts taken away, plot. With
    // them, on 2026-10-30, it fought the player and died.)
    let scripts: Vec<String> = commoner
        .fields
        .iter()
        .map(|f| f.label.to_string_lossy())
        .filter(|l| l.starts_with("Script"))
        .collect();
    for label in scripts {
        commoner.set(&label, Value::resref(ResRef::EMPTY));
    }
    commoner.set("Plot", Value::Byte(1));
    commoner.set("Tag", Value::String(b"MG_PC0_COMMONER".to_vec()));
    let mut chest =
        instance(&placing, ResType::UTP, &bp("plc_chest1", ResType::UTP).root, at(21.5, 24.0), &[])
            .unwrap();
    chest.set("Faction", Value::Dword(0));
    chest.set("Tag", Value::String(b"MG_PC0_CHEST".to_vec()));
    // References: a goblin of the Hostile faction and a commoner of the
    // Commoner faction, standing still, unkillable, far off.
    let reference = |name: &str, tag: &str, x: f32| {
        let mut c =
            instance(&placing, ResType::UTC, &bp(name, ResType::UTC).root, at(x, 36.0), &[])
                .unwrap();
        let scripts: Vec<String> = c
            .fields
            .iter()
            .map(|f| f.label.to_string_lossy())
            .filter(|l| l.starts_with("Script"))
            .collect();
        for label in scripts {
            c.set(&label, Value::resref(ResRef::EMPTY));
        }
        c.set("Plot", Value::Byte(1));
        c.set("Tag", Value::String(tag.as_bytes().to_vec()));
        c
    };
    let hostile = reference("nw_goblina", "MG_HOSTILE_REF", 6.0);
    let common = reference("nw_commale", "MG_COMMONER_REF", 34.0);
    git.root.set("Creature List", Value::List(vec![commoner, hostile, common]));
    git.root.set("Placeable List", Value::List(vec![chest]));
    let square = [[-3.0, -3.0, 0.0], [3.0, -3.0, 0.0], [3.0, 3.0, 0.0], [-3.0, 3.0, 0.0]];
    let encounter = |blueprint: &str, tag: &str, faction: Option<u32>| {
        let b = bp(blueprint, ResType::UTE);
        let mut e = instance(&placing, ResType::UTE, &b.root, at(20.0, 20.0), &square).unwrap();
        e.set("Tag", Value::String(tag.as_bytes().to_vec()));
        match faction {
            Some(f) => e.set("Faction", Value::Dword(f)),
            None => {
                e.remove("Faction");
            }
        }
        e
    };
    // (`MG_ENCOUNTERS`: `faction0`, `none`, `kobolds` (the kobolds' own
    // faction kept) or `both` (the default), which encounters there are.)
    let which = std::env::var("MG_ENCOUNTERS").unwrap_or_else(|_| "both".into());
    let mut encounters = Vec::new();
    if which == "both" || which == "faction0" {
        encounters.push(encounter("x2_kobold001", "MG_ENC_FACTION0", Some(0)));
    }
    if which == "kobolds" {
        encounters.push(encounter("x2_kobold001", "MG_ENC_KOBOLDS", Some(1)));
    }
    if which == "both" || which == "none" {
        encounters.push(encounter("nw_goblingrp", "MG_ENC_NONE", None));
    }
    git.root.set("Encounter List", Value::List(encounters));
    m.set_gff(git_key, &git).unwrap();
    let modules = joined::server_dir(&dir).join("modules");
    std::fs::create_dir_all(&modules).unwrap();
    m.save_as(&ModuleLocation::Archive(modules.join("MgJoined.mod"))).unwrap();
    std::fs::create_dir_all(joined::client_dir(&dir)).unwrap();
    std::fs::write(
        joined::client_dir(&dir).join("settings.tml"),
        format!("{SETTINGS}[client]\n\t[client.identity]\n\t\tname = \"MgTester\"\n"),
    )
    .unwrap();
    let j = joined::start(&dir, "MgJoined", Join::Player, None, &[]).unwrap();
    let hold = std::env::var("MG_JOINED_HOLD").ok().and_then(|s| s.parse().ok()).unwrap_or(240);
    let done = j.wait_for("MG_DONE", Duration::from_secs(hold));
    let _ = j.winshot("pc_faction_end");
    for line in j.server.log().lines().filter(|l| l.contains("MG_")) {
        eprintln!("{line}");
    }
    eprintln!("{} (done: {done})", dir.display());
    j.stop();
}
