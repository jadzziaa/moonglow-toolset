//! The renderer's checks in the window: the area view's watch on what the
//! GPU draws (`mg_render::watch`), quiet where the area is drawn (an empty
//! area, the game's shaders warming up, Fade Geometry, the sky, a zoom)
//! and failing where nothing is (`mg_render::guard::blank_draws`, as
//! Heed's Radeon drew through Direct3D 12); the notice it raises, Restart
//! with … through the close flow, and the record that Automatic passes
//! over on the next start. Neither builder's GPU is here: their adapters
//! are told to the app as Windows would list them.

use crate::common::*;
use mg_render::watch::{Quiet, Verdict};
use mg_ui::area_view::{Fade, Orbit};

/// Puts the draws back when a test that blanked them ends, even by a
/// panic.
struct Unblank;

impl Drop for Unblank {
    fn drop(&mut self) {
        mg_render::guard::blank_draws(false);
    }
}

struct Unwait;

impl Drop for Unwait {
    fn drop(&mut self) {
        mg_render::watch::wait_for_looks(false);
    }
}

/// A harness of `app` (with an area's tab asked for), a quarter of a
/// second a frame: the watch looks once a second and fails after five,
/// each look read in the frame after it. Callers must hold
/// `mg_testkit::gpu::alone()` before opening the GPU because
/// `wait_for_looks` is process-global.
fn watched(app: Moonglow, rs: egui_wgpu::RenderState) -> (Harness<'static, Moonglow>, Unwait) {
    // (Each look read in the frame after it: the verdicts follow the
    // harness's clock, not the GPU's load.)
    mg_render::watch::wait_for_looks(true);
    let unwait = Unwait;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .with_step_dt(0.25)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    (h, unwait)
}

/// Acquires `mg_testkit::gpu::alone()` before `area_app` opens the GPU, then
/// runs `prepare` and enters `watched`.
fn watched_area(
    name: &str,
    tileset: &str,
    edit: &dyn Fn(&mut mg_module::Module, &mg_rules::GameData, ResRef),
    prepare: impl FnOnce(&mut Moonglow),
) -> Option<(Harness<'static, Moonglow>, Unwait, ResRef)> {
    mg_testkit::gpu::alone();
    let (mut app, rs, area) = area_app(name, tileset, None, 4, None, edit)?;
    prepare(&mut app);
    let (h, unwait) = watched(app, rs);
    Some((h, unwait, area))
}

/// Runs `seconds` of frames, `each` before every frame (given the
/// harness and the time, seconds); the verdict of every look, in order.
fn watch_for(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    seconds: f32,
    mut each: impl FnMut(&mut Harness<'_, Moonglow>, f32),
) -> Vec<Verdict> {
    let mut verdicts = Vec::new();
    let mut looks = h.state().area_views[&area].watching.looks;
    let mut t = 0.0;
    while t < seconds {
        each(h, t);
        h.step();
        let w = &h.state().area_views[&area].watching;
        if w.looks != looks {
            looks = w.looks;
            verdicts.push(w.seen.expect("a look's verdict").1);
        }
        t += 0.25;
    }
    verdicts
}

fn failed(verdicts: &[Verdict]) -> bool {
    verdicts.iter().any(|v| matches!(v, Verdict::Failed(_)))
}

/// Saves the picture of the window in `target/test-output/renderer-watch/`
/// when `MG_SHOT` is set.
fn picture(h: &mut Harness<'_, Moonglow>, name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("MG_SHOT")?;
    let path = mg_testkit::output_dir("renderer-watch").join(format!("{name}.png"));
    h.render().expect("render").save(&path).unwrap();
    Some(path)
}

/// The area drawn (a 4 by 4 rural area, Moonglow's own shader): every
/// look finds it drawn, and the view's orbit zoomed in and out by the
/// wheel's steps all the while changes nothing: each look is judged
/// against its own frame's camera.
#[test]
fn the_watch_stays_quiet_where_the_area_is_drawn_and_while_zooming() {
    let Some((mut h, _unwait, area)) = watched_area("watch-drawn", "ttr01", &|_, _, _| {}, |_| {})
    else {
        return;
    };
    let verdicts = watch_for(&mut h, area, 10.0, |_, _| {});
    eprintln!("drawn: {verdicts:?}");
    assert!(verdicts.len() >= 7, "{verdicts:?}");
    assert!(verdicts[3..].iter().all(|v| *v == Verdict::Quiet(Quiet::Drawn)), "{verdicts:?}");
    picture(&mut h, "drawn");
    // Zooming: in to 2 m and out to 150 m and back, a step a frame.
    let zoom = |h: &mut Harness<'_, Moonglow>, t: f32| {
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        o.distance = 2.0 + 74.0 * (1.0 - (t * 1.3).cos());
    };
    let zooming = watch_for(&mut h, area, 12.0, zoom);
    eprintln!("zooming: {zooming:?}");
    assert!(!failed(&zooming), "{zooming:?}");
    assert!(zooming.contains(&Verdict::Quiet(Quiet::Drawn)), "{zooming:?}");
    picture(&mut h, "zooming");
    assert_eq!(h.state().renderer_check.trouble, None);
}

