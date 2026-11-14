//! The Outliner (the redesign's M3, T13): under the Left Dock's tabs, the
//! area in front's placed objects by kind. Its eyes are the area toolbar's
//! Show toggles (one state); the view's selection marks and reveals its
//! row, and a row selects in the view (one, Ctrl, Shift); its counts
//! follow an edit and Undo; it comes and goes with the active editor in
//! both modes of the Open editors switch; the seam over it is kept across
//! a restart and a short window; Expand All and Collapse All; its names
//! for a screen reader; its keys.

use crate::common::*;
use egui::accesskit::Toggled;
use mg_area::ObjectKind;

/// Frames until `done`, for ten seconds at most (an area's view asks for
/// frames as it draws, so `settle` may not end beside one).
fn until(h: &mut Harness<'_, Moonglow>, what: &str, done: impl Fn(&Harness<'_, Moonglow>) -> bool) {
    let started = std::time::Instant::now();
    while !done(h) {
        assert!(started.elapsed().as_secs() < 10, "never: {what}");
        h.step();
    }
}

/// Frames until the Outliner's rows stop moving (a row brought into sight
/// scrolls there over a few frames): the last kind's row in the same place
/// three frames running.
fn rest(h: &mut Harness<'_, Moonglow>) {
    let at = |h: &Harness<'_, Moonglow>| in_outliner(h, "Show Waypoints").rect();
    let started = std::time::Instant::now();
    let (mut was, mut same) = (at(h), 0);
    while same < 3 {
        assert!(started.elapsed().as_secs() < 10, "the Outliner never rests");
        h.step();
        let now = at(h);
        same = if now == was { same + 1 } else { 0 };
        was = now;
    }
}

/// `chests` chests placed in `area` of `m`, tagged `CHEST_00` and on.
fn place_chests(m: &mut Module, game: &mg_rules::GameData, area: ResRef, chests: usize) {
    use mg_module::instances::{Placement, Placing, instance};
    let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let items: Vec<mg_gff::Struct> = (0..chests)
        .map(|i| {
            let at = [4.0 + (i % 10) as f32 * 3.0, 4.0 + (i / 10) as f32 * 3.0, 0.0];
            let at = Placement { position: at, rotation: 0.0 };
            let mut item = instance(&placing, ResType::UTP, &bp, at, &[]).unwrap();
            item.set("Tag", mg_gff::Value::String(format!("CHEST_{i:02}").into_bytes()));
            item
        })
        .collect();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set("Placeable List", mg_gff::Value::List(items));
    m.set_gff(git_key, &git).unwrap();
}

/// [`area_harness`]'s area (two waypoints) with `chests` chests besides,
/// tagged `CHEST_00` and on, in the window as the mode has it; with
/// `second`, a second area ("Meadow") with as many chests and no
/// waypoints, its tab not opened.
fn outliner_app_with(
    name: &str,
    chests: usize,
    mode: Mode,
    second: bool,
) -> Option<(Harness<'static, Moonglow>, ResRef, Option<ResRef>)> {
    let other = std::cell::Cell::new(None);
    let place = |m: &mut Module, game: &mg_rules::GameData, area: ResRef| {
        place_chests(m, game, area, chests);
        if second {
            let spec = mg_module::new::AreaSpec {
                name: "Meadow".into(),
                tileset: ResRef::from_str("ttr01").unwrap(),
                width: 4,
                height: 4,
            };
            let mut rng = fastrand::Rng::with_seed(11);
            let b = mg_module::new::add_area(m, game, &spec, &mut rng).unwrap();
            place_chests(m, game, b, chests);
            other.set(Some(b));
        }
    };
    let (mut app, rs, area) = area_app(name, "ttr01", None, 4, None, &place)?;
    put_in(&mut app, mode);
    // (1000 high: the Outliner under the Module panel's 496 points has
    // room for its kinds and some rows.)
    let mut h = area_gpu_harness(app, rs, AREA_HARNESS_WIDTH, HEIGHT);
    until(&mut h, "the area in front", |h| h.state().area_views.contains_key(&area));
    // (The builder unfolds the Left Dock, folded at this width.)
    unfold_left_dock(&mut h);
    until(&mut h, "the Outliner", |h| h.query_by_label(mg_ui::OUTLINER).is_some());
    rest(&mut h);
    Some((h, area, other.get()))
}

/// [`outliner_app_with`] of one area.
fn outliner_app(
    name: &str,
    chests: usize,
    mode: Mode,
) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    outliner_app_with(name, chests, mode, false).map(|(h, a, _)| (h, a))
}

/// [`outliner_app`]'s window's height.
const HEIGHT: f32 = 1000.0;

/// [`outliner_app`] as an existing user's (As Floating Windows).
fn outliner_harness(name: &str, chests: usize) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    outliner_app(name, chests, Mode::Floating)
}

/// The Outliner's `name`d node.
#[track_caller]
fn in_outliner<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    h.get_by_label(mg_ui::OUTLINER).get_by_label(name)
}

/// The Outliner's row whose name contains `words` (an object's: "tag
/// CHEST_03"), if it is laid out.
fn outliner_row<'t>(
    h: &'t Harness<'_, Moonglow>,
    words: &'t str,
) -> Option<egui_kittest::Node<'t>> {
    h.get_by_label(mg_ui::OUTLINER).query_by_label_contains(words)
}

/// Whether a toggle (an eye, the toolbar's Show toggle) is pressed.
fn pressed(n: &egui_kittest::Node<'_>) -> bool {
    n.accesskit_node().toggled() == Some(Toggled::True)
}

