//! The deferred sprint's sixth client batch (2026-11-01): readings of the
//! client that answer the deferred list's questions, beside the first five
//! batches' (`readings.rs`, `batch2.rs` to `batch5.rs`, whose scaffolding
//! they use). Each writes the client's pictures to its scratch directory
//! (`target/test-output/client_reading_<name>/`) and prints where; what
//! was seen is in its comment.
//!
//! Needs the game and the off-screen display; run by hand, one client at a
//! time:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render batch6 -- --ignored --nocapture --test-threads 1`.

use super::batch5::{base_game, patched, program, region_means};
use super::readings::{INVISIBLE, Reading, enter, placeables_with, print_log};
use super::*;

/// The wall clock's seconds since the epoch (UTC), for the timer's
/// readings.
fn now_utc() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

/// `worldtimerTimeOfDay` held over a long run (the deferred list's M1,
/// "what the count is"): the time module of `time_uniforms_in_the_client`
/// (13 h, 30 minutes an hour: a day of 43,200,000 ms, so the count is seen
/// as it is), its debug shader reading the timer back, the client left
/// running, a picture every `MG_TIMER_GAP` seconds (180) `MG_TIMER_SHOTS`
/// times (11): each shot's timer beside the wall clock (UTC) and the
/// seconds since the first.
///
/// Seen (2026-11-01, 89.8193.37): the timer ran with the wall clock to within
/// 60 ms over 30 minutes (11 pictures 3 minutes apart), from 24,904,369 seven
/// seconds after entry (the count B of `docs/deferred.md`'s M1 entry): no jump
/// within a run.
#[test]
#[ignore]
fn batch6_timer_held() {
    let game = base_game();
    let shader = debug_shader(&game);
    let gap: u64 = std::env::var("MG_TIMER_GAP").ok().and_then(|s| s.parse().ok()).unwrap_or(180);
    let shots: usize =
        std::env::var("MG_TIMER_SHOTS").ok().and_then(|s| s.parse().ok()).unwrap_or(11);
    let dir = scratch_dir("client_reading_timer_held");
    let light = Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
    let (mut m, _) = build_module(&game, &dir, light);
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", TIME_ENTER));
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_START_HOUR, 13u8);
    info.root.write(&ifo::MOD_MIN_PER_HOUR, 30u8);
    m.set_info(&info).unwrap();
    std::fs::create_dir_all(dir.join("user/override")).unwrap();
    std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
    std::fs::write(dir.join("user/override/inc_standard.shd"), &shader).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let taken = common::client_screenshots(&dir, "MgScene", shots, Duration::from_secs(gap))
        .unwrap_or_else(|| panic!("the client did not reach the scene (see {})", dir.display()));
    // (The wall clock at the first shot: the last one's less the time between.)
    let started = now_utc() - taken.last().unwrap().0 as u64;
    for (i, (at, img)) in taken.iter().enumerate() {
        save_png(img, &dir.join(format!("shot_{i}.png")));
        let timer = debug_int(img, 17);
        eprintln!(
            "shot {i} at {at:8.1} s (UTC {}): worldtimerTimeOfDay {timer}, less the seconds \
             since the first {}, sceneCurrentFrame {}",
            started + *at as u64,
            timer - (*at * 1000.0) as i64,
            debug_int(img, 21),
        );
    }
}

/// A standing card of [`super::batch4::probe_model`]: `w` m wide and `h`
/// high, its west edge at `x` and its foot at `z`, facing south (the
/// camera), its texture whole over it (u east, v up), drawn as `bitmap`.
fn card(name: &str, parent: &str, (x, z): (f32, f32), (w, h): (f32, f32), bitmap: &str) -> String {
    card_at(name, parent, [x, 0.0, z], (w, h), bitmap)
}

/// [`card`] with its west foot at `[x, y, z]`.
fn card_at(
    name: &str,
    parent: &str,
    [x, y, z]: [f32; 3],
    (w, h): (f32, f32),
    bitmap: &str,
) -> String {
    format!(
        "node trimesh {name}\n  parent {parent}\n  position {x} {y} {z}\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n  \
         render 1\n  shadow 0\n  verts 4\n    0 0 0\n    {w} 0 0\n    0 0 {h}\n    {w} 0 {h}\n  \
         tverts 4\n    0 0 0\n    1 0 0\n    0 1 0\n    1 1 0\n  faces 2\n    0 1 3 1 0 1 3 0\n    \
         0 3 2 1 0 3 2 0\nendnode\n"
    )
}

/// A custom vertex program: the stock lit one, no normal map.
const PLAIN_VS: &str = "\
#define SHADER_TYPE 1
#define FOG 0
#define KEYHOLING 0
#define NORMAL_MAP 0
#define LIGHTING 1
#include \"inc_standard\"
void main()
{
    ApplyStandardShader();
}
";

/// A custom fragment program: `before` (lines put before `main`, after
/// `inc_standard`), and the fragment painted `paint` (a `vec4`; the
/// mesh's texture is `tex`, sampled at the fragment's coordinates).
fn paint_fs(before: &str, paint: &str) -> String {
    format!(
        "#define SHADER_TYPE 2\n#define FOG 0\n#define KEYHOLING 0\n#define LIGHTING 1\n\
         #define NORMAL_MAP 0\n#define SPECULAR_MAP 0\n#define ROUGHNESS_MAP 0\n\
         #define HEIGHT_MAP 0\n#define SELF_ILLUMINATION_MAP 0\n#define ENVIRONMENT_MAP 0\n\
         #include \"inc_standard\"\n{before}\nvoid main()\n{{\n    FragmentColor = vec4(1.0);\n    \
         SetupStandardShaderInputs();\n    vec4 tex = texture2D(texUnit0, vVertexTexCoords.xy);\n    \
         gl_FragColor = {paint};\n}}\n"
    )
}

/// An MTR drawing `texture` with [`PLAIN_VS`] (as `mgplainvs`) and the
/// fragment program `fs`.
fn custom_mtr(texture: &str, fs: &str) -> Vec<u8> {
    format!("texture0 {texture}\ncustomshadervs mgplainvs\ncustomshaderfs {fs}\n").into_bytes()
}

/// Overrides named by `String`s, as [`Reading::new`] takes them.
fn as_overrides(files: &[(String, Vec<u8>)]) -> Vec<(&str, Vec<u8>)> {
    files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect()
}

/// The include-splicing cases of [`batch6_include_splicing`], west to east:
/// the card's name and the lines its fragment program has before `main`.
fn include_cases() -> Vec<(String, String)> {
    let mut out = vec![
        ("control".to_string(), String::new()),
        ("spaces".to_string(), "    #include \"mgi_color\"".to_string()),
        ("tab".to_string(), "\t#include \"mgi_color\"".to_string()),
        ("missing".to_string(), "#include \"mgi_nowhere\"".to_string()),
    ];
    // (`MG_DEPTHS`: the chains' depths, comma-separated; 15 to 18 by
    // default.)
    let depths = std::env::var("MG_DEPTHS").unwrap_or_else(|_| "15,16,17,18".into());
    for n in depths.split(',').filter_map(|n| n.trim().parse::<usize>().ok()) {
        out.push((format!("deep{n}"), format!("#include \"mgc{n}_001\"")));
    }
    // (`MG_LOOP`: a file that includes itself, unguarded.)
    if std::env::var_os("MG_LOOP").is_some() {
        out.push(("loop".to_string(), "#include \"mgi_loop\"".to_string()));
    }
    out
}

