//! `Billboard_to_Local_Z` particles in the client beside Moonglow's (PR
//! #43): three still emitters of one particle each on a placeable north
//! of the player, one not turned, one tipped a quarter about its X axis
//! and one about its Y, each its own colour; what each shows (its pixels
//! and the box they fill) in the client's picture and in Moonglow's of the
//! same camera.

use super::*;

/// The emitters: what, where (model x, y, z), turned (axis, angle) and
/// the colour each is told by.
type Emitter = (&'static str, [f32; 3], [f32; 4], [u8; 3]);

const EMITTERS: [Emitter; 5] = [
    ("flat", [-1.6, 0.0, 1.2], [0.0, 0.0, 0.0, 0.0], [255, 0, 0]),
    ("tipped about X", [0.0, 0.0, 1.2], [1.0, 0.0, 0.0, std::f32::consts::FRAC_PI_2], [0, 255, 0]),
    ("tipped about Y", [1.6, 0.0, 1.2], [0.0, 1.0, 0.0, std::f32::consts::FRAC_PI_2], [0, 0, 255]),
    // Its back to the camera: whether the client draws a particle's back.
    (
        "tipped back about X",
        [-1.6, 0.0, 2.4],
        [1.0, 0.0, 0.0, -std::f32::consts::FRAC_PI_2],
        [255, 255, 0],
    ),
    // The same with `twosidedtex 1`.
    (
        "back, two-sided",
        [1.6, 0.0, 2.4],
        [1.0, 0.0, 0.0, -std::f32::consts::FRAC_PI_2],
        [0, 255, 255],
    ),
];

/// Where the placeable stands.
const AT: [f32; 3] = [20.0, 23.0, 0.0];

/// The particles' size.
const SIZE: f32 = 0.8;

fn model() -> String {
    let mut nodes = String::new();
    for (i, (_, at, turn, rgb)) in EMITTERS.iter().enumerate() {
        let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
        nodes += &format!(
            "node emitter lz{i}\n  parent mg_lz\n  position {} {} {}\n  orientation {} {} {} {}\n\
             update Fountain\n  render Billboard_to_Local_Z\n  blend Normal\n  texture mgwhite\n\
             loop 1\n  xgrid 1\n  ygrid 1\n  spawntype 0\n  birthrate 1\n  lifeExp 30\n\
             velocity 0\n  randvel 0\n  spread 0\n  mass 0\n  particleRot 0\n  sizeStart {SIZE}\n\
             sizeEnd {SIZE}\n  xsize 0\n  ysize 0\n  alphaStart 1\n  alphaEnd 1\n\
             colorStart {r} {g} {b}\n  colorEnd {r} {g} {b}\n  twosidedtex {}\nendnode\n",
            at[0],
            at[1],
            at[2],
            turn[0],
            turn[1],
            turn[2],
            turn[3],
            u8::from(i == 4)
        );
    }
    format!(
        "newmodel mg_lz\nsetsupermodel mg_lz NULL\nclassification Character\n\
         setanimationscale 1\nbeginmodelgeom mg_lz\nnode dummy mg_lz\n  parent NULL\n\
         endnode\n{nodes}endmodelgeom mg_lz\ndonemodel mg_lz\n"
    )
}

/// The scene's module: the placeable of [`model`] at [`AT`].
fn module(root: &Path, dir: &Path) -> (GameData, Gff, Gff) {
    use mg_module::instances::{Placement, Placing, instance};
    let over = dir.join("user/override");
    std::fs::create_dir_all(&over).unwrap();
    std::fs::write(over.join("mg_lz.mdl"), model()).unwrap();
    std::fs::write(over.join("mgwhite.tga"), white_tga()).unwrap();
    let base = GameData::open(&GameInstall::new(root, None, "en")).unwrap();
    let table = base.resman.get_named("placeables", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).into_owned();
    let header: Vec<&str> = table.lines().nth(2).unwrap().split_whitespace().collect();
    let model_col = header.iter().position(|c| *c == "ModelName").unwrap() + 1;
    let row = table.lines().count() - 3;
    let mut armoire: Vec<String> =
        table.lines().nth(3).unwrap().split_whitespace().map(str::to_owned).collect();
    armoire[0] = row.to_string();
    armoire[model_col] = "mg_lz".into();
    if !table.ends_with('\n') {
        table.push('\n');
    }
    table += &(armoire.join(" ") + "\n");
    std::fs::write(over.join("placeables.2da"), table).unwrap();
    std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
    let game = GameData::open(&GameInstall::new(root, Some(dir.join("user")), "en")).unwrap();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let (mut m, are) = build_module(&game, dir, light);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: AT, rotation: 0.0 };
    let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    p.set("Appearance", Value::Dword(row as u32));
    p.set("Static", Value::Byte(0));
    git.root.set("Placeable List", Value::List(vec![p]));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    (game, are, git)
}

