//! The Right Dock's Preview panel (the redesign's M3, T6), the Palette's
//! neighbor on the dock's tab bar: what it shows of the blueprint chosen
//! in the palette; each tab showing its panel alone, the Preview drawing
//! nothing behind the Palette's; Tools › Preview and the folded strip's
//! buttons; the tabs' words or icons by the dock's width; the arrow keys
//! on the tabs; which tab is in front as a module opens; Escape over it.
//! A blueprint's own Preview (M4's U11): following its tab, its one
//! header row, the Status Bar's hints beside it, its Preview button and
//! Pop Out.

use crate::common::*;
use mg_module::palette::BlueprintKind;
use mg_ui::Panel;

/// The sample module with the game's data, opened, at `width` by
/// `height`, with no GPU (the Preview's 3D view then says so).
fn preview_harness(name: &str, width: f32, height: f32) -> Option<Harness<'static, Moonglow>> {
    let root = mg_testkit::nwn_root()?;
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, width, height);
    settle(&mut h);
    Some(h)
}

/// The Right Dock's tab `name` (Palette or Preview).
#[track_caller]
fn dock_tab<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    h.get_by_label(mg_ui::RIGHT_DOCK).get_by_label(name)
}

/// The Right Dock with its tab `panel` in front, as Tools › Palettes and
/// Tools › Preview show it.
fn show(h: &mut Harness<'_, Moonglow>, panel: Panel) {
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(panel));
    settle(h);
    assert_eq!(h.state().right_dock_shows(), Some(panel));
}

/// Where the Preview's body is drawn. It must be in front.
fn preview_rect(h: &Harness<'_, Moonglow>) -> egui::Rect {
    h.state().preview_panel_rect().expect("the Preview panel in front")
}

/// The node `label` within the Preview's body.
#[track_caller]
fn in_preview<'t>(h: &'t Harness<'_, Moonglow>, label: &'t str) -> egui_kittest::Node<'t> {
    let preview = preview_rect(h);
    let mut found = h.query_all_by_label(label).filter(|n| preview.contains(n.rect().center()));
    let first = found.next().unwrap_or_else(|| panic!("no {label} in the Preview panel"));
    assert!(found.next().is_none(), "two or more {label} in the Preview panel");
    first
}

/// The model view of the blueprint `key`, where one is kept.
fn model_view_of(h: &Harness<'_, Moonglow>, key: ResKey) -> bool {
    h.state().model_views.contains_key(&mg_ui::model_view::Source::Resource(key))
}

/// The Preview shows the blueprint chosen in the palette (as the Preview
/// window did): the words asking for one before any is chosen; a
/// waypoint's fields; a chest also in 3D; an item also its inventory icon;
/// a creature by its first and last names. A row chosen by a click in the
/// palette, the Preview's tab then brought to the front, is the one shown.
#[test]
fn the_preview_panel_shows_the_chosen_blueprint() {
    // (A creature of the module's with a first and a last name.)
    let guard = |m: &mut mg_module::Module, _: &mg_rules::GameData, _: ResRef| {
        let text = |t: &str| {
            mg_gff::Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, t))
        };
        let mut guard = Gff::new(*b"UTC ");
        guard.root.set("FirstName", text("Hent"));
        guard.root.set("LastName", text("Fynolds"));
        m.set(ResKey::parse("guard001", ResType::UTC).unwrap(), guard.to_bytes().unwrap());
    };
    let Some((mut h, _area)) = area_harness_with("preview", "ttr01", None, 4, None, &guard) else {
        return;
    };
    show(&mut h, Panel::Preview);
    let dock = right_dock_rect(&h);
    let body = preview_rect(&h);
    assert!(dock.contains_rect(body), "in the Right Dock: {body:?} {dock:?}");
    assert!(body.height() > dock.height() - 60.0, "the dock's whole body: {body:?} {dock:?}");
    in_preview(&h, "Choose a blueprint in the palette.");
    // A waypoint: its fields.
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    settle(&mut h);
    in_preview(&h, "Blueprint ResRef");
    in_preview(&h, "nw_waypoint001");
    // A chest: also in 3D.
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.state_mut().palette.selected = Some(chest);
    settle(&mut h);
    in_preview(&h, "plc_chest1");
    let source = mg_ui::model_view::Source::Resource(chest);
    assert!(h.state().model_views[&source].holds_targets(), "the 3D view drawn");
    // An item: also its inventory icon.
    h.state_mut().palette.selected = ResKey::parse("nw_wswls001", ResType::UTI);
    settle(&mut h);
    in_preview(&h, "Item icon");
    // A creature: by its first and last names, as the module tree names it.
    h.state_mut().palette.selected = ResKey::parse("guard001", ResType::UTC);
    settle(&mut h);
    in_preview(&h, "Hent Fynolds");
    // A row clicked in the palette, the Preview's tab clicked after.
    show(&mut h, Panel::Palette);
    {
        let p = &mut h.state_mut().palette;
        (p.tiles, p.kind, p.custom) = (false, BlueprintKind::Waypoint, false);
        p.filter = "nw_wp_tavern".into();
    }
    settle(&mut h);
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    palette_row(&h, &tavern).click();
    settle(&mut h);
    assert_eq!(h.state().palette.selected, ResKey::parse("nw_wp_tavern", ResType::UTW));
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Palette), "no tab switched by itself");
    dock_tab(&h, "Preview").click();
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview));
    in_preview(&h, "nw_wp_tavern");
    in_preview(&h, &tavern);
    assert!(h.query_by_label("Hent Fynolds").is_none(), "the creature no longer shown");
}

/// A store chosen in the palette: its fields and its stock by page, under
/// them in the panel.
#[test]
fn the_preview_panel_shows_a_store_s_stock() {
    let store = |m: &mut mg_module::Module, _: &mg_rules::GameData, _: ResRef| {
        let mut utm = Gff::new(*b"UTM ");
        utm.root.set("ResRef", mg_gff::Value::ResRef(b"my_store".to_vec()));
        // (A longsword on the Weapons page, id 4; the others empty.)
        let page = |id: u32| {
            let mut s = mg_gff::Struct::new(id);
            let mut items = Vec::new();
            if id == 4 {
                let mut sword = mg_gff::Struct::new(0);
                sword.set("InventoryRes", mg_gff::Value::ResRef(b"nw_wswls001".to_vec()));
                items.push(sword);
            }
            s.set("ItemList", mg_gff::Value::List(items));
            s
        };
        utm.root.set("StoreList", mg_gff::Value::List((0..5).map(page).collect()));
        m.set(ResKey::parse("my_store", ResType::UTM).unwrap(), utm.to_bytes().unwrap());
    };
    let Some((mut h, _area)) = area_harness_with("preview-store", "ttr01", None, 4, None, &store)
    else {
        return;
    };
    h.state_mut().palette.selected = ResKey::parse("my_store", ResType::UTM);
    show(&mut h, Panel::Preview);
    in_preview(&h, "my_store");
    // (Its pages, each by its name and count: as the Preview window
    // listed them.)
    let page = in_preview(&h, "Weapons (1)").rect();
    assert!(page.top() > in_preview(&h, "Blueprint ResRef").rect().top(), "under its fields");
    // (Its icon and its name.)
    let preview = preview_rect(&h);
    let sword = h.query_all_by_label("Longsword").filter(|n| preview.contains(n.rect().center()));
    assert!(sword.count() > 0, "its item");
}

/// The Right Dock holds both tabs, Palette and Preview, beside an area;
/// each tab shows its own panel and only it: with the Palette's in front
/// the Preview draws nothing (no body, no model view, no model read: a
/// creature chosen in the palette is not loaded), and with the Preview's
/// the palette's nodes are gone from the tree. Brought to the front, the
/// Preview shows the blueprint chosen meanwhile; choosing another in the
/// palette and coming back, it follows.
#[test]
fn each_tab_shows_its_panel_and_only_it() {
    let Some((mut h, _area)) = area_harness("preview-tabs") else { return };
    let tabs = |h: &Harness<'_, Moonglow>| {
        let bar = h.get_by_label(mg_ui::RIGHT_DOCK);
        (bar.query_by_label("Palette").is_some(), bar.query_by_label("Preview").is_some())
    };
    assert_eq!(tabs(&h), (true, true), "both tabs beside the area");
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Palette), "the Palette in front");
    let creature = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    h.state_mut().palette.selected = Some(creature);
    settle(&mut h);
    // The Palette in front: the Preview draws nothing.
    palette_types(&h).expect("the palette's types");
    assert!(h.state().preview_panel_rect().is_none(), "no Preview body");
    assert!(h.query_by_label("Blueprint ResRef").is_none(), "none of its nodes");
    assert!(!model_view_of(&h, creature), "no model view made behind the Palette");
    // The Preview's tab: its body, the Palette's gone.
    dock_tab(&h, "Preview").click();
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview));
    assert!(palette_types(&h).is_none(), "the Palette's nodes gone");
    in_preview(&h, "nw_bandit001");
    assert!(model_view_of(&h, creature), "the creature's model view");
    // Another chosen with the Palette in front: shown when the Preview is.
    dock_tab(&h, "Palette").click();
    settle(&mut h);
    palette_types(&h).expect("the palette's types");
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.state_mut().palette.selected = Some(chest);
    settle(&mut h);
    assert!(!model_view_of(&h, chest), "not loaded behind the Palette");
    assert!(h.state().preview_panel_rect().is_none());
    dock_tab(&h, "Preview").click();
    settle(&mut h);
    in_preview(&h, "plc_chest1");
    assert!(h.state().model_views[&mg_ui::model_view::Source::Resource(chest)].holds_targets());
}

/// Tools › Preview (the Preview Window command's id kept, and a key
/// given it), a plain row as Tools › Palettes and Resource Browser are:
/// from the Palette's tab it brings the Preview's to the front; from the
/// dock folded it unfolds the dock on it; the strip's buttons unfold it
/// on their own panels. Tools › Palettes brings the Palette back.
#[test]
fn tools_preview_and_the_strip_show_the_preview() {
    let Some(mut h) = preview_harness("ui-preview-tools", 1400.0, 800.0) else { return };
    let id = mg_ui::commands::Id::all().into_iter().find(|i| i.name() == "Preview");
    assert_eq!(id.map(|i| i.id()).as_deref(), Some("preview-window"), "its id kept");
    // (A plain row: not one that is ticked.)
    h.get_by_label("Top Bar").get_by_label("Tools").click();
    settle(&mut h);
    let row = menu_row(&h, "Preview").expect("Tools › Preview").1;
    let ticked = h.query_all_by(|n| n.role() == egui::accesskit::Role::CheckBox);
    assert!(ticked.filter(|n| n.rect() == row).count() == 0, "a plain row");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Palette));
    menu_command(&mut h, "Tools", "Preview");
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview), "from the Palette's tab");
    menu_command(&mut h, "Tools", "Palettes");
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Palette));
    // Folded: Tools › Preview unfolds it on the Preview.
    fold_palette(h.state_mut());
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), None);
    menu_command(&mut h, "Tools", "Preview");
    assert!(!h.state().settings.hide_palettes, "unfolded");
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview));
    // The strip's buttons, each on its panel.
    for (name, panel) in [("Palette", Panel::Palette), ("Preview", Panel::Preview)] {
        fold_palette(h.state_mut());
        settle(&mut h);
        h.get_by_label(mg_ui::RIGHT_DOCK).get_by_label(name).click();
        settle(&mut h);
        assert_eq!(h.state().right_dock_shows(), Some(panel), "the strip's {name}");
    }
}

