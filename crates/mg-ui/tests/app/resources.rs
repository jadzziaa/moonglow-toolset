//! Resources: the resource browser, override and haks, the hak editor,
//! models and the generic editor's raw fields.

use crate::common::*;

#[test]
fn reload_resources_picks_up_new_files_in_override() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-reload");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    let path = sample_module(&dir);
    let mut app = game_app(&root, Some(user.clone()));
    app.open_module(&path);
    let key = ResKey::parse("mg_reload_test", ResType::TWODA).unwrap();
    assert!(!app.game.as_deref().unwrap().resman.contains(&key));
    std::fs::write(user.join("override/mg_reload_test.2da"), "2DA V2.0\n\n   Label\n0  x\n")
        .unwrap();
    app.run(mg_ui::Action::ReloadResources);
    assert!(app.game.as_deref().unwrap().resman.contains(&key), "{:?}", app.log.entries);
    assert!(app.log.entries.iter().any(|(_, m)| m == "Reloaded override"), "{:?}", app.log.entries);
    app.run(mg_ui::Action::ReloadResources);
    assert!(app.log.entries.iter().any(|(_, m)| m == "Reload Resources: nothing changed"));
}

/// The override folder is watched off the window's thread: a file new to
/// it is the game data's within a few seconds, without being asked for,
/// and nothing is read again while nothing changed.
#[test]
fn a_file_new_to_the_override_is_seen_by_the_watch() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-reload-watch");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    let mut app = game_app(&root, Some(user.clone()));
    let key = ResKey::parse("mg_watch_test", ResType::TWODA).unwrap();
    // (The watch begins; what it says at its start finds nothing changed.)
    app.reload_changed_content();
    std::thread::sleep(std::time::Duration::from_millis(300));
    app.reload_changed_content();
    assert!(!app.log.entries.iter().any(|(_, m)| m.starts_with("Reloaded")));
    std::fs::write(user.join("override/mg_watch_test.2da"), "2DA V2.0\n\n   Label\n0  x\n")
        .unwrap();
    let had = |app: &Moonglow| app.game.as_deref().unwrap().resman.contains(&key);
    for _ in 0..80 {
        if had(&app) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        app.reload_changed_content();
    }
    assert!(had(&app), "{:?}", app.log.entries);
    assert!(app.log.entries.iter().any(|(_, m)| m == "Reloaded override"));
}

#[test]
fn browse_view_and_copy_a_game_resource() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-browser");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let dialogs = NoDialogs { folders: vec![out.clone()], ..Default::default() };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.run();
    // Those listed go out as loose files.
    h.state_mut().browser.filter = "nw_it_torch00".into();
    h.run();
    h.get_by_label_contains("as Files…").click();
    h.run();
    assert!(out.join("nw_it_torch001.uti").is_file());
    // The module's own script is listed from the module layer.
    h.state_mut().browser.filter = "hello".into();
    h.run();
    h.get_by_label("hello.nss");
    h.get_by_label("module");

    // A game blueprint: listed from the keys, viewed read-only, copied in.
    h.state_mut().browser.filter = "nw_it_torch001".into();
    h.run();
    h.get_by_label("nw_it_torch001.uti").click_secondary();
    h.run();
    h.get_by_label("Open").click();
    h.run();
    let key = ResKey::parse("nw_it_torch001", ResType::UTI).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Resource(key)).is_some());
    h.get_by_label("LocalizedName");
    h.get_by_label("Copy to Module").click();
    h.run();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key));
    assert!(
        h.state().dock.find_tab(&Tab::Blueprint(key)).is_some(),
        "the copy opens in its editor"
    );
}

/// A module with two haks of our own in `user`'s hak folder, each with a
/// portraits.2da (its one row's label the hak's name), the first over
/// the second.
fn module_with_two_tables(dir: &std::path::Path, user: &std::path::Path) -> std::path::PathBuf {
    std::fs::create_dir_all(user.join("hak")).unwrap();
    for name in ["mg_top", "mg_low"] {
        let mut w = mg_erf::ErfWriter::new(*b"HAK ");
        let table = format!("2DA V2.0\n\n LABEL\n0 {name}\n").into_bytes();
        w.add(ResRef::from_str("portraits").unwrap(), ResType::TWODA, table).unwrap();
        std::fs::write(user.join(format!("hak/{name}.hak")), w.to_bytes().unwrap()).unwrap();
    }
    let path = sample_module(dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    for h in ["mg_top", "mg_low"] {
        let mut s = ifo::MOD_HAK_LIST.new_item();
        s.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from(h));
        info.root.items_mut(&ifo::MOD_HAK_LIST).push(s);
    }
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

/// The resource browser lists every copy of a resource: the one the game
/// reads, and under it those it overrides, each opened, copied to the
/// module and saved as its layer has it (a builder's report: of a
/// portraits.2da in two haks, only the higher was found).
#[test]
fn the_browser_lists_the_copies_a_higher_layer_overrides() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-browser-copies");
    let user = dir.join("user");
    let path = module_with_two_tables(&dir, &user);
    let saved = dir.join("saved.2da");
    let install = mg_resman::GameInstall::new(&root, Some(user), "en");
    let exported = dir.join("exported");
    let dialogs = NoDialogs {
        save: vec![saved.clone()],
        folders: vec![exported.clone()],
        ..Default::default()
    };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.state_mut().browser.filter = "portraits".into();
    h.state_mut().browser.restype = Some(ResType::TWODA);
    h.run();
    // The hak the game reads it from, then the other hak's and the game's,
    // overridden by it.
    h.get_by_label("hak:mg_top");
    h.get_by_label("hak:mg_low, overridden by hak:mg_top");
    let game = h.query_all_by_label_contains(", overridden by hak:mg_top").count();
    assert!(game >= 2, "the game's own too ({game})");
    h.get_by_label_contains("1 of ");
    h.get_by_label_contains(&format!("{game} overridden copies"));
    let rows: Vec<_> = h.get_all_by_label("portraits.2da").collect();
    assert_eq!(rows.len(), 1 + game, "a row a copy");

    // The lower hak's, opened: its own row, said to be overridden.
    rows[1].click_secondary();
    h.run();
    h.get_by_label("Open").click();
    h.run();
    let key = ResKey::parse("portraits", ResType::TWODA).unwrap();
    let tab = Tab::ResourceCopy { key, layer: "hak:mg_low".into() };
    assert!(h.state().dock.find_tab(&tab).is_some());
    h.get_by_label("mg_low");
    assert!(h.query_by_label("mg_top").is_none(), "the lower hak's copy, not the higher's");
    h.get_by_label("Overridden by hak:mg_top, whose copy the game reads.");
    // Copied to the module, as that hak has it; and saved.
    h.get_by_label("Copy to Module").click();
    h.run();
    let module = &h.state().ws.as_ref().unwrap().module;
    let data = module.get(&key).unwrap();
    assert!(String::from_utf8_lossy(data).contains("mg_low"));
    h.get_by_label("Save As…").click();
    h.run();
    assert!(std::fs::read_to_string(&saved).unwrap().contains("mg_low"));

    // Exported as files: the copy the game reads, and each it overrides
    // in a folder of its layer's.
    close_tab(h.state_mut(), &tab);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.run();
    h.get_by_label_contains("as Files…").click();
    h.run();
    let read = |p: &str| std::fs::read_to_string(exported.join(p)).unwrap_or_default();
    assert!(read("portraits.2da").contains("mg_top"), "the one the game reads");
    assert!(read("hak_mg_low/portraits.2da").contains("mg_low"), "the lower hak's");
    let folders =
        std::fs::read_dir(&exported).unwrap().filter(|e| e.as_ref().unwrap().path().is_dir());
    assert_eq!(folders.count(), game, "a folder a copy overridden");
    let said = format!("{game} overridden copies in folders of their layers");
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains(&said)), "said");

    // Without the overridden copies: the one the game reads.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.state_mut().browser.show_copies = false;
    h.run();
    assert_eq!(h.query_all_by_label_contains(", overridden by ").count(), 0);
    h.get_by_label("hak:mg_top");
    h.get_by_label_contains("1 of ");
}