/// Whether a row is marked (said toggled, as a selectable label is).
fn selected(n: &egui_kittest::Node<'_>) -> bool {
    pressed(n)
}

fn expanded(n: &egui_kittest::Node<'_>) -> bool {
    n.accesskit_node().data().is_expanded() == Some(true)
}

/// Where the area's camera looks.
fn looks_at(h: &Harness<'_, Moonglow>, area: ResRef) -> Option<glam::Vec3> {
    h.state().area_views[&area].orbit.map(|o| o.target)
}

fn selection(h: &Harness<'_, Moonglow>, area: ResRef) -> Vec<(ObjectKind, usize)> {
    let mut s = h.state().area_views[&area].selection.clone();
    s.sort();
    s
}

/// The eye of a kind and the area toolbar's Show toggle are one state:
/// the eye hides the kind in the view (a waypoint selected is no longer)
/// and the toolbar's toggle is off; the toolbar's toggle shows it, and the
/// eye is on again. A row of a hidden kind clicked (with Ctrl too) shows
/// the kind, and selects the object.
#[test]
fn the_eye_and_the_area_toolbar_s_show_toggle_are_one_state() {
    let Some((mut h, area)) = outliner_harness("outliner-eye", 3) else { return };
    let shown =
        |h: &Harness<'_, Moonglow>| h.state().area_views[&area].show[ObjectKind::Waypoint.index()];
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    rest(&mut h);
    assert!(shown(&h) && pressed(&in_outliner(&h, "Show Waypoints")));
    assert!(pressed(&area_tool(&h, "Show Waypoints")));
    in_outliner(&h, "Show Waypoints").click();
    until(&mut h, "hidden", |h| !shown(h));
    // (The toolbar drawn after the Outliner: the same frame; the eye the
    // next.)
    until(&mut h, "the toolbar's toggle off", |h| !pressed(&area_tool(h, "Show Waypoints")));
    until(&mut h, "the eye off", |h| !pressed(&in_outliner(h, "Show Waypoints")));
    assert!(h.state().area_views[&area].selection.is_empty(), "a hidden object is not selected");
    assert!(h.state().settings.hidden_in_areas.iter().any(|k| k == "Waypoints"), "kept");
    // The toolbar's toggle the other way.
    area_tool(&h, "Show Waypoints").click();
    until(&mut h, "shown", |h| shown(h));
    until(&mut h, "the eye on", |h| pressed(&in_outliner(h, "Show Waypoints")));
    // A row of a hidden kind: the kind shown, the object selected.
    in_outliner(&h, "Show Waypoints").click();
    until(&mut h, "hidden again", |h| !shown(h));
    if !expanded(&in_outliner(&h, "2 waypoints")) {
        in_outliner(&h, "2 waypoints").click();
    }
    until(&mut h, "its rows", |h| {
        h.get_by_label(mg_ui::OUTLINER).query_all_by_label_contains("tag NW_WAYPOINT001").count()
            == 2
    });
    rest(&mut h);
    let row = h.get_by_label(mg_ui::OUTLINER).get_all_by_label_contains("tag NW_WAYPOINT001");
    row.last().unwrap().click();
    until(&mut h, "shown and selected", |h| {
        shown(h) && h.state().area_views[&area].selection.len() == 1
    });
    // And with Ctrl, as it is chosen with others.
    in_outliner(&h, "Show Waypoints").click();
    until(&mut h, "hidden once more", |h| !shown(h) && selection(h, area).is_empty());
    rest(&mut h);
    let row = h.get_by_label(mg_ui::OUTLINER).get_all_by_label_contains("tag NW_WAYPOINT001");
    row.last().unwrap().click_modifiers(egui::Modifiers::COMMAND);
    until(&mut h, "shown and chosen", |h| shown(h) && selection(h, area).len() == 1);
}

/// An object selected in the view (as a click there selects it) opens its
/// kind in the Outliner, marks its row and brings it into sight; another
/// selected moves the mark; several selected are all marked.
#[test]
fn selecting_in_the_view_marks_and_reveals_the_row() {
    let Some((mut h, area)) = outliner_harness("outliner-reveal", 40) else { return };
    // (The Outliner's rows: under its header, over the Status Bar.)
    let top = in_outliner(&h, "Collapse All").rect().bottom();
    let rows = egui::Rect::from_x_y_ranges(0.0..=300.0, top..=HEIGHT - 24.0);
    assert!(outliner_row(&h, "tag CHEST_39").is_none(), "closed at first");
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 39)];
    until(&mut h, "the row marked", |h| {
        outliner_row(h, "tag CHEST_39").is_some_and(|n| selected(&n))
    });
    until(&mut h, "the row in sight", |h| {
        outliner_row(h, "tag CHEST_39").is_some_and(|n| rows.contains_rect(n.rect()))
    });
    assert!(expanded(&in_outliner(&h, "40 placeables")));
    assert!(!expanded(&in_outliner(&h, "2 waypoints")), "the others stay closed");
    // Another: the mark moves.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 1)];
    until(&mut h, "the mark moved", |h| {
        outliner_row(h, "tag CHEST_01").is_some_and(|n| selected(&n))
    });
    until(&mut h, "the other unmarked", |h| {
        outliner_row(h, "tag CHEST_39").is_none_or(|n| !selected(&n))
    });
    // Several: each marked.
    let several = vec![(ObjectKind::Placeable, 1), (ObjectKind::Placeable, 2)];
    h.state_mut().area_views.get_mut(&area).unwrap().selection = several;
    until(&mut h, "both marked", |h| {
        ["tag CHEST_01", "tag CHEST_02"]
            .iter()
            .all(|t| outliner_row(h, t).is_some_and(|n| selected(&n)))
    });
}

