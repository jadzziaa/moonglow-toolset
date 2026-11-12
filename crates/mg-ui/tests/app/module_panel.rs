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
    let ws = h.state().ws.as_ref().unwrap();
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
    // The four, in the board's order, before the fold button.
    let names = ["Module Properties", "New Area", "Expand All", "Collapse All"];
    let lefts: Vec<f32> = names.iter().map(|n| module_tool(&h, n).rect().left()).collect();
    assert!(lefts.windows(2).all(|w| w[0] < w[1]), "in order: {lefts:?}");
    let fold = h.get_by_label("Left Dock").get_by_label("Fold the Left Dock").rect();
    assert!(lefts[3] < fold.left());
    for (name, tip) in [
        ("Module Properties", "Module properties"),
        ("New Area", "Area Wizard"),
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

/// A tab chosen for an area deep in the Areas group brings its row into
/// view inside the well; with the tree scrolled down past the well (the
/// Scripts group open, its rows wheeled to), the tree scrolls back to
/// the well too.
#[test]
fn the_tree_follows_an_area_tab_into_the_well() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    let deep = all[90];
    assert!(!in_well(&h, &deep.to_string()), "out of the well's view to begin with");
    // (As a click on the area's tab asks: `reveal_tab`.)
    h.state_mut().tree_reveal = Some((ResKey::new(deep, ResType::ARE), false));
    h.run_steps(3);
    settle(&mut h);
    assert!(in_well(&h, &deep.to_string()), "brought into the well's view");
    assert_eq!(h.state().tree_area, Some(deep));
    // The tree scrolled down past the well (the Scripts group open, its
    // row wheeled 300 points up), then another area's tab.
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
    let other = all[40];
    h.state_mut().tree_reveal = Some((ResKey::new(other, ResType::ARE), false));
    h.run_steps(3);
    settle(&mut h);
    // The row in sight, under the Filter and above the next group (in
    // the well).
    let row = h.get_by_label("Module Tree Rows").get_by_label(&other.to_string()).rect();
    let next = group(&h, "Conversations").rect();
    assert!(row.top() >= filter.bottom() - 0.5 && row.bottom() <= next.top() + 0.5, "{row:?}");
}

/// A row in the well has its menu (View Area, Properties); a blueprint's
/// row in a group under the well still starts a drag (to drop into an
/// area).
#[test]
fn a_row_in_the_well_has_its_menu_and_a_blueprint_row_drags() {
    let Some(mut h) = chapter1(1000.0, 800.0) else { return };
    let all = areas(&h);
    let area = all[2].to_string();
    h.get_by_label(&area).hover();
    h.run_steps(1);
    h.get_by_label(&area).click_secondary();
    h.run_steps(3);
    assert!(h.query_by_label("View Area").is_some(), "the area's menu");
    assert!(h.query_all_by_label("Properties").count() >= 1);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // A creature's blueprint dragged from its row.
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