/// An empty area (its tiles not in the tileset: nothing to draw, as
/// where a tileset's hak is missing) expects nothing, and the watch says
/// nothing, however long.
#[test]
fn the_watch_stays_quiet_on_an_empty_area() {
    let empty = |m: &mut mg_module::Module, _: &mg_rules::GameData, area: ResRef| {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        for t in are.root.list_mut("Tile_List").unwrap() {
            t.set("Tile_ID", mg_gff::Value::Int(9999));
        }
        m.set_gff(key, &are).unwrap();
    };
    let Some((mut h, _unwait, area)) = watched_area("watch-empty", "ttr01", &empty, |_| {}) else {
        return;
    };
    let verdicts = watch_for(&mut h, area, 12.0, |_, _| {});
    eprintln!("empty: {verdicts:?}");
    assert!(verdicts.len() >= 8, "{verdicts:?}");
    assert!(verdicts[3..].iter().all(|v| *v == Verdict::Quiet(Quiet::BelowFloor)), "{verdicts:?}");
    picture(&mut h, "empty");
    assert_eq!(h.state().renderer_check.trouble, None);
}

/// Fade Geometry (Always) on an interior's walled tiles: their walls are
/// left out, their floors drawn, and the watch finds them drawn.
#[test]
fn the_watch_stays_quiet_with_fade_geometry() {
    let walled = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        let set = mg_area::tileset(game, ResRef::from_str("tin01").unwrap()).unwrap();
        let wall = set.tiles.iter().position(|t| t.model.eq_ignore_ascii_case("tin01_a01_01"));
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        for t in are.root.list_mut("Tile_List").unwrap() {
            t.set("Tile_ID", mg_gff::Value::Int(wall.unwrap() as i32));
        }
        m.set_gff(key, &are).unwrap();
    };
    let Some((mut h, _unwait, area)) = watched_area("watch-fade", "tin01", &walled, |app| {
        app.settings.fade_geometry = Fade::Always as u8;
    }) else {
        return;
    };
    assert_eq!(h.state().area_views[&area].fade, Fade::Always);
    // (A view from the side and above, where the walls would stand.)
    {
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        (o.pitch, o.distance) = (0.7, 35.0);
    }
    let verdicts = watch_for(&mut h, area, 10.0, |_, _| {});
    eprintln!("fade: {verdicts:?}");
    assert!(!failed(&verdicts), "{verdicts:?}");
    assert!(verdicts[3..].iter().all(|v| *v == Verdict::Quiet(Quiet::Drawn)), "{verdicts:?}");
    picture(&mut h, "fade_geometry");
}

/// The camera turned to the sky (the orbit at its lowest pitch, lifted
/// 15 m over the ground: the horizon across the middle, the sky above):
/// the sky expects nothing, and little ground is left to expect.
#[test]
fn the_watch_stays_quiet_with_the_camera_to_the_sky() {
    let Some((mut h, _unwait, area)) = watched_area("watch-sky", "ttr01", &|_, _, _| {}, |_| {})
    else {
        return;
    };
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        let o = view.orbit.as_mut().unwrap();
        *o = Orbit { target: glam::Vec3::new(20.0, 20.0, 15.0), pitch: 0.05, distance: 20.0, ..*o };
    }
    let verdicts = watch_for(&mut h, area, 10.0, |_, _| {});
    eprintln!("sky: {verdicts:?}");
    assert!(!failed(&verdicts), "{verdicts:?}");
    let (o, _) = h.state().area_views[&area].watching.seen.unwrap();
    assert!(o.expected < (36 * 64) / 2, "most of the view is sky: {o:?}");
    picture(&mut h, "sky");
}

