//! Pictures to look at: ignored tests that write a view's picture
//! (`cargo test -p mg-ui --test app -- --ignored look_`).

use crate::common::*;

/// A screenshot of terrain mode (`target/test-output/screens/terrain.png`),
/// for looking at: `cargo test -p mg-ui --test app terrain_screen -- --ignored`.
#[test]
#[ignore]
fn terrain_screen() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("terrain-screen") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("Tiles").click();
    h.run_steps(2);
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    for (brush, at) in [
        ("Water", [10.0, 20.0]),
        ("Water", [10.0, 30.0]),
        ("Trees", [30.0, 30.0]),
        ("↕ Raise/Lower", [40.0, 30.0]),
    ] {
        h.get_by_label(brush).click();
        h.run_steps(2);
        click(&mut h, Vec3::new(at[0], at[1], 0.0));
    }
    h.get_by_label("Road").click();
    h.run_steps(2);
    let from = screen(&h, area, Vec3::new(7.0, 5.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    for x in [10.0, 15.0, 20.0, 22.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 5.0, 0.0)));
        h.run_steps(1);
    }
    let to = screen(&h, area, Vec3::new(22.0, 5.0, 0.0));
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.get_by_label("Water").click();
    h.run_steps(2);
    h.hover_at(screen(&h, area, Vec3::new(20.0, 20.0, 0.0)));
    h.run_steps(4);
    let dir = mg_testkit::scratch_dir("screens-terrain");
    let image = h.render().expect("render");
    let out = dir.parent().unwrap().join("screens");
    std::fs::create_dir_all(&out).unwrap();
    image.save(out.join("terrain.png")).unwrap();
}

/// A look at Options › Area's marks: a door's orientation arrow and an
/// encounter's spawn point posts. Writes
/// `target/test-output/ui-area-marks/marks.png`; run by hand.
#[test]
#[ignore]
fn area_marks_screen() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("marks") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let game = h.state().game.as_deref().unwrap();
    let read = |r: &str, t: ResType| {
        Gff::read(&game.resman.get(&ResKey::parse(r, t).unwrap()).unwrap()).unwrap()
    };
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let door = instance(
        &placing,
        ResType::UTD,
        &read("nw_door_normal", ResType::UTD).root,
        Placement { position: [15.0, 15.0, 0.0], rotation: 0.6 },
        &[],
    )
    .unwrap();
    let outline = [[-3.0, -3.0, 0.0], [3.0, -3.0, 0.0], [3.0, 3.0, 0.0], [-3.0, 3.0, 0.0]];
    let mut enc = instance(
        &placing,
        ResType::UTE,
        &read("nw_giantevil", ResType::UTE).root,
        Placement { position: [25.0, 25.0, 0.0], rotation: 0.0 },
        &outline,
    )
    .unwrap();
    let spawn = |x: f32, y: f32| {
        let mut s = mg_gff::Struct::new(2);
        for (l, v) in [("X", x), ("Y", y), ("Z", 0.0), ("Orientation", 0.0)] {
            s.set(l, mg_gff::Value::Float(v));
        }
        s
    };
    enc.set("SpawnPointList", mg_gff::Value::List(vec![spawn(24.0, 24.0), spawn(26.0, 26.0)]));
    let edits = vec![
        mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Door List".into(),
            index: 0,
            item: door,
        },
        mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Encounter List".into(),
            index: 0,
            item: enc,
        },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("place", edits)));
    h.run_steps(5);
    let dir = mg_testkit::scratch_dir("ui-area-marks");
    h.render().expect("render").save(dir.join("marks.png")).unwrap();
}