/// Each emitter's pixels in a picture (near its colour) and the box they
/// fill: (count, width, height).
fn shapes(img: &Rgba) -> Vec<(usize, u32, u32)> {
    EMITTERS
        .iter()
        .map(|(_, _, _, rgb)| {
            let (mut n, mut x0, mut y0, mut x1, mut y1) = (0, u32::MAX, u32::MAX, 0, 0);
            // (The middle of the picture: the game's own interface has red.)
            for y in img.height / 5..img.height * 7 / 10 {
                for x in img.width / 4..img.width * 3 / 4 {
                    let i = ((y * img.width + x) * 4) as usize;
                    let p = &img.data[i..i + 3];
                    let near = p.iter().zip(rgb).all(|(a, b)| a.abs_diff(*b) < 90);
                    if near {
                        n += 1;
                        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                    }
                }
            }
            if n == 0 { (0, 0, 0) } else { (n, x1 - x0 + 1, y1 - y0 + 1) }
        })
        .collect()
}

/// The client's picture (kept as `captures/client/local_z.png`;
/// `MG_CLIENT_CAPTURED=1` reads it again) beside Moonglow's: each
/// emitter's pixels and box in both, within a fifth of each other.
/// Pictures and the report in `target/test-output/client_local_z/`.
#[test]
#[ignore]
fn local_z_particles_lie_as_the_client_lays_them() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("client_local_z");
    let (game, are, git) = module(&root, &dir);
    let Some(client) = client_capture(&dir, "MgScene", "local_z") else {
        panic!("the client did not reach the scene (see {})", dir.display());
    };
    save_png(&client, &dir.join("client.png"));
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped Moonglow's side: no GPU");
        return;
    };
    let tileset = mg_area::tileset(&game, ResRef::from_str("tic01").unwrap()).ok();
    let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let view = mg_area::View { fog: false, time: 6.0, ..mg_area::View::of(&model) };
    let mut scene = area_scene.scene(&model, &view);
    let mut sims = HashMap::new();
    scene.particles = area_scene.particles(&model, &view, &mut sims, 0.0, camera.view());
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    r.game_shaders = true;
    let ours = r.render_image(&gpu, &game.resman, &scene, &camera, client.width, client.height);
    save_png(&ours, &dir.join("moonglow.png"));
    let (theirs, mine) = (shapes(&client), shapes(&ours));
    let mut report = String::new();
    let mut wrong = Vec::new();
    for (i, (what, ..)) in EMITTERS.iter().enumerate() {
        let (c, m) = (theirs[i], mine[i]);
        report += &format!(
            "{what:16} client {} px, {} × {}; Moonglow {} px, {} × {}\n",
            c.0, c.1, c.2, m.0, m.1, m.2
        );
        let near =
            |a: usize, b: usize| (a as f64 - b as f64).abs() <= 0.2 * a.max(b).max(10) as f64;
        if !near(c.0, m.0) || !near(c.1 as usize, m.1 as usize) || !near(c.2 as usize, m.2 as usize)
        {
            wrong.push(*what);
        }
    }
    std::fs::write(dir.join("report.txt"), &report).unwrap();
    eprintln!("{report}{}", dir.display());
    assert!(wrong.is_empty(), "drawn otherwise than the client: {wrong:?}");
}
