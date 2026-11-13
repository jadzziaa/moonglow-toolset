//! The Status Bar's hints and counts (the UI redesign's M3, T12): the
//! area view's states as key caps and a word each, in place of the line
//! that stood under its toolbar; the keys as the user bound them; the
//! counts following the active editor; the Welcome page's keys. (Esc and
//! Enter for a docked tab with OK and Cancel: `open_editors.rs`; the bar's
//! room: `status_bar.rs`.)

use crate::common::*;
use mg_ui::keys::Cmd;

/// What the Status Bar says: each hint as a screen reader says it, then
/// each count.
fn said(h: &Harness<'_, Moonglow>) -> Vec<String> {
    h.state().status_said().to_vec()
}

/// Frames until the Status Bar says each of `what` (a hint or a count)
/// and none of `not`, for ten seconds at most; then each said hint is a
/// node of the bar's "Hints" group, named so (a screen reader reaches it,
/// and it takes no focus).
fn says(h: &mut Harness<'_, Moonglow>, what: &[&str], not: &[&str]) {
    let started = std::time::Instant::now();
    loop {
        let now = said(h);
        let has = |w: &&str| now.iter().any(|s| s == w);
        if what.iter().all(has) && !not.iter().any(has) {
            break;
        }
        assert!(
            started.elapsed().as_secs() < 10,
            "never said {what:?} without {not:?}: it says {now:?}"
        );
        h.step();
    }
    for w in what {
        let in_hints = h.get_by_label("Status Bar").query_by_label(w);
        assert!(in_hints.is_some(), "{w} in the Status Bar");
    }
}

/// The view of `area`.
fn view<'a>(h: &'a mut Harness<'_, Moonglow>, area: ResRef) -> &'a mut mg_ui::area_view::AreaView {
    h.state_mut().area_views.get_mut(&area).unwrap()
}

/// [`says`] for a hint whose words begin with `start` (a blueprint's or
/// a light's name after it).
fn says_starting(h: &mut Harness<'_, Moonglow>, start: &str) {
    let started = std::time::Instant::now();
    while !said(h).iter().any(|s| s.starts_with(start)) {
        assert!(started.elapsed().as_secs() < 10, "never said {start}…: {:?}", said(h));
        h.step();
    }
}

