//! The Left Dock's Module panel (the UI redesign's shell, M3's T3): its
//! header's buttons, its Filter, each group's row with its icon and its
//! count, and the Areas group that scrolls in a well of its own (ruling
//! 10) while the groups under it stay in sight and in place, with the
//! tree's keys, menus, drags and following the tab chosen or the object
//! selected working across the well. Most open the game's Chapter1 (113
//! areas) and skip without the game, as their neighbors do.

use mg_ui::TreeAt;

use crate::common::*;

/// Chapter1 open in a window `width` by `height`, its tree as it opens
/// (`None` without the game).
fn chapter1(width: f32, height: f32) -> Option<Harness<'static, Moonglow>> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = harness(app, width, height);
    settle(&mut h);
    Some(h)
}

/// The module's areas in the tree's order (by ResRef).
fn areas(h: &Harness<'_, Moonglow>) -> Vec<ResRef> {
    areas_of(h.state())
}

/// `app`'s module's areas in the tree's order (by ResRef).
fn areas_of(app: &Moonglow) -> Vec<ResRef> {
    let ws = app.ws.as_ref().unwrap();
    let mut all: Vec<ResRef> = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
    all.sort();
    all
}

/// The row of the tree's group `name` ("Conversations"), whatever its
/// count.
fn group<'t>(h: &'t Harness<'_, Moonglow>, name: &str) -> egui_kittest::Node<'t> {
    let prefix = format!("{name} (");
    let tree = h.get_by_label("Module Tree Rows");
    let found =
        tree.query_all_by(move |n| n.label().is_some_and(|l| l.starts_with(&prefix))).next();
    found.unwrap_or_else(|| panic!("the group {name}"))
}

/// The groups under Areas, by name.
const BELOW: [&str; 13] = [
    "Conversations",
    "Scripts",
    "NUI windows",
    "Creatures",
    "Doors",
    "Encounters",
    "Items",
    "Placeables",
    "Sounds",
    "Stores",
    "Triggers",
    "Waypoints",
    "Journal and factions",
];

/// Where each group's row is.
fn groups_at(h: &Harness<'_, Moonglow>) -> Vec<egui::Rect> {
    BELOW.iter().map(|g| group(h, g).rect()).collect()
}

/// The well's bounds: under the Areas group's row, over the next group's.
fn well(h: &Harness<'_, Moonglow>) -> std::ops::RangeInclusive<f32> {
    group(h, "Areas").rect().bottom()..=group(h, "Conversations").rect().top()
}

/// Whether the row `label` of the tree is laid out wholly inside the well.
fn in_well(h: &Harness<'_, Moonglow>, label: &str) -> bool {
    let Some(row) = h.get_by_label("Module Tree Rows").query_by_label(label) else { return false };
    let (r, w) = (row.rect(), well(h));
    w.contains(&(r.top() + 0.5)) && w.contains(&(r.bottom() - 0.5))
}

/// Wheels `dy` points over `at`, a frame or two each.
fn wheel(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, dy: f32, times: usize) {
    h.hover_at(at);
    for _ in 0..times {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, dy),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(2);
    }
    settle(h);
}

/// With 113 areas in an 800 px high window, the Areas group scrolls in
/// its own well: wheeled over it, its rows move and the groups under it
/// stay where they were; every group's row is in sight above the Status
/// Bar, each with its count right-aligned (the number of its kind in the
/// module, in its accessible name too).
#[test]
fn the_areas_group_scrolls_in_its_own_well_and_the_groups_below_stay() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    assert_eq!(all.len(), 113);
    group(&h, "Areas");
    h.get_by_label("Module Tree Rows").get_by_label("Areas (113)");
    // Every group in sight, over the Status Bar.
    let bottom = h.ctx.content_rect().bottom() - mg_uikit::metrics::STATUS_BAR;
    for (name, r) in BELOW.iter().zip(groups_at(&h)) {
        assert!(r.bottom() <= bottom, "{name} in sight in 800 px: {r:?}");
    }
    // The well holds rows (at least the board's five), the first area's
    // at its top, not the last's.
    let w = well(&h);
    assert!(w.end() - w.start() >= 5.0 * mg_uikit::metrics::ROW_HEIGHT - 0.5, "{w:?}");
    let (first, last) = (all[0].to_string(), all[all.len() - 1].to_string());
    assert!(in_well(&h, &first));
    assert!(!in_well(&h, &last), "the last area is further down the well");
    // Each group's count: its kind's number, drawn against the row's
    // right edge (6 px in), in its name for a screen reader.
    let ws = h.state().ws.as_ref().unwrap();
    let count = |types: &[ResType]| ws.module.keys().filter(|k| types.contains(&k.restype)).count();
    let drawn = drawn_shapes_text(&h);
    for (name, types) in [
        ("Conversations", &[ResType::DLG][..]),
        ("Scripts", &[ResType::NSS]),
        ("Creatures", &[ResType::UTC]),
        ("Waypoints", &[ResType::UTW]),
    ] {
        let n = count(types);
        let row = h.get_by_label("Module Tree Rows").get_by_label(&format!("{name} ({n})")).rect();
        let at = (drawn.iter())
            .find(|(t, r)| *t == n.to_string() && row.contains(r.center()))
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("{name}'s count drawn in its row"));
        let from_right = row.right() - at.right();
        assert!((from_right - mg_uikit::tree::ROW_PADDING_RIGHT).abs() < 1.5, "{name}: {at:?}");
    }
    // Wheeled over the well: its rows move, the groups stay.
    let before = groups_at(&h);
    let areas_row = group(&h, "Areas").rect();
    let first_at = h.get_by_label(&first).rect();
    wheel(&mut h, first_at.center(), -200.0, 3);
    assert!(!in_well(&h, &first), "the first area scrolled away");
    assert_eq!(groups_at(&h), before, "the groups below stay in place");
    assert_eq!(group(&h, "Areas").rect(), areas_row);
    // To the well's foot: the last area, and still the groups in place.
    wheel(&mut h, first_at.center(), -400.0, 12);
    assert!(in_well(&h, &last), "the last area at the well's foot");
    assert_eq!(groups_at(&h), before);
}

/// The words drawn in the last frame, where.
fn drawn_shapes_text(h: &Harness<'_, Moonglow>) -> Vec<(String, egui::Rect)> {
    (drawn(h).into_iter())
        .filter_map(|s| match s {
            egui::Shape::Text(t) => {
                Some((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2())))
            }
            _ => None,
        })
        .collect()
}

