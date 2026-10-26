//! An upright particle (`render Aligned_to_World_Z`) in the game client,
//! seen from several sides: whether the client turns it about Z to the
//! eye or keeps a fixed side.
//!
//! The armoire's model (`plc_a01`, overridden in the scratch user folder)
//! holds two still emitters, one above the other, each keeping 1 m
//! particles showing a texture of four colors (drawn: blue and white over
//! red and green): the lower one as it is, the upper one turned a
//! quarter about Z (`orientation 0 0 1 1.5707964`). The armoire is placed
//! (in the area's GIT, facing south as ENTER's is) three metres ahead of
//! the player along the camera's facing, so it is in the middle of each
//! view; the client's camera looks along each facing in turn, one client
//! run each. Moonglow's area view draws the same area from the same side.
//!
//! What each picture shows of each quad (its width across, and which side
//! of it its red quarter is on) and of a card fixed above them (which says
//! the view did turn) is printed and held to what was measured (EE
//! 89.8193.37): both quads face the eye, the same way round, from every
//! side, in the client and in Moonglow. Both pictures go to
//! `target/test-output/client_upright_<facing>/`; `MG_NO_CLIENT=1` draws
//! Moonglow's alone.
//!
//! Needs the game, a GPU and the off-screen display; run by hand:
//! `DISPLAY=:2 cargo test -p mg-corpus-tests --release --test client_render upright -- --ignored --nocapture`.

use super::*;
use mg_module::instances::{Placement, Placing, instance};

/// The camera's facings (degrees anticlockwise from east): north, north-east,
/// then east, south and west.
const FACINGS: [f32; 5] = [90.0, 45.0, 0.0, 270.0, 180.0];

/// The probe model: the two emitters.
fn model() -> String {
    let still = "render Aligned_to_World_Z\n  texture mgquad\n  sizeStart 1\n  sizeEnd 1\n  \
                 birthrate 2\n  velocity 0\n  lifeExp 30";
    let emitters = [
        probe_emitter("plain", [0.0, 0.0, 0.7], still),
        probe_emitter(
            "turned",
            [0.0, 0.0, 2.0],
            &format!("{still}\n  orientation 0 0 1 1.5707964"),
        ),
    ];
    // A card above them, standing in the model's X and Z (two-sided): fixed
    // in the world (the armoire faces south), edge on from north and south and
    // face on from east and west, it shows which way the view looks.
    let card = "node trimesh card\n  parent plc_a01\n  ambient 1 1 1\n  diffuse 1 1 1\n  \
                bitmap mgmagenta\n  verts 4\n    -0.5 0 2.9\n    0.5 0 2.9\n    0.5 0 3.4\n    \
                -0.5 0 3.4\n  tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  faces 4\n    \
                0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\n    0 2 1 1 0 2 1 0\n    0 3 2 1 0 3 2 0\n\
                endnode\n";
    format!(
        "newmodel plc_a01\nsetsupermodel plc_a01 NULL\nclassification Character\n\
         setanimationscale 1\nbeginmodelgeom plc_a01\nnode dummy plc_a01\n  parent NULL\n\
         endnode\n{}{card}endmodelgeom plc_a01\ndonemodel plc_a01\n",
        emitters.concat()
    )
}

/// A magenta 8×8 texture.
fn magenta_tga() -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 32, 8];
    t.extend([255u8, 0, 255, 255].repeat(64));
    t
}

/// How many columns of the picture (clear of the GUI) the card takes, above
/// the row `below` (a fraction of the picture's height: the quads' top).
fn card_width(img: &Rgba, below: f32) -> u32 {
    let (w, h) = (img.width as usize, img.height as usize);
    let columns = (w / 10..w * 9 / 10).filter(|&x| {
        (h / 20..(below * h as f32) as usize).any(|y| {
            let i = (y * w + x) * 4;
            let [r, g, b] = [img.data[i], img.data[i + 1], img.data[i + 2]].map(i32::from);
            r > 60 && b > 60 && 3 * g < r.min(b) && (r - b).abs() < 40
        })
    });
    columns.count() as u32
}

