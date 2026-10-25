//! A custom shader pair that picks which of a mesh's texture coordinate
//! sets it draws with (`tverts`, `tverts1`…`tverts3`, the engine's `vTcIn`
//! and `vTcIn1`…`vTcIn3`) by an MTR `parameter int`, and moves them by MTR
//! floats: in the client beside Moonglow's area view, with the Shaders
//! switch off and on.
//!
//! The scene: an interior (`tni01`, or `MG_UV_TILESET`) lit by its sun and
//! its tiles' lights, and two rows of cards north of the player (each the
//! model of a placeable appearance, replaced in the scratch override), of
//! one texture of four vertical stripes (gray, red, green, blue). Set 0 of
//! each card covers the whole texture, and sets 1, 2 and 3 each cover one
//! stripe (red, green, blue): the card's color says which set the shader
//! read. The gray stripe holds the coordinates (0, 0), which an input
//! without data would read. [`CARDS`] lists the cards: each set, a set the
//! mesh doesn't have, the normal map alone, every map, the coordinates
//! moved, and the stock shaders for comparison.
//!
//! The shaders are Moonglow's own ([`UV_VS`], [`UV_FS`]: the stock lit
//! pair, the vertex program choosing the set), or those in the folder
//! `MG_UV_SHADERS` (a builder's, which take the same parameters: a vertex
//! and a fragment program, told apart by their `SHADER_TYPE`).
//! `MG_UV_PAINT=bound` paints every lit fragment with which of its
//! material's maps are bound (the client's `textureNBound` and Moonglow's,
//! through the same patched `inc_standard`; [`bound_maps`] reads them),
//! `MG_UV_PAINT=uv` with its texture coordinates. `MG_UV_STATIC=1` makes
//! the placeables static; `MG_UV_NO_CLIENT` draws Moonglow's pictures
//! alone. The pictures go to
//! `target/test-output/client_custom_uvs/`; each card's mean color and
//! saturation is printed, the client's beside Moonglow's.

use super::*;

/// Moonglow's test vertex program: the stock lit one (`vslit_nm`), its
/// texture coordinates then taken from the set `nUvmap` names (0 or any
/// other number: `tverts`), scaled by `fUvScale` (when not 0), moved half
/// a texture back by `nUvCenter` 1, offset by `fUvOffsetX` and
/// `fUvOffsetY` and turned by `fUvRotation` degrees about the texture's
/// middle.
pub(crate) const UV_VS: &str = "\
// vsmguv: Moonglow's test of the extra texture coordinate sets.
#define SHADER_TYPE 1
#define FOG 1
#define KEYHOLING 1
#define NORMAL_MAP 1
#define LIGHTING 1
#include \"inc_standard\"

attribute vec2 vTcIn1;
attribute vec2 vTcIn2;
attribute vec2 vTcIn3;

uniform int nUvmap;
uniform float fUvScale;
uniform int nUvCenter;
uniform float fUvOffsetX;
uniform float fUvOffsetY;
uniform float fUvRotation;

void main()
{
    ApplyStandardShader();
    vec2 uv = vTcIn;
    if (nUvmap == 1) uv = vTcIn1;
    else if (nUvmap == 2) uv = vTcIn2;
    else if (nUvmap == 3) uv = vTcIn3;
    if (fUvScale != 0.0) uv *= fUvScale;
    if (nUvCenter == 1) uv -= vec2(0.5);
    uv += vec2(fUvOffsetX, fUvOffsetY);
    if (fUvRotation != 0.0)
    {
        float a = radians(fUvRotation);
        uv = mat2(cos(a), -sin(a), sin(a), cos(a)) * (uv - vec2(0.5)) + vec2(0.5);
    }
    vVertexTexCoords = mat2(m_texture) * uv;
}
";

/// Moonglow's test fragment program: the stock normal-mapped one
/// (`fslit_nm`) without keyholing.
pub(crate) const UV_FS: &str = "\
// fsmguv: the stock lit, mapped fragment program, keyholing off.
#define SHADER_TYPE 2
#define FOG 1
#define KEYHOLING 0
#define LIGHTING 1
#define NORMAL_MAP 1
#define SPECULAR_MAP 1
#define ROUGHNESS_MAP 1
#define HEIGHT_MAP 1
#define SELF_ILLUMINATION_MAP 1
#define ENVIRONMENT_MAP 0
#include \"inc_standard\"

void main()
{
    FragmentColor = vec4(1.0);
    SetupStandardShaderInputs();
    ApplyStandardShader();
    gl_FragColor = FragmentColor;
}
";

