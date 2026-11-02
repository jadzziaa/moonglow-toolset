//! The deferred sprint's seventh client batch (2026-11-02): readings of the
//! client that answer the deferred list's questions, beside the first six
//! batches' (`readings.rs`, `batch2.rs` to `batch6.rs`, whose scaffolding
//! they use). Each writes the client's pictures to its scratch directory
//! (`target/test-output/client_reading_<name>/`) and prints where; what
//! was seen is in its comment.
//!
//! Needs the game and the off-screen display; run by hand, one client at a
//! time:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render batch7 -- --ignored --nocapture --test-threads 1`.

use std::process::Command;

use super::programs::{Mark, Quad, card_corner, marked, marks_at, model, standing};
use super::readings::{INVISIBLE, Reading, enter, placeables_with};
use super::*;

/// The cards of [`batch7_bucket_order`], west to east: what each is, its
/// texture (of the set's names: `<set>_plain`…) and its mesh's lines.
const BUCKET_CARDS: [(&str, &str, &str); 6] = [
    ("plain", "plain", ""),
    ("plain alpha 0.5", "plain", "  alpha 0.5\n"),
    ("decal red", "decalr", ""),
    ("decal red alpha 0.5", "decalr", "  alpha 0.5\n"),
    ("envmap", "env", ""),
    ("envmap alpha 0.5", "env", "  alpha 0.5\n"),
];

/// The rows of [`batch7_bucket_order`], from the ground up: what holds
/// them, the texture set (a copy of `programs.rs`' textures of its own,
/// so that no two rows share a bucket) and whether each pair's
/// see-through mesh comes first in the model.
const BUCKET_ROWS: [(&str, &str, bool); 4] = [
    ("static placeable, opaque first", "mgq", false),
    ("static placeable, see-through first", "mgr", true),
    ("tile, opaque first", "mgt", false),
    ("tile, see-through first", "mgu", true),
];

/// A texture name of a set, for a model's `bitmap` (they live as long
/// as the test).
fn set_texture(set: &str, texture: &str) -> &'static str {
    Box::leak(format!("{set}_{texture}").into_boxed_str())
}

/// Row `row`'s cards as quads, in the model's order (the pairs' order
/// as the row says), each card at its place in the row (`card_corner`'s
/// columns, `x` and `y` added, `z` metres up).
fn bucket_quads(row: usize, [x, y, z]: [f32; 3]) -> Vec<Quad> {
    let (_, set, see_through_first) = BUCKET_ROWS[row];
    let mut order: Vec<usize> = (0..BUCKET_CARDS.len()).collect();
    if see_through_first {
        for pair in order.chunks_mut(2) {
            pair.swap(0, 1);
        }
    }
    order
        .into_iter()
        .map(|i| {
            let (_, texture, extra) = BUCKET_CARDS[i];
            Quad {
                name: format!("r{row}c{i}"),
                at: [x + card_corner(i).0, y, z],
                corners: standing(0.8),
                bitmap: set_texture(set, texture),
                extra,
                tverts: true,
                colors: false,
            }
        })
        .collect()
}