/// The Module panel's header buttons: Module Properties (named so, its
/// tree row apart from it in the Module Tree group), New Area, Expand All
/// and Collapse All, each with a tip; Expand All and Collapse All open and
/// close the tree's groups and not the palette's (which has buttons of
/// the same names), and the palette's leave the tree's.
#[test]
fn the_module_panel_s_header_buttons_act_on_the_tree_with_tips() {
    // (With the game and an area: the palette and its own Expand All and
    // Collapse All.)
    let Some((mut h, _)) = area_harness("module-panel-header") else { return };
    settle(&mut h);
    // The four, in the board's order, the last at the tab bar's end (the
    // fold button is on the dock's edge, not on the bar).
    let names = ["Module Properties", "New Area", "Expand All", "Collapse All"];
    let lefts: Vec<f32> = names.iter().map(|n| module_tool(&h, n).rect().left()).collect();
    assert!(lefts.windows(2).all(|w| w[0] < w[1]), "in order: {lefts:?}");
    assert!(h.get_by_label("Left Dock").query_by_label("Fold the Left Dock").is_none());
    let fold = h.get_by_label("Fold the Left Dock").rect();
    let last = module_tool(&h, "Collapse All").rect();
    assert!(last.right() <= fold.center().x && fold.center().x - last.right() < 6.0, "{last:?}");
    for (name, tip) in [
        ("Module Properties", "Open the module's properties"),
        ("New Area", "Make a new area with the Area Wizard"),
        ("Expand All", "Open every group"),
        ("Collapse All", "Close every group"),
    ] {
        let rect = module_tool(&h, name).rect();
        let said = tip_at(&mut h, rect);
        assert!(said.as_deref().is_some_and(|t| t.contains(tip)), "{name}: {said:?}");
    }
    // The button and the row of one name are told apart by their groups
    // and roles.
    let button = module_tool(&h, "Module Properties");
    let row = tree_row(&h, "Module Properties");
    assert_eq!(button.accesskit_node().role(), egui::accesskit::Role::Button);
    assert_ne!(row.rect(), button.rect());
    assert_eq!(h.query_all_by_label("Module Properties").count(), 2);
    // Expand All and Collapse All, on the tree only; the palette's on
    // the palette only. (What the palette shows: every node outside the
    // Left Dock's tab bar and the Module panel.)
    // (Measured with the pointer away: no button's tip open.)
    let palette_shows = |h: &mut Harness<'_, Moonglow>| {
        h.hover_at(egui::pos2(1.0, 1.0));
        settle(h);
        (h.query_all_by(|_| true))
            .filter(|n| !in_group(n, "Module Panel") && !in_group(n, "Left Dock"))
            .count()
    };
    let tree_open = |h: &Harness<'_, Moonglow>| {
        let tree = h.get_by_label("Module Tree Rows");
        tree.query_all_by(|n| n.data().is_expanded() == Some(true)).count()
    };
    let palette = palette_shows(&mut h);
    module_tool(&h, "Expand All").click();
    settle(&mut h);
    let opened = tree_open(&h);
    assert!(opened > 1, "every group open: {opened}");
    assert_eq!(palette_shows(&mut h), palette, "the palette as it was");
    module_tool(&h, "Collapse All").click();
    settle(&mut h);
    assert_eq!(tree_open(&h), 0, "every group closed");
    assert_eq!(palette_shows(&mut h), palette, "the palette as it was");
    // The palette's Expand All and Collapse All leave the tree's groups.
    module_tool(&h, "Expand All").click();
    settle(&mut h);
    apart_all(&h, "Expand All").next().expect("the palette's Expand All").click();
    settle(&mut h);
    assert_eq!(tree_open(&h), opened, "the tree's groups as they were");
    let palette_open = palette_shows(&mut h);
    assert!(palette_open > palette, "the palette's groups opened: {palette} {palette_open}");
    apart_all(&h, "Collapse All").next().expect("the palette's Collapse All").click();
    settle(&mut h);
    assert_eq!(tree_open(&h), opened, "the tree's groups as they were");
    assert!(palette_shows(&mut h) < palette_open, "the palette's groups closed");
}

/// The tree's keys walk from the Areas group's well out to the next
/// group and back: Up from Conversations goes to the last area, which the
/// well scrolls to (inside it, the first area out of sight); Down goes
/// back to Conversations, which has not moved; Home goes to the tree's
/// first row, the well back at its top. A click in the well counts as a
/// click in the tree (for the plugins' commands).
#[test]
fn the_keys_walk_out_of_the_areas_well_and_back() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    let (first, last) = (all[0], all[all.len() - 1]);
    group(&h, "Conversations").click();
    settle(&mut h);
    let conversations = group(&h, "Conversations").rect();
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Group("Conversations")));
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(2);
        settle(h);
    };
    key(&mut h, egui::Key::ArrowUp);
    let at_last = Some(TreeAt::Resource(ResKey::new(last, ResType::ARE)));
    assert_eq!(h.state().tree_cursor, at_last);
    assert!(in_well(&h, &last.to_string()), "the last area brought into the well's view");
    assert!(!in_well(&h, &first.to_string()));
    assert_eq!(group(&h, "Conversations").rect(), conversations);
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Group("Conversations")));
    assert_eq!(group(&h, "Conversations").rect(), conversations, "it has not moved");
    // Up again, then Up from the last area to the one before it.
    key(&mut h, egui::Key::ArrowUp);
    key(&mut h, egui::Key::ArrowUp);
    let before_last = ResKey::new(all[all.len() - 2], ResType::ARE);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(before_last)));
    assert!(in_well(&h, &all[all.len() - 2].to_string()));
    key(&mut h, egui::Key::Home);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Group("Areas")));
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(ResKey::new(first, ResType::ARE))));
    assert!(in_well(&h, &first.to_string()), "the well back at its top");
    // A click on a row in the well: the tree's pane.
    h.state_mut().plugins.pane = None;
    h.get_by_label(&all[1].to_string()).click();
    settle(&mut h);
    assert_eq!(h.state().plugins.pane, Some(mg_ui::plugins::Pane::Tree));
}