/// How the client splices a program's includes (the deferred list's M0,
/// the three places Moonglow assumes what the engine does): north of the
/// player, a row of cards, each drawn by a custom pair whose fragment
/// program ([`include_cases`]) has, before `main`, nothing (`control`);
/// `#include "mgi_color"` indented by four spaces (`spaces`) or a tab
/// (`tab`), the file defining the green `MgColor()` the program paints; an
/// include of a file that is nowhere (`missing`), painting green; an
/// include of the first of a chain of files 15, 16, 17 and 18 deep
/// (`deep<n>`: `mgc<n>_01` includes `mgc<n>_02`… the last defines
/// `MG_DEEP`), painting green where `MG_DEEP` is defined and red where it
/// is not. Each card's texture is the four-color quadrant TGA, which a
/// program that fails might fall back to drawing. Beside it, Moonglow's
/// area view with the Shaders switch on (`moonglow_on.png`: the same
/// programs, assembled by `mg_shader`), and its own shader
/// (`moonglow.png`, the quadrants everywhere: a control).
///
/// Seen (2026-11-01, 89.8193.37): all green but `missing`, which the client
/// drew magenta (its program failed); chains of 15 to 18, 24, 32, 64, 128, 250
/// and 1,000 (`MG_DEPTHS`) all green. Moonglow drew `missing` and the chains
/// past 16 with its own shader (the quadrants).
#[test]
#[ignore]
fn batch6_include_splicing() {
    let base = base_game();
    let cases = include_cases();
    let nodes: Vec<String> = cases
        .iter()
        .enumerate()
        .map(|(i, (case, _))| {
            let x = -0.5 * cases.len() as f32 + i as f32 * 1.0;
            card(case, "mg_incl", (x, 0.2), (0.8, 1.2), &format!("mgi_{case}"))
        })
        .collect();
    let (table, first) = placeables_with(&base, &[("mg_incl", 0)]);
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("mg_incl.mdl".into(), super::batch4::probe_model("mg_incl", &nodes).into_bytes()),
        ("mgplainvs.shd".into(), PLAIN_VS.as_bytes().to_vec()),
        ("mgquad.tga".into(), quadrant_tga(32)),
        ("placeables.2da".into(), table.into_bytes()),
        ("mgi_color.shd".into(), b"vec3 MgColor() { return vec3(0.0, 1.0, 0.0); }\n".to_vec()),
        ("mgi_loop.shd".into(), b"#include \"mgi_loop\"\n".to_vec()),
    ];
    for (case, before) in &cases {
        let paint = if case.starts_with("deep") {
            "vec4(0.0, 1.0, 0.0, 1.0);\n#ifndef MG_DEEP\n    gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);\
             \n#endif\n    gl_FragColor = gl_FragColor"
        } else if before.contains("mgi_color") {
            "vec4(MgColor(), 1.0)"
        } else {
            "vec4(0.0, 1.0, 0.0, 1.0)"
        };
        files.push((format!("mgi_{case}fs.shd"), paint_fs(before, paint).into_bytes()));
        files.push((format!("mgi_{case}.mtr"), custom_mtr("mgquad", &format!("mgi_{case}fs"))));
        if let Some(n) = case.strip_prefix("deep").and_then(|n| n.parse::<usize>().ok()) {
            for k in 1..=n {
                let text = if k < n {
                    format!("#include \"mgc{n}_{:03}\"\n", k + 1)
                } else {
                    "#define MG_DEEP 1\n".to_string()
                };
                files.push((format!("mgc{n}_{k:03}.shd"), text.into_bytes()));
            }
        }
    }
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("include_splicing", "tic01", light, &as_overrides(&files));
    r.place(first, [20.0, 24.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// A 16-bit TGA as a model's texture (the NUI Creator's "15- and 16-bit
/// TGAs": the client draws a NUI image of one with level c as c × 255 / 31
/// rounded down and no alpha from its attribute bit; a model's texture of
/// the same file not read): `nui.rs`'s `mg16.tga` (a white band, then the
/// 32 levels of red with the bit set, then of green with it clear, 8
/// texels a level) on three cards 8 m wide north of the player, one above
/// the other, each a custom pair painting, bottom to top: the texture as
/// it is sampled (`raw`), its lowest bit (`fract(t × 255 / 2)`: 0.25 even,
/// 0.75 odd, so a level a step off shows), and its alpha as gray
/// (`alpha`). Beside it, Moonglow's area view with the Shaders switch on
/// (`moonglow_on.png`: the same programs, the texture as `mg-image` reads
/// it).
///
/// Seen (2026-11-01, 89.8193.37): levels `(c << 3) | (c >> 2)` in red and
/// green (33, 66 … 247), the alpha card white over both bands (no alpha from
/// the bit). The first run, without the `raw32` and `stock16` controls, drew
/// the cards as uninitialized texture memory (other textures' scraps and
/// noise); seen once.
#[test]
#[ignore]
fn batch6_tga16_texture() {
    let base = base_game();
    let paints = [
        ("raw", "vec4(tex.rgb, 1.0)"),
        ("lsb", "vec4(fract(tex.rgb * 255.0 * 0.5 + 0.25), 1.0)"),
        ("alpha", "vec4(vec3(tex.a), 1.0)"),
    ];
    let mut nodes: Vec<String> = paints
        .iter()
        .enumerate()
        .map(|(i, (p, _))| {
            card(p, "mg_tga16", (-4.0, 0.05 + 1.6 * i as f32), (8.0, 1.5), &format!("mgt16_{p}"))
        })
        .collect();
    // (Controls, west and east of them: the same picture as a 32-bit TGA
    // through the same custom pair (`mgt32_raw`), and the 16-bit TGA as a
    // mesh's own bitmap, drawn by the stock programs.)
    nodes.push(card("raw32", "mg_tga16", (-6.1, 0.05), (2.0, 1.5), "mgt32_raw"));
    nodes.push(card("stock16", "mg_tga16", (4.1, 0.05), (2.0, 1.5), "mg16"));
    let (table, first) = placeables_with(&base, &[("mg_tga16", 0)]);
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("mg_tga16.mdl".into(), super::batch4::probe_model("mg_tga16", &nodes).into_bytes()),
        ("mgplainvs.shd".into(), PLAIN_VS.as_bytes().to_vec()),
        ("mg16.tga".into(), super::nui::tga16()),
        (
            "mg32.tga".into(),
            mg_image::write_tga(
                &mg_image::read(ResType::TGA, &super::nui::tga16()).unwrap().to_rgba(),
            ),
        ),
        ("mgt32_raw.mtr".into(), custom_mtr("mg32", "mgt16_rawfs")),
        ("placeables.2da".into(), table.into_bytes()),
    ];
    for (p, paint) in paints {
        files.push((format!("mgt16_{p}fs.shd"), paint_fs("", paint).into_bytes()));
        files.push((format!("mgt16_{p}.mtr"), custom_mtr("mg16", &format!("mgt16_{p}fs"))));
    }
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("tga16_texture", "tic01", light, &as_overrides(&files));
    r.place(first, [20.0, 24.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// A case of [`batch6_replaced_textures`]: the texture replaced, by the
/// script (else the field), the items worn (slot bit, blueprint), the
/// facing.
type ReplaceCase<'a> = (&'a str, bool, &'a [(u32, &'a str)], f32);

/// A creature's texture replacements (`TextureReplace`, as the Visuals
/// page writes it, and `ReplaceObjectTexture`, the script call that sets
/// it; GitHub issue 32): north of the player, west to east, commoners
/// made humans (appearance 6, head 1), each with one texture replaced by
/// `mgred` (a red TGA in the override), by the field or by the script:
/// the head's PLT `pmh0_head001` (bare-headed, facing the camera), a
/// helmet's (`nw_arhe001`) PLT `helm_001` (facing the camera), a cloak's
/// (`x2_it_mcloak001`) PLT `cloak_003` (back to the camera); then an ogre
/// with its TGA `c_ogrea` (its appearance 127's `c_ogreA`) replaced by the field (the field taken on a
/// creature at all). Beside it, Moonglow's area view (which draws the
/// object's own replacements by the field).
///
/// Seen (2026-11-01, 89.8193.37): the head red by the field and by the script;
/// the helmet and the cloak as they were either way; the ogre as it was (its
/// texture is `c_ogrea`, appearance 127's `c_ogreA`: the case named `c_ogre`
/// then, corrected since, not run again). Moonglow drew the field's head and
/// helmet red before the fix (`Part::item`).
#[test]
#[ignore]
fn batch6_replaced_textures() {
    let red = Rgba { width: 8, height: 8, data: [255, 0, 0, 255].repeat(64) };
    let overrides = [("mgred.tga", mg_image::write_tga(&red))];
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("replaced_textures", "tic01", light, &overrides);
    let replace = |old: &str| {
        let mut e = mg_gff::Struct::new(0);
        e.set("OldTexture", Value::resref(ResRef::from_str(old).unwrap()));
        e.set("NewTexture", Value::resref(ResRef::from_str("mgred").unwrap()));
        let mut t = mg_gff::Struct::new(9);
        t.set("TextureReplaceLi", Value::List(vec![e]));
        Value::Struct(t)
    };
    // (What's replaced, by the script or the field, the items worn, the
    // facing: 180 faces the camera.)
    let helmet: &[(u32, &str)] = &[(1, "nw_arhe001")];
    let cloak: &[(u32, &str)] = &[(64, "x2_it_mcloak001")];
    let cases: [ReplaceCase; 6] = [
        ("pmh0_head001", false, &[], 180.0),
        ("pmh0_head001", true, &[], 180.0),
        ("helm_001", false, helmet, 180.0),
        ("helm_001", true, helmet, 180.0),
        ("cloak_003", false, cloak, 0.0),
        ("cloak_003", true, cloak, 0.0),
    ];
    let mut script = String::new();
    for (i, (old, by_script, equip, facing)) in cases.into_iter().enumerate() {
        let tag = format!("MG_R{i}");
        let mut fields = vec![
            ("Appearance_Type", Value::Word(6)),
            ("Appearance_Head", Value::Byte(1)),
            ("Tag", Value::String(tag.clone().into_bytes())),
        ];
        if by_script {
            script += &format!(
                "ReplaceObjectTexture(GetObjectByTag(\"{tag}\"), \"{old}\", \"mgred\");\n    "
            );
        } else {
            fields.push(("TextureReplace", replace(old)));
        }
        let x = 16.5 + 1.0 * i as f32;
        super::batch4::put_creature(&mut r, "nw_commale", [x, 22.0, 0.0], facing, &fields, equip);
    }
    super::batch4::put_creature(
        &mut r,
        "nw_ogre01",
        [23.5, 22.5, 0.0],
        180.0,
        &[("TextureReplace", replace("c_ogrea"))],
        &[],
    );
    let body = format!("{INVISIBLE}\n    {script}");
    let (dir, _) = super::batch4::shoot_close(r, &body, SETTINGS, (90.0, 7.0, 70.0), &|_| {});
    print_log(&dir);
}

/// What `fslit` is patched to paint in [`batch6_cut_out_lighter`], one a
/// run: `whole` (as it is), the color before lighting (`color`: the
/// texture times the material, linear, its alpha 1), the lighting alone
/// (`light`: white lit, as the program writes it, its alpha the
/// fragment's), the fragment's alpha (`alpha`, gray, its alpha 1), and
/// black with the fragment's alpha (`black`: what the blending keeps of
/// what lies behind).
const CUT_PARTS: [(&str, Option<&str>); 5] = [
    ("whole", None),
    (
        "color",
        Some(
            "FragmentColor = vec4(1.0);\n    SetupStandardShaderInputs();\n    \
         gl_FragColor = vec4(FragmentColor.rgb, 1.0);",
        ),
    ),
    (
        "light",
        Some(
            "FragmentColor = vec4(1.0);\n    SetupStandardShaderInputs();\n    \
         FragmentColor.rgb = vec3(1.0);\n    ApplyStandardShader();\n    gl_FragColor = FragmentColor;",
        ),
    ),
    (
        "alpha",
        Some(
            "FragmentColor = vec4(1.0);\n    SetupStandardShaderInputs();\n    \
         gl_FragColor = vec4(vec3(FragmentColor.a), 1.0);",
        ),
    ),
    (
        "black",
        Some(
            "FragmentColor = vec4(1.0);\n    SetupStandardShaderInputs();\n    \
         gl_FragColor = vec4(0.0, 0.0, 0.0, FragmentColor.a);",
        ),
    ),
];

/// An 8×8 TGA of one color.
fn flat_tga(rgb: [u8; 3]) -> Vec<u8> {
    let img = Rgba { width: 8, height: 8, data: [rgb[0], rgb[1], rgb[2], 255].repeat(64) };
    mg_image::write_tga(&img)
}

/// A model of the chain of [`batch6_supermodel_chain`]: `name`, its
/// supermodel `sup` (`NULL`: none) and its `setanimationscale`, two cards
/// on its root (`base`, red, and `mover`, green, a little east of it,
/// both at the ground), and, where it is the chain's last
/// (`sup` `NULL`), a `default` animation holding `mover` 0.4 m up.
fn chain_model(name: &str, sup: &str, scale: f32) -> String {
    let nodes = [
        card("base", name, (-0.25, 0.0), (0.2, 0.3), "mgchred"),
        card("mover", name, (0.05, 0.0), (0.2, 0.3), "mgchgreen"),
    ];
    let anim = if sup == "NULL" {
        format!(
            "newanim default {name}\n  length 1\n  transtime 0\n  animroot {name}\n\
             node dummy {name}\n  parent NULL\nendnode\n\
             node trimesh base\n  parent {name}\nendnode\n\
             node trimesh mover\n  parent {name}\n  positionkey 2\n    0 0.05 0 0.4\n    \
             1 0.05 0 0.4\nendnode\ndoneanim default {name}\n"
        )
    } else {
        String::new()
    };
    super::batch4::probe_model(name, &nodes)
        .replace(&format!("setsupermodel {name} NULL"), &format!("setsupermodel {name} {sup}"))
        .replace("setanimationscale 1", &format!("setanimationscale {scale}"))
        .replace(&format!("donemodel {name}\n"), &format!("{anim}donemodel {name}\n"))
}

/// Which scale a chain of scaled supermodels plays its animation at (the
/// deferred list's "Custom creatures from the Vault and CEP"): dynamic
/// placeables, west to east, of models ([`chain_model`]) whose last link
/// `mg_chc` (scale 1) holds a green card 0.4 m up in its `default`
/// animation: `mg_chc` itself, `mg_chb` (scale 3, on `mg_chc`), `mg_cha`
/// (scale 2, on `mg_chb`), `mg_chd` (scale 2, on `mg_chc`) and `mg_che`
/// (scale 1, on `mg_chb`). The green card's height over the red one tells
/// the scale its animation plays at: Moonglow's, the playing model's alone
/// (0.4, 1.2, 0.8, 0.8, 0.4 m), or each link's together (0.4, 1.2, 2.4,
/// 0.8, 1.2). Beside it, Moonglow's area view (Animations on).
///
/// Seen (2026-11-01, 89.8193.37): the green card above the red by 13, 40, 83,
/// 26 and 40 pixels in the client (× 1, 3, 6, 2, 3: each link's scale
/// together), 12, 38, 24, 24 and 12 in Moonglow (the playing model's alone);
/// the same with `MG_CHAIN_OWNER=0.5`.
#[test]
#[ignore]
fn batch6_supermodel_chain() {
    let base = base_game();
    // (`MG_CHAIN_OWNER`: the scale of `mg_chc`, whose animation it is; 1
    // by default.)
    let owner: f32 =
        std::env::var("MG_CHAIN_OWNER").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let links = [
        ("mg_chc", "NULL", owner),
        ("mg_chb", "mg_chc", 3.0),
        ("mg_cha", "mg_chb", 2.0),
        ("mg_chd", "mg_chc", 2.0),
        ("mg_che", "mg_chb", 1.0),
    ];
    let models: Vec<(&str, u8)> = links.iter().map(|(n, _, _)| (*n, 0)).collect();
    let (table, first) = placeables_with(&base, &models);
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("placeables.2da".into(), table.into_bytes()),
        ("mgchred.tga".into(), flat_tga([255, 0, 0])),
        ("mgchgreen.tga".into(), flat_tga([0, 255, 0])),
    ];
    for (name, sup, scale) in links {
        files.push((format!("{name}.mdl"), chain_model(name, sup, scale).into_bytes()));
    }
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r =
        Reading::new(&format!("supermodel_chain_{owner}"), "tic01", light, &as_overrides(&files));
    for i in 0..links.len() {
        let x = 17.0 + 1.5 * i as f32;
        r.place(first + i as u32, [x, 23.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    }
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|v| {
        v.animate = true;
        v.time = 0.5;
    });
    eprintln!("{}", dir.display());
}

/// A `Linked` ribbon's cell at more particle sizes, and a see-through
/// mesh of another model in front of one (the deferred list's "A `Linked`
/// ribbon and a flip-book"): `batch5.rs`'s `reading_ribbon_pieces_and_order`
/// again, for each size of `MG_SIZES` (comma-separated metres; 0.15, 0.45,
/// 0.9 and 1.2 by default), seen from the side as it was: north of the
/// player, west to east, ribbons of particles of the gradient texture
/// (green its v) rising 1 m/s (one a second: pieces a metre long), 0.5
/// m/s, and twice 1 m/s, the third behind a red card of mesh `alpha 0.5`
/// of another placeable (`mg_rib6c`, 0.6 m nearer the camera, 1 to 1.6 m
/// up), the fourth behind one of its own model (the fifth batch's case).
/// `ribbon_profiles.py` in the scratch directory reads the cells off the
/// pictures.
///
/// Seen (2026-11-01, 89.8193.37): the cell's v at the next particle 0.93,
/// 0.82, 0.69 and 0.63 at 0.15, 0.45, 0.9 and 1.2 m (a metre's gap; the
/// ribbons twice their particles' size across, overlapping at the larger
/// sizes); both cards hide the ribbon behind them (red over the wall).
#[test]
#[ignore]
fn batch6_ribbon_sizes() {
    let base = base_game();
    let sizes = std::env::var("MG_SIZES").unwrap_or_else(|_| "0.15,0.45,0.9,1.2".into());
    let card = |m: &str, name: &str, x: f32, y: f32| {
        format!(
            "node trimesh {name}\n  parent {m}\n  position {x} {y} 1.0\n  orientation 0 0 0 0\n  \
             ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap mgred\n  \
             alpha 0.5\n  render 1\n  shadow 0\n  verts 4\n    -0.25 0 0\n    0.25 0 0\n    \
             0.25 0 0.6\n    -0.25 0 0.6\n  tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  \
             faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n"
        )
    };
    for size in sizes.split(',') {
        let m = "mg_rib6";
        let p = format!(
            "texture mggrad\n  sizeStart {size}\n  sizeEnd {size}\n  birthrate 1\n  lifeExp 4\n  \
             render Linked"
        );
        let e = super::batch4::emitter;
        let model = super::batch4::probe_model(
            m,
            &[
                e(m, "link1", [-0.9, 0.0, 0.0], &format!("{p}\n  velocity 1")),
                e(m, "link05", [-0.3, 0.0, 0.0], &format!("{p}\n  velocity 0.5")),
                e(m, "linkc", [0.3, 0.0, 0.0], &format!("{p}\n  velocity 1")),
                e(m, "linko", [0.9, 0.0, 0.0], &format!("{p}\n  velocity 1")),
                card(m, "own", 0.9, -0.6),
            ],
        );
        let other = super::batch4::probe_model("mg_rib6c", &[card("mg_rib6c", "other", 0.0, 0.0)]);
        let (table, first) = placeables_with(&base, &[(m, 0), ("mg_rib6c", 0)]);
        let red = Rgba { width: 4, height: 4, data: [255, 0, 0, 255].repeat(16) };
        let files: Vec<(String, Vec<u8>)> = vec![
            ("mg_rib6.mdl".into(), model.into_bytes()),
            ("mg_rib6c.mdl".into(), other.into_bytes()),
            ("mggrad.tga".into(), super::batch5::gradient_tga()),
            ("mgred.tga".into(), mg_image::write_tga(&red)),
            ("placeables.2da".into(), table.into_bytes()),
        ];
        let light =
            Lighting { ambient: 0x606060, diffuse: 0x606060, main_light: 0, only_tile: None };
        let mut r =
            Reading::new(&format!("ribbon_sizes_{size}"), "tic01", light, &as_overrides(&files));
        r.place(first, [20.0, 24.0, 0.0], 0.0, &[]);
        r.place(first + 1, [20.3, 23.4, 0.0], 0.0, &[]);
        let (dir, _) =
            super::batch4::shoot_close(r, INVISIBLE, SETTINGS, (90.0, 6.0, 80.0), &|v| {
                v.animate = true
            });
        eprintln!("{}", dir.display());
    }
}

/// What `fsgrass` paints in [`batch6_grass_material`], one a run: the
/// tuft as it is, the material's ambient, diffuse and emissive colors the
/// engine gives the draw (raw, alpha 1), and the vertex color the vertex
/// program lit it with.
const GRASS_PARTS: [(&str, Option<&str>); 7] = [
    ("whole", None),
    // (The fade: red the diffuse's alpha, green the fragment's distance
    // from the camera and blue from the camera's focus (the player), a
    // 64th of a meter each, wrapped every 4 m in the low half.)
    (
        "fade",
        Some(
            "vec4(materialFrontDiffuse.a, length(vPosWorld - cameraPosition) / 64.0, \
             length(vPosWorld.xy - cameraFocus.xy) / 64.0, 1.0)",
        ),
    ),
    ("ambient", Some("vec4(materialFrontAmbient.rgb, 1.0)")),
    ("diffuse", Some("vec4(materialFrontDiffuse.rgb, 1.0)")),
    ("diffuse_a", Some("vec4(vec3(materialFrontDiffuse.a), 1.0)")),
    ("emissive", Some("vec4(materialFrontEmissive.rgb, 1.0)")),
    ("vertex", Some("vec4(VertexColor.rgb, 1.0)")),
];

/// The material colors the engine gives the area's grass (the deferred
/// list's M5, "The grass": the `.set`'s [GRASS] colors not read): a rural
/// field (ttr01, whose [GRASS] has ambient and diffuse 0.5, 0.6, 0.3)
/// with the client's grass on, seen from the reference scenes' camera
/// (where Moonglow's view grows its grass), `fsgrass` patched to paint one of
/// [`GRASS_PARTS`] a run (`MG_PARTS` picks). Each run prints the picture's
/// most common colors (the tufts' among them), the client's and
/// Moonglow's with the Shaders switch on (the same patch, on its tufts).
///
/// Seen (2026-11-01, 89.8193.37): ambient and diffuse (55, 83, 18) in both
/// (ttr01's colors made linear), emissive 0, the vertex color (16, 24, 5) in
/// both; the diffuse's alpha 1 in Moonglow, falling with the camera's distance
/// in the client (`fade`: 0.94 at 17 m to 0.49 at 24).
#[test]
#[ignore]
fn batch6_grass_material() {
    let base = base_game();
    let fsgrass = program(&base, "fsgrass");
    let only = std::env::var("MG_PARTS").unwrap_or_default();
    for (part, paint) in GRASS_PARTS {
        if !only.is_empty() && !only.split(',').any(|p| p == part) {
            continue;
        }
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        if let Some(paint) = paint {
            let mut src = patched(
                &fsgrass,
                "gl_FragColor = FragmentColor;",
                &format!("gl_FragColor = {paint};"),
            );
            if part == "fade" {
                // (The world position handed on, as `sway_programs` has it.)
                let world = "#define POSITION_WORLD 1\n";
                src = format!("{world}#include \"inc_uniforms\"\n{src}");
                let vs = format!("{world}{}", program(&base, "vsgrass"));
                files.push(("vsgrass.shd".into(), vs.into_bytes()));
            }
            files.push(("fsgrass.shd".into(), src.into_bytes()));
        }
        let light =
            Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
        let mut r =
            Reading::new(&format!("grass_material_{part}"), "ttr01", light, &as_overrides(&files));
        r.are.root.set("WindPower", Value::Int(0));
        let (dir, _) = r.shoot_beside(INVISIBLE, &super::readings::with_grass(), &|_| {});
        for name in ["client", "moonglow_on"] {
            let path = dir.join(format!("{name}.png"));
            if !path.exists() {
                continue;
            }
            let img = common::read_png(&path);
            let mut counts: HashMap<[u8; 3], usize> = HashMap::new();
            for p in img.data.as_chunks::<4>().0 {
                *counts.entry([p[0], p[1], p[2]]).or_default() += 1;
            }
            let mut top: Vec<_> = counts.into_iter().collect();
            top.sort_by_key(|t| std::cmp::Reverse(t.1));
            let top: Vec<String> = top.iter().take(6).map(|(c, n)| format!("{c:?} ×{n}")).collect();
            eprintln!("{part}: {name}: {}", top.join(", "));
        }
    }
}

/// A TGA `side` texels square whose red is its u and green its v (0 to
/// 252), blue 0: the texel a pixel shows.
fn uv_tga(side: u16) -> Vec<u8> {
    // (Bottom row first, BGRA.)
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    t.extend(side.to_le_bytes());
    t.extend(side.to_le_bytes());
    t.extend([32, 0]);
    let level = |i: u16| (f32::from(i) * 252.0 / f32::from(side - 1)).round() as u8;
    for y in 0..side {
        for x in 0..side {
            t.extend([0, level(y), level(x), 255]);
        }
    }
    t
}

/// The ripple's probes of [`batch6_ripple`], in order (two rows of four,
/// west to east, the far row first): the card's name and its TXI (`None`:
/// none). `zombie` is `dd_ce3_txhotcoal`'s.
const RIPPLES: [(&str, Option<&str>); 8] = [
    ("none", None),
    (
        "zombie",
        Some(
            "proceduretype arturo\ndistort 1\narturowidth 32\narturoheight 32\n\
             distortionamplitude 10\nspeed 16\nchannelscale 4\n0\n0\n0\n0\n\
             channeltranslate 4\n1\n1\n1\n1\n",
        ),
    ),
    (
        "distort0",
        Some(
            "proceduretype arturo\ndistort 0\narturowidth 32\narturoheight 32\n\
             distortionamplitude 10\nspeed 16\nchannelscale 4\n0\n0\n0\n0\n\
             channeltranslate 4\n1\n1\n1\n1\n",
        ),
    ),
    (
        "amp40",
        Some(
            "proceduretype arturo\ndistort 1\narturowidth 32\narturoheight 32\n\
             distortionamplitude 40\nspeed 16\nchannelscale 4\n0\n0\n0\n0\n\
             channeltranslate 4\n1\n1\n1\n1\n",
        ),
    ),
    (
        "width64",
        Some(
            "proceduretype arturo\ndistort 1\narturowidth 32\narturoheight 32\n\
             distortionamplitude 10\nspeed 16\ndefaultwidth 64\nchannelscale 4\n0\n0\n0\n0\n\
             channeltranslate 4\n1\n1\n1\n1\n",
        ),
    ),
    (
        "width16",
        Some(
            "proceduretype arturo\ndistort 1\narturowidth 32\narturoheight 32\n\
             distortionamplitude 10\nspeed 16\ndefaultwidth 16\nchannelscale 4\n0\n0\n0\n0\n\
             channeltranslate 4\n1\n1\n1\n1\n",
        ),
    ),
    (
        "speed4",
        Some(
            "proceduretype arturo\ndistort 1\narturowidth 32\narturoheight 32\n\
             distortionamplitude 10\nspeed 4\nchannelscale 4\n0\n0\n0\n0\n\
             channeltranslate 4\n1\n1\n1\n1\n",
        ),
    ),
    (
        "stock",
        Some(
            "proceduretype arturo\ndistort 1\narturowidth 32\narturoheight 32\n\
             distortionamplitude 10\nspeed 16\nchannelscale 4\n0\n0\n0\n0\n\
             channeltranslate 4\n1\n1\n1\n1\n",
        ),
    ),
];

/// How far and how fast a texture ripples (the deferred list's "A rippling
/// glow"): north of the player, two rows of four cards ([`RIPPLES`]), each
/// a texture whose red is its u and green its v ([`uv_tga`], under its own
/// name with its probe's TXI), painted as sampled by a custom pair (no
/// lighting: the colour a pixel shows is the texel it samples), with the
/// client's clock (`worldtimerTimeOfDay`'s three low bytes) along each
/// card's foot; the last card (`stock`) the zombie's TXI drawn by the
/// stock program (whether the custom pair's texture ripples as the
/// stock's does). Seen close (`SetCameraFacing(90, 5, 70)`), `MG_SHOTS`
/// pictures (24) as fast as they come. `ripple_profiles.py` in the
/// scratch directory reads them.
///
/// Seen (2026-11-01, 89.8193.37): see `docs/findings.md`, "How far and how
/// fast a texture ripples" (with `MG_SHOTS=80 MG_GAP_MS=150`, and
/// `MG_RIPPLE_SIDE=128`).
#[test]
#[ignore]
fn batch6_ripple() {
    let base = base_game();
    let shots = std::env::var("MG_SHOTS").ok().and_then(|s| s.parse().ok()).unwrap_or(24);
    let paint = "vec4(tex.rgb, 1.0);\n    int t = worldtimerTimeOfDay;\n    \
                 if (vVertexTexCoords.y < 0.1) gl_FragColor = vec4(float(t & 255) / 255.0, \
                 float((t >> 8) & 255) / 255.0, float((t >> 16) & 255) / 255.0, 1.0)";
    let mut nodes = Vec::new();
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("mgplainvs.shd".into(), PLAIN_VS.as_bytes().to_vec()),
        ("mgripfs.shd".into(), paint_fs("", paint).into_bytes()),
    ];
    for (i, (name, txi)) in RIPPLES.iter().enumerate() {
        let (row, col) = (i / 4, i % 4);
        let bitmap = format!("mgrip_{name}");
        // (The far row 1.5 m behind, raised 1.2 m: both in view.)
        let far = (1 - row) as f32;
        let at = [-2.2 + 1.1 * col as f32, 1.5 * far, 0.05 + 1.2 * far];
        nodes.push(card_at(name, "mg_ripple", at, (1.0, 1.0), &bitmap));
        // (`MG_RIPPLE_SIDE`: the texture's side, 64 by default.)
        let side = std::env::var("MG_RIPPLE_SIDE").ok().and_then(|s| s.parse().ok()).unwrap_or(64);
        files.push((format!("{bitmap}.tga"), uv_tga(side)));
        if let Some(txi) = txi {
            files.push((format!("{bitmap}.txi"), txi.as_bytes().to_vec()));
        }
        if *name != "stock" {
            files.push((format!("{bitmap}.mtr"), custom_mtr(&bitmap, "mgripfs")));
        }
    }
    let (table, first) = placeables_with(&base, &[("mg_ripple", 0)]);
    files.push((
        "mg_ripple.mdl".into(),
        super::batch4::probe_model("mg_ripple", &nodes).into_bytes(),
    ));
    files.push(("placeables.2da".into(), table.into_bytes()));
    let light = Lighting { ambient: 0x808080, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("ripple", "tic01", light, &as_overrides(&files));
    r.place(first, [20.0, 23.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    r.save(&enter(INVISIBLE, (90.0, 5.0, 70.0)), SETTINGS);
    // (`MG_GAP_MS`: milliseconds between pictures, 0 by default.)
    let gap = std::env::var("MG_GAP_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let pictures = common::client_screenshots(&r.dir, "MgScene", shots, Duration::from_millis(gap))
        .expect("the client's pictures");
    for (i, (t, img)) in pictures.iter().enumerate() {
        save_png(img, &r.dir.join(format!("ripple_{i:03}.png")));
        eprintln!("picture {i} at {t:.2} s");
    }
    eprintln!("{}", r.dir.display());
}

/// Which environment map an item lying on the ground reflects when it
/// lies as its own model (the deferred list's M4: the fourth batch's
/// amulet lay as the bag, and its sword's sphere drew in neither): the
/// scene of `batch4.rs`'s `reading_item_env_map` (a rural interior,
/// `tin01`, whose map `tin01__ref01` is a copper brown, `inc_standard`
/// painting each environment-mapped fragment with its map), north of the
/// player, west, a small shield (`nw_ashsw001`, `ModelPart1` 11: base
/// item 14, `Part1EnvMap` 1) whose model `ashsw_011` is made a sphere of
/// a gray texture of alpha 0; east, the big mirror (`Reflection`
/// `default`) made the same sphere. In the client beside Moonglow's area
/// view, from close.
///
/// Seen (2026-11-01, 89.8193.37): the shield's sphere painted with
/// `tin01__ref01` (copper) in the client, `chrome1` in Moonglow before the
/// fix; the mirror's `chrome1` in both.
#[test]
#[ignore]
fn batch6_item_env_map_own_model() {
    use mg_module::instances::{Placement, Placing, instance};
    let base = base_game();
    let ball = super::batch4::sphere_model;
    let overrides = [
        ("ashsw_011.mdl", ball("ashsw_011", "item", "mgshiny", 0.5).into_bytes()),
        ("px2_b02.mdl", ball("px2_b02", "character", "mgshiny", 1.2).into_bytes()),
        ("mgshiny.tga", super::batch4::gray_alpha_tga(0)),
        ("inc_standard.shd", super::batch4::env_paint_shader(&base).into_bytes()),
    ];
    let light = Lighting { ambient: 0x808080, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("item_env_map_own_model", "tin01", light, &overrides);
    let bp = Gff::read(&r.game.resman.get_named("nw_ashsw001", ResType::UTI).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &r.game, item: &none };
    let at = Placement { position: [19.0, 22.0, 0.0], rotation: 0.0 };
    let item = instance(&placing, ResType::UTI, &bp.root, at, &[]).unwrap();
    r.git.root.set(mg_area::ObjectKind::Item.list(), Value::List(vec![item]));
    r.place(428, [21.5, 22.5, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = super::batch4::shoot_close(r, INVISIBLE, SETTINGS, (90.0, 5.0, 60.0), &|_| {});
    print_log(&dir);
}

/// A BioWare DDS (`width`, `height`, channels 3 or 4, the top level's
/// size, alpha mean, then the levels) with the standard `DDS ` header in
/// its place (DXT1 or DXT5, its levels as they are).
fn standard_dds(bioware: &[u8]) -> Vec<u8> {
    let u = |i: usize| u32::from_le_bytes(bioware[i..i + 4].try_into().unwrap());
    let (w, h, channels) = (u(0), u(4), u(8));
    let block = if channels == 3 { 8 } else { 16 };
    let data = &bioware[20..];
    let (mut levels, mut at) = (0u32, 0usize);
    while at < data.len() {
        let (lw, lh) = ((w >> levels).max(1), (h >> levels).max(1));
        at += (lw.div_ceil(4) * lh.div_ceil(4) * block) as usize;
        levels += 1;
        if lw == 1 && lh == 1 {
            break;
        }
    }
    let mut out = b"DDS ".to_vec();
    let mut put = |v: u32| out.extend(v.to_le_bytes());
    // (DDSD_CAPS | HEIGHT | WIDTH | PIXELFORMAT | MIPMAPCOUNT | LINEARSIZE.)
    put(124);
    put(0x1 | 0x2 | 0x4 | 0x1000 | 0x20000 | 0x80000);
    put(h);
    put(w);
    put(w.div_ceil(4) * h.div_ceil(4) * block);
    put(0);
    put(levels);
    for _ in 0..11 {
        put(0);
    }
    // The pixel format: FOURCC.
    put(32);
    put(0x4);
    out.extend(if channels == 3 { b"DXT1" } else { b"DXT5" });
    for _ in 0..5 {
        out.extend(0u32.to_le_bytes());
    }
    // (DDSCAPS_TEXTURE | MIPMAP | COMPLEX.)
    out.extend((0x1000u32 | 0x400000 | 0x8).to_le_bytes());
    for _ in 0..4 {
        out.extend(0u32.to_le_bytes());
    }
    out.extend(&data[..at.min(data.len())]);
    out
}

/// What the client makes of a BioWare DXT1 (the deferred list's M4: TDoD's
/// `dd_ce3_txburneds.dds` alone in the override drew its zombie a pale
/// gray, see-through, and the game's own `tic01_floor01.dds` in its place
/// pale white): north of the player, west to east, standing cards of the
/// game's `tic01_floor01` as the game has it (its key), the same file in
/// the override under another name (`bw`), its blocks under a standard
/// `DDS ` header (`std`), its picture as a TGA (`tga`); then, with The
/// Domains of Dread's haks, `dd_ce3_txburneds.dds` under its own name
/// (`zbw`), under a standard header (`zstd`), and two of its zombies
/// (appearance 28518, its model and the body DDS alone in the override, as
/// `reading_rippling_glow`'s `dds` case had it). Beside it, Moonglow's area
/// view.
///
/// Seen (2026-11-01, 89.8193.37): the four floor cards alike (the key's, the
/// override's BioWare DXT1, the standard header's, the TGA), and TDoD's two
/// charred cards alike; the zombies here pale and one aflame (their glow's
/// textures left out of the override: not the reading's question).
#[test]
#[ignore]
fn batch6_bioware_dxt1() {
    let base = base_game();
    let floor = base.resman.get_named("tic01_floor01", ResType::DDS).unwrap().to_vec();
    let picture = mg_image::read(ResType::DDS, &floor).unwrap().to_rgba();
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("mgdx_bw.dds".into(), floor.clone()),
        ("mgdx_std.dds".into(), standard_dds(&floor)),
        ("mgdx_tga.tga".into(), mg_image::write_tga(&picture)),
    ];
    let mut cards =
        vec![("key", "tic01_floor01"), ("bw", "mgdx_bw"), ("std", "mgdx_std"), ("tga", "mgdx_tga")];
    let tdod = super::batch4::tdod_haks().is_some_and(|d| d.join("tdod_p_ce3.hak").is_file());
    if tdod {
        let z = super::batch4::from_hak(
            "tdod_p_ce3.hak",
            &["dd_ce3_zombieash.mdl", "dd_ce3_txburneds.dds"],
        );
        let burned = z[1].1.clone();
        files.extend(z);
        files.push(("mgdx_zstd.dds".into(), standard_dds(&burned)));
        files.extend(super::batch4::from_hak("tdod_top.hak", &["appearance.2da"]));
        cards.extend([("zbw", "dd_ce3_txburneds"), ("zstd", "mgdx_zstd")]);
    }
    let nodes: Vec<String> = cards
        .iter()
        .enumerate()
        .map(|(i, (name, bitmap))| {
            card(name, "mg_dxt", (-3.3 + 1.1 * i as f32, 0.05), (1.0, 1.0), bitmap)
        })
        .collect();
    let (table, first) = placeables_with(&base, &[("mg_dxt", 0)]);
    files.push(("mg_dxt.mdl".into(), super::batch4::probe_model("mg_dxt", &nodes).into_bytes()));
    files.push(("placeables.2da".into(), table.into_bytes()));
    let light = Lighting { ambient: 0x808080, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("bioware_dxt1", "tic01", light, &as_overrides(&files));
    r.place(first, [20.0, 23.5, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    if tdod {
        for x in [19.0, 21.0] {
            super::batch4::put_creature(
                &mut r,
                "nw_commale",
                [x, 21.8, 0.0],
                270.0,
                &[("Appearance_Type", Value::Word(28518))],
                &[],
            );
        }
    }
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// Where `Best` (`with`) is darker than `Off` (`without`), white, for
/// looking at, written to `path`; and how many columns of the picture's
/// band `rows` (fractions) hold such pixels, as runs (one a shadow).
fn darker_runs(with: &Rgba, without: &Rgba, path: &Path, rows: [f32; 2]) -> Vec<(usize, usize)> {
    let mut diff = with.clone();
    let pairs = with.data.as_chunks::<4>().0.iter().zip(without.data.as_chunks::<4>().0);
    for (d, (b, o)) in diff.data.as_chunks_mut::<4>().0.iter_mut().zip(pairs) {
        let sum = |p: &[u8]| p[0] as f32 + p[1] as f32 + p[2] as f32;
        let v = ((sum(o) - sum(b)).max(0.0) * 2.0).min(255.0) as u8;
        d[..3].copy_from_slice(&[v, v, v]);
    }
    save_png(&diff, path);
    let (w, h) = (with.width as usize, with.height as usize);
    let (y0, y1) = ((rows[0] * h as f32) as usize, (rows[1] * h as f32) as usize);
    let dark: Vec<bool> = (0..w)
        .map(|x| (y0..y1).filter(|y| diff.data[(y * w + x) * 4] >= 12).count() >= 3)
        .collect();
    let mut runs = Vec::new();
    let mut start = None;
    for (x, d) in dark.iter().chain([&false]).enumerate() {
        match (start, d) {
            (None, true) => start = Some(x),
            (Some(s), false) => {
                if x - s >= 4 {
                    runs.push((s, x - 1));
                }
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// How many creatures cast a light's shadow at once (the deferred list's
/// "Shadows": `max-casting-lights` changed nothing with one; Moonglow casts
/// 16): a dim tic01 room (as `reading_light_shadows`), one lamp post
/// (placeables.2da row 73) at (20, 25), and `MG_OGRES` ogres (8 by
/// default) in a row 3 m south of it, 1.2 m apart, each one's shadow
/// falling south towards the camera; the client with Creature Shadow
/// Detail Best and Off, compared: the shadows south of their feet counted
/// (runs of darker columns), `darker.png` to look at.
///
/// Seen (2026-11-01, 89.8193.37): six runs of darker columns for eight ogres
/// (the server pushed some together), every ogre its own shadow.
#[test]
#[ignore]
fn batch6_light_shadow_casters() {
    let n: usize = std::env::var("MG_OGRES").ok().and_then(|s| s.parse().ok()).unwrap_or(8);
    let mut pictures = Vec::new();
    for mode in [2u8, 0] {
        let light =
            Lighting { ambient: 0x303030, diffuse: 0x303030, main_light: 0, only_tile: None };
        let mut r = Reading::new(&format!("light_shadow_casters_{n}_{mode}"), "tic01", light, &[]);
        for (label, v) in [("SunShadows", 0u8), ("MoonShadows", 0), ("ShadowOpacity", 100)] {
            r.are.root.set(label, Value::Byte(v));
        }
        for i in 0..n {
            let x = 20.0 + 1.2 * (i as f32 - (n as f32 - 1.0) / 2.0);
            super::batch3::put_creature(&mut r, "nw_ogre01", [x, 22.0, 0.0], 270.0);
        }
        r.place(73, [20.0, 25.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
        let body = format!("{INVISIBLE}\n{}", super::batch3::WHERE);
        let (dir, client) =
            r.shoot(&enter(&body, (90.0, 14.0, 50.0)), &super::batch3::shadow_settings(mode));
        print_log(&dir);
        pictures.push((dir, client));
    }
    let (best, off) = (&pictures[0].1, &pictures[1].1);
    let runs = darker_runs(best, off, &pictures[0].0.join("darker.png"), [0.52, 0.62]);
    eprintln!("{n} ogres: {} shadows south of their feet: {runs:?}", runs.len());
    eprintln!("{}", pictures[0].0.display());
}

/// A player's own light at night (the deferred list's "Shadows": Moonglow
/// places no player): a dark ttr01 field at night (no moon shadows), the
/// player visible with a white light of 20 m on itself
/// (`VFX_DUR_LIGHT_WHITE_20`), an ogre 2 m north of it; the client with
/// Creature Shadow Detail Best and Off, compared (`darker.png`): whether
/// the player's light casts the ogre's shadow (north, away from the
/// camera), and the player's own.
///
/// Seen (2026-11-01, 89.8193.37): the ogre's shadow north of it, from the
/// player's light, and the floor south of the player alike (0.05 and −0.01).
#[test]
#[ignore]
fn batch6_player_light_at_night() {
    let mut pictures = Vec::new();
    for mode in [2u8, 0] {
        let light =
            Lighting { ambient: 0x101010, diffuse: 0x101010, main_light: 0, only_tile: None };
        let mut r = Reading::new(&format!("player_light_{mode}"), "ttr01", light, &[]);
        for (label, v) in
            [("SunShadows", 0u8), ("MoonShadows", 0), ("ShadowOpacity", 100), ("IsNight", 1)]
        {
            r.are.root.set(label, Value::Byte(v));
        }
        super::batch3::put_creature(&mut r, "nw_ogre01", [20.0, 22.0, 0.0], 270.0);
        let body = format!(
            "ApplyEffectToObject(DURATION_TYPE_PERMANENT, \
             EffectVisualEffect(VFX_DUR_LIGHT_WHITE_20), pc);\n{}",
            super::batch3::WHERE
        );
        let (dir, client) =
            r.shoot(&enter(&body, (90.0, 10.0, 50.0)), &super::batch3::shadow_settings(mode));
        print_log(&dir);
        pictures.push((dir, client));
    }
    let (best, off) = (&pictures[0].1, &pictures[1].1);
    let runs = darker_runs(best, off, &pictures[0].0.join("darker.png"), [0.30, 0.50]);
    eprintln!("player's light: darker columns north of the ogre: {runs:?}");
    for (what, r) in [
        ("north of the ogre", [0.45, 0.30, 0.55, 0.42]),
        ("south of the player", [0.45, 0.60, 0.55, 0.70]),
    ] {
        eprintln!("{what}: darker by {:.3}", super::batch3::darker(best, off, r));
    }
    eprintln!("{}", pictures[0].0.display());
}

/// Creatures whose nodes an animation moves otherwise by number than by
/// name, walking (the deferred list's "Animations bind by part number"):
/// a mohrg (appearance 123, `c_mohrg`: `a_ba`'s cloak bones' keys fall on
/// its `dangles04`, `01` and `05` by number) and a blade (appearance 400,
/// `c_blade_m`: its `head_g` numbered −1, turned by `a_ba`'s walks by
/// name), walking east and west along the area, 3 m apart, seen from the
/// side (`SetCameraFacing(90, 9, 85)`), `MG_SHOTS` pictures (30) as fast
/// as they come. Moonglow's side: `mgv render c_mohrg --anim cwalk` (and
/// `c_blade_m`) from the side, at times through the walk.
///
/// Seen (2026-11-01, 89.8193.37): with `MG_WALKER=mohrg` and `blade`: the
/// mohrg's entrails swing with the walk and the blade's helmet stays upright,
/// as `mgv render c_mohrg --anim walk` and `c_blade_m` draw them from the
/// side.
#[test]
#[ignore]
fn batch6_walking() {
    let shots = std::env::var("MG_SHOTS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    // (`MG_WALKER`: `mohrg` or `blade` alone, walking 1.5 m north of the
    // player, seen from 4 m.)
    let walker = std::env::var("MG_WALKER").ok();
    let mut r = Reading::new(
        &format!("walking{}", walker.as_deref().map_or(String::new(), |w| format!("_{w}"))),
        "ttr01",
        light,
        &[],
    );
    let walkers: Vec<(&str, u16, f32)> = match walker.as_deref() {
        Some("mohrg") => vec![("MG_MOHRG", 123, 21.5)],
        Some("blade") => vec![("MG_BLADE", 400, 21.5)],
        _ => vec![("MG_MOHRG", 123, 23.0), ("MG_BLADE", 400, 26.0)],
    };
    let ys: Vec<f32> = walkers.iter().map(|w| w.2).collect();
    for &(tag, appearance, y) in &walkers {
        let fields = [
            ("Appearance_Type", Value::Word(appearance)),
            ("Tag", Value::String(tag.as_bytes().to_vec())),
            ("Equip_ItemList", Value::List(Vec::new())),
        ];
        super::batch4::put_creature(&mut r, "nw_commale", [16.0, y, 0.0], 0.0, &fields, &[]);
    }
    let body = format!(
        "{INVISIBLE}\n    int i;\n    for (i = 0; i < 2; i++)\n    {{\n        \
         object c = GetObjectByTag(i == 0 ? \"MG_MOHRG\" : \"MG_BLADE\");\n        \
         float y = i == 0 ? {:.1} : {:.1};\n        int k;\n        for (k = 0; k < 8; k++)\n        \
         {{\n            AssignCommand(c, ActionMoveToLocation(Location(area, Vector(24.0, y, 0.0), 0.0)));\n            \
         AssignCommand(c, ActionMoveToLocation(Location(area, Vector(16.0, y, 0.0), 180.0)));\n        \
         }}\n    }}",
        ys[0],
        ys.last().copied().unwrap_or(26.0)
    );
    let distance = if walker.is_some() { 4.0 } else { 9.0 };
    r.save(&enter(&body, (90.0, distance, 85.0)), SETTINGS);
    let pictures = common::client_screenshots(&r.dir, "MgScene", shots, Duration::from_millis(0))
        .expect("the client's pictures");
    for (i, (t, img)) in pictures.iter().enumerate() {
        save_png(img, &r.dir.join(format!("walk_{i:02}.png")));
        eprintln!("picture {i} at {t:.2} s");
    }
    print_log(&r.dir);
}

/// What the client draws of a DDS with a TXI's rippling procedure (the
/// M4 item's BioWare DXT1: `batch6_bioware_dxt1` drew TDoD's
/// `dd_ce3_txburneds.dds` and the stock `tic01_floor01.dds` whole on a
/// card, so the zombie's pallor is not its body's DDS; its glow,
/// `dd_ce3_txhotcoal`, is an additive DDS with `proceduretype arturo`):
/// north of the player, west to east, standing cards of
/// `dd_ce3_txhotcoal.dds` with its TXI (`hot`), the same DDS under
/// another name without one (`hotbare`), its TGA (the hak's) with the
/// TXI (`hottga`), the stock `tic01_floor01.dds` with the zombie's TXI
/// (`floorrip`) and without (`floor`, under another name), and the floor
/// as a TGA with the TXI (`floortga`). Beside it, Moonglow's area view.
///
/// Seen (2026-11-01, 89.8193.37): `hot` and `floorrip` flat light gray (89,
/// 82, 78 both), `hotbare` the coals' picture, `hottga` the coals rippling,
/// `floor` the stones, `floortga` the stones rippling, added (pale).
#[test]
#[ignore]
fn batch6_procedure_on_dds() {
    let Some(_) = super::batch4::tdod_haks().filter(|d| d.join("tdod_p_ce3.hak").is_file()) else {
        eprintln!("skipped: no tdod_p_ce3.hak");
        return;
    };
    let base = base_game();
    let hak = super::batch4::from_hak(
        "tdod_p_ce3.hak",
        &["dd_ce3_txhotcoal.dds", "dd_ce3_txhotcoal.txi", "dd_ce3_txhotcoal.tga"],
    );
    let (dds, txi, tga) = (hak[0].1.clone(), hak[1].1.clone(), hak[2].1.clone());
    let floor = base.resman.get_named("tic01_floor01", ResType::DDS).unwrap().to_vec();
    let floor_tga = mg_image::write_tga(&mg_image::read(ResType::DDS, &floor).unwrap().to_rgba());
    let files: Vec<(String, Vec<u8>)> = vec![
        ("mgpr_hot.dds".into(), dds.clone()),
        ("mgpr_hot.txi".into(), txi.clone()),
        ("mgpr_hotbare.dds".into(), dds),
        ("mgpr_hottga.tga".into(), tga),
        ("mgpr_hottga.txi".into(), txi.clone()),
        ("mgpr_floorrip.dds".into(), floor.clone()),
        ("mgpr_floorrip.txi".into(), txi.clone()),
        ("mgpr_floor.dds".into(), floor),
        ("mgpr_floortga.tga".into(), floor_tga),
        ("mgpr_floortga.txi".into(), txi),
    ];
    let cards = ["hot", "hotbare", "hottga", "floorrip", "floor", "floortga"];
    let nodes: Vec<String> = cards
        .iter()
        .enumerate()
        .map(|(i, c)| {
            card(c, "mg_proc", (-3.3 + 1.1 * i as f32, 0.05), (1.0, 1.0), &format!("mgpr_{c}"))
        })
        .collect();
    let (table, first) = placeables_with(&base, &[("mg_proc", 0)]);
    let mut files = files;
    files.push(("mg_proc.mdl".into(), super::batch4::probe_model("mg_proc", &nodes).into_bytes()));
    files.push(("placeables.2da".into(), table.into_bytes()));
    let light = Lighting { ambient: 0x606060, diffuse: 0xA0A0A0, main_light: 0, only_tile: None };
    let mut r = Reading::new("procedure_on_dds", "tic01", light, &as_overrides(&files));
    r.place(first, [20.0, 23.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
    let (dir, _) = super::batch4::shoot_close(r, INVISIBLE, SETTINGS, (90.0, 6.0, 70.0), &|v| {
        v.animate = true
    });
    eprintln!("{}", dir.display());
}

/// Why a cut-out disc of mesh `alpha 0.5` is a quarter lighter in the
/// client than the plain one (the deferred list's "Where a cut-out is
/// cut": 156, 99, 40 against 126, 79, 31 at its middle): the scene of
/// `batch4.rs`'s `reading_see_through_cut_out` (1.5 m cards of the
/// orange disc with TXI `blending punchthrough`, west to east: as it is,
/// the mesh's `alpha 0.5`, its `transparencyhint 1`), `fslit` patched to
/// paint one of [`CUT_PARTS`] a run (`MG_PARTS` picks), the client beside
/// Moonglow's area view with the Shaders switch on (`moonglow_on.png`,
/// the same patch). Each run prints the three discs' middles and the
/// floor between them.
///
/// Seen (2026-11-01, 89.8193.37): the discs' color before lighting alike (255,
/// 91, 11), their lighting 126, 156 and 126, the `alpha 0.5` disc's alpha 0.5
/// and black with its alpha black (no blending): the same in Moonglow's game
/// path to the pixel; its own shader lit the three alike (126).
#[test]
#[ignore]
fn batch6_cut_out_lighter() {
    let base = base_game();
    let fslit = program(&base, "fslit");
    let only = std::env::var("MG_PARTS").unwrap_or_default();
    let card = |name: &str, extra: &str| {
        format!(
            "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
             setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
             node trimesh card\n  parent {name}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
             ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap mg_cutd\n  \
             {extra}render 1\n  shadow 0\n  verts 4\n    -0.75 0 0.1\n    0.75 0 0.1\n    \
             0.75 0 1.6\n    -0.75 0 1.6\n  tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  \
             faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n\
             endmodelgeom {name}\ndonemodel {name}\n"
        )
    };
    for (part, paint) in CUT_PARTS {
        if !only.is_empty() && !only.split(',').any(|p| p == part) {
            continue;
        }
        let (table, first) =
            placeables_with(&base, &[("mg_cutp", 0), ("mg_cuta", 0), ("mg_cuth", 0)]);
        let mut files: Vec<(String, Vec<u8>)> = vec![
            ("mg_cutp.mdl".into(), card("mg_cutp", "").into_bytes()),
            ("mg_cuta.mdl".into(), card("mg_cuta", "alpha 0.5\n  ").into_bytes()),
            ("mg_cuth.mdl".into(), card("mg_cuth", "transparencyhint 1\n  ").into_bytes()),
            ("mg_cutd.tga".into(), disc_tga()),
            ("mg_cutd.txi".into(), b"blending punchthrough\n".to_vec()),
            ("placeables.2da".into(), table.into_bytes()),
        ];
        if let Some(paint) = paint {
            let find = "FragmentColor = vec4(1.0);\n \tSetupStandardShaderInputs();\n    \
                        ApplyStandardShader();\n    gl_FragColor = FragmentColor;";
            files.push(("fslit.shd".into(), patched(&fslit, find, paint).into_bytes()));
        }
        let light =
            Lighting { ambient: 0x808080, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
        let mut r =
            Reading::new(&format!("cut_out_lighter_{part}"), "tic01", light, &as_overrides(&files));
        for (i, x) in [18.0, 20.0, 22.0].into_iter().enumerate() {
            r.place(first + i as u32, [x, 23.0, 0.0], 0.0, &[("Static", Value::Byte(0))]);
        }
        let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
        let names = ["client", "moonglow_on", "moonglow"];
        for (what, x) in [("plain", 0.33), ("alpha 0.5", 0.5), ("hint", 0.67)] {
            region_means(
                &dir,
                &format!("{part}: {what}"),
                [x - 0.01, 0.36, x + 0.01, 0.40],
                &names,
            );
        }
        region_means(&dir, &format!("{part}: floor"), [0.40, 0.60, 0.44, 0.64], &names);
    }
}