/// Which mesh of a static bucket gives its `NO_DISCARD`, and whether tiles
/// bucket as static placeables do (the deferred list's M4: a static
/// placeable's cards sharing a texture seemed to share one `NO_DISCARD`).
/// Every stock program marked (`program_choice_in_the_client`'s), and
/// three pairs of cards sharing a texture and a program, each an opaque
/// mesh and the same with `alpha 0.5` (plain, a red decal, an environment
/// map): in four rows north of the player, each of its own copies of the
/// textures, so that no two share a bucket. The lowest two are static
/// placeables (placeables.2da `Static` 1), the pairs' opaque mesh first in
/// one, the see-through one first in the other; the upper two the same
/// cards in a tile's model (the tile north-west of the player, its model
/// replaced in the override). Each card's mark printed (`tcn01`,
/// `client_reading_bucket_order`).
///
/// Seen (2026-11-02, 89.8193.37): the first mesh of a pair in the model's
/// order gives both their `NO_DISCARD` and their opacity, in a static
/// placeable and in a tile alike: opaque first, every card `NO_DISCARD` 1
/// and drawn solid, the `alpha 0.5` cards too; see-through first, every
/// card `NO_DISCARD` 0 and drawn half see-through, the opaque ones too
/// (`MG_UNMARKED=1`); the programs are each card's own (`vslit`/`fslit`,
/// `vsvt`/`fst` for the decal, `vslit_sm`/`fslit_sm` for the map).
#[test]
#[ignore]
fn batch7_bucket_order() {
    let base = super::batch5::base_game();
    let (vs, fs) = super::programs::programs(&base);
    let mut overrides: Vec<(String, Vec<u8>)> = Vec::new();
    // (`MG_UNMARKED=1`: the cards as the client draws them, for whether a
    // see-through card is blended.)
    let unmarked = std::env::var_os("MG_UNMARKED").is_some();
    for (list, vertex) in [(&vs, true), (&fs, false)].into_iter().filter(|_| !unmarked) {
        for (i, name) in list.iter().enumerate() {
            let src = base.resman.get_named(name, ResType::SHD).unwrap();
            let text = marked(&String::from_utf8_lossy(&src), vertex, i + 1);
            overrides.push((format!("{name}.shd"), text.into_bytes()));
        }
    }
    let (table, first) = placeables_with(&base, &[("mg_bk_ab", 1), ("mg_bk_ba", 1)]);
    overrides.push(("placeables.2da".into(), table.into_bytes()));
    // (The cards stand where `program_choice_in_the_client`'s did: their
    // placeables at (18, 23), on the tile whose middle is (15, 25).)
    let (at, tile_middle) = ([18.0f32, 23.0], [15.0f32, 25.0]);
    for (row, name) in [(0, "mg_bk_ab"), (1, "mg_bk_ba")] {
        let quads = bucket_quads(row, [0.0, 0.0, row as f32 + 0.1]);
        overrides.push((format!("{name}.mdl"), model(name, &quads).into_bytes()));
    }
    let named: Vec<(&str, Vec<u8>)> =
        overrides.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("bucket_order", "tcn01", light, &named);
    let over = r.dir.join("user/override");
    super::programs::write_textures(&over);
    for (_, set, _) in BUCKET_ROWS {
        for texture in ["plain", "decalr", "env"] {
            for ext in ["tga", "txi"] {
                let from = over.join(format!("mgp_{texture}.{ext}"));
                if from.is_file() {
                    std::fs::copy(&from, over.join(format!("{set}_{texture}.{ext}"))).unwrap();
                }
            }
        }
    }
    // The tile: one of the tileset's whose model no other tile of the area
    // has, its model the upper rows' cards (and a floor of its own).
    let set = r.game.resman.get_named("tcn01", ResType::SET).unwrap();
    let set = mg_set::Tileset::parse(&set, r.game.language.codepage()).unwrap();
    let width = r.are.root.int("Width").unwrap_or(4) as usize;
    let tile_index = (tile_middle[1] / 10.0) as usize * width + (tile_middle[0] / 10.0) as usize;
    let tiles = r.are.root.list_mut("Tile_List").unwrap();
    let used: Vec<String> = tiles
        .iter()
        .filter_map(|t| set.tiles.get(t.int("Tile_ID").unwrap_or(0) as usize))
        .map(|t| t.model.to_ascii_lowercase())
        .collect();
    let id = set
        .tiles
        .iter()
        .position(|t| !t.model.is_empty() && !used.contains(&t.model.to_ascii_lowercase()))
        .expect("a tile of a model the area has not");
    let tile_model = set.tiles[id].model.to_ascii_lowercase();
    tiles[tile_index].set("Tile_ID", Value::Int(id as i32));
    tiles[tile_index].set("Tile_Orientation", Value::Int(0));
    tiles[tile_index].set("Tile_Height", Value::Int(0));
    let local = [at[0] - tile_middle[0], at[1] - tile_middle[1]];
    let mut quads = Vec::new();
    for row in [2, 3] {
        quads.extend(bucket_quads(row, [local[0], local[1], row as f32 + 0.1]));
    }
    quads.push(Quad {
        name: "floor".into(),
        at: [-5.0, -5.0, 0.0],
        corners: [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0], [0.0, 10.0, 0.0]],
        bitmap: "mgp_nrm",
        extra: "",
        tverts: true,
        colors: false,
    });
    std::fs::write(over.join(format!("{tile_model}.mdl")), model(&tile_model, &quads)).unwrap();
    for row in [first, first + 1] {
        r.place(row, [at[0], at[1], 0.0], 0.0, &[("Static", Value::Byte(1))]);
    }
    let (dir, client) = r.shoot(&enter(INVISIBLE, (90.0, 12.0, 45.0)), SETTINGS);
    save_png(
        &client,
        &dir.join(if unmarked { "client_unmarked.png" } else { "client_marked.png" }),
    );

    let (w, h) = (client.width as f32, client.height as f32);
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let vp = camera.projection(w / h) * camera.view();
    let screen = |p: Vec3| {
        let c = vp * p.extend(1.0);
        let n = c.truncate() / c.w;
        ((n.x * 0.5 + 0.5) * w, (0.5 - n.y * 0.5) * h)
    };
    let name = |m: &Mark| {
        let v = vs.get(usize::from(m.vs).wrapping_sub(1)).map_or("?", String::as_str);
        let f = fs.get(usize::from(m.fs).wrapping_sub(1)).map_or("?", String::as_str);
        format!("{v}/{f} NO_DISCARD {}", u8::from(m.no_discard))
    };
    eprintln!("the tile replaced: {tile_model} (tile {tile_index})");
    for (row, (what, set, _)) in BUCKET_ROWS.iter().enumerate() {
        for (i, (card, ..)) in BUCKET_CARDS.iter().enumerate() {
            let x = at[0] + card_corner(i).0 + 0.4;
            let (sx, sy) = screen(Vec3::new(x, at[1], row as f32 + 0.5));
            let marks = marks_at(&client, sx, sy, 4);
            let seen = marks.first().map_or("(no mark)".to_string(), |(m, s)| {
                format!("{} ({:.0}%)", name(m), s * 100.0)
            });
            eprintln!("{what} ({set}), {card:20}: {seen}");
        }
    }
    eprintln!("{}", dir.display());
}