/// A card: its name, whether its MTR names the custom pair, the MTR's
/// other lines, and how many texture coordinate sets its mesh has.
pub(crate) struct UvCard {
    pub(crate) name: &'static str,
    pub(crate) custom: bool,
    pub(crate) lines: &'static str,
    pub(crate) sets: usize,
}

const fn uv_card(name: &'static str, lines: &'static str) -> UvCard {
    UvCard { name, custom: true, lines, sets: 4 }
}

/// The cards, back row (west to east) then front row.
pub(crate) const CARDS: [UvCard; 12] = [
    uv_card("set 0", "parameter int nUvmap 0\n"),
    uv_card("set 1", "parameter int nUvmap 1\n"),
    uv_card("set 2", "parameter int nUvmap 2\n"),
    uv_card("set 3", "parameter int nUvmap 3\n"),
    UvCard { name: "set 2 missing", custom: true, lines: "parameter int nUvmap 2\n", sets: 2 },
    uv_card("set 1 normal map", "parameter int nUvmap 1\ntexture1 mguvn\n"),
    uv_card(
        "set 1 all maps",
        "parameter int nUvmap 1\ntexture1 mguvn\ntexture2 mguvs\ntexture3 mguvr\n\
         texture4 mguvh\ntexture5 mguvi\n",
    ),
    uv_card(
        "set 1 normal hinted",
        "parameter int nUvmap 1\nrenderhint NormalAndSpecMapped\ntexture1 mguvn\n",
    ),
    uv_card(
        "set 1 all hinted",
        "parameter int nUvmap 1\nrenderhint NormalAndSpecMapped\ntexture1 mguvn\n\
         texture2 mguvs\ntexture3 mguvr\ntexture4 mguvh\ntexture5 mguvi\n",
    ),
    uv_card(
        "set 0 moved",
        "parameter int nUvmap 0\nparameter float fUvScale 2.0\nparameter float fUvOffsetX 0.25\n\
         parameter float fUvRotation 90.0\n",
    ),
    UvCard { name: "stock", custom: false, lines: "", sets: 4 },
    UvCard {
        name: "stock maps",
        custom: false,
        lines: "renderhint NormalAndSpecMapped\ntexture1 mguvn\ntexture2 mguvs\n",
        sets: 4,
    },
];

/// Card `i`'s placement: x and y.
pub(crate) fn card_at(i: usize) -> (f32, f32) {
    let (row, column) = (i / 6, i % 6);
    (20.0 + (column as f32 - 2.5) * 2.4, if row == 0 { 23.0 } else { 19.5 })
}

/// The card's texture coordinate sets: set 0 the whole texture, set `k`
/// its stripe `k` (a margin left at each side of it).
pub(crate) fn card_uvs(set: usize) -> [[f32; 2]; 4] {
    let (u0, u1) = if set == 0 {
        (0.0, 1.0)
    } else {
        let k = set as f32;
        (k * 0.25 + 0.03, k * 0.25 + 0.22)
    };
    [[u0, 0.0], [u1, 0.0], [u1, 1.0], [u0, 1.0]]
}

/// An ASCII model named `name`: a card 2.2 m wide and tall facing south,
/// its `bitmap` `bitmap`, with `sets` texture coordinate sets.
pub(crate) fn uv_card_model(name: &str, bitmap: &str, sets: usize) -> String {
    let mut tverts = String::new();
    for set in 0..sets {
        let key = if set == 0 { "tverts".to_string() } else { format!("tverts{set}") };
        tverts += &format!("  {key} 4\n");
        for [u, v] in card_uvs(set) {
            tverts += &format!("    {u} {v} 0\n");
        }
    }
    format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
         node trimesh card\n  parent {name}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n  \
         render 1\n  shadow 0\n  verts 4\n    -1.1 0 0.1\n    1.1 0 0.1\n    1.1 0 2.3\n    \
         -1.1 0 2.3\n{tverts}  faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\n\
         endnode\nendmodelgeom {name}\ndonemodel {name}\n"
    )
}

/// A TGA (bottom row first, BGRA), `size` square, of `pixel(x, y)`.
fn tga(size: u16, pixel: impl Fn(u16, u16) -> [u8; 4]) -> Vec<u8> {
    let [lo, hi] = size.to_le_bytes();
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, lo, hi, lo, hi, 32, 8];
    for y in 0..size {
        for x in 0..size {
            let [r, g, b, a] = pixel(x, y);
            t.extend([b, g, r, a]);
        }
    }
    t
}

/// The stripes (left to right: gray, red, green, blue), opaque.
pub(crate) const STRIPES: [[u8; 3]; 4] =
    [[128, 128, 128], [200, 40, 40], [40, 200, 40], [40, 40, 200]];