/// A click on an object's row selects it in the view (the camera where it
/// is); Ctrl+click adds another, and takes it away again; Shift+click
/// selects those from the one clicked before to it, as listed. In Select
/// Tiles, a click on a row leaves it for the objects, as a click on an
/// object in the view does.
#[test]
fn a_click_on_a_row_selects_in_the_view() {
    let Some((mut h, area)) = outliner_harness("outliner-select", 6) else { return };
    let p = |i: usize| (ObjectKind::Placeable, i);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.tile_mode = true;
        view.tile_selection = vec![(1, 1)];
    }
    in_outliner(&h, "6 placeables").click();
    until(&mut h, "its rows", |h| outliner_row(h, "tag CHEST_05").is_some());
    let camera = looks_at(&h, area);
    outliner_row(&h, "tag CHEST_00").unwrap().click();
    until(&mut h, "one selected", |h| selection(h, area) == [p(0)]);
    // (The view has drawn it: the Status Bar says it, from the frame after.)
    until(&mut h, "said selected", |h| {
        h.state().status_said().iter().any(|s| s.contains("1 selected"))
    });
    assert_eq!(looks_at(&h, area), camera, "the camera left where it is");
    let view = &h.state().area_views[&area];
    assert!(!view.tile_mode && view.tile_selection.is_empty(), "Select Tiles left");
    outliner_row(&h, "tag CHEST_02").unwrap().click_modifiers(egui::Modifiers::COMMAND);
    until(&mut h, "two selected", |h| selection(h, area) == [p(0), p(2)]);
    // (From the one clicked last, CHEST_02.)
    outliner_row(&h, "tag CHEST_04").unwrap().click_modifiers(egui::Modifiers::SHIFT);
    until(&mut h, "a run", |h| selection(h, area) == [p(2), p(3), p(4)]);
    outliner_row(&h, "tag CHEST_00").unwrap().click_modifiers(egui::Modifiers::COMMAND);
    until(&mut h, "added", |h| selection(h, area) == [p(0), p(2), p(3), p(4)]);
    outliner_row(&h, "tag CHEST_00").unwrap().click_modifiers(egui::Modifiers::COMMAND);
    until(&mut h, "taken away", |h| selection(h, area) == [p(2), p(3), p(4)]);
    // (The rows marked.)
    until(&mut h, "marked", |h| {
        (2..=4).all(|i| outliner_row(h, &format!("tag CHEST_{i:02}")).is_some_and(|n| selected(&n)))
    });
}

/// The counts follow an edit (a waypoint deleted) and its Undo, and a
/// rename shows in the row (a tag changed).
#[test]
fn the_counts_follow_an_edit_and_undo() {
    let Some((mut h, area)) = outliner_harness("outliner-counts", 2) else { return };
    let git = ResKey::new(area, ResType::GIT);
    in_outliner(&h, "2 waypoints");
    in_outliner(&h, "2 placeables");
    let delete = mg_area::edit::delete_edits(git, &[(ObjectKind::Waypoint, 0)]);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Delete", delete)));
    until(&mut h, "one left", |h| outliner_row(h, "1 waypoint").is_some());
    h.state_mut().actions.push(mg_ui::Action::Undo);
    until(&mut h, "two again", |h| outliner_row(h, "2 waypoints").is_some());
    // A tag changed: the row says it.
    let tag = mg_edit::Edit::SetField {
        key: git,
        path: mg_edit::GffPath::root().item("Placeable List", 1),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(b"RENAMED".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![tag])));
    in_outliner(&h, "2 placeables").click();
    until(&mut h, "the new tag", |h| outliner_row(h, "tag RENAMED").is_some());
    assert!(outliner_row(&h, "tag CHEST_01").is_none());
}

/// The Outliner shows while an area's tab is the active docked editor,
/// in both modes of the Open editors switch: Module Properties and a
/// placed object's Properties float beside the area in As Floating
/// Windows (a floating window never counts: the Outliner stays) and are
/// tabs brought to the front in As Tabs (it goes, and comes back with the
/// area in front); the area closed, it goes, the Left Dock the module tree
/// alone.
#[test]
fn the_outliner_comes_and_goes_with_the_active_editor() {
    for mode in MODES {
        let Some((mut h, area)) = outliner_app(&mode.dir("outliner-comes"), 0, mode) else {
            return;
        };
        let here = |h: &Harness<'_, Moonglow>| h.query_by_label(mg_ui::OUTLINER).is_some();
        // The module tree's area opened out to its kinds, in the Areas
        // group's well: over the Outliner the well has room for some and
        // the last kind (Waypoints) is out of its sight; with the tree the
        // whole dock high (no empty split left behind), it is in sight.
        h.get_by_label_contains("Placed Objects in ").click();
        until(&mut h, "the area opened out", |h| {
            h.get_by_label("Module Tree Rows").query_by_label("Waypoints (2)").is_some()
        });
        // (Under the well, the next group's row: the last kind is in the
        // well's sight where it ends above it.)
        let last_row = |h: &Harness<'_, Moonglow>| {
            let rows = h.get_by_label("Module Tree Rows");
            let kind = rows.get_by_label("Waypoints (2)").rect();
            kind.bottom() <= rows.get_by_label("Conversations (0)").rect().top() + 0.5
        };
        assert!(!last_row(&h), "{mode:?}: out of sight over the Outliner");
        let path = mg_edit::GffPath::root().item("WaypointList", 0);
        for tab in [Tab::ModuleProperties, Tab::Instance { area, path }] {
            h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
            until(&mut h, "opened", |h| h.state().dock.find_tab(&tab).is_some());
            // (The Outliner follows from the frame after the tab is in
            // front: one way or the other within a few.)
            until(&mut h, "the Outliner as the active editor has it", |h| {
                here(h) == (h.state().active_editor() == Some(&Tab::Area(area)))
            });
            until(&mut h, "where its mode puts it", |h| {
                let docked = h.state().dock.find_tab(&tab).is_some_and(|p| p.surface.is_main());
                docked == mode.tabs() && here(h) == !mode.tabs()
            });
            let docked = h.state().dock.find_tab(&tab).is_some_and(|p| p.surface.is_main());
            assert_eq!(docked, mode.tabs(), "{mode:?}: {tab:?} docked");
            assert_eq!(here(&h), !mode.tabs(), "{mode:?}: {tab:?}");
            assert_eq!(last_row(&h), mode.tabs(), "{mode:?}: {tab:?}: the tree the whole dock");
            h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
            until(&mut h, "back with the area", |h| here(h));
        }
        h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Area(area)));
        until(&mut h, "gone with the area", |h| !here(h));
        until(&mut h, "the tree the whole dock", |h| last_row(h));
        assert!(h.query_by_label("Filter Module Tree").is_some(), "{mode:?}: the tree stays");
    }
}