/// A tab chosen as a builder chooses it (Ctrl+Page Up, a click on it) for an area deep in the Areas group brings its row into view
/// inside the well; with the tree scrolled down past the well (the
/// Scripts group open, its rows wheeled to), the tree scrolls back to the
/// well too. (And as a tab chosen asks the tree, `tree_reveal` set: the
/// same.)
#[test]
fn the_tree_follows_an_area_tab_into_the_well() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(rs) = gpu() else { return };
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    app.settings.no_last_area = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let all = areas_of(&app);
    let (deep, other) = (all[90], all[40]);
    let mut h = area_gpu_harness(app, rs, 1000.0, 800.0);
    // Two areas opened one after the other, each a tab of its own (as a
    // double click on its row opens it).
    for area in [deep, other] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        h.run_steps(5);
        settle(&mut h);
    }
    // (The well back at its top.)
    let (a, c) = (*well(&h).start(), *well(&h).end());
    let x = group(&h, "Areas").rect().center().x;
    wheel(&mut h, egui::pos2(x, (a + c) / 2.0), 400.0, 12);
    assert!(in_well(&h, &all[0].to_string()), "the well at its top");
    // Ctrl+Page Up, from the other's tab to the deep one's before it.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::PageUp);
    h.run_steps(3);
    settle(&mut h);
    assert_eq!(h.state().tree_marked, Some(ResKey::new(deep, ResType::ARE)));
    assert!(in_well(&h, &deep.to_string()), "brought into the well's view");
    // The tree scrolled down past the well (the Scripts group open, its
    // row wheeled 300 points up), then a click on the other area's tab.
    group(&h, "Scripts").click();
    settle(&mut h);
    let scripts = group(&h, "Scripts").rect();
    wheel(&mut h, scripts.center(), -100.0, 3);
    let moved = scripts.top() - group(&h, "Scripts").rect().top();
    assert!(moved > 250.0, "the tree scrolled down: {moved}");
    let filter = h.get(egui_kittest::kittest::by().predicate(tree_filter)).rect();
    let out_of_view = |n: &egui_kittest::Node<'_>| {
        let bottom = n.accesskit_node().bounding_box().map(|b| b.y1 as f32);
        bottom.is_none_or(|b| b <= filter.bottom())
    };
    let first = all[0].to_string();
    assert!(
        h.get_by_label("Module Tree Rows").query_by_label(&first).is_none_or(|n| out_of_view(&n)),
        "the well out of the tree's view"
    );
    // (The tab's title drawn in its leaf's tab bar.)
    let tab = |h: &Harness<'_, Moonglow>, area: ResRef| -> egui::Pos2 {
        let s = h.state();
        let path = s.dock.find_tab(&Tab::Area(area)).expect("open");
        let leaf = s.dock.leaf(path.node_path()).unwrap();
        let bar = leaf.rect.with_max_y(leaf.viewport.top());
        (drawn_shapes_text(h).into_iter())
            .find(|(t, r)| *t == area.to_string() && bar.contains(r.center()))
            .map(|(_, r)| r.center())
            .expect("the tab's title")
    };
    let other_tab = tab(&h, other);
    click_pos(&h, other_tab);
    h.run_steps(3);
    settle(&mut h);
    assert_eq!(h.state().tree_marked, Some(ResKey::new(other, ResType::ARE)), "the tab clicked");
    // The row in sight, under the Filter and above the next group (in
    // the well).
    let in_sight = |h: &Harness<'_, Moonglow>, area: ResRef| {
        let row = h.get_by_label("Module Tree Rows").get_by_label(&area.to_string()).rect();
        let next = group(h, "Conversations").rect();
        row.top() >= filter.bottom() - 0.5 && row.bottom() <= next.top() + 0.5
    };
    assert!(in_sight(&h, other), "the clicked tab's area in the well, the well in sight");
    // As a tab chosen asks the tree (`reveal_tab`): the same.
    let third = all[100];
    h.state_mut().tree_reveal = Some((ResKey::new(third, ResType::ARE), false));
    h.run_steps(3);
    settle(&mut h);
    assert!(in_sight(&h, third));
    assert_eq!(h.state().tree_area, Some(third));
}

/// A row in the well has its menu (View Area, Properties), the well
/// scrolled down to it; and the rows at the well's top and foot, partly
/// out of its sight, have theirs from the part in sight. Nothing in the
/// well is dragged (an area's row and a placed object's are clicked, not
/// dragged: only a blueprint's row drags, into an area); a blueprint's
/// row under the well, the well scrolled, drags its own blueprint and
/// leaves the well as it was.
#[test]
fn a_row_in_the_well_has_its_menu_and_a_blueprint_row_drags() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    let x = group(&h, "Areas").rect().center().x;
    let (a, c) = (*well(&h).start(), *well(&h).end());
    // Wheeled to the middle of the well, then a row there.
    wheel(&mut h, egui::pos2(x, (a + c) / 2.0), -200.0, 4);
    let area = (all.iter().map(|r| r.to_string()))
        .find(|l| in_well(&h, l) && h.get_by_label(l).rect().top() > a + 44.0)
        .expect("a row well inside the well");
    let menu = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| -> bool {
        h.hover_at(at);
        h.run_steps(1);
        let button = egui::PointerButton::Secondary;
        let none = egui::Modifiers::NONE;
        h.event(egui::Event::PointerButton { pos: at, button, pressed: true, modifiers: none });
        h.event(egui::Event::PointerButton { pos: at, button, pressed: false, modifiers: none });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while h.query_by_label("View Area").is_none() && std::time::Instant::now() < deadline {
            h.run_steps(1);
        }
        // (An area's menu: View Area, and its Properties last.)
        let open = h.query_by_label("View Area").is_some()
            && h.query_all_by_label("Properties").count() >= 1;
        h.key_press(egui::Key::Escape);
        settle(h);
        open
    };
    let at = h.get_by_label(&area).rect().center();
    assert!(menu(&mut h, at), "the area's menu, {area} in the well");
    // The rows at its top and foot, wheeled half a row so that they are
    // partly out of its sight: their menus from the part in sight.
    wheel(&mut h, egui::pos2(x, (a + c) / 2.0), -11.0, 1);
    let tree = h.get_by_label("Module Tree Rows");
    let rows: Vec<egui::Rect> = (all.iter())
        .filter_map(|r| tree.query_by_label(&r.to_string()).map(|n| n.rect()))
        .collect();
    let top = rows.iter().find(|r| r.top() < a - 1.0 && r.bottom() > a + 1.0).expect("cut at top");
    let foot =
        rows.iter().find(|r| r.top() < c - 1.0 && r.bottom() > c + 1.0).expect("cut at foot");
    let (top, foot) = (*top, *foot);
    assert!(menu(&mut h, egui::pos2(x, (a + top.bottom()) / 2.0)), "the top row's menu");
    assert!(menu(&mut h, egui::pos2(x, (foot.top() + c) / 2.0)), "the foot row's menu");
    // Nothing is a row of the well above or under it: a click just over
    // the next group opens no menu of the well's.
    let next = group(&h, "Conversations").rect();
    assert!(!menu(&mut h, egui::pos2(x, next.center().y)), "the next group's row, not the well's");
    // A creature's blueprint dragged from its row, the well scrolled.
    let first_in_well = |h: &Harness<'_, Moonglow>| {
        (all.iter().map(|r| r.to_string())).find(|l| in_well(h, l)).unwrap()
    };
    let in_well_before = first_in_well(&h);
    group(&h, "Creatures").click();
    settle(&mut h);
    let below = group(&h, "Creatures").rect().bottom();
    let row = (h
        .get_by_label("Module Tree Rows")
        .query_all_by(|n| n.label().is_some_and(|l| l.ends_with(".utc"))))
    .map(|n| n.rect())
    .find(|r| r.top() >= below - 0.5)
    .expect("a creature's row");
    let from = row.center();
    h.hover_at(from);
    h.run();
    press(&h, from, true, egui::Modifiers::NONE);
    h.run();
    for k in 1..=4 {
        h.hover_at(from + egui::vec2(20.0, 10.0) * k as f32);
        h.run();
    }
    assert!(h.ctx.dragged_id().is_some(), "a drag started from the row");
    press(&h, from + egui::vec2(80.0, 40.0), false, egui::Modifiers::NONE);
    settle(&mut h);
    assert_eq!(first_in_well(&h), in_well_before, "the well as it was");
}