/// The resource browser's Preview shows an overridden copy as its layer
/// has it: the game's own model under one of ours in override (a builder's
/// report on 2.0.0: Preview showed only the copy the game reads).
#[test]
fn an_overridden_copy_is_previewed_as_its_layer_has_it() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let dir = mg_testkit::scratch_dir("ui-preview-copy");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    // Ours: a dummy and two quads.
    let quad = |name: &str, x: f32| {
        format!(
            "node trimesh {name}\n  parent plc_a01\n  bitmap white\n  verts 4\n    -1 1 0\n    \
             -1 -1 0\n    1 1 0\n    1 -1 0\n  tverts 4\n    0 1 0\n    0 0 0\n    1 1 0\n    \
             1 0 0\n  faces 2\n    0 1 2 1 0 1 2 1\n    3 2 1 1 3 2 1 1\n  position {x} 0 0\n\
             endnode\n"
        )
    };
    let ours = format!(
        "newmodel plc_a01\nsetsupermodel plc_a01 NULL\nclassification character\n\
         beginmodelgeom plc_a01\nnode dummy plc_a01\n  parent NULL\nendnode\n{}{}\
         endmodelgeom plc_a01\ndonemodel plc_a01\n",
        quad("left", -1.0),
        quad("right", 1.0)
    );
    std::fs::write(user.join("override/plc_a01.mdl"), ours).unwrap();
    let path = sample_module(&dir);
    let mut app = game_app(&root, Some(user));
    app.set_render_state(rs.clone());
    app.settings.game_shaders = Some(false);
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    let rm = &app.game.as_deref().unwrap().resman;
    assert_eq!(rm.origin(&key), Some("override"));
    let game_layer = rm.copies(&key).into_iter().nth(1).map(|i| rm.layers()[i].label.clone());
    let game_layer = game_layer.expect("the game's own under it");
    app.actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    app.browser.filter = "plc_a01".into();
    app.browser.restype = Some(ResType::MDL);
    let mut h = gpu_harness(app, rs, 1100.0, 800.0);
    h.run();
    // The game's copy's row, under ours: its menu has Preview.
    let rows: Vec<_> = h.get_all_by_label("plc_a01.mdl").collect();
    assert!(rows.len() >= 2, "ours and the game's");
    rows[1].click_secondary();
    h.run();
    h.get_by_label("Preview").click();
    h.run();
    let tab = Tab::ModelCopy { key, layer: game_layer.clone() };
    assert!(h.state().dock.find_tab(&tab).is_some(), "the copy's viewer");
    // (What the viewer says of the model: `PLC_A01: 22 nodes, 12 meshes`.)
    let said = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let node = |n: egui_kittest::Node<'_>| n.accesskit_node().value();
        h.query_all_by_label_contains(" nodes, ").filter_map(node).collect()
    };
    let ours = |h: &Harness<'_, Moonglow>| {
        said(h).iter().any(|s| s.to_lowercase() == "plc_a01: 3 nodes, 2 meshes")
    };
    assert_eq!(said(&h).len(), 1, "a model shown");
    assert!(!ours(&h), "the game's model, not ours: {:?}", said(&h));
    assert!(h.state().model_views.keys().any(|s| s.to_string().ends_with(&game_layer))); // The model the game reads, ours, in the viewer of the resource.
    close_tab(h.state_mut(), &tab);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    h.run();
    assert!(ours(&h), "ours, which overrides it");
}

/// Whether `app`'s game data has the two haks of [`module_with_two_tables`]
/// layered, the higher read first, without the log saying that they could
/// not be.
fn has_the_two_haks(app: &Moonglow) -> Result<(), String> {
    let game = app.game.as_deref().ok_or("no game data")?;
    let labels: Vec<&str> = game.resman.layers().iter().map(|l| l.label.as_str()).collect();
    let key = ResKey::parse("portraits", ResType::TWODA).unwrap();
    let origin = game.resman.origin(&key);
    let refused = app.log.entries.iter().any(|(_, m)| m.contains("could not be added"));
    let both = labels.contains(&"hak:mg_top") && labels.contains(&"hak:mg_low");
    if !both || origin != Some("hak:mg_top") || refused {
        return Err(format!(
            "layers {labels:?}, portraits.2da from {origin:?}; log {:?}",
            app.log.entries
        ));
    }
    Ok(())
}

/// The game data's layers of a module: its haks and itself.
fn module_layers(app: &Moonglow) -> Vec<String> {
    let game = app.game.as_deref().expect("the game data");
    (game.resman.layers().iter())
        .filter(|l| l.priority == mg_resman::priority::MODULE || l.label.starts_with("hak:"))
        .map(|l| l.label.clone())
        .collect()
}

/// A module's haks are layered into the game data when it is opened as the
/// lit check starts (`moonglow path/to/module.mod` opens it in the window's
/// first frame, the check's), and again once the check is done: a
/// builder's report on 2.13.1, where every module opened said its haks
/// "could not be added to the game data (it is in use)", closed and opened
/// again or not (the 3D views kept the game data the check had read).
#[test]
fn a_module_s_haks_are_layered_with_the_lit_check() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let dir = mg_testkit::scratch_dir("ui-haks-lit-check");
    let user = dir.join("user");
    let path = module_with_two_tables(&dir, &user);
    let mut app = game_app(&root, Some(user));
    app.set_render_state(rs.clone());
    // (As in the desktop app, where the check runs.)
    app.background_jobs = true;
    app.settings.no_last_area = true;
    app.actions.push(mg_ui::Action::OpenModule(path.clone()));
    let mut h = gpu_harness(app, rs, 1000.0, 800.0);
    h.step();
    has_the_two_haks(h.state()).unwrap();
    let started = std::time::Instant::now();
    while h.state().lit_check.found.is_none() {
        assert!(started.elapsed().as_secs() < 60, "the lit check never ended");
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.step();
    }
    // Closed, and opened again.
    h.state_mut().close();
    assert_eq!(module_layers(h.state()), Vec::<String>::new());
    h.state_mut().open_module(&path);
    has_the_two_haks(h.state()).unwrap();
}

