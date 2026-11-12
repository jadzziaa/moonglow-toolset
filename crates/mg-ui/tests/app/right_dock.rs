//! The Right Dock and its Palette panel (the redesign's M3, T5): shown
//! and folded by Tools › Palettes, View › Right Dock, their keys, its fold
//! button and its strip; the palette laid out inside it at every width;
//! F6 through the docks; settings and records of the palettes' pane read;
//! the plugins' pane; the palettes' lists' own scroll places.

use crate::common::*;
use mg_module::palette::BlueprintKind;

/// The sample module with the game's data, opened, at `width` by 800.
fn dock_harness(name: &str, width: f32) -> Option<Harness<'static, Moonglow>> {
    let root = mg_testkit::nwn_root()?;
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, width, 800.0);
    settle(&mut h);
    Some(h)
}

/// The Right Dock folded or not: what the settings say, and what is
/// drawn (its fold button, or its strip's Palette button and the button
/// that unfolds it).
fn folded(h: &Harness<'_, Moonglow>) -> bool {
    let folded = h.state().settings.hide_palettes;
    if folded {
        assert!(h.query_by_label("Fold the Right Dock").is_none());
        h.get_by_label("Show the Right Dock");
        h.get_by_label("Right Dock").get_by_label("Palette");
        assert!(h.query_by_label("Palette types").is_none(), "the palette not drawn");
    } else {
        h.get_by_label("Fold the Right Dock");
        h.get_by_label("Palette types");
        assert!(palette_shown(h.state()));
    }
    folded
}

/// The palette list's first row in sight: its name (the rows under the
/// palette's header, 140 points down the dock).
fn top_label(h: &Harness<'_, Moonglow>) -> String {
    let dock = right_dock_rect(h);
    let list_top = dock.top() + 140.0;
    h.query_all_by(|n| n.role() == egui::accesskit::Role::Button)
        .filter(|n| dock.contains(n.rect().center()) && n.rect().top() > list_top)
        .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .and_then(|n| n.accesskit_node().label())
        .unwrap_or_default()
}

/// Clicks the palette's Expand All (not the Module panel's).
fn expand_all_in_the_dock(h: &mut Harness<'_, Moonglow>) {
    let dock = right_dock_rect(h);
    let button = h.get_all_by_label("Expand All").find(|n| dock.contains(n.rect().center()));
    button.expect("the palette's Expand All").click();
    settle(h);
}

/// Tools › Palettes shows the Right Dock unfolded on the Palette panel,
/// folded or not (asked again, it stays); View › Right Dock and its key
/// (Ctrl+Alt+2, the palettes' as before: the command keeps its id) fold
/// and unfold it, the row ticked while it is shown; the strip's Palette
/// button unfolds it.
#[test]
fn tools_palettes_and_view_right_dock_show_and_fold_it() {
    let Some(mut h) = dock_harness("ui-right-dock-commands", 1400.0) else { return };
    assert!(!folded(&h), "shown with the module");
    let ticked = |h: &mut Harness<'_, Moonglow>| {
        h.get_by_label("View").click();
        h.run();
        let row = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::CheckBox
                && n.label().as_deref() == Some("Right Dock")
        };
        let node = h.get(egui_kittest::kittest::by().predicate(row));
        let on = node.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True);
        h.key_press(egui::Key::Escape);
        settle(h);
        on
    };
    assert!(ticked(&mut h), "ticked while shown");
    // View › Right Dock folds it, and shows it again.
    menu_command(&mut h, "View", "Right Dock");
    settle(&mut h);
    assert!(folded(&h));
    assert!(!ticked(&mut h), "not ticked folded");
    menu_command(&mut h, "View", "Right Dock");
    settle(&mut h);
    assert!(!folded(&h));
    // Its key.
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    h.key_press_modifiers(keys, egui::Key::Num2);
    settle(&mut h);
    assert!(folded(&h));
    // The strip's Palette button unfolds it on the Palette panel.
    h.get_by_label("Right Dock").get_by_label("Palette").click();
    settle(&mut h);
    assert!(!folded(&h));
    h.key_press_modifiers(keys, egui::Key::Num2);
    settle(&mut h);
    assert!(folded(&h));
    // Tools › Palettes: unfolded, and asked again it stays so.
    for _ in 0..2 {
        menu_command(&mut h, "Tools", "Palettes");
        settle(&mut h);
        assert!(!folded(&h));
    }
    // (The Command Palette finds the row by the dock's name.)
    assert!(
        mg_ui::commands::Id::all().into_iter().any(|id| id.name() == "Right Dock"),
        "View's row named Right Dock"
    );
}