/// The area view's states, each with its hints in the Status Bar and the
/// line under the toolbar gone: nothing selected; one waypoint selected
/// (its count "1 selected", its tag and place in its tip); both; Shift
/// held over one (the axes, no Tilt or Scale: a waypoint's model takes
/// no visual transform); a drag under way with the grid's snap; a
/// blueprint, the start location and a light being placed; the placeable
/// brush with nothing chosen; Select Tiles with four tiles selected; a
/// tile brush. The area's counts at the bar's right throughout.
#[test]
fn each_area_state_shows_its_hints() {
    let Some((mut h, area)) = area_harness("status-hints") else { return };
    h.state_mut().settings.snap_grid = Some(50);
    settle(&mut h);
    let counts = ["4 by 4 tiles", "2 objects"];
    let idle = ["Click: Select", "Drag: Move or box", "Ctrl, Drag: Move view"];
    says(&mut h, &[&idle[..], &counts[..]].concat(), &[]);
    says(&mut h, &["Ctrl, Right Drag: Turn view"], &["1 selected"]);
    assert!(h.query_by_label_contains("Click to select").is_none(), "no line under the toolbar");
    let wp = |i| (mg_area::ObjectKind::Waypoint, i);
    // One selected: the Main board's.
    view(&mut h, area).selection = vec![wp(0)];
    let selected = [
        "Drag: Move",
        "Turning ring: Turn",
        "Shift: Handles",
        "Alt, Drag: Raise",
        "Q, E: Turn by a step",
        "G: Drop to ground",
        "1 selected",
    ];
    says(&mut h, &selected, &["Click: Select"]);
    let one = h.get_by_label("Status Bar").get_by_label("1 selected").rect();
    let tip = tip_at(&mut h, one).unwrap_or_default();
    assert!(tip.starts_with("Waypoint ") && tip.contains("at 20.00, 20.00"), "{tip}");
    view(&mut h, area).selection = vec![wp(0), wp(1)];
    says(&mut h, &["2 selected", "Drag: Move"], &["1 selected"]);
    // Shift held: the handles' hints.
    view(&mut h, area).selection = vec![wp(0)];
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    let shift = ["Shift: Held", "Axis arrows: Move along an axis", "Right Drag: Turn view"];
    says(&mut h, &shift, &["Tilt rings: Tilt", "Scale handle: Scale", "Drag: Move"]);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    says(&mut h, &["Drag: Move"], &["Shift: Held"]);
    // A drag of the waypoint under way: Esc, the view, the snap.
    let at = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(at);
    h.step();
    press(&h, at, true, egui::Modifiers::NONE);
    // (One step a move: a drag starts past egui's threshold.)
    for k in 1..=6 {
        h.hover_at(at + egui::vec2(8.0 * k as f32, 0.0));
        h.step();
    }
    says(&mut h, &["Esc: Cancel", "Middle Drag: Turn view", "Snap 0.5 m"], &["Drag: Move"]);
    h.key_press(egui::Key::Escape);
    h.step();
    press(&h, at + egui::vec2(48.0, 0.0), false, egui::Modifiers::NONE);
    settle(&mut h);
    // Placing: a blueprint, the start location, a light.
    view(&mut h, area).selection.clear();
    let blueprint = mg_resman::ResKey::parse("nw_waypoint001", ResType::UTW).unwrap();
    h.state_mut().palette.selected = Some(blueprint);
    says(&mut h, &["Placing nw_waypoint001", "Click: Place", "Right Click or Esc: Stop"], &[]);
    says(&mut h, &["Q or E: Turn", "Page Up or Page Down: Onto the next surface up or down"], &[]);
    h.state_mut().palette.selected = None;
    h.state_mut().palette.placing_start = true;
    says(&mut h, &["Placing the start location", "Click: Put it here, facing the view"], &[]);
    h.state_mut().palette.placing_start = false;
    h.state_mut().palette.placing_light = Some(0);
    says_starting(&mut h, "Placing a light (");
    h.state_mut().palette.placing_light = None;
    // The placeable brush, nothing chosen.
    h.state_mut().paint = true;
    let paint = [
        "Paint: choose placeables in the palette",
        "Ctrl, Click: Choose several in the palette",
        "Esc: Stop",
    ];
    says(&mut h, &paint, &[]);
    h.state_mut().paint = false;
    // Select Tiles, four tiles selected (the AreaTiles board's).
    view(&mut h, area).tile_mode = true;
    view(&mut h, area).tile_selection = vec![(0, 0), (1, 0), (0, 1), (1, 1)];
    let tiles = [
        "Click: Select a tile",
        "Drag: Box",
        "Shift, Click: Add",
        "Ctrl, Click: Add or take out",
        "Right Click: Tile menu",
        "Shift, Right Click: Next tile",
        "2 by 2 tiles selected",
    ];
    says(&mut h, &tiles, &[]);
    view(&mut h, area).tile_selection = vec![(0, 0), (1, 1)];
    says(&mut h, &["2 tiles selected"], &["2 by 2 tiles selected"]);
    view(&mut h, area).tile_mode = false;
    view(&mut h, area).tile_selection.clear();
    // A tile brush.
    h.state_mut().palette.tile_brush = Some(mg_ui::terrain_mode::TileBrush {
        tileset: ResRef::from_str("ttr01").unwrap(),
        label: "Raise/Lower".into(),
        brush: mg_area::terrain::Brush::RaiseLower,
        target: None,
        exact: false,
    });
    says(&mut h, &["In hand: Raise/Lower", "Click: Raise", "Right Click: Lower", "Esc: Stop"], &[]);
    h.state_mut().palette.tile_brush = None;
    says(&mut h, &idle, &["Esc: Stop"]);
    says(&mut h, &counts, &[]);
}

