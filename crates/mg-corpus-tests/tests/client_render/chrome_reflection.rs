//! How a surface that reflects wholly shows the environment map
//! `chrome1`, in the client beside Moonglow's area view with the Shaders
//! switch off and on (the deferred entry "A mesh whose texture is nowhere
//! is drawn darker than the client draws it": Moonglow draws the tops of
//! such a mesh black where the client draws them gray).
//!
//! The scene: four boxes a metre across (authored models in the scratch
//! override, put over two placeables whose `Reflection` is `default`, which
//! the client binds `chrome1`, rows 428 and 447; not static), of a white
//! TGA whose alpha is 0 (reflecting wholly), north of the player: two with
//! a face of their own to each of their six sides (flat normals), turned 0
//! and 45 degrees, and two of eight shared corners in one smoothing group
//! (the normals of a corner shared). Seen from three cameras: the client's
//! own as the player enters (looking north, 19 m, 50 degrees from straight
//! down), `SetCameraFacing(180, 12, 45)` (looking west) and one from above
//! (`SetCameraFacing(90, 8, 15)`: Moonglow's eye for it is unfitted, the
//! ratio of [`Camera::GAME_SCRIPTED_12_45`] to 12 and 45, so each face is
//! sampled well inside), each under the area's sun (diffuse white, ambient
//! gray), without (ambient gray, no diffuse) and in the dark (ambient
//! near 0).
//!
//! Seen (2026-11-25, EE 89.8193.37, Shaders off and on drew alike): on
//! placeables whose `Reflection` is `default` the client and Moonglow
//! agree within a few of 255 on every face under the sun and without it
//! (the top of a flat box 132.7 / 132.2, a side 17.7 / 17.4), the paint
//! probes agreeing too (the map's coordinates, the normal, `envMapCube` 0,
//! `texEnvBound` 1) and a gradient map shows the same texel in both
//! (`chrome1-gradient`): `chrome1`'s upload, orientation, sampler and
//! coordinates are not the difference. The difference is on a worn
//! helmet (`chrome1_helmet_reflection_look`): the client samples the
//! area's map (`tic01__ref01`, warm gray: its top 131.9 126.9 122.3, a
//! raw sample 180 169 160) where Moonglow binds `chrome1` (blue gray:
//! 130.7 139.6 146.8, a raw sample 178 195 211) at the same coordinates
//! and normals; with `tic01__ref01.tga` replaced by the gradient
//! (`ref01-gradient`) the client's helmet shows the gradient and the
//! boxes do not. `mg_preview::creature` gave a worn helmet `chrome1`
//! (`p.env_map`); the client reflects the creature's `default`, the
//! area's, on a held shield (`MG_CHROME_HELD=shield`) and a held long
//! sword (`weapon`) too, as it now does in Moonglow (`default` in
//! `creature.rs` and `item.rs`; after the fix the gradient reading agrees
//! on the sides to 1 or 2 of 255). (Besides: in the dark, ambient 0x080808, the client's boxes
//! are 4 to 8 times brighter than Moonglow's, 23.7 against 4.0 on a top;
//! `frontLightModelProductSceneColor` and `lightAreaAmbient` are the same
//! in both, so what lights them there is not told; smoothed normals of
//! shared corners differ a little, 22 against 35 in the red of one
//! top's normal; and a placeable's own TXI `envmaptexture mgenv` is
//! ignored by the client where its `Reflection` is `default`, which
//! Moonglow now does too.)
//!
//! [`VIEWS`] and [`LIGHTS`] are the cameras and lights; `MG_CHROME_VIEWS`
//! and `MG_CHROME_LIGHTS` (comma-separated names) take some. Each face's
//! mean color (an inset window of it) in the client's picture and in
//! Moonglow's two is printed, and written to `chrome_<variant>.txt` beside
//! the scratch folders; the pictures go to
//! `target/test-output/client_reading_chrome_<variant>_<view>_<light>/`
//! (`client.png`, `moonglow.png`, `moonglow_on.png`).
//!
//! `MG_CHROME_VARIANT` says what the boxes reflect (the environment map's
//! way in):
//!
//! - `chrome1` (the default): the object's own `default`, as a placeable's;
//! - `txi-chrome1`: the texture's TXI naming `chrome1` (a control of the
//!   TXI route, which the variants below take);
//! - `gradient`: the TXI naming a gradient map of our own (`mgenv`, 256 by
//!   256: red the column, green the row of the file, top row first), which
//!   shows which texel each pixel samples;
//! - `chrome1-gradient`: `chrome1.tga` (a 64 by 64 beside the game's
//!   256 by 256 DDS) replaced by the gradient in the override;
//! - `ref01-gradient`: the same for `tic01__ref01.tga`, the area's map.
//!
//! `MG_CHROME_PAINT` paints every lit fragment that reflects (the client
//! and Moonglow's game path alike, through a patched `inc_standard`)
//! with: `uv` (the map's coordinates), `normal` (the view-space normal,
//! halved and raised), `flags` (`envMapCube`, `texEnvBound`,
//! `texEnvCubeBound`), `raw` (the map sampled at those coordinates, no bias)
//! or `biased` (the same, at a bias of 4).