/// With no module open there is no Right Dock (the palettes came with a
/// module), and View › Right Dock is grayed; Tools › Palettes shows it on
/// the game's standard palettes all the same (as it opened the palettes'
/// pane beside the start page before), and from then on View's row and
/// its key fold and unfold it.
#[test]
fn without_a_module_tools_palettes_shows_the_standard_palettes() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut h = harness(game_app(&root, None), 1400.0, 800.0);
    settle(&mut h);
    assert!(h.query_by_label("Right Dock").is_none(), "no Right Dock without a module");
    assert!(h.query_by_label("Fold the Right Dock").is_none());
    let row = |h: &Harness<'_, Moonglow>| menu_row(h, "Right Dock");
    h.get_by_label("View").click();
    h.run();
    assert!(row(&h).is_some());
    let grayed = h.get_by_label("Right Dock").accesskit_node().is_disabled();
    assert!(grayed, "View › Right Dock grayed with no panel to show");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    menu_command(&mut h, "Tools", "Palettes");
    settle(&mut h);
    assert!(!folded(&h));
    // The standard placeables (the custom palette is a module's).
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.kind = BlueprintKind::Placeable;
    settle(&mut h);
    h.get_by_label_contains("items");
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    h.key_press_modifiers(keys, egui::Key::Num2);
    settle(&mut h);
    assert!(folded(&h), "its key folds it");
    menu_command(&mut h, "View", "Right Dock");
    settle(&mut h);
    assert!(!folded(&h), "View's row unfolds it");
}

/// The palette inside the Right Dock at every width from its least, 280,
/// to 600, on every type of palette (the tiles' with an area open, each
/// blueprint type, prefabs, lights, the start location) and in the
/// Gallery: nothing of the panel lies outside the dock, under its fold
/// button, or over another of its widgets. (With the palettes' pane
/// dragged to about 135 px, a blueprint type's list toolbar spilled left
/// under its fold button: the dock is never so narrow.)
#[test]
fn the_palette_fits_the_right_dock_from_its_least_width_to_600() {
    let Some((mut h, _)) = area_harness("right-dock-sweep") else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    settle(&mut h);
    #[derive(Debug, Clone, Copy)]
    enum Kind {
        Tiles,
        Blueprint(BlueprintKind, bool),
        Prefabs,
        Lights,
        Start,
    }
    let mut kinds = vec![Kind::Tiles];
    kinds.extend(BlueprintKind::ALL.map(|k| Kind::Blueprint(k, false)));
    kinds.extend([
        Kind::Blueprint(BlueprintKind::Placeable, true),
        Kind::Prefabs,
        Kind::Lights,
        Kind::Start,
    ]);
    let (mut wrong, mut seen) = (Vec::new(), 0);
    for kind in kinds {
        {
            let app = h.state_mut();
            let p = &mut app.palette;
            (p.tiles, p.prefabs, p.lights, p.start) = (false, false, false, false);
            app.settings.palette_gallery = false;
            match kind {
                Kind::Tiles => p.tiles = true,
                Kind::Blueprint(k, gallery) => {
                    (p.kind, p.custom) = (k, false);
                    app.settings.palette_gallery = gallery;
                }
                Kind::Prefabs => p.prefabs = true,
                Kind::Lights => p.lights = true,
                Kind::Start => p.start = true,
            }
        }
        for width in (280..=600).step_by(16) {
            right_dock_width(&h, width as f32);
            settle(&mut h);
            let dock = right_dock_rect(&h);
            assert!((dock.width() - width as f32).abs() < 1.5, "{width}: {dock:?}");
            let fold = h.get_by_label("Fold the Right Dock").rect();
            // Every widget drawn in the dock: inside it, clear of the
            // button beside its edge.
            for n in h.query_all_by(|n| n.bounding_box().is_some()) {
                let r = n.rect();
                if !dock.contains(r.center()) || r.height() > dock.height() - 4.0 {
                    continue;
                }
                seen += 1;
                let a = n.accesskit_node();
                let name = a.label().or(a.value()).unwrap_or_default();
                if r.left() < dock.left() - 0.5 || r.right() > dock.right() + 0.5 {
                    wrong.push(format!("{kind:?} at {width}: {name:?} at {r:?}, past {dock:?}"));
                }
                if r.intersects(fold) {
                    wrong.push(format!("{kind:?} at {width}: {name:?} under the fold button"));
                }
            }
            for o in overlapping(&h) {
                wrong.push(format!("{kind:?} at {width}: {o}"));
            }
        }
    }
    assert!(seen > 2000, "the panel's widgets looked at: {seen}");
    assert!(wrong.is_empty(), "{} found:\n{}", wrong.len(), wrong.join("\n"));
}