/// A read-only area (another builder's): selecting and the camera, no
/// hint of moving, turning or raising, with or without a selection.
#[test]
fn a_read_only_area_hints_no_change() {
    let Some((mut h, area)) = read_only_area_harness("status-hints-read-only") else { return };
    settle(&mut h);
    let changes = ["Drag: Move or box", "Drag: Move", "Alt, Drag: Raise", "G: Drop to ground"];
    says(&mut h, &["Click: Select", "Ctrl, Drag: Move view"], &changes);
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    says(&mut h, &["Click: Select", "1 selected"], &changes);
}

/// The keys shown are the user's: a command given another key shows it,
/// one of a pair given none leaves the other, and a command with no key
/// leaves its hint out.
#[test]
fn a_rebound_key_shows_as_bound_and_an_unbound_one_drops_its_hint() {
    let Some((mut h, area)) = area_harness("status-hints-keys") else { return };
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    says(&mut h, &["G: Drop to ground", "Q, E: Turn by a step"], &[]);
    let shift_d = egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::D);
    h.state_mut().keymap.set(Cmd::DropToGround, vec![shift_d]);
    h.state_mut().keymap.set(Cmd::TurnLeft, Vec::new());
    says(&mut h, &["Shift, D: Drop to ground", "E: Turn by a step"], &["G: Drop to ground"]);
    h.state_mut().keymap.set(Cmd::DropToGround, Vec::new());
    h.state_mut().keymap.set(Cmd::TurnRight, Vec::new());
    says(&mut h, &["Drag: Move"], &["Shift, D: Drop to ground", "E: Turn by a step"]);
    assert!(!said(&h).iter().any(|s| s.ends_with("Drop to ground") || s.contains("by a step")));
}

/// The Welcome page's hints: New Module, Open Module and Quick Open by
/// their keys, as bound; "No module open" at the right, no counts.
#[test]
fn the_welcome_page_shows_its_keys_as_bound() {
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    let mut h = harness(app, 1400.0, 700.0);
    settle(&mut h);
    let welcome =
        ["Ctrl, N: New module", "Ctrl, O: Open module", "Ctrl, P: Go to resource or command"];
    says(&mut h, &welcome, &[]);
    assert_eq!(said(&h), welcome, "nothing else");
    let ctrl_shift_o = egui::KeyboardShortcut::new(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::O,
    );
    h.state_mut().keymap.set(Cmd::QuickOpen, vec![ctrl_shift_o]);
    h.state_mut().keymap.set(Cmd::NewModule, Vec::new());
    says(&mut h, &["Ctrl, Shift, O: Go to resource or command"], &["Ctrl, N: New module"]);
}

/// The counts follow the active docked editor, as the Right Dock does:
/// an area's tiles and objects while its tab is in front of the focused
/// group; none while Module Properties is (a tab in As Tabs); the area's
/// still under a floating window (As Floating Windows: Module Properties
/// a window, the area the active editor).
#[test]
fn the_counts_follow_the_active_editor() {
    counts_follow(Mode::Floating);
}

#[test]
fn the_counts_follow_the_active_editor_as_tabs() {
    counts_follow(Mode::Tabs);
}

fn counts_follow(mode: Mode) {
    let Some((mut app, rs, area)) =
        area_app(&mode.dir("status-counts"), "ttr01", None, 4, None, &|_, _, _| {})
    else {
        return;
    };
    put_in(&mut app, mode);
    app.settings.window_places = [("module-properties".to_string(), "window".to_string())]
        .into_iter()
        .filter(|_| !mode.tabs())
        .collect();
    let mut h = area_gpu_harness(app, rs, AREA_HARNESS_WIDTH, 800.0);
    settle(&mut h);
    let counts = ["4 by 4 tiles", "2 objects"];
    says(&mut h, &counts, &[]);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    if mode.tabs() {
        assert_eq!(h.state().active_editor(), Some(&Tab::ModuleProperties));
        says(&mut h, &["Tab: Next field"], &counts);
    } else {
        assert_eq!(h.state().active_editor(), Some(&Tab::Area(area)), "{mode:?}");
        says(&mut h, &counts, &[]);
    }
    click_tab(&mut h, &Tab::Area(area));
    settle(&mut h);
    says(&mut h, &counts, &["Tab: Next field"]);
}

