//! The Right Dock's Preview panel (the redesign's M3, T6), the Palette's
//! neighbor on the dock's tab bar: what it shows of the blueprint chosen
//! in the palette; each tab showing its panel alone, the Preview drawing
//! nothing behind the Palette's; Tools › Preview and the folded strip's
//! buttons; the tabs' words or icons by the dock's width; the arrow keys
//! on the tabs; which tab is in front as a module opens; Escape over it.

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

/// Which tab is in front is kept as the Left Dock's is, for the run
/// (nothing of it in the settings), and a module opened brings the
/// Palette to the front.
#[test]
fn the_palette_is_in_front_as_a_module_opens() {
    let Some(mut h) = preview_harness("ui-preview-front", 1400.0, 800.0) else { return };
    show(&mut h, Panel::Preview);
    // (Saved and read back: nothing of it in the settings.)
    let saved = h.state().settings.saved_text();
    let fields = ["right_panel", "hide_preview", "preview_height", "preview:"];
    assert!(!fields.iter().any(|f| saved.contains(f)), "{saved}");
    // Another module (the same, opened again): the Palette in front.
    let dir = mg_testkit::scratch_dir("ui-preview-front-again");
    h.state_mut().open_module(&sample_module(&dir));
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Palette));
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