/// One facing's scene: the module saved in the scratch user folder (its
/// `override` holding the model and texture), and the area's ARE and GIT.
fn scene(game: &GameData, dir: &Path, facing: f32) -> (Gff, Gff) {
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    std::fs::write(user.join("override/plc_a01.mdl"), model()).unwrap();
    std::fs::write(user.join("override/mgquad.tga"), quadrant_tga(0)).unwrap();
    std::fs::write(user.join("override/mgmagenta.tga"), magenta_tga()).unwrap();
    std::fs::write(user.join("settings.tml"), SETTINGS).unwrap();
    let light = Lighting { ambient: 0x606060, diffuse: 0x808080, main_light: 0, only_tile: None };
    let (mut m, are) = build_module(game, dir, light);
    let enter: String =
        ENTER.lines().filter(|l| !l.contains("CreateObject")).collect::<Vec<_>>().join("\n");
    // (A second after the player enters: at once, the client keeps its
    // own camera.)
    let enter = enter.replace(
        "AssignCommand(pc, SetCameraFacing(90.0, 12.0, 45.0, CAMERA_TRANSITION_TYPE_SNAP));",
        &format!(
            "DelayCommand(1.0, AssignCommand(pc, SetCameraFacing({facing:.1}, 6.0, 80.0, \
             CAMERA_TRANSITION_TYPE_SNAP)));"
        ),
    );
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(dir, "mg_enter", &enter));
    let git_key = *m.keys_of(ResType::GIT).next().unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let (s, c) = facing.to_radians().sin_cos();
    let at = Placement {
        position: [20.0 + AHEAD * c, 20.0 + AHEAD * s, 0.0],
        rotation: -std::f32::consts::FRAC_PI_2,
    };
    let p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    git.root.set("Placeable List", Value::List(vec![p]));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(user.join("modules/MgScene.mod"))).unwrap();
    (are, git)
}