/// After an area has been drawn, its textures read apart from the window
/// (as in the desktop app): Reload Resources looks at the game data's
/// folders, Close takes the module's layers out of the game data, and the
/// next module opened has its haks layered (in 2.13.0 too, the second
/// module opened after an area had been viewed had none).
#[test]
fn a_module_s_haks_are_layered_after_an_area_was_drawn() {
    let dir = mg_testkit::scratch_dir("ui-haks-after-area");
    let user = dir.join("user");
    let path = module_with_two_tables(&dir, &user);
    let Some((mut app, rs, area)) =
        area_app("haks-after-area", "ttr01", None, 4, Some(user), &|_, _, _| {})
    else {
        return;
    };
    app.background_jobs = true;
    let mut h = area_gpu_harness(app, rs, 1100.0, 800.0);
    let started = std::time::Instant::now();
    let pictures = |app: &Moonglow| app.area_views.get(&area).map_or(0, |v| v.pictures);
    let pending = |app: &Moonglow| app.viewport.as_ref().unwrap().renderer.drawn.textures_pending;
    let reading = |app: &Moonglow| app.viewport.as_ref().unwrap().renderer.textures_read() > 0;
    // (The first picture, its textures read apart: some still to come,
    // as like as not.)
    while pictures(h.state()) == 0 || !reading(h.state()) {
        assert!(started.elapsed().as_secs() < 60, "the area was never drawn");
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.step();
    }
    let (was, waiting) = (pictures(h.state()), pending(h.state()));
    eprintln!("textures still to come as Reload Resources ran: {waiting}");
    h.state_mut().reload_resources(true);
    let entries = &h.state().log.entries;
    assert!(entries.iter().any(|(_, m)| m == "Reload Resources: nothing changed"), "{entries:?}");
    // The view draws on, with every texture: those asked for when the
    // views let go of the game data are asked for again.
    while pending(h.state()) > 0 || pictures(h.state()) == was {
        assert!(started.elapsed().as_secs() < 60, "textures never read: {}", pending(h.state()));
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.step();
    }
    // Closed: the area module's layer is gone.
    h.state_mut().close();
    assert_eq!(module_layers(h.state()), Vec::<String>::new());
    h.state_mut().settings.no_last_area = true;
    h.state_mut().open_module(&path);
    has_the_two_haks(h.state()).unwrap();
    // (And the area view of the next module draws as before.)
    h.run_steps(2);
}

/// A module closed while its area is still read on a thread of its own:
/// the window waits for the reading to let go of the game data, and the
/// next module's haks are layered.
#[test]
fn a_module_s_haks_are_layered_after_one_closed_while_its_area_was_read() {
    let dir = mg_testkit::scratch_dir("ui-haks-area-read");
    let user = dir.join("user");
    let path = module_with_two_tables(&dir, &user);
    let Some((mut app, rs, area)) =
        area_app("haks-area-read", "ttr01", None, 4, Some(user), &|_, _, _| {})
    else {
        return;
    };
    app.background_jobs = true;
    let mut h = area_gpu_harness(app, rs, 1100.0, 800.0);
    // (The tab opened, then the reading started.)
    h.run_steps(2);
    let read = h.state().area_views.get(&area).is_some_and(|v| v.pictures > 0);
    eprintln!("the area read before it was closed: {read}");
    h.state_mut().close();
    assert_eq!(module_layers(h.state()), Vec::<String>::new());
    h.state_mut().settings.no_last_area = true;
    h.state_mut().open_module(&path);
    has_the_two_haks(h.state()).unwrap();
}

/// An older module's single hak (`Mod_Hak`, and no list) is layered, as
/// `mg` layers it, and Module Properties says so rather than that the
/// module has none.
#[test]
fn an_older_module_s_single_hak_is_layered_and_said() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-hak-older");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
    let key = ResKey::parse("mg_older_hak", ResType::TWODA).unwrap();
    hak.add(key.resref, key.restype, b"2DA V2.0\n".to_vec()).unwrap();
    std::fs::write(user.join("hak/older.hak"), hak.to_bytes().unwrap()).unwrap();
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_HAK, ExoString::from("older"));
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    assert!(app.game.as_ref().unwrap().resman.contains(&key), "the hak is layered");
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.get_by_label("Custom Content").click();
    h.run();
    h.get_by_label("Hak Files (0)");
    h.get_by_label_contains("names its hak, older, in the older single field (Mod_Hak)");
    assert!(h.query_by_label_contains("uses the game's own resources").is_none());
}

/// Module Properties › Custom Content: the module's haks in a list of
/// their own, numbered from the one that overrides the others. Rows are
/// chosen (Ctrl and Shift for more) and moved by the keys, the arrows
/// under the list or a drag, and removed, one undo each; a hak not in the
/// hak folders is flagged.
#[test]
fn the_hak_list_is_reordered_by_keys_buttons_and_a_drag() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-hak-list");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    for name in ["alpha", "bravo", "charlie"] {
        let empty = mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap();
        std::fs::write(user.join(format!("hak/{name}.hak")), empty).unwrap();
    }
    let path = sample_module(&dir);
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    let set = |h: &mut Harness<'_, Moonglow>, names: &[&str]| {
        let items = names
            .iter()
            .map(|n| {
                let mut item = ifo::MOD_HAK_LIST.new_item();
                item.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from(*n));
                item
            })
            .collect();
        let edit = mg_edit::Edit::SetField {
            key: info,
            path: mg_edit::GffPath::root(),
            label: ifo::MOD_HAK_LIST.label.to_string(),
            value: Some(mg_gff::Value::List(items)),
        };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Haks", vec![edit])));
        h.run();
    };
    let haks = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let root = ws.doc(&info).unwrap().root.clone();
        root.items(&ifo::MOD_HAK_LIST)
            .iter()
            .map(|i| {
                String::from_utf8_lossy(i.read(&ifo::mod_hak_list::MOD_HAK).as_bytes()).into_owned()
            })
            .collect()
    };
    let chosen = |h: &Harness<'_, Moonglow>| -> Vec<usize> {
        h.state().hak_list.selected.iter().copied().collect()
    };
    h.get_by_label("Custom Content").click();
    h.run();
    set(&mut h, &["alpha", "bravo", "charlie", "gone"]);
    h.get_by_label("Hak Files (4)");
    h.get_by_label("Top overrides lower");
    assert_eq!(h.query_all_by_label("not found").count(), 1, "the one not in the hak folder");

    // A click chooses a row; Alt + Up moves it up, one undo back.
    h.get_by_label("bravo").click();
    h.run();
    assert_eq!(chosen(&h), [1]);
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowUp);
    h.run();
    assert_eq!(haks(&mut h), ["bravo", "alpha", "charlie", "gone"]);
    assert_eq!(chosen(&h), [0], "what moved stays chosen");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(haks(&mut h), ["alpha", "bravo", "charlie", "gone"]);

    // Down chooses the next; with Shift, more; the arrow under the list
    // moves what is chosen down together.
    h.get_by_label("alpha").click();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(chosen(&h), [1]);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    h.run();
    assert_eq!(chosen(&h), [1, 2]);
    // (The last: the first is the module tree's Expand All.)
    h.get_all_by_label("⏷").last().unwrap().click();
    h.run();
    assert_eq!(haks(&mut h), ["alpha", "gone", "bravo", "charlie"]);
    assert_eq!(chosen(&h), [2, 3]);
    // (At the end: no further.)
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowDown);
    h.run();
    assert_eq!(haks(&mut h), ["alpha", "gone", "bravo", "charlie"]);

    // A row dragged by its name to before another: it is among the two
    // chosen, which go together.
    let from = h.get_by_label("charlie").rect().center();
    let to = h.get_by_label("alpha").rect().center() - egui::vec2(0.0, 6.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let drag = |h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2| {
        h.hover_at(from);
        h.run_steps(1);
        h.event(button(from, true));
        for k in 1..=6 {
            h.hover_at(from + (to - from) * (k as f32 / 6.0));
            h.run_steps(1);
        }
        h.event(button(to, false));
        h.run();
    };
    drag(&mut h, from, to);
    assert_eq!(haks(&mut h), ["bravo", "charlie", "alpha", "gone"]);
    assert_eq!(chosen(&h), [0, 1]);
    // One not among those chosen is dragged alone: to the end.
    let from = h.get_by_label("alpha").rect().center();
    let to = h.get_by_label("gone").rect().center() + egui::vec2(0.0, 6.0);
    drag(&mut h, from, to);
    assert_eq!(haks(&mut h), ["bravo", "charlie", "gone", "alpha"]);
    assert_eq!(chosen(&h), [3]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(haks(&mut h), ["bravo", "charlie", "alpha", "gone"], "one undo for a drag");

    // Ctrl + click chooses another too; Delete removes them, one undo.
    h.get_by_label("alpha").click();
    h.run();
    h.get_by_label("gone").click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    assert_eq!(chosen(&h), [2, 3]);
    // Ctrl + Shift + click adds the rows from the one clicked last, as
    // every list of several does (it was a Ctrl + click here once).
    h.get_by_label("bravo").click();
    h.run();
    h.get_by_label("alpha").click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    h.get_by_label("gone").click_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT);
    h.run();
    assert_eq!(chosen(&h), [0, 2, 3]);
    h.get_by_label("bravo").click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    assert_eq!(chosen(&h), [2, 3]);
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(haks(&mut h), ["bravo", "charlie"]);
    assert_eq!(h.query_all_by_label("not found").count(), 0);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(haks(&mut h), ["bravo", "charlie", "alpha", "gone"]);
    // Remove under the list does the same for the rows chosen.
    h.get_by_label("bravo").click();
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    assert_eq!(haks(&mut h), ["charlie", "alpha", "gone"]);
}