use super::readings::{Reading, enter, logged, print_log};
use super::*;

/// A camera: its name, the script's (facing, distance, pitch), and whether
/// Moonglow's eye for it is fitted ([`Camera::GAME_ENTRY`] and
/// [`Camera::GAME_SCRIPTED_12_45`]).
struct View {
    name: &'static str,
    script: (f32, f32, f32),
    fitted: bool,
}

const VIEWS: [View; 3] = [
    View { name: "entry", script: (90.0, 12.0, 45.0), fitted: true },
    View { name: "west", script: (180.0, 12.0, 45.0), fitted: true },
    View { name: "above", script: (90.0, 8.0, 15.0), fitted: false },
];

/// A light: its name, the area's ambient and diffuse colors.
struct Light {
    name: &'static str,
    ambient: u32,
    diffuse: u32,
}

const LIGHTS: [Light; 3] = [
    Light { name: "sun", ambient: 0x606060, diffuse: 0xFFFFFF },
    Light { name: "nosun", ambient: 0x606060, diffuse: 0x000000 },
    Light { name: "dark", ambient: 0x080808, diffuse: 0x000000 },
];

/// A box: its placeable's row, whether its corners are shared, the turn
/// (degrees) and where it stands (metres right of the player's line).
struct Cube {
    row: u32,
    shared: bool,
    turn: f32,
    lateral: f32,
}

const BOXES: [Cube; 4] = [
    Cube { row: 428, shared: false, turn: 0.0, lateral: -3.9 },
    Cube { row: 428, shared: false, turn: 45.0, lateral: -1.3 },
    Cube { row: 447, shared: true, turn: 0.0, lateral: 1.3 },
    Cube { row: 447, shared: true, turn: 45.0, lateral: 3.9 },
];

/// How far ahead of the player the boxes stand, metres.
const FORWARD: f32 = 2.5;