/// Medieval City 2 builds its ground 5 m above its tiles' height (0): a
/// view brought down near the ground from the overview (whose target is
/// at the tiles' height) has its camera under that ground, where nothing
/// of it is drawn (its underside isn't) and nothing of it is to be
/// expected. The watch expected the ground at the tiles' height, found
/// none of it drawn and failed, blaming the driver (M5 V3). From above
/// the ground, the ground is expected and found drawn.
#[test]
fn the_watch_stays_quiet_with_the_camera_under_a_raised_ground() {
    let Some((mut h, _unwait, area)) = watched_area("watch-raised", "tcm02", &|_, _, _| {}, |_| {})
    else {
        return;
    };
    {
        // Tilted down and zoomed in, as the mouse does, from a target at
        // the tiles' height (as the overview put it before it was put on
        // the ground; as Ctrl + Shift lowers it): the eye 1.3 m up, the
        // ground at 5.
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        (o.target.z, o.pitch, o.distance) = (0.0, 0.16, 8.0);
    }
    let under = watch_for(&mut h, area, 10.0, |_, _| {});
    eprintln!("under: {under:?}");
    picture(&mut h, "under_raised_ground");
    assert!(!failed(&under), "{under:?}");
    assert!(under[3..].iter().all(|v| *v == Verdict::Quiet(Quiet::BelowFloor)), "{under:?}");
    assert_eq!(h.state().renderer_check.trouble, None);
    {
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        (o.target.z, o.pitch, o.distance) = (5.0, 0.7, 30.0);
    }
    let above = watch_for(&mut h, area, 6.0, |_, _| {});
    eprintln!("above: {above:?}");
    assert!(above[2..].iter().all(|v| *v == Verdict::Quiet(Quiet::Drawn)), "{above:?}");
    picture(&mut h, "above_raised_ground");
}

/// The game's shaders warming up (their programs made on the worker, from
/// nothing: no cache) with every draw blanked meanwhile: no look counts
/// while a mesh waits for a pipeline, nor for the first seconds after.
/// Then, drawn by the game's programs, the area is drawn; and blanked once
/// they are (the game's path drawing nothing), the watch fails. (The
/// worker is held back for ten seconds of the harness's clock, then waited
/// for, and each look is read in the frame after it is taken: the
/// verdicts follow that clock alone, not the machine's load. A wall-clock
/// wait failed it once in a run of the whole workspace.)
#[test]
fn the_watch_waits_out_the_game_s_shaders_and_catches_their_path_drawing_nothing() {
    struct Unhold;
    impl Drop for Unhold {
        fn drop(&mut self) {
            mg_render::guard::hold_programs(false);
            mg_render::guard::blank_draws(false);
        }
    }
    // (The programs held back and every draw blanked: no other test
    // renders meanwhile.)
    let _unhold = Unhold;
    let Some((mut h, _unwait, area)) = watched_area("watch-warm", "ttr01", &|_, _, _| {}, |app| {
        app.settings.game_shaders = Some(true);
        mg_render::guard::hold_programs(true);
        mg_render::guard::blank_draws(true);
    }) else {
        return;
    };
    let pending = |h: &Harness<'_, Moonglow>| {
        h.state().viewport.as_ref().unwrap().renderer.drawn.game_pending
    };
    // Waiting for them (for less than `PATIENCE`: slow, not stuck).
    let warming = watch_for(&mut h, area, 10.0, |_, _| {});
    eprintln!("warming up: {warming:?}");
    assert!(pending(&h) > 0, "the programs are held back");
    assert!(warming.len() >= 8, "{warming:?}");
    assert!(warming.iter().all(|v| *v == Verdict::Quiet(Quiet::NotReady)), "{warming:?}");
    picture(&mut h, "warming_up_blanked");
    // Made (the frames ask for what the ones made need: the see-through
    // meshes' second part), and the first looks after warm up.
    mg_render::guard::hold_programs(false);
    for _ in 0..20 {
        h.state_mut().viewport.as_mut().unwrap().renderer.wait_for_game_programs();
        h.step();
        if pending(&h) == 0 {
            break;
        }
    }
    assert_eq!(pending(&h), 0, "the programs are made");
    let warm = watch_for(&mut h, area, 1.5, |_, _| {});
    eprintln!("warm: {warm:?}");
    assert!(warm.contains(&Verdict::Quiet(Quiet::WarmingUp)), "{warm:?}");
    assert!(!failed(&warm), "{warm:?}");
    mg_render::guard::blank_draws(false);
    let drawn = watch_for(&mut h, area, 6.0, |_, _| {});
    eprintln!("the game's shaders: {drawn:?}");
    assert!(h.state().viewport.as_ref().unwrap().renderer.drawn.game > 0, "the game's programs");
    assert!(!failed(&drawn), "{drawn:?}");
    assert_eq!(drawn.last(), Some(&Verdict::Quiet(Quiet::Drawn)));
    picture(&mut h, "game_shaders");
    mg_render::guard::blank_draws(true);
    let blanked = watch_for(&mut h, area, 10.0, |_, _| {});
    eprintln!("the game's shaders blanked: {blanked:?}");
    assert!(failed(&blanked), "{blanked:?}");
    assert!(h.state().renderer_check.trouble.is_some());
}