/// Module Properties' Add hak… by the keys (GitHub issue 7): once its box
/// is clicked, Down opens it at its first hak, Down and Up move along
/// them, Enter adds the one there and closes the list.
#[test]
fn add_hak_takes_the_arrow_keys() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-add-hak-keys");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    for name in ["alpha", "bravo", "charlie"] {
        let empty = mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap();
        std::fs::write(user.join(format!("hak/{name}.hak")), empty).unwrap();
    }
    let path = sample_module(&dir);
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    h.get_by_label("Custom Content").click();
    h.run_steps(3);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let add = |h: &Harness<'_, Moonglow>| {
        h.query_all(combo.clone())
            .find(|n| n.accesskit_node().value().as_deref() == Some("Add hak…"))
            .map(|n| n.rect().center())
            .expect("Add hak…")
    };
    // A click opens it, another closes it, the box keeping the keys.
    let at = add(&h);
    h.hover_at(at);
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
    }
    assert!(h.query_by_label("bravo").is_none(), "closed");
    for key in
        [egui::Key::ArrowDown, egui::Key::ArrowDown, egui::Key::ArrowDown, egui::Key::ArrowUp]
    {
        h.key_press(key);
        h.run_steps(2);
    }
    assert!(h.query_by_label("charlie").is_some(), "open");
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let root = ws.doc(&info).unwrap().root.clone();
    let haks: Vec<String> = root
        .items(&ifo::MOD_HAK_LIST)
        .iter()
        .map(|i| String::from_utf8_lossy(i.read(&ifo::mod_hak_list::MOD_HAK).as_bytes()).into())
        .collect();
    assert_eq!(haks, ["bravo"], "the second, Down twice from the first and Up once");
    assert!(h.query_by_label("charlie").is_none(), "the list closed");
}

/// A hak dragged to the bottom edge of a list longer than its frame
/// scrolls the list (GitHub issue 17), and is let go among the rows that
/// came into sight.
#[test]
fn a_hak_dragged_to_the_list_s_edge_scrolls_it() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-hak-list-scroll");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let path = sample_module(&dir);
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    let names: Vec<String> = (1..=30).map(|n| format!("hak{n:02}")).collect();
    let items = names
        .iter()
        .map(|n| {
            let mut item = ifo::MOD_HAK_LIST.new_item();
            item.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from(n.as_str()));
            item
        })
        .collect();
    let edit = mg_edit::Edit::SetField {
        key: info,
        path: mg_edit::GffPath::root(),
        label: ifo::MOD_HAK_LIST.label.to_string(),
        value: Some(mg_gff::Value::List(items)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Haks", vec![edit])));
    h.run();
    h.get_by_label("Custom Content").click();
    h.run();
    let haks = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let root = ws.doc(&info).unwrap().root.clone();
        root.items(&ifo::MOD_HAK_LIST)
            .iter()
            .map(|i| {
                String::from_utf8_lossy(i.read(&ifo::mod_hak_list::MOD_HAK).as_bytes()).into_owned()
            })
            .collect()
    };
    // The frame shows 14 rows: the last ones are out of sight, below.
    let last = |h: &Harness<'_, Moonglow>| h.get_by_label("hak28").rect().top();
    let hidden = last(&h);
    let edge = h.get_by_label("hak14").rect().bottom();
    assert!(hidden > edge + 100.0, "out of sight: {hidden}, {edge}");
    // hak02 dragged by its name down to the frame's bottom edge, and held
    // there: the list scrolls up under it.
    let from = h.get_by_label("hak02").rect().center();
    let to = egui::pos2(from.x, edge - 4.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.hover_at(from);
    h.run_steps(1);
    h.event(button(from, true));
    for k in 1..=6 {
        h.hover_at(from + (to - from) * (k as f32 / 6.0));
        h.run_steps(1);
    }
    h.run_steps(60);
    let scrolled = last(&h);
    assert!(scrolled < hidden - 100.0, "the list scrolled: {hidden} to {scrolled}");
    // Let go above hak28, now in sight.
    let at = egui::pos2(from.x, h.get_by_label("hak28").rect().top() + 2.0);
    h.hover_at(at);
    h.run_steps(1);
    h.event(button(at, false));
    h.run();
    let order = haks(&mut h);
    let place = order.iter().position(|n| n == "hak02").unwrap();
    assert!(place > 20, "let go among the rows that came into sight: {order:?}");
    assert_eq!(order[place + 1], "hak28", "{order:?}");
}

/// Files added to a hak that has some of them already are asked about
/// first, as Aurora's hak editor asks: replaced, left out, or nothing
/// added at all.
#[test]
fn files_a_hak_already_has_are_asked_about_before_they_are_replaced() {
    let dir = mg_testkit::scratch_dir("ui-hak-add");
    let table = |text: &str| format!("2DA V2.0\n\n   Label\n0  {text}\n");
    let (old, new) = (dir.join("old"), dir.join("new"));
    for d in [&old, &new] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(old.join("mg_one.2da"), table("old")).unwrap();
    std::fs::write(new.join("mg_one.2da"), table("new one")).unwrap();
    std::fs::write(new.join("mg_two.2da"), table("two")).unwrap();
    let chosen = vec![new.join("mg_one.2da"), new.join("mg_two.2da")];
    // (Add Files… is answered three times, the last first.)
    let dialogs = NoDialogs {
        open_many: vec![chosen.clone(), chosen.clone(), chosen, vec![old.join("mg_one.2da")]],
        ..Default::default()
    };
    let app = Moonglow::new(None, Box::new(dialogs));
    let mut h = harness(app, 1100.0, 760.0);
    mg_ui::hak_view::new_hak(h.state_mut());
    h.run();
    let sizes = |h: &Harness<'_, Moonglow>| -> Vec<(String, u64)> {
        let mut items: Vec<(String, u64)> =
            h.state().haks[0].hak.items().iter().map(|i| (i.key.to_string(), i.size)).collect();
        items.sort();
        items
    };
    let (was, now) = (table("old").len() as u64, table("new one").len() as u64);
    // Nothing of the name there yet: added without a word.
    h.get_by_label("Add Files…").click();
    h.run();
    assert_eq!(sizes(&h), [("mg_one.2da".to_string(), was)]);
    assert!(h.query_by_label("Files Already in the Hak").is_none());
    // One of the two chosen is there: asked. Cancel adds nothing.
    h.get_by_label("Add Files…").click();
    h.run();
    h.get_by_label("Files Already in the Hak");
    assert!(h.query_by_label_contains("already has 1 of the 2 files").is_some());
    assert_eq!(sizes(&h).len(), 1, "nothing added before the answer");
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!(sizes(&h), [("mg_one.2da".to_string(), was)]);
    // Skip Those: the other is added, the hak's own stays.
    h.get_by_label("Add Files…").click();
    h.run();
    h.get_by_label("Skip Those").click();
    h.run();
    let two = table("two").len() as u64;
    assert_eq!(sizes(&h), [("mg_one.2da".to_string(), was), ("mg_two.2da".to_string(), two)]);
    // Replace: the chosen files take their place.
    h.get_by_label("Add Files…").click();
    h.run();
    assert!(h.query_by_label_contains("already has 2 of the 2 files").is_some());
    h.get_by_label("Replace 2").click();
    h.run();
    assert_eq!(sizes(&h), [("mg_one.2da".to_string(), now), ("mg_two.2da".to_string(), two)]);
}

