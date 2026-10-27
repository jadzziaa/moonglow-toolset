//! What the views draw: the area view's lighting, shaders and grass, the
//! model viewer and the previews.

use crate::common::*;

#[test]
fn model_viewer_plays_animations() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    // (Moonglow's own shader: the view asks for frames while the game's
    // programs are made, which `run` would wait out.)
    app.settings.game_shaders = Some(false);
    let key = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = gpu_harness(app, rs, 900.0, 700.0);
    h.run();
    h.run();
    h.get_by_label_contains(" nodes, ");
    // Choose "open" from the animation list.
    h.get_by_label("Animation").click();
    h.run();
    h.get_by_label("open").click();
    // A playing animation keeps repainting: step frames.
    h.run_steps(3);
    assert_eq!(
        h.state().model_views[&mg_ui::model_view::Source::Resource(key)].animation.as_deref(),
        Some("open")
    );
    let img = h.render().expect("render");
    assert!(img.width() > 0);
}

/// The model view's Shaders switch is the area view's (one setting for
/// every 3D view): on by default, the game's lit shaders drawing the model
/// once its programs are made; off, Moonglow's own; on again.
#[test]
fn the_model_view_s_shaders_switch_draws_with_the_game_s_shaders() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    let key = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = gpu_harness(app, rs, 900.0, 700.0);
    h.run_steps(1);
    assert_eq!(h.state().settings.game_shaders, None);
    assert!(h.state().settings.game_shaders());
    let drawn = |h: &Harness<'_, Moonglow>| h.state().viewport.as_ref().unwrap().renderer.drawn;
    // (Its programs made while the view goes on: Moonglow's shader first.)
    assert!(drawn(&h).game_pending > 0 || drawn(&h).game > 0, "{:?}", drawn(&h));
    until_the_game_draws(&mut h);
    let vp = h.state().viewport.as_ref().unwrap();
    assert_eq!(vp.renderer.game_shader_failures(), &[]);
    assert!(drawn(&h).game > 0, "{:?}", drawn(&h));
    h.get_by_label("Shaders").click();
    h.run_steps(3);
    assert_eq!(h.state().settings.game_shaders, Some(false));
    assert_eq!(drawn(&h).game, 0);
    h.get_by_label("Shaders").click();
    until_the_game_draws(&mut h);
    assert_eq!(h.state().settings.game_shaders, Some(true));
    assert!(drawn(&h).game > 0, "{:?}", drawn(&h));
}

#[test]
fn blueprint_previews_open() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    // A gnome with robe, cloak and crossbow: its skeleton, standing idle.
    let key = ResKey::parse("nw_hen_bod_05", ResType::UTC).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = gpu_harness(app, rs, 900.0, 700.0);
    // The idle animation plays from the start: step frames.
    h.run_steps(4);
    h.get_by_label_contains("pmg0: ");
    assert_eq!(
        h.state().model_views[&mg_ui::model_view::Source::Resource(key)].animation.as_deref(),
        Some("pause1")
    );
    let img = h.render().expect("render");
    assert!(img.width() > 0);
}

/// The mean colour of the picture `image` in a square `half` points each
/// way around `at` (points).
fn mean_around(image: &image::RgbaImage, ppp: f32, at: egui::Pos2, half: f32) -> [f32; 3] {
    let (mut sum, mut n) = ([0f32; 3], 0f32);
    let (x0, y0) = (((at.x - half) * ppp) as u32, ((at.y - half) * ppp) as u32);
    let (x1, y1) = (((at.x + half) * ppp) as u32, ((at.y + half) * ppp) as u32);
    for y in y0..y1.min(image.height()) {
        for x in x0..x1.min(image.width()) {
            let p = image.get_pixel(x, y);
            for c in 0..3 {
                sum[c] += f32::from(p[c]);
            }
            n += 1.0;
        }
    }
    sum.map(|s| s / n.max(1.0))
}

