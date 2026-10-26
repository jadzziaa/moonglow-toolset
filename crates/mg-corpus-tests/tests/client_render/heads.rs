//! A creature's head of 0 in the game client: a human wearing nothing,
//! close before the camera, its `Appearance_Head` (and EE twin) 0, 1, 2,
//! a head with no model (254), or no field at all, one client run each.
//! What the client draws for 0 is the picture it is nearest to, over the
//! head; the server's `GetCreatureBodyPart` says what the engine holds.
//!
//! Measured (EE 89.8193.37): the engine holds 0 for both 0 and no field,
//! and the client draws no head for either, as for a head with no model:
//! the neck alone. Moonglow's area view (`mg-preview`) is held to the same.
//! Each case's client picture and Moonglow's (the area view's drawing of
//! the same area) go to `target/test-output/client_heads_<case>/`;
//! `MG_HEADS=0,1` runs some cases only, `MG_CLIENT_CAMERA` (nwscript)
//! moves the client's camera.
//!
//! Needs the game, a GPU and the off-screen display; run by hand:
//! `DISPLAY=:2 cargo test -p mg-corpus-tests --release --test client_render heads -- --ignored --nocapture`.

use super::*;
use mg_module::instances::{Placement, Placing, instance};

/// The cases: the head written (`None`: neither field).
const CASES: [(&str, Option<u16>); 5] =
    [("0", Some(0)), ("1", Some(1)), ("2", Some(2)), ("254", Some(254)), ("none", None)];

/// The client's camera: close and nearly level, a second after the player
/// enters (at once, the client keeps its own).
const CAMERA: &str = "DelayCommand(1.0, AssignCommand(pc, SetCameraFacing(90.0, 3.0, 80.0, CAMERA_TRANSITION_TYPE_SNAP)));";

/// One case's scene: the module saved in the scratch user folder, and the
/// area's ARE and GIT.
fn scene(game: &GameData, dir: &Path, head: Option<u16>) -> (Gff, Gff) {
    let light = Lighting { ambient: 0x808080, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let (mut m, are) = build_module(game, dir, light);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    let camera = std::env::var("MG_CLIENT_CAMERA").unwrap_or_else(|_| CAMERA.to_string());
    let enter = enter.replace(
        "AssignCommand(pc, SetCameraFacing(90.0, 12.0, 45.0, CAMERA_TRANSITION_TYPE_SNAP));",
        &camera,
    );
    // What the engine holds for the head, logged.
    let enter = enter.replace(
        "    LockCameraDirection(pc, TRUE);",
        "    object o = GetFirstObjectInArea(GetAreaFromLocation(start));\n\
         \x20   while (GetIsObjectValid(o)) {\n\
         \x20       if (GetObjectType(o) == OBJECT_TYPE_CREATURE)\n\
         \x20           Log(\"MG_HEAD \" + IntToString(GetCreatureBodyPart(CREATURE_PART_HEAD, o)));\n\
         \x20       o = GetNextObjectInArea(GetAreaFromLocation(start));\n\
         \x20   }\n\
         \x20   LockCameraDirection(pc, TRUE);",
    );
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(dir, "mg_enter", &enter));

    // The first of the game's human men, wearing and carrying nothing.
    let mut keys: Vec<ResKey> = game
        .resman
        .entries()
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| k.restype == ResType::UTC)
        .collect();
    keys.sort_by_key(|k| k.to_string());
    let human = keys
        .iter()
        .filter_map(|k| Gff::read(&game.resman.get(k).ok()?).ok())
        .find(|g| {
            g.root.integer("Appearance_Type") == Some(6)
                && g.root.integer("Gender") == Some(0)
                && g.root.integer("Appearance_Head").is_some()
        })
        .expect("a human man");
    let mut c = human.root.clone();
    let scripts: Vec<String> = c
        .fields
        .iter()
        .map(|f| f.label.to_string_lossy())
        .filter(|l| l.starts_with("Script"))
        .collect();
    for label in scripts {
        c.set(&label, Value::resref(ResRef::EMPTY));
    }
    c.set("Equip_ItemList", Value::List(Vec::new()));
    c.set("ItemList", Value::List(Vec::new()));
    let wide = mg_rules::items::wide_label("Appearance_Head");
    match head {
        Some(n) => {
            c.set("Appearance_Head", Value::Byte(n as u8));
            c.set(&wide, Value::Word(n));
        }
        None => c.fields.retain(|f| {
            let l = f.label.to_string_lossy();
            l != "Appearance_Head" && l != wide
        }),
    }
    let item = |_: ResRef| -> Option<mg_gff::Struct> { None };
    let placing = Placing { game, item: &item };
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    // A metre and a half north of the player, facing it.
    let at = Placement { position: [20.0, 21.5, 0.0], rotation: std::f32::consts::PI };
    let placed = instance(&placing, ResType::UTC, &c, at, &[]).unwrap();
    git.root.set("Creature List", Value::List(vec![placed]));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    (are, git)
}