/// The side editors (a hak's, a 2DA's, a tileset's) ask of unsaved changes
/// as the module does (Save, Don't Save, Cancel), and of each tab closed
/// in turn: Close All with two changed haks asks of both, not only the
/// last. Cancel keeps the tab open and asks no more.
#[test]
fn closing_two_changed_haks_asks_of_both() {
    let dir = mg_testkit::scratch_dir("ui-hak-close-two");
    let file = dir.join("mg_one.2da");
    std::fs::write(&file, "2DA V2.0\n\n   Label\n0  one\n").unwrap();
    let app = Moonglow::new(None, Box::new(NoDialogs::default()));
    let mut h = harness(app, 1400.0, 900.0);
    for _ in 0..2 {
        mg_ui::hak_view::new_hak(h.state_mut());
        h.run();
    }
    let ids: Vec<u32> = h.state().haks.iter().map(|d| d.id).collect();
    for (d, name) in h.state_mut().haks.iter_mut().zip(["first.hak", "second.hak"]) {
        d.hak.path = Some(dir.join(name));
        d.hak.add_files(std::slice::from_ref(&file));
        assert!(d.hak.is_dirty());
    }
    let tabs = || ids.iter().map(|&id| Tab::Hak(id)).collect::<Vec<_>>();
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(tabs()));
    h.run();
    h.get_by_label("Save changes to first.hak?");
    assert!(h.query_by_label_contains("1 more tab has unsaved changes").is_some());
    assert!(h.query_by_label("Discard").is_none(), "said as the module's question says it");
    h.get_by_label("Don't Save").click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Hak(ids[0])).is_none(), "the first closed");
    // The second is asked of too.
    h.get_by_label("Save changes to second.hak?");
    assert!(h.query_by_label_contains("more tab").is_none());
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.query_by_label("Save changes to second.hak?").is_none());
    assert!(h.state().dock.find_tab(&Tab::Hak(ids[1])).is_some(), "kept open");
    assert_eq!(h.state().haks.len(), 1);
    // Closed again and saved: written, and closed.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(tabs()));
    h.run();
    h.get_by_label("Save changes to second.hak?");
    h.get_all_by_label("Save").last().unwrap().click();
    h.run();
    assert!(dir.join("second.hak").is_file());
    assert!(!dir.join("first.hak").exists(), "the first was not saved");
    assert!(h.state().haks.is_empty());
    assert!(h.state().dock.find_tab(&Tab::Hak(ids[1])).is_none());
}

/// Undo and Redo over a hak's editor are the hak's (Ctrl+Z, Ctrl+Shift+Z),
/// not the module's: with the pointer over it, even with nothing of the
/// hak's left to undo. Away from it, they are the module's again.
#[test]
fn undo_over_a_hak_editor_is_the_hak_s() {
    let dir = mg_testkit::scratch_dir("ui-hak-undo");
    let file = dir.join("mg_one.2da");
    std::fs::write(&file, "2DA V2.0\n\n   Label\n0  one\n").unwrap();
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(SMALL_TABLE.to_vec()) }],
    )));
    let mut h = harness(app, 1800.0, 1000.0);
    h.run();
    mg_ui::hak_view::new_hak(h.state_mut());
    h.run();
    h.state_mut().haks[0].hak.add_files(std::slice::from_ref(&file));
    h.run();
    let items = |h: &Harness<'_, Moonglow>| h.state().haks[0].hak.items().len();
    let module_step =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    assert_eq!((items(&h), module_step(&h).as_deref()), (1, Some("A table")));
    let ctrl = egui::Modifiers::COMMAND;
    h.hover_at(h.get_by_label("Add Files…").rect().center());
    h.run();
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run();
    assert_eq!(items(&h), 0, "the hak's step undone");
    assert_eq!(module_step(&h).as_deref(), Some("A table"), "not the module's");
    // Nothing of the hak's to undo: nothing is, not the module's.
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run();
    assert_eq!(module_step(&h).as_deref(), Some("A table"));
    h.key_press_modifiers(ctrl | egui::Modifiers::SHIFT, egui::Key::Z);
    h.run();
    assert_eq!(items(&h), 1, "redone");
    // Away from the editor (over the module tree), the module's.
    h.hover_at(h.get_by_label("Areas (1)").rect().center());
    h.run();
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run();
    assert_eq!(items(&h), 1);
    assert_ne!(module_step(&h).as_deref(), Some("A table"), "the module's step undone");
}