/// The Right Dock's tabs show their words where the dock has the room and
/// their icons alone where it has not, by its width alone, whichever tab
/// is in front: words at 320 and 280 (beside the fold button at the bar's
/// end), in a wide window, a narrow one and the least one, each in the
/// same place on either panel.
#[test]
fn the_right_dock_s_tabs_keep_their_form_by_its_width() {
    let Some(mut h) = preview_harness("ui-preview-tab-form", 1400.0, 800.0) else { return };
    let words = |h: &Harness<'_, Moonglow>| {
        let (a, b) = (dock_tab(h, "Palette").rect(), dock_tab(h, "Preview").rect());
        // (A tab with its words is wider than an icon's 32 points.)
        (a.width() > 40.0 && b.width() > 40.0, a, b)
    };
    // (The dock's least, 280, in the narrow window's and, unfolded by
    // hand, in the least window's: under 962 it never stands beside the
    // Left Dock, so it is never narrower, M3 T14.)
    for (width, window) in [(320.0, 1600.0), (280.0, 1600.0), (280.0, 1280.0), (280.0, 420.0)] {
        h.set_size(egui::vec2(window, 800.0));
        right_dock_width(&mut h, width);
        settle(&mut h);
        unfold_right_dock(&mut h);
        assert!((right_dock_rect(&h).width() - width).abs() < 2.0, "{width}");
        let mut each = Vec::new();
        for panel in [Panel::Palette, Panel::Preview] {
            show(&mut h, panel);
            each.push(words(&h));
        }
        assert_eq!(each[0], each[1], "{width}: the tabs as on the other panel");
        assert!(each[0].0, "{width}: their words");
        let dock = right_dock_rect(&h);
        assert!(dock.contains_rect(each[0].2), "{width}: inside the dock");
    }
}

/// Left and Right on a focused tab of the Right Dock choose the other tab
/// (its panel shown with it), round the ends.
#[test]
fn the_arrow_keys_on_a_right_dock_tab_choose_the_other() {
    let Some(mut h) = preview_harness("ui-preview-tab-arrows", 1400.0, 800.0) else { return };
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Palette));
    dock_tab(&h, "Palette").focus();
    // (One step: the focus given, before the keys.)
    h.run_steps(2);
    for (key, panel, name) in [
        (egui::Key::ArrowRight, Panel::Preview, "Preview"),
        (egui::Key::ArrowRight, Panel::Palette, "Palette"),
        (egui::Key::ArrowLeft, Panel::Preview, "Preview"),
    ] {
        h.key_press(key);
        settle(&mut h);
        assert_eq!(h.state().right_dock_shows(), Some(panel), "{key:?}");
        assert!(dock_tab(&h, name).accesskit_node().is_focused(), "{key:?}: {name} focused");
    }
}

/// The area remembers the Right Dock's tab (the owner's ruling of
/// 2026-11-15, overturning M3's "for the run, the Palette again as a
/// module opens"): with nothing chosen (a new installation's settings),
/// the Palette in front; the Preview's tab chosen, the Preview in front
/// again after reopening the module and after a restart (the settings
/// saved and read back), kept as the area kind's layout.
#[test]
fn the_area_remembers_the_right_dock_s_tab() {
    let Some(mut h) = preview_harness("ui-preview-front", 1400.0, 800.0) else { return };
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Palette), "nothing chosen");
    dock_tab(&h, "Preview").click();
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview));
    // The module opened again: the Preview.
    let dir = mg_testkit::scratch_dir("ui-preview-front-again");
    let path = sample_module(&dir);
    h.state_mut().open_module(&path);
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview), "after reopening");
    // A restart: the settings saved and read back.
    let saved = h.state().settings.saved_text();
    assert!(saved.contains("dock_layouts:{\"area\":(fronts:[\"Preview\"])}"), "{saved}");
    let root = mg_testkit::nwn_root().unwrap();
    let mut app = game_app(&root, None);
    app.settings = mg_ui::Settings::read_fields(&saved).unwrap().0;
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview), "after a restart");
}

/// Escape with the pointer over the Preview does what it does over the
/// Palette, as the manual says for both: the Preview has no window to
/// close, and a blueprint in hand to place in the area is let go of only
/// with the pointer over the area's view. Over either panel it stays in
/// hand (the area's view still says it is placing it); over the view,
/// Escape lets go of it.
#[test]
fn escape_over_the_preview_does_as_over_the_palette() {
    let Some((mut h, area)) = area_harness("preview-escape") else { return };
    let waypoint = ResKey::parse("nw_waypoint001", ResType::UTW);
    let placing = |h: &Harness<'_, Moonglow>| {
        h.query_all_by(|n| n.label().or(n.value()).is_some_and(|l| l.starts_with("Placing ")))
            .count()
            > 0
    };
    for panel in [Panel::Palette, Panel::Preview] {
        show(&mut h, panel);
        h.state_mut().palette.selected = waypoint;
        settle(&mut h);
        assert!(placing(&h), "{panel:?}: placing it");
        let dock = right_dock_rect(&h);
        h.hover_at(egui::pos2(dock.center().x, dock.bottom() - 60.0));
        settle(&mut h);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert_eq!(h.state().palette.selected, waypoint, "{panel:?}: still in hand");
        assert!(placing(&h), "{panel:?}: the area still placing it");
        assert_eq!(h.state().right_dock_shows(), Some(panel), "{panel:?}: still in front");
        // Over the area's view: let go of.
        let centre = h.state().area_views[&area].rect.center();
        h.hover_at(centre);
        settle(&mut h);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert_eq!(h.state().palette.selected, None, "{panel:?}: let go of over the view");
        assert!(!placing(&h), "{panel:?}: no longer placing");
    }
}

/// F6 reaches the Right Dock with either tab in front, between the
/// editor area and the log, and Shift+F6 the other way: with the
/// Palette's, the arrow keys are its rows'; with the Preview's, they turn
/// its model as a drag does (Left and Right about it, Up and Down over
/// it), a creature chosen.
#[test]
fn f6_reaches_the_right_dock_with_either_tab_in_front() {
    let Some((mut h, area)) = area_harness("preview-f6") else { return };
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    h.state_mut().settings.hide_log = false;
    let creature = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    h.state_mut().palette.selected = Some(creature);
    settle(&mut h);
    let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
    let step = |h: &mut Harness<'_, Moonglow>, back: bool| {
        let m = if back { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
        h.key_press_modifiers(m, egui::Key::F6);
        settle(h);
        keys(h)
    };
    for (panel, name) in [(Panel::Palette, "Palette"), (Panel::Preview, "Preview")] {
        show(&mut h, panel);
        // (The pointer in the area's view: the keys its; then away.)
        let centre = h.state().area_views[&area].rect.center();
        h.hover_at(centre);
        settle(&mut h);
        assert_eq!(keys(&h), "Area");
        h.hover_at(egui::pos2(centre.x, 20.0));
        settle(&mut h);
        // (The Outliner under the Left Dock's tabs, with the area in
        // front: M3's T13.)
        let forward: Vec<String> = (0..5).map(|_| step(&mut h, false)).collect();
        assert_eq!(forward, [name, "Log", "Tree", "Outliner", "Area"], "{name}: F6");
        let back: Vec<String> = (0..5).map(|_| step(&mut h, true)).collect();
        assert_eq!(back, ["Outliner", "Tree", "Log", name, "Area"], "{name}: Shift+F6");
    }
    // The Preview's keys turn its model.
    assert_eq!(step(&mut h, false), "Preview");
    let source = mg_ui::model_view::Source::Resource(creature);
    let orbit = |h: &Harness<'_, Moonglow>| {
        let o = h.state().model_views[&source].orbit;
        (o.yaw, o.pitch)
    };
    let (yaw, pitch) = orbit(&h);
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert!((orbit(&h).0 - yaw).abs() > 0.2, "turned about it: {yaw} -> {:?}", orbit(&h));
    h.key_press(egui::Key::ArrowUp);
    settle(&mut h);
    assert!((orbit(&h).1 - pitch).abs() > 0.1, "turned over it: {pitch} -> {:?}", orbit(&h));
    assert_eq!(h.state().palette.selected, Some(creature), "the palette's choice left alone");
}

/// The Preview's view of the blueprint chosen is kept while it stays
/// chosen: its animation and camera as left after its tab has been behind
/// the Palette's for longer than an unseen view is kept (its targets let
/// go of meanwhile); another blueprint chosen, it goes.
#[test]
fn the_preview_keeps_its_view_while_its_blueprint_is_chosen() {
    let Some((mut h, _area)) = area_harness("preview-kept") else { return };
    let creature = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    let source = mg_ui::model_view::Source::Resource(creature);
    h.state_mut().palette.selected = Some(creature);
    show(&mut h, Panel::Preview);
    assert!(model_view_of(&h, creature));
    {
        let view = h.state_mut().model_views.get_mut(&source).unwrap();
        view.animation = Some("pause1".into());
        view.orbit.yaw += 1.0;
        view.model_lights = !view.model_lights;
    }
    settle(&mut h);
    let left = {
        let v = &h.state().model_views[&source];
        (v.animation.clone(), v.orbit.yaw, v.model_lights)
    };
    show(&mut h, Panel::Palette);
    // (Longer than a view unseen is kept, by the window's clock.)
    let from = h.ctx.input(|i| i.time);
    let started = std::time::Instant::now();
    while h.ctx.input(|i| i.time) < from + 3.0 {
        h.step();
        assert!(started.elapsed().as_secs() < 60, "the window's clock stands still");
    }
    settle(&mut h);
    let view = h.state().model_views.get(&source).expect("its view kept");
    assert!(!view.holds_targets(), "its targets let go of");
    show(&mut h, Panel::Preview);
    let v = &h.state().model_views[&source];
    assert_eq!((v.animation.clone(), v.orbit.yaw, v.model_lights), left, "as left");
    // Another chosen (behind the Palette's tab): the creature's goes.
    show(&mut h, Panel::Palette);
    h.state_mut().palette.selected = ResKey::parse("plc_chest1", ResType::UTP);
    let from = h.ctx.input(|i| i.time);
    while h.ctx.input(|i| i.time) < from + 3.0 {
        h.step();
        assert!(started.elapsed().as_secs() < 120, "the window's clock stands still");
    }
    settle(&mut h);
    assert!(!model_view_of(&h, creature), "dropped, no longer chosen");
}