#[test]
#[ignore = "a look at items lying in an area (MG_ITEMS=resref,resref…)"]
fn look_items_placed() {
    use glam::Vec3;
    let items = std::env::var("MG_ITEMS")
        .unwrap_or_else(|_| "nw_it_mpotion001,nw_ashsw001,nw_wswss001,nw_cloth001".into());
    let Some((mut h, area)) = area_harness("items-placed") else { return };
    h.set_size(egui::vec2(1500.0, 1000.0));
    h.run_steps(10);
    for (i, name) in items.split(',').enumerate() {
        h.state_mut().palette.selected = ResKey::parse(name.trim(), ResType::UTI);
        let at = screen(&h, area, Vec3::new(12.0 + 1.5 * i as f32, 14.0, 0.0));
        h.hover_at(at);
        h.run_steps(2);
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        h.run_steps(4);
    }
    h.state_mut().palette.selected = None;
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    view.selection.clear();
    if let Some(o) = view.orbit.as_mut() {
        o.target = Vec3::new(15.0, 14.0, 0.3);
        (o.distance, o.yaw, o.pitch) = (4.5, -1.57, 0.6);
    }
    h.run_steps(20);
    let dir = mg_testkit::scratch_dir("ui-items-placed");
    h.render().expect("render").save(dir.join("a.png")).unwrap();
}

#[test]
#[ignore = "a count of the particles a blueprint shows in an area (MG_BP=resref.utc)"]
fn look_particles_of() {
    use glam::Vec3;
    let Ok(bp) = std::env::var("MG_BP") else { return };
    let Some((mut h, area)) = area_harness("particles-of") else { return };
    h.run_steps(10);
    let (name, ext) = bp.split_once('.').unwrap();
    let restype = if ext == "utc" { ResType::UTC } else { ResType::UTP };
    h.state_mut().palette.selected = ResKey::parse(name, restype);
    let at = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(60);
    eprintln!("PARTICLES {bp}: {}", h.state().area_views[&area].particles_shown);
}

#[test]
#[ignore = "a look at some appearances' pictures"]
fn look_gallery_pictures() {
    let Some((mut h, _area)) = area_harness("gallery-look") else { return };
    h.state_mut().settings.gallery_tile = Some(220);
    h.state_mut().actions.push(mg_ui::Action::PlaceableGallery);
    h.run_steps(5);
    h.state_mut().placeable_gallery.as_mut().unwrap().filter =
        std::env::var("MG_FILTER").unwrap_or_default();
    h.run_steps(60);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-gallery-look").join("look.png")).unwrap();
}

#[test]
#[ignore = "a look at tiles' source lights (torches' flames)"]
fn look_source_lights() {
    let Some((mut h, area)) = area_harness_on("look-flames", "tic01", Some(0)) else { return };
    let key = ResKey::new(area, ResType::ARE);
    let tiles = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&key).unwrap().root.list("Tile_List").unwrap().len()
    };
    let edits = (0..tiles)
        .flat_map(|i| {
            ["Tile_SrcLight1", "Tile_SrcLight2"].map(|label| mg_edit::Edit::SetField {
                key,
                path: mg_edit::GffPath::root().item("Tile_List", i),
                label: label.into(),
                value: Some(mg_gff::Value::Byte(1)),
            })
        })
        .collect();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Flames", edits)));
    h.run_steps(10);
    let dir = mg_testkit::scratch_dir("ui-source-lights");
    for (i, yaw) in [0.0f32, 1.57, std::f32::consts::PI, 4.71].into_iter().enumerate() {
        h.state_mut().area_views.get_mut(&area).unwrap().orbit = Some(mg_ui::area_view::Orbit {
            target: glam::Vec3::new(15.0, 15.0, 1.5),
            yaw,
            pitch: 0.35,
            distance: 14.0,
        });
        h.run_steps(30);
        h.render().expect("render").save(dir.join(format!("look-{i}.png"))).unwrap();
    }
}

#[test]
#[ignore = "a look at the palette's gallery of items"]
fn look_item_gallery() {
    let Some((mut h, _area)) = area_harness("gallery-items") else { return };
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Item;
        (p.tiles, p.custom) = (false, false);
        p.filter = std::env::var("MG_FILTER").unwrap_or_else(|_| "shield".into());
    }
    h.state_mut().settings.palette_gallery = true;
    h.run_steps(60);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-gallery-items").join("look.png")).unwrap();
}