/// The area view lights an area with the light nodes of its objects'
/// models (Troubletcat's and Heed's reports: a builder's lamps, lit in
/// Aurora and the game, were dark in the area view), and **Illuminated**
/// off takes away a placeable's appearance light (Jasperre's report), its
/// model's light nodes staying, as in the client
/// (`model_light_uniforms` in `client_render.rs`). In an area with no
/// light of its own (black sun and moon, every tile light off): an
/// armoire whose model is authored with a green light node over it, and
/// the game's "Light, White" placeable (placeables.2da `LightColor` 1).
/// The pictures go to `target/test-output/ui-area-lamps-shots/`.
#[test]
fn the_area_view_is_lit_by_objects_lights_and_illumination() {
    let user = mg_testkit::scratch_dir("ui-area-lamps-user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    std::fs::write(
        user.join("override/plc_a01.mdl"),
        "newmodel plc_a01\nsetsupermodel plc_a01 NULL\nclassification placeable\n\
         beginmodelgeom plc_a01\nnode dummy plc_a01\n  parent NULL\nendnode\n\
         node trimesh square\n  parent plc_a01\n  ambient 1 1 1\n  diffuse 1 1 1\n  \
         bitmap NULL\n  verts 3\n    0 0 0.1\n    0.3 0 0.1\n    0 0.3 0.1\n  faces 1\n    \
         0 1 2 1 0 0 0 0\nendnode\nnode light lamp\n  parent plc_a01\n  position 0 0 1\n  \
         color 0 1 0\n  radius 4\n  multiplier 1\n  lightpriority 2\nendnode\n\
         endmodelgeom plc_a01\ndonemodel plc_a01\n",
    )
    .unwrap();
    let prepare = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::instances::{Placement, Placing, instance};
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        for label in ["SunAmbientColor", "SunDiffuseColor", "MoonAmbientColor", "MoonDiffuseColor"]
        {
            are.root.set(label, mg_gff::Value::Dword(0));
        }
        for t in are.root.list_mut("Tile_List").unwrap() {
            for label in ["Tile_MainLight1", "Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2"]
            {
                t.set(label, mg_gff::Value::Byte(0));
            }
        }
        m.set_gff(key, &are).unwrap();
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let placed = [(0, [15.0, 15.0, 0.0]), (15112, [25.0, 25.0, 1.0])].map(|(row, position)| {
            let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Lamp", 0);
            bp.root.set("Appearance", mg_gff::Value::Dword(row));
            bp.root.set("Static", mg_gff::Value::Byte(1));
            let at = Placement { position, rotation: 0.0 };
            instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap()
        });
        git.root.set("Placeable List", mg_gff::Value::List(placed.to_vec()));
        m.set_gff(git_key, &git).unwrap();
    };
    let Some((mut h, area)) = area_harness_with("lamps", "tic01", None, 4, Some(user), &prepare)
    else {
        return;
    };
    // (Close over the room, from above.)
    if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
        o.target = glam::Vec3::new(20.0, 20.0, 0.0);
        (o.distance, o.pitch, o.yaw) = (26.0, 1.4, (-90f32).to_radians());
    }
    h.run_steps(10);
    let dir = mg_testkit::scratch_dir("ui-area-lamps-shots");
    let ppp = h.ctx.pixels_per_point();
    // The floor beside the lamp, and beside the light (clear of its marker).
    let at_lamp = screen(&h, area, glam::Vec3::new(15.0, 15.0, 0.0));
    let at_light = screen(&h, area, glam::Vec3::new(23.0, 25.0, 0.0));
    let around = |img: &image::RgbaImage, at| mean_around(img, ppp, at, 12.0);
    let lit = h.render().unwrap();
    lit.save(dir.join("lit.png")).unwrap();
    let (green, white) = (around(&lit, at_lamp), around(&lit, at_light));
    eprintln!("lit: by the lamp {green:?}, by the light {white:?}");
    let greener = |c: [f32; 3]| c[1] > 2.0 * c[0].max(c[2]) && c[1] > c[0].max(c[2]) + 4.0;
    assert!(greener(green), "green by the lamp");
    assert!(white.iter().all(|&c| c > 8.0), "lit by the light: {white:?}");
    // Illuminated off: the light's placeable lights nothing, the lamp's
    // light node still does.
    let off = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::GIT),
        path: mg_edit::GffPath::root().item("Placeable List", 1),
        label: "LightState".into(),
        value: Some(mg_gff::Value::Byte(0)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Light", vec![off])));
    h.run_steps(10);
    let unlit = h.render().unwrap();
    unlit.save(dir.join("illuminated-off.png")).unwrap();
    let (green_after, white_after) = (around(&unlit, at_lamp), around(&unlit, at_light));
    eprintln!("Illuminated off: by the lamp {green_after:?}, by the light {white_after:?}");
    assert!(white_after[0] < white[0] / 2.0, "dark: {white_after:?}");
    assert!(greener(green_after), "the lamp's light node stays on");
    // With the Lighting switch off, the even working light: no green.
    h.state_mut().area_views.get_mut(&area).unwrap().lit = false;
    h.run_steps(10);
    let working = h.render().unwrap();
    working.save(dir.join("lighting-off.png")).unwrap();
    let g = around(&working, at_lamp);
    assert!(!greener(g), "no lamp with the lighting off: {g:?}");
}

/// Opens `area` again as the desktop app opens one (the window kept going:
/// read on a thread of its own, its textures apart, read anew), and runs
/// frames until it is drawn whole: whether a frame went by before it was
/// read.
fn open_apart(h: &mut Harness<'_, Moonglow>, area: ResRef) -> bool {
    h.state_mut().area_views.remove(&area);
    if let Some(vp) = h.state_mut().viewport.as_mut() {
        vp.renderer.clear_textures();
    }
    h.state_mut().background_jobs = true;
    h.run_steps(1);
    let waited = h.state().area_views.get(&area).is_some_and(|v| v.model.is_none());
    let started = std::time::Instant::now();
    loop {
        let app = h.state();
        let view = &app.area_views[&area];
        let pending = app.viewport.as_ref().map_or(0, |v| v.renderer.drawn.textures_pending);
        if view.model.is_some() && view.pictures > 0 && pending == 0 {
            break;
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(30), "not read");
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.run_steps(1);
    }
    // (Drawn again at the animations' next step.)
    h.run_steps(6);
    h.state_mut().background_jobs = false;
    waited
}

/// The area view's part of the window's picture.
fn view_picture(h: &mut Harness<'_, Moonglow>, area: ResRef) -> image::RgbaImage {
    h.run_steps(1);
    let rect = h.state().area_views[&area].rect;
    let img = h.render().expect("render");
    let ppp = h.ctx.pixels_per_point();
    let (x, y) = ((rect.min.x * ppp) as u32, (rect.min.y * ppp) as u32);
    let (w, hh) = ((rect.width() * ppp) as u32, (rect.height() * ppp) as u32);
    image::imageops::crop_imm(&img, x, y, w.min(img.width() - x), hh.min(img.height() - y))
        .to_image()
}

/// An area opened as the desktop app opens one is read on a thread of its
/// own and its textures apart (a large area's held the window for seconds):
/// frames go on meanwhile, and once read it looks as one read in the frame.
/// A model that fails as it is read there is left out and said so, as on
/// the window's thread (`mg_render::guard`).
#[test]
fn an_area_read_apart_looks_as_one_read_in_the_frame() {
    let Some((mut h, area)) = area_harness("read-apart") else { return };
    h.run_steps(6);
    let here = view_picture(&mut h, area);
    assert!(open_apart(&mut h, area), "a frame went by while the area was read");
    let apart = view_picture(&mut h, area);
    assert_eq!(here.dimensions(), apart.dimensions());
    let differ = here.pixels().zip(apart.pixels()).filter(|(a, b)| a != b).count();
    assert_eq!(differ, 0, "{differ} pixels differ");

    // A tile's model failing as it is read: left out, the area drawn.
    let tile = h.state().area_views[&area].model.as_ref().unwrap().tiles[0]
        .model
        .clone()
        .expect("a tile model");
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            mg_render::guard::fail_on(None);
        }
    }
    // (No other test renders while a model fails on purpose.)
    mg_testkit::gpu::alone();
    let _reset = Reset;
    mg_render::guard::fail_on(Some(&tile));
    open_apart(&mut h, area);
    // (Told in the log, as the window takes the failures.)
    let told = |h: &Harness<'_, Moonglow>| {
        h.state().log.entries.iter().any(|(_, m)| m.contains("left out") && m.contains(&tile))
    };
    assert!(told(&h), "{:?}", h.state().log.entries);
    let failed = view_picture(&mut h, area);
    let differ = here.pixels().zip(failed.pixels()).filter(|(a, b)| a != b).count();
    assert!(differ > 0, "the tile is left out");
    assert!(h.state().area_views[&area].error.is_none());
}