/// Tiles whose model breaks as it is built (made to fail, as a renderer
/// that fails every model would) are left out of the frame and expected
/// all the same: nothing of the area is drawn, and the watch fails,
/// saying that the tiles failed. (They were once left out of the
/// expectation too: nothing expected, no verdict.)
#[test]
fn tiles_left_out_as_they_failed_are_expected() {
    struct Unfail;
    impl Drop for Unfail {
        fn drop(&mut self) {
            mg_render::guard::fail_on(None);
        }
    }
    // (Every tile's model made to fail: no other test renders meanwhile.)
    let model = std::cell::RefCell::new(String::new());
    let one = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        let set = mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap();
        let (id, tile) = set.tiles.iter().enumerate().find(|(_, t)| !t.model.is_empty()).unwrap();
        *model.borrow_mut() = tile.model.to_ascii_lowercase();
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        for t in are.root.list_mut("Tile_List").unwrap() {
            t.set("Tile_ID", mg_gff::Value::Int(id as i32));
        }
        m.set_gff(key, &are).unwrap();
    };
    let _unfail = Unfail;
    let Some((mut h, _unwait, area)) = watched_area("watch-failed-tiles", "ttr01", &one, |_| {
        mg_render::guard::fail_on(Some(&model.borrow()));
    }) else {
        return;
    };
    let verdicts = watch_for(&mut h, area, 12.0, |_, _| {});
    eprintln!("failed tiles: {verdicts:?}");
    picture(&mut h, "failed_tiles");
    let Some(Verdict::Failed(f)) = verdicts.last().copied() else { panic!("{verdicts:?}") };
    assert!(f.failed > 0 && f.failed == f.expected && f.drawn * 50 < f.expected, "{f:?}");
    let said = h.state().renderer_check.trouble.clone().expect("the notice").said;
    assert!(said.contains("of tiles that failed and were left out"), "{said}");
}

/// Textures that never come (read apart, as the window reads them, and
/// made never to come back): the tiles wait for them, left out, and the
/// frames are never ready. The watch waits, and once nothing has been
/// finished for `PATIENCE` it judges them all the same, and fails, saying
/// what it waited for. (Before, it waited for ever, and said nothing.)
#[test]
fn frames_that_never_get_ready_are_judged_after_a_while() {
    struct Unstall;
    impl Drop for Unstall {
        fn drop(&mut self) {
            mg_render::guard::stall_textures(false);
        }
    }
    use mg_render::watch::PATIENCE;
    // (Every texture held back: no other test renders meanwhile.)
    let _unstall = Unstall;
    let Some((mut h, _unwait, area)) = watched_area("watch-stuck", "ttr01", &|_, _, _| {}, |app| {
        app.background_jobs = true;
        mg_render::guard::stall_textures(true);
    }) else {
        return;
    };
    // (The area read on a thread of its own, as in the desktop app: its
    // first look a few seconds in, and on a busy machine many more frames
    // in. The watch is counted from that look: counted from the start, a
    // first look 18 s of frames in left too few for the failure.)
    let (started, before) = (std::time::Instant::now(), h.state().area_views[&area].watching.looks);
    while h.state().area_views[&area].watching.looks == before {
        h.step();
        assert!(started.elapsed() < std::time::Duration::from_secs(60), "never looked at");
    }
    let mut verdicts = vec![h.state().area_views[&area].watching.seen.expect("a verdict").1];
    verdicts.extend(watch_for(&mut h, area, PATIENCE as f32 + 20.0, |_, _| {}));
    eprintln!("never ready: {verdicts:?}");
    picture(&mut h, "never_ready");
    let pending = h.state().viewport.as_ref().unwrap().renderer.drawn.textures_pending;
    assert!(pending > 0, "the tiles wait for their textures");
    let waited = verdicts.iter().take_while(|v| **v == Verdict::Quiet(Quiet::NotReady)).count();
    assert!(waited as f64 >= PATIENCE - 2.0, "{verdicts:?}");
    let Some(Verdict::Failed(f)) = verdicts.last().copied() else { panic!("{verdicts:?}") };
    assert!(f.waiting.is_some_and(|w| w >= PATIENCE), "{f:?}");
    let said = h.state().renderer_check.trouble.clone().expect("the notice").said;
    assert!(said.contains("none finished for"), "{said}");
}