#[test]
#[ignore = "a look at the Creature Wizard's Appearance page"]
fn look_creature_wizard_appearance() {
    let Some((mut h, _area)) = area_harness("cw-look") else { return };
    let mut w = mg_ui::creature_wizard::CreatureWizard::default();
    (w.page, w.race, w.appearance) = (3, Some(6), 6);
    w.classes = vec![(4, 1)];
    w.appearance_find = std::env::var("MG_FILTER").unwrap_or_default();
    h.state_mut().creature_wizard = Some(w);
    h.run_steps(40);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-cw-look").join("look.png")).unwrap();
}

#[test]
#[ignore = "a look at a game's item, viewed: its parts as pictures, read-only"]
fn look_viewed_item() {
    let Some((mut h, _area)) = area_harness("viewed-item-look") else { return };
    let name = std::env::var("MG_ITEM").unwrap_or_else(|_| "nw_wswls001".into());
    if std::env::var("MG_SCREEN").is_ok() {
        h.set_size(egui::vec2(1920.0, 1080.0));
        h.run_steps(3);
    }
    if let Ok(w) = std::env::var("MG_WIDTH") {
        let w: u32 = w.parse().unwrap();
        h.set_size(egui::vec2(w as f32 + 500.0, 900.0));
        h.state_mut().settings.window_sizes.push(("item".into(), [w, 700]));
        h.run_steps(3);
    }
    let key = ResKey::parse(&name, ResType::UTI).unwrap();
    {
        let app = h.state_mut();
        let data = app.game.as_ref().unwrap().resman.get(&key).unwrap();
        app.ws.as_mut().unwrap().view(key, mg_gff::Gff::read(&data).unwrap());
        app.blueprint_pages.insert((key, mg_edit::GffPath::root()), "Appearance");
        app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    }
    h.run_steps(20);
    if std::env::var("MG_FEMALE").is_ok() {
        h.get_by_label("Female").click();
        h.run_steps(10);
    }
    if let Ok(w) = std::env::var("MG_RESIZE") {
        let path = h.state().dock.find_tab(&Tab::Blueprint(key)).unwrap();
        let state = h.state_mut().dock.get_window_state_mut(path.surface).unwrap();
        state.set_size(egui::vec2(w.parse().unwrap(), 700.0));
        h.run_steps(20);
    }
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-viewed-item-look").join("look.png")).unwrap();
}

#[test]
#[ignore = "a look at the palette's Gallery of waypoints, by their flags"]
fn look_marker_gallery() {
    let Some((mut h, _area)) = area_harness("marker-gallery-look") else { return };
    h.state_mut().settings.palette_gallery = true;
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Waypoint;
        (p.tiles, p.custom) = (false, false);
        p.filter = std::env::var("MG_FILTER").unwrap_or_else(|_| "a".into());
    }
    h.run_steps(60);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-marker-gallery-look").join("waypoints.png")).unwrap();
}

#[test]
#[ignore = "a look at the grid from the area's edge, low"]
fn look_grid_from_the_edge() {
    let Some((mut h, area)) = area_harness("grid-look") else { return };
    let dir = mg_testkit::scratch_dir("ui-grid-look");
    for (name, pitch) in [("low", 0.08), ("mid", 0.4), ("high", 1.2)] {
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        (o.yaw, o.pitch, o.distance) = (-1.57, pitch, 45.0);
        h.run_steps(5);
        let img = h.render().expect("render");
        img.save(dir.join(format!("{name}.png"))).unwrap();
    }
}