/// Another module with an area of the same name shows its own area, not
/// the one of the module open before (Jasperre's report: both modules'
/// `area001`, the first's shown and edited in the second, until Moonglow
/// was closed). The first module's area has two waypoints, the second's
/// none; neither module is changed by opening the other.
#[test]
fn another_module_shows_its_own_area_of_the_same_name() {
    let Some((mut h, area)) = area_harness("modules-apart") else { return };
    let waypoints = |h: &Harness<'_, Moonglow>| {
        let view = &h.state().area_views[&area];
        view.model.as_ref().unwrap().objects.len()
    };
    assert_eq!(waypoints(&h), 2);
    let ws = h.state().ws.as_ref().unwrap();
    let first = ws.module.location.as_ref().unwrap().path().to_path_buf();
    let mut m = mg_module::Module::open(&first).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set("WaypointList", mg_gff::Value::List(Vec::new()));
    m.set_gff(git_key, &git).unwrap();
    let second = mg_testkit::scratch_dir("ui-area-modules-apart-2").join("area.mod");
    m.save_as(&ModuleLocation::Archive(second.clone())).unwrap();
    let before = std::fs::read(&first).unwrap();

    h.state_mut().open_module(&second);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    h.run_steps(3);
    assert_eq!(waypoints(&h), 0, "the first module's area shown in the second");
    // And back: the first's own again.
    h.state_mut().open_module(&first);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    h.run_steps(3);
    assert_eq!(waypoints(&h), 2);
    assert_eq!(std::fs::read(&first).unwrap(), before);
}

/// Frames until the game's programs, made while the frames go on, draw
/// (at most a minute).
fn until_the_game_draws(h: &mut Harness<'_, Moonglow>) {
    let start = std::time::Instant::now();
    loop {
        h.run_steps(1);
        let d = h.state().viewport.as_ref().unwrap().renderer.drawn;
        if d.game > 0 && d.game_pending == 0 {
            // (And one more frame: the area view's picture is drawn anew
            // as its animations step.)
            std::thread::sleep(std::time::Duration::from_millis(50));
            h.run_steps(2);
            return;
        }
        assert!(start.elapsed().as_secs() < 60, "the game's programs never drew: {d:?}");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// The area view's Shaders switch: on by default (the setting not yet
/// chosen: the other area view tests choose it off); off, Moonglow's own
/// shader; on, the area is drawn again with the game's own shaders (the
/// stock lit programs for the tiles), and the choice is kept in the
/// settings; off again, with Moonglow's own.
#[test]
fn the_area_view_s_shaders_switch_draws_with_the_game_s_shaders() {
    let Some((mut h, area)) = area_harness("shaders-switch") else { return };
    h.state_mut().settings.game_shaders = None;
    assert!(h.state().settings.game_shaders());
    let drawn = |h: &Harness<'_, Moonglow>| h.state().viewport.as_ref().unwrap().renderer.drawn;
    until_the_game_draws(&mut h);
    h.get_by_label("🎨 Shaders").click();
    h.run_steps(3);
    assert_eq!(h.state().settings.game_shaders, Some(false));
    let pictures = h.state().area_views[&area].pictures;
    assert!(pictures > 0);
    assert_eq!(drawn(&h).game, 0);
    h.get_by_label("🎨 Shaders").click();
    h.run_steps(1);
    // (Its programs made while the view goes on: Moonglow's shader first.)
    assert!(drawn(&h).game_pending > 0 || drawn(&h).game > 0, "{:?}", drawn(&h));
    until_the_game_draws(&mut h);
    assert_eq!(h.state().settings.game_shaders, Some(true));
    assert!(h.state().area_views[&area].pictures > pictures, "drawn again");
    let vp = h.state().viewport.as_ref().unwrap();
    assert!(vp.renderer.game_shaders);
    assert_eq!(vp.renderer.game_shader_failures(), &[]);
    assert!(drawn(&h).game > 0, "{:?}", drawn(&h));
    // Kept as the settings are.
    let saved = serde_json::to_string(&h.state().settings).unwrap();
    let read: mg_ui::Settings = serde_json::from_str(&saved).unwrap();
    assert!(read.game_shaders());
    h.get_by_label("🎨 Shaders").click();
    h.run_steps(3);
    assert_eq!(h.state().settings.game_shaders, Some(false));
    assert_eq!(drawn(&h).game, 0);
}

/// The mean colour and the mean saturation (HSV, 0 to 1) of a picture.
fn mean_and_saturation(img: &image::RgbaImage) -> ([f64; 3], f64) {
    let (mut sum, mut sat) = ([0f64; 3], 0f64);
    for p in img.pixels() {
        for (c, s) in sum.iter_mut().enumerate() {
            *s += f64::from(p[c]);
        }
        let (max, min) = (p[0].max(p[1]).max(p[2]), p[0].min(p[1]).min(p[2]));
        if max > 0 {
            sat += f64::from(max - min) / f64::from(max);
        }
    }
    let n = f64::from(img.width() * img.height()).max(1.0);
    (sum.map(|s| s / n), sat / n)
}

/// A stock interior lit (by default the Chapter 1 smithy, `pensmithy1`:
/// `tin01`, its tiles' lights and the night's; `MG_INTERIOR=module.nwm:area`
/// another) looks alike in the area view with the Shaders switch on (the
/// game's own shaders) and off (Moonglow's): the same colours and as
/// saturated, to within a level of 255. A builder saw the view darker,
/// grayer and colder with the switch on (Heed, on 2.5.1); with the stock
/// shaders it isn't (the client agrees with both: `area_lighting_look`
/// in `client_render.rs`), and a hak's or the override's `inc_lighting`
/// (as The Domains of Dread's diffuse wrap) is what changes the switch's
/// side alone, as it changes the game's. Pictures in
/// `target/test-output/ui-interior-shaders/`.
#[test]
fn a_stock_interior_looks_alike_with_the_shaders_on_and_off() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let which = std::env::var("MG_INTERIOR").unwrap_or_else(|_| "Chapter1.nwm:pensmithy1".into());
    let (file, name) = which.split_once(':').expect("module.nwm:area");
    let source = mg_module::Module::open(&root.join("data/nwm").join(file)).unwrap();
    let from = ResRef::from_str(name).unwrap();
    let are = source.gff(&ResKey::new(from, ResType::ARE)).unwrap().unwrap();
    let tileset = are.root.resref("Tileset").unwrap().to_string();
    let side = are.root.int("Width").unwrap().max(are.root.int("Height").unwrap()) as u32;
    // The area made the stock one (under its own name, its scripts and
    // creatures left out: they walk), lit as it is.
    let prepare = |m: &mut mg_module::Module, _: &mg_rules::GameData, area: ResRef| {
        let read = |t| source.gff(&ResKey::new(from, t)).unwrap().unwrap();
        let (mut are, mut git) = (read(ResType::ARE), read(ResType::GIT));
        are.root.set("ResRef", mg_gff::Value::resref(area));
        for label in ["OnEnter", "OnExit", "OnHeartbeat", "OnUserDefined"] {
            are.root.set(label, mg_gff::Value::resref(ResRef::EMPTY));
        }
        git.root.set("Creature List", mg_gff::Value::List(Vec::new()));
        m.set_gff(ResKey::new(area, ResType::ARE), &are).unwrap();
        m.set_gff(ResKey::new(area, ResType::GIT), &git).unwrap();
    };
    let Some((mut h, area)) =
        area_harness_with("interior-shaders", &tileset, None, side, None, &prepare)
    else {
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-interior-shaders");
    let (w, d) = (are.root.int("Width").unwrap() as f32, are.root.int("Height").unwrap() as f32);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        assert!(view.lit, "lit at first");
        view.grid = false;
        if let Some(o) = view.orbit.as_mut() {
            o.target = glam::Vec3::new(w * 5.0, d * 5.0, 0.0);
            (o.distance, o.pitch, o.yaw) = (22.0, 0.85, (-90f32).to_radians());
        }
    }
    let picture = |h: &mut Harness<'_, Moonglow>, on: bool| {
        h.state_mut().settings.game_shaders = Some(on);
        h.hover_at(egui::pos2(5.0, 5.0));
        h.run_steps(3);
        if on {
            until_the_game_draws(h);
            let vp = h.state().viewport.as_ref().unwrap();
            assert_eq!(vp.renderer.game_shader_failures(), &[]);
        } else {
            h.run_steps(3);
        }
        let r = h.state().area_views[&area].rect;
        let img = h.render().expect("render");
        let img = image::imageops::crop_imm(
            &img,
            r.min.x as u32 + 8,
            r.min.y as u32 + 8,
            r.width() as u32 - 16,
            r.height() as u32 - 16,
        )
        .to_image();
        img.save(dir.join(if on { "on.png" } else { "off.png" })).unwrap();
        img
    };
    let (off, on) = (picture(&mut h, false), picture(&mut h, true));
    let ((a, sa), (b, sb)) = (mean_and_saturation(&off), mean_and_saturation(&on));
    eprintln!(
        "{which}: Shaders off ({:.1}, {:.1}, {:.1}) saturation {sa:.3}; on ({:.1}, {:.1}, {:.1}) \
         saturation {sb:.3}; {}",
        a[0],
        a[1],
        a[2],
        b[0],
        b[1],
        b[2],
        dir.display()
    );
    assert!(a.iter().sum::<f64>() > 30.0, "lit, not dark: {a:?}");
    for c in 0..3 {
        assert!((a[c] - b[c]).abs() < 1.0, "channel {c}: off {a:?}, on {b:?}");
    }
    assert!((sa - sb).abs() < 0.01, "saturation: off {sa}, on {sb}");
}

