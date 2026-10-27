//! Soft particles in the client (`docs/framebuffer-proposal.md`, step 4):
//! what the client gives `fsparticle` (`particleSoftness`, `blendMode`,
//! the shader quality it compiles for), read through a patched copy of the
//! game's own `fsparticle.shd` in the scratch user directory's `override`
//! that paints them over each particle; and the client's picture of
//! particles that cut the floor beside Moonglow's, with and without the
//! fade.
//!
//! The scene: the reference room (`tic01`), a placeable north of the
//! player whose model ([`soft_model`]) holds still emitters of one
//! particle each, in a row at 1.5 m ([`READ_EMITTERS`]: sizes, blends,
//! render modes, a growing one), and three larger ones on the floor
//! ([`FLOOR_EMITTERS`]), half under it.

use super::*;

/// An emitter of [`soft_model`]: what it is, where (model x, y, z), its
/// size at birth and death, its blend and its render mode.
struct Emitter {
    what: &'static str,
    at: [f32; 3],
    size: (f32, f32),
    blend: &'static str,
    render: &'static str,
}

const fn emitter(
    what: &'static str,
    at: [f32; 3],
    size: (f32, f32),
    blend: &'static str,
    render: &'static str,
) -> Emitter {
    Emitter { what, at, size, blend, render }
}

/// The emitters read, numbered from 1 by their place here (index 0: the
/// black card behind the Lighten one, which adds its colour to what is
/// behind it).
const READ_EMITTERS: [Emitter; 13] = [
    emitter("black card", [1.4, 1.0, 1.5], (1.2, 1.2), "Normal", "Normal"),
    emitter("size 0.25", [-2.6, 0.0, 1.5], (0.25, 0.25), "Normal", "Normal"),
    emitter("size 0.5", [-2.0, 0.0, 1.5], (0.5, 0.5), "Normal", "Normal"),
    emitter("size 1", [-1.0, 2.0, 2.4], (1.0, 1.0), "Normal", "Normal"),
    emitter("size 2", [1.0, 4.0, 3.2], (2.0, 2.0), "Normal", "Normal"),
    emitter("Punch-Through 0.5", [-0.3, 0.0, 1.5], (0.5, 0.5), "Punch-Through", "Normal"),
    emitter("Lighten 0.5", [1.4, 0.0, 1.5], (0.5, 0.5), "Lighten", "Normal"),
    emitter("Aligned_to_World_Z 0.5", [0.4, 0.0, 1.5], (0.5, 0.5), "Normal", "Aligned_to_World_Z"),
    emitter(
        "Billboard_to_Local_Z 0.5",
        [2.2, 0.0, 1.5],
        (0.5, 0.5),
        "Normal",
        "Billboard_to_Local_Z",
    ),
    emitter("growing 0.25 to 1", [2.9, 0.0, 2.2], (0.25, 1.0), "Normal", "Normal"),
    emitter("shrinking 1 to 0.25", [2.9, 0.0, 3.2], (1.0, 0.25), "Normal", "Normal"),
    emitter(
        "Billboard_to_World_Z 0.5",
        [-2.6, 0.0, 2.4],
        (0.5, 0.5),
        "Normal",
        "Billboard_to_World_Z",
    ),
    emitter("Motion_Blur 0.5", [-1.9, 0.0, 3.1], (0.5, 0.5), "Normal", "Motion_Blur"),
];

/// The particles on the floor, half under it: what each is, where, its
/// blend and its colour.
const FLOOR_EMITTERS: [(&str, [f32; 3], &str, [f32; 3]); 3] = [
    ("floor Normal", [-1.6, 0.0, 0.0], "Normal", [1.0, 0.55, 0.25]),
    ("floor Punch-Through", [0.0, 0.0, 0.0], "Punch-Through", [0.3, 0.9, 0.4]),
    ("floor Lighten", [1.6, 0.0, 0.0], "Lighten", [0.4, 0.5, 1.0]),
];

/// The floor particles' size.
const FLOOR_SIZE: f32 = 1.2;

/// The rows of a floor particle's foot compared, from its edge up.
const FOOT_ROWS: usize = 8;

