//! The script uniforms (`SetShaderUniformFloat`, `SetShaderUniformInt`
//! and `SetShaderUniformVec`) in the client beside Moonglow's area view
//! given the same values (`mg_area::View::script_uniforms`).
//!
//! The scene: cards north of the player (each the model of a placeable
//! appearance, replaced in the scratch override, as `uv_sets` places
//! them), each drawn by one custom pair of Moonglow's ([`SU_VS`],
//! [`SU_FS`], which include the game's own `inc_scriptable`), which paints
//! the card unlit in a color [`SU_CARDS`] says, picked by the MTR's
//! `parameter int nCard`. The module's OnClientEnter script sets the
//! uniforms a second after the player enters ([`SU_SET`]). A control card
//! paints a fixed color (what the client's picture does to a color the
//! fragment program writes); one card reads uniforms no script set (the
//! game's default: zero gives gray); one reads `scriptableVec3` in the
//! vertex program. The pictures go to
//! `target/test-output/client_script_uniforms/`; each card's mean color
//! is printed, expected beside the client's and Moonglow's.

use super::*;
use crate::uv_sets::{card_at, card_rect, uv_card_model};

/// The vertex program: the stock lit one, `scriptableVec3` handed on.
const SU_VS: &str = "\
// vsmgsu: Moonglow's test of the script uniforms.
#define SHADER_TYPE 1
#define FOG 1
#define KEYHOLING 1
#define LIGHTING 1
#include \"inc_standard\"
#include \"inc_scriptable\"

varying vec4 vMgScript;

void main()
{
    ApplyStandardShader();
    vMgScript = scriptableVec3;
}
";

/// The fragment program: the card's color by `nCard` ([`SU_CARDS`]).
const SU_FS: &str = "\
// fsmgsu: Moonglow's test of the script uniforms.
#define SHADER_TYPE 2
#define FOG 1
#define KEYHOLING 0
#define LIGHTING 1
#include \"inc_standard\"
#include \"inc_scriptable\"

uniform int nCard;
varying vec4 vMgScript;

void main()
{
    vec3 c = vec3(0.2, 0.4, 0.6);
    if (nCard == 1) c = vec3(scriptableFloat1, scriptableFloat2, 0.0);
    else if (nCard == 2) c = vec3(float(scriptableInt1), float(scriptableInt2), 0.0) / 255.0;
    else if (nCard == 3) c = scriptableVec1.rgb;
    else if (nCard == 4) c = scriptableVec16.rgb;
    else if (nCard == 5) c = vMgScript.rgb;
    else if (nCard == 6)
        c = vec3(0.5) + 0.5 * vec3(scriptableFloat5, float(scriptableInt5), scriptableVec5.x);
    gl_FragColor = vec4(c, 1.0);
}
";

/// The cards: what each shows and the color expected (0 to 255).
const SU_CARDS: [(&str, [f32; 3]); 7] = [
    ("control (fixed)", [51.0, 102.0, 153.0]),
    ("floats 1 and 2", [51.0, 204.0, 0.0]),
    ("ints 1 and 2", [200.0, 40.0, 0.0]),
    ("vec 1", [255.0, 0.0, 255.0]),
    ("vec 16 (index 15)", [0.0, 153.0, 255.0]),
    ("vec 3 (vertex)", [255.0, 255.0, 0.0]),
    ("unset (zero: gray)", [127.5, 127.5, 127.5]),
];

/// The script's lines, run a second after the player enters (`pc`).
const SU_SET: &str = "\
    DelayCommand(1.0, SetShaderUniformFloat(pc, SHADER_UNIFORM_1, 0.2));
    DelayCommand(1.0, SetShaderUniformFloat(pc, SHADER_UNIFORM_2, 0.8));
    DelayCommand(1.0, SetShaderUniformInt(pc, SHADER_UNIFORM_1, 200));
    DelayCommand(1.0, SetShaderUniformInt(pc, SHADER_UNIFORM_2, 40));
    DelayCommand(1.0, SetShaderUniformVec(pc, SHADER_UNIFORM_1, 1.0, 0.0, 1.0, 1.0));
    DelayCommand(1.0, SetShaderUniformVec(pc, SHADER_UNIFORM_16, 0.0, 0.6, 1.0, 1.0));
    DelayCommand(1.0, SetShaderUniformVec(pc, SHADER_UNIFORM_3, 1.0, 1.0, 0.0, 1.0));
";

/// What [`SU_SET`] sets, as Moonglow takes it.
fn su_values() -> mg_render::ScriptUniforms {
    let mut u = mg_render::ScriptUniforms::default();
    u.set_float(0, 0.2);
    u.set_float(1, 0.8);
    u.set_int(0, 200);
    u.set_int(1, 40);
    u.set_vec(0, [1.0, 0.0, 1.0, 1.0]);
    u.set_vec(15, [0.0, 0.6, 1.0, 1.0]);
    u.set_vec(2, [1.0, 1.0, 0.0, 1.0]);
    u
}