/// The box's ASCII model: `size` metres a side on the floor, its texture `bitmap`
/// whole on each face. `shared`: eight corners in one smoothing group;
/// else a corner of its own to each face (and a smoothing group each).
fn box_model(name: &str, bitmap: &str, shared: bool, double: bool, size: f32) -> String {
    let (h, top) = (size / 2.0, size);
    // (Each face counterclockwise seen from outside.)
    let faces: [[[f32; 3]; 4]; 6] = [
        [[-h, -h, top], [h, -h, top], [h, h, top], [-h, h, top]],
        [[-h, h, 0.0], [h, h, 0.0], [h, -h, 0.0], [-h, -h, 0.0]],
        [[h, -h, 0.0], [h, h, 0.0], [h, h, top], [h, -h, top]],
        [[-h, h, 0.0], [-h, -h, 0.0], [-h, -h, top], [-h, h, top]],
        [[h, h, 0.0], [-h, h, 0.0], [-h, h, top], [h, h, top]],
        [[-h, -h, 0.0], [h, -h, 0.0], [h, -h, top], [-h, -h, top]],
    ];
    let (mut verts, mut triangles) = (String::new(), String::new());
    let mut shared_corners: Vec<[f32; 3]> = Vec::new();
    let mut count = 0;
    for (f, face) in faces.iter().enumerate() {
        let index: Vec<usize> = face
            .iter()
            .map(|c| {
                if shared {
                    shared_corners.iter().position(|s| s == c).unwrap_or_else(|| {
                        shared_corners.push(*c);
                        shared_corners.len() - 1
                    })
                } else {
                    verts += &format!("    {} {} {}\n", c[0], c[1], c[2]);
                    count += 1;
                    count - 1
                }
            })
            .collect();
        let group = if shared { 1 } else { f + 1 };
        for t in [[0, 1, 2], [0, 2, 3]] {
            triangles += &format!(
                "    {} {} {} {group} {} {} {} 0\n",
                index[t[0]], index[t[1]], index[t[2]], t[0], t[1], t[2]
            );
            if double {
                // (Each also the other way round, as the part textures' boxes.)
                triangles += &format!(
                    "    {} {} {} {group} {} {} {} 0\n",
                    index[t[0]], index[t[2]], index[t[1]], t[0], t[2], t[1]
                );
            }
        }
    }
    if shared {
        for c in &shared_corners {
            verts += &format!("    {} {} {}\n", c[0], c[1], c[2]);
        }
        count = shared_corners.len();
    }
    format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
         node trimesh box\n  parent {name}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n  \
         render 1\n  shadow 0\n  verts {count}\n{verts}  tverts 4\n    0 0 0\n    1 0 0\n    \
         1 1 0\n    0 1 0\n  faces {}\n{triangles}endnode\nendmodelgeom {name}\ndonemodel {name}\n",
        triangles.lines().count()
    )
}

/// A 32-bit TGA of `size` squared, the file's first row the top, from
/// `pixel(column, row)` (red, green, blue, alpha).
fn tga32(size: u16, pixel: impl Fn(u16, u16) -> [u8; 4]) -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    t.extend(size.to_le_bytes());
    t.extend(size.to_le_bytes());
    t.extend([32, 0x28]);
    for y in 0..size {
        for x in 0..size {
            let [r, g, b, a] = pixel(x, y);
            t.extend([b, g, r, a]);
        }
    }
    t
}

/// The gradient map: red the column, green the row (the file's first row
/// 0), blue a ring of radius a third of the picture.
fn gradient() -> Vec<u8> {
    tga32(256, |x, y| {
        let d = ((f32::from(x) - 128.0).powi(2) + (f32::from(y) - 128.0).powi(2)).sqrt();
        [x as u8, y as u8, if (d - 85.0).abs() < 2.0 { 255 } else { 0 }, 255]
    })
}

/// `inc_standard` painting what `what` says over every lit fragment that
/// reflects (the map's coordinates as the shader makes them from the
/// fragment's normal).
fn painting(game: &GameData, what: &str) -> String {
    let at = "CalculateEnvironmentTexCoords(vFragmentNormal)";
    let paint = match what {
        "uv" => format!("vec4(fract({at}), 0.0, 1.0)"),
        "normal" => "vec4(vFragmentNormal * 0.5 + 0.5, 1.0)".to_string(),
        "flags" => {
            "vec4(float(envMapCube), float(texEnvBound), float(texEnvCubeBound), 1.0)".to_string()
        }
        "raw" => format!("vec4(texture2D(texUnitEnv, {at}).rgb, 1.0)"),
        "biased" => format!("vec4(texture2D(texUnitEnv, {at}, 4.0).rgb, 1.0)"),
        "scenecolor" => "vec4(frontLightModelProductSceneColor.rgb, 1.0)".to_string(),
        "ambient" => "vec4(lightAreaAmbient, 1.0)".to_string(),
        other => {
            panic!("MG_CHROME_PAINT {other}: uv, normal, flags, raw, biased, scenecolor or ambient")
        }
    };
    let src = game.resman.get_named("inc_standard", ResType::SHD).unwrap();
    String::from_utf8_lossy(&src).replace("\r\n", "\n").replace(
        "\tApplyDebugModeOutput(FragmentColor);\n}",
        &format!(
            "\tApplyDebugModeOutput(FragmentColor);\n\
             #if SHADER_TYPE == 2 && ENVIRONMENT_MAP == 1 && LIGHTING == 1\n\
             \tFragmentColor = {paint};\n#endif\n}}"
        ),
    )
}