/// The area view with The Domains of Dread's tileset and shaders (its haks
/// where `MOONGLOW_HAKS` has them, linked into a scratch user folder; the
/// haks themselves untouched; skipped without): a field of `mst04` tiles
/// whose trees' leaves and plants sway by its custom shaders. With the
/// Shaders switch off the view holds still from one moment to the next;
/// on, it is drawn otherwise (its own lighting, `tdod_shaders`'
/// `inc_lighting`), and moves. Pictures in
/// `target/test-output/ui-area-tdod-shaders/` to look at.
#[cfg(unix)]
#[test]
fn tdod_s_wind_shaders_move_in_the_area_view() {
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else { return };
    let Some(haks) = mg_testkit::builder_hak_dirs()
        .into_iter()
        .find(|d| d.join("tdod_t_tap.hak").is_file() && d.join("tdod_shaders.hak").is_file())
    else {
        eprintln!("skipped: no tdod_t_tap.hak in MOONGLOW_HAKS");
        return;
    };
    let Some(rs) = gpu() else { return };
    let dir = mg_testkit::scratch_dir("ui-area-tdod-shaders");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let names = ["tdod_shaders", "tdod_t_tap"];
    for name in names {
        let file = format!("{name}.hak");
        mg_testkit::link_or_copy(&haks.join(&file), &user.join("hak").join(&file)).unwrap();
    }
    // A module on a stock tileset, made Medieval Rural's field of TDoD's
    // `mst04` tiles 42 and 48 (trees and plants of the wind shaders).
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Wind", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let key = ResKey::new(area, ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    are.root.set("Tileset", mg_gff::Value::ResRef(b"mst04".to_vec()));
    for (i, t) in are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
        t.set("Tile_ID", mg_gff::Value::Int(if i % 2 == 0 { 42 } else { 48 }));
        t.set("Tile_Orientation", mg_gff::Value::Int(0));
        t.set("Tile_Height", mg_gff::Value::Int(0));
    }
    m.set_gff(key, &are).unwrap();
    let ifo_key = ResKey::parse("module", ResType::IFO).unwrap();
    let mut ifo = m.gff(&ifo_key).unwrap().unwrap();
    let list = mg_module::attach::hak_list(&ifo.root, &names.map(String::from));
    ifo.root.set("Mod_HakList", list);
    m.set_gff(ifo_key, &ifo).unwrap();
    let path = dir.join("wind.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    // (Off first, as the test compares the two.)
    app.settings.game_shaders = Some(false);
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    let mut h = area_gpu_harness(app, rs, 1200.0, 900.0);
    h.run_steps(20);
    // Close on the trees, from low down.
    if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
        o.target = glam::Vec3::new(5.0, 10.0, 2.0);
        (o.distance, o.pitch) = (16.0, 0.35);
    }
    h.run_steps(10);
    let shot = |h: &mut Harness<'_, Moonglow>, name: &str| {
        let img = h.render().expect("render");
        img.save(dir.join(format!("{name}.png"))).unwrap();
        img
    };
    let differ = |a: &image::RgbaImage, b: &image::RgbaImage| {
        a.pixels().zip(b.pixels()).filter(|(p, q)| (0..3).any(|i| p[i].abs_diff(q[i]) > 2)).count()
    };
    let off = [shot(&mut h, "off_a"), {
        h.run_steps(30);
        shot(&mut h, "off_b")
    }];
    h.get_by_label("🎨 Shaders").click();
    until_the_game_draws(&mut h);
    let on_a = shot(&mut h, "on_a");
    h.run_steps(30);
    let on_b = shot(&mut h, "on_b");
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    let drawn = h.state().viewport.as_ref().unwrap().renderer.drawn;
    let (still, moved, changed) =
        (differ(&off[0], &off[1]), differ(&on_a, &on_b), differ(&off[1], &on_a));
    eprintln!("{drawn:?}; off {still} pixels moved, on {moved}; {changed} changed by the switch");
    assert!(
        !log.iter().any(|l| l.contains("drawn with Moonglow's own")),
        "no program failed: {log:?}"
    );
    assert!(drawn.game > 0 && drawn.timed > 0, "{drawn:?}");
    assert!(changed > 1000, "the switch changes the view: {changed}");
    assert!(moved > still + 200, "the wind moves it: {moved} against {still}");
}