/// The textures: the stripes, and the maps (`mguvn` a normal map of
/// slopes, `mguvs` specularity 0.6, `mguvr` roughness 0.3, `mguvh` a
/// height ramp, `mguvi` a dim warm glow).
pub(crate) fn uv_textures() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "mguvtex",
            tga(64, |x, _| {
                let [r, g, b] = STRIPES[(x / 16) as usize];
                [r, g, b, 255]
            }),
        ),
        (
            "mguvn",
            tga(
                64,
                |_, y| if (y / 8) % 2 == 0 { [170, 128, 236, 255] } else { [86, 128, 236, 255] },
            ),
        ),
        ("mguvs", tga(16, |_, _| [153, 153, 153, 255])),
        ("mguvr", tga(16, |_, _| [77, 77, 77, 255])),
        ("mguvh", tga(64, |x, _| [(x * 4) as u8, (x * 4) as u8, (x * 4) as u8, 255])),
        ("mguvi", tga(16, |_, _| [60, 30, 0, 255])),
    ]
}

/// The MTR of card `c` with the pair `vs`/`fs`.
pub(crate) fn uv_mtr(c: &UvCard, vs: &str, fs: &str) -> String {
    let pair = if c.custom {
        format!("customshadervs {vs}\ncustomshaderfs {fs}\n")
    } else {
        String::new()
    };
    format!("texture0 mguvtex\n{pair}{}", c.lines)
}

/// `inc_standard` painting a lit fragment's bound maps over everything
/// else (red: 32 for the normal map, 64 specular, 128 roughness; green:
/// 32 height, 64 self-illumination, 128 the diffuse texture) or its
/// texture coordinates (red and green, their fractions).
fn painting_inc_standard(game: &GameData, what: &str) -> String {
    let src = game.resman.get_named("inc_standard", ResType::SHD).unwrap();
    let paint = match what {
        "uv" => "\tFragmentColor = vec4(fract(vTexCoords.xy), 0.0, 1.0);\n".to_string(),
        _ => {
            let mut s =
                "\tfloat mgA = 0.0;\n\tfloat mgB = 0.5 * float(texture0Bound != 0);\n".to_string();
            for (define, unit, into, weight) in [
                ("NORMAL_MAP", 1, "mgA", "0.125"),
                ("SPECULAR_MAP", 2, "mgA", "0.25"),
                ("ROUGHNESS_MAP", 3, "mgA", "0.5"),
                ("HEIGHT_MAP", 4, "mgB", "0.125"),
                ("SELF_ILLUMINATION_MAP", 5, "mgB", "0.25"),
            ] {
                s += &format!(
                    "#if {define} == 1\n\t{into} += {weight} * float(texture{unit}Bound != 0);\n#endif\n"
                );
            }
            s + "\tFragmentColor = vec4(mgA, mgB, 1.0, 1.0);\n"
        }
    };
    String::from_utf8_lossy(&src).replace("\r\n", "\n").replace(
        "\tApplyDebugModeOutput(FragmentColor);\n}",
        &format!(
            "\tApplyDebugModeOutput(FragmentColor);\n#if SHADER_TYPE == 2 && LIGHTING == 1\n{{\n{paint}}}\n#endif\n}}"
        ),
    )
}

/// The maps a card painted by [`painting_inc_standard`] (`bound`) has
/// bound, from its mean color: (normal, specular, roughness, height,
/// self-illumination, diffuse).
pub(crate) fn bound_maps(m: [f32; 3]) -> [bool; 6] {
    let bits = |v: f32| (v / 32.0).round() as u32;
    let (a, b) = (bits(m[0]), bits(m[1]));
    [a & 1 != 0, a & 2 != 0, a & 4 != 0, b & 1 != 0, b & 2 != 0, b & 4 != 0]
}

/// The middle of card `i` on the picture (fractions, top-left origin), as
/// `camera` sees it: the card inset a fifth at each side.
fn card_rect(camera: &Camera, (w, h): (u32, u32), i: usize) -> [f32; 4] {
    let (x, y) = card_at(i);
    let vp = camera.projection_gl(w as f32 / h as f32) * camera.view();
    let at = |p: Vec3| {
        let c = vp * p.extend(1.0);
        ((c.x / c.w + 1.0) / 2.0, (1.0 - c.y / c.w) / 2.0)
    };
    let (a, b) = (at(Vec3::new(x - 0.66, y, 1.86)), at(Vec3::new(x + 0.66, y, 0.54)));
    [a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)]
}