/// F6 gives the arrow keys to the panes left to right and then down: the
/// Left Dock's panel, the editor area (the area's view), the Right Dock's
/// Palette panel, the Bottom Dock's log, and round; Shift+F6 the other
/// way. A folded Right Dock is passed over.
#[test]
fn f6_steps_through_the_docks_and_the_editor_area() {
    let Some((mut h, area)) = area_harness("right-dock-f6") else { return };
    h.state_mut().settings.hide_log = false;
    settle(&mut h);
    // (The pointer in the area's view: the keys are its.)
    let centre = h.state().area_views[&area].rect.center();
    h.hover_at(centre);
    settle(&mut h);
    let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
    assert_eq!(keys(&h), "Area");
    let step = |h: &mut Harness<'_, Moonglow>, back: bool| {
        let m = if back { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
        h.key_press_modifiers(m, egui::Key::F6);
        settle(h);
        keys(h)
    };
    // (Away from the view, which takes the keys back while the pointer
    // is in it.)
    h.hover_at(egui::pos2(centre.x, 20.0));
    settle(&mut h);
    let forward: Vec<String> = (0..4).map(|_| step(&mut h, false)).collect();
    assert_eq!(forward, ["Palette", "Log", "Tree", "Area"]);
    let back: Vec<String> = (0..4).map(|_| step(&mut h, true)).collect();
    assert_eq!(back, ["Tree", "Log", "Palette", "Area"]);
    // Folded, the Right Dock is passed over.
    fold_palette(h.state_mut());
    settle(&mut h);
    let forward: Vec<String> = (0..3).map(|_| step(&mut h, false)).collect();
    assert_eq!(forward, ["Log", "Tree", "Area"]);
}