/// Where the placeable stands (its model's origin; turned by nothing).
const SOFT_AT: [f32; 3] = [20.0, 23.0, 0.0];

/// One still particle (made once a second, living 30: the same place),
/// textured white. (A Lighten one lives 0.9 s: a few in one place add up
/// to white. A motion-blurred one drifts, 1 cm a second, to have a
/// direction.)
fn one(
    name: &str,
    at: [f32; 3],
    size: (f32, f32),
    blend: &str,
    render: &str,
    rgb: [f32; 3],
) -> String {
    let [r, g, b] = rgb;
    let life = if blend == "Lighten" { 0.9 } else { 30.0 };
    let speed = if render == "Motion_Blur" { 0.01 } else { 0.0 };
    format!(
        "node emitter {name}\n  parent mg_soft\n  position {} {} {}\n  orientation 0 0 0 0\n\
         update Fountain\n  render {render}\n  blend {blend}\n  texture mgwhite\n  loop 1\n\
         xgrid 1\n  ygrid 1\n  spawntype 0\n  birthrate 1\n  lifeExp {life}\n  velocity {speed}\n\
         randvel 0\n  spread 0\n  mass 0\n  particleRot 0\n  sizeStart {}\n  sizeEnd {}\n\
         xsize 0\n  ysize 0\n  alphaStart 1\n  alphaEnd 1\n  colorStart {r} {g} {b}\n\
         colorEnd {r} {g} {b}\nendnode\n",
        at[0], at[1], at[2], size.0, size.1
    )
}

/// The placeable's model: with `read`, [`READ_EMITTERS`] too, each
/// coloured by its number (red: number / 16) for the patched `fsparticle`
/// to tell them apart.
fn soft_model(read: bool) -> String {
    let mut nodes = String::new();
    if read {
        for (i, e) in READ_EMITTERS.iter().enumerate() {
            let red = i as f32 / 16.0;
            nodes += &one(&format!("r{i}"), e.at, e.size, e.blend, e.render, [red, 1.0, 1.0]);
        }
    }
    for (k, (_, at, blend, rgb)) in FLOOR_EMITTERS.iter().enumerate() {
        let rgb = if read { [(READ_EMITTERS.len() + k) as f32 / 16.0, 1.0, 1.0] } else { *rgb };
        nodes += &one(&format!("f{k}"), *at, (FLOOR_SIZE, FLOOR_SIZE), blend, "Normal", rgb);
    }
    format!(
        "newmodel mg_soft\nsetsupermodel mg_soft NULL\nclassification Character\n\
         setanimationscale 1\nbeginmodelgeom mg_soft\nnode dummy mg_soft\n  parent NULL\n\
         endnode\n{nodes}endmodelgeom mg_soft\ndonemodel mg_soft\n"
    )
}

/// The game's `fsparticle.shd` painting, over each particle, what it was
/// given: red and green `particleSoftness` / 64 (in two bytes), blue
/// 200 + 3 × the emitter's number (its colour's red × 16) + a flag (0;
/// 1: no softness uniform, the shader quality in red / 4; 2: the softness
/// 64 or more). Number 0 paints black.
fn patched_fsparticle(game: &GameData) -> String {
    let src = game.resman.get_named("fsparticle", ResType::SHD).expect("fsparticle.shd");
    let src = String::from_utf8_lossy(&src).replace("\r\n", "\n");
    let end = "    gl_FragColor = FragmentColor;\n}";
    let at = src.rfind(end).expect("fsparticle's end");
    let mut out = src.clone();
    out.insert_str(
        at + end.len() - 1,
        r#"    int mgIndex = int(VertexColor.r * 16.0 + 0.5);
    float mgFlag = 0.0;
    float mgValue = 0.0;
#if SHADER_QUALITY_MODE > 0 && !defined MOBILE
    mgValue = particleSoftness / 64.0;
    if (mgValue >= 1.0) { mgFlag = 2.0; mgValue = 0.99999; }
#else
    mgFlag = 1.0;
    mgValue = float(SHADER_QUALITY_MODE) / 4.0;
#endif
    float mgHi = floor(mgValue * 255.0);
    gl_FragColor = vec4(mgHi / 255.0, fract(mgValue * 255.0), (200.0 + 3.0 * float(mgIndex) + mgFlag) / 255.0, 1.0);
    if (mgIndex == 0) gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
"#,
    );
    out
}