/// The Filter finds rows in every group (areas, conversations, scripts by
/// their ResRef, the placed objects of the areas), each group's count
/// then what it finds; Clear Filter restores the tree.
#[test]
fn the_filter_finds_rows_in_every_group_and_clear_filter_restores_them() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let whole: Vec<String> = ["Areas", "Conversations", "Scripts"]
        .iter()
        .map(|g| group(&h, g).accesskit_node().label().unwrap())
        .collect();
    type_into_tree_filter(&mut h, "m1q1a");
    for _ in 0..20 {
        h.run_steps(2);
    }
    settle(&mut h);
    for (g, was) in ["Areas", "Conversations", "Scripts"].iter().zip(&whole) {
        let now = group(&h, g).accesskit_node().label().unwrap();
        assert_ne!(&now, was, "{g} filtered");
        assert!(!now.ends_with("(0)"), "{g} finds something: {now}");
    }
    assert!(h.get_by_label("Module Tree Rows").query_by_label("map_m1q1a").is_some());
    h.get_by_label("Clear Filter").click();
    settle(&mut h);
    let back: Vec<String> = ["Areas", "Conversations", "Scripts"]
        .iter()
        .map(|g| group(&h, g).accesskit_node().label().unwrap())
        .collect();
    assert_eq!(back, whole, "the tree as it was");
}

/// An object selected in an area's view (Chapter1's `map_m1q1a`, one of
/// its creatures far down their list) opens its area out in the tree and
/// brings its row into view inside the Areas group's well, marked.
#[test]
fn an_object_selected_in_an_area_brings_its_row_into_the_well_s_view() {
    use mg_area::ObjectKind;
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(rs) = gpu() else { return };
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    app.settings.no_last_area = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let area = ResRef::from_str("map_m1q1a").unwrap();
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Area(area)));
    let mut h = area_gpu_harness(app, rs, 1100.0, 800.0);
    h.run_steps(5);
    settle(&mut h);
    let git = ResKey::new(area, ResType::GIT);
    let git = h.state().ws.as_ref().unwrap().module.gff(&git).unwrap().unwrap();
    let many = git.root.list(ObjectKind::Creature.list()).map_or(0, |l| l.len());
    assert!(many > 30, "map_m1q1a's creatures: {many}");
    let marked = |h: &Harness<'_, Moonglow>| -> Vec<(String, egui::Rect)> {
        let tree = h.get_by_label("Module Tree Rows");
        (tree.query_all_by(|n| n.toggled() == Some(egui::accesskit::Toggled::True)))
            .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
            .filter(|(l, _)| *l != "map_m1q1a")
            .collect()
    };
    for index in [many - 2, 1] {
        h.state_mut().area_views.get_mut(&area).unwrap().selection =
            vec![(ObjectKind::Creature, index)];
        h.run_steps(8);
        settle(&mut h);
        assert_eq!(h.state().tree_object, Some((area, ObjectKind::Creature, index)));
        assert!(!h.state().tree_object_pending, "gone to");
        let rows = marked(&h);
        assert_eq!(rows.len(), 1, "the object's row marked: {rows:?}");
        let r = rows[0].1;
        let w = well(&h);
        assert!(w.contains(&(r.top() + 0.5)) && w.contains(&(r.bottom() - 0.5)), "{r:?} {w:?}");
    }
}

/// A module of `n` areas (empty AREs, `area_000`...) and 40 scripts, in
/// a scratch folder named `name`.
fn n_areas(name: &str, n: usize) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("SAMPLE"));
    for i in 0..n {
        let mut a = ifo::MOD_AREA_LIST.new_item();
        let r = ResRef::from_str(&format!("area_{i:03}")).unwrap();
        a.write(&ifo::mod_area_list::AREA_NAME, r);
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
        m.set(ResKey::new(r, ResType::ARE), Gff::new(*b"ARE ").to_bytes().unwrap());
    }
    m.set_info(&info).unwrap();
    for i in 0..40 {
        let k = ResKey::parse(&format!("script_{i:02}"), ResType::NSS).unwrap();
        m.set(k, b"void main() { }\n".to_vec());
    }
    let path = dir.join("many.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

/// [`n_areas`]'s module open in a window `width` by `height`.
fn small(name: &str, n: usize, width: f32, height: f32) -> Harness<'static, Moonglow> {
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&n_areas(name, n));
    let mut h = harness(app, width, height);
    settle(&mut h);
    h
}

/// The foot of the tree's view (the Status Bar's top).
fn view_bottom(h: &Harness<'_, Moonglow>) -> f32 {
    h.ctx.content_rect().bottom() - mg_uikit::metrics::STATUS_BAR
}

/// How many rows high the well is.
fn well_rows(h: &Harness<'_, Moonglow>) -> f32 {
    let w = well(h);
    (w.end() - w.start()) / mg_uikit::metrics::ROW_HEIGHT
}

/// The first area wholly in the well's view.
fn first_in_well(h: &Harness<'_, Moonglow>) -> Option<String> {
    let all = areas(h);
    let tree = h.get_by_label("Module Tree Rows");
    let w = well(h);
    let mut rows: Vec<(String, f32)> = (all.iter().map(|r| r.to_string()))
        .filter_map(|l| tree.query_by_label(&l).map(|n| n.rect()).map(|r| (l, r)))
        .filter(|(_, r)| w.contains(&(r.top() + 0.5)) && w.contains(&(r.bottom() - 0.5)))
        .map(|(l, r)| (l, r.top()))
        .collect();
    rows.sort_by(|a, b| a.1.total_cmp(&b.1));
    rows.first().map(|r| r.0.clone())
}