/// The hints' cost, by hand (`cargo test -p mg-ui --release --test app
/// hints_cost -- --ignored --nocapture`): the median time of a frame of
/// the whole window with the fullest set of hints an area shows (Select
/// Tiles' seven and a refusal's notice, with its counts) against the
/// same window with none, in turns, 400 frames each; the hints are laid
/// out every frame (they follow the pointer and the keys held).
#[test]
#[ignore]
fn hints_cost() {
    use mg_uikit::status::Hint;
    let fullest = || {
        let hints = vec![
            Hint::note("No tile fits the tiles shown in red: the terrain refuses it"),
            Hint::keys(["Click"], "Select a tile"),
            Hint::keys(["Drag"], "Box"),
            Hint::keys(["Shift", "Click"], "Add"),
            Hint::keys(["Ctrl", "Click"], "Add or take out"),
            Hint::keys(["Right Click"], "Tile menu"),
            Hint::keys(["Shift", "Right Click"], "Next tile"),
            Hint::keys(["Shift", "Drag"], "Add a box"),
        ];
        let counts = ["2 by 2 tiles selected", "11 by 11 tiles", "192 objects"];
        Some((hints, counts.map(String::from).to_vec()))
    };
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let mut times: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for round in 0..20 {
        for (i, shown) in [fullest(), Some((Vec::new(), Vec::new()))].into_iter().enumerate() {
            h.state_mut().status_for_tests(shown);
            h.step();
            for _ in 0..20 {
                let started = std::time::Instant::now();
                h.step();
                times[i].push(started.elapsed().as_secs_f64() * 1000.0);
            }
        }
        let _ = round;
    }
    let median = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let (with, without) = (median(&mut times[0]), median(&mut times[1]));
    println!(
        "a frame with the fullest hints {with:.3} ms, with none {without:.3} ms: the hints {:.3} ms",
        with - without
    );
}

/// The Resources panel in use shows its hints (the Resources board's):
/// with the pointer over it, Double Click Open, Right Click More and
/// Find's key; a row clicked (the list has the keys), Up or Down and
/// Enter too, kept with the pointer away (no Find then: its key goes to
/// the Name field only with the pointer over the panel); the keys given
/// back (Escape), the editor's hints again.
#[test]
fn the_resources_panel_in_use_shows_its_hints() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-status-hints-resources");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&path);
    show_resources(&mut app);
    app.browser.filter = "nw_it_torch00".into();
    let mut h = harness(app, 1280.0, 800.0);
    settle(&mut h);
    let row = h.get_by_label("nw_it_torch001.uti").rect().center();
    h.hover_at(row);
    h.step();
    let panel = ["Double Click: Open", "Right Click: More", "Ctrl, F: Find"];
    says(&mut h, &panel, &["Enter: Open"]);
    h.get_by_label("nw_it_torch001.uti").click();
    says(&mut h, &["Up or Down: Move in the list", "Enter: Open", "Ctrl, F: Find"], &[]);
    let away = h.state().editor_area_rect().unwrap().center();
    h.hover_at(away);
    h.step();
    says(&mut h, &["Up or Down: Move in the list", "Double Click: Open"], &["Ctrl, F: Find"]);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    says(&mut h, &[], &["Double Click: Open", "Enter: Open"]);
}

/// A script's tab in front (As Tabs) shows the Script board's keys, as
/// bound; F7's Compile all with them.
#[test]
fn a_script_tab_shows_its_keys() {
    let dir = mg_testkit::scratch_dir("ui-status-hints-script");
    let path = sample_module(&dir);
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let script = Tab::Script(mg_resman::ResKey::parse("hello", ResType::NSS).unwrap());
    app.actions.push(mg_ui::Action::OpenTab(script.clone()));
    let mut h = harness(app, 1600.0, 800.0);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&script));
    let keys = [
        "Ctrl, F: Find",
        "F3: Find next",
        "F12: Definition",
        "Shift, F12: References",
        "F2: Complete",
        "F5: Bookmark",
        "F7: Compile all",
        "Ctrl, R: Replace",
    ];
    says(&mut h, &keys, &["4 by 4 tiles"]);
}