#[test]
#[ignore = "a look at a wide fog placeable in the area view"]
fn look_fog_placeable() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("fog-look") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let row: u32 = std::env::var("MG_ROW").ok().and_then(|r| r.parse().ok()).unwrap_or(15601);
    let fog = {
        let game = h.state().game.clone().unwrap();
        let key = ResKey::parse("x3_plc_mist", ResType::UTP).unwrap();
        let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let at = Placement { position: [20.0, 20.0, 0.0], rotation: 0.0 };
        let mut fog = instance(&placing, ResType::UTP, &bp, at, &[]).unwrap();
        fog.set("Appearance", mg_gff::Value::Dword(row));
        fog.set("Static", mg_gff::Value::Byte(1));
        fog
    };
    let placed = Edit::InsertItem {
        key: git,
        path: GffPath::root(),
        list: "Placeable List".into(),
        index: 0,
        item: fog,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    h.run_steps(240);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-fog-look").join("look.png")).unwrap();
}

#[test]
#[ignore = "a look at a builder's tileset group (MG_TEST_USER, MG_TILESET, MG_GROUP)"]
fn look_custom_group() {
    use glam::Vec3;
    let tileset = std::env::var("MG_TILESET").unwrap_or_else(|_| "testing".into());
    let group = std::env::var("MG_GROUP").unwrap_or_else(|_| "Volcano - Path 1".into());
    let Some((mut h, area)) = area_harness_on("custom-group-look", &tileset, None) else { return };
    h.set_size(egui::vec2(1500.0, 1000.0));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    // (The palette's own Expand All: the one to the right.)
    let expand = h
        .query_all_by_label("⏷")
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .unwrap()
        .rect()
        .center();
    h.hover_at(expand);
    press(&h, expand, true, egui::Modifiers::NONE);
    press(&h, expand, false, egui::Modifiers::NONE);
    h.run_steps(3);
    h.get_by_label(&group).click();
    h.run_steps(2);
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(5);
    h.key_press(egui::Key::Escape);
    {
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        o.pitch = std::env::var("MG_PITCH").ok().and_then(|p| p.parse().ok()).unwrap_or(0.9);
    }
    let dir = mg_testkit::scratch_dir("ui-custom-group-look");
    for (name, steps) in [("a", 30), ("b", 240)] {
        h.run_steps(steps);
        h.render().expect("render").save(dir.join(format!("{name}.png"))).unwrap();
    }
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    eprintln!("LOG {}", log.join(" | ").chars().take(600).collect::<String>());
}

#[test]
#[ignore = "a look at an area of a module with a user folder's haks (MG_MODULE, MG_AREA, MG_TEST_USER)"]
fn look_module_area() {
    let (Ok(module), Ok(area)) = (std::env::var("MG_MODULE"), std::env::var("MG_AREA")) else {
        return;
    };
    let Some(root) = mg_testkit::nwn_root() else { return };
    let Some(rs) = gpu() else { return };
    let user = std::env::var_os("MG_TEST_USER").map(std::path::PathBuf::from);
    let area = ResRef::from_str(&area).unwrap();
    let mut app = game_app(&root, user);
    app.set_render_state(rs.clone());
    app.open_module(std::path::Path::new(&module));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    let mut h = area_gpu_harness(app, rs, 1500.0, 1000.0);
    h.run_steps(40);
    // MG_AT: "x,y,z,distance,yaw,pitch" to look at one place.
    if let Ok(at) = std::env::var("MG_AT") {
        let v: Vec<f32> = at.split(',').filter_map(|t| t.trim().parse().ok()).collect();
        if let (Some(view), [x, y, z, distance, yaw, pitch]) =
            (h.state_mut().area_views.get_mut(&area), v.as_slice())
            && let Some(o) = view.orbit.as_mut()
        {
            o.target = glam::Vec3::new(*x, *y, *z);
            (o.distance, o.yaw, o.pitch) = (*distance, *yaw, *pitch);
        }
        h.run_steps(10);
    }
    let dir = mg_testkit::scratch_dir("ui-module-area-look");
    h.render().expect("render").save(dir.join("a.png")).unwrap();
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    eprintln!("LOG {}", log.join(" | ").chars().take(900).collect::<String>());
}