// ------------------------------------------------------------ a blueprint's own (M4's U11)

/// `key`'s blueprint viewed and opened as a tab, docked in the middle in
/// front of the area (`docked`) or in a floating window.
fn open_blueprint(h: &mut Harness<'_, Moonglow>, key: ResKey, docked: bool) {
    let kind = match key.restype {
        ResType::UTC => "creature",
        ResType::UTI => "item",
        ResType::UTP => "placeable",
        _ => "door",
    };
    let place = if docked { "middle" } else { "window" };
    let app = h.state_mut();
    app.settings.window_places = vec![(kind.to_string(), place.to_string())];
    let data = app.game.as_ref().unwrap().resman.get(&key).unwrap();
    app.ws.as_mut().unwrap().view(key, Gff::read(&data).unwrap());
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    settle(h);
}

/// Whether a Palette tab is drawn in the Right Dock.
fn palette_tab_shown(h: &Harness<'_, Moonglow>) -> bool {
    let dock = right_dock_rect(h);
    h.query_all_by_label("Palette").any(|n| dock.contains(n.rect().center()))
}

/// Whether `key`'s model view holds its picture's targets (it is drawn).
fn drawn(h: &Harness<'_, Moonglow>, key: ResKey) -> bool {
    let source = mg_ui::model_view::Source::Resource(key);
    h.state().model_views.get(&source).is_some_and(|v| v.holds_targets())
}

/// The model view's header row (`Model header`), drawn once: its record.
fn model_header(h: &Harness<'_, Moonglow>) -> mg_uikit::header::HeaderRecord {
    let drawn = h.query_all_by_label(mg_ui::model_view::MODEL_HEADER).count();
    assert_eq!(drawn, 1, "one model header drawn");
    mg_uikit::header::records(&h.ctx, mg_ui::model_view::MODEL_HEADER).remove(0)
}

/// The Preview follows the blueprint's tab (M4's U11, the Creature
/// board): a creature's tab in front brings its own Preview alone (no
/// Palette tab), its header titled with the model's info line, its model
/// drawn and not the palette's choice; the area's tab in front brings the
/// palettes back, the Preview the palette's choice; the creature in a
/// floating window has its Preview in the window's right pane, the main
/// Right Dock keeping the area's.
#[test]
fn the_preview_follows_the_blueprint_s_tab() {
    let Some((mut h, area)) = area_harness("preview-follows") else { return };
    let bandit = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.state_mut().palette.selected = Some(chest);
    open_blueprint(&mut h, bandit, true);
    assert_eq!(h.state().active_editor(), Some(&Tab::Blueprint(bandit)));
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview), "its own Preview");
    assert!(!palette_tab_shown(&h), "no Palette beside a blueprint");
    let titled = h
        .query_all_by_label_contains("Preview, ")
        .next()
        .map(|n| n.accesskit_node().label().unwrap_or_default());
    let titled = titled.expect("the Preview's header named with its detail");
    assert!(titled.contains("nodes") && titled.contains("meshes"), "the info line: {titled}");
    assert!(drawn(&h, bandit), "the creature drawn");
    assert!(!drawn(&h, chest), "not the palette's choice");
    let header = model_header(&h);
    assert!(right_dock_rect(&h).contains_rect(header.row), "the header in the dock");
    // The area's tab: the palettes, the Preview the palette's choice.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    settle(&mut h);
    assert!(palette_tab_shown(&h), "the palettes beside the area");
    show(&mut h, Panel::Preview);
    in_preview(&h, "plc_chest1");
    assert!(drawn(&h, chest), "the palette's choice drawn");
    // Floating: the creature's own in its window's right pane.
    let at = h.state().dock.find_tab(&Tab::Blueprint(bandit)).unwrap();
    h.state_mut().dock.remove_tab(at);
    // (A window wide enough for its form and its pane: one opens at most
    // 85% of the main pane wide.)
    h.set_size(egui::vec2(2200.0, 1000.0));
    settle(&mut h);
    open_blueprint(&mut h, bandit, false);
    let at = h.state().dock.find_tab(&Tab::Blueprint(bandit)).unwrap();
    assert!(!at.surface.is_main(), "floating");
    assert_eq!(h.state().active_editor(), Some(&Tab::Area(area)), "the area's docks kept");
    in_preview(&h, "plc_chest1");
    let headers = h.query_all_by_label(mg_ui::model_view::MODEL_HEADER).count();
    assert_eq!(headers, 2, "the dock's and the window's");
    assert!(drawn(&h, bandit) && drawn(&h, chest), "the creature in its window, the chest");
}

/// The Preview's one header row (the Creature board's `.panel-bar`): at
/// the narrow dock's 280 every control of the model viewer's old toolbar
/// is on the row or in its More, the row one row (30 points) with the 3D
/// view right under it; the same at 320 in a wider window.
#[test]
fn the_preview_s_header_is_one_row_at_280() {
    let Some((mut h, _area)) = area_harness("preview-one-row") else { return };
    let bandit = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    open_blueprint(&mut h, bandit, true);
    for (width, dock) in [(1160.0, 280.0), (1600.0, 320.0)] {
        h.set_size(egui::vec2(width, 800.0));
        settle(&mut h);
        assert_eq!(right_dock_rect(&h).width().round(), dock, "the dock at {dock}");
        let header = model_header(&h);
        assert_eq!(header.row.height(), mg_uikit::metrics::EDITOR_HEADER, "one row at {dock}");
        let playing = {
            let source = mg_ui::model_view::Source::Resource(bandit);
            h.state().model_views[&source].playing
        };
        let play = if playing { "Pause" } else { "Play" };
        let all = ["Animation", play, "Frame", "Lights", "Shadow", "Shaders", "Export Files…"];
        let mut names = all.to_vec();
        names.push("Pop Out");
        // (Each item's place: what the view looks like goes into More
        // first, Export Files… then the switches; the animation and how it
        // plays stay on the row, the review's A4.)
        use mg_uikit::header::Place;
        let places = mg_uikit::header::places(&h.ctx, mg_ui::model_view::MODEL_HEADER);
        let place = |n: &str| places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
        let export = if dock < 300.0 { Place::More } else { Place::Icon };
        let expected = [
            ("Animation", Place::Icon),
            (play, Place::Icon),
            ("Frame", Place::Icon),
            ("Look", Place::Row),
            ("Export Files…", export),
            ("Pop Out", Place::More),
        ];
        for (name, at) in expected {
            assert_eq!(place(name), Some(at), "{name} at {dock}: {places:?}");
        }
        header_holds(&mut h, mg_ui::model_view::MODEL_HEADER, &names);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        // (Flush with the dock's top and sides, the Creature board's: no
        // margin round the row, under the panel's header.)
        let dock = right_dock_rect(&h);
        let body = h.state().preview_panel_rect().unwrap();
        assert!((header.row.top() - body.top()).abs() < 1.0, "at the top: {header:?} {body:?}");
        let (left, right) = (header.row.left() - dock.left(), dock.right() - header.row.right());
        assert!(left.abs() <= 1.0 && right.abs() <= 1.0, "across the dock: {header:?} {dock:?}");
    }
}

/// Beside a blueprint's own Preview the Status Bar gives the 3D view's
/// ways (the Creature board's five), not Tab's; the area's tab its own.
#[test]
fn the_status_bar_gives_the_preview_s_hints() {
    let Some((mut h, area)) = area_harness("preview-hints") else { return };
    let bandit = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    open_blueprint(&mut h, bandit, true);
    let five = [
        "Drag: Turn view",
        "Right Drag: Move view",
        "Middle Drag: Move view",
        "Wheel: Zoom",
        "Shift, Wheel: Zoom finer",
    ];
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said().to_vec();
    let now = said(&h);
    for w in five {
        assert!(now.iter().any(|s| s == w), "{w} in {now:?}");
        assert!(h.get_by_label("Status Bar").query_by_label(w).is_some(), "{w} drawn");
    }
    assert!(!now.iter().any(|s| s == "Tab: Next field"), "not Tab's: {now:?}");
    // (In the board's order, one after another; and so drawn, left to
    // right.)
    let at = now.iter().position(|s| s == five[0]).unwrap();
    assert_eq!(now[at..at + 5], five, "in the board's order: {now:?}");
    let xs: Vec<f32> =
        (five.iter()).map(|w| h.get_by_label("Status Bar").get_by_label(w).rect().left()).collect();
    assert!(xs.windows(2).all(|p| p[0] < p[1]), "drawn in order: {xs:?}");
    // Folded away: no view, no hints of it.
    fold_palette(h.state_mut());
    settle(&mut h);
    assert!(!said(&h).iter().any(|s| s == five[0]), "folded: {:?}", said(&h));
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(Panel::Preview));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    settle(&mut h);
    assert!(!said(&h).iter().any(|s| s == five[0]), "the area's: {:?}", said(&h));
}

/// The Basic page's Preview button and Tools › Preview beside a
/// blueprint bring its own Preview (the dock unfolded on it), never the
/// palettes with it; Pop Out, in the header's More, shows the model in a
/// tab of its own, the Preview then saying so with Bring Back, which
/// closes that tab and draws it in the panel again.
#[test]
fn the_preview_button_and_pop_out() {
    let Some((mut h, _area)) = area_harness("preview-button") else { return };
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.state_mut().blueprint_pages.insert((chest, mg_edit::GffPath::root()), "Basic");
    open_blueprint(&mut h, chest, true);
    for how in ["button", "Tools"] {
        fold_palette(h.state_mut());
        settle(&mut h);
        assert_eq!(h.state().right_dock_shows(), None, "{how}: folded");
        match how {
            "button" => {
                let button = h.get_all_by_label("Preview").find(|n| {
                    n.accesskit_node().role() == egui::accesskit::Role::Button
                        && in_group(n, mg_ui::blueprint::WINDOW_GROUP)
                });
                button.expect("Basic's Preview button").click();
            }
            _ => h.state_mut().actions.push(mg_ui::Action::ShowPanel(Panel::Preview)),
        }
        settle(&mut h);
        assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview), "{how}: shown");
        assert!(!palette_tab_shown(&h), "{how}: no palettes with it");
        assert!(drawn(&h, chest), "{how}: drawn");
    }
    // Pop Out, in More.
    let row = h.get_by_label(mg_ui::model_view::MODEL_HEADER);
    row.get_by_label(mg_uikit::header::MORE).click();
    settle(&mut h);
    let more = mg_uikit::header::more_name(mg_ui::model_view::MODEL_HEADER);
    h.get_by_label(&more).get_by_label("Pop Out").click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Model(chest)).is_some(), "the model's own tab");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(chest)));
    settle(&mut h);
    let note = h.get_by_label(mg_ui::area_tools::POPPED_OUT).rect();
    assert!(right_dock_rect(&h).contains_rect(note), "said in the Preview");
    let dock = right_dock_rect(&h);
    let rows = mg_uikit::header::records(&h.ctx, mg_ui::model_view::MODEL_HEADER);
    assert!(rows.iter().all(|r| !dock.contains_rect(r.row)), "none in the panel: {rows:?}");
    h.get_by_label("Bring Back").click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Model(chest)).is_none(), "its tab closed");
    assert!(h.query_by_label(mg_ui::area_tools::POPPED_OUT).is_none());
    assert!(drawn(&h, chest), "drawn in the panel again");
}