/// Wheels `dx` points sideways over `at`, a frame or two each.
fn wheel_sideways(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, dx: f32, times: usize) {
    h.hover_at(at);
    for _ in 0..times {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(dx, 0.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(2);
    }
    settle(h);
}

/// How far the tree goes sideways when wheeled over the Scripts group's
/// row (and back after).
fn sideways(h: &mut Harness<'_, Moonglow>) -> f32 {
    let at = group(h, "Scripts").rect();
    wheel_sideways(h, at.center(), -100.0, 8);
    let moved = at.left() - group(h, "Scripts").rect().left();
    wheel_sideways(h, at.center(), 100.0, 8);
    moved
}

/// The Left Dock's right edge (where it is dragged wider or narrower).
fn dock_edge(h: &Harness<'_, Moonglow>) -> f32 {
    h.get_by_label("Fold the Left Dock").rect().center().x + 1.0
}

/// The Left Dock widened by 300 points, then narrowed to its least width:
/// the tree, with no row wider than the dock, does not scroll sideways
/// (the Areas group's well had been taken as wide as the widest view it
/// was drawn in, kept for good, and the tree scrolled 322 points into
/// nothing), and so has no scroll bar across its foot to take a row from
/// the well. With the Areas group closed, the same.
#[test]
fn a_narrowed_dock_leaves_the_tree_no_wider_than_its_rows() {
    for areas_open in [true, false] {
        let Some(mut h) = chapter1(1280.0, 800.0) else { return };
        if !areas_open {
            group(&h, "Areas").click();
            settle(&mut h);
        }
        let rows = areas_open.then(|| well_rows(&h));
        assert_eq!(sideways(&mut h), 0.0, "as it opens");
        // (Dragged by the edge above its fold button, which is half way
        // down.)
        let w = dock_edge(&h);
        drag(&mut h, egui::pos2(w, 250.0), egui::pos2(w + 300.0, 250.0));
        settle(&mut h);
        assert!(dock_edge(&h) > w + 250.0, "widened");
        assert_eq!(sideways(&mut h), 0.0, "widened");
        let w = dock_edge(&h);
        drag(&mut h, egui::pos2(w, 250.0), egui::pos2(w - 600.0, 250.0));
        settle(&mut h);
        assert!(dock_edge(&h) < 200.0, "narrowed to its least: {}", dock_edge(&h));
        assert_eq!(sideways(&mut h), 0.0, "narrowed (Areas open: {areas_open})");
        if let Some(rows) = rows {
            assert_eq!(well_rows(&h), rows, "the well's rows as they were");
        }
    }
}

/// The well keeps its rows (14 in an 800 px window) whatever else of the
/// tree is open: the Scripts group open (whose rows made the tree a
/// scroll bar across its foot and took a row from the well), every group
/// open, the tree wheeled frame by frame; and the tree, none of its rows
/// wider than it, does not scroll sideways with the Scripts group open.
#[test]
fn the_well_keeps_its_rows_whatever_else_is_open() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let rows = well_rows(&h);
    assert_eq!(rows, 14.0, "with the groups closed");
    group(&h, "Scripts").click();
    settle(&mut h);
    assert_eq!(well_rows(&h), rows, "the Scripts group open");
    assert_eq!(sideways(&mut h), 0.0, "no row wider than the tree");
    assert_eq!(well_rows(&h), rows);
    module_tool(&h, "Expand All").click();
    settle(&mut h);
    assert_eq!(well_rows(&h), rows, "every group open");
    // Frame by frame while the tree is wheeled down.
    let at = group(&h, "Scripts").rect().center();
    h.hover_at(at);
    for _ in 0..20 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -60.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
        if let Some(areas) = h.get_by_label("Module Tree Rows").query_by_label("Areas (113)")
            && let Some(next) =
                h.get_by_label("Module Tree Rows").query_by_label("Conversations (264)")
        {
            let tall = next.rect().top() - areas.rect().bottom();
            assert_eq!(tall / mg_uikit::metrics::ROW_HEIGHT, rows, "frame by frame");
        }
    }
}

/// Home and End over the well go to its top and foot, the groups under it
/// staying; and with the Areas group closed, over where the well was,
/// they go to the tree's top and end (the well's place was remembered
/// after it was gone, and the tree's Home and End did nothing there).
#[test]
fn home_and_end_go_to_the_well_s_ends_over_it_and_the_tree_s_elsewhere() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    let (first, last) = (all[0].to_string(), all[all.len() - 1].to_string());
    let (a, c) = (*well(&h).start(), *well(&h).end());
    let x = group(&h, "Areas").rect().center().x;
    let key = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2, key: egui::Key| {
        h.hover_at(at);
        h.run_steps(2);
        h.key_press(key);
        h.run_steps(2);
        settle(h);
    };
    let groups = groups_at(&h);
    // (The keys not the tree's: Home and End over it are the pointer's.)
    assert_eq!(h.state().tree_cursor, None);
    key(&mut h, egui::pos2(x, (a + c) / 2.0), egui::Key::End);
    assert!(in_well(&h, &last) && !in_well(&h, &first), "the well at its foot");
    assert_eq!(groups_at(&h), groups, "the groups under it stay");
    key(&mut h, egui::pos2(x, (a + c) / 2.0), egui::Key::Home);
    assert!(in_well(&h, &first) && !in_well(&h, &last), "the well at its top");
    assert_eq!(groups_at(&h), groups);
    // The Areas group closed, the Scripts group open: over where the
    // well was, and below it.
    group(&h, "Areas").click();
    settle(&mut h);
    group(&h, "Scripts").click();
    settle(&mut h);
    // (The clicks gave the tree the keys: a click in the log takes them.)
    let log = h.get_by_label_contains("Opened").rect().center();
    click_pos(&h, log);
    settle(&mut h);
    h.state_mut().tree_cursor = None;
    let scripts = group(&h, "Scripts").rect();
    for y in [(a + c) / 2.0, 736.0] {
        key(&mut h, egui::pos2(x, y), egui::Key::End);
        let at = group(&h, "Scripts").rect().top();
        assert!(at < scripts.top() - 1000.0, "at the tree's end (y {y}): Scripts at {at}");
        let last = group(&h, "Journal and factions").rect();
        assert!(last.bottom() <= view_bottom(&h) + 0.5, "the last group in sight: {last:?}");
        key(&mut h, egui::pos2(x, y), egui::Key::Home);
        assert_eq!(group(&h, "Scripts").rect(), scripts, "back at the tree's top (y {y})");
    }
}

/// The well, scrolled down its areas, is where it was once the Filter is
/// cleared, after a word that finds some areas and one that finds
/// nothing (it came back at its top); so is the tree, scrolled down the
/// Scripts group.
#[test]
fn the_well_and_the_tree_keep_their_places_through_the_filter() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let (a, c) = (*well(&h).start(), *well(&h).end());
    let x = group(&h, "Areas").rect().center().x;
    wheel(&mut h, egui::pos2(x, (a + c) / 2.0), -200.0, 5);
    let before = first_in_well(&h).unwrap();
    assert_ne!(Some(&before), areas(&h).first().map(|r| r.to_string()).as_ref());
    let filter = |h: &mut Harness<'_, Moonglow>, text: &str| {
        type_into_tree_filter(h, text);
        for _ in 0..20 {
            h.run_steps(2);
        }
        settle(h);
        h.get_by_label("Clear Filter").click();
        settle(h);
    };
    for text in ["m1q1", "zqxw"] {
        filter(&mut h, text);
        assert_eq!(first_in_well(&h), Some(before.clone()), "after {text}");
    }
    // The tree scrolled down the Scripts group (in a window as it opens:
    // a word typed opens the groups it finds in, which stay open).
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    group(&h, "Scripts").click();
    settle(&mut h);
    let scripts = group(&h, "Scripts").rect();
    wheel(&mut h, scripts.center(), -100.0, 5);
    let top = |h: &Harness<'_, Moonglow>| tree_row(h, "Module Properties").rect().top();
    let at = top(&h);
    assert!(at < scripts.top() - 300.0, "scrolled down: {at}");
    for text in ["zqxw", "m1q1"] {
        filter(&mut h, text);
        assert_eq!(top(&h), at, "the tree after {text}");
    }
}

