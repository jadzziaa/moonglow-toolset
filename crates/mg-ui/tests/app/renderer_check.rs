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

/// A harness of `app` (with an area's tab asked for), a quarter of a
/// second a frame: the watch looks once a second and fails after five,
/// each look read in the frame after it.
fn watched(app: Moonglow, rs: egui_wgpu::RenderState) -> Harness<'static, Moonglow> {
    mg_testkit::gpu::hold();
    // (Each look read in the frame after it: the verdicts follow the
    // harness's clock, not the GPU's load.)
    mg_render::watch::wait_for_looks(true);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .with_step_dt(0.25)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h
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

/// The picture of the window, in `target/test-output/renderer-watch/`.
fn picture(h: &mut Harness<'_, Moonglow>, name: &str) -> std::path::PathBuf {
    let path = mg_testkit::output_dir("renderer-watch").join(format!("{name}.png"));
    h.render().expect("render").save(&path).unwrap();
    path
}

/// The area drawn (a 4 by 4 rural area, Moonglow's own shader): every
/// look finds it drawn, and the view's orbit zoomed in and out by the
/// wheel's steps all the while changes nothing: each look is judged
/// against its own frame's camera.
#[test]
fn the_watch_stays_quiet_where_the_area_is_drawn_and_while_zooming() {
    let Some((app, rs, area)) = area_app("watch-drawn", "ttr01", None, 4, None, &|_, _, _| {})
    else {
        return;
    };
    let mut h = watched(app, rs);
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
    let Some((app, rs, area)) = area_app("watch-empty", "ttr01", None, 4, None, &empty) else {
        return;
    };
    let mut h = watched(app, rs);
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
    let Some((mut app, rs, area)) = area_app("watch-fade", "tin01", None, 4, None, &walled) else {
        return;
    };
    app.settings.fade_geometry = Fade::Always as u8;
    let mut h = watched(app, rs);
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
    let Some((app, rs, area)) = area_app("watch-sky", "ttr01", None, 4, None, &|_, _, _| {}) else {
        return;
    };
    let mut h = watched(app, rs);
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
    mg_testkit::gpu::alone();
    let Some((mut app, rs, area)) = area_app("watch-warm", "ttr01", None, 4, None, &|_, _, _| {})
    else {
        return;
    };
    app.settings.game_shaders = Some(true);
    let _unhold = Unhold;
    mg_render::guard::hold_programs(true);
    mg_render::guard::blank_draws(true);
    let mut h = watched(app, rs);
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
    mg_testkit::gpu::alone();
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
    let Some((app, rs, area)) = area_app("watch-failed-tiles", "ttr01", None, 4, None, &one) else {
        return;
    };
    let _unfail = Unfail;
    mg_render::guard::fail_on(Some(&model.borrow()));
    let mut h = watched(app, rs);
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
    mg_testkit::gpu::alone();
    let Some((mut app, rs, area)) = area_app("watch-stuck", "ttr01", None, 4, None, &|_, _, _| {})
    else {
        return;
    };
    app.background_jobs = true;
    let _unstall = Unstall;
    mg_render::guard::stall_textures(true);
    let mut h = watched(app, rs);
    // (The area read on a thread of its own, as in the desktop app: its
    // first look a few seconds in.)
    let verdicts = watch_for(&mut h, area, PATIENCE as f32 + 20.0, |_, _| {});
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
/// the rest of the run, the notice says what was found (not that nothing
/// is drawn: the area was) and offers the other backend, and the Shaders
/// button turns the game's on again. Where they draw nothing of it, the
/// notice says that.
#[test]
fn the_lit_check_turns_the_game_s_shaders_off_where_they_draw_wrong() {
    let Some((mut app, rs, _area)) = area_app("lit-check", "ttr01", None, 4, None, &|_, _, _| {})
    else {
        return;
    };
    app.settings.game_shaders = Some(true);
    app.background_jobs = true;
    app.renderer_check.adapter = Some(heeds(wgpu::Backend::Dx12));
    app.renderer_check.other = Some(wgpu::Backend::Vulkan);
    let mut h = watched(app, rs);
    let started = std::time::Instant::now();
    while h.state().lit_check.found.is_none() {
        assert!(started.elapsed().as_secs() < 60, "the lit check never ended");
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.step();
    }
    let lit = h.state().lit_check.found.clone().unwrap();
    eprintln!("lit check: {lit}");
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
    picture(&mut h, "lit_check_game_off");
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
    let Some((mut app, rs, area)) = area_app("watch-heed", "ttr01", None, 4, None, &|_, _, _| {})
    else {
        return;
    };
    app.renderer_check.adapter = Some(heeds(wgpu::Backend::Dx12));
    app.renderer_check.other = Some(wgpu::Backend::Vulkan);
    let mut h = watched(app, rs);
    mg_testkit::gpu::alone();
    let _unblank = Unblank;
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
    let shot = picture(&mut h, "notice");
    eprintln!("{}", shot.display());
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