#[test]
#[ignore]
fn custom_shader_uvs_look() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let tileset = std::env::var("MG_UV_TILESET").unwrap_or_else(|_| "tni01".into());
    let dir = scratch_dir("client_custom_uvs");
    let over = dir.join("user/override");
    std::fs::create_dir_all(&over).unwrap();
    std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
    // The pair: Moonglow's, or the folder's.
    let (mut vs, mut fs) = ("vsmguv".to_string(), "fsmguv".to_string());
    match std::env::var_os("MG_UV_SHADERS") {
        Some(from) => {
            for f in mg_testkit::files_with_ext(Path::new(&from), "shd") {
                let text = std::fs::read_to_string(&f).unwrap();
                let stem = f.file_stem().unwrap().to_string_lossy().to_ascii_lowercase();
                if text.contains("#define SHADER_TYPE 1") {
                    vs = stem.clone();
                } else if text.contains("#define SHADER_TYPE 2") {
                    fs = stem.clone();
                }
                std::fs::write(over.join(format!("{stem}.shd")), text).unwrap();
            }
        }
        None => {
            std::fs::write(over.join("vsmguv.shd"), UV_VS).unwrap();
            std::fs::write(over.join("fsmguv.shd"), UV_FS).unwrap();
        }
    }
    eprintln!("the pair: {vs} / {fs}");
    let paint = std::env::var("MG_UV_PAINT").ok();
    if let Some(what) = &paint {
        std::fs::write(over.join("inc_standard.shd"), painting_inc_standard(&base, what)).unwrap();
    }
    for (name, data) in uv_textures() {
        std::fs::write(over.join(format!("{name}.tga")), data).unwrap();
    }
    let table = base.table("placeables").unwrap();
    for (i, c) in CARDS.iter().enumerate() {
        let model = table.get(i, "ModelName").unwrap().to_ascii_lowercase();
        let material = format!("mguv{i}");
        std::fs::write(over.join(format!("{model}.mdl")), uv_card_model(&model, &material, c.sets))
            .unwrap();
        std::fs::write(over.join(format!("{material}.mtr")), uv_mtr(c, &vs, &fs)).unwrap();
    }
    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let light = Lighting { ambient: 0x505050, diffuse: 0x909090, main_light: 2, only_tile: None };
    let (mut m, are) = build_module_on(&game, &dir, light, &tileset, 0);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let is_static = std::env::var("MG_UV_STATIC").is_ok_and(|v| v == "1");
    let placed: Vec<_> = (0..CARDS.len())
        .map(|i| {
            let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Card", 0);
            bp.root.set("Appearance", Value::Dword(i as u32));
            bp.root.set("Static", Value::Byte(u8::from(is_static)));
            let (x, y) = card_at(i);
            let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
            instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap()
        })
        .collect();
    git.root.set("Placeable List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = if std::env::var_os("MG_UV_NO_CLIENT").is_some() {
        None
    } else {
        client_screenshot(&dir, "MgScene")
    };
    if let Some(c) = &client {
        save_png(c, &dir.join("client.png"));
    }

    let set = mg_area::tileset(&game, ResRef::from_str(&tileset).unwrap()).ok();
    let model = mg_area::AreaModel::read(&game, &are.root, &git.root, set.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let size = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    let off = r.render_image(&gpu, &game.resman, &scene, &camera, size.0, size.1);
    r.game_shaders = true;
    let on = r.render_image(&gpu, &game.resman, &scene, &camera, size.0, size.1);
    eprintln!(
        "Shaders on: {} of {} meshes by the game's programs; failures {:?}",
        r.drawn.game,
        r.drawn.meshes,
        r.game_shader_failures()
    );
    save_png(&off, &dir.join("moonglow_off.png"));
    save_png(&on, &dir.join("moonglow_on.png"));
    for (i, c) in CARDS.iter().enumerate() {
        let rect = card_rect(&camera, size, i);
        let mut line = format!("{:18} {:.3?}", c.name, rect);
        let mut pictures = vec![("off", &off), ("on", &on)];
        if let Some(cl) = &client {
            pictures.insert(0, ("client", cl));
        }
        for (name, img) in pictures {
            let (m, s) = mean_and_saturation(img, rect);
            line += &format!(" | {name} ({:5.1}, {:5.1}, {:5.1}) S {s:.3}", m[0], m[1], m[2]);
            if paint.as_deref() == Some("bound") && name != "off" {
                line += &format!(" {:?}", bound_maps(m).map(u8::from));
            }
        }
        eprintln!("{line}");
    }
    if let Some(c) = &client {
        eprintln!(
            "from the client's picture: Shaders off {:.2}, on {:.2} of 255",
            picture_difference(c, &off),
            picture_difference(c, &on)
        );
    }
    eprintln!("{}: client {}", dir.display(), client.is_some());
}