/// The upper panel's height over the Outliner: from the Left Dock's tab
/// bar's top to the Outliner's header's.
fn seam(h: &Harness<'_, Moonglow>) -> f32 {
    // (From the middle of the one bar to the middle of the other: each 26
    // points high.)
    let header = in_outliner(h, "Collapse All").rect().center().y;
    header - h.get_by_label("Left Dock").get_by_label("Module").rect().center().y
}

/// The seam between the module tree and the Outliner: where the board
/// has it at first (the upper panel 496 points); dragged, kept in the
/// settings; held where the window is too short for it and back as chosen
/// once it is tall again; and where it was after a restart (a new window
/// with the settings kept).
#[test]
fn the_seam_is_kept_across_a_restart_and_a_short_window() {
    let Some((mut h, _)) = outliner_harness("outliner-seam", 0) else { return };
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    assert_eq!(h.state().settings.module_panel_height, None);
    let board = mg_uikit::metrics::MODULE_PANEL;
    assert!((seam(&h) - board).abs() < 1.5, "the board's: {}", seam(&h));
    // Dragged up 120 points.
    let at = egui::pos2(60.0, in_outliner(&h, "Collapse All").rect().center().y - 13.0);
    h.event(egui::Event::PointerMoved(at));
    // (One step for each input event.)
    h.run_steps(1);
    let button = |h: &mut Harness<'_, Moonglow>, pos, pressed| {
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.event(egui::Event::PointerButton { pos, button, pressed, modifiers });
        // (One step for each input event.)
        h.run_steps(1);
    };
    button(&mut h, at, true);
    for k in 1..=6 {
        h.event(egui::Event::PointerMoved(at - egui::vec2(0.0, 20.0 * k as f32)));
        h.run_steps(1);
    }
    button(&mut h, at - egui::vec2(0.0, 120.0), false);
    until(&mut h, "kept", |h| h.state().settings.module_panel_height.is_some());
    let chosen = h.state().settings.module_panel_height.expect("kept") as f32;
    assert!((chosen - (board - 120.0)).abs() < 3.0, "{chosen}");
    assert!((seam(&h) - chosen).abs() < 1.5);
    // A short window: held (the Outliner keeps its least); tall again, as
    // chosen.
    h.set_size(egui::vec2(AREA_HARNESS_WIDTH, 420.0));
    until(&mut h, "held", |h| seam(h) < chosen - 50.0);
    assert_eq!(h.state().settings.module_panel_height, Some(chosen as u32), "kept as chosen");
    h.set_size(egui::vec2(AREA_HARNESS_WIDTH, HEIGHT));
    until(&mut h, "as chosen again", |h| (seam(h) - chosen).abs() < 1.5);
    // A restart: the same settings, the same seam.
    let settings = h.state().settings.clone();
    let Some((mut again, _)) = outliner_harness("outliner-seam-again", 0) else { return };
    again.state_mut().settings = settings;
    until(&mut again, "after a restart", |h| (seam(h) - chosen).abs() < 1.5);
    // Reset Layout: the board's again.
    menu_command(&mut again, "View", "Reset Layout");
    // (Folded by the width again, as designed: unfolded by hand.)
    unfold_left_dock(&mut again);
    until(&mut again, "reset", |h| {
        h.state().settings.module_panel_height.is_none() && (seam(h) - board).abs() < 1.5
    });
}

/// The Outliner's Expand All opens every kind with something placed, and
/// Collapse All closes them; a kind with nothing placed has no arrow.
#[test]
fn expand_all_and_collapse_all() {
    let Some((mut h, _)) = outliner_harness("outliner-fold", 3) else { return };
    let kinds = ["3 placeables", "2 waypoints"];
    assert!(kinds.iter().all(|k| !expanded(&in_outliner(&h, k))), "closed at first");
    in_outliner(&h, "Expand All").click();
    until(&mut h, "opened", |h| kinds.iter().all(|k| expanded(&in_outliner(h, k))));
    assert!(outliner_row(&h, "tag CHEST_02").is_some());
    let empty = in_outliner(&h, "0 creatures");
    assert_eq!(empty.accesskit_node().data().is_expanded(), None, "no arrow");
    in_outliner(&h, "Collapse All").click();
    until(&mut h, "closed", |h| kinds.iter().all(|k| !expanded(&in_outliner(h, k))));
}

