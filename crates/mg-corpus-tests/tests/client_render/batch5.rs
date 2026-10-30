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