/// What [`read_marks`] found of an emitter: its pixels, the softness's
/// values with their pixels (the most common first), its largest flag.
type Marks = (usize, Vec<(f32, usize)>, u8);

/// Whether a pixel shows a floor particle (by its colour).
type Shows = fn(&[u8]) -> bool;

/// What [`patched_fsparticle`] painted: by emitter number, the pixels
/// seen, the softness's values (the most common first) and the flags.
fn read_marks(img: &Rgba) -> std::collections::BTreeMap<usize, Marks> {
    let mut seen: std::collections::BTreeMap<usize, (usize, HashMap<u32, usize>, u8)> =
        Default::default();
    for p in img.data.chunks(4) {
        let b = p[2];
        if b < 200 || p[3] != 255 {
            continue;
        }
        let (index, flag) = (usize::from(b - 200) / 3, (b - 200) % 3);
        if index == 0 || index > READ_EMITTERS.len() + FLOOR_EMITTERS.len() {
            continue;
        }
        let raw = u32::from(p[0]) * 256 + u32::from(p[1]);
        let e = seen.entry(index).or_insert_with(|| (0, HashMap::new(), flag));
        e.0 += 1;
        *e.1.entry(raw).or_default() += 1;
        e.2 = e.2.max(flag);
    }
    seen.into_iter()
        .map(|(i, (n, values, flag))| {
            let mut v: Vec<(f32, usize)> = values
                .into_iter()
                .map(|(raw, c)| {
                    let (hi, lo) = ((raw >> 8) as f32, (raw & 255) as f32);
                    ((hi + lo / 255.0) / 255.0 * if flag == 1 { 4.0 } else { 64.0 }, c)
                })
                .collect();
            v.sort_by_key(|e| std::cmp::Reverse(e.1));
            (i, (n, v, flag))
        })
        .collect()
}

/// The name of emitter number `i` of [`read_marks`].
fn emitter_name(i: usize) -> &'static str {
    READ_EMITTERS
        .get(i)
        .map(|e| e.what)
        .unwrap_or_else(|| FLOOR_EMITTERS.get(i - READ_EMITTERS.len()).map_or("?", |f| f.0))
}

/// The scene's module in `dir` (its user folder: the model, the white
/// texture, a `placeables.2da` with the placeable's row, `settings`).
fn soft_module(root: &Path, dir: &Path, read: bool, settings: &str) -> (GameData, Gff, Gff) {
    use mg_module::instances::{Placement, Placing, instance};
    let over = dir.join("user/override");
    std::fs::create_dir_all(&over).unwrap();
    std::fs::write(over.join("mg_soft.mdl"), soft_model(read)).unwrap();
    std::fs::write(over.join("mgwhite.tga"), white_tga()).unwrap();
    let base = GameData::open(&GameInstall::new(root, None, "en")).unwrap();
    if read {
        std::fs::write(over.join("fsparticle.shd"), patched_fsparticle(&base)).unwrap();
    }
    let table = base.resman.get_named("placeables", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).into_owned();
    let header: Vec<&str> = table.lines().nth(2).unwrap().split_whitespace().collect();
    let model_col = header.iter().position(|c| *c == "ModelName").unwrap() + 1;
    let row = table.lines().count() - 3;
    let mut armoire: Vec<String> =
        table.lines().nth(3).unwrap().split_whitespace().map(str::to_owned).collect();
    armoire[0] = row.to_string();
    armoire[model_col] = "mg_soft".into();
    if !table.ends_with('\n') {
        table.push('\n');
    }
    table += &(armoire.join(" ") + "\n");
    std::fs::write(over.join("placeables.2da"), table).unwrap();
    std::fs::write(dir.join("user/settings.tml"), settings).unwrap();
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
    let at = Placement { position: SOFT_AT, rotation: 0.0 };
    let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    p.set("Appearance", Value::Dword(row as u32));
    p.set("Static", Value::Byte(0));
    git.root.set("Placeable List", Value::List(vec![p]));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
    (game, are, git)
}