/// A hak's include overrides the stock one through the load order
/// (`docs/shaders-proposal.md`, M4): an area of The Domains of Dread's
/// `mst04` tiles (`MOONGLOW_HAKS`; skipped without), drawn by the game's
/// shaders with `tdod_shaders.hak` attached (its `inc_lighting`, a
/// diffuse-wrap lighting, over the stock one) and without it, and with the
/// switch off. Pictures in `target/test-output/ui-area-tdod-lighting/`
/// (`with.png`, `without.png`, `off_with.png`, `off_without.png`).
#[cfg(unix)]
#[test]
fn tdod_s_inc_lighting_changes_the_area_view_through_the_load_order() {
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else { return };
    let Some(haks) = mg_testkit::builder_hak_dirs()
        .into_iter()
        .find(|d| d.join("tdod_t_tap.hak").is_file() && d.join("tdod_shaders.hak").is_file())
    else {
        eprintln!("skipped: no tdod_t_tap.hak in MOONGLOW_HAKS");
        return;
    };
    mg_testkit::gpu::hold();
    if mg_render::Gpu::headless().is_none() {
        eprintln!("skipped: no GPU adapter");
        return;
    }
    let dir = mg_testkit::scratch_dir("ui-area-tdod-lighting");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    for name in ["tdod_shaders", "tdod_t_tap"] {
        let file = format!("{name}.hak");
        mg_testkit::link_or_copy(&haks.join(&file), &user.join("hak").join(&file)).unwrap();
    }
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    // The area: 4 by 4 of mst04's first tiles, in a module listing the
    // haks given.
    let module = |haks: &[&str], file: &str| {
        let mut rng = fastrand::Rng::with_seed(7);
        let mut m = new_module(&game, "Lighting", &mut rng).unwrap();
        let spec = AreaSpec {
            name: "Field".into(),
            tileset: ResRef::from_str("ttr01").unwrap(),
            width: 4,
            height: 4,
        };
        let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        are.root.set("Tileset", mg_gff::Value::ResRef(b"mst04".to_vec()));
        for (i, t) in are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
            t.set("Tile_ID", mg_gff::Value::Int(i as i32));
            t.set("Tile_Orientation", mg_gff::Value::Int(0));
            t.set("Tile_Height", mg_gff::Value::Int(0));
        }
        m.set_gff(key, &are).unwrap();
        let ifo_key = ResKey::parse("module", ResType::IFO).unwrap();
        let mut ifo = m.gff(&ifo_key).unwrap().unwrap();
        let names: Vec<String> = haks.iter().map(|h| h.to_string()).collect();
        let list = mg_module::attach::hak_list(&ifo.root, &names);
        ifo.root.set("Mod_HakList", list);
        m.set_gff(ifo_key, &ifo).unwrap();
        let path = dir.join(file);
        m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        (path, area)
    };
    // The area view of a module, the switch on or off, after the same
    // number of frames (the wind shaders' clock the same).
    let picture = |path: &std::path::Path, area: ResRef, on: bool, name: &str| {
        let rs = egui_kittest::wgpu::create_render_state(
            egui_kittest::wgpu::default_wgpu_setup(),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let mut app = Moonglow::new(Some(install.clone()), Box::new(NoDialogs::default()));
        app.set_render_state(rs.clone());
        app.settings.game_shaders = Some(on);
        app.open_module(path);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        let mut h = area_gpu_harness(app, rs, 1200.0, 900.0);
        h.run_steps(20);
        if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
            o.target = glam::Vec3::new(20.0, 20.0, 0.0);
            (o.distance, o.pitch) = (42.0, 0.8);
        }
        let mut steps = 20;
        let start = std::time::Instant::now();
        if on {
            loop {
                h.run_steps(1);
                steps += 1;
                let d = h.state().viewport.as_ref().unwrap().renderer.drawn;
                if d.game > 0 && d.game_pending == 0 {
                    break;
                }
                assert!(start.elapsed().as_secs() < 60, "the game's programs never drew: {d:?}");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        assert!(steps < 300, "{steps} frames to draw");
        h.run_steps(300 - steps);
        let img = h.render().expect("render");
        img.save(dir.join(format!("{name}.png"))).unwrap();
        let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
        assert!(
            !log.iter().any(|l| l.contains("drawn with Moonglow's own")),
            "no program failed: {log:?}"
        );
        img
    };
    let (with_path, area) = module(&["tdod_shaders", "tdod_t_tap"], "with.mod");
    let (without_path, area2) = module(&["tdod_t_tap"], "without.mod");
    assert_eq!(area, area2);
    let with = picture(&with_path, area, true, "with");
    let without = picture(&without_path, area, true, "without");
    let off_with = picture(&with_path, area, false, "off_with");
    let off_without = picture(&without_path, area, false, "off_without");
    // (The area view's picture: the window around it names the module.)
    let view = |a: &image::RgbaImage| image::imageops::crop_imm(a, 270, 220, 660, 500).to_image();
    let (with, without, off_with, off_without) =
        (view(&with), view(&without), view(&off_with), view(&off_without));
    let differ = |a: &image::RgbaImage, b: &image::RgbaImage| {
        a.pixels().zip(b.pixels()).filter(|(p, q)| (0..3).any(|i| p[i].abs_diff(q[i]) > 2)).count()
    };
    let brightness = |a: &image::RgbaImage| {
        a.pixels().map(|p| (f64::from(p[0]) + f64::from(p[1]) + f64::from(p[2])) / 3.0).sum::<f64>()
            / f64::from(a.width() * a.height())
    };
    let (lit, off) = (differ(&with, &without), differ(&off_with, &off_without));
    eprintln!(
        "with tdod_shaders' inc_lighting {lit} pixels differ from without it (brightness {:.1} \
         against {:.1}); with the switch off {off} (brightness {:.1}, {:.1})",
        brightness(&with),
        brightness(&without),
        brightness(&off_with),
        brightness(&off_without),
    );
    // The hak's include changes what the game's shaders draw, and nothing
    // of Moonglow's own.
    assert!(lit > 50_000, "{lit}");
    assert!(off == 0, "{off}");
}

#[test]
fn preview_window_shows_the_chosen_blueprint() {
    let Some((mut h, _area)) = area_harness("preview") else { return };
    h.get_by_label("👁 Preview").click();
    h.run_steps(2);
    h.get_by_label("Choose a blueprint in the palette.");
    // A waypoint: its fields.
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    h.run_steps(2);
    h.get_by_label("Blueprint ResRef");
    h.get_by_label("nw_waypoint001");
    // A chest: also in 3D.
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.state_mut().palette.selected = Some(chest);
    h.run_steps(3);
    h.get_by_label("plc_chest1");
    assert!(
        h.state().model_views.contains_key(&mg_ui::model_view::Source::Resource(chest)),
        "the 3D view"
    );
    // An item: also its inventory icon.
    h.state_mut().palette.selected = ResKey::parse("nw_wswls001", ResType::UTI);
    h.run_steps(3);
    h.get_by_label("Item icon");
}

#[test]
fn a_preview_follows_its_blueprint_s_editor() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let dir = mg_testkit::scratch_dir("ui-preview-live");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    app.open_module(&path);
    // A human of parts, in the module, its preview open beside its editor.
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named("nw_bandit001", ResType::UTC).unwrap().into_owned();
    let key = ResKey::parse("live_look", ResType::UTC).unwrap();
    let set = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(value),
    };
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "copy",
        vec![
            mg_edit::Edit::SetResource { key, data: Some(data) },
            set("Appearance_Type", mg_gff::Value::Word(6)),
            set("Appearance_Head", mg_gff::Value::Byte(1)),
        ],
    )));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = gpu_harness(app, rs, 1000.0, 700.0);
    h.run_steps(4);
    let head = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let p = h.state().model_views[&mg_ui::model_view::Source::Resource(key)]
            .preview()
            .expect("a preview");
        p.parts.iter().map(|p| p.model.clone()).filter(|m| m.contains("_head")).collect()
    };
    assert_eq!(head(&h), ["pmh0_head001"]);
    // The editor changes the head: the open preview shows the new one.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Head",
        vec![set("Appearance_Head", mg_gff::Value::Byte(3))],
    )));
    h.run_steps(4);
    assert_eq!(head(&h), ["pmh0_head003"]);
}

