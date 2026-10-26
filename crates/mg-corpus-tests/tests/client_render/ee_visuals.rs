//! The Visuals page and the area's Shader Flags in the game client: each
//! field written into the module, as Moonglow saves it, with no script
//! setting it, and the client's picture showing it took effect.
//!
//! `engine_ee_fields.rs` shows the server reads the same fields back; this
//! shows the client draws them. One scene a case: the armoire of the
//! reference scenes, placed in the area's GIT instead of created by
//! script, with one field set on it.
//!
//! - texture replaced: each of the armoire's textures drawn as
//!   `mg_magenta`, a texture of one color written to the scratch override;
//! - shader parameters: a patched `inc_standard` paints a mesh cyan when
//!   its `mgParam` uniform is 7 and its `mgVec` is (1, 2, 3, 4), which
//!   only the object's `Material` sets (a case with 6 stays uncolored).
//!   The object is a card whose MTR names its own shader pair: the game
//!   passes parameters to no other material (a stock mesh, or an MTR
//!   without `customshadervs`/`customshaderfs`, stays at 0, even with the
//!   MTR's own `parameter int mgParam 7`). Set by the script calls
//!   instead, the same.
//! - visible distance 1 m: the armoire isn't drawn from the camera;
//! - animation replaced: a chest playing its open animation for its idle
//!   one;
//! - shader flags: the same `inc_standard` writes the `areaFlags` uniform's
//!   bytes into the picture, and the area's `Flags` of 0x8109 read back.
//!
//! Needs the game, a GPU and the off-screen display; run by hand:
//! `DISPLAY=:1 cargo test -p mg-corpus-tests --test client_render ee_visuals -- --ignored --nocapture`.

use super::*;
use mg_gff::Struct;
use mg_module::instances::{Placement, Placing, instance};

/// Painting added to the game's `inc_standard.shd`: the shader parameters'
/// test color, and `areaFlags`' four bytes in blocks 0-3 of row 6 (as
/// [`debug_value`] reads them).
const EE_FUNCTIONS: &str = r#"
uniform int mgParam;
uniform vec4 mgVec;
void MgEe(inout vec4 c)
{
    if (mgParam == 7 && mgVec == vec4(1.0, 2.0, 3.0, 4.0)) { c = vec4(0.0, 1.0, 1.0, 1.0); return; }
    int row = int(gl_FragCoord.y / 32.0) - 6;
    int k = int(gl_FragCoord.x / 32.0);
    if (row != 0 || k >= 4) return;
    float x = (float((areaFlags >> (8 * k)) & 255) + 0.5) / 256.0;
    c = vec4(floor(x * 255.0) / 255.0, fract(x * 255.0), 0.0, 1.0);
}
"#;

fn ee_shader(game: &GameData) -> String {
    let src = game.resman.get_named("inc_standard", ResType::SHD).expect("inc_standard.shd");
    let mut src = String::from_utf8_lossy(&src).replace("\r\n", "\n");
    let at = src.rfind("void ApplyStandardShader()").expect("ApplyStandardShader");
    src.insert_str(at, &format!("#if SHADER_TYPE == 2\n{EE_FUNCTIONS}\n#endif\n"));
    let end = "\tApplyDebugModeOutput(FragmentColor);\n}";
    let at = src.rfind(end).expect("ApplyDebugModeOutput");
    src.insert_str(at + end.len() - 1, "#if SHADER_TYPE == 2\n\tMgEe(FragmentColor);\n#endif\n");
    src
}

fn resref(s: &str) -> Value {
    Value::resref(ResRef::from_str(s).unwrap())
}

fn text(s: &str) -> Value {
    Value::String(s.as_bytes().to_vec())
}

/// A model's textures (the slot-0 bitmaps of its meshes).
fn textures(model: &Model) -> Vec<String> {
    let mut out: Vec<String> = model
        .nodes
        .iter()
        .filter_map(|n| n.mesh()?.textures[0].clone())
        .filter(|t| t != "null")
        .collect();
    out.sort();
    out.dedup();
    out
}

/// One field set on the placed object, as the Visuals page writes it.
enum Case {
    Plain,
    Texture(Vec<String>),
    Param {
        materials: Vec<String>,
        int: i32,
    },
    /// The same parameters set by the script calls, for comparison.
    ParamScript(Vec<String>),
    Distance(f32),
    Animation(String, String),
    AreaFlags(u32),
}