/// The well's height in whole rows: at least the board's five where the
/// window is too low for more (336 and 400 px: the groups under it then
/// scrolled to), and no higher than the rows it holds (a module of three
/// areas shows three, the next group right under them with no room
/// between).
#[test]
fn the_well_shows_five_rows_at_least_and_no_more_than_it_holds() {
    for height in [336.0, 400.0] {
        let Some(h) = chapter1(1000.0, height) else { return };
        assert_eq!(well_rows(&h), 5.0, "{height} px");
    }
    let Some(h) = chapter1(1000.0, 700.0) else { return };
    assert!(well_rows(&h) > 5.0, "more where there is room");
    for height in [336.0, 800.0] {
        let h = small(&format!("ui-module-panel-three-{height}"), 3, 1000.0, height);
        assert_eq!(well_rows(&h), 3.0, "three areas, three rows ({height} px)");
        let row = |n: usize| h.get_by_label(&format!("area_{n:03}")).rect();
        let next = group(&h, "Conversations").rect();
        assert_eq!(row(2).bottom(), next.top(), "no room under the last area");
        assert_eq!(row(0).top(), group(&h, "Areas").rect().bottom());
    }
}

/// In the least window (420 by 320 points), with the groups closed, every
/// group's row is reached: by wheeling the tree down, and by the keys,
/// each group's row in sight as the keys come to it.
#[test]
fn every_group_is_reached_in_the_least_window() {
    // (A test's window has a 16 px frame: `shell::TEST_FRAME`.)
    let least = mg_ui::LEAST_WINDOW_SIZE + egui::Vec2::splat(16.0);
    let Some(mut h) = chapter1(least.x, least.y) else { return };
    assert_eq!(well_rows(&h), 5.0);
    let view = |h: &Harness<'_, Moonglow>| {
        let filter = h.get(egui_kittest::kittest::by().predicate(tree_filter)).rect();
        let bottom = h.ctx.content_rect().bottom() - mg_uikit::metrics::STATUS_BAR;
        filter.bottom()..=bottom
    };
    let in_sight = |h: &Harness<'_, Moonglow>, name: &str| {
        let (r, v) = (group(h, name).rect(), view(h));
        v.contains(&(r.top() + 0.5)) && v.contains(&(r.bottom() - 0.5))
    };
    assert!(!in_sight(&h, "Journal and factions"), "out of sight at first");
    // Wheeled over Conversations' row: down to the last group.
    let at = group(&h, "Conversations").rect().center();
    wheel(&mut h, at, -100.0, 10);
    assert!(in_sight(&h, "Journal and factions"), "wheeled to");
    wheel(&mut h, at, 100.0, 10);
    // By the keys (given to the tree by a click on Areas' row, twice:
    // closed and opened again), Home, then Down to every group.
    for _ in 0..2 {
        group(&h, "Areas").click();
        settle(&mut h);
    }
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(2);
        settle(h);
    };
    key(&mut h, egui::Key::Home);
    let mut seen = Vec::new();
    for _ in 0..400 {
        key(&mut h, egui::Key::ArrowDown);
        if let Some(TreeAt::Group(name)) = h.state().tree_cursor
            && !seen.contains(&name)
        {
            assert!(in_sight(&h, name), "{name} in sight as the keys come to it");
            seen.push(name);
        }
        if seen.last() == Some(&"Journal and factions") {
            break;
        }
    }
    assert_eq!(seen, BELOW, "every group under Areas, by the keys");
}

/// F2 on an area deep in the well (the keys at it) asks for its new name
/// in the Rename window (the tree has no field of its own in a row). What
/// is typed there is the window's alone: Escape closes it and leaves the
/// tree as it was before F2, the keys its own at the same row (Down goes
/// on from it); Enter renames it, and the keys are at its new row, in the
/// well's view, and still the tree's.
#[test]
fn f2_renames_an_area_in_the_well() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    let deep = all[60];
    let at = |k: ResRef| Some(TreeAt::Resource(ResKey::new(k, ResType::ARE)));
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(2);
        settle(h);
    };
    // The keys at the deep area: to the tree, then by its letters' row.
    for _ in 0..2 {
        group(&h, "Conversations").click();
        settle(&mut h);
    }
    h.state_mut().tree_cursor = at(deep);
    key(&mut h, egui::Key::ArrowDown);
    key(&mut h, egui::Key::ArrowUp);
    assert!(in_well(&h, &deep.to_string()), "the keys' row in the well's view");
    let before = first_in_well(&h);
    key(&mut h, egui::Key::F2);
    assert_eq!(h.state().rename.as_ref().map(|r| r.from), Some(ResKey::new(deep, ResType::ARE)));
    h.get_by_label(&format!("Rename {deep}.are"));
    // Down in the window is not the tree's.
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().tree_cursor, at(deep), "the window's key");
    key(&mut h, egui::Key::Escape);
    assert!(h.state().rename.is_none(), "Escape closes it");
    assert!(h.query_by_label(&format!("Rename {deep}.are")).is_none());
    assert_eq!(h.state().tree_cursor, at(deep), "the keys where they were");
    assert!(in_well(&h, &deep.to_string()), "the row as it was");
    assert_eq!(first_in_well(&h), before);
    // Down goes on from the row being renamed, and Up back.
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().tree_cursor, at(all[61]), "the keys still the tree's");
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(h.state().tree_cursor, at(deep));
    // F2 again, a new name typed, and Enter.
    key(&mut h, egui::Key::F2);
    assert_eq!(h.state().rename.as_ref().map(|r| r.from), Some(ResKey::new(deep, ResType::ARE)));
    let name = deep.to_string();
    typed_name(&h, &name).click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run_steps(1);
    typed_name(&h, &name).type_text("zz_renamed");
    h.run_steps(2);
    key(&mut h, egui::Key::Enter);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let renamed = ResKey::parse("zz_renamed", ResType::ARE).unwrap();
    while !h.state().ws.as_ref().unwrap().module.contains(&renamed) {
        assert!(std::time::Instant::now() < deadline, "renamed");
        h.run_steps(1);
    }
    settle(&mut h);
    assert!(h.state().rename.is_none());
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(renamed)), "at its new row");
    assert!(in_well(&h, "zz_renamed"), "in the well's view (the last area now)");
    // Up goes on from it: the keys still the tree's.
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(h.state().tree_cursor, at(all[all.len() - 1]));
}