/// One visible face of box `b` for a camera looking along `f` (a unit
/// vector, with `r` to its right): its label, the center of a window of it
/// and the window's corners (world), the window 0.4 m across.
fn faces(b: &Cube, pc: Vec3, f: Vec3, r: Vec3) -> Vec<(&'static str, [Vec3; 4])> {
    let center = pc + f * FORWARD + r * b.lateral;
    let mut out = Vec::new();
    let window = |c: Vec3, a: Vec3, b: Vec3| {
        [c - a * 0.2 - b * 0.2, c + a * 0.2 - b * 0.2, c + a * 0.2 + b * 0.2, c - a * 0.2 + b * 0.2]
    };
    out.push(("top", window(center + Vec3::Z, r, f)));
    for k in 0..4 {
        let phi = (b.turn + 90.0 * k as f32).to_radians();
        let n = r * phi.cos() + f * phi.sin();
        if n.dot(f) > -0.2 {
            continue;
        }
        let label = match (n.dot(r) * 10.0).round() as i32 {
            0 => "front",
            x if x < 0 => "frontL",
            _ => "frontR",
        };
        let tangent = n.cross(Vec3::Z);
        out.push((label, window(center + n * 0.5 + Vec3::Z * 0.5, Vec3::Z, tangent)));
    }
    out
}

/// The window's rectangle on the picture (fractions, top-left origin).
fn window_rect(camera: &Camera, (w, h): (u32, u32), corners: [Vec3; 4]) -> [f32; 4] {
    let vp = camera.projection_gl(w as f32 / h as f32) * camera.view();
    let p: Vec<(f32, f32)> = corners
        .iter()
        .map(|p| {
            let c = vp * p.extend(1.0);
            ((c.x / c.w + 1.0) / 2.0, (1.0 - c.y / c.w) / 2.0)
        })
        .collect();
    let xs = p.iter().map(|p| p.0);
    let ys = p.iter().map(|p| p.1);
    [
        xs.clone().fold(f32::MAX, f32::min),
        ys.clone().fold(f32::MAX, f32::min),
        xs.fold(f32::MIN, f32::max),
        ys.fold(f32::MIN, f32::max),
    ]
}

/// The files a variant puts in the override (besides the boxes' models).
fn variant_files(variant: &str) -> Vec<(String, Vec<u8>)> {
    let mut files = vec![("mgbox.tga".to_string(), tga32(16, |_, _| [255, 255, 255, 0]))];
    match variant {
        "chrome1" => {}
        "txi-chrome1" => files.push(("mgbox.txi".into(), b"envmaptexture chrome1\n".to_vec())),
        "gradient" => {
            files.push(("mgbox.txi".into(), b"envmaptexture mgenv\n".to_vec()));
            files.push(("mgenv.tga".into(), gradient()));
        }
        "chrome1-gradient" => files.push(("chrome1.tga".into(), gradient())),
        "ref01-gradient" => files.push(("tic01__ref01.tga".into(), gradient())),
        other => panic!("MG_CHROME_VARIANT {other}"),
    }
    files
}