/// A settings file of an older Moonglow, with the palettes split off the
/// middle's left (their own place, `split left 300`), a kind kept in the
/// palettes' pane (place `palettes`) and the palettes folded away: it
/// reads with no warning; the app opened with it has none of those
/// places, every other as it was, and the Right Dock folded (the
/// palettes were), its strip at the window's right; Tools › Palettes
/// unfolds it.
#[test]
fn old_settings_with_the_palettes_places_open_right() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-right-dock-old-settings");
    let old = mg_ui::Settings {
        window_places: vec![
            ("palette".into(), "split left 300".into()),
            ("factions".into(), "palettes".into()),
            ("manual".into(), "split below 330".into()),
        ],
        window_sizes: vec![("palette".into(), [400, 700]), ("script".into(), [1100, 700])],
        hide_palettes: true,
        ..Default::default()
    };
    let file = dir.join("app.ron");
    let map: std::collections::HashMap<String, String> =
        [("moonglow-settings".to_string(), old.saved_text())].into_iter().collect();
    std::fs::write(&file, ron::to_string(&map).unwrap()).unwrap();
    let read = mg_ui::Settings::read_eframe(&file, "moonglow-settings");
    assert!(read.unread.is_empty() && read.unreadable.is_none(), "{:?}", read.problem());
    assert!(read.kept.is_none(), "nothing put aside");
    let migrated = Moonglow::with_settings(read.settings, Box::new(NoDialogs::default()));
    let app = &migrated;
    assert_eq!(app.settings.window_places, [("manual".to_string(), "split below 330".to_string())]);
    assert_eq!(app.settings.window_sizes, [("script".to_string(), [1100, 700])]);
    let said: Vec<_> =
        (app.log.entries.iter()).filter(|(level, _)| *level != mg_ui::Level::Info).collect();
    assert!(said.is_empty(), "no warning: {said:?}");
    // The window with them (the game's data read as the tests read it).
    let mut app = game_app(&root, None);
    let s = &migrated.settings;
    app.settings.window_places = s.window_places.clone();
    app.settings.window_sizes = s.window_sizes.clone();
    app.settings.hide_palettes = s.hide_palettes;
    let path = sample_module(&dir);
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    assert!(folded(&h), "folded, as the palettes were");
    let show = h.get_by_label("Show the Right Dock").rect();
    assert!(show.right() > 1400.0 - 60.0, "at the window's right: {show:?}");
    // The Faction Editor, kept in the palettes' pane: where its kind
    // opens, a window of its own.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    let at = h.state().dock.find_tab(&Tab::Factions).unwrap();
    assert!(!at.surface.is_main(), "a window of its own");
    menu_command(&mut h, "Tools", "Palettes");
    settle(&mut h);
    assert!(!folded(&h));
}

/// The settings as JSON (a settings file is not; a plugin's or a test's
/// may be) with the palettes' places: migrated as the app takes them.
#[test]
fn json_settings_with_the_palettes_places_open_without_them() {
    let json = r#"{
        "window_places": [["palette", "split left 300"], ["journal", "palettes"], ["script", "middle"]],
        "window_sizes": [["palette", [400, 700]]],
        "hide_palettes": true
    }"#;
    let read: mg_ui::Settings = serde_json::from_str(json).unwrap();
    assert_eq!(read.window_places.len(), 3, "read as it is");
    let app = Moonglow::with_settings(read, Box::new(NoDialogs::default()));
    assert_eq!(app.settings.window_places, [("script".to_string(), "middle".to_string())]);
    assert!(app.settings.window_sizes.is_empty());
    assert!(app.settings.hide_palettes);
}

/// A press in the Palette panel makes it the plugins' "pane last clicked
/// in" (as a press in the palettes' tab did): a command from the Plugins
/// menu then runs on the blueprint chosen in the palette, not on what the
/// module tree has chosen.
#[test]
fn a_press_in_the_palette_makes_it_the_plugins_pane() {
    let Some((mut h, _)) = area_harness("right-dock-plugin-pane") else { return };
    {
        let p = &mut h.state_mut().palette;
        (p.tiles, p.kind, p.custom) = (false, BlueprintKind::Waypoint, false);
        p.filter = "nw_wp_tavern".into();
    }
    settle(&mut h);
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    // (The tree's choice, and its pane the one clicked in last.)
    let start = ResKey::parse("start", ResType::ARE).unwrap();
    h.state_mut().tree_selected = [start].into_iter().collect();
    h.state_mut().plugins.pane = Some(mg_ui::plugins::Pane::Tree);
    h.get_by_label(&tavern).click();
    settle(&mut h);
    assert_eq!(h.state().plugins.pane, Some(mg_ui::plugins::Pane::Palette));
    let chosen = h.state().palette.selected.expect("the waypoint chosen");
    assert_eq!(h.state().plugin_selection(), [mg_plugin::Selected::Blueprint(chosen)]);
    // A press in the panel away from its rows: the palette's still.
    h.state_mut().plugins.pane = Some(mg_ui::plugins::Pane::Tree);
    let types = h.get_by_label("Palette types").get_by_label("Waypoints").rect();
    click_pos(&h, types.center());
    settle(&mut h);
    assert_eq!(h.state().plugins.pane, Some(mg_ui::plugins::Pane::Palette));
}