/// The client's world timer as a day of 5 minutes an hour starts it
/// (`CWorldTimer::CWorldTimer`: 23:59:56 of 1 June 1340, 10,496,000 ms
/// into a day of 7,200,000).
const TIMER_START: i64 = 10_496_000;
/// That first day's length, in milliseconds.
const TIMER_FIRST_DAY: i64 = 7_200_000;

/// The machine's `CLOCK_MONOTONIC` in milliseconds (the clock the client's
/// timers read), by Python's `time.monotonic`.
fn monotonic_ms() -> i64 {
    let out = Command::new("python3")
        .args(["-I", "-c", "import time; print(int(time.monotonic() * 1000))"])
        .output()
        .expect("python3");
    String::from_utf8_lossy(&out.stdout).trim().parse().expect("a count of milliseconds")
}

/// `tools/nwclient/timeshift.c` built into `dir` (outside /tmp, which the
/// sandbox hides): the library to preload.
fn timeshift(dir: &Path) -> PathBuf {
    let lib = dir.join("libtimeshift.so");
    let ok = Command::new("cc")
        .args(["-O2", "-shared", "-fPIC", "-o"])
        .arg(&lib)
        .arg(common::repo().join("tools/nwclient/timeshift.c"))
        .arg("-ldl")
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "cc could not build timeshift.c");
    lib
}