/// The hak editor's View shows a picture as a picture, a sound with Play,
/// a 2DA as rows and columns, and the rest as the resource browser does
/// (a model not in the load order as its text).
#[test]
fn the_hak_editor_shows_pictures_sounds_and_tables() {
    let dir = mg_testkit::scratch_dir("ui-hak-view");
    let mut image = mg_image::Rgba::new(8, 4);
    image.data.fill(200);
    let samples: Vec<u8> = [0i16; 800].iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut wav = b"RIFF".to_vec();
    wav.extend((36 + samples.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    for n in [1u16, 1] {
        wav.extend(n.to_le_bytes());
    }
    for n in [8000u32, 16000] {
        wav.extend(n.to_le_bytes());
    }
    for n in [2u16, 16] {
        wav.extend(n.to_le_bytes());
    }
    wav.extend(b"data");
    wav.extend((samples.len() as u32).to_le_bytes());
    wav.extend(samples);
    let files = [
        ("mg_pic.tga", mg_image::write_tga(&image)),
        ("mg_snd.wav", wav),
        (
            "mg_tab.2da",
            b"2DA V2.0\n\n   Label    Model\n0  first    mg_a\n1  second   mg_b\n".to_vec(),
        ),
        ("mg_note.txt", b"just a note".to_vec()),
    ];
    let mut hak = mg_module::hak_edit::Hak::new();
    let sources = files
        .iter()
        .map(|(n, d)| {
            let key = ResKey::from_filename(n).unwrap();
            (key, mg_module::hak_edit::Source::Bytes(d.clone().into()))
        })
        .collect();
    hak.add_sources(sources).unwrap();
    let path = dir.join("mg_view.hak");
    hak.save(Some(&path)).unwrap();
    let mut h = harness(app_with(Vec::new()), 1400.0, 900.0);
    h.run();
    mg_ui::hak_view::open_at(h.state_mut(), &path);
    h.run();
    let view = |h: &mut Harness<'_, Moonglow>, name: &str| {
        h.get_by_label(name).click_secondary();
        h.run();
        h.get_all_by_label("View").last().unwrap().click();
        h.run();
        h.run();
        let shown = h.state().haks[0].viewing().unwrap();
        assert_eq!(shown.0.to_string(), name);
        shown.1
    };
    assert_eq!(view(&mut h, "mg_pic.tga"), "picture");
    h.get_by_label_contains("8 by 4 pixels");
    assert_eq!(view(&mut h, "mg_snd.wav"), "sound");
    h.get_by_label("Play").click();
    h.run();
    h.get_by_label("Stop");
    assert_eq!(view(&mut h, "mg_tab.2da"), "table");
    // (Its columns' names over its cells.)
    h.get_by_label("Model");
    h.get_by_label("second");
    assert_eq!(view(&mut h, "mg_note.txt"), "plain");
    h.get_by_label_contains("just a note");
    h.get_by_label("Close").click();
    h.run();
    assert!(h.state().haks[0].viewing().is_none());
}

/// The hak editor's Compile Models compiles the hak's models kept as
/// text, one step to undo; a model whose supermodel is nowhere stays as
/// text and is named in the log, and a compiled model shows as text.
#[test]
fn the_hak_editor_compiles_the_models_kept_as_text() {
    let dir = mg_testkit::scratch_dir("ui-hak-models");
    let base = "newmodel base\nsetsupermodel base NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom base\nnode dummy base\n  parent NULL\nendnode\n\
node dummy arm\n  parent base\nendnode\nendmodelgeom base\ndonemodel base\n";
    let orphan = base.replace("base", "orphan").replace("orphan NULL", "orphan mg_nowhere_zz");
    std::fs::write(dir.join("base.mdl"), base).unwrap();
    std::fs::write(dir.join("orphan.mdl"), &orphan).unwrap();
    let dialogs = NoDialogs {
        open_many: vec![vec![dir.join("base.mdl"), dir.join("orphan.mdl")]],
        ..Default::default()
    };
    let app = Moonglow::new(None, Box::new(dialogs));
    let mut h = harness(app, 1100.0, 760.0);
    mg_ui::hak_view::new_hak(h.state_mut());
    h.run();
    h.get_by_label("Add Files…").click();
    h.run();
    let model = |h: &Harness<'_, Moonglow>, name: &str| {
        h.state().haks[0].hak.data(ResKey::parse(name, ResType::MDL).unwrap()).unwrap()
    };
    assert!(!mg_mdl::is_binary(&model(&h, "base")));
    h.get_by_label("Compile Models").click();
    h.run();
    assert!(mg_mdl::is_binary(&model(&h, "base")), "compiled");
    assert_eq!(model(&h, "orphan"), orphan.as_bytes(), "left as text");
    let said = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state().log.entries.iter().any(|(_, m)| m.contains(what))
    };
    assert!(
        said(&h, "1 compiled") && said(&h, "Left as text: orphan.mdl"),
        "{:?}",
        h.state().log.entries
    );
    // One step to undo, named for what it was.
    assert_eq!(h.state().haks[0].hak.undo_label(), Some("Compile Models"));
    h.state_mut().haks[0].hak.undo();
    assert!(!mg_mdl::is_binary(&model(&h, "base")));
    // As the window runs it: on a thread of its own (no module is open),
    // the hak changed when the work is handed back.
    h.state_mut().background_jobs = true;
    h.run();
    h.get_by_label("Compile Models").click();
    let started = std::time::Instant::now();
    while !mg_mdl::is_binary(&model(&h, "base")) {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "not compiled");
    }
    h.run_steps(2);
    assert!(!h.state().busy(), "the job is done");
    assert_eq!(model(&h, "orphan"), orphan.as_bytes());
}