#[test]
fn placed_objects_preview_as_placed() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("instance-preview") else { return };
    // A human of parts, placed in the area.
    let git = ResKey::new(area, ResType::GIT);
    let entry = {
        let game = h.state().game.as_deref().unwrap();
        let data = game.resman.get_named("nw_bandit001", ResType::UTC).unwrap();
        let mut bp = Gff::read(&data).unwrap().root;
        bp.set("Appearance_Type", mg_gff::Value::Word(6));
        bp.set("Appearance_Head", mg_gff::Value::Byte(1));
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [25.0, 25.0, 0.0], rotation: 0.0 };
        instance(&placing, ResType::UTC, &bp, at, &[]).unwrap()
    };
    let path = mg_edit::GffPath::root().item("Creature List", 0);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "place",
        vec![mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Creature List".into(),
            index: 0,
            item: entry,
        }],
    )));
    h.state_mut()
        .actions
        .push(mg_ui::Action::OpenTab(Tab::InstanceModel { area, path: path.clone() }));
    h.run_steps(4);
    let source = mg_ui::model_view::Source::Instance { area, path: path.clone() };
    let head = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let p = h.state().model_views[&source].preview().expect("a preview of the placed creature");
        p.parts.iter().map(|p| p.model.clone()).filter(|m| m.contains("_head")).collect()
    };
    assert_eq!(head(&h), ["pmh0_head001"]);
    // Its Properties change its head (both fields, as the editor sets them:
    // a placed creature has EE's wide one too): the preview follows.
    let set = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
        key: git,
        path: path.clone(),
        label: label.into(),
        value: Some(value),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Head",
        vec![
            set("Appearance_Head", mg_gff::Value::Byte(3)),
            set("xAppearance_Head", mg_gff::Value::Word(3)),
        ],
    )));
    h.run_steps(4);
    assert_eq!(head(&h), ["pmh0_head003"]);
}

#[test]
fn a_store_s_preview_lists_what_it_sells() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-store-preview");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.open_palette = false;
    // The thieves' store of Hordes of the Underdark (its jewelry page has
    // rings and amulets), as the palette's double click opens it.
    let key = ResKey::parse("x2_storethief001", ResType::UTM).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = harness(app, 1200.0, 900.0);
    h.run_steps(3);
    assert!(h.query_by_label_contains("no preview").is_none());
    h.get_by_label("Blueprint ResRef");
    h.get_by_label_contains("Rings & Amulets (");
}