/// What starts `worldtimerTimeOfDay` (the deferred list's M1): the time
/// module of `time_uniforms_in_the_client` (13 h, 60 minutes an hour: a
/// day of 86,400,000 ms) run three times, as it is, with the wall clock
/// two hours on (`MG_SHIFT_REAL`) and with the clock counted from the
/// machine's boot two hours on (`MG_SHIFT_MONO`), both moved for the client
/// alone by `tools/nwclient/timeshift.c` (`MG_CLIENT_PRELOAD`). Each run's
/// first picture's timer is held to the rule read from the client
/// (`CWorldTimer`, 89.8193.37: its constructor's day of 5 minutes an hour
/// set to 23:59:56, its offset taken from `CLOCK_MONOTONIC` then; the
/// module's minutes an hour, set as the module loads, change the day's
/// length and not the offset): the timer is
/// `(10,496,000 + 7,200,000 × ⌊boot clock at the start / 7,200,000⌋ +
/// the milliseconds since) mod the module's day`.
///
/// Seen (2026-11-02, 89.8193.37, the boot clock at 66,083 s, ⌊…⌋ 9):
/// 75,304,169 as it is (8.2 s after the start), 75,304,327 with the wall
/// clock two hours on (no change), 82,504,381 with the boot clock two
/// hours on (7,200,000 more: ⌊…⌋ 10) and 3,304,388 four hours on (⌊…⌋
/// 11, past the day). The earlier batches' two values, 17,697,400 and
/// 24,897,600 as the player entered, were ⌊…⌋ 1 and 2 at 30 minutes an
/// hour and less; within a sitting of runs the boot clock's two-hour step
/// stays the same, which made them look fixed.
#[test]
#[ignore]
fn batch7_timer_origin() {
    let game = super::batch5::base_game();
    let shader = debug_shader(&game);
    let probe = scratch_dir("client_reading_timer_origin");
    let lib = timeshift(&probe);
    let lib = lib.to_string_lossy().into_owned();
    let day = 86_400_000i64;
    let cases: [(&str, &str, i64); 3] = [
        ("as it is", "", 0),
        ("wall clock +2 h", "MG_SHIFT_REAL", 0),
        ("boot clock +2 h", "MG_SHIFT_MONO", 7200),
    ];
    let mut seen = Vec::new();
    for (i, (what, var, mono)) in cases.into_iter().enumerate() {
        let dir = scratch_dir(&format!("client_reading_timer_origin_{i}"));
        let light =
            Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
        let (mut m, _) = build_module(&game, &dir, light);
        m.set(
            ResKey::parse("mg_enter", ResType::NCS).unwrap(),
            compile(&dir, "mg_enter", TIME_ENTER),
        );
        let mut info = m.info().unwrap();
        info.root.write(&ifo::MOD_START_HOUR, 13u8);
        info.root.write(&ifo::MOD_MIN_PER_HOUR, 60u8);
        m.set_info(&info).unwrap();
        std::fs::create_dir_all(dir.join("user/override")).unwrap();
        std::fs::write(dir.join("user/settings.tml"), SETTINGS).unwrap();
        std::fs::write(dir.join("user/override/inc_standard.shd"), &shader).unwrap();
        m.save_as(&ModuleLocation::Archive(dir.join("user/modules/MgScene.mod"))).unwrap();
        let mut env = vec![("MG_CLIENT_PRELOAD", lib.as_str())];
        if !var.is_empty() {
            env.push((var, "7200"));
        }
        let boot = monotonic_ms() + mono * 1000;
        let shots =
            common::client_screenshots_with(&dir, "MgScene", 2, Duration::from_secs(2), &env)
                .unwrap_or_else(|| {
                    panic!("the client did not reach the scene (see {})", dir.display())
                });
        let timer = debug_int(&shots[0].1, 17);
        let start = (TIMER_START + TIMER_FIRST_DAY * (boot / TIMER_FIRST_DAY)) % day;
        let since = (timer - start).rem_euclid(day);
        eprintln!(
            "{what}: worldtimerTimeOfDay {timer}; the boot clock {boot} ms at the start, its \
             two-hour steps {}: the rule's start {start}, {since} ms before the first picture",
            boot / TIMER_FIRST_DAY
        );
        // (Some seconds after the client starts its services; a run that
        // starts across a two-hour step of the boot clock would be off by
        // one step.)
        assert!((1_000..60_000).contains(&since), "{what}: {timer} is not the rule's {start}");
        seen.push(timer);
    }
    // The wall clock changes nothing; the boot clock moves it a step.
    assert!((seen[1] - seen[0]).abs() < 30_000, "{seen:?}");
    assert!(((seen[2] - seen[0]).rem_euclid(day) - TIMER_FIRST_DAY).abs() < 30_000, "{seen:?}");
}