/// A window maximized over the whole window (the docks too) is put back
/// by a double click on its bar wherever the bar is: over the Left Dock,
/// the editor area or the Right Dock. (Its bar's clicks were cut to the
/// editor area: over a dock, a double click did nothing.)
#[test]
fn a_maximized_window_s_bar_puts_it_back_over_the_docks() {
    let Some((mut h, area)) = area_harness("right-dock-maximized") else { return };
    let props = Tab::AreaProperties(area);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
    settle(&mut h);
    let pane = |h: &Harness<'_, Moonglow>| {
        let path = h.state().dock.find_tab(&props).unwrap();
        h.state().dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1.rect
    };
    let first = pane(&h);
    let left = h.get_by_label("Fold the Left Dock").rect().left() - 40.0;
    let middle = h.state().area_views[&area].rect.center().x;
    let right = right_dock_rect(&h).center().x;
    for x in [left, middle, right] {
        h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(props.clone()));
        h.run_steps(8);
        let big = pane(&h);
        assert!(big.width() > first.width() + 200.0, "maximized: {big:?}");
        let at = egui::pos2(x, big.top() + 10.0);
        h.hover_at(at);
        h.run_steps(2);
        for _ in 0..2 {
            press(&h, at, true, egui::Modifiers::NONE);
            press(&h, at, false, egui::Modifiers::NONE);
        }
        h.run_steps(8);
        assert!((pane(&h).size() - first.size()).length() < 3.0, "put back from {x}");
        h.hover_at(at + egui::vec2(0.0, 300.0));
        h.run_steps(30);
    }
}

/// The palette's scroll bar, at the height of the Right Dock's fold
/// button (the button stands beside the dock's inner edge, the bar at its
/// outer one): a press on it scrolls the list and folds nothing. (The
/// edge dragged above and below the button, and a press beside it:
/// `windows::each_pane_folds_by_one_button_on_its_inner_edge`.)
#[test]
fn a_press_on_the_palette_s_scroll_bar_scrolls_it() {
    let Some((mut h, _)) = area_harness("right-dock-scroll-bar") else { return };
    {
        let p = &mut h.state_mut().palette;
        (p.tiles, p.kind, p.custom) = (false, BlueprintKind::Placeable, false);
    }
    settle(&mut h);
    // Every category open: a list longer than the dock.
    expand_all_in_the_dock(&mut h);
    let dock = right_dock_rect(&h);
    let fold = h.get_by_label("Fold the Right Dock").rect();
    let before = top_label(&h);
    // The bar inside the panel's 8 px margin, at the dock's outer edge,
    // at the button's height (the kit's bar: 4 wide, 2 from the list's
    // side).
    let at = egui::pos2(dock.right() - 12.0, fold.center().y);
    drag(&mut h, at, at + egui::vec2(0.0, 120.0));
    settle(&mut h);
    assert!(!h.state().settings.hide_palettes, "the dock left open");
    assert_ne!(top_label(&h), before, "the list scrolled");
}