/// One group is named "Outliner" (its header's name is apart); its eyes
/// share their names with the area toolbar's Show toggles, each found in
/// its own group, and the kinds' and objects' rows are named apart from
/// the module tree's rows of the same objects beside them.
#[test]
fn the_outliner_s_names_for_a_screen_reader() {
    let Some((mut h, area)) = outliner_harness("outliner-names", 1) else { return };
    assert_eq!(h.query_all_by_label(mg_ui::OUTLINER).count(), 1);
    for kind in ObjectKind::ALL {
        let name = format!("Show {}", kind.label());
        let all: Vec<_> = h.query_all_by_label(&name).collect();
        assert_eq!(all.len(), 2, "{name}: the toolbar's and the eye");
        assert_eq!(all.iter().filter(|n| in_group(n, "Area toolbar")).count(), 1, "{name}");
        assert_eq!(all.iter().filter(|n| in_group(n, mg_ui::OUTLINER)).count(), 1, "{name}");
    }
    // The module tree's rows beside its own: none named alike. (The area
    // opened out in the tree by its arrow, its object selected.)
    h.get_by_label_contains("Placed Objects in ").click();
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    until(&mut h, "both rows", |h| outliner_row(h, "tag CHEST_00").is_some());
    until(&mut h, "the tree's row", |h| {
        h.get_by_label("Module Tree Rows").query_by_label("Placeables (1)").is_some()
    });
    assert_eq!(h.query_all_by_label("Placeables (1)").count(), 1, "the tree's");
    assert_eq!(h.query_all_by_label("1 placeable").count(), 1, "the Outliner's");
}

/// The keys: a click on a row gives them to the Outliner; Down goes to the
/// next row, Enter goes to its object (selected, the camera to it); Delete
/// deletes the object at the keys' row; letters go to a kind by its name,
/// Enter opens it; Escape hands them back. F6 reaches the Outliner after
/// the Left Dock's tab (`right_dock.rs`).
#[test]
fn the_outliner_s_keys() {
    let Some((mut h, area)) = outliner_harness("outliner-keys", 3) else { return };
    let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
    in_outliner(&h, "3 placeables").click();
    until(&mut h, "its rows", |h| outliner_row(h, "tag CHEST_02").is_some());
    outliner_row(&h, "tag CHEST_00").unwrap().click();
    until(&mut h, "the keys", |h| keys(h) == "Outliner");
    // (Away from the view, which takes the keys back while the pointer is
    // in it.)
    h.hover_at(egui::pos2(600.0, 20.0));
    h.key_press(egui::Key::ArrowDown);
    until(&mut h, "the keys' row", |h| {
        outliner_row(h, "tag CHEST_01").is_some_and(|n| selected(&n))
    });
    let camera = looks_at(&h, area);
    h.key_press(egui::Key::Enter);
    until(&mut h, "gone to", |h| selection(h, area) == [(ObjectKind::Placeable, 1)]);
    until(&mut h, "the camera to it", |h| looks_at(h, area) != camera);
    // Delete at the keys' row.
    h.key_press(egui::Key::Delete);
    until(&mut h, "deleted", |h| outliner_row(h, "2 placeables").is_some());
    // Letters: to Waypoints, which Enter opens.
    h.key_press(egui::Key::W);
    // (One step for the key's event: W read before Enter is.)
    h.step();
    h.key_press(egui::Key::Enter);
    until(&mut h, "opened", |h| expanded(&in_outliner(h, "2 waypoints")));
    h.key_press(egui::Key::Escape);
    until(&mut h, "handed back", |h| keys(h) != "Outliner");
}

/// A read-only area (another builder's): its eyes show and hide; the
/// Delete key at its object's row deletes nothing, the log saying why, the
/// object selected still and its row marked.
#[test]
fn a_read_only_area_s_outliner_hides_and_deletes_nothing() {
    let Some((mut h, area)) = read_only_area_harness("outliner-read-only") else { return };
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    until(&mut h, "the Outliner", |h| h.query_by_label(mg_ui::OUTLINER).is_some());
    let shown =
        |h: &Harness<'_, Moonglow>| h.state().area_views[&area].show[ObjectKind::Placeable.index()];
    in_outliner(&h, "Show Placeables").click();
    until(&mut h, "hidden", |h| !shown(h));
    in_outliner(&h, "Show Placeables").click();
    until(&mut h, "shown", |h| shown(h));
    in_outliner(&h, "1 placeable").click();
    until(&mut h, "its row", |h| outliner_row(h, "tag ").is_some());
    outliner_row(&h, "tag ").unwrap().click();
    until(&mut h, "selected", |h| selection(h, area) == [(ObjectKind::Placeable, 0)]);
    h.hover_at(egui::pos2(600.0, 20.0));
    let said = h.state().log.entries.len();
    h.key_press(egui::Key::Delete);
    until(&mut h, "refused", |h| {
        h.state().log.entries.iter().skip(said).any(|e| e.1.contains("read-only"))
    });
    assert!(outliner_row(&h, "1 placeable").is_some(), "nothing deleted");
    assert_eq!(selection(&h, area), [(ObjectKind::Placeable, 0)], "still selected");
    assert!(outliner_row(&h, "tag ").is_some_and(|n| selected(&n)), "its row marked");
}