/// Delete on an area deep in the well (the keys at it) asks, and Enter
/// answers Delete; once it is deleted, the keys are at the row that took
/// its place (the next area), in the well's view and still the tree's (Down
/// goes on from there), and the well is as high as before (it holds more
/// than it shows) and where it was. Escape in the question leaves the tree
/// as it was. Undone, the area is back and the keys stay on a row there.
#[test]
fn deleting_an_area_in_the_well_leaves_the_well_in_place() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    let deep = all[60];
    let at = |k: ResRef| Some(TreeAt::Resource(ResKey::new(k, ResType::ARE)));
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(2);
        settle(h);
    };
    for _ in 0..2 {
        group(&h, "Conversations").click();
        settle(&mut h);
    }
    h.state_mut().tree_cursor = at(deep);
    key(&mut h, egui::Key::ArrowDown);
    key(&mut h, egui::Key::ArrowUp);
    assert!(in_well(&h, &deep.to_string()));
    let (rows, before) = (well_rows(&h), first_in_well(&h));
    let groups = groups_at(&h);
    // Asked, then Escape: nothing deleted, the keys where they were.
    key(&mut h, egui::Key::Delete);
    assert_eq!(h.state().confirm_delete, Some(vec![ResKey::new(deep, ResType::ARE)]));
    h.get_by_label("Delete");
    key(&mut h, egui::Key::Escape);
    assert_eq!(h.state().confirm_delete, None, "Escape is Cancel");
    assert_eq!(h.state().tree_cursor, at(deep), "the keys where they were");
    // Asked again, and Enter (once it has read what names it).
    key(&mut h, egui::Key::Delete);
    assert_eq!(h.state().confirm_delete, Some(vec![ResKey::new(deep, ResType::ARE)]));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while h.get_by_label("Delete").accesskit_node().is_disabled() {
        assert!(std::time::Instant::now() < deadline, "Delete enabled");
        h.run_steps(1);
    }
    key(&mut h, egui::Key::Enter);
    while h.state().ws.as_ref().unwrap().module.contains(&ResKey::new(deep, ResType::ARE)) {
        assert!(std::time::Instant::now() < deadline, "deleted");
        h.run_steps(1);
    }
    settle(&mut h);
    assert_eq!(h.state().tree_cursor, at(all[61]), "at the row that took its place");
    assert!(in_well(&h, &all[61].to_string()), "in the well's view");
    assert_eq!(well_rows(&h), rows, "as high");
    assert_eq!(first_in_well(&h), before, "where it was");
    assert_eq!(h.get_by_label("Module Tree Rows").query_all_by_label("Areas (112)").count(), 1);
    assert_eq!(groups_at(&h), groups, "the groups under it stay");
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().tree_cursor, at(all[62]), "Down goes on from there");
    // Undone: back, and the keys on a row that is there.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert!(h.state().ws.as_ref().unwrap().module.contains(&ResKey::new(deep, ResType::ARE)));
    assert_eq!(h.state().tree_cursor, at(all[62]));
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(h.state().tree_cursor, at(all[61]));
}

/// A tab's menu's Show in Module Tree, for an area deep in the well (its
/// row brought into the well's view, the area opened out) and for a
/// script far down the Scripts group under the well (the group opened,
/// the tree scrolled to the script's row).
#[test]
fn show_in_module_tree_from_a_tab_s_menu_reaches_into_the_well_and_below() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let Some(rs) = gpu() else { return };
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    app.settings.no_last_area = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let all = areas_of(&app);
    let deep = all[90];
    let script = {
        let ws = app.ws.as_ref().unwrap();
        let mut scripts: Vec<ResKey> = ws.module.keys_of(ResType::NSS).copied().collect();
        scripts.sort();
        scripts[1500]
    };
    let mut h = area_gpu_harness(app, rs, 1000.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(deep)));
    h.run_steps(5);
    settle(&mut h);
    // Away from the tab: the groups closed but Areas, the well at its top.
    module_tool(&h, "Collapse All").click();
    settle(&mut h);
    group(&h, "Areas").click();
    settle(&mut h);
    let (a, c) = (*well(&h).start(), *well(&h).end());
    let x = group(&h, "Areas").rect().center().x;
    wheel(&mut h, egui::pos2(x, (a + c) / 2.0), 400.0, 12);
    assert!(!in_well(&h, &deep.to_string()));
    show_in_tree_from_tab(&mut h, &Tab::Area(deep), &deep.to_string());
    assert_eq!(h.state().tree_marked, Some(ResKey::new(deep, ResType::ARE)));
    assert!(in_well(&h, &deep.to_string()), "the area's row in the well's view");
    let arrow = format!("Placed Objects in {deep}");
    let arrow = h.get_by_label(&arrow);
    assert_eq!(arrow.accesskit_node().data().is_expanded(), Some(true), "opened out");
    // A script far down its group, opened (in a window of its own), and
    // the tree back at its top: its tab's Show in Module Tree.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(script)));
    h.run_steps(5);
    settle(&mut h);
    module_tool(&h, "Collapse All").click();
    settle(&mut h);
    group(&h, "Areas").click();
    settle(&mut h);
    show_in_tree_from_tab(&mut h, &Tab::Script(script), &script.resref.to_string().to_lowercase());
    assert_eq!(h.state().tree_marked, Some(script));
    let filter = h.get(egui_kittest::kittest::by().predicate(tree_filter)).rect();
    let bottom = h.ctx.content_rect().bottom() - mg_uikit::metrics::STATUS_BAR;
    let row = h.get_by_label("Module Tree Rows").get_by_label(&script.resref.to_string()).rect();
    assert!(row.top() >= filter.bottom() - 0.5 && row.bottom() <= bottom + 0.5, "{row:?}");
}

/// Show in Module Tree from the menu of `tab`'s tab (its title `title`,
/// lowercase, drawn in its leaf's tab bar, right-clicked: a press, and its
/// release the next frame).
fn show_in_tree_from_tab(h: &mut Harness<'_, Moonglow>, tab: &Tab, title: &str) {
    let s = h.state();
    let path = s.dock.find_tab(tab).expect("open");
    let leaf = s.dock.leaf(path.node_path()).unwrap();
    let bar = leaf.rect.with_max_y(leaf.viewport.top());
    let at = (drawn_shapes_text(h).into_iter())
        .find(|(t, r)| t.to_lowercase().starts_with(title) && bar.contains(r.center()))
        .map(|(_, r)| r.center())
        .expect("the tab's title");
    h.hover_at(at);
    h.run_steps(2);
    let button = egui::PointerButton::Secondary;
    let none = egui::Modifiers::NONE;
    h.event(egui::Event::PointerButton { pos: at, button, pressed: true, modifiers: none });
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: at, button, pressed: false, modifiers: none });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while h.query_by_label("Show in Module Tree").is_none() {
        assert!(std::time::Instant::now() < deadline, "the tab's menu");
        h.run_steps(1);
    }
    // (The item under the pointer a frame before it is clicked.)
    let item = popup_item(h, "Show in Module Tree").rect().center();
    h.hover_at(item);
    h.run_steps(2);
    popup_item(h, "Show in Module Tree").click();
    h.step();
    h.run_steps(1);
    assert!(h.query_by_label("Show in Module Tree").is_none(), "the menu closed");
    h.run_steps(5);
    settle(h);
}

/// Show in Module Tree from a tab's menu with the Left Dock on its
/// Resources panel: the dock goes to its Module tab, the row marked.
#[test]
fn show_in_module_tree_with_the_resources_panel_chooses_the_module_tab() {
    let Some(mut h) = game_harness("show-in-tree-resources") else { return };
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    show_resources(h.state_mut());
    h.run_steps(5);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources));
    show_in_tree_from_tab(&mut h, &Tab::Script(hello), "hello");
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Module));
    assert_eq!(h.state().tree_marked, Some(hello));
    h.get_by_label("Module Tree Rows").get_by_label("hello");
}