/// The palette's lists keep their own scroll places: the placeables'
/// scrolled down stays so through the tiles palette shown and back, the
/// tiles' list at its top meanwhile; and the Left Dock's tree keeps its
/// place too (each scroll area under an id of its own).
#[test]
fn the_palette_s_lists_keep_their_own_scroll_places() {
    let Some((mut h, _)) = area_harness("right-dock-scroll-places") else { return };
    let set = |h: &mut Harness<'_, Moonglow>, tiles: bool| {
        let p = &mut h.state_mut().palette;
        (p.tiles, p.kind, p.custom) = (tiles, BlueprintKind::Placeable, false);
        settle(h);
    };
    set(&mut h, false);
    expand_all_in_the_dock(&mut h);
    let dock = right_dock_rect(&h);
    let at_top = top_label(&h);
    let over = egui::pos2(dock.center().x, dock.bottom() - 100.0);
    h.hover_at(over);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -2000.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    h.run_steps(30);
    let scrolled = top_label(&h);
    assert_ne!(scrolled, at_top, "scrolled");
    let tree = h.state().tree_cursor;
    set(&mut h, true);
    let tiles_top = top_label(&h);
    set(&mut h, false);
    h.run_steps(10);
    assert_eq!(top_label(&h), scrolled, "the placeables' list where it was");
    set(&mut h, true);
    assert_eq!(top_label(&h), tiles_top, "the tiles' list where it was");
    assert_eq!(h.state().tree_cursor, tree);
}

/// View › Reset Layout puts the Right Dock back as it is at first:
/// unfolded, 320 points wide, however it was left.
#[test]
fn reset_layout_puts_the_right_dock_back_at_320() {
    let Some(mut h) = dock_harness("ui-right-dock-reset", 1400.0) else { return };
    right_dock_width(&h, 470.0);
    settle(&mut h);
    assert!((right_dock_rect(&h).width() - 470.0).abs() < 1.5);
    fold_palette(h.state_mut());
    settle(&mut h);
    menu_command(&mut h, "View", "Reset Layout");
    settle(&mut h);
    assert!(!folded(&h));
    let width = right_dock_rect(&h).width();
    let want = mg_uikit::metrics::RIGHT_DOCK;
    assert!((width - want).abs() < 1.5, "{width}, not {want}");
}

/// In the least window (and at 480 and 600), both side docks unfolded:
/// nothing panics, the two fold buttons stand apart, the Left Dock's
/// first, the Right Dock inside the window (narrower than its least
/// width where the window has not the room: never past its edge), and at
/// 600 nothing overlaps. (The editor area is left little room under
/// about 760: the shell's yields, M3 T14, fold the Left Dock in narrow
/// windows.)
#[test]
fn the_least_window_holds_both_docks() {
    let Some(mut h) = dock_harness("ui-right-dock-least-window", 1400.0) else { return };
    for size in [mg_ui::LEAST_WINDOW_SIZE, egui::vec2(480.0, 400.0), egui::vec2(600.0, 500.0)] {
        h.set_size(size);
        settle(&mut h);
        assert!(!h.state().settings.hide_tree && !folded(&h));
        let window = h.ctx.content_rect();
        let left = h.get_by_label("Fold the Left Dock").rect();
        let right = h.get_by_label("Fold the Right Dock").rect();
        let both = left.intersect(right);
        assert!(!both.is_positive(), "at {size:?}: {left:?} {right:?}");
        let dock = right_dock_rect(&h);
        assert!(window.expand(0.5).contains_rect(dock), "at {size:?}: {dock:?} in {window:?}");
        assert!(left.right() <= right.left() + 0.5, "the Left Dock's first: {left:?} {right:?}");
        // (Under about 500 the editor area is too narrow for the Bottom
        // Dock's tab and buttons side by side: T14's yields give it room.)
        if size.x >= 600.0 {
            let found = overlapping(&h);
            assert!(found.is_empty(), "at {size:?}:\n{}", found.join("\n"));
        }
    }
}