/// The Outliner stays where it is under a click on its rows: a row cut by
/// its foot, clicked, does not move from under the pointer (a double
/// click's second click lands on it); a Ctrl+click on a row with another
/// selected out of sight does not scroll to that one. A selection made in
/// the view is what it brings into sight.
#[test]
fn a_click_on_a_row_leaves_the_outliner_still() {
    let Some((mut h, area)) = outliner_harness("outliner-still", 40) else { return };
    in_outliner(&h, "40 placeables").click();
    until(&mut h, "its rows", |h| outliner_row(h, "tag CHEST_00").is_some());
    rest(&mut h);
    // A row cut by the Outliner's foot, its middle in sight (the rows
    // scrolled by the wheel so far as to make one so).
    let foot = HEIGHT - 24.0;
    let spanning = |h: &Harness<'_, Moonglow>| {
        (h.get_by_label(mg_ui::OUTLINER).query_all_by_label_contains("tag CHEST_"))
            .find(|n| n.rect().top() < foot - 1.0 && n.rect().bottom() > foot + 1.0)
            .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
    };
    let by = match spanning(&h) {
        Some((_, r)) => (r.center().y - foot + 6.0).max(0.0),
        None => 8.0,
    };
    if by > 0.0 {
        let over = outliner_row(&h, "tag CHEST_00").unwrap().rect().center();
        h.hover_at(over);
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -by),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        rest(&mut h);
    }
    let (label, rect) = spanning(&h).expect("a row cut by the foot");
    assert!(rect.center().y < foot, "its middle in sight: {rect:?}");
    let tag = label.split("tag ").nth(1).unwrap().to_string();
    let i: usize = tag.trim_start_matches("CHEST_").parse().unwrap();
    outliner_row(&h, &format!("tag {tag}")).unwrap().click();
    until(&mut h, "selected", |h| selection(h, area) == [(ObjectKind::Placeable, i)]);
    rest(&mut h);
    let now = outliner_row(&h, &format!("tag {tag}")).unwrap().rect();
    assert_eq!(now, rect, "the row stays under the pointer");
    // CHEST_39 selected in the view: brought into sight.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 39)];
    until(&mut h, "39 in sight", |h| outliner_row(h, "tag CHEST_39").is_some());
    rest(&mut h);
    // Scrolled back to the top by Home over the rows, 39 out of sight;
    // Ctrl+click on CHEST_02: no scroll to 39, the last of those chosen.
    let row = outliner_row(&h, "tag CHEST_35").unwrap().rect();
    h.hover_at(row.center());
    h.key_press(egui::Key::Home);
    until(&mut h, "at the top", |h| outliner_row(h, "tag CHEST_00").is_some());
    rest(&mut h);
    assert!(outliner_row(&h, "tag CHEST_39").is_none(), "39 out of sight");
    let two = outliner_row(&h, "tag CHEST_02").unwrap().rect();
    outliner_row(&h, "tag CHEST_02").unwrap().click_modifiers(egui::Modifiers::COMMAND);
    let p = |i: usize| (ObjectKind::Placeable, i);
    until(&mut h, "both chosen", |h| selection(h, area) == [p(2), p(39)]);
    rest(&mut h);
    assert_eq!(outliner_row(&h, "tag CHEST_02").map(|n| n.rect()), Some(two), "no scroll");
}

/// How many chests `area` holds (its GIT as the workspace has it).
fn chests_in(h: &mut Harness<'_, Moonglow>, area: ResRef) -> usize {
    let git = ResKey::new(area, ResType::GIT);
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&git).unwrap().root.list("Placeable List").map_or(0, |l| l.len())
}

/// Two areas open: nothing the Outliner does acts on a row of the area
/// it listed before. A row of A clicked (the keys' row, the anchor), B
/// brought in front with nothing selected: Delete deletes nothing (the
/// keys at B's first row, a kind's, which Enter opens), and a Shift+click
/// in B chooses from nothing of A's. Docked and As Floating Windows.
#[test]
fn nothing_acts_on_a_row_of_another_area() {
    for mode in MODES {
        let Some((mut h, a, b)) = outliner_app_with(&mode.dir("outliner-two-areas"), 3, mode, true)
        else {
            return;
        };
        let b = b.unwrap();
        let p = |i: usize| (ObjectKind::Placeable, i);
        in_outliner(&h, "3 placeables").click();
        until(&mut h, "A's rows", |h| outliner_row(h, "tag CHEST_02").is_some());
        outliner_row(&h, "tag CHEST_01").unwrap().click();
        until(&mut h, "A's CHEST_01", |h| selection(h, a) == [p(1)]);
        let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
        until(&mut h, "the keys", |h| keys(h) == "Outliner");
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(b)));
        until(&mut h, "B in front", |h| {
            h.state().active_editor() == Some(&Tab::Area(b))
                && h.state().area_views.contains_key(&b)
        });
        until(&mut h, "B listed", |h| {
            h.query_by_label(mg_ui::OUTLINER).is_some() && outliner_row(h, "0 waypoints").is_some()
        });
        assert!(selection(&h, b).is_empty(), "{mode:?}: nothing selected in B");
        h.hover_at(egui::pos2(600.0, 20.0));
        for _ in 0..4 {
            if keys(&h) == "Outliner" {
                break;
            }
            h.key_press(egui::Key::F6);
            // (One step for the key's event.)
            h.step();
        }
        assert_eq!(keys(&h), "Outliner", "{mode:?}");
        h.key_press(egui::Key::Delete);
        // (One step for the key's event: Delete read before Enter.)
        h.step();
        h.key_press(egui::Key::Enter);
        until(&mut h, "B's kind opened by Enter", |h| expanded(&in_outliner(h, "3 placeables")));
        assert_eq!(chests_in(&mut h, a), 3, "{mode:?}: A as it was");
        assert_eq!(chests_in(&mut h, b), 3, "{mode:?}: B as it was");
        assert_eq!(selection(&h, a), [p(1)], "{mode:?}: A's selection as it was");
        // A Shift+click in B: from nothing of A's (the anchor A's CHEST_01).
        until(&mut h, "B's rows", |h| outliner_row(h, "tag CHEST_02").is_some());
        outliner_row(&h, "tag CHEST_02").unwrap().click_modifiers(egui::Modifiers::SHIFT);
        until(&mut h, "B's CHEST_02 alone", |h| selection(h, b) == [p(2)]);
    }
}