/// The lit check runs once the game's data is read (where jobs run in the
/// background, as in the desktop app), apart from the window's thread,
/// and finds both Moonglow's own shader and the game's drawing here. Told
/// of a check where the game's programs draw the mesh in the wrong colors
/// (as Heed's Vulkan found them, 2.13.1): Moonglow draws with its own for
/// the rest of the run, every mesh drawn before drawn still and every tile
/// in the picture, the notice says what was found (not that nothing is
/// drawn: the area was) and offers the other backend, and the Shaders
/// button turns the game's on again. Where they draw nothing of it, the
/// notice says that. (The picture was once taken while the tile with a
/// rock still waited for its textures, read apart a mesh at a time: black.)
#[test]
fn the_lit_check_turns_the_game_s_shaders_off_where_they_draw_wrong() {
    let Some((mut h, _unwait, area)) = watched_area("lit-check", "ttr01", &|_, _, _| {}, |app| {
        app.settings.game_shaders = Some(true);
        app.background_jobs = true;
        app.renderer_check.adapter = Some(heeds(wgpu::Backend::Dx12));
        app.renderer_check.other = Some(wgpu::Backend::Vulkan);
    }) else {
        return;
    };
    let started = std::time::Instant::now();
    while h.state().lit_check.found.is_none() {
        assert!(started.elapsed().as_secs() < 60, "the lit check never ended");
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.step();
    }
    let lit = h.state().lit_check.found.clone().unwrap();
    eprintln!("lit check: {lit}");
    // (The area's textures read apart and the game's programs made on
    // their threads, as in the desktop app: the frames until they are.
    // The lit check can end first, and a tile still waiting for its
    // textures is left out of the picture.)
    let drawn = |h: &Harness<'_, Moonglow>| h.state().viewport.as_ref().unwrap().renderer.drawn;
    let started = std::time::Instant::now();
    while drawn(&h).textures_pending > 0 || drawn(&h).game_pending > 0 {
        assert!(started.elapsed().as_secs() < 30, "never ready: {:?}", drawn(&h));
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.step();
    }
    let meshes = drawn(&h).meshes;
    assert_eq!((&lit.own, &lit.game), (&Ok(()), &Some(Ok(()))), "{lit}");
    assert!(h.state().game_shaders());
    assert_eq!(h.state().renderer_check.trouble, None);
    // The game's programs drawing the mesh in the wrong colors (Heed's
    // Vulkan, 2.13.1: washed-out pink where red is drawn here): the area
    // is drawn, so the notice must not say nothing is.
    let pink = mg_render::probe::Lit {
        game: Some(Err("the lit mesh is [212, 133, 133] at (24, 24), not red".into())),
        game_color: Some([212, 133, 133]),
        ..lit.clone()
    };
    mg_ui::renderer_check::lit_found(h.state_mut(), pink);
    h.run_steps(2);
    let app = h.state();
    assert!(!app.game_shaders(), "Moonglow's own for the rest of the run");
    assert!(app.settings.game_shaders(), "the builder's setting is kept");
    assert!(!app.viewport.as_ref().unwrap().renderer.game_shaders);
    let trouble = app.renderer_check.trouble.clone().expect("the notice");
    assert!(
        trouble.said.contains(
            "the game's shaders draw the lit, textured test mesh in the wrong colors through \
             Direct3D 12 on Radeon RX 580 Series (the lit mesh is [212, 133, 133]"
        ),
        "{trouble:?}"
    );
    assert!(trouble.said.contains("Moonglow draws with its own shaders"), "{trouble:?}");
    assert!(!trouble.said.contains("nothing"), "{trouble:?}");
    assert_eq!(trouble.offer, Some(wgpu::Backend::Vulkan));
    // Every mesh drawn with the game's programs drawn with Moonglow's own,
    // and every tile in the picture (at its middle, a little square).
    let shot = h.render().expect("render");
    let shot_label = if std::env::var_os("MG_SHOT").is_some() {
        let path = mg_testkit::output_dir("renderer-watch").join("lit_check_game_off.png");
        shot.save(&path).unwrap();
        path.display().to_string()
    } else {
        "lit_check_game_off".to_owned()
    };
    let now = drawn(&h);
    assert_eq!((now.meshes, now.textures_pending, now.game), (meshes, 0, 0), "{now:?}");
    let ppp = h.ctx.pixels_per_point();
    for (x, y) in (0..4).flat_map(|y| (0..4).map(move |x| (x, y))) {
        let middle = glam::Vec3::new(x as f32 * 10.0 + 5.0, y as f32 * 10.0 + 5.0, 0.0);
        let at = screen(&h, area, middle);
        let (cx, cy) = ((at.x * ppp) as u32, (at.y * ppp) as u32);
        let mut sum = 0.0;
        for (px, py) in (0..9).flat_map(|j| (0..9).map(move |i| (cx + i - 4, cy + j - 4))) {
            let p = shot.get_pixel(px, py);
            sum += 0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2]);
        }
        let luma = sum / 81.0;
        assert!(luma > 30.0, "tile ({x}, {y}) is black ({luma:.0}) in {shot_label}");
    }
    // Drawing nothing of it: said as such.
    let wrong = mg_render::probe::Lit {
        game: Some(Err("the lit mesh is [0, 0, 0] at (24, 24), not red".into())),
        game_color: Some([0, 0, 0]),
        ..lit
    };
    mg_ui::renderer_check::lit_found(h.state_mut(), wrong);
    let trouble = h.state().renderer_check.trouble.clone().expect("the notice");
    assert!(
        trouble.said.contains("the game's shaders draw nothing of the lit, textured test mesh"),
        "{trouble:?}"
    );
    // The Shaders button, off and on: the game's again.
    h.state_mut().set_game_shaders(false);
    h.state_mut().set_game_shaders(true);
    h.run_steps(2);
    assert!(h.state().game_shaders());
    assert!(h.state().viewport.as_ref().unwrap().renderer.game_shaders);
}