/// The Domains of Dread's fountain at the client's own phase (the deferred
/// list's M1): the scene of `tdod_custom_shaders_look` with the debug
/// `inc_standard` in the override (the haks replace `inc_lighting` alone),
/// so that each of the client's pictures shows its own
/// `worldtimerTimeOfDay` in the band (the floor's rows 6 to 9 of 32-pixel
/// blocks from the bottom, below the fountain); Moonglow draws the fountain
/// with the game's shaders and water at that clock (`View::time`, seconds),
/// half the falling water's 5-second cycle off it, and at its old 4 s, and
/// each is compared with the client over the fountain's pixels outside the
/// band (the mean difference per channel, of 255).
///
/// Seen (2026-11-02, 89.8193.37; three pictures, the timer 424,388 to
/// 428,504 in a day of 2 minutes an hour: the falling water's cycle at
/// 4.39, 1.47 and 3.50 of its 5 s): over the fountain's 7,091 pixels
/// Moonglow differs from the client by 10.57, 10.54 and 10.57 of 255 at the
/// client's clock, 10.57, 10.53 and 10.56 half a cycle off and 10.60 at
/// 4 s: the phase is not the difference. Moonglow's own pictures half a
/// cycle apart differ by 0.2 to 0.8 there. What differs is the basin's
/// water (11 to 12 of 255 over it, its mean colour within 2): the client
/// shows the basin's floor through it, its pattern and all, where
/// Moonglow's surface is smooth.
#[test]
#[ignore]
fn batch7_fountain_phase() {
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU");
        return;
    };
    let base = super::batch5::base_game();
    let overrides = [("inc_standard.shd", debug_shader(&base).into_bytes())];
    let Some(TdodFountain { dir, game, are, git, empty }) =
        tdod_fountain("client_reading_fountain_phase", &overrides)
    else {
        return;
    };
    let shots = common::client_screenshots(&dir, "MgScene", 3, Duration::from_secs(2))
        .unwrap_or_else(|| panic!("the client did not reach the scene (see {})", dir.display()));
    let tileset = mg_area::tileset(&game, ResRef::from_str("tic01").unwrap()).ok();
    let camera =
        fitted_camera(Vec3::new(20.0, 20.0, 0.0), FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let (w, h) = (shots[0].1.width, shots[0].1.height);
    let draw = |git: &Gff, on: bool, time: f32| {
        let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
        let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
        let view = mg_area::View { fog: false, time, ..mg_area::View::of(&model) };
        let scene = area_scene.scene(&model, &view);
        let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
        r.game_shaders = on;
        r.game_water = on && mg_render::GAME_WATER;
        r.render_image(&gpu, &game.resman, &scene, &camera, w, h)
    };
    // The fountain's pixels (where it changes Moonglow's picture) above the
    // band.
    let (bare, off) = (draw(&empty, false, 4.0), draw(&git, false, 4.0));
    let band = (h as usize - 10 * 32)..(h as usize - 6 * 32);
    let mask: Vec<bool> = (bare.data.chunks(4).zip(off.data.chunks(4)))
        .enumerate()
        .map(|(k, (a, b))| {
            !band.contains(&(k / w as usize)) && (0..3).any(|i| a[i].abs_diff(b[i]) > 6)
        })
        .collect();
    let over = |img: &Rgba, client: &Rgba| {
        let (mut sum, mut n) = (0.0f64, 0.0f64);
        for (k, (a, b)) in img.data.chunks(4).zip(client.data.chunks(4)).enumerate() {
            if mask[k] {
                sum += (0..3).map(|i| f64::from(a[i].abs_diff(b[i]))).sum::<f64>() / 3.0;
                n += 1.0;
            }
        }
        (sum / n.max(1.0), n as usize)
    };
    let old = draw(&git, true, 4.0);
    for (i, (at, client)) in shots.iter().enumerate() {
        save_png(client, &dir.join(format!("client_{i}.png")));
        let timer = debug_int(client, 17);
        let seconds = timer as f32 / 1000.0;
        let matched = draw(&git, true, seconds);
        let opposite = draw(&git, true, seconds + 2.5);
        save_png(&matched, &dir.join(format!("moonglow_matched_{i}.png")));
        save_png(&opposite, &dir.join(format!("moonglow_opposite_{i}.png")));
        let (d_matched, n) = over(&matched, client);
        let (d_opposite, _) = over(&opposite, client);
        let (d_old, _) = over(&old, client);
        eprintln!(
            "shot {i} at {at:.2} s, worldtimerTimeOfDay {timer} (the falling water's cycle at \
             {:.2} of 5 s): over the fountain's {n} pixels Moonglow differs by {d_matched:.2} of \
             255 at the client's clock, {d_opposite:.2} half a cycle off, {d_old:.2} at 4 s",
            (seconds % 5.0)
        );
    }
    save_png(&old, &dir.join("moonglow_4s.png"));
    eprintln!("{}", dir.display());
}

/// The mean of a picture's three channels over a rectangle (fractions of
/// the picture, as [`mean`]).
fn brightness(img: &Rgba, r: [f32; 4]) -> f32 {
    mean(img, r).iter().sum::<f32>() / 3.0
}