fn apply(case: &Case, p: &mut Struct) {
    match case {
        Case::Plain | Case::AreaFlags(_) | Case::ParamScript(_) => {}
        Case::Texture(names) => {
            let list = names
                .iter()
                .map(|old| {
                    let mut e = Struct::new(0);
                    e.set("OldTexture", resref(old));
                    e.set("NewTexture", resref("mg_magenta"));
                    e
                })
                .collect();
            let mut t = Struct::new(9);
            t.set("TextureReplaceLi", Value::List(list));
            p.set("TextureReplace", Value::Struct(t));
        }
        Case::Param { materials, int } => {
            let mut params = Vec::new();
            for material in materials {
                let mut i = Struct::new(0);
                i.set("Material", text(material));
                i.set("Param", text("mgParam"));
                i.set("Type", Value::Byte(1));
                i.set("Int", Value::Int(*int));
                let mut v = Struct::new(0);
                v.set("Material", text(material));
                v.set("Param", text("mgVec"));
                v.set("Type", Value::Byte(2));
                for (k, f) in [1.0, 2.0, 3.0, 4.0].into_iter().enumerate() {
                    v.set(&format!("Float{}", k + 1), Value::Float(f));
                }
                params.extend([i, v]);
            }
            let mut m = Struct::new(7);
            m.set("ShaderParams", Value::List(params));
            p.set("Material", Value::Struct(m));
        }
        Case::Distance(d) => {
            let mut misc = Struct::new(8);
            misc.set("VisibleDistance", Value::Float(*d));
            p.set("MiscVisuals", Value::Struct(misc));
        }
        Case::Animation(old, new) => {
            let mut e = Struct::new(0);
            e.set("OldAnimation", resref(old));
            e.set("NewAnimation", resref(new));
            let mut a = Struct::new(11);
            a.set("AnimationReplace", Value::List(vec![e]));
            p.set("AnimationReplace", Value::Struct(a));
        }
    }
}

/// The client's picture of the scene with `blueprint` placed where the
/// reference scenes' armoire stands, `case` set on it.
fn shot(game: &GameData, shader: &str, name: &str, blueprint: &str, case: &Case) -> Rgba {
    let dir = scratch_dir(&format!("client_ee_visuals_{name}"));
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let (mut m, mut are) = build_module(game, &dir, light);
    let mut enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    if let Case::ParamScript(materials) = case {
        let mut calls = String::from(
            "    object o = GetFirstObjectInArea(GetAreaFromLocation(start));\n    \
             while (GetIsObjectValid(o) && GetObjectType(o) != OBJECT_TYPE_PLACEABLE) \
             o = GetNextObjectInArea(GetAreaFromLocation(start));\n",
        );
        for t in materials {
            calls += &format!(
                "    SetMaterialShaderUniformInt(o, \"{t}\", \"mgParam\", 7);\n    \
                 SetMaterialShaderUniformVec4(o, \"{t}\", \"mgVec\", 1.0, 2.0, 3.0, 4.0);\n"
            );
        }
        let at = enter.find("    ApplyEffectToObject").unwrap();
        enter.insert_str(at, &calls);
    }
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&dir, "mg_enter", &enter));
    if let Case::AreaFlags(flags) = case {
        are.root.set("Flags", Value::Dword(*flags));
        let area =
            m.info().unwrap().root.list("Mod_Area_list").unwrap()[0].resref("Area_Name").unwrap();
        m.set_gff(ResKey::new(area, ResType::ARE), &are).unwrap();
    }
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let bp = Gff::read(&game.resman.get_named(blueprint, ResType::UTP).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    // Where ENTER put the armoire: AHEAD north of the start, facing south.
    // (The card faces south at bearing 0.)
    let card = matches!(case, Case::Param { .. } | Case::ParamScript(_));
    let rotation = if card { 0.0 } else { -FRAC_PI_2 };
    let at = Placement { position: [20.0, 20.0 + AHEAD, 0.0], rotation };
    let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    apply(case, &mut p);
    git.root.set("Placeable List", Value::List(vec![p]));
    m.set_gff(git_key, &git).unwrap();
    std::fs::create_dir_all(dir.join("user/override")).unwrap();
    std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
    std::fs::write(dir.join("user/override/inc_standard.shd"), shader).unwrap();
    let magenta = Rgba { width: 4, height: 4, data: [255, 0, 255, 255].repeat(16) };
    std::fs::write(dir.join("user/override/mg_magenta.tga"), mg_image::write_tga(&magenta))
        .unwrap();
    if let Case::Param { .. } | Case::ParamScript(_) = case {
        // The armoire's model (placeables.2da row 0) replaced by a card
        // whose mesh draws with the MTR `mgee`. The game hands an MTR's
        // parameters only to a material with its own shader pair: the
        // stock lit pair, copied under new names, which include the
        // patched `inc_standard`.
        let over = dir.join("user/override");
        let card = super::uv_sets::uv_card_model("plc_a01", "mgee", 1);
        std::fs::write(over.join("plc_a01.mdl"), card).unwrap();
        for (stock, ours) in [("vslit", "mgeevs"), ("fslit", "mgeefs")] {
            let src = game.resman.get_named(stock, ResType::SHD).unwrap();
            std::fs::write(over.join(format!("{ours}.shd")), src).unwrap();
        }
        let mtr = "texture0 plc_wood256\ncustomshadervs mgeevs\ncustomshaderfs mgeefs\n\
                   parameter int mgParam 0\nparameter float mgVec 0 0 0 0\n";
        std::fs::write(over.join("mgee.mtr"), mtr).unwrap();
    }
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    // (The client now and then crashes while it starts, before it loads the
    // module: once more then.)
    let img = client_screenshot(&dir, "MgScene")
        .or_else(|| {
            eprintln!("{name}: the client did not reach the scene; once more");
            client_screenshot(&dir, "MgScene")
        })
        .unwrap_or_else(|| {
            panic!("{name}: the client did not reach the scene ({})", dir.display())
        });
    save_png(&img, &dir.join("shot.png"));
    img
}

const FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2;
/// The armoire's region in the reference scenes' camera.
const OBJECT: [f32; 4] = [0.492, 0.37, 0.508, 0.46];

/// The share of a rectangle's pixels that differ by more than 30 in a
/// channel.
fn changed(a: &Rgba, b: &Rgba, r: [f32; 4]) -> f32 {
    let (w, h) = (a.width as f32, a.height as f32);
    let (x0, x1) = ((r[0] * w) as usize, (r[2] * w) as usize);
    let (y0, y1) = ((r[1] * h) as usize, (r[3] * h) as usize);
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = (y * a.width as usize + x) * 4;
            n += (0..3).any(|c| a.data[i + c].abs_diff(b.data[i + c]) > 30) as usize;
        }
    }
    n as f32 / ((x1 - x0) * (y1 - y0)) as f32
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).abs()).fold(0.0, f32::max)
}

#[test]
#[ignore]
fn ee_visuals_in_the_client() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let rm = &game.resman;
    let shader = ee_shader(&game);
    let load = |name: &str| Model::read(&rm.get_named(name, ResType::MDL).unwrap()).unwrap();
    let armoire = textures(&load("plc_a01"));
    eprintln!("armoire textures: {armoire:?}");
    let chest = std::env::var("MG_CHEST_MODEL").unwrap_or("plc_c03".into());
    let names: Vec<String> = load(&chest).animations.iter().map(|a| a.name.clone()).collect();
    eprintln!("{chest} animations: {names:?}");
    let (old, new) = match std::env::var("MG_CHEST_ANIM") {
        Ok(v) => {
            let (o, n) = v.split_once(':').unwrap();
            (o.to_string(), n.to_string())
        }
        Err(_) => ("default".into(), "open".into()),
    };
    let chest_bp = std::env::var("MG_CHEST").unwrap_or("plc_chest1".into());
    let mut failures = Vec::new();
    let mut check = |ok: bool, what: String| {
        eprintln!("{} {what}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            failures.push(what);
        }
    };

    let plain = shot(&game, &shader, "plain", "plc_armoire", &Case::Plain);
    let base = mean(&plain, OBJECT);
    let plain_flags = debug_int(&plain, 0);
    eprintln!("plain: armoire {base:?}, areaFlags {plain_flags:#x}");

    let img = shot(&game, &shader, "texture", "plc_armoire", &Case::Texture(armoire.clone()));
    let c = mean(&img, OBJECT);
    check(
        c[0] > 2.0 * c[1] + 20.0 && c[2] > 2.0 * c[1] + 20.0,
        format!("texture replaced: armoire {c:?} is magenta (plain {base:?})"),
    );

    let materials = vec!["mgee".to_string()];
    let img = shot(
        &game,
        &shader,
        "param",
        "plc_armoire",
        &Case::Param { materials: materials.clone(), int: 7 },
    );
    let c = mean(&img, OBJECT);
    check(
        c[1] > 2.0 * c[0] + 40.0 && c[2] > 2.0 * c[0] + 40.0,
        format!("shader parameters on {materials:?}: armoire {c:?} is cyan"),
    );
    let img =
        shot(&game, &shader, "param_script", "plc_armoire", &Case::ParamScript(materials.clone()));
    let s = mean(&img, OBJECT);
    check(distance(c, s) < 6.0, format!("set by script instead: {s:?}, as the saved field's"));
    let img = shot(&game, &shader, "param_6", "plc_armoire", &Case::Param { materials, int: 6 });
    let six = mean(&img, OBJECT);
    check(distance(six, c) > 60.0, format!("mgParam 6: card {six:?}, not cyan"));

    let img = shot(&game, &shader, "distance", "plc_armoire", &Case::Distance(1.0));
    let c = mean(&img, OBJECT);
    check(distance(c, base) > 20.0, format!("visible distance 1 m: {c:?}, plain {base:?}"));
    let far = mean(&img, [0.40, 0.25, 0.60, 0.30]);
    eprintln!("  (the wall behind: {far:?})");

    let closed = shot(&game, &shader, "chest", &chest_bp, &Case::Plain);
    let img =
        shot(&game, &shader, "chest_anim", &chest_bp, &Case::Animation(old.clone(), new.clone()));
    let changed = changed(&closed, &img, [0.47, 0.37, 0.53, 0.47]);
    check(
        changed > 0.2,
        format!("{chest_bp}: {old} played as {new}: {:.0}% of the chest changed", changed * 100.0),
    );

    let img = shot(&game, &shader, "flags", "plc_armoire", &Case::AreaFlags(0x8109));
    let flags = debug_int(&img, 0);
    check(flags == 0x8109, format!("shader flags: areaFlags {flags:#x}, the area's 0x8109"));

    assert!(failures.is_empty(), "{failures:#?}");
}