/// The Shift+click anchor follows what is selected: (b) a row clicked and
/// deleted, Shift+click chooses from nothing (the places after it
/// shifted: the same place is another object); (a) a row clicked, another
/// selected in the view, Shift+click goes from the one selected in the
/// view.
#[test]
fn the_shift_anchor_follows_the_selection_and_a_delete() {
    let Some((mut h, area)) = outliner_harness("outliner-anchor", 6) else { return };
    let p = |i: usize| (ObjectKind::Placeable, i);
    in_outliner(&h, "6 placeables").click();
    until(&mut h, "its rows", |h| outliner_row(h, "tag CHEST_05").is_some());
    // (b) CHEST_03 clicked and deleted (its key, the keys the Outliner's).
    outliner_row(&h, "tag CHEST_03").unwrap().click();
    until(&mut h, "CHEST_03", |h| selection(h, area) == [p(3)]);
    h.hover_at(egui::pos2(600.0, 20.0));
    h.key_press(egui::Key::Delete);
    until(&mut h, "deleted", |h| outliner_row(h, "5 placeables").is_some());
    outliner_row(&h, "tag CHEST_00").unwrap().click_modifiers(egui::Modifiers::SHIFT);
    until(&mut h, "CHEST_00 alone", |h| selection(h, area) == [p(0)]);
    // (a) CHEST_01 clicked; CHEST_04 (now in place 3) selected in the view
    // (as a click there selects it).
    outliner_row(&h, "tag CHEST_01").unwrap().click();
    until(&mut h, "CHEST_01", |h| selection(h, area) == [p(1)]);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![p(3)];
    until(&mut h, "CHEST_04 marked", |h| {
        outliner_row(h, "tag CHEST_04").is_some_and(|n| selected(&n))
    });
    outliner_row(&h, "tag CHEST_05").unwrap().click_modifiers(egui::Modifiers::SHIFT);
    until(&mut h, "from CHEST_04", |h| selection(h, area) == [p(3), p(4)]);
}

/// With the Outliner shown, an object selected in the view is followed by
/// the Outliner (its kind opened, its row in sight and marked) and not by
/// the module tree, which stays where it was: its area not opened out,
/// its rows where they were (the owner's ruling of 2026-11-14). Without
/// the Outliner the tree follows, as before
/// (`the_module_tree_shows_the_object_selected_in_the_area`).
#[test]
fn with_the_outliner_shown_the_tree_stays_where_it_was() {
    let Some((mut h, area)) = outliner_harness("outliner-tree-still", 3) else { return };
    let tree_rows = |h: &Harness<'_, Moonglow>| -> Vec<(String, egui::Rect)> {
        let tree = h.get_by_label("Module Tree Rows");
        (tree.query_all_by(|n| n.label().is_some()))
            .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
            .collect()
    };
    let before = tree_rows(&h);
    assert!(h.query_by_label("Placeables (3)").is_none(), "the tree's area closed");
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 2)];
    until(&mut h, "the Outliner's row marked", |h| {
        outliner_row(h, "tag CHEST_02").is_some_and(|n| selected(&n))
    });
    rest(&mut h);
    assert!(h.query_by_label("Placeables (3)").is_none(), "the tree's area still closed");
    assert_eq!(tree_rows(&h), before, "the tree as it was");
    assert_eq!(h.state().tree_object, Some((area, ObjectKind::Placeable, 2)), "its mark kept");
}

/// The owner's ruling (T13's review: while the Outliner shows, the tree
/// stops following the selection) where a narrow window folds the Left
/// Dock (M3 T14): "the Outliner shown" is the dock as drawn, not the
/// builder's `hide_tree`. Folded by a 1280 window, neither the tree nor the
/// Outliner is drawn, and the tree moves for nothing; the builder's own
/// fold on and the dock unfolded by hand there, the Outliner is shown and
/// the tree stays where it was as the view's selection changes, as it
/// does with the dock unfolded at any width.
#[test]
fn a_dock_unfolded_by_hand_in_a_narrow_window_keeps_the_tree_still() {
    let Some((mut h, area)) = outliner_harness("outliner-narrow", 3) else { return };
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    // (By way of 1600, where the dock unfolded by hand at the harness's
    // width is the builder's own fold again.)
    wide_window(&mut h);
    h.state_mut().settings.hide_tree = true;
    h.set_size(egui::vec2(1280.0, HEIGHT));
    // (Folded, the strip's button of the Outliner's name stands for it;
    // the Outliner itself is drawn where its Collapse All is.)
    let outliner_shown = |h: &Harness<'_, Moonglow>| {
        h.query_by_label("Show the Left Dock").is_none()
            && (h.query_by_label(mg_ui::OUTLINER))
                .is_some_and(|o| o.query_by_label("Collapse All").is_some())
    };
    until(&mut h, "the Left Dock folded", |h| h.query_by_label("Show the Left Dock").is_some());
    assert!(!outliner_shown(&h));
    h.get_by_label("Show the Left Dock").click();
    until(&mut h, "the Outliner shown", outliner_shown);
    rest(&mut h);
    assert!(h.state().settings.hide_tree, "the builder's fold kept");
    let tree_rows = |h: &Harness<'_, Moonglow>| -> Vec<(String, egui::Rect)> {
        let tree = h.get_by_label("Module Tree Rows");
        (tree.query_all_by(|n| n.label().is_some()))
            .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
            .collect()
    };
    let before = tree_rows(&h);
    assert!(h.query_by_label("Placeables (3)").is_none(), "the tree's area closed");
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 2)];
    until(&mut h, "the Outliner's row marked", |h| {
        outliner_row(h, "tag CHEST_02").is_some_and(|n| selected(&n))
    });
    rest(&mut h);
    assert!(h.query_by_label("Placeables (3)").is_none(), "the tree's area still closed");
    assert_eq!(tree_rows(&h), before, "the tree as it was");
}