/// Moonglow's area view of the scene, from about where the client looks.
fn moonglow(gpu: &Gpu, game: &GameData, are: &Gff, git: &Gff, size: (u32, u32)) -> Rgba {
    let model = mg_area::AreaModel::read(
        game,
        &are.root,
        &git.root,
        mg_area::tileset(game, ResRef::from_str("tic01").unwrap()).ok().as_ref(),
    );
    let area_scene = mg_area::AreaScene::new(gpu, game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let camera = fitted_camera(Vec3::new(20.0, 20.0, 0.0), 48.0, 1.4, 2.6, 82.0);
    let mut r = Renderer::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    r.render_image(gpu, &game.resman, &scene, &camera, size.0, size.1)
}

#[test]
#[ignore]
fn heads_look() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let (mut client, mut moonglow_pictures) = (HashMap::new(), HashMap::new());
    let only = std::env::var("MG_HEADS").ok();
    let cases: Vec<_> = CASES
        .into_iter()
        .filter(|(n, _)| only.as_deref().is_none_or(|o| o.split(',').any(|x| x == *n)))
        .collect();
    for &(name, head) in &cases {
        let dir = scratch_dir(&format!("client_heads_{name}"));
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        let (are, git) = scene(&game, &dir, head);
        let shot = client_screenshot(&dir, "MgScene").expect("the client's picture");
        let log =
            std::fs::read_to_string(dir.join("user/logs/nwclientLog1.txt")).unwrap_or_default();
        let held: Vec<&str> =
            log.lines().filter_map(|l| l.split_once("MG_HEAD ").map(|(_, v)| v.trim())).collect();
        eprintln!("head {name}: the engine holds {held:?}");
        let ours = moonglow(&gpu, &game, &are, &git, (shot.width, shot.height));
        save_png(&ours, &dir.join("moonglow.png"));
        client.insert(name, shot);
        moonglow_pictures.insert(name, ours);
    }
    // How far each picture's head is from the others'.
    let names: Vec<&str> = cases.iter().map(|(n, _)| *n).collect();
    for (what, pictures, region) in
        [("client", &client, CLIENT_HEAD), ("moonglow", &moonglow_pictures, MOONGLOW_HEAD)]
    {
        for a in &names {
            let line: Vec<String> = names
                .iter()
                .map(|b| format!("{b} {:5.2}", difference(&pictures[a], &pictures[b], region)))
                .collect();
            eprintln!("{what} head {a:>4}: {}", line.join(" | "));
        }
    }
    if names.len() < CASES.len() {
        return;
    }
    // Head 0, and no field, look as a head with no model (nothing), not as
    // head 1: in the client, and in Moonglow's area view.
    for (what, pictures, region) in
        [("client", &client, CLIENT_HEAD), ("moonglow", &moonglow_pictures, MOONGLOW_HEAD)]
    {
        let d = |a: &str, b: &str| difference(&pictures[a], &pictures[b], region);
        for case in ["0", "none"] {
            assert!(
                d(case, "254") * 4.0 < d(case, "1"),
                "{what}: head {case} is no head ({:.2} from none, {:.2} from head 1)",
                d(case, "254"),
                d(case, "1")
            );
        }
        assert!(d("1", "254") > 0.5 && d("2", "254") > 0.5, "{what}: heads 1 and 2 are drawn");
    }
}

/// Where the creature's head is in the client's picture and in Moonglow's
/// (fractions of the picture, top-left origin).
const CLIENT_HEAD: [f32; 4] = [0.40, 0.20, 0.60, 0.38];
const MOONGLOW_HEAD: [f32; 4] = [0.40, 0.10, 0.60, 0.45];

/// The mean difference per channel (of 255) of two pictures over a region.
fn difference(a: &Rgba, b: &Rgba, r: [f32; 4]) -> f64 {
    let (w, h) = (a.width as f32, a.height as f32);
    let (mut sum, mut n) = (0f64, 0f64);
    for y in (r[1] * h) as usize..(r[3] * h) as usize {
        for x in (r[0] * w) as usize..(r[2] * w) as usize {
            let i = (y * a.width as usize + x) * 4;
            sum +=
                (0..3).map(|c| f64::from(a.data[i + c].abs_diff(b.data[i + c]))).sum::<f64>() / 3.0;
            n += 1.0;
        }
    }
    sum / n.max(1.0)
}