#[test]
fn hak_built_from_a_folder_attached_edited_and_reloaded() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-hak-editor");
    let (user, content, download) = (dir.join("user"), dir.join("mg_ui_hak"), dir.join("download"));
    for d in [&user, &content, &download] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(content.join("mg_ui_test.2da"), "2DA V2.0\n\n   Label\n0  hak\n").unwrap();
    std::fs::write(content.join("mg_ui_other.2da"), "2DA V2.0\n\n   Label\n0  other\n").unwrap();
    std::fs::write(content.join("a_name_far_too_long.2da"), "x").unwrap();
    let tlk = mg_tlk::Tlk::new(Language::ENGLISH).to_bytes().unwrap();
    std::fs::write(download.join("mg_ui.tlk"), tlk).unwrap();
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    let hak = user.join("hak/mg_ui_hak.hak");
    let dialogs = NoDialogs {
        folders: vec![content.clone()],
        save: vec![hak.clone()],
        open_many: vec![vec![hak.clone(), download.join("mg_ui.tlk")]],
        open: vec![hak.clone()],
    };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();

    // Built from the folder: the name too long is left out, and said why.
    mg_ui::hak_view::build_from_folder(h.state_mut());
    h.run();
    let id = h.state().haks[0].id;
    assert!(h.state().dock.find_tab(&Tab::Hak(id)).is_some());
    h.get_by_label("mg_ui_test.2da");
    let log = |h: &Harness<'_, Moonglow>| {
        h.state().log.entries.iter().map(|e| e.1.clone()).collect::<Vec<_>>()
    };
    assert!(
        log(&h).iter().any(|m| m.contains("a_name_far_too_long.2da") && m.contains("16")),
        "{:?}",
        log(&h)
    );
    // Shift + click selects from the one clicked last to the one clicked.
    h.get_by_label("mg_ui_other.2da").click();
    h.run();
    h.get_by_label("mg_ui_test.2da").click_modifiers(egui::Modifiers::SHIFT);
    h.run();
    assert_eq!(h.state().haks[0].selected.len(), 2);
    h.get_by_label("mg_ui_test.2da").click();
    h.run();
    assert_eq!(h.state().haks[0].selected.len(), 1);
    // The Find field's keys (GitHub issue 12): Ctrl+F with the pointer
    // over the editor goes to it, Escape clears it and then leaves it.
    let over = h.get_by_label("mg_ui_test.2da").rect().center();
    h.hover_at(over);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run();
    h.event(egui::Event::Text("other".into()));
    h.run();
    assert_eq!(h.state().haks[0].filter, "other");
    assert!(h.query_by_label("mg_ui_test.2da").is_none(), "the list is narrowed");
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(h.state().haks[0].filter, "");
    h.key_press(egui::Key::Escape);
    h.run();
    h.event(egui::Event::Text("x".into()));
    h.run();
    assert_eq!(h.state().haks[0].filter, "", "left: what is typed is not the field's");
    assert!(h.state().dock.find_tab(&Tab::Hak(id)).is_some(), "and the editor is open still");
    h.get_by_label("Save").click();
    h.run();
    assert!(hak.is_file(), "{:?}", log(&h));
    assert!(!h.state().haks[0].hak.is_dirty());

    // Attached with a talk table in one step; the game data has it at once.
    let tab = h.state().dock.find_tab(&Tab::Hak(id)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().haks.clear();
    h.run();
    h.get_by_label("Custom Content").click();
    h.run();
    h.get_by_label("Add Haks and Talk Table…").click();
    h.run();
    assert!(h.state().attach.is_some());
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert!(user.join("tlk/mg_ui.tlk").is_file());
    let key = ResKey::parse("mg_ui_test", ResType::TWODA).unwrap();
    let has = |h: &Harness<'_, Moonglow>| h.state().game.as_deref().unwrap().resman.contains(&key);
    assert!(has(&h), "{:?}", log(&h));
    assert!(h.state().game.as_deref().unwrap().custom_tlk().is_some());
    // Undone, the hak leaves the game data.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(!has(&h));
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    h.run();
    assert!(has(&h));

    // Edited and saved, the module's hak is read again.
    mg_ui::hak_view::open_hak(h.state_mut());
    h.run();
    h.get_by_label("mg_ui_test.2da").click();
    h.run();
    h.get_by_label("Remove (1)").click();
    h.run();
    assert!(h.state().haks[0].hak.is_dirty());
    h.get_by_label("Undo").click();
    h.run();
    h.get_by_label("Redo").click();
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert!(!has(&h), "{:?}", log(&h));
    let other = ResKey::parse("mg_ui_other", ResType::TWODA).unwrap();
    assert!(h.state().game.as_deref().unwrap().resman.contains(&other));

    // Opened again, the hak knows the folder it was built from: Update
    // from Folder has what the folder has now (the removed table back, a
    // new file with it).
    assert_eq!(h.state().haks[0].folder.as_deref(), Some(content.as_path()));
    std::fs::write(content.join("mg_ui_new.2da"), "2DA V2.0\n\n   Label\n0  a longer one\n")
        .unwrap();
    h.get_by_label("Update from Folder").click();
    h.run();
    let names = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        h.state().haks[0].hak.items().iter().map(|i| i.key.to_string()).collect()
    };
    let mut have = names(&h);
    have.sort();
    assert_eq!(have, ["mg_ui_new.2da", "mg_ui_other.2da", "mg_ui_test.2da"]);
    // (One step to undo, not two.)
    assert_eq!(h.state().haks[0].hak.undo_label(), Some("Update from Folder"));
    // By size, the largest is listed first.
    h.state_mut().haks[0].sort = mg_ui::hak_view::Sort::Size;
    h.run();
    let first = |h: &Harness<'_, Moonglow>| {
        let rows: Vec<_> = h.query_all_by_label_contains(".2da").collect();
        rows.iter()
            .map(|n| (n.rect().top(), n.accesskit_node().label()))
            .fold((f32::MAX, None), |best, row| if row.0 < best.0 { row } else { best })
    };
    assert_eq!(first(&h).1.as_deref(), Some("mg_ui_new.2da"));
    // View (or a double click) shows a resource under the list: a table
    // as its rows and columns.
    assert!(h.state().haks[0].viewing().is_none());
    h.get_by_label("mg_ui_new.2da").click_secondary();
    h.run();
    // (The row's View: the menu bar has a View too.)
    h.get_all_by_label("View").last().unwrap().click();
    h.run();
    h.run();
    assert_eq!(h.state().haks[0].viewing().map(|v| v.1), Some("table"));
    assert!(h.query_by_label("Label").is_some(), "its column is shown");
    h.get_by_label("Close").click();
    h.run();
    assert!(h.state().haks[0].viewing().is_none());
    // A model of the hak the game reads (the module's hak, saved) is
    // drawn in the model viewer; changed and not saved, it is its text.
    let model = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    let bytes = h.state().game.as_deref().unwrap().resman.get(&model).unwrap().into_owned();
    std::fs::write(content.join("mg_ui_box.mdl"), &bytes).unwrap();
    h.get_by_label("Update from Folder").click();
    h.run();
    h.get_by_label("Save").click();
    h.run();
    let shown_as = |h: &mut Harness<'_, Moonglow>| {
        h.get_by_label("mg_ui_box.mdl").click_secondary();
        h.run();
        h.get_all_by_label("View").last().unwrap().click();
        h.run();
        h.state().haks[0].viewing().map(|v| v.1)
    };
    assert_eq!(shown_as(&mut h), Some("model"));
    std::fs::write(content.join("mg_ui_box.mdl"), &bytes[..bytes.len() - 1]).unwrap();
    h.get_by_label("Update from Folder").click();
    h.run();
    assert_eq!(shown_as(&mut h), Some("plain"));
}

/// Export as Files and Copy to Scratch Folder: a script goes out with its
/// compiled script, as it is now in the toolset (saved or not), into the
/// folder chosen; the scratch folder is asked for once, then kept.
#[test]
fn resources_are_exported_as_files() {
    let dir = mg_testkit::scratch_dir("ui-export-files");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let ncs = ResKey::parse("hello", ResType::NCS).unwrap();
    m.set(ncs, b"NCS old".to_vec());
    m.save().unwrap();
    let (chosen, scratch) = (dir.join("out"), dir.join("scratch"));
    // (The folders the dialog gives, the last first.)
    let folders = vec![scratch.clone(), chosen.clone()];
    let mut app = Moonglow::new(None, Box::new(NoDialogs { folders, ..Default::default() }));
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Compiled again in the toolset, not saved: what is exported.
    let edit = mg_edit::Edit::SetResource { key: ncs, data: Some(b"NCS new".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Compile", vec![edit])));
    h.run();
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    let export = |h: &mut Harness<'_, Moonglow>, scratch: bool| {
        let keys = vec![script];
        h.state_mut().actions.push(mg_ui::Action::ExportFiles {
            keys,
            dependencies: false,
            scratch,
        });
        h.run();
    };
    export(&mut h, false);
    assert_eq!(h.state().settings.scratch_dir, None, "Export as Files isn't the scratch folder");
    export(&mut h, true);
    for folder in [&chosen, &scratch] {
        assert_eq!(std::fs::read(folder.join("hello.ncs")).unwrap(), b"NCS new", "{folder:?}");
        assert_eq!(std::fs::read(folder.join("hello.nss")).unwrap(), b"void main() { }\n");
        assert_eq!(std::fs::read_dir(folder).unwrap().count(), 2);
    }
    let log = &h.state().log.entries;
    assert!(log.iter().any(|(_, m)| m.contains("hello.nss, hello.ncs")), "{log:?}");
    // The scratch folder is kept: not asked for again (the dialog has no
    // more folders to give).
    assert_eq!(h.state().settings.scratch_dir.as_deref(), Some(scratch.as_path()));
    std::fs::remove_file(scratch.join("hello.ncs")).unwrap();
    export(&mut h, true);
    assert!(scratch.join("hello.ncs").exists());
    // The Export window offers files too, for the resources ticked.
    h.state_mut().actions.push(mg_ui::Action::ExportDialog(vec![script]));
    h.run();
    h.get_by_label("Export 1 as Files…");
}

/// A compiled model of the load order goes out as the text it compiles
/// from: Save As Text… for one, and Export as Files with Models as text
/// ticked for those listed; without it, as it is.
#[test]
fn compiled_models_are_saved_as_text() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-models-as-text");
    let (plain, text) = (dir.join("plain"), dir.join("text"));
    for d in [&plain, &text] {
        std::fs::create_dir_all(d).unwrap();
    }
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let dialogs = NoDialogs {
        save: vec![dir.join("one.mdl")],
        folders: vec![text.clone(), plain.clone()],
        ..Default::default()
    };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    let key = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    let is_text = |p: &std::path::Path| {
        let data = std::fs::read(p).unwrap();
        !mg_mdl::is_binary(&data) && String::from_utf8_lossy(&data).contains("newmodel")
    };
    app.run(mg_ui::Action::SaveModelText(key, None));
    assert!(is_text(&dir.join("one.mdl")));
    app.run(mg_ui::Action::SaveResources(vec![(key, None)]));
    assert!(!is_text(&plain.join("plc_a01.mdl")), "as it is, unless asked");
    app.browser.models_as_text = true;
    app.run(mg_ui::Action::SaveResources(vec![(key, None)]));
    assert!(is_text(&text.join("plc_a01.mdl")));
    assert!(app.log.entries.iter().any(|(_, m)| m.contains("1 compiled models as text")));
}