/// An item in its preview: one modeled lying (a sword) is stood up; a
/// helmet and a shield, modeled as worn, are shown so, a shield from its
/// front (a builder's report: they came up on their sides).
#[test]
fn helmets_and_shields_preview_as_worn() {
    let root = mg_testkit::corpus!();
    let mut app = game_app(&root, None);
    for name in ["nw_wswls001", "nw_arhe001", "nw_ashsw001"] {
        let key = ResKey::parse(name, ResType::UTI).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    }
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let view = |name: &str| {
        let key = ResKey::parse(name, ResType::UTI).unwrap();
        let v = &h.state().model_views[&mg_ui::model_view::Source::Resource(key)];
        (v.stood(), v.yaw.to_degrees().round())
    };
    assert_eq!(view("nw_wswls001"), (true, 60.0), "a sword stood up");
    assert_eq!(view("nw_arhe001"), (false, 60.0), "a helmet as worn");
    assert_eq!(view("nw_ashsw001"), (false, 240.0), "a shield as worn, from its front");
}

/// A placeable that is only an effect (flames, magic sparks: models with
/// emitters and no mesh) shows its particles in the area, while the
/// animations play.
#[test]
fn effect_placeables_show_their_particles_in_the_area() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("particles") else { return };
    h.run_steps(40);
    assert_eq!(h.state().area_views[&area].particles_shown, 0);
    h.state_mut().palette.selected = ResKey::parse("plc_flamelarge", ResType::UTP);
    let at = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(30);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    assert_eq!(git.root.list("Placeable List").map(<[mg_gff::Struct]>::len), Some(1));
    assert!(h.state().area_views[&area].particles_shown > 0, "its particles are drawn");
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    h.hover_at(at + egui::vec2(150.0, 150.0));
    h.run_steps(200);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-particles").join("particles.png")).unwrap();
}

/// An area's picture is kept while nothing it shows changes (a frame is
/// drawn for every move of the pointer, and the picture is most of what a
/// frame costs): drawn again as its animations step, and at once when the
/// camera moves, the view is set to show something else, or the area is
/// edited.
#[test]
fn an_area_s_picture_is_kept_until_something_in_it_changes() {
    let Some((mut h, area)) = area_harness("picture-kept") else { return };
    h.run_steps(10);
    let pictures = |h: &Harness<'static, Moonglow>| h.state().area_views[&area].pictures;
    let mut now = h.ctx.input(|i| i.time);
    let mut frame = |h: &mut Harness<'static, Moonglow>, later: f64| {
        now += later;
        h.input_mut().time = Some(now);
        h.run_steps(1);
    };
    frame(&mut h, 0.25);
    let drawn = pictures(&h);
    // Frames a 200th of a second apart: the same picture.
    for _ in 0..5 {
        frame(&mut h, 0.005);
    }
    assert_eq!(pictures(&h), drawn, "kept from frame to frame");
    // The animations step some 25 times a second.
    frame(&mut h, 0.03);
    assert_eq!(pictures(&h), drawn + 1, "drawn as the animations step");
    // The camera turned: at once.
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw += 0.1;
    frame(&mut h, 0.005);
    assert_eq!(pictures(&h), drawn + 2, "drawn as the camera moves");
    // By night.
    h.state_mut().area_views.get_mut(&area).unwrap().night ^= true;
    frame(&mut h, 0.005);
    assert_eq!(pictures(&h), drawn + 3, "drawn as the view changes");
    // The pointer over it, with nothing to place: the same picture.
    let waypoint = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(waypoint);
    frame(&mut h, 0.005);
    h.hover_at(waypoint + egui::vec2(3.0, 0.0));
    frame(&mut h, 0.005);
    assert_eq!(pictures(&h), drawn + 3, "kept under the pointer");
    // The area edited (a waypoint deleted).
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    h.key_press(egui::Key::Delete);
    frame(&mut h, 0.005);
    frame(&mut h, 0.005);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    assert_eq!(git.root.list("WaypointList").map(<[mg_gff::Struct]>::len), Some(1));
    assert!(pictures(&h) > drawn + 3, "drawn as the area changes");
    let drawn = pictures(&h);
    for _ in 0..3 {
        frame(&mut h, 0.005);
    }
    assert_eq!(pictures(&h), drawn, "and kept again");
}