/// The city street a low sun leaves lit in the client (the deferred
/// list's "Shadows"): `reading_low_sun_shadow_length`'s `tcn01` area (the
/// same tiles, a sun from the west 14 degrees up, Sun Shadows at 100) as
/// it is, and then each of its tiles but the plain cobbles (`tcn01` tile
/// 44) alone, every other tile made the plain cobbles, each beside
/// Moonglow's area view with its shadows and without: the brightness of
/// the street before the player (the picture's left half, a third to four
/// fifths down) in each. Where the client is as bright as Moonglow without
/// shadows and Moonglow with them is darker, the tile's shadow is one
/// the client leaves out (`MG_TILES`: the tiles' places, comma
/// separated, or `all` alone).
///
/// Seen (2026-11-02, 89.8193.37): all the tiles, the client 56.7,
/// Moonglow with shadows 24.7, without 58.0; alone, two tiles darken
/// Moonglow's street and not the client's, both on the area's west
/// edge, turned to face into it: `tcn01_c02_07` (tile 4: 58.4, 37.9,
/// 58.7) and `tcn01_c02_08` (tile 8: 57.3, 45.6, 58.1); the others change
/// neither. Put in sight (`MG_PLACE=9:199:2`) the first casts only a
/// curb's and a sign's short shadows in the client, its walls none, where
/// Moonglow lays the building's long shadow; turned to face the sun
/// (`MG_PLACE=9:199:0`) it casts the long shadow in the client too. The
/// city's house (tile 54) put out of sight in tile 4's place casts in
/// both (42.7 and 42.2): sight has nothing to do with it. The walls are
/// single faces: the client casts from a face's front only
/// ([`batch7_wall_sides`]).
#[test]
#[ignore]
fn batch7_low_sun_city_tiles() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let probe = Reading::new("low_sun_city_probe", "tcn01", light, &[]);
    let set = probe.game.resman.get_named("tcn01", ResType::SET).unwrap();
    let set = mg_set::Tileset::parse(&set, probe.game.language.codepage()).unwrap();
    let generated: Vec<(i32, i32, i32)> = probe
        .are
        .root
        .list("Tile_List")
        .unwrap()
        .iter()
        .map(|t| {
            let n = |l| t.int(l).unwrap_or(0) as i32;
            (n("Tile_ID"), n("Tile_Orientation"), n("Tile_Height"))
        })
        .collect();
    for (i, (id, turn, height)) in generated.iter().enumerate() {
        let model = set.tiles.get(*id as usize).map_or("?", |t| t.model.as_str());
        eprintln!("tile {i}: {id} ({model}), turned {turn}, height {height}");
    }
    const PLAIN: i32 = 44;
    let mut cases: Vec<Option<usize>> = vec![None];
    cases.extend((0..generated.len()).filter(|i| generated[*i].0 != PLAIN).map(Some));
    if let Ok(only) = std::env::var("MG_TILES") {
        cases = only.split(',').map(|c| c.trim().parse().ok()).collect();
    }
    // (`MG_PLACE=place:tile:turns,…`: a tile of the tileset put alone in
    // another place, turned so many quarters, every other tile plain: where
    // a tile casts from, in sight or out of it.)
    let mut generated = generated;
    if let Ok(places) = std::env::var("MG_PLACE") {
        cases.clear();
        for p in places.split(',') {
            let n: Vec<i32> = p.split(':').filter_map(|v| v.trim().parse().ok()).collect();
            let [place, id, turn] = n[..] else { panic!("MG_PLACE: {p}") };
            generated[place as usize] = (id, turn, 0);
            cases.push(Some(place as usize));
        }
    }
    let generated = generated;
    let street = [0.0, 0.33, 0.5, 0.8];
    for case in cases {
        let name = case.map_or("low_sun_city_all".to_string(), |i| {
            format!("low_sun_city_{i}_{}_{}", generated[i].0, generated[i].1)
        });
        let mut r = Reading::new(&name, "tcn01", light, &[]);
        for (label, v) in [("SunShadows", 1u8), ("MoonShadows", 0), ("ShadowOpacity", 100)] {
            r.are.root.set(label, Value::Byte(v));
        }
        if let Some(keep) = case {
            for (i, t) in r.are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
                let (id, turn, height) = if i == keep { generated[i] } else { (PLAIN, 0, 0) };
                t.set("Tile_ID", Value::Int(id));
                t.set("Tile_Orientation", Value::Int(turn));
                t.set("Tile_Height", Value::Int(height));
            }
        }
        let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else {
            panic!("no AreaProperties")
        };
        for (axis, v) in [("X", -1.0f32), ("Y", 0.0), ("Z", 0.25)] {
            props.set(&format!("SunDirection{axis}"), Value::Float(v));
            props.set(&format!("MoonDirection{axis}"), Value::Float(v));
        }
        let (are, git) = (r.are.clone(), r.git.clone());
        let user = r.dir.join("user");
        let settings = super::shadows::settings();
        let (dir, client) =
            r.shoot(&enter(&super::readings::logged(INVISIBLE), (90.0, 12.0, 45.0)), &settings);
        let game =
            GameData::open(&GameInstall::new(mg_testkit::nwn_root().unwrap(), Some(user), "en"))
                .unwrap();
        let beside = |shadows: bool, name: &str| {
            super::readings::moonglow_beside_as(
                &dir,
                &game,
                &are,
                &git,
                &client,
                &|v| v.shadows = shadows,
                name,
            )
        };
        let (Some((_, shadowed)), Some((_, plain))) =
            (beside(true, "moonglow_shadows"), beside(false, "moonglow_plain"))
        else {
            return;
        };
        let what = case.map_or("all tiles".to_string(), |i| {
            let (id, turn, _) = generated[i];
            format!("tile {i} ({id}, {}, turned {turn}) alone", set.tiles[id as usize].model)
        });
        eprintln!(
            "{what}: the street's brightness: the client {:.1}, Moonglow with shadows {:.1}, \
             without {:.1} ({})",
            brightness(&client, street),
            brightness(&shadowed, street),
            brightness(&plain, street),
            dir.display()
        );
    }
}