/// The keys at an area deep in the well: F6 gives them to the next pane
/// and round again back to the tree, at the same row, the well showing
/// it; Shift+F6 the same the other way. Tab moves the keyboard's focus
/// from a row in the well to the next row, as from any of the tree's rows.
#[test]
fn f6_and_tab_into_and_out_of_the_module_panel_with_a_row_in_the_well() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    let deep = ResKey::new(all[60], ResType::ARE);
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key, m: egui::Modifiers| {
        h.key_press_modifiers(m, key);
        h.run_steps(2);
        settle(h);
    };
    let none = egui::Modifiers::NONE;
    for _ in 0..2 {
        group(&h, "Conversations").click();
        settle(&mut h);
    }
    h.state_mut().tree_cursor = Some(TreeAt::Resource(deep));
    key(&mut h, egui::Key::ArrowDown, none);
    key(&mut h, egui::Key::ArrowUp, none);
    assert!(in_well(&h, &all[60].to_string()));
    // Out and round again, whichever way.
    for m in [none, egui::Modifiers::SHIFT] {
        // (Whether the keys are the tree's: Down moves its cursor, and Up
        // brings it back.)
        let mut steps = 0;
        loop {
            key(&mut h, egui::Key::F6, m);
            steps += 1;
            key(&mut h, egui::Key::ArrowDown, none);
            if h.state().tree_cursor != Some(TreeAt::Resource(deep)) {
                key(&mut h, egui::Key::ArrowUp, none);
                break;
            }
            assert!(steps < 6, "back to the tree");
        }
        assert!(steps > 1, "the keys went to another pane first");
        assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(deep)), "at the row they were at");
        // Wheeled away, then Down and Up: back in the well's view.
        let (a, c) = (*well(&h).start(), *well(&h).end());
        let x = group(&h, "Areas").rect().center().x;
        wheel(&mut h, egui::pos2(x, (a + c) / 2.0), 400.0, 12);
        key(&mut h, egui::Key::ArrowDown, none);
        key(&mut h, egui::Key::ArrowUp, none);
        assert!(in_well(&h, &all[60].to_string()), "the keys' row in the well's view");
    }
    // Tab: the keyboard's focus from a row in the well to the next.
    h.get_by_label(&all[60].to_string()).focus();
    h.run_steps(2);
    assert!(h.get_by_label(&all[60].to_string()).is_focused());
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
    let arrow = format!("Placed Objects in {}", all[61]);
    let next =
        [arrow.as_str(), &all[61].to_string()].into_iter().any(|l| h.get_by_label(l).is_focused());
    assert!(next, "the focus on the next row");
}

/// Shift and Ctrl with a click choose several across the well's border:
/// from an area at the well's top, Shift and a click on the last area
/// (the well scrolled to its foot) chooses every area; Ctrl and a click on
/// the first script under the well adds it (several kinds chosen
/// together), and Ctrl again takes it away.
#[test]
fn several_are_chosen_across_the_well_s_border() {
    let mut h = small("ui-module-panel-several", 30, 1000.0, 600.0);
    let names: Vec<String> = (0..30).map(|i| format!("area_{i:03}")).collect();
    let keys: Vec<ResKey> =
        (0..30).map(|i| ResKey::new(ResRef::from_str(&names[i]).unwrap(), ResType::ARE)).collect();
    let chosen = |h: &Harness<'_, Moonglow>| -> Vec<ResKey> {
        h.state().tree_selected.iter().copied().collect()
    };
    h.get_by_label(&names[0]).click();
    h.run_steps(2);
    let (a, c) = (*well(&h).start(), *well(&h).end());
    let x = group(&h, "Areas").rect().center().x;
    wheel(&mut h, egui::pos2(x, (a + c) / 2.0), -400.0, 6);
    assert!(in_well(&h, &names[29]) && !in_well(&h, &names[0]));
    h.get_by_label(&names[29]).click_modifiers(egui::Modifiers::SHIFT);
    h.run_steps(2);
    assert_eq!(chosen(&h), keys, "every area, the first out of the well's view");
    group(&h, "Scripts").click();
    settle(&mut h);
    let script = ResKey::parse("script_00", ResType::NSS).unwrap();
    h.get_by_label("script_00").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    let mut with = keys.clone();
    with.push(script);
    with.sort();
    let mut now = chosen(&h);
    now.sort();
    assert_eq!(now, with, "the script added under the well");
    h.get_by_label("script_00").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    assert_eq!(chosen(&h), keys, "and taken away");
    // Several chosen: each marked, those in the well's view too.
    let marked = (names.iter()).filter(|n| in_well(&h, n)).all(|n| {
        h.get_by_label(n).accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
    });
    assert!(marked, "the chosen rows in the well marked");
}

/// Deleted from the tree by the keys (Delete, then Enter for the
/// question), resources leave the keys at the row that takes their place,
/// the keys still the tree's: several chosen together (past them all),
/// the last of a group (the one before it), the only one of a group (the
/// group's row).
#[test]
fn the_keys_go_on_past_resources_deleted() {
    let script = |n: usize| ResKey::parse(&format!("script_{n:02}"), ResType::NSS).unwrap();
    let mut h = small("ui-module-panel-deleted", 1, 1000.0, 800.0);
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(2);
        settle(h);
    };
    let delete = |h: &mut Harness<'_, Moonglow>, gone: &[ResKey]| {
        // (The pointer off the tree: Delete is then the keys' row's.)
        h.hover_at(egui::pos2(600.0, 300.0));
        h.run_steps(2);
        key(h, egui::Key::Delete);
        assert!(h.state().confirm_delete.is_some(), "asked");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while h.get_by_label("Delete").accesskit_node().is_disabled() {
            assert!(std::time::Instant::now() < deadline, "Delete enabled");
            h.run_steps(1);
        }
        key(h, egui::Key::Enter);
        while gone.iter().any(|k| h.state().ws.as_ref().unwrap().module.contains(k)) {
            assert!(std::time::Instant::now() < deadline, "deleted");
            h.run_steps(1);
        }
        settle(h);
    };
    group(&h, "Scripts").click();
    settle(&mut h);
    // Two chosen (a click, Shift and the next), the keys at the second.
    h.get_by_label("script_05").click();
    h.run_steps(2);
    h.get_by_label("script_06").click_modifiers(egui::Modifiers::SHIFT);
    h.run_steps(2);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(script(6))));
    delete(&mut h, &[script(5), script(6)]);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(script(7))), "past both");
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(script(4))), "the keys the tree's");
    // The last of the group: the one before it, in sight.
    // (The keys to it: past it, and Up.)
    h.state_mut().tree_cursor = Some(TreeAt::Group("NUI windows"));
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(script(39))));
    delete(&mut h, &[script(39)]);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(script(38))));
    let row = h.get_by_label("script_38").rect();
    assert!(row.bottom() <= view_bottom(&h) + 0.5, "in sight: {row:?}");
    // The only area: the group's row.
    let only = ResKey::parse("area_000", ResType::ARE).unwrap();
    key(&mut h, egui::Key::Home);
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(only)));
    delete(&mut h, &[only]);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Group("Areas")));
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Group("Conversations")));
}