#[test]
#[ignore]
fn chrome1_reflection_look() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    crate::common::gpu_after_display();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let wanted = |var: &str, name: &str| {
        std::env::var(var).map_or(true, |v| v.split(',').any(|n| n == name))
    };
    let variant = std::env::var("MG_CHROME_VARIANT").unwrap_or_else(|_| "chrome1".into());
    let paint = std::env::var("MG_CHROME_PAINT").ok();
    let tag = match &paint {
        Some(p) => format!("{variant}_{p}"),
        None => variant.clone(),
    };
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let table = base.table("placeables").unwrap();
    let model_of = |row: u32| table.get(row as usize, "ModelName").unwrap().to_ascii_lowercase();
    let mut files = variant_files(&variant);
    let second = std::env::var("MG_CHROME_SHARED").unwrap_or_else(|_| "shared".into());
    for (row, shared) in [(428, false), (447, true)] {
        let model = model_of(row);
        files.push((
            format!("{model}.mdl"),
            box_model(&model, "mgbox", shared, shared && second == "double", 1.0).into_bytes(),
        ));
    }
    if let Some(what) = &paint {
        files.push(("inc_standard.shd".into(), painting(&base, what).into_bytes()));
    }
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let mut report = String::new();
    for view in VIEWS.iter().filter(|v| wanted("MG_CHROME_VIEWS", v.name)) {
        for light in LIGHTS.iter().filter(|l| wanted("MG_CHROME_LIGHTS", l.name)) {
            let lighting = Lighting {
                ambient: light.ambient,
                diffuse: light.diffuse,
                main_light: 0,
                only_tile: None,
            };
            let name = format!("chrome_{tag}_{}_{}", view.name, light.name);
            let mut r = Reading::new(&name, "tic01", lighting, &refs);
            let (facing, ..) = view.script;
            let rad = facing.to_radians();
            let f = Vec3::new(rad.cos(), rad.sin(), 0.0);
            let right = Vec3::new(rad.sin(), -rad.cos(), 0.0);
            let player = Vec3::new(20.0, 20.0, 0.0);
            for b in &BOXES {
                let at = player + f * FORWARD + right * b.lateral;
                r.place(b.row, [at.x, at.y, 0.0], b.turn, &[("Static", Value::Byte(0))]);
            }
            let install =
                GameInstall::new(mg_testkit::nwn_root().unwrap(), Some(r.dir.join("user")), "en");
            let game = GameData::open(&install).unwrap();
            let (are, git) = (r.are.clone(), r.git.clone());
            let (dir, client) = r.shoot(&enter(&logged(""), view.script), SETTINGS);
            print_log(&dir);
            let tileset = mg_area::tileset(&game, are.root.resref("Tileset").unwrap()).ok();
            let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
            let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
            let pc = player_stood(&dir, &game, &model, glam::Vec2::new(20.0, 20.0));
            let camera = if view.fitted {
                scripted_camera(pc, facing)
            } else {
                let (_, d, p) = view.script;
                Camera::game(pc, facing, d * 0.92, p - 1.0, FIT_FOCUS, FIT_FOV)
            };
            let scene = area_scene
                .scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
            let size = (client.width, client.height);
            let (off, on) =
                off_and_on(&gpu, &game, &scene, &camera, size, Some(&client), &dir, "moonglow");
            for (i, b) in BOXES.iter().enumerate() {
                for (face, corners) in faces(b, pc, f, right) {
                    let rect = window_rect(&camera, size, corners);
                    let mut line = format!(
                        "{} {} box{i}({} {:.0}) {face:6}",
                        view.name,
                        light.name,
                        if !b.shared { "flat" } else { second.as_str() },
                        b.turn
                    );
                    for (who, img) in [("client", &client), ("off", &off), ("on", &on)] {
                        let m = mean(img, rect);
                        line += &format!(" | {who} ({:5.1} {:5.1} {:5.1})", m[0], m[1], m[2]);
                    }
                    eprintln!("{line}");
                    report += &(line + "\n");
                }
            }
        }
    }
    let out = mg_testkit::test_output().join(format!("chrome_{tag}.txt"));
    std::fs::write(&out, report).unwrap();
    eprintln!("the numbers: {}", out.display());
}

/// The helmet's box, metres a side.
const HELMET_SIZE: f32 = 0.6;

/// A held item's box, metres a side.
const HELD_SIZE: f32 = 0.5;

/// A helmet whose model is `n` (`helm_<n>`), as the module's `mg_helm<n>`.
fn helmet_item(game: &GameData, n: u16) -> Gff {
    let mut keys: Vec<ResKey> = game
        .resman
        .entries()
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| k.restype == ResType::UTI)
        .collect();
    keys.sort_by_key(|k| k.to_string());
    let mut h = keys
        .iter()
        .filter_map(|k| Gff::read(&game.resman.get(k).ok()?).ok())
        .find(|g| g.root.integer("BaseItem") == Some(17))
        .expect("a helmet");
    h.root.set("ModelPart1", Value::Byte(n as u8));
    h.root.set("PropertiesList", Value::List(Vec::new()));
    let name = format!("mg_helm{n}");
    h.root.set("TemplateResRef", Value::resref(ResRef::from_str(&name).unwrap()));
    h
}