/// The raw fields view names what a number stands for: an area's music,
/// ambient sounds and audio environment are shown by name (their rows of
/// ambientmusic.2da, ambientsound.2da and soundeax.2da), each in a list to
/// choose another by name.
#[test]
fn raw_fields_name_their_2da_rows() {
    let Some((mut h, area)) = area_harness("gff-rows") else { return };
    let git = ResKey::new(area, ResType::GIT);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Gff(git)));
    h.run_steps(3);
    // AreaProperties is the GIT's first struct.
    h.get_all_by_label_contains(" fields").next().expect("AreaProperties").click();
    h.run_steps(3);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let props = ws.doc(&git).unwrap().root.child("AreaProperties").unwrap().clone();
    let shown: Vec<String> = h
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .filter_map(|n| n.accesskit_node().value())
        .collect();
    let game = h.state().game.clone().unwrap();
    for field in
        ["AmbientSndDay", "AmbientSndNight", "EnvAudio", "MusicBattle", "MusicDay", "MusicNight"]
    {
        let table = mg_rules::rows::row_table("GIT ", &["AreaProperties"], field).unwrap();
        let name = game.row_name(&table, props.integer(field).unwrap()).expect(field);
        assert!(shown.contains(&name), "{field} is shown as {name:?}: {shown:?}");
    }
}

/// The raw fields view says an object's faction field of 0 puts it in the
/// PC faction, as Verify warns of it: a creature blueprint's, a placed
/// door's; not another faction's, nor a placed trigger's (the owner's
/// decision).
#[test]
fn raw_fields_warn_of_the_pc_faction() {
    use egui_kittest::kittest::Queryable;
    let dir = mg_testkit::scratch_dir("ui-raw-pc-faction");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let blueprint = |faction: u16| {
        let mut g = mg_gff::Gff::new(*b"UTC ");
        g.root.set("FactionID", mg_gff::Value::Word(faction));
        g
    };
    m.set_gff(ResKey::parse("guard", ResType::UTC).unwrap(), &blueprint(0)).unwrap();
    m.set_gff(ResKey::parse("hostile", ResType::UTC).unwrap(), &blueprint(1)).unwrap();
    let mut area = mg_gff::Gff::new(*b"GIT ");
    let placed = |faction: u32| {
        let mut s = mg_gff::Struct::new(1);
        s.set("Faction", mg_gff::Value::Dword(faction));
        s
    };
    area.root.set("Door List", mg_gff::Value::List(vec![placed(0)]));
    area.root.set("TriggerList", mg_gff::Value::List(vec![placed(0)]));
    m.set_gff(ResKey::parse("area", ResType::GIT).unwrap(), &area).unwrap();
    let path = dir.join("pcfac.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let warned = |name: &str, t: ResType, open: &[usize]| {
        let mut h = harness(app_with(Vec::new()), 1000.0, 700.0);
        h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
        h.run_steps(2);
        let key = ResKey::parse(name, t).unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Gff(key)));
        h.run_steps(3);
        for &i in open {
            h.get_all_by_label_contains(" items").nth(i).unwrap().click();
            h.run_steps(2);
            h.get_all_by_label_contains("struct").last().unwrap().click();
            h.run_steps(2);
        }
        h.query_all_by_label_contains("The PC faction").count()
    };
    assert_eq!(warned("guard", ResType::UTC, &[]), 1);
    assert_eq!(warned("hostile", ResType::UTC, &[]), 0);
    // The GIT's lists in their order: the doors', then the triggers'.
    assert_eq!(warned("area", ResType::GIT, &[0]), 1, "a placed door");
    assert_eq!(warned("area", ResType::GIT, &[1]), 0, "a placed trigger");
}

/// Build › Compile Models compiles the module's own models kept as text,
/// as one step to undo; one whose supermodel is nowhere stays as text.
#[test]
fn the_module_s_text_models_are_compiled() {
    let Some((mut h, _)) = area_harness("module-models") else { return };
    let base = "newmodel mg_base\nsetsupermodel mg_base NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom mg_base\nnode dummy mg_base\n  parent NULL\nendnode\n\
node dummy arm\n  parent mg_base\nendnode\nendmodelgeom mg_base\ndonemodel mg_base\n";
    let orphan =
        base.replace("mg_base", "mg_orphan").replace("mg_orphan NULL", "mg_orphan mg_nowhere_zz");
    let key = |n: &str| ResKey::parse(n, ResType::MDL).unwrap();
    let edits = vec![
        mg_edit::Edit::SetResource { key: key("mg_base"), data: Some(base.into()) },
        mg_edit::Edit::SetResource { key: key("mg_orphan"), data: Some(orphan.clone().into()) },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Models", edits)));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::CompileModels);
    h.run_steps(3);
    let model = |h: &mut Harness<'_, Moonglow>, n: &str| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        ws.module.get(&key(n)).unwrap().to_vec()
    };
    assert!(mg_mdl::is_binary(&model(&mut h, "mg_base")), "{:?}", h.state().log.entries);
    assert_eq!(model(&mut h, "mg_orphan"), orphan.as_bytes());
    let log = &h.state().log.entries;
    assert!(log.iter().any(|(_, m)| m.contains("Compile Models: 1 compiled")), "{log:?}");
    assert!(log.iter().any(|(_, m)| m.contains("Left as text: mg_orphan.mdl")), "{log:?}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Compile Models"));
}

/// The resource browser's Filter has the side panes' keys (GitHub issue
/// 12): Ctrl+F goes to it, Escape clears it, Down goes into the list,
/// where the arrow keys choose and Enter opens.
#[test]
fn the_resource_browser_s_filter_and_list_take_the_keys() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-browser-keys");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.run_steps(4);
    let over = h.get_by_label("Refresh").rect().center();
    h.hover_at(over);
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run_steps(2);
    h.event(egui::Event::Text("zzz".into()));
    h.run_steps(2);
    assert_eq!(h.state().browser.filter, "zzz", "typed into the Filter");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert_eq!(h.state().browser.filter, "", "Escape clears it");
    h.event(egui::Event::Text("nw_it_gem00".into()));
    h.run_steps(2);
    // Down: into the list, at its first row; again, the second.
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(h.state().browser.cursor, Some(0));
    assert_eq!(h.state().browser.filter, "nw_it_gem00", "the arrow is the list's now");
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    h.key_press(egui::Key::End);
    h.run_steps(2);
    h.key_press(egui::Key::Home);
    h.run_steps(2);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(h.state().browser.cursor, Some(1));
    // Enter opens the row the keys are at.
    let opened = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        (h.state().dock.iter_all_tabs())
            .filter_map(|(_, t)| match t {
                Tab::Resource(k) | Tab::Blueprint(k) => Some(k.to_string()),
                _ => None,
            })
            .collect()
    };
    assert!(opened(&h).is_empty());
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    let now = opened(&h);
    assert_eq!(now.len(), 1, "{now:?}");
    assert!(now[0].starts_with("nw_it_gem00"), "{now:?}");
}