/// A Preview taller than it is wide (the dock beside a form) frames the
/// model whole across it: a door, framed as the view opens, leaves the
/// background at both sides of the picture at its middle (the camera's
/// 20° either way held to the narrower width); the picture is framed
/// again as it changes shape.
#[test]
fn a_tall_preview_frames_the_model_across() {
    let Some((mut h, _area)) = area_harness("preview-tall") else { return };
    let door = ResKey::parse("nw_door_ttr_01", ResType::UTD).unwrap();
    for size in [egui::vec2(1600.0, 1000.0), egui::vec2(1300.0, 900.0)] {
        h.set_size(size);
        if h.state().dock.find_tab(&Tab::Blueprint(door)).is_none() {
            open_blueprint(&mut h, door, true);
        }
        settle(&mut h);
        let body = h.state().preview_panel_rect().expect("the Preview");
        let header = model_header(&h).row;
        let view = egui::Rect::from_min_max(egui::pos2(body.left(), header.bottom()), body.max);
        assert!(view.height() > 1.5 * view.width(), "taller than wide: {view:?}");
        let image = h.render().expect("render");
        let ppp = h.ctx.pixels_per_point();
        let at = |p: egui::Pos2| {
            let px = image.get_pixel((p.x * ppp) as u32, (p.y * ppp) as u32);
            [px[0], px[1], px[2]]
        };
        let ground = at(view.left_top() + egui::vec2(4.0, 4.0));
        let near = |a: [u8; 3]| a.iter().zip(ground).all(|(x, y)| x.abs_diff(y) <= 8);
        let mid = view.center().y;
        let (left, right) =
            (at(egui::pos2(view.left() + 3.0, mid)), at(egui::pos2(view.right() - 3.0, mid)));
        assert!(
            near(left) && near(right),
            "at {size:?}: the sides {left:?} {right:?}, the ground {ground:?}"
        );
    }
}

/// The view framed: a tall creature fills the Preview's height (Creature
/// board: Aribeth from head to feet down the panel; here a commoner, who
/// holds no sword out at her side), what it reaches fitted
/// across and up apart (the review's A8), as far as a fit for every yaw
/// allows (M5's W8); moved or zoomed by hand, the
/// camera stays where it was put as the panel changes shape (B6).
#[test]
fn a_tall_creature_fills_the_preview_and_a_zoom_is_kept() {
    let Some((mut h, _area)) = area_harness("preview-fills") else { return };
    h.set_size(egui::vec2(1600.0, 1000.0));
    let bandit = ResKey::parse("nw_comfemale", ResType::UTC).unwrap();
    open_blueprint(&mut h, bandit, true);
    // (The board's dock, 380 wide: at 320 her width, arms and shadow,
    // holds her to about three quarters of so tall a view.)
    right_dock_width(&mut h, 380.0);
    settle(&mut h);
    let body = h.state().preview_panel_rect().expect("the Preview");
    let header = model_header(&h).row;
    let view = egui::Rect::from_min_max(egui::pos2(body.left(), header.bottom()), body.max);
    let image = h.render().expect("render");
    image.save(mg_testkit::scratch_dir("ui-preview-fills").join("fills.png")).unwrap();
    let ppp = h.ctx.pixels_per_point();
    let px = |x: f32, y: f32| {
        let p = image.get_pixel((x * ppp) as u32, (y * ppp) as u32);
        [p[0], p[1], p[2]]
    };
    // (Under the header's line.)
    let ground = px(view.left() + 4.0, view.top() + 8.0);
    let differs = |a: [u8; 3]| a.iter().zip(ground).any(|(x, y)| x.abs_diff(y) > 12);
    // (The rows with something drawn in them, across the view.)
    let rows: Vec<f32> = (view.top() as i32 + 6..view.bottom() as i32 - 4)
        .map(|y| y as f32)
        .filter(|y| {
            (0..60).any(|i| differs(px(view.left() + 2.0 + i as f32 * view.width() / 61.0, *y)))
        })
        .collect();
    let (top, bottom) = (rows.first().copied().unwrap(), rows.last().copied().unwrap());
    let filled = (bottom - top) / view.height();
    // (Fitted for every yaw at this pitch, M5's W8: as tight as turning
    // about her allows, at some yaw she comes to the fit's room's edge. A
    // fit for every tilt too made her fill 0.64: the owner's ruling "fit:
    // b" took it back.)
    {
        let source = mg_ui::model_view::Source::Resource(bandit);
        let framed = h.state().model_views[&source].orbit;
        swept(&mut h, &source, "the creature");
        h.state_mut().model_views.get_mut(&source).unwrap().orbit = framed;
        h.run_steps(2);
    }
    assert!(
        filled >= 0.8,
        "the creature fills {filled:.2} of the view's height ({top}..{bottom} in {view:?})"
    );
    // Zoomed by hand: kept as the panel changes shape.
    let source = mg_ui::model_view::Source::Resource(bandit);
    let framed = h.state().model_views[&source].orbit;
    h.hover_at(view.center());
    h.run();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 120.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    settle(&mut h);
    let zoomed = h.state().model_views[&source].orbit;
    assert!(zoomed.distance < framed.distance, "zoomed: {framed:?} {zoomed:?}");
    h.set_size(egui::vec2(1500.0, 800.0));
    settle(&mut h);
    let after = h.state().model_views[&source].orbit;
    assert_eq!(after.distance, zoomed.distance, "the zoom kept");
    assert_eq!(after.target, zoomed.target, "where it looks kept");
}

/// The model headers drawn outside the main window's Right Dock: a
/// floating form's Preview pane's.
fn window_headers(h: &Harness<'_, Moonglow>) -> usize {
    let dock = h.state().right_dock_rect();
    let drawn = h.query_all_by_label(mg_ui::model_view::MODEL_HEADER).count();
    let rows = mg_uikit::header::records(&h.ctx, mg_ui::model_view::MODEL_HEADER);
    let docked = rows.iter().filter(|r| dock.is_some_and(|d| d.contains_rect(r.row))).count();
    drawn.saturating_sub(docked)
}

/// Basic's Preview button in the form's window (ShowPreview's floating
/// branches, the review's A3): where the window's pane shows the Preview,
/// beside a form keeping its 972 (R1: a new window opens 980 and the pane
/// wide, A2), the button opens nothing more; in a window too narrow to keep
/// the form's 972 beside the pane, the pane is not drawn and the button
/// opens the model's own tab. A new window in a screen that cannot give it
/// that width (1600, 1920 beside the area's docks), and one of the kind
/// left at U9's 980, have no pane.
#[test]
fn the_preview_button_in_a_form_s_window() {
    let Some((mut h, _area)) = area_harness("preview-window-button") else { return };
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.state_mut().blueprint_pages.insert((chest, mg_edit::GffPath::root()), "Basic");
    let click = |h: &mut Harness<'_, Moonglow>| {
        let button = h.get_all_by_label("Preview").find(|n| {
            n.accesskit_node().role() == egui::accesskit::Role::Button
                && in_group(n, mg_ui::blueprint::WINDOW_GROUP)
        });
        button.expect("Basic's Preview button").click();
        settle(h);
    };
    let close = |h: &mut Harness<'_, Moonglow>, tab: &Tab| {
        if let Some(at) = h.state().dock.find_tab(tab) {
            h.state_mut().dock.remove_tab(at);
        }
        settle(h);
    };
    // (Not at a size a window of the kind was left at.)
    let fresh = |h: &mut Harness<'_, Moonglow>| h.state_mut().settings.window_sizes.clear();
    for width in [1600.0, 1920.0] {
        h.set_size(egui::vec2(width, 1000.0));
        fresh(&mut h);
        open_blueprint(&mut h, chest, false);
        assert_eq!(window_headers(&h), 0, "at {width}: no pane beside a form under 972");
        close(&mut h, &Tab::Blueprint(chest));
    }
    h.set_size(egui::vec2(2600.0, 1200.0));
    // A window of the kind left at U9's 980: opened so, the form keeping
    // its room, no pane (R1).
    h.state_mut().settings.window_sizes = vec![("placeable".to_string(), [980, 672])];
    open_blueprint(&mut h, chest, false);
    assert_eq!(window_headers(&h), 0, "left at 980: no pane");
    close(&mut h, &Tab::Blueprint(chest));
    fresh(&mut h);
    open_blueprint(&mut h, chest, false);
    let at = h.state().dock.find_tab(&Tab::Blueprint(chest)).unwrap();
    assert!(!at.surface.is_main(), "floating");
    assert_eq!(window_headers(&h), 1, "its pane");
    let left = h.get_all_by_label("Basic").map(|n| n.rect().left()).fold(f32::MAX, f32::min);
    // (To the pane's left edge: its header row's.)
    let dock = h.state().right_dock_rect();
    let rows = mg_uikit::header::records(&h.ctx, mg_ui::model_view::MODEL_HEADER);
    let pane = rows.iter().find(|r| !dock.is_some_and(|d| d.contains_rect(r.row))).unwrap();
    let right = pane.row.left();
    assert!(right - left >= 970.0, "the form keeps its 972: {left} to {right}");
    click(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Model(chest)).is_none(), "the pane shows it already");
    assert_eq!(window_headers(&h), 1);
    // Narrowed: the pane goes, the button opens the model's tab.
    if let Some(w) = h.state_mut().dock.get_window_state_mut(at.surface) {
        w.set_size(egui::vec2(1100.0, 700.0));
    }
    h.run_steps(4);
    settle(&mut h);
    assert_eq!(window_headers(&h), 0, "no pane beside a form under 972");
    click(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Model(chest)).is_some(), "the model's own tab");
}