/// Philos's case (November 2026: a Radeon RX 480 that has to draw through
/// Vulkan, where the game's programs draw the lit mesh pink as Heed's RX
/// 580 did, on every start): the notice offers Don't Show Again. Hidden,
/// it is closed and remembered for this GPU, driver and backend; on the
/// next start the check finds the same, Moonglow draws with its own
/// shaders as before, and the log alone says so (as news, not a warning,
/// naming where to show the notice again); Options › General's Show Again
/// shows it again. The notices of Moonglow's own drawing wrong don't offer
/// it.
#[test]
fn the_game_s_shaders_notice_can_be_hidden_on_this_driver() {
    let mut info = heeds(wgpu::Backend::Vulkan);
    (info.device, info.name) = (0x67df, "Radeon (TM) RX 480 Graphics".into());
    let pink = mg_render::probe::Lit {
        backend: wgpu::Backend::Vulkan,
        adapter: info.name.clone(),
        own: Ok(()),
        game: Some(Err("the lit mesh is [212, 133, 133] at (24, 24), not red".into())),
        fell_back: None,
        panicked: false,
        own_time: Default::default(),
        game_time: Default::default(),
        own_picture: None,
        game_picture: None,
        own_color: Some([204, 56, 56]),
        game_color: Some([212, 133, 133]),
        overridden: Vec::new(),
    };
    let start = |settings: mg_ui::Settings| {
        let mut app = app_with(Vec::new());
        app.settings = settings;
        app.settings.game_shaders = Some(true);
        let check = &mut app.renderer_check;
        (check.adapter, check.asked_for, check.other) =
            (Some(info.clone()), true, Some(wgpu::Backend::Dx12));
        mg_ui::renderer_check::lit_found(&mut app, pink.clone());
        let mut h = harness(app, 1270.0, 700.0);
        h.run_steps(2);
        h
    };
    let mut h = start(mg_ui::Settings::default());
    let trouble = h.state().renderer_check.trouble.clone().expect("the notice");
    assert!(trouble.hideable, "{trouble:?}");
    assert!(!h.state().game_shaders());
    // (Pictures to look at, where there is a GPU.)
    if std::env::var_os("MG_SHOT").is_some() && adapter() {
        mg_testkit::gpu::hold();
        if let Some(p) = picture(&mut h, "notice_hideable") {
            eprintln!("{}", p.display());
        }
    }
    h.get_by_label("Don't Show Again").click();
    h.run_steps(2);
    let app = h.state();
    assert_eq!(app.renderer_check.trouble, None);
    let hidden = &app.settings.graphics_notices_hidden;
    assert_eq!(hidden.len(), 1, "{hidden:?}");
    assert_eq!(
        (hidden[0].backend.as_str(), hidden[0].adapter.as_str()),
        ("Vulkan", "Radeon (TM) RX 480 Graphics")
    );
    assert!(app.settings.graphics_failures.is_empty(), "no backend passed over");
    assert!(!app.game_shaders(), "Moonglow's own still");

    // The next start: the settings as saved; the same found, said in the log alone.
    let saved: mg_ui::Settings = ron::from_str(&ron::to_string(&app.settings).unwrap()).unwrap();
    let mut h = start(saved);
    let app = h.state();
    assert_eq!(app.renderer_check.trouble, None, "hidden");
    assert!(!app.game_shaders(), "Moonglow's own for the run");
    assert!(app.settings.game_shaders(), "the builder's setting is kept");
    let (level, said) = app.log.entries.last().cloned().expect("the log's line");
    assert_eq!(level, mg_ui::Level::Info, "{said}");
    assert!(said.contains("the game's shaders draw the lit, textured test mesh"), "{said}");
    assert!(said.contains("Options › General shows it again"), "{said}");

    // Options › General › Show Again, then OK: shown again from the next start.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run_steps(2);
    h.get_by_label("General").click();
    h.run_steps(2);
    if std::env::var_os("MG_SHOT").is_some() && adapter() {
        mg_testkit::gpu::hold();
        if let Some(p) = picture(&mut h, "notice_hidden_options") {
            eprintln!("{}", p.display());
        }
    }
    // (Under Appearance, below the page's first sections.)
    // (Scrolling is animated: run until it is there.)
    h.get_by_label("Show Again").scroll_to_me();
    h.run();
    h.get_by_label("Show Again").click();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(2);
    let settings = h.state().settings.clone();
    assert!(settings.graphics_notices_hidden.is_empty());
    let h = start(settings);
    assert!(h.state().renderer_check.trouble.as_ref().is_some_and(|t| t.hideable));

    // Moonglow's own shader drawing wrong: no Don't Show Again.
    let own = mg_render::probe::Lit { own: Err("the lit mesh is black".into()), ..pink.clone() };
    let mut app = app_with(Vec::new());
    app.renderer_check.adapter = Some(info.clone());
    mg_ui::renderer_check::lit_found(&mut app, own);
    assert!(!app.renderer_check.trouble.unwrap().hideable);
}