/// The script uniforms in the client and in Moonglow: each card's color
/// the expected one, in both, within `MG_SU_TOLERANCE` (default 6 of
/// 255). `MG_SU_NO_CLIENT` draws Moonglow's picture alone.
#[test]
#[ignore]
fn script_uniforms_in_the_client() {
    use mg_module::instances::{Placement, Placing, instance};
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let tileset = "tni01";
    let dir = scratch_dir("client_script_uniforms");
    let over = dir.join("user/override");
    std::fs::create_dir_all(&over).unwrap();
    std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
    std::fs::write(over.join("vsmgsu.shd"), SU_VS).unwrap();
    std::fs::write(over.join("fsmgsu.shd"), SU_FS).unwrap();
    std::fs::write(over.join("mgsuwhite.tga"), white_tga()).unwrap();
    let table = base.table("placeables").unwrap();
    for i in 0..SU_CARDS.len() {
        let model = table.get(i, "ModelName").unwrap().to_ascii_lowercase();
        let material = format!("mgsu{i}");
        std::fs::write(over.join(format!("{model}.mdl")), uv_card_model(&model, &material, 1))
            .unwrap();
        let mtr = format!(
            "texture0 mgsuwhite\ncustomshadervs vsmgsu\ncustomshaderfs fsmgsu\n\
             parameter int nCard {i}\n"
        );
        std::fs::write(over.join(format!("{material}.mtr")), mtr).unwrap();
    }
    let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
    let light = Lighting { ambient: 0x505050, diffuse: 0x909090, main_light: 2, only_tile: None };
    let (mut m, are) = build_module_on(&game, &dir, light, tileset, 0);
    let ready = "    DelayCommand(4.0, Log(\"MG_READY\"));\n";
    assert!(ENTER.contains(ready));
    let enter: String = ENTER
        .replace(ready, &format!("{SU_SET}{ready}"))
        .lines()
        .filter(|l| !l.contains("CreateObject"))
        .collect::<Vec<_>>()
        .join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let placed: Vec<_> = (0..SU_CARDS.len())
        .map(|i| {
            let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Card", 0);
            bp.root.set("Appearance", Value::Dword(i as u32));
            bp.root.set("Static", Value::Byte(0));
            let (x, y) = card_at(i);
            let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
            instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap()
        })
        .collect();
    git.root.set("Placeable List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    let client = if std::env::var_os("MG_SU_NO_CLIENT").is_some() {
        None
    } else {
        let c = client_screenshot(&dir, "MgScene");
        assert!(c.is_some(), "the client did not reach the scene (see {})", dir.display());
        c
    };
    if let Some(c) = &client {
        save_png(c, &dir.join("client.png"));
    }

    let set = mg_area::tileset(&game, ResRef::from_str(tileset).unwrap()).ok();
    crate::common::gpu_after_display();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let model = mg_area::AreaModel::read(&game, &are.root, &git.root, set.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let view = mg_area::View {
        fog: false,
        script_uniforms: Some(su_values()),
        ..mg_area::View::of(&model)
    };
    let scene = area_scene.scene(&model, &view);
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let size = client.as_ref().map_or((1280, 800), |c| (c.width, c.height));
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    r.game_shaders = true;
    let ours = r.render_image(&gpu, &game.resman, &scene, &camera, size.0, size.1);
    assert_eq!(r.game_shader_failures(), &[]);
    save_png(&ours, &dir.join("moonglow.png"));
    let tolerance: f32 =
        std::env::var("MG_SU_TOLERANCE").ok().and_then(|t| t.parse().ok()).unwrap_or(6.0);
    let mut off = Vec::new();
    for (i, (name, want)) in SU_CARDS.iter().enumerate() {
        // (The card's upper half: the tiles' candles stand before some
        // cards' feet.)
        let [x0, y0, x1, y1] = card_rect(&camera, size, i);
        let rect = [x0, y0, x1, (y0 + y1) / 2.0];
        let mut line = format!("{name:20} expected {want:5.1?}");
        let mut pictures = vec![("moonglow", &ours)];
        if let Some(c) = &client {
            pictures.insert(0, ("client", c));
        }
        for (who, img) in pictures {
            let (got, _) = mean_and_saturation(img, rect);
            line += &format!(" | {who} ({:5.1}, {:5.1}, {:5.1})", got[0], got[1], got[2]);
            if (0..3).any(|k| (got[k] - want[k]).abs() > tolerance) {
                off.push(format!("{who}: {name} {got:.1?}, not {want:.1?}"));
            }
        }
        eprintln!("{line}");
    }
    eprintln!("{}: client {}", dir.display(), client.is_some());
    assert!(off.is_empty(), "{off:#?}");
}