#[test]
#[ignore = "a look at fields that differ among blueprints edited together"]
fn look_edit_together_mixed() {
    use mg_edit::{Command, Edit};
    let Some(root) = mg_testkit::nwn_root() else { return };
    let Some(rs) = gpu() else { return };
    let dir = mg_testkit::scratch_dir("ui-mixed-look");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.open_palette = false;
    let game = app.game.as_deref().unwrap();
    let base = Gff::read(&game.resman.get_named("plc_chest1", ResType::UTP).unwrap()).unwrap();
    let keys: Vec<ResKey> =
        ["mg_box_a", "mg_box_b"].iter().map(|n| ResKey::parse(n, ResType::UTP).unwrap()).collect();
    let edits = keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let mut g = base.clone();
            g.root.set("TemplateResRef", mg_gff::Value::resref(k.resref));
            if i == 1 {
                g.root.set("Tag", mg_gff::Value::String(b"OTHER".to_vec()));
                g.root.set("Hardness", mg_gff::Value::Byte(77));
                g.root.set("Plot", mg_gff::Value::Byte(1));
            }
            Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprints(keys)));
    let mut h = gpu_harness(app, rs, 1000.0, 800.0);
    h.run_steps(5);
    h.render().expect("render").save(dir.join("a.png")).unwrap();
}

/// A placeable of a placeables.2da row, placed in the test area and seen
/// up close: for a model of a user folder's override (a builder's corpse
/// placeable drawn standing, 1.31.0). Writes
/// `target/test-output/ui-placeable-look/a.png`.
#[test]
#[ignore = "a look at a placeable's appearance in an area (MG_APPEARANCE, MG_TEST_USER)"]
fn look_placeable_appearance() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some(row) = std::env::var("MG_APPEARANCE").ok().and_then(|r| r.parse::<u32>().ok()) else {
        return;
    };
    let Some((mut h, area)) = area_harness("placeable-look") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let placed = {
        let game = h.state().game.clone().unwrap();
        let bp = game.resman.get(&ResKey::parse("plc_chest1", ResType::UTP).unwrap()).unwrap();
        let mut bp = Gff::read(&bp).unwrap();
        bp.root.set("Appearance", mg_gff::Value::Dword(row));
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let at = Placement { position: [25.0, 25.0, 0.0], rotation: 0.0 };
        instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap()
    };
    let edit = mg_edit::Edit::InsertItem {
        key: git,
        path: mg_edit::GffPath::root(),
        list: "Placeable List".into(),
        index: 0,
        item: placed,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.run_steps(20);
    if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
        o.target = glam::Vec3::new(25.0, 25.0, 0.5);
        (o.distance, o.pitch) = (5.0, 0.5);
    }
    h.run_steps(20);
    let dir = mg_testkit::scratch_dir("ui-placeable-look");
    h.render().expect("render").save(dir.join("a.png")).unwrap();
}

#[test]
#[ignore = "a look at the palettes with the game's data in another language (MG_LANG)"]
fn look_palette_in_language() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let Some(rs) = gpu() else { return };
    let lang = std::env::var("MG_LANG").unwrap_or_else(|_| "es".into());
    let dir = mg_testkit::scratch_dir("ui-palette-language");
    let path = sample_module(&dir);
    let user = std::env::var_os("MG_TEST_USER").map(std::path::PathBuf::from);
    let install = mg_resman::GameInstall::new(&root, user, &lang);
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Creature;
    app.palette.tiles = false;
    app.palette.custom = std::env::var_os("MG_CUSTOM").is_some();
    let mut h = gpu_harness(app, rs, 1000.0, 700.0);
    h.run_steps(6);
    h.render().expect("render").save(dir.join("a.png")).unwrap();
    let game = h.state().game.as_deref().unwrap();
    eprintln!("LANG {:?} sample {:?}", game.language, game.string(mg_core::StrRef(6686)));
}