/// In the wide layout a form's lone Preview is 549 wide (WideCreature's
/// dock, for the board's five columns of pages), and the width it is
/// dragged to is kept for its kind, apart from the area's two columns'
/// (the review's A5): a creature's dragged to 640, the area's tab still
/// 741, the creature's again 640.
#[test]
fn a_lone_preview_keeps_its_own_wide_width() {
    let Some((mut h, area)) = area_harness("preview-wide-width") else { return };
    h.set_size(egui::vec2(3440.0, 1200.0));
    settle(&mut h);
    let bandit = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    open_blueprint(&mut h, bandit, true);
    let width = |h: &Harness<'_, Moonglow>| right_dock_rect(h).width();
    assert!((width(&h) - 550.0).abs() <= 1.5, "549 and its line: {}", width(&h));
    let dock = right_dock_rect(&h);
    let from = egui::pos2(dock.left() + 1.0, dock.top() + 300.0);
    drag(&mut h, from, from - egui::vec2(640.0 - dock.width(), 0.0));
    settle(&mut h);
    let dragged = width(&h);
    assert!((dragged - 640.0).abs() < 2.5, "dragged: {dragged}");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    settle(&mut h);
    // Ultrawide auto-hides Properties when nothing is selected (reclaiming to single column).
    assert_eq!(width(&h), mg_uikit::shell::RIGHT_DOCK_WIDE_SINGLE, "no selection: 1 column");
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    settle(&mut h);
    assert_eq!(width(&h), mg_uikit::shell::RIGHT_DOCK_WIDE, "the area's own two columns");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(bandit)));
    settle(&mut h);
    assert!((width(&h) - dragged).abs() < 1.0, "the creature's again: {}", width(&h));
}

// ------------------------------------------------------------ wide models in a tall Preview (M5's W8)

/// A plank 6 m long, 0.3 m deep and 0.4 m high, of our own (for
/// `plc_a01.mdl` in a scratch user folder's override: row 0's model).
const PLANK: &str = "newmodel plc_a01\nsetsupermodel plc_a01 NULL\nclassification placeable\n\
    beginmodelgeom plc_a01\nnode dummy plc_a01\n  parent NULL\nendnode\n\
    node trimesh plank\n  parent plc_a01\n  ambient 0.8 0.6 0.3\n  diffuse 0.8 0.6 0.3\n  \
    bitmap NULL\n  verts 8\n    -3 -0.15 0\n    3 -0.15 0\n    3 0.15 0\n    -3 0.15 0\n    \
    -3 -0.15 0.4\n    3 -0.15 0.4\n    3 0.15 0.4\n    -3 0.15 0.4\n  faces 12\n    \
    0 2 1 1 0 0 0 0\n    0 3 2 1 0 0 0 0\n    4 5 6 1 0 0 0 0\n    4 6 7 1 0 0 0 0\n    \
    0 1 5 1 0 0 0 0\n    0 5 4 1 0 0 0 0\n    1 2 6 1 0 0 0 0\n    1 6 5 1 0 0 0 0\n    \
    2 3 7 1 0 0 0 0\n    2 7 6 1 0 0 0 0\n    3 0 4 1 0 0 0 0\n    3 4 7 1 0 0 0 0\n\
    endnode\nendmodelgeom plc_a01\ndonemodel plc_a01\n";

/// [`area_harness`] with placeables' blueprints `(resref, appearance)`
/// in its module, and the authored [`PLANK`] as row 0's model.
fn wide_harness(name: &str, rows: &[(&str, u32)]) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let user = mg_testkit::scratch_dir(&format!("{name}-user"));
    std::fs::create_dir_all(user.join("override")).unwrap();
    std::fs::write(user.join("override/plc_a01.mdl"), PLANK).unwrap();
    let rows: Vec<(String, u32)> = rows.iter().map(|(n, r)| (n.to_string(), *r)).collect();
    let prepare = move |m: &mut mg_module::Module, _: &mg_rules::GameData, _: ResRef| {
        for (resref, row) in &rows {
            let key = ResKey::parse(resref, ResType::UTP).unwrap();
            let mut bp = mg_module::blueprints::placeable(key.resref, resref, 0);
            bp.root.set("Appearance", mg_gff::Value::Dword(*row));
            m.set_gff(key, &bp).unwrap();
        }
    };
    area_harness_with(name, "ttr01", None, 4, Some(user), &prepare)
}

/// Where the Preview's picture of `key` was last drawn, within the
/// Preview's body.
fn picture_rect(h: &Harness<'_, Moonglow>, key: ResKey) -> egui::Rect {
    let body = h.state().preview_panel_rect().expect("the Preview");
    let view = &h.state().model_views[&mg_ui::model_view::Source::Resource(key)];
    let shown = view.shown_at().expect("its picture drawn");
    assert!(body.expand(0.5).contains_rect(shown), "in the Preview: {shown:?} {body:?}");
    shown
}

/// Where in the Preview's picture (0..1 from its top-left, by the
/// picture's own shape) each point `key`'s model reaches is shown.
fn projected(h: &Harness<'_, Moonglow>, key: ResKey) -> Vec<glam::Vec2> {
    let view = &h.state().model_views[&mg_ui::model_view::Source::Resource(key)];
    let picture = picture_rect(h, key);
    let camera = view.orbit.camera();
    let reached = view.reached();
    assert!(!reached.is_empty(), "{key}: something to fit");
    let aspect = picture.width() / picture.height();
    reached
        .iter()
        .map(|p| mg_area::pick::project(&camera, aspect, *p).expect("in front of the camera"))
        .collect()
}

/// `key`'s model whole in the Preview's picture (`what`, for the
/// failure): every point it reaches projected inside it, and framed no
/// farther than it needs to stay whole however it is turned
/// ([`nearest_over_turns`]: since M5's W8 the fit holds every yaw, so at
/// the first view's yaw a model need not fill the width or the height); in
/// the picture as drawn, nothing at
/// its sides' edges, and where it fits the width (the outermost within an
/// eighth of each side) something near each.
#[track_caller]
fn whole(h: &mut Harness<'_, Moonglow>, key: ResKey, what: &str) {
    let points = projected(h, key);
    let span = |of: fn(&glam::Vec2) -> f32| {
        points.iter().map(of).fold((f32::MAX, f32::MIN), |(l, r), v| (l.min(v), r.max(v)))
    };
    let ((low, high), (top, bottom)) = (span(|p| p.x), span(|p| p.y));
    let inside = |(l, r): (f32, f32)| l >= -0.001 && r <= 1.001;
    assert!(
        inside((low, high)) && inside((top, bottom)),
        "{what}: {key} reaches {low:.3}..{high:.3} across the picture, {top:.3}..{bottom:.3} up"
    );
    let fills = |(l, r): (f32, f32)| l < 0.125 && r > 0.875;
    let across = fills((low, high));
    let nearest = nearest_over_turns(h, &mg_ui::model_view::Source::Resource(key));
    assert!(
        nearest > 0.88,
        "{what}: {key} framed larger than it needs, nearest the edge {nearest:.3} at any turn \
         (fills {low:.3}..{high:.3} across)"
    );
    let view = picture_rect(h, key);
    let image = h.render().expect("render");
    let ppp = h.ctx.pixels_per_point();
    let at = |x: f32, y: f32| {
        let p = image.get_pixel((x * ppp) as u32, (y * ppp) as u32);
        [p[0], p[1], p[2]]
    };
    let ground = at(view.left() + 4.0, view.top() + 4.0);
    let differs = |a: [u8; 3]| a.iter().zip(ground).any(|(x, y)| x.abs_diff(y) > 12);
    let rows: Vec<f32> =
        (view.top() as i32 + 4..view.bottom() as i32 - 4).map(|y| y as f32).collect();
    for x in [view.left() + 2.0, view.right() - 3.0] {
        let drawn = rows.iter().find(|y| differs(at(x, **y)));
        assert!(drawn.is_none(), "{what}: drawn at the picture's edge, at ({x}, {drawn:?})");
    }
    let margin = view.width() / 8.0;
    for x in [view.left() + margin, view.right() - margin].into_iter().filter(|_| across) {
        assert!(rows.iter().any(|y| differs(at(x, *y))), "{what}: nothing drawn near x {x}");
    }
}

/// M5's W8, the owner's report (2026-11-21: a cart cut off at the sides
/// in the tall Preview, which Moonglow 4.2.1 fitted by its height
/// alone): a wide placeable's own Preview beside its tab docked (the
/// dock's whole height, a picture far taller than wide), at the dock's
/// usual 320 and least 280 and in a window of another shape, is whole
/// and fills the width: the game's long table (`pwc_tabl_001`, 5 m) and
/// a plank of our own 6 m long. Zoomed by hand it is cut (the check
/// bites) and Frame makes it whole again. Changed in its form from the
/// full wagon (2 m) to the long table, it is framed anew.
#[test]
fn a_wide_placeable_is_whole_in_a_tall_preview() {
    let rows = [("w8_table", 732), ("w8_plank", 0), ("w8_changed", 42)];
    let Some((mut h, _area)) = wide_harness("preview-wide-whole", &rows) else { return };
    h.state_mut().settings.window_places = vec![("placeable".into(), "middle".into())];
    let key = |n: &str| ResKey::parse(n, ResType::UTP).unwrap();
    for name in ["w8_table", "w8_plank"] {
        let key = key(name);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
        for (size, width) in
            [((1600.0, 1000.0), 320.0), ((1600.0, 1000.0), 280.0), ((1500.0, 800.0), 320.0)]
        {
            h.set_size(egui::vec2(size.0, size.1));
            right_dock_width(&mut h, width);
            settle(&mut h);
            let view = picture_rect(&h, key);
            assert!(view.height() > 2.0 * view.width(), "a tall picture: {view:?}");
            whole(&mut h, key, &format!("{name} at {size:?}, the dock {width}"));
        }
    }
    // Zoomed by hand: cut. Frame: whole again.
    let plank = key("w8_plank");
    let view = picture_rect(&h, key("w8_plank"));
    h.hover_at(view.center());
    h.run();
    for _ in 0..3 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 240.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        h.step();
    }
    settle(&mut h);
    let cut = projected(&h, plank).iter().any(|p| !(0.0..=1.0).contains(&p.x));
    assert!(cut, "zoomed in, the plank goes past the picture's sides");
    h.get_by_label(mg_ui::model_view::MODEL_HEADER).get_by_label("Frame").click();
    settle(&mut h);
    whole(&mut h, plank, "framed after a zoom");
    // Its appearance changed in its form: framed anew for the new model.
    let changed = key("w8_changed");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(changed)));
    settle(&mut h);
    let edit = mg_edit::Edit::SetField {
        key: changed,
        path: mg_gff::GffPath::default(),
        label: "Appearance".into(),
        value: Some(mg_gff::Value::Dword(732)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
    settle(&mut h);
    let view = &h.state().model_views[&mg_ui::model_view::Source::Resource(changed)];
    let model = view.preview().map(|p| p.base.model.to_string());
    assert_eq!(model.as_deref(), Some("pwc_tabl_001"), "the table drawn now");
    whole(&mut h, changed, "its appearance changed to the long table");
}

/// The cursor shown with the pointer at `at`.
fn cursor_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) -> egui::CursorIcon {
    h.hover_at(at);
    h.run_steps(2);
    h.output().platform_output.cursor_icon
}