/// Area Properties › Grass: the tileset's grass turned off, grass added
/// to a surface material, each one step to undo, stored in the GIT where
/// the game reads it (and kept when the module is saved), and grown in
/// the area's view; the toolbar's Grass switch shows it or not, in the
/// picture.
#[test]
fn grass_is_set_in_area_properties_and_grown_in_the_view() {
    use mg_area::grass::{AreaGrass, GRASS_MATERIAL};
    let Some((mut h, area)) = area_harness("grass") else { return };
    if let Some(o) = &mut h.state_mut().area_views.get_mut(&area).unwrap().orbit {
        o.target = glam::Vec3::new(20.0, 20.0, 0.0);
        (o.pitch, o.yaw, o.distance) = (35f32.to_radians(), -60f32.to_radians(), 9.0);
    }
    h.run_steps(3);
    let grown = |h: &Harness<'_, Moonglow>| -> Vec<(u32, String)> {
        let view = &h.state().area_views[&area];
        let grown = view.grass_grown().unwrap_or(&[]);
        grown.iter().map(|(m, l)| (*m, l.texture.clone())).collect()
    };
    // The tileset's (ttr01: `grass` on grass faces).
    assert_eq!(grown(&h), [(GRASS_MATERIAL, "grass".to_string())]);
    // In the picture, and gone with the toolbar's switch.
    let with = h.render().expect("render");
    h.get_by_label(&format!("{} Grass", "🌿")).click();
    h.run_steps(3);
    assert!(h.state().settings.no_grass);
    let without = h.render().expect("render");
    let changed = with
        .pixels()
        .zip(without.pixels())
        .filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 24))
        .count();
    assert!(changed > 20_000, "the grass changed {changed} pixels");
    h.get_by_label(&format!("{} Grass", "🌿")).click();
    h.run_steps(3);
    assert!(!h.state().settings.no_grass);

    let git = ResKey::new(area, ResType::GIT);
    let stored = |h: &mut Harness<'_, Moonglow>| {
        AreaGrass::read(&h.state_mut().ws.as_mut().unwrap().doc(&git).unwrap().root)
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
    h.run_steps(5);
    // (The window's page: the palette has a Grass terrain too.)
    h.get_all_by_label("Grass").last().unwrap().click();
    h.run_steps(3);
    h.get_by_label("Grow the tileset's grass on Grass faces").click();
    h.run_steps(3);
    assert!(stored(&mut h).default_disabled);
    assert!(grown(&h).is_empty(), "nothing grows: {:?}", grown(&h));
    // Grass of the area's own on dirt: the tileset's, as a start.
    h.get_by_label("Add Grass to…").click();
    h.run_steps(2);
    h.get_by_label("Dirt (1)").click();
    h.run_steps(3);
    let now = stored(&mut h);
    assert_eq!(now.overrides.len(), 1);
    assert_eq!(now.overrides[0].material, 1);
    assert_eq!(now.overrides[0].look.texture, "grass");
    assert_eq!(now.overrides[0].look.height, 1.2);
    assert_eq!(grown(&h), [(1, "grass".to_string())]);
    // Its density dragged: grown in the view as it goes, set when let go.
    let density = |h: &Harness<'_, Moonglow>| {
        h.state().area_views[&area].grass_grown().unwrap_or(&[]).first().map(|(_, l)| l.density)
    };
    let field = h.get_by_value("5.00").rect().center();
    h.hover_at(field);
    press(&h, field, true, egui::Modifiers::NONE);
    for k in 1..=10 {
        h.hover_at(field + egui::vec2(8.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    h.run_steps(2);
    let dragged = density(&h).unwrap();
    assert!(dragged > 5.5, "grown at {dragged} while dragged");
    assert_eq!(stored(&mut h).overrides[0].look.density, 5.0, "not set yet");
    press(&h, field + egui::vec2(80.0, 0.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    let set = stored(&mut h).overrides[0].look.density;
    assert!(set > 5.5, "set to {set}");
    assert_eq!(density(&h), Some(set));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(stored(&mut h).overrides[0].look.density, 5.0, "the drag, one step");
    // Each a step of its own.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert!(stored(&mut h).overrides.is_empty());
    h.state_mut().actions.push(mg_ui::Action::Redo);
    h.run_steps(3);
    assert_eq!(stored(&mut h).overrides.len(), 1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    // Saved, and read again from the file.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    let location = h.state().ws.as_ref().unwrap().module.location.clone();
    let Some(ModuleLocation::Archive(path)) = location else { panic!("{location:?}") };
    let back = mg_module::Module::open(&path).unwrap();
    let saved = AreaGrass::read(&back.gff(&git).unwrap().unwrap().root);
    assert_eq!(saved, stored(&mut h));
    assert!(saved.default_disabled && saved.overrides[0].material == 1);
}

/// The area view's grass covers what the view shows, not a reach around
/// its focus (the owner's report: grass only near the camera's target, the
/// rest of a field bare): on a 32 by 32 field seen across from low down,
/// and from halfway up, the grass shows at the far parts of the view, a
/// hundred meters and more from the target, and on either side of it.
/// (It once ended 30 m from the target: there nothing changed.) Close up
/// it looks as it did (the pictures, `target/test-output/ui-area-grass-far/`,
/// before and after the change). From high up a tuft (an upright quad
/// seen from above, a meter high) is a pixel or two in a window this
/// size, and grass changes next to nothing in the picture, near the target
/// or far: the view from there is drawn, not judged.
#[test]
fn grass_covers_the_whole_view() {
    let Some((mut h, area)) = area_harness_sized("grass-far", "ttr01", None, 32) else { return };
    let dir = mg_testkit::output_dir("ui-area-grass-far");
    // (The views: the target, pitch, yaw and distance; and the points
    // far off in each where the grass must show.)
    let middle = glam::Vec3::new(160.0, 160.0, 0.0);
    let views = [
        ("high", middle, 0.9, -60f32.to_radians(), 420.0),
        ("across", middle, 0.3, -60f32.to_radians(), 50.0),
        ("halfway", middle, 0.6, -60f32.to_radians(), 150.0),
        ("close", glam::Vec3::new(20.0, 20.0, 0.0), 35f32.to_radians(), -60f32.to_radians(), 9.0),
    ];
    let mut bare = Vec::new();
    // (Without the grid, which would cover what is far off.)
    h.state_mut().area_views.get_mut(&area).unwrap().grid = false;
    for (name, target, pitch, yaw, distance) in views {
        if let Some(o) = &mut h.state_mut().area_views.get_mut(&area).unwrap().orbit {
            o.target = target;
            (o.pitch, o.yaw, o.distance) = (pitch, yaw, distance);
        }
        let mut shots = Vec::new();
        for no_grass in [false, true] {
            h.state_mut().settings.no_grass = no_grass;
            // (Grown as the view comes to it: a few frames.)
            h.run_steps(12);
            let shot = h.render().expect("render");
            let what = if no_grass { "without" } else { "with" };
            shot.save(dir.join(format!("{name}-{what}.png"))).unwrap();
            shots.push(shot);
        }
        let view = &h.state().area_views[&area];
        let eye = view.camera().unwrap().eye;
        // Far off: a hundred meters and more past the target (looking
        // across), and around it on every side (from halfway up).
        let away = (target - eye).with_z(0.0).normalize();
        let side = away.cross(glam::Vec3::Z);
        let far: Vec<glam::Vec3> = match name {
            "across" => vec![target + away * 100.0, target + away * 150.0],
            "halfway" => vec![
                target + away * 40.0,
                target - away * 35.0,
                target + side * 50.0,
                target - side * 50.0,
            ],
            "close" => vec![target],
            _ => Vec::new(),
        };
        for p in far {
            let at = view.screen_pos(p).expect("in the view");
            assert!(
                view.rect.shrink(20.0).contains(at),
                "{name}: {p} at {at:?} is out of the view"
            );
            let (x0, y0) = (at.x as u32 - 20, at.y as u32 - 20);
            let changed = (x0..x0 + 40)
                .flat_map(|x| (y0..y0 + 40).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    let (a, b) = (shots[0].get_pixel(x, y), shots[1].get_pixel(x, y));
                    (0..3).any(|c| a[c].abs_diff(b[c]) > 12)
                })
                .count();
            let off = p.distance(eye);
            if changed <= 40 {
                bare.push(format!(
                    "{name}: the grass at {p} ({off:.0} m from the eye) changed {changed} of \
                     1600 pixels"
                ));
            }
        }
    }
    assert!(bare.is_empty(), "{bare:#?}");
}