/// Which side of a wall of one face casts (what [`batch7_low_sun_city_tiles`]
/// found of the city's walls): a static placeable of two single white
/// walls, 2 m wide and 4 m high, a quad of two faces each, the north one
/// facing west (its front to the low sun from the west), the south one
/// facing east (its back to it), on tcn01's plain cobbles west of the
/// player, as the client and Moonglow's area view (Fast and High) draw
/// them (`client_reading_wall_sides`).
///
/// Seen (2026-11-02, 89.8193.37): the client casts the north wall's
/// shadow (its front to the sun) and none of the south one (its back to
/// it); Moonglow casts both.
#[test]
#[ignore]
fn batch7_wall_sides() {
    let base = super::batch5::base_game();
    let (table, first) = placeables_with(&base, &[("mg_walls", 1)]);
    let (s, h) = (2.0f32, 4.0f32);
    let wall = |name: &str, at: [f32; 3], corners: [[f32; 3]; 4]| Quad {
        name: name.into(),
        at,
        corners,
        bitmap: "mgwhite",
        extra: "",
        tverts: true,
        colors: false,
    };
    let quads = [
        // (Its front to the west: wound so that the face's normal is −X.)
        wall("west", [0.0, 1.0, 0.0], [[0.0, s, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, h], [0.0, s, h]]),
        // (Its front to the east, +X.)
        wall(
            "east",
            [0.0, -3.0, 0.0],
            [[0.0, 0.0, 0.0], [0.0, s, 0.0], [0.0, s, h], [0.0, 0.0, h]],
        ),
    ];
    let walls = model("mg_walls", &quads).replace("  shadow 0\n", "  shadow 1\n");
    let overrides = [
        ("mg_walls.mdl", walls.into_bytes()),
        ("mgwhite.tga", super::white_tga()),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("wall_sides", "tcn01", light, &overrides);
    for (label, v) in [("SunShadows", 1u8), ("MoonShadows", 0), ("ShadowOpacity", 100)] {
        r.are.root.set(label, Value::Byte(v));
    }
    for t in r.are.root.list_mut("Tile_List").unwrap().iter_mut() {
        t.set("Tile_ID", Value::Int(44));
        t.set("Tile_Orientation", Value::Int(0));
        t.set("Tile_Height", Value::Int(0));
    }
    r.place(first, [12.0, 24.0, 0.0], 0.0, &[("Static", Value::Byte(1))]);
    let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else {
        panic!("no AreaProperties")
    };
    for (axis, v) in [("X", -1.0f32), ("Y", 0.0), ("Z", 0.25)] {
        props.set(&format!("SunDirection{axis}"), Value::Float(v));
        props.set(&format!("MoonDirection{axis}"), Value::Float(v));
    }
    let settings = super::shadows::settings();
    let (dir, _) = r.shoot_beside(INVISIBLE, &settings, &|v| v.shadows = true);
    eprintln!("{}", dir.display());
}