/// In the least window (420 by 320) the Outliner folds to its header (its
/// rows out, Expand All and Collapse All still there) before the panel
/// over it goes under its least (180 points: the tab bar, the Filter and
/// five rows); in a taller window its rows come back.
#[test]
fn the_outliner_folds_to_its_header_in_the_least_window() {
    let Some((mut h, _)) = outliner_harness("outliner-least", 3) else { return };
    h.set_size(egui::vec2(420.0, 320.0));
    until(&mut h, "folded to its header", |h| {
        h.query_by_label(mg_ui::OUTLINER).is_some() && outliner_row(h, "3 placeables").is_none()
    });
    in_outliner(&h, "Collapse All");
    assert!(seam(&h) >= 180.0 - 0.5, "the panel over it at its least or more: {}", seam(&h));
    let bar = in_outliner(&h, "Collapse All").rect();
    assert!(bar.bottom() <= 320.0 - 24.0 + 0.5, "its header in the window: {bar:?}");
    h.set_size(egui::vec2(AREA_HARNESS_WIDTH, HEIGHT));
    until(&mut h, "its rows back", |h| outliner_row(h, "3 placeables").is_some());
}

/// Tab goes through the Outliner by named stops only (its header's
/// buttons, each kind's row and eye, an object's row), in order; Space
/// and Enter on a focused eye hide and show its kind.
#[test]
fn tab_goes_through_the_outliner_by_named_stops() {
    let Some((mut h, area)) = outliner_harness("outliner-tab", 2) else { return };
    in_outliner(&h, "Expand All").focus();
    until(&mut h, "Expand All focused", |h| in_outliner(h, "Expand All").is_focused());
    let focused = |h: &Harness<'_, Moonglow>| {
        h.query_by(|n| n.is_focused()).map(|n| {
            let inside = in_group(&n, mg_ui::OUTLINER);
            (n.accesskit_node().label().unwrap_or_default(), inside)
        })
    };
    let mut stops = Vec::new();
    for _ in 0..40 {
        h.key_press(egui::Key::Tab);
        // (One step for the key's event.)
        h.step();
        let Some((label, inside)) = focused(&h) else { break };
        if !inside {
            break;
        }
        assert!(!label.is_empty(), "a stop with no name after {stops:?}");
        stops.push(label);
    }
    // (A kind with nothing placed: its eye alone, its row no stop.)
    let order = [
        "Collapse All",
        "Show Creatures",
        "Show Doors",
        "Show Encounters",
        "Show Items",
        "2 placeables",
        "Show Placeables",
        "Show Sounds",
        "Show Stores",
        "Show Triggers",
        "2 waypoints",
        "Show Waypoints",
    ];
    assert_eq!(stops, order, "every stop named, in order");
    // Space and Enter on the focused eye.
    let shown =
        |h: &Harness<'_, Moonglow>| h.state().area_views[&area].show[ObjectKind::Placeable.index()];
    in_outliner(&h, "Show Placeables").focus();
    until(&mut h, "the eye focused", |h| in_outliner(h, "Show Placeables").is_focused());
    h.key_press(egui::Key::Space);
    until(&mut h, "hidden by Space", |h| !shown(h));
    h.key_press(egui::Key::Enter);
    until(&mut h, "shown by Enter", |h| shown(h));
}

/// The module tree's open groups, kept by egui from run to run, are as
/// they were across two restarts with the area in front (the tree's
/// widgets keep their ids: the Outliner's split changed them once, from
/// the version before it, and never again). A restart: a new window given
/// the memory egui kept.
#[test]
fn the_tree_s_open_groups_are_kept_across_two_restarts() {
    let Some((mut h, _)) = outliner_harness("outliner-restart-0", 0) else { return };
    let open = |h: &Harness<'_, Moonglow>| {
        let row = h.get_by_label("Module Tree Rows").get_by_label("Areas (1)");
        expanded(&row)
    };
    assert!(open(&h), "open at first");
    h.get_by_label("Module Tree Rows").get_by_label("Areas (1)").click();
    until(&mut h, "closed", |h| !open(h));
    for run in 1..=2 {
        let kept = h.ctx.memory(|m| m.clone());
        let Some((again, _)) = outliner_harness(&format!("outliner-restart-{run}"), 0) else {
            return;
        };
        h = again;
        h.ctx.memory_mut(|m| *m = kept);
        // (One step drawn with the memory given.)
        h.step();
        until(&mut h, "drawn", |h| h.query_by_label("Module Tree Rows").is_some());
        assert!(!open(&h), "restart {run}: still closed");
    }
}