/// The adapter Heed's machine draws with on Automatic, as Windows lists
/// it, and its other backend: Moonglow can't run Direct3D 12 here, so the
/// app is told this one (the picture is Vulkan's, blanked).
fn heeds(backend: wgpu::Backend) -> wgpu::AdapterInfo {
    let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, backend);
    (info.vendor, info.device) = (0x1002, 0x67df);
    info.name = "Radeon RX 580 Series".into();
    info.driver_info = "25.9.1".into();
    info
}

/// Heed's case, by injection: the area drawn, then nothing drawn (every
/// draw blanked) for long enough. The watch fails; the notice names the
/// adapter and the backend and offers the other one; Restart with Vulkan
/// quits (nothing unsaved: no question) to start again with the module,
/// the failure recorded; and on the next start, Automatic passes over
/// Direct3D 12 on this GPU and driver and takes Vulkan.
#[test]
fn nothing_drawn_raises_the_notice_and_the_next_start_passes_the_backend_over() {
    let _unblank = Unblank;
    let Some((mut h, _unwait, area)) = watched_area("watch-heed", "ttr01", &|_, _, _| {}, |app| {
        app.renderer_check.adapter = Some(heeds(wgpu::Backend::Dx12));
        app.renderer_check.other = Some(wgpu::Backend::Vulkan);
    }) else {
        return;
    };
    let drawn = watch_for(&mut h, area, 4.0, |_, _| {});
    assert!(!failed(&drawn), "{drawn:?}");
    picture(&mut h, "before_blank");
    mg_render::guard::blank_draws(true);
    let blanked = watch_for(&mut h, area, 12.0, |_, _| {});
    eprintln!("blanked: {blanked:?}");
    let first = blanked.iter().position(|v| matches!(v, Verdict::Failed(_))).expect("a failure");
    assert!(first >= 4, "not before the collapse lasted: {blanked:?}");
    let said = h.state().renderer_check.trouble.clone().expect("the notice").said;
    assert!(said.contains("Direct3D 12 on Radeon RX 580 Series"), "{said}");
    h.run_steps(2);
    if let Some(shot) = picture(&mut h, "notice") {
        eprintln!("{}", shot.display());
    }
    let restart = h.get_by_label("Restart with Vulkan");
    restart.click();
    h.run_steps(3);
    mg_render::guard::blank_draws(false);
    let app = h.state();
    assert!(app.quit_requested, "quit, nothing unsaved to ask about");
    let again = app.relaunch.clone().expect("to start again");
    assert_eq!(again.backend, wgpu::Backend::Vulkan);
    assert_eq!(again.module, app.module_path());
    assert!(again.module.is_some());
    let recorded = &app.settings.graphics_failures;
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(
        (recorded[0].backend.as_str(), recorded[0].driver.as_str()),
        ("Direct3D 12", "25.9.1")
    );
    assert!(recorded[0].what.starts_with("the area view's watch: "), "{}", recorded[0].what);
    // The next start: the settings as eframe keeps them, read before the
    // window opens; Windows' list for the GPU; Automatic.
    let dir = mg_testkit::scratch_dir("watch-heed-next-start");
    let file = dir.join("app.ron");
    let kv = std::collections::HashMap::from([(
        "moonglow-settings".to_string(),
        ron::to_string(&app.settings).unwrap(),
    )]);
    std::fs::write(&file, ron::to_string(&kv).unwrap()).unwrap();
    let saved = mg_ui::Settings::read_eframe_file(&file, "moonglow-settings").unwrap();
    let list = [heeds(wgpu::Backend::Vulkan), heeds(wgpu::Backend::Dx12)];
    let power = wgpu::PowerPreference::HighPerformance;
    let windows = mg_ui::graphics::Platform::Windows;
    let pick =
        mg_ui::graphics::pick_adapter(&list, None, false, power, windows, &saved.failed_backends());
    let pick = pick.unwrap();
    assert_eq!(list[pick.index].backend, wgpu::Backend::Vulkan);
    assert_eq!(pick.passed_over, [wgpu::Backend::Dx12]);
    // (Without the record: Direct3D 12, Automatic's for AMD on Windows.)
    let before = mg_ui::graphics::pick_adapter(&list, None, false, power, windows, &[]).unwrap();
    assert_eq!(list[before.index].backend, wgpu::Backend::Dx12);
}