/// The Right Dock's edge with the Preview in front (M5's W8, the owner's
/// question: "is the Preview pane resizable at the left edge by
/// dragging?"), beside an area (the palette's choice) and beside a
/// placeable's tab docked (its own Preview), at 1600 by 1000 and in the
/// wide layout: the resize cursor shown within 2.5 points either side of
/// it and not 8 points into the dock; dragged from just outside it 100
/// points wider, the dock follows, the width is kept (`right_dock_width`,
/// `right_dock_width_wide` in the wide layout) and the model is framed
/// anew for the picture's new shape, whole; dragged far either way, held
/// to two fifths of the window and to 280.
#[test]
fn the_right_dock_s_edge_is_dragged_beside_the_preview() {
    let Some((mut h, area)) = wide_harness("preview-dock-edge", &[("w8_table", 732)]) else {
        return;
    };
    let table = ResKey::parse("w8_table", ResType::UTP).unwrap();
    h.state_mut().settings.window_places = vec![("placeable".into(), "middle".into())];
    h.state_mut().palette.selected = Some(table);
    for (size, beside) in [
        ((1600.0, 1000.0), "area"),
        ((1600.0, 1000.0), "blueprint"),
        ((3440.0, 1440.0), "area"),
        ((3440.0, 1440.0), "blueprint"),
    ] {
        let what = format!("beside the {beside} at {size:?}");
        h.set_size(egui::vec2(size.0, size.1));
        let tab = if beside == "area" { Tab::Area(area) } else { Tab::Blueprint(table) };
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        settle(&mut h);
        show(&mut h, Panel::Preview);
        let dock = right_dock_rect(&h);
        let y = dock.center().y;
        for dx in [-2.5, -1.0, 1.0, 2.5] {
            let cursor = cursor_at(&mut h, egui::pos2(dock.left() + dx, y));
            assert_eq!(cursor, egui::CursorIcon::ResizeHorizontal, "{what}: at {dx} from its edge");
        }
        let inside = cursor_at(&mut h, egui::pos2(dock.left() + 8.0, y));
        assert_ne!(inside, egui::CursorIcon::ResizeHorizontal, "{what}: 8 points into the dock");
        let first = dock.width();
        let from = egui::pos2(dock.left() - 2.0, y);
        drag(&mut h, from, from - egui::vec2(100.0, 0.0));
        settle(&mut h);
        let dragged = right_dock_rect(&h).width();
        assert!((dragged - first - 100.0).abs() < 2.5, "{what}: {first} dragged to {dragged}");
        let s = &h.state().settings;
        let kept = if size.0 > 3000.0 {
            s.right_dock_width_wide.or(s.right_dock_width_wide_one)
        } else {
            s.right_dock_width
        };
        let kept_blueprint = h
            .state()
            .settings
            .dock_layouts
            .values()
            .any(|l| l.wide_width.is_some_and(|w| (w as f32 - dragged).abs() < 1.5));
        let kept = kept.is_some_and(|w| (w as f32 - dragged).abs() < 1.5) || kept_blueprint;
        assert!(kept, "{what}: {dragged} kept");
        whole(&mut h, table, &format!("{what}, the dock dragged to {dragged}"));
        let edge = |h: &Harness<'_, Moonglow>| egui::pos2(right_dock_rect(h).left() - 2.0, y);
        let from = edge(&h);
        drag(&mut h, from, egui::pos2(10.0, y));
        settle(&mut h);
        let most = right_dock_rect(&h).width();
        assert!(most <= 0.4 * size.0 + 1.5, "{what}: held to two fifths, {most}");
        let from = edge(&h);
        drag(&mut h, from, egui::pos2(size.0 - 10.0, y));
        settle(&mut h);
        let least = right_dock_rect(&h).width();
        assert!((least - mg_ui::RIGHT_DOCK_LEAST).abs() < 1.5, "{what}: held to 280, {least}");
        whole(&mut h, table, &format!("{what}, the dock at its least"));
        right_dock_width(&mut h, 320.0);
        h.state_mut().settings.right_dock_width_wide = None;
        h.state_mut().settings.right_dock_width_wide_one = None;
        for l in h.state_mut().settings.dock_layouts.values_mut() {
            l.wide_width = None;
        }
    }
}

/// The left edge of a floating form's own Preview pane (M5's W8: a
/// placeable's window at its first width in a 2600-point screen, its
/// pane beside the form): the resize cursor within 2.5 points either side
/// of it (the form beside it took the 3 points outside egui's edge, 3
/// points in all to find); dragged from just outside it 100 points wider,
/// the pane follows.
#[test]
fn a_floating_form_s_preview_pane_edge_is_dragged() {
    let Some((mut h, _area)) = area_harness("preview-window-edge") else { return };
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.set_size(egui::vec2(2600.0, 1200.0));
    h.state_mut().settings.window_sizes.clear();
    open_blueprint(&mut h, chest, false);
    let pane = |h: &Harness<'_, Moonglow>| {
        let dock = h.state().right_dock_rect();
        let rows = mg_uikit::header::records(&h.ctx, mg_ui::model_view::MODEL_HEADER);
        let row = rows.iter().find(|r| !dock.is_some_and(|d| d.contains_rect(r.row)));
        row.expect("the window's pane's model header").row
    };
    let first = pane(&h);
    let y = first.bottom() + 200.0;
    // (Its edge: the pane's line, a point left of its header.)
    let edge = first.left() - 1.0;
    for dx in [-2.5, -1.0, 1.0, 2.5] {
        let cursor = cursor_at(&mut h, egui::pos2(edge + dx, y));
        assert_eq!(cursor, egui::CursorIcon::ResizeHorizontal, "at {dx} from the pane's edge");
    }
    let from = egui::pos2(edge - 2.0, y);
    drag(&mut h, from, from - egui::vec2(100.0, 0.0));
    settle(&mut h);
    let after = pane(&h);
    let wider = after.width() - first.width();
    assert!((wider - 100.0).abs() < 2.5, "dragged 100 wider: {first:?} to {after:?}");
}

/// The Right Dock's edge seam's id (`shell::right_edge_seam`).
fn right_edge_seam() -> egui::Id {
    egui::Id::new("right-dock-edge").with("seam")
}

/// The edge seams made last frame (`shell::edge_seam`'s: a drag
/// alone, at most 8 points wide, taller than 40, not a window's own
/// edge; egui's own panel edge senses a click too).
fn edge_seams(h: &Harness<'_, Moonglow>) -> Vec<(egui::Id, egui::Rect)> {
    h.ctx.viewport(|v| {
        (v.prev_pass.widgets.layers())
            .flat_map(|(_, w)| w.iter())
            .filter(|w| w.sense == egui::Sense::DRAG)
            .filter(|w| w.rect.width() <= 8.0 && w.rect.height() > 40.0)
            // (Not a window's own edges, egui's `Window` resize.)
            .filter(|w| {
                let edges = egui::Id::new(w.layer_id).with("edge_drag");
                !["left", "right"].iter().any(|side| w.id == edges.with(side))
            })
            .map(|w| (w.id, w.rect))
            .collect()
    })
}

/// How a line at `x` in `image` is lit towards `want` against `rest` (the
/// same place, `rx`, unlit), at height `y`: of the columns about it (a
/// line a point wide falls between two, blended), the one that came
/// nearest `want`: its colour, how far from `want` (the largest
/// channel's difference) and how much nearer than at rest.
fn lit_towards(
    rest: &image::RgbaImage,
    rx: f32,
    image: &image::RgbaImage,
    x: f32,
    y: f32,
    ppp: f32,
    want: egui::Color32,
) -> ([u8; 3], i32, i32) {
    let w = [want.r(), want.g(), want.b()];
    let at = |im: &image::RgbaImage, x: f32| {
        let p = im.get_pixel((x * ppp) as u32, (y * ppp) as u32);
        let c = [p[0], p[1], p[2]];
        (c, c.iter().zip(w).map(|(a, b)| i32::from(a.abs_diff(b))).max().unwrap())
    };
    (-1..=2)
        .map(|dx| {
            let (c, off) = at(image, x + dx as f32);
            let (_, before) = at(rest, rx + dx as f32);
            (c, off, before - off)
        })
        .max_by_key(|(_, _, gain)| *gain)
        .unwrap()
}

/// The Right Dock's edge seam lit in the kit's accent (M5's W8, B5 of
/// review C; the colour by review D): beside a placeable's tab docked, in
/// Dark and in Light, the pointer 2 points outside the edge (where egui's
/// own edge gives way to the form) draws the line in the accent, and a
/// drag too, at the edge where it is dragged to; the pointer away, not.
/// Lit, it stands apart from both its sides: the form at its left and the
/// Preview's dark picture at its right.
#[test]
fn the_right_dock_s_seam_is_lit_on_hover_and_drag() {
    for light in [false, true] {
        let name = format!("preview-seam-lit-{}", if light { "light" } else { "dark" });
        let Some((mut h, _area)) = wide_harness(&name, &[("w8_table", 732)]) else { return };
        let table = ResKey::parse("w8_table", ResType::UTP).unwrap();
        h.state_mut().settings.light_theme = light;
        h.state_mut().settings.window_places = vec![("placeable".into(), "middle".into())];
        h.set_size(egui::vec2(1600.0, 1000.0));
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(table)));
        settle(&mut h);
        show(&mut h, Panel::Preview);
        let theme = if light { mg_uikit::Theme::Light } else { mg_uikit::Theme::Dark };
        assert_eq!(mg_uikit::applied(&h.ctx), Some(theme), "the theme applied");
        let accent = mg_uikit::tokens(&h.ctx).accent;
        let (hovered, active) = (accent, accent);
        let ppp = h.ctx.pixels_per_point();
        let dock = right_dock_rect(&h);
        let (x, y) = (dock.left(), dock.center().y);
        // (The line looked at well away from the pointer.)
        let look = y + 150.0;
        h.hover_at(egui::pos2(dock.left() - 300.0, y));
        h.run_steps(2);
        let rest = h.render().expect("render");
        let what = format!("{theme:?}");
        h.hover_at(egui::pos2(x - 2.0, y));
        h.run_steps(2);
        let image = h.render().expect("render");
        let (lit, off, gain) = lit_towards(&rest, x, &image, x, look, ppp, hovered);
        assert!(
            gain >= 30 && off <= 40,
            "{what}: hovered, the edge {lit:?}, {off} from {hovered:?}, {gain} nearer than at rest"
        );
        for side in [x - 8.0, x + 8.0] {
            let p = image.get_pixel((side * ppp) as u32, (look * ppp) as u32);
            let apart = lit.iter().zip(&p.0[..3]).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
            assert!(apart >= 60, "{what}: the lit edge {lit:?} beside {:?} at {side}", &p.0[..3]);
        }
        h.drag_at(egui::pos2(x - 2.0, y));
        h.run_steps(1);
        for k in 1..=4 {
            h.hover_at(egui::pos2(x - 2.0 - 5.0 * k as f32, y));
            h.run_steps(1);
        }
        let moved = right_dock_rect(&h).left();
        assert!(moved < x - 10.0, "{what}: dragged from {x} to {moved}");
        let image = h.render().expect("render");
        let (lit, off, gain) = lit_towards(&rest, x, &image, moved, look, ppp, active);
        assert!(
            gain >= 30 && off <= 40,
            "{what}: dragged, the edge {lit:?}, {off} from {active:?}, {gain} nearer than at rest"
        );
        h.drop_at(egui::pos2(x - 22.0, y));
        settle(&mut h);
    }
}