/// A human man wearing (or holding) `worn`, each (inventory slot struct
/// id, template name, item), placed at `at` turned `rotation` (radians, as
/// the part textures' rows turn theirs: pi faces the player).
fn place_worn(r: &mut Reading, at: [f32; 2], rotation: f32, worn: &[(u32, &str, &Gff)]) {
    use mg_module::instances::{Placement, Placing, instance};
    let bp = Gff::read(&r.game.resman.get_named("nw_commale", ResType::UTC).unwrap()).unwrap();
    let mut c = bp.root;
    let look = mg_preview::CreatureLook::new(6);
    for f in look.to_utc().fields {
        c.set(&f.label.to_string_lossy(), f.value);
    }
    c.set("Race", Value::Byte(6));
    let equipped = worn
        .iter()
        .map(|(slot, name, _)| {
            let mut e = mg_gff::Struct::new(*slot);
            e.set("EquippedRes", Value::resref(ResRef::from_str(name).unwrap()));
            e
        })
        .collect();
    c.set("Equip_ItemList", Value::List(equipped));
    // (The game takes off what a creature may not wear: the shield and
    // weapon proficiencies.)
    let feats = [32u16, 44, 45, 46]
        .map(|f| {
            let mut e = mg_gff::Struct::new(1);
            e.set("Feat", Value::Word(f));
            e
        })
        .to_vec();
    c.set("FeatList", Value::List(feats));
    c.set("ItemList", Value::List(Vec::new()));
    let scripts: Vec<String> = c
        .fields
        .iter()
        .map(|f| f.label.to_string_lossy())
        .filter(|l| l.starts_with("Script"))
        .collect();
    for label in scripts {
        c.set(&label, Value::resref(ResRef::EMPTY));
    }
    let item = |res: ResRef| {
        worn.iter().find(|(_, name, _)| res.as_str() == Some(*name)).map(|(_, _, g)| g.root.clone())
    };
    let placing = Placing { game: &r.game, item: &item };
    let at = Placement { position: [at[0], at[1], 0.0], rotation };
    let placed = instance(&placing, ResType::UTC, &c, at, &[]).unwrap();
    let mut list = r.git.root.list("Creature List").map(<[_]>::to_vec).unwrap_or_default();
    list.push(placed);
    r.git.root.set("Creature List", Value::List(list));
}

/// The faces of the helmet box with the model-to-world `m` that the eye
/// sees, each a window of it (0.4 of its side), the faces named by the
/// model's axes.
fn helmet_faces(m: glam::Mat4, eye: Vec3, s: f32) -> Vec<(&'static str, [Vec3; 4])> {
    let middle = Vec3::new(0.0, 0.0, s / 2.0);
    let mut out = Vec::new();
    for (label, n, a, b) in [
        ("top", Vec3::Z, Vec3::X, Vec3::Y),
        ("+x", Vec3::X, Vec3::Y, Vec3::Z),
        ("-x", -Vec3::X, Vec3::Y, Vec3::Z),
        ("+y", Vec3::Y, Vec3::X, Vec3::Z),
        ("-y", -Vec3::Y, Vec3::X, Vec3::Z),
    ] {
        let p = middle + n * (s / 2.0);
        let (world, normal) = (m.transform_point3(p), m.transform_vector3(n).normalize());
        let facing = normal.dot((eye - world).normalize());
        if facing < if label == "top" { 0.0 } else { 0.2 } {
            continue;
        }
        let w = 0.2 * s;
        let corners = [p - a * w - b * w, p + a * w - b * w, p + a * w + b * w, p - a * w + b * w]
            .map(|c| m.transform_point3(c));
        out.push((label, corners));
    }
    out
}