/// A backend chosen in Options is kept (the notice says so) and Restart
/// with … changes the choice there, recording nothing; with unsaved work
/// it asks first, as closing the window does.
#[test]
fn restart_with_asks_about_unsaved_work_and_changes_a_chosen_backend() {
    let dir = mg_testkit::scratch_dir("ui-restart-graphics");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.settings.graphics_backend = mg_ui::settings::GraphicsBackend::Direct3d12;
    let check = &mut app.renderer_check;
    (check.adapter, check.asked_for, check.other) =
        (Some(heeds(wgpu::Backend::Dx12)), true, Some(wgpu::Backend::Vulkan));
    check.startup_failed("the solid mesh is [0, 0, 0] at (8, 18)", &[]);
    set_tag(&mut app, "changed");
    let mut h = harness(app, 1100.0, 800.0);
    h.run_steps(2);
    let said = h.state().renderer_check.trouble.clone().unwrap().said;
    assert!(
        said.starts_with("Moonglow's startup check found that Direct3D 12 on Radeon"),
        "{said}"
    );
    h.get_by_label("Restart with Vulkan").click();
    h.run();
    assert!(
        matches!(h.state().confirm_discard, Some(mg_ui::Action::RestartGraphics(_))),
        "asked about the unsaved work first"
    );
    assert!(!h.state().quit_requested && h.state().relaunch.is_none());
    h.get_by_label("Don't Save").click();
    h.run();
    let app = h.state();
    assert!(app.quit_requested);
    assert_eq!(app.relaunch.as_ref().map(|r| r.backend), Some(wgpu::Backend::Vulkan));
    assert_eq!(app.settings.graphics_backend, mg_ui::settings::GraphicsBackend::Vulkan);
    assert!(app.settings.graphics_failures.is_empty(), "a choice is changed, not recorded");
}