/// The keyboard's Tab passes the Right Dock's seams (M5's W8: egui's
/// `Sense::drag()` is focusable too, and a seam made last took the keys
/// as Tab came to it): beside an area, a placeable selected, at 1600 (the
/// sections' seam, the dock's edge) and in the wide layout (the columns'
/// seam, the dock's edge), Tab round every control never stops on one.
#[test]
fn tab_passes_the_right_dock_s_seams() {
    let Some((mut h, area)) = wide_harness("preview-seams-tab", &[("w8_table", 732)]) else {
        return;
    };
    let mut seams = vec![right_edge_seam(), egui::Id::new("right-dock-column-seam")];
    for c in 0..2 {
        for i in 0..4 {
            seams.push(egui::Id::new((format!("right-dock-column-{c}").as_str(), "seam", i)));
        }
    }
    for (size, kinds) in [((1600.0, 1000.0), "sections"), ((3440.0, 1440.0), "columns")] {
        h.set_size(egui::vec2(size.0, size.1));
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        // (Not `settle`: once an object is selected, the Outliner brings its
        // row into sight over a few frames.)
        h.run_steps(30);
        h.state_mut().area_views.get_mut(&area).unwrap().selection =
            vec![(mg_area::ObjectKind::Placeable, 0)];
        // (Not `settle`: the Outliner brings the selected row into sight
        // over a few frames.)
        h.run_steps(30);
        let made: Vec<egui::Id> =
            seams.iter().copied().filter(|s| h.ctx.read_response(*s).is_some()).collect();
        assert!(made.contains(&right_edge_seam()), "{kinds}: the dock's edge seam made");
        let between = made.iter().filter(|s| **s != right_edge_seam()).count();
        assert!(between > 0, "{kinds}: a seam between the dock's {kinds}");
        h.ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap_or(egui::Id::NULL)));
        let mut seen = std::collections::HashSet::new();
        for _ in 0..600 {
            h.key_press(egui::Key::Tab);
            h.step();
            let Some(focused) = h.ctx.memory(|m| m.focused()) else { continue };
            assert!(!made.contains(&focused), "{kinds}: Tab stopped on a seam ({focused:?})");
            if !seen.insert(focused) {
                break;
            }
        }
        assert!(seen.len() > 10, "{kinds}: Tab went round {} controls", seen.len());
    }
}

/// Where the Right Dock's edge cannot be dragged there is no seam (M5's
/// W8): in the least window's (under 1400 the dock is held at 280) and
/// folded to its strip, no seam made, no resize cursor about its edge,
/// and a drag from beside it leaves the dock as it was.
#[test]
fn no_seam_where_the_right_dock_s_edge_is_held() {
    let Some((mut h, _area)) = wide_harness("preview-seam-held", &[("w8_table", 732)]) else {
        return;
    };
    let table = ResKey::parse("w8_table", ResType::UTP).unwrap();
    h.state_mut().settings.window_places = vec![("placeable".into(), "middle".into())];
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(table)));
    // (Drawn at 1600 first: the seam is there.)
    h.set_size(egui::vec2(1600.0, 1000.0));
    settle(&mut h);
    show(&mut h, Panel::Preview);
    assert!(h.ctx.read_response(right_edge_seam()).is_some(), "the seam at 1600");
    for held in ["the least window", "folded"] {
        match held {
            "folded" => {
                h.set_size(egui::vec2(1600.0, 1000.0));
                h.state_mut().settings.hide_palettes = true;
                settle(&mut h);
            }
            _ => {
                h.set_size(egui::vec2(1300.0, 900.0));
                settle(&mut h);
                show(&mut h, Panel::Preview);
            }
        }
        h.run_steps(2);
        assert!(h.ctx.read_response(right_edge_seam()).is_none(), "{held}: a seam made");
        assert!(edge_seams(&h).is_empty(), "{held}: {:?}", edge_seams(&h));
        // (Folded, a strip, not the dock: the seam's absence is all.)
        let Some(dock) = h.state().right_dock_rect() else {
            assert_eq!(held, "folded", "the Right Dock drawn in the least window");
            continue;
        };
        let y = dock.center().y;
        for dx in [-2.5, -1.0] {
            let cursor = cursor_at(&mut h, egui::pos2(dock.left() + dx, y));
            assert_ne!(cursor, egui::CursorIcon::ResizeHorizontal, "{held}: at {dx} from its edge");
        }
        let from = egui::pos2(dock.left() - 2.0, y);
        drag(&mut h, from, from - egui::vec2(100.0, 0.0));
        settle(&mut h);
        let after = right_dock_rect(&h);
        assert!(
            (after.width() - dock.width()).abs() < 0.5,
            "{held}: {dock:?} dragged to {after:?}"
        );
        h.state_mut().settings.hide_palettes = false;
    }
}

/// A floating script's pane held to half its window (narrower than twice
/// 280: its least is its most) has no seam at its edge; wider, it has
/// (M5's W8).
#[test]
fn no_seam_at_a_floating_pane_held_to_half_its_window() {
    let dir = mg_testkit::scratch_dir("preview-seam-half");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = mg_module::Module::open(&path).unwrap();
    m.set(key, b"void main()\n{\n}\n".to_vec());
    m.save().unwrap();
    let mut app = app_in(Mode::Floating, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    // (The window's pane's: any but the Right Dock's.)
    let seams = |h: &Harness<'_, Moonglow>| {
        let seams = edge_seams(h).into_iter();
        seams.filter(|(id, _)| *id != right_edge_seam()).collect::<Vec<_>>()
    };
    assert!(!seams(&h).is_empty(), "wide, the pane's seam");
    h.set_size(egui::vec2(500.0, 700.0));
    settle(&mut h);
    h.run_steps(2);
    assert!(seams(&h).is_empty(), "held to half the window: {:?}", seams(&h));
}

/// Where `source`'s model reaches in its picture as last drawn (0..1 from
/// its top-left, by the picture's own shape), seen from its camera now.
fn projected_of(h: &Harness<'_, Moonglow>, source: &mg_ui::model_view::Source) -> Vec<glam::Vec2> {
    let view = h.state().model_views.get(source).expect("the model's view");
    let picture = view.shown_at().expect("its picture drawn");
    let camera = view.orbit.camera();
    let aspect = picture.width() / picture.height();
    let reached = view.reached();
    assert!(!reached.is_empty(), "something to fit");
    reached
        .iter()
        .map(|p| mg_area::pick::project(&camera, aspect, *p).expect("in front of the camera"))
        .collect()
}

/// How near the edge of its picture (of the half picture, either way)
/// `source`'s model comes at its distance now, turned every 15° about it
/// at its pitch now: by the fit's room, 1/1.08 where it is framed for
/// every yaw and no farther (computed, not drawn).
fn nearest_over_turns(h: &Harness<'_, Moonglow>, source: &mg_ui::model_view::Source) -> f32 {
    let view = h.state().model_views.get(source).expect("the model's view");
    let picture = view.shown_at().expect("its picture drawn");
    let aspect = picture.width() / picture.height();
    let reached = view.reached();
    let mut most = 0f32;
    for step in 0..24 {
        let yaw = (15.0 * step as f32).to_radians();
        let camera = mg_ui::area_view::Orbit { yaw, ..view.orbit }.camera();
        for p in &reached {
            let q = mg_area::pick::project(&camera, aspect, *p).expect("in front");
            most = most.max((2.0 * q.x - 1.0).abs().max((2.0 * q.y - 1.0).abs()));
        }
    }
    most
}

/// `source`'s model inside its picture (`what`, for the failure).
#[track_caller]
fn inside(h: &Harness<'_, Moonglow>, source: &mg_ui::model_view::Source, what: &str) {
    let points = projected_of(h, source);
    let span = |of: fn(&glam::Vec2) -> f32| {
        points.iter().map(of).fold((f32::MAX, f32::MIN), |(l, r), v| (l.min(v), r.max(v)))
    };
    let ((l, r), (t, b)) = (span(|p| p.x), span(|p| p.y));
    assert!(
        l >= -0.001 && r <= 1.001 && t >= -0.001 && b <= 1.001,
        "{what}: reaches {l:.3}..{r:.3} across, {t:.3}..{b:.3} up"
    );
}

/// `source`'s framed model turned all the way round in 15° steps at the
/// pitch it is at (the camera as a drag leaves it; a tilt is not fitted,
/// by the owner's ruling "fit: b"): inside the picture at every step, at
/// the one distance Frame gave; and at some step it comes
/// near the picture's edge (the fit no larger than it needs: its room
/// leaves 1/1.08 of the half picture, and the sweep's steps are coarser
/// than the fit's). Gives that nearest, of the half picture.
#[track_caller]
fn swept(h: &mut Harness<'_, Moonglow>, source: &mg_ui::model_view::Source, what: &str) -> f32 {
    let framed = h.state().model_views[source].orbit;
    let mut most = 0f32;
    for step in 0..24 {
        let o = &mut h.state_mut().model_views.get_mut(source).unwrap().orbit;
        o.yaw = (15.0 * step as f32).to_radians();
        h.run_steps(1);
        let now = h.state().model_views[source].orbit;
        assert_eq!(now.distance, framed.distance, "{what}: the distance held");
        assert_eq!(now.pitch, framed.pitch, "{what}: the pitch held");
        inside(h, source, &format!("{what}, yaw {}°", 15 * step));
        let out = projected_of(h, source)
            .iter()
            .map(|p| (2.0 * p.x - 1.0).abs().max((2.0 * p.y - 1.0).abs()))
            .fold(0f32, f32::max);
        most = most.max(out);
    }
    eprintln!("{what}: nearest the edge {most:.3} of the half picture");
    assert!(most > 0.88, "{what}: framed larger than it needs, nearest the edge {most:.3}");
    most
}