/// [`chrome1_reflection_look`] on a helmet: three men wearing `helm_241`
/// (a box of [`HELMET_SIZE`] of the same white TGA of alpha 0, flat
/// normals; `MG_CHROME_SHARED=shared` or `double` for the part textures'
/// kind of box), facing the camera and turned 45 degrees each side, as the
/// part textures' rows `@` and `Z` put theirs (the game's `Z9@^~` group,
/// whose helmet tops Moonglow draws black where the client draws them
/// gray), from [`VIEWS`] in the same lights. Each face is the model's
/// axis, its window taken from the helmet's place in Moonglow's scene.
/// Moonglow's worn helmet reflects `chrome1` (`mg_preview::creature`'s
/// `p.env_map`); the client's reflects the area's map. The variants and
/// `MG_CHROME_PAINT` are the boxes'.
#[test]
#[ignore]
fn chrome1_helmet_reflection_look() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    crate::common::gpu_after_display();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let wanted = |var: &str, name: &str| {
        std::env::var(var).map_or(true, |v| v.split(',').any(|n| n == name))
    };
    let variant = std::env::var("MG_CHROME_VARIANT").unwrap_or_else(|_| "chrome1".into());
    let paint = std::env::var("MG_CHROME_PAINT").ok();
    let tag = match &paint {
        Some(p) => format!("helmet_{variant}_{p}"),
        None => format!("helmet_{variant}"),
    };
    let tag = match std::env::var("MG_CHROME_HELD") {
        Ok(h) => tag.replace("helmet", &h),
        Err(_) => tag,
    };
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let helmet_kind = std::env::var("MG_CHROME_SHARED").unwrap_or_else(|_| "flat".into());
    // `MG_CHROME_HELD=shield|weapon`: no helmet, a small shield in the left
    // hand or a weapon in the right, whose first model is the box (the
    // others empty).
    let held = std::env::var("MG_CHROME_HELD").ok();
    let mut files = variant_files(&variant);
    let (slot, template, shown, box_size) = match held.as_deref() {
        None => (0x1, "mg_helm241".to_string(), "helm_241".to_string(), HELMET_SIZE),
        Some(what) => {
            let from = if what == "shield" { "nw_ashsw001" } else { "nw_wswls001" };
            let uti = Gff::read(&base.resman.get_named(from, ResType::UTI).unwrap()).unwrap();
            let look = mg_preview::item(&base, &uti.root).unwrap();
            let models: Vec<String> = std::iter::once(&look.base)
                .chain(&look.parts)
                .map(|p| p.model.clone())
                .filter(|m| !m.is_empty())
                .collect();
            eprintln!("the held {what}: {from}, models {models:?}");
            for (i, m) in models.iter().enumerate() {
                let text = if i == 0 {
                    box_model(m, "mgbox", false, false, HELD_SIZE)
                } else {
                    format!(
                        "newmodel {m}\nsetsupermodel {m} NULL\nclassification character\n\
                         setanimationscale 1\nbeginmodelgeom {m}\nnode dummy {m}\n  parent NULL\n\
                         endnode\nendmodelgeom {m}\ndonemodel {m}\n"
                    )
                };
                files.push((format!("{m}.mdl"), text.into_bytes()));
            }
            let slot = if what == "shield" { 0x20 } else { 0x10 };
            (slot, format!("mg_{what}"), models[0].clone(), HELD_SIZE)
        }
    };
    if held.is_none() {
        files.push((
            "helm_241.mdl".into(),
            box_model(
                "helm_241",
                "mgbox",
                helmet_kind != "flat",
                helmet_kind == "double",
                HELMET_SIZE,
            )
            .into_bytes(),
        ));
    }
    if let Some(what) = &paint {
        files.push(("inc_standard.shd".into(), painting(&base, what).into_bytes()));
    }
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let mut report = String::new();
    for view in VIEWS.iter().filter(|v| wanted("MG_CHROME_VIEWS", v.name)) {
        for light in LIGHTS.iter().filter(|l| wanted("MG_CHROME_LIGHTS", l.name)) {
            let lighting = Lighting {
                ambient: light.ambient,
                diffuse: light.diffuse,
                main_light: 0,
                only_tile: None,
            };
            let name = format!("chrome_{tag}_{}_{}", view.name, light.name);
            let mut r = Reading::new(&name, "tic01", lighting, &refs);
            let worn_item = match held.as_deref() {
                None => helmet_item(&r.game, 241),
                Some(what) => {
                    let from = if what == "shield" { "nw_ashsw001" } else { "nw_wswls001" };
                    let mut g =
                        Gff::read(&r.game.resman.get_named(from, ResType::UTI).unwrap()).unwrap();
                    g.root.set("PropertiesList", Value::List(Vec::new()));
                    g.root
                        .set("TemplateResRef", Value::resref(ResRef::from_str(&template).unwrap()));
                    g
                }
            };
            r.m.set_gff(
                ResKey::new(ResRef::from_str(&template).unwrap(), ResType::UTI),
                &worn_item,
            )
            .unwrap();
            let (facing, ..) = view.script;
            let rad = facing.to_radians();
            let f = Vec3::new(rad.cos(), rad.sin(), 0.0);
            let right = Vec3::new(rad.sin(), -rad.cos(), 0.0);
            let player = Vec3::new(20.0, 20.0, 0.0);
            // (Facing the player, and turned 45 degrees each side.)
            let pi = std::f32::consts::PI;
            let toward = rad - 90f32.to_radians() + pi;
            for (lateral, turn) in [(-1.6, 0.0), (0.0, 0.25 * pi), (1.6, -0.25 * pi)] {
                let at = player + f * 3.0 + right * lateral;
                place_worn(&mut r, [at.x, at.y], toward + turn, &[(slot, &template, &worn_item)]);
            }
            let install =
                GameInstall::new(mg_testkit::nwn_root().unwrap(), Some(r.dir.join("user")), "en");
            let game = GameData::open(&install).unwrap();
            let (are, git) = (r.are.clone(), r.git.clone());
            if std::env::var_os("MG_CHROME_DEBUG").is_some() {
                for c in git.root.list("Creature List").unwrap_or_default() {
                    for e in c.list("Equip_ItemList").unwrap_or_default() {
                        let labels: Vec<String> =
                            e.fields.iter().map(|f| f.label.to_string_lossy()).collect();
                        eprintln!("equipped {:?}: {labels:?}", e.id);
                    }
                }
            }
            let (dir, client) =
                r.shoot(&enter(&logged(super::readings::INVISIBLE), view.script), SETTINGS);
            print_log(&dir);
            let tileset = mg_area::tileset(&game, are.root.resref("Tileset").unwrap()).ok();
            let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
            let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
            let pc = player_stood(&dir, &game, &model, glam::Vec2::new(20.0, 20.0));
            let camera = if view.fitted {
                scripted_camera(pc, facing)
            } else {
                let (_, d, p) = view.script;
                Camera::game(pc, facing, d * 0.92, p - 1.0, FIT_FOCUS, FIT_FOV)
            };
            let scene = area_scene
                .scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
            let size = (client.width, client.height);
            let (off, on) =
                off_and_on(&gpu, &game, &scene, &camera, size, Some(&client), &dir, "moonglow");
            let mut helmets: Vec<&Instance> = scene
                .instances
                .iter()
                .filter(|i| i.model.model.name.eq_ignore_ascii_case(&shown))
                .collect();
            helmets.sort_by(|a, b| a.transform.w_axis.x.total_cmp(&b.transform.w_axis.x));
            if helmets.is_empty() {
                let names: std::collections::BTreeSet<&str> =
                    scene.instances.iter().map(|i| i.model.model.name.as_str()).collect();
                eprintln!("no instance of {shown}; models: {names:?}");
            }
            eprintln!(
                "{} helmets at {:?}",
                helmets.len(),
                helmets.iter().map(|h| h.transform.w_axis.truncate()).collect::<Vec<_>>()
            );
            for (i, helmet) in helmets.iter().enumerate() {
                for (face, corners) in helmet_faces(helmet.transform, camera.eye, box_size) {
                    let rect = window_rect(&camera, size, corners);
                    let mut line = format!("{} {} helmet{i} {face:6}", view.name, light.name);
                    for (who, img) in [("client", &client), ("off", &off), ("on", &on)] {
                        let m = mean(img, rect);
                        line += &format!(" | {who} ({:5.1} {:5.1} {:5.1})", m[0], m[1], m[2]);
                    }
                    eprintln!("{line}");
                    report += &(line + "\n");
                }
            }
        }
    }
    let out = mg_testkit::test_output().join(format!("chrome_{tag}.txt"));
    std::fs::write(&out, report).unwrap();
    eprintln!("the numbers: {}", out.display());
}