/// Moonglow's area view of the scene, from about where the client looks.
fn moonglow(
    gpu: &Gpu,
    game: &GameData,
    (are, git): (&Gff, &Gff),
    facing: f32,
    size: (u32, u32),
) -> Rgba {
    let model = mg_area::AreaModel::read(
        game,
        &are.root,
        &git.root,
        mg_area::tileset(game, ResRef::from_str("tic01").unwrap()).ok().as_ref(),
    );
    let area_scene = mg_area::AreaScene::new(gpu, game, &model);
    let view = mg_area::View { fog: false, ..mg_area::View::of(&model) };
    let mut scene = area_scene.scene(&model, &view);
    let camera = camera_facing(Vec3::new(20.0, 20.0, 0.0), facing, 48.0, 1.25, 5.4, 80.0);
    let mut sims = HashMap::new();
    scene.particles = area_scene.particles(&model, &view, &mut sims, 0.0, camera.view());
    let mut r = Renderer::new(gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    r.render_image(gpu, &game.resman, &scene, &camera, size.0, size.1)
}

/// What a picture shows of a quad: the columns its red, green and blue
/// quarters take (first, last), among the rows `rows` (fractions of the
/// picture's height).
#[derive(Debug, Clone, Copy, Default)]
struct Seen {
    red: Option<(u32, u32)>,
    green: Option<(u32, u32)>,
    blue: Option<(u32, u32)>,
}

impl Seen {
    /// Its width across, in pixels (0: not seen, edge on).
    fn width(&self) -> u32 {
        let spans: Vec<(u32, u32)> =
            [self.red, self.green, self.blue].into_iter().flatten().collect();
        let first = spans.iter().map(|s| s.0).min();
        let last = spans.iter().map(|s| s.1).max();
        first.zip(last).map_or(0, |(a, b)| b - a + 1)
    }

    /// Whether its red quarter is left of its green one (the picture's
    /// right way round), if both are seen.
    fn red_left(&self) -> Option<bool> {
        let (r, g) = (self.red?, self.green?);
        Some(r.0 + r.1 < g.0 + g.1)
    }
}

fn seen(img: &Rgba, rows: [f32; 2]) -> Seen {
    let (w, h) = (img.width, img.height);
    let mut spans = [None::<(u32, u32)>; 3];
    // (Clear of the game's GUI at the sides.)
    for y in (rows[0] * h as f32) as u32..(rows[1] * h as f32) as u32 {
        for x in w / 10..w * 9 / 10 {
            let i = (y as usize * w as usize + x as usize) * 4;
            let [r, g, b] = [img.data[i], img.data[i + 1], img.data[i + 2]].map(i32::from);
            let k = if r > 120 && r > 2 * g.max(b) {
                0
            } else if g > 120 && g > 2 * r.max(b) {
                1
            } else if b > 120 && b > 2 * r.max(g) {
                2
            } else {
                continue;
            };
            let s = spans[k].get_or_insert((x, x));
            s.0 = s.0.min(x);
            s.1 = s.1.max(x);
        }
    }
    Seen { red: spans[0], green: spans[1], blue: spans[2] }
}

/// The rows of the picture the upper (turned) and the lower (plain) quad
/// take: split where no quad color is, between the two.
fn bands(img: &Rgba) -> ([f32; 2], [f32; 2]) {
    let (w, h) = (img.width as usize, img.height as usize);
    let colored = |y: usize| {
        (w / 10..w * 9 / 10).any(|x| {
            let i = (y * w + x) * 4;
            let [r, g, b] = [img.data[i], img.data[i + 1], img.data[i + 2]].map(i32::from);
            (r > 120 && r > 2 * g.max(b))
                || (g > 120 && g > 2 * r.max(b))
                || (b > 120 && b > 2 * r.max(g))
        })
    };
    let rows: Vec<usize> = (0..h * 7 / 10).filter(|&y| colored(y)).collect();
    let (Some(&top), Some(&bottom)) = (rows.first(), rows.last()) else {
        return ([0.0, 0.0], [0.0, 0.0]);
    };
    // The widest gap between colored rows.
    let split = rows
        .windows(2)
        .max_by_key(|p| p[1] - p[0])
        .map_or((top + bottom) / 2, |p| (p[0] + p[1]) / 2);
    let f = |y: usize| y as f32 / h as f32;
    ([f(top), f(split)], [f(split), f(bottom + 1)])
}

#[test]
#[ignore]
fn upright_particles_look() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    // `MG_NO_CLIENT=1`: Moonglow's pictures alone.
    let skip_client = std::env::var_os("MG_NO_CLIENT").is_some();
    let mut results = std::collections::BTreeMap::new();
    for facing in FACINGS {
        let dir = scratch_dir(&format!("client_upright_{facing}"));
        let (are, git) = scene(&game, &dir, facing);
        let client = (!skip_client).then(|| client_screenshot(&dir, "MgScene")).flatten();
        if let Some(c) = &client {
            save_png(c, &dir.join("client.png"));
        }
        // (The scratch override's model, as the client reads it.)
        let ours_game =
            GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
        let size = client.as_ref().map_or((1024, 768), |c| (c.width, c.height));
        let ours = moonglow(&gpu, &ours_game, (&are, &git), facing, size);
        save_png(&ours, &dir.join("moonglow.png"));
        for (what, img) in [("client", client.as_ref()), ("moonglow", Some(&ours))] {
            let Some(img) = img else { continue };
            let (upper, lower) = bands(img);
            let (turned, plain) = (seen(img, upper), seen(img, lower));
            let card = card_width(img, upper[0]);
            eprintln!(
                "facing {facing:5.1} {what:8}: card {card:3} px | plain {:3} px wide, red left \
                 {:?} | turned {:3} px wide, red left {:?}",
                plain.width(),
                plain.red_left(),
                turned.width(),
                turned.red_left(),
            );
            results.entry(what).or_insert_with(Vec::new).push((facing, card, plain, turned));
        }
    }
    // Measured (EE 89.8193.37): the card, fixed in the world, is edge on
    // from north and south and face on from east and west (the view
    // turned), while each quad, the turned emitter's too, shows the same
    // width and the same side from every facing: it is turned about Z to
    // the eye. Moonglow's area view draws the same.
    for (what, seen) in &results {
        let (_, _, plain, turned) = seen[0];
        for &(facing, card, p, t) in seen {
            let edge_on = facing == 90.0 || facing == 270.0;
            assert!(
                if edge_on { card < 6 } else { card > 40 },
                "{what}, facing {facing}: the card is {card} px wide (the view did not turn?)"
            );
            for (name, q, first) in [("plain", p, plain), ("turned", t, turned)] {
                assert!(
                    q.width().abs_diff(first.width()) * 10 <= first.width() && first.width() > 20,
                    "{what}, facing {facing}: the {name} quad is {} px wide, {} from the north",
                    q.width(),
                    first.width()
                );
                assert_eq!(q.red_left(), Some(true), "{what}, facing {facing}: the {name} quad");
            }
        }
    }
}