/// Exploration and comparison: soft particles in the client. `MG_RUNS`
/// (comma-separated) picks the runs: `read_hq` (the patched `fsparticle`
/// at the tests' settings, "High Quality", which Moonglow compiles for),
/// `read_default` (at the client's default shader quality, "Performance"),
/// `read_medium` ("Medium"), `look_default` (the stock program at a
/// player's default settings, to look at) and `look` (the stock program; the client's
/// picture kept as `captures/client/soft_particles.png`, which
/// `MG_CLIENT_CAPTURED=1` reads instead of running the client, beside
/// Moonglow's with the fade and without it). Default: all but `read_medium`.
/// Pictures and reports in `target/test-output/client_soft_<run>/`.
#[test]
#[ignore]
fn soft_particles_in_the_client() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let runs = std::env::var("MG_RUNS").unwrap_or_else(|_| "read_hq,read_default,look".into());
    for run in runs.split(',').map(str::trim) {
        let dir = scratch_dir(&format!("client_soft_{run}"));
        let read = run.starts_with("read");
        // (The client's default shader quality, "Performance", with the
        // post effects off as in the tests' settings: they change the
        // colours read; a player's defaults are `look_default`.)
        let quality = |q: &str| SETTINGS.replace("\"High Quality\"", &format!("\"{q}\""));
        let settings = match run {
            "read_default" => quality("Performance"),
            "read_medium" => quality("Medium"),
            "look_default" => common::player_settings(false),
            _ => SETTINGS.to_string(),
        };
        let (game, are, git) = soft_module(&root, &dir, read, &settings);
        let client = if run == "look" {
            client_capture(&dir, "MgScene", "soft_particles")
        } else {
            client_screenshot(&dir, "MgScene")
        };
        let Some(client) = client else {
            panic!("{run}: the client did not reach the scene (see {})", dir.display());
        };
        save_png(&client, &dir.join("client.png"));
        if read {
            let mut report = format!("run {run}\n");
            for (i, (n, values, flag)) in read_marks(&client) {
                let shown: Vec<String> =
                    values.iter().take(3).map(|(v, c)| format!("{v:.4} ({c} px)")).collect();
                let what = match flag {
                    1 => "no particleSoftness: shader quality",
                    2 => "particleSoftness 64 or more",
                    _ => "particleSoftness",
                };
                report +=
                    &format!("{:26} {n:6} px  {what} {}\n", emitter_name(i), shown.join(", "));
            }
            std::fs::write(dir.join("report.txt"), &report).unwrap();
            eprintln!("{report}{}", dir.display());
            continue;
        }
        if run != "look" {
            continue;
        }
        soft_in_moonglow(&game, &are, &git, &client, &dir);
    }
}