/// Turns `key`'s model in the Preview by a drag of `by` (degrees about
/// it, across; over or under it, up) across its picture (0.01 radian a
/// point, `ModelView`'s drag).
fn turn(h: &mut Harness<'_, Moonglow>, key: ResKey, by: egui::Vec2) {
    let from = picture_rect(h, key).center();
    let to = from + egui::vec2(by.x.to_radians(), by.y.to_radians()) / 0.01;
    h.hover_at(from);
    h.run_steps(1);
    h.drag_at(from);
    h.run_steps(1);
    h.hover_at(from + (to - from) * 0.5);
    h.run_steps(1);
    h.hover_at(to);
    h.run_steps(1);
    h.drop_at(to);
    h.run_steps(2);
}

/// M5's W8, the owner's rulings "preview turn: a" (W8-1 of review C) and
/// "fit: b": a framed model stays whole however it is turned about it.
/// The plank of our own (6 m) and the game's long table, framed at yaws
/// 60 (the first view's), 0, -30 and 90 in the docked Preview (319 by
/// 864): turned by drags across, inside at every step and at one
/// distance, and swept all the way round; then tilted over and under it,
/// at the same distance (a tilt is not fitted). (The fit for the angle it
/// was made at carried the plank framed end-on to -1.3..2.3 of the width.)
#[test]
fn a_framed_model_stays_whole_as_it_turns() {
    let rows = [("w8_plank", 0), ("w8_table", 732)];
    let Some((mut h, _area)) = wide_harness("preview-turned", &rows) else { return };
    h.state_mut().settings.window_places = vec![("placeable".into(), "middle".into())];
    h.set_size(egui::vec2(1600.0, 1000.0));
    right_dock_width(&mut h, 320.0);
    for name in ["w8_plank", "w8_table"] {
        let key = ResKey::parse(name, ResType::UTP).unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
        settle(&mut h);
        let picture = picture_rect(&h, key);
        let shape = (picture.width().round(), picture.height().round());
        assert!((318.0..=320.0).contains(&shape.0) && shape.1 > 800.0, "the picture {shape:?}");
        let source = mg_ui::model_view::Source::Resource(key);
        for yaw in [60f32, 0.0, -30.0, 90.0] {
            let o = &mut h.state_mut().model_views.get_mut(&source).unwrap().orbit;
            (o.yaw, o.pitch) = (yaw.to_radians(), 20f32.to_radians());
            h.get_by_label(mg_ui::model_view::MODEL_HEADER).get_by_label("Frame").click();
            settle(&mut h);
            let distance = h.state().model_views[&source].orbit.distance;
            for across in [15.0, 15.0, 30.0, -90.0] {
                turn(&mut h, key, egui::vec2(across, 0.0));
                let what = format!("{name} framed at {yaw}°, dragged {across}° across");
                inside(&h, &source, &what);
                let now = h.state().model_views[&source].orbit.distance;
                assert!((now - distance).abs() < 1e-3, "{what}: {distance} became {now}");
            }
            swept(&mut h, &source, &format!("{name} framed at {yaw}°"));
            for over in [25.0, -50.0] {
                turn(&mut h, key, egui::vec2(0.0, over));
                let now = h.state().model_views[&source].orbit.distance;
                let what = format!("{name} framed at {yaw}°, tilted {over}°");
                assert!((now - distance).abs() < 1e-3, "{what}: {distance} became {now}");
            }
        }
    }
}

/// The same in a wide picture (M5's W8, review D's case): the plank's
/// model in a window of its own, its picture far wider than tall, framed at
/// the first view's angle and swept all the way round. Tilted steeply, it
/// reaches past the picture at the same distance: by the owner's ruling
/// "fit: b" (2026-11-22) a tilt is not fitted (fitting it made a creature
/// in the tall Preview about a quarter smaller).
#[test]
fn a_model_in_a_wide_picture_stays_whole_as_it_turns() {
    let Some((mut h, _area)) = wide_harness("preview-turned-wide", &[("w8_plank", 0)]) else {
        return;
    };
    h.set_size(egui::vec2(1600.0, 1000.0));
    let model = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Model(model)));
    settle(&mut h);
    // (Its window made as wide as review D's, 906 by 631 of picture.)
    own_window(&mut h, &Tab::Model(model), egui::pos2(100.0, 80.0), egui::vec2(920.0, 700.0));
    settle(&mut h);
    let source = mg_ui::model_view::Source::Resource(model);
    let picture = h.state().model_views[&source].shown_at().expect("drawn");
    assert!(picture.width() > 1.4 * picture.height(), "a wide picture: {picture:?}");
    h.get_by_label(mg_ui::model_view::MODEL_HEADER).get_by_label("Frame").click();
    settle(&mut h);
    inside(&h, &source, "framed");
    swept(&mut h, &source, "the plank's tab");
    let framed = h.state().model_views[&source].orbit;
    let o = &mut h.state_mut().model_views.get_mut(&source).unwrap().orbit;
    (o.yaw, o.pitch) = (0.0, -1.45);
    h.run_steps(2);
    assert_eq!(h.state().model_views[&source].orbit.distance, framed.distance, "not refitted");
    let reach = projected_of(&h, &source).iter().map(|p| p.y).fold(0f32, f32::max);
    assert!(reach > 1.0, "tilted under it, end-on: reaches {reach:.3} of the height");
}

/// A framed model's file read again (exported again into the override)
/// keeps its camera; its picture then given another shape, it is framed
/// by its points as they are now, not by the fit Frame kept of the old
/// ones (M5's W8, review D: the fit kept for a dock drag's refits).
#[test]
fn a_model_read_again_is_framed_anew_by_its_new_points() {
    let Some((mut h, _area)) = wide_harness("preview-read-again", &[("w8_plank", 0)]) else {
        return;
    };
    h.set_size(egui::vec2(1600.0, 1000.0));
    let model = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    let tab = Tab::Model(model);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(&mut h);
    own_window(&mut h, &tab, egui::pos2(100.0, 80.0), egui::vec2(920.0, 700.0));
    settle(&mut h);
    let source = mg_ui::model_view::Source::Resource(model);
    h.get_by_label(mg_ui::model_view::MODEL_HEADER).get_by_label("Frame").click();
    settle(&mut h);
    // (Exported again into the module: twice as long.)
    let longer = PLANK.replace("\n    -3 ", "\n    -6 ").replace("\n    3 ", "\n    6 ");
    assert_ne!(longer, PLANK, "the plank made longer");
    h.state_mut().ws.as_mut().unwrap().module.set(model, longer.into_bytes());
    let kept = h.state().model_views[&source].orbit;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while h.state().model_views[&source].reached().iter().all(|p| p.x.abs() < 4.0) {
        assert!(std::time::Instant::now() < deadline, "the model read again");
        h.run_steps(1);
    }
    assert_eq!(h.state().model_views[&source].orbit, kept, "its camera kept");
    own_window(&mut h, &tab, egui::pos2(100.0, 80.0), egui::vec2(600.0, 700.0));
    settle(&mut h);
    inside(&h, &source, "read again, its picture narrower");
    swept(&mut h, &source, "the longer plank");
}

/// The Preview's arrow keys (F6 to it) turn the framed plank about it,
/// inside the picture at every key, and over and under it as far as they
/// go at the same distance (M5's W8; a tilt is not fitted, the owner's
/// ruling "fit: b", so not asserted whole).
#[test]
fn the_arrow_keys_turn_a_framed_model_whole() {
    let Some((mut h, area)) = wide_harness("preview-turned-keys", &[("w8_plank", 0)]) else {
        return;
    };
    let plank = ResKey::parse("w8_plank", ResType::UTP).unwrap();
    h.set_size(egui::vec2(1600.0, 1000.0));
    h.state_mut().palette.selected = Some(plank);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    settle(&mut h);
    show(&mut h, Panel::Preview);
    h.hover_at(egui::pos2(800.0, 20.0));
    for _ in 0..8 {
        if mg_ui::palette_view::arrow_keys_are(&h.ctx) == "Preview" {
            break;
        }
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Preview", "the keys the Preview's");
    let source = mg_ui::model_view::Source::Resource(plank);
    let distance = h.state().model_views[&source].orbit.distance;
    for (key, times) in [
        (egui::Key::ArrowRight, 24),
        (egui::Key::ArrowLeft, 30),
        (egui::Key::ArrowUp, 9),
        (egui::Key::ArrowDown, 18),
    ] {
        for i in 0..times {
            h.key_press(key);
            settle(&mut h);
            let o = h.state().model_views[&source].orbit;
            let what = format!("{key:?} {i}: yaw {}, pitch {}", o.yaw, o.pitch);
            if matches!(key, egui::Key::ArrowRight | egui::Key::ArrowLeft) {
                inside(&h, &source, &what);
            }
            assert!((o.distance - distance).abs() < 1e-3, "{what}: the distance held");
        }
    }
}

/// The hak editor's Preview (M5's W8): the plank's model in a hak of ours,
/// chosen in the editor, framed and swept all the way round.
#[test]
fn the_hak_editor_s_preview_stays_whole_as_it_turns() {
    use mg_module::hak_edit::{Hak, Source};
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let path = mg_testkit::scratch_dir("preview-turned-hak").join("zz_plank.hak");
    let model = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    let mut hak = Hak::new();
    hak.add_sources(vec![(model, Source::Bytes(PLANK.as_bytes().to_vec().into()))]).unwrap();
    hak.save(Some(&path)).unwrap();
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    app.set_render_state(rs.clone());
    let mut h = gpu_harness(app, rs, 1600.0, 1000.0);
    settle(&mut h);
    let id = mg_ui::hak_view::open_at(h.state_mut(), &path).unwrap();
    settle(&mut h);
    h.state_mut().haks.iter_mut().find(|d| d.id == id).unwrap().choose(model);
    settle(&mut h);
    let source = (h.state().model_views.keys())
        .find(|s| matches!(s, mg_ui::model_view::Source::Hak { .. }))
        .cloned()
        .expect("the hak's model's view");
    inside(&h, &source, "framed");
    swept(&mut h, &source, "the hak's plank");
}

/// The Appearance Gallery's Preview (a picker's live preview, M5's W8):
/// the plank's appearance chosen, framed and swept all the way round.
#[test]
fn the_gallery_s_preview_stays_whole_as_it_turns() {
    let Some((mut h, _area)) = wide_harness("preview-turned-gallery", &[("w8_plank", 0)]) else {
        return;
    };
    let plank = ResKey::parse("w8_plank", ResType::UTP).unwrap();
    h.set_size(egui::vec2(1600.0, 1000.0));
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    h.state_mut().settings.window_places = vec![("placeable".into(), "middle".into())];
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(plank)));
    settle(&mut h);
    h.get_by_label("Gallery…").click();
    settle(&mut h);
    let source = mg_ui::model_view::Source::Appearance { restype: ResType::UTP, row: 0 };
    assert!(h.state().model_views.contains_key(&source), "the plank's row in the Preview");
    inside(&h, &source, "framed");
    swept(&mut h, &source, "the gallery's plank");
}