/// Moonglow's side of the `look` run: the scene with the particles'
/// fade and without (`Renderer::soft_particles`), each held to the
/// client over the floor particles' feet (where they meet the floor) and
/// over the whole picture.
fn soft_in_moonglow(game: &GameData, are: &Gff, git: &Gff, client: &Rgba, dir: &Path) {
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped Moonglow's side: no GPU");
        return;
    };
    let tileset = mg_area::tileset(game, ResRef::from_str("tic01").unwrap()).ok();
    let model = mg_area::AreaModel::read(game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, game, &model);
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let view = mg_area::View { fog: false, time: 6.0, ..mg_area::View::of(&model) };
    let mut scene = area_scene.scene(&model, &view);
    let mut sims = HashMap::new();
    scene.particles = area_scene.particles(&model, &view, &mut sims, 0.0, camera.view());
    let (w, h) = (client.width, client.height);
    let vp = camera.projection(w as f32 / h as f32) * camera.view();
    let fraction = |p: Vec3| {
        let c = vp * p.extend(1.0);
        let n = c.truncate() / c.w;
        (n.x * 0.5 + 0.5, 0.5 - n.y * 0.5)
    };
    // A particle's foot, where it meets the floor: in each picture, down
    // each column of its middle (from the camera's projection, a fifth of
    // its width each side), the last rows it shows before the floor, as
    // seen by its colour; their mean colour row by row from the edge up,
    // [`FOOT_ROWS`] of them. (Each picture's own edge: the fitted camera
    // is a few pixels off the client's.)
    let feet: Vec<(&str, u32, u32, u32, Shows)> = FLOOR_EMITTERS
        .iter()
        .zip([
            (|p: &[u8]| p[0] > 140 && p[2] < 140 && p[0] > p[1]) as Shows,
            |p: &[u8]| p[1] > 140 && p[0] < 140,
            |p: &[u8]| p[2] > 150 && p[2] >= p[0],
        ])
        .map(|((what, at, _, _), shows)| {
            let base = Vec3::from(SOFT_AT) + Vec3::from(*at);
            let (x0, y) = fraction(base - Vec3::new(FLOOR_SIZE * 0.2, 0.0, 0.0));
            let (x1, _) = fraction(base + Vec3::new(FLOOR_SIZE * 0.2, 0.0, 0.0));
            let px = |f: f32, n: u32| (f * n as f32) as u32;
            (*what, px(x0, w), px(x1, w), px(y, h), shows)
        })
        .collect();
    let profile = |img: &Rgba, (x0, x1, y, shows): (u32, u32, u32, Shows)| {
        let mut rows = [[0f64; 3]; FOOT_ROWS];
        let mut n = 0f64;
        for x in x0..=x1 {
            let at = |y: u32| (y as usize * w as usize + x as usize) * 4;
            let Some(edge) = (y.saturating_sub(40)..(y + 40).min(h))
                .rev()
                .find(|&y| shows(&img.data[at(y)..at(y) + 4]))
            else {
                continue;
            };
            for (k, row) in rows.iter_mut().enumerate() {
                let i = at(edge.saturating_sub(k as u32));
                for (sum, v) in row.iter_mut().zip(&img.data[i..i + 3]) {
                    *sum += f64::from(*v);
                }
            }
            n += 1.0;
        }
        rows.iter().map(|r| r.map(|v| (v / n.max(1.0)) as f32)).collect::<Vec<[f32; 3]>>()
    };
    let mut report = String::new();
    let mut pictures = Vec::new();
    for (soft, name) in [(false, "moonglow_hard"), (true, "moonglow_soft")] {
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
        r.game_shaders = true;
        r.soft_particles = soft;
        let img = r.render_image(&gpu, &game.resman, &scene, &camera, w, h);
        assert_eq!(r.game_shader_failures(), &[], "{name}");
        save_png(&img, &dir.join(format!("{name}.png")));
        pictures.push(img);
    }
    let away = |a: [f32; 3], b: [f32; 3]| (0..3).map(|i| (a[i] - b[i]).abs()).sum::<f32>() / 3.0;
    for (what, x0, x1, y, shows) in &feet {
        let c = profile(client, (*x0, *x1, *y, *shows));
        let [hard, soft] = [&pictures[0], &pictures[1]].map(|p| profile(p, (*x0, *x1, *y, *shows)));
        let mean_away = |ours: &[[f32; 3]]| {
            c.iter().zip(ours).map(|(a, b)| away(*a, *b)).sum::<f32>() / c.len() as f32
        };
        report += &format!(
            "{what:22} the foot's last {FOOT_ROWS} rows: without the fade {:.1}, with it {:.1} of 255 \
             from the client's\n",
            mean_away(&hard),
            mean_away(&soft)
        );
        for k in 0..FOOT_ROWS {
            report += &format!(
                "    row {k}: client {:5.1?} without {:5.1?} with {:5.1?}\n",
                c[k], hard[k], soft[k]
            );
        }
    }
    report += &format!(
        "the picture: without the fade {:.2}, with it {:.2} of 255 from the client's\n",
        super::picture_difference(client, &pictures[0]),
        super::picture_difference(client, &pictures[1])
    );
    std::fs::write(dir.join("report.txt"), &report).unwrap();
    eprintln!("{report}{}", dir.display());
}
