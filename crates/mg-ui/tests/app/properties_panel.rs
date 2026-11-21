//! The Properties panel (M4's U6, scope move S2; the Main, Narrow,
//! WideArea, Floating and AreaPaint boards): what is selected in an area,
//! in the Right Dock's lower section under the Palette and the Preview,
//! with every tool but Select Tiles; the Properties window's pages
//! without OK and Cancel, as a rail; each change applied at once, one
//! Undo step; Open in Tab; a double click (ruling Q1); three tabs under
//! 1400, two columns in the wide layout with their seam dragged; the
//! palette's narrow row. Docked and floating (`Mode`: the area is docked
//! in both, As Tabs and As Floating Windows). (The game's data and a GPU,
//! else skipped.)

use crate::common::*;
use glam::Vec3;
use mg_area::ObjectKind;
use mg_ui::properties_panel::{NOTHING_SELECTED, OPEN_IN_TAB};

/// An area as `area_harness` makes it (two waypoints, at 20 and 30 by 20)
/// with two chests at 12 and 14 by 20, and two of each other kind (a
/// bandit, a store, an encounter, a door, a trigger, a sound, an item), in `mode`, in a window `width` by 900 (1600: the Right
/// Dock's sections stacked, the palettes over Properties).
fn props_area(name: &str, mode: Mode, width: f32) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let chests = |m: &mut Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::instances::{Placement, Placing, instance};
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let list = [12.0, 14.0].map(|x| {
            let at = Placement { position: [x, 20.0, 0.0], rotation: 0.0 };
            instance(&placing, ResType::UTP, &bp, at, &[]).unwrap()
        });
        let one = |t: ResType, resref: &str, x: f32| {
            let key = ResKey::parse(resref, t).unwrap();
            let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
            let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
            instance(&placing, t, &bp, at, &[]).unwrap()
        };
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        git.root.set("Placeable List", mg_gff::Value::List(list.to_vec()));
        let others = [
            (ObjectKind::Creature.list(), ResType::UTC, "nw_bandit001"),
            (ObjectKind::Store.list(), ResType::UTM, "x2_storethief001"),
            (ObjectKind::Encounter.list(), ResType::UTE, "nw_giantevil"),
            (ObjectKind::Door.list(), ResType::UTD, "nw_door_normal"),
            (ObjectKind::Trigger.list(), ResType::UTT, "newgeneric"),
            (ObjectKind::Sound.list(), ResType::UTS, "animalcriesday"),
            (ObjectKind::Item.list(), ResType::UTI, "nw_wswls001"),
        ];
        // (Two of each: edited together, as two chests are.)
        for (i, (list, t, resref)) in others.into_iter().enumerate() {
            let x = 12.0 + 4.0 * i as f32;
            let two = vec![one(t, resref, x), one(t, resref, x + 1.5)];
            git.root.set(list, mg_gff::Value::List(two));
        }
        m.set_gff(git_key, &git).unwrap();
    };
    let (mut app, rs, area) = area_app(&mode.dir(name), "ttr01", None, 4, None, &chests)?;
    put_in(&mut app, mode);
    let mut h = area_gpu_harness(app, rs, width, 900.0);
    h.run_steps(3);
    settle(&mut h);
    Some((h, area))
}

fn select(h: &mut Harness<'_, Moonglow>, area: ResRef, objects: &[(ObjectKind, usize)]) {
    h.state_mut().area_views.get_mut(&area).unwrap().selection = objects.to_vec();
    settle(h);
}

fn placeables(h: &mut Harness<'_, Moonglow>, area: ResRef) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    git.root.list("Placeable List").unwrap().to_vec()
}

fn tag(s: &mg_gff::Struct) -> String {
    String::from_utf8_lossy(s.string("Tag").unwrap()).into_owned()
}

fn revision(h: &Harness<'_, Moonglow>) -> u64 {
    h.state().ws.as_ref().unwrap().revision()
}

fn undo_label(h: &Harness<'_, Moonglow>) -> Option<String> {
    h.state().ws.as_ref().unwrap().can_undo().map(str::to_string)
}

fn is_input(n: &egui_kittest::Node<'_>) -> bool {
    n.accesskit_node().role() == egui::accesskit::Role::TextInput
}

/// The left edge of the page rail (its Basic page's button, 24 wide,
/// centered in its 31 points).
fn rail_left(h: &Harness<'_, Moonglow>) -> f32 {
    h.get_by_label("Pages").get_by_label("Basic").rect().left() - 3.5
}

/// How many named controls the page `page` draws in the Right Dock under
/// its heading, in sight or scrolled out of it (0: no heading, the page
/// not drawn).
fn page_controls(h: &Harness<'_, Moonglow>, page: &str) -> usize {
    let dock = right_dock_rect(h);
    let rect = |n: &egui_kittest::Node<'_>| {
        let b = n.accesskit_node().bounding_box()?;
        Some(egui::Rect::from_min_max(
            egui::pos2(b.x0 as f32, b.y0 as f32),
            egui::pos2(b.x1 as f32, b.y1 as f32),
        ))
    };
    let wide =
        |r: &egui::Rect| dock.contains(r.center()) && r.width() > mg_uikit::pages::PAGE_BUTTON;
    let Some(heading) = h.query_all_by_label(page).filter_map(|n| rect(&n)).find(wide) else {
        return 0;
    };
    let under = |n: &egui_kittest::Node<'_>| {
        let node = n.accesskit_node();
        let named = node.label().or_else(|| node.value()).is_some_and(|l| !l.trim().is_empty());
        let leaf = node.children().next().is_none();
        named
            && leaf
            && rect(n)
                .is_some_and(|r| r.top() > heading.bottom() && dock.x_range().contains(r.left()))
    };
    h.query_all(egui_kittest::kittest::by()).filter(under).count()
}

/// Types `text` over the text field showing `value`, and leaves it (Tab).
fn retype(h: &mut Harness<'_, Moonglow>, value: &str, text: &str) {
    h.get_all_by_value(value).find(is_input).expect("the field").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(value).find(is_input).unwrap().type_text(text);
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
}

/// The panel follows the selection: with nothing selected it says how to
/// choose; one placeable, its kind in the header ("Properties,
/// Placeable"), its pages as the rail's buttons, Basic in front, its
/// fields; two of one kind, "2 placeables"; a placeable and a waypoint,
/// said and no fields. No OK or Cancel anywhere in it.
#[test]
fn the_panel_shows_the_selection() {
    for mode in MODES {
        let Some((mut h, area)) = props_area("props-shows", mode, 1600.0) else { return };
        let w = mode.word();
        h.get_by_label("Properties, none selected");
        h.get_by_label(NOTHING_SELECTED);
        select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
        h.get_by_label("Properties, Placeable");
        let rail = h.get_by_label("Pages");
        for page in ["Basic", "Inventory", "Lock", "Trap", "Scripts", "Variables"] {
            rail.get_by_label(page);
        }
        assert!(rail.query_by_label("Comments").is_none(), "{w}: the blueprint's alone");
        let basic = rail.get_by_label("Basic").accesskit_node().toggled();
        assert_eq!(basic, Some(egui::accesskit::Toggled::True), "{w}: Basic in front");
        let chest = tag(&placeables(&mut h, area)[0]);
        assert!(h.get_all_by_value(&chest).any(|n| is_input(&n)), "{w}: its tag's field");
        assert!(h.query_by_label_contains("plc_chest1 at 12.00, 20.00").is_some(), "{w}: where");
        for gone in ["OK", "Cancel"] {
            assert!(h.query_by_label(gone).is_none(), "{w}: no {gone}");
        }
        select(&mut h, area, &[(ObjectKind::Placeable, 0), (ObjectKind::Placeable, 1)]);
        h.get_by_label("Properties, 2 placeables");
        select(&mut h, area, &[(ObjectKind::Placeable, 0), (ObjectKind::Waypoint, 1)]);
        h.get_by_label("Properties, 2 objects");
        h.get_by_label_contains("2 objects of 2 types are selected");
        assert!(h.query_by_label("Pages").is_none(), "{w}: no pages for a mixed selection");
    }
}

/// A change in the panel is the module's at once, one step, which Undo
/// takes back; the panel shown, each of its pages, and other objects
/// selected write nothing.
#[test]
fn a_change_applies_at_once_and_undo_takes_it_back() {
    for mode in MODES {
        let Some((mut h, area)) = props_area("props-change", mode, 1600.0) else { return };
        let w = mode.word();
        let (at, label) = (revision(&h), undo_label(&h));
        select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
        for page in ["Inventory", "Lock", "Trap", "Scripts", "Advanced", "Variables", "Basic"] {
            h.get_by_label("Pages").get_by_label(page).click();
            settle(&mut h);
        }
        select(&mut h, area, &[(ObjectKind::Placeable, 0), (ObjectKind::Placeable, 1)]);
        select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
        assert_eq!((revision(&h), undo_label(&h)), (at, label.clone()), "{w}: nothing written");
        let before = placeables(&mut h, area);
        retype(&mut h, &tag(&before[0]), "CHEST_PANEL");
        let now = placeables(&mut h, area);
        assert_eq!(tag(&now[0]), "CHEST_PANEL", "{w}: written at once");
        assert_eq!(now[1], before[1], "{w}: the other untouched");
        assert!(undo_label(&h).is_some_and(|l| l != label.as_deref().unwrap_or("")), "{w}");
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        settle(&mut h);
        assert_eq!(placeables(&mut h, area), before, "{w}: one step, undone byte for byte");
        assert_eq!(undo_label(&h), label, "{w}");
    }
}

/// Two placeables selected are edited together: a tag typed is set on
/// both, as one step.
#[test]
fn several_of_one_type_edit_together() {
    for mode in MODES {
        let Some((mut h, area)) = props_area("props-several", mode, 1600.0) else { return };
        let w = mode.word();
        let both = [(ObjectKind::Placeable, 0), (ObjectKind::Placeable, 1)];
        select(&mut h, area, &both);
        let before = placeables(&mut h, area);
        let label = undo_label(&h);
        retype(&mut h, &tag(&before[0]), "CHEST_BOTH");
        let now = placeables(&mut h, area);
        assert_eq!((tag(&now[0]), tag(&now[1])), ("CHEST_BOTH".into(), "CHEST_BOTH".into()), "{w}");
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        settle(&mut h);
        assert_eq!(placeables(&mut h, area), before, "{w}: one step for both");
        assert_eq!(undo_label(&h), label, "{w}");
    }
}

/// Under 1400 the sections fold into one tab bar, Palette, Properties,
/// Preview (Narrow), the palette's types a dropdown with Standard and
/// Custom beside it and Expand All and Collapse All at the Group by
/// row's end; Properties chosen, its detail and Open in Tab at its own
/// top.
#[test]
fn the_narrow_dock_has_three_tabs_and_the_narrow_row() {
    for mode in MODES {
        let Some((mut h, area)) = props_area("props-narrow", mode, 1280.0) else { return };
        let w = mode.word();
        h.state_mut().palette.tiles = false;
        h.state_mut().palette.kind = mg_module::palette::BlueprintKind::Placeable;
        settle(&mut h);
        let dock = h.get_by_label("Right Dock");
        let tabs: Vec<(String, f32)> = ["Palette", "Properties", "Preview"]
            .iter()
            .map(|t| (t.to_string(), dock.get_by_label(t).rect().left()))
            .collect();
        assert!(tabs.windows(2).all(|p| p[0].1 < p[1].1), "{w}: in this order: {tabs:?}");
        // The palette's narrow row.
        let types = palette_types(&h).expect("the types").rect();
        let standard = h.get_by_label("Standard").rect();
        assert!((standard.center().y - types.center().y).abs() < 4.0, "{w}: Standard beside");
        assert!(standard.left() > types.right(), "{w}: after the dropdown");
        let group = h.get_by_label("Group by").rect();
        let expand = h.get_by_label("Expand All").rect();
        assert!((expand.center().y - group.center().y).abs() < 4.0, "{w}: on Group by's row");
        assert!(expand.left() > group.right() + 60.0, "{w}: at its end");
        let dock = right_dock_rect(&h);
        let in_dock = |n: &egui_kittest::Node<'_>| dock.contains(n.rect().center());
        let more = h.get_all_by_label("More").find(in_dock).expect("the palette's More").rect();
        let items = h.get_by_label_contains(" items").rect();
        assert!((more.center().y - items.center().y).abs() < 4.0, "{w}: the toolbar one row");
        select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
        h.get_by_label("Right Dock").get_by_label("Properties").click();
        settle(&mut h);
        h.get_by_label("Pages");
        h.get_by_label(OPEN_IN_TAB);
        h.get_by_label("Placeable");
        // Remembered (the owner's ruling of U2's review), the module
        // opened again: Properties in front of the one bar.
        let path = h.state().module_path().expect("the module's path");
        h.state_mut().open_module(&path);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        settle(&mut h);
        let shows = h.state().right_dock_shows();
        assert_eq!(shows, Some(mg_ui::Panel::Properties), "{w}: after reopening");
    }
}

/// In the wide layout the Palette and the Preview stand in the first
/// column and Properties in the second (WideArea); the seam between them
/// is dragged, kept for the area's kind, and held at 280 for either.
#[test]
fn the_wide_layout_s_columns_and_their_seam() {
    let Some((mut h, area)) = props_area("props-wide", Mode::Tabs, 1600.0) else { return };
    h.set_size(egui::vec2(3440.0, 1440.0));
    settle(&mut h);
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    let dock = right_dock_rect(&h);
    let palette = h.get_by_label("Right Dock").get_by_label("Palette").rect();
    let preview = h.get_by_label("Right Dock").get_by_label("Preview").rect();
    assert!((palette.top() - preview.top()).abs() < 1.0, "Palette and Preview one bar");
    let first = rail_left(&h) - dock.left();
    assert!((339.0..=342.0).contains(&first), "Properties in the second column: {first}");
    // The seam dragged 100 to the right.
    let x = dock.left() + 340.5;
    let y = dock.center().y;
    h.hover_at(egui::pos2(x, y));
    h.drag_at(egui::pos2(x, y));
    for k in 1..=5 {
        h.hover_at(egui::pos2(x + 20.0 * k as f32, y));
        h.run_steps(1);
    }
    h.drop_at(egui::pos2(x + 100.0, y));
    settle(&mut h);
    let kept = h.state().settings.dock_layouts.get("area").and_then(|l| l.column);
    assert_eq!(kept, Some(440), "kept for the area");
    let moved = rail_left(&h) - dock.left();
    assert!((439.0..=442.0).contains(&moved), "drawn there: {moved}");
    // Held at 280 for the second column, past its end.
    let x = dock.left() + 440.5;
    h.hover_at(egui::pos2(x, y));
    h.drag_at(egui::pos2(x, y));
    for k in 1..=5 {
        h.hover_at(egui::pos2(x + 200.0 * k as f32, y));
        h.run_steps(1);
    }
    h.drop_at(egui::pos2(x + 1000.0, y));
    settle(&mut h);
    let kept = h.state().settings.dock_layouts.get("area").and_then(|l| l.column).unwrap();
    let second = dock.width() - 1.0 - f32::from(kept);
    assert!((280.0..=282.0).contains(&second), "the second at least 280: {second}");
}

/// A double click on a placed object (ruling Q1) shows the Properties
/// panel in front, the Right Dock unfolded, in either mode, and opens no
/// window; Open in Tab opens today's window (OK and Cancel), and the panel
/// then leaves the fields to it, so that a tag typed there is one command.
#[test]
fn a_double_click_shows_the_panel_and_open_in_tab_the_window() {
    for mode in MODES {
        let Some((mut h, area)) = props_area("props-double", mode, 1280.0) else { return };
        let w = mode.word();
        h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Palette));
        h.state_mut().settings.hide_palettes = true;
        settle(&mut h);
        h.run_steps(40);
        let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
        for _ in 0..2 {
            press(&h, at, true, egui::Modifiers::NONE);
            press(&h, at, false, egui::Modifiers::NONE);
        }
        settle(&mut h);
        let path = mg_edit::GffPath::root().item("WaypointList", 1);
        let window = Tab::Instance { area, path };
        assert!(h.state().dock.find_tab(&window).is_none(), "{w}: no window");
        assert!(!h.state().settings.hide_palettes, "{w}: the Right Dock unfolded");
        assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 1)], "{w}");
        // (Under 1400 a tab of the one bar: in front, its detail at its
        // top.)
        assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Properties), "{w}: in front");
        h.get_by_label("Pages");
        h.get_by_label("Waypoint");
        h.get_by_label(OPEN_IN_TAB).click();
        settle(&mut h);
        assert!(h.state().dock.find_tab(&window).is_some(), "{w}: the window, by Open in Tab");
        h.get_by_label("OK");
        // (As Floating Windows the area stays the docked editor in front,
        // its panel saying where the fields are; As Tabs the window is the
        // tab in front, which brings no Right Dock.)
        match mode {
            Mode::Floating => {
                h.get_by_label_contains("open in a tab or window of their own");
            }
            Mode::Tabs => assert_eq!(h.state().right_dock_shows(), None, "{w}"),
        }
        let label = undo_label(&h);
        let at = revision(&h);
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let old = tag(&git.root.list("WaypointList").unwrap()[1]);
        retype(&mut h, &old, "WP_WINDOW");
        assert_eq!(revision(&h), at + 1, "{w}: one command");
        assert_ne!(undo_label(&h), label, "{w}");
    }
}

/// Text typed in the panel and left by selecting another object (a click
/// in the view) is set on the object it was typed for.
#[test]
fn text_typed_goes_to_its_own_object_when_another_is_selected() {
    let Some((mut h, area)) = props_area("props-typed", Mode::Floating, 1600.0) else { return };
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    let before = placeables(&mut h, area);
    let old = tag(&before[0]);
    h.get_all_by_value(&old).find(is_input).expect("the field").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&old).find(is_input).unwrap().type_text("CHEST_FIRST");
    h.run_steps(2);
    select(&mut h, area, &[(ObjectKind::Placeable, 1)]);
    let now = placeables(&mut h, area);
    assert_eq!(tag(&now[0]), "CHEST_FIRST", "the first's");
    assert_eq!(tag(&now[1]), tag(&before[1]), "not the second's");
}

/// In an area another builder holds (a project that keeps those
/// read-only), the panel's fields are only looked at: said, and a tag
/// typed changes nothing.
#[test]
fn a_read_only_area_s_properties_are_looked_at() {
    let Some((mut h, area)) = read_only_area_harness("props-read-only") else { return };
    wide_window(&mut h);
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    h.get_by_label("Properties, Placeable");
    h.get_by_label_contains("read-only here, shown to look at");
    let before = placeables(&mut h, area);
    let old = tag(&before[0]);
    // (Its field there, only looked at: typing into it changes nothing.)
    let field = h.get_all_by_value(&old).find(is_input).expect("its tag's field").rect();
    click_pos(&h, field.center());
    h.run_steps(2);
    h.get_all_by_value(&old).find(is_input).unwrap().type_text("NOT_MINE");
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert_eq!(placeables(&mut h, area), before, "nothing changed");
}

/// F6 stops at the Properties panel (after the palettes' section), its
/// page shown taking the keyboard's focus; the keys typed there act on
/// the field, not on the object (Delete deletes nothing).
#[test]
fn f6_reaches_the_properties_panel() {
    for mode in MODES {
        let Some((mut h, area)) = props_area("props-f6", mode, 1600.0) else { return };
        let w = mode.word();
        select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
        let mut stops = Vec::new();
        for _ in 0..8 {
            h.key_press(egui::Key::F6);
            settle(&mut h);
            let props = mg_ui::palette_view::arrow_keys_are(&h.ctx) == "Properties";
            stops.push(props);
            if props {
                break;
            }
        }
        assert!(stops.last() == Some(&true), "{w}: no Properties stop: {stops:?}");
        let focused = h.get_by_label("Pages").get_by_label("Basic");
        assert!(focused.accesskit_node().is_focused(), "{w}: its page shown has the focus");
        let before = placeables(&mut h, area);
        let old = tag(&before[0]);
        h.get_all_by_value(&old).find(is_input).unwrap().click();
        h.run_steps(2);
        // (The pointer then over the area's view, where Delete would
        // delete the selection but for the field typed in.)
        h.hover_at(h.state().area_views[&area].rect.center());
        h.run_steps(2);
        h.key_press(egui::Key::Delete);
        h.key_press(egui::Key::Q);
        settle(&mut h);
        // (Typed into the field, not taken by the view: nothing deleted or
        // turned, the field not left.)
        assert_eq!(placeables(&mut h, area), before, "{w}: nothing deleted or turned");
    }
}

/// The lower section goes by the tool in hand: Properties with Select
/// Objects and Paint, Tile Properties with Select Tiles (and neither
/// while its header toggle hides it), Properties again with Select
/// Objects.
#[test]
fn the_tool_in_hand_chooses_the_lower_section() {
    let Some((mut h, area)) = props_area("props-tools", Mode::Floating, 1600.0) else { return };
    let lower = |h: &Harness<'_, Moonglow>| {
        let props = (h.query_all_by_label_contains("Properties, "))
            .filter_map(|n| n.accesskit_node().label())
            .find(|l| !l.starts_with("Tile"));
        let tiles = h.query_by_label_contains("Tile Properties, ").is_some();
        (props, tiles)
    };
    assert_eq!(lower(&h), (Some("Properties, none selected".into()), false), "Select Objects");
    h.state_mut().paint = true;
    settle(&mut h);
    assert!(lower(&h).0.is_some(), "Paint: Properties, as AreaPaint draws it");
    h.state_mut().paint = false;
    settle(&mut h);
    area_click(&mut h, "Select Tiles");
    settle(&mut h);
    assert_eq!(lower(&h), (None, true), "Select Tiles: Tile Properties in its place");
    h.state_mut().settings.hide_tile_properties = true;
    settle(&mut h);
    assert_eq!(lower(&h), (None, false), "hidden by its toggle: neither");
    h.state_mut().settings.hide_tile_properties = false;
    area_click(&mut h, "Select Objects");
    settle(&mut h);
    assert!(lower(&h).0.is_some(), "Select Objects again: Properties");
    let _ = area;
}

/// A section dragged as low as it goes ([`SECTION_LEAST`], 80) keeps
/// every page in reach: the rail scrolls under the wheel to its last.
#[test]
fn the_rail_reaches_every_page_in_a_low_section() {
    let Some((mut h, area)) = props_area("props-low", Mode::Tabs, 1600.0) else { return };
    let layout = h.state_mut().settings.dock_layouts.entry("area".into()).or_default();
    layout.shares = vec![1900, 100];
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    let dock = right_dock_rect(&h);
    let last = h.get_by_label("Pages").get_by_label("Description").rect();
    assert!(last.top() > dock.bottom(), "out of sight at first: {last:?} {dock:?}");
    let basic = h.get_by_label("Pages").get_by_label("Basic").rect();
    h.hover_at(basic.center());
    for _ in 0..20 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -40.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
    }
    settle(&mut h);
    let last = h.get_by_label("Pages").get_by_label("Description").rect();
    assert!(last.bottom() <= dock.bottom() + 0.5, "in reach: {last:?} in {dock:?}");
    h.get_by_label("Pages").get_by_label("Description").click();
    settle(&mut h);
    let shown = h.get_by_label("Pages").get_by_label("Description").accesskit_node().toggled();
    assert_eq!(shown, Some(egui::accesskit::Toggled::True), "chosen");
    assert!(page_controls(&h, "Description") >= 2, "the Description page drawn");
}

/// The panel's controls draw nothing over each other in the Right Dock at
/// its least, 280 (a window under 1400, Properties chosen) and stacked at
/// 280, on each of a placeable's pages.
#[test]
fn the_panel_draws_nothing_over_anything_else_at_280() {
    let Some((mut h, area)) = props_area("props-overlap", Mode::Floating, 1280.0) else { return };
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    h.get_by_label("Right Dock").get_by_label("Properties").click();
    settle(&mut h);
    let mut found = Vec::new();
    for width in [1280.0, 1600.0] {
        h.set_size(egui::vec2(width, 900.0));
        right_dock_width(&mut h, 280.0);
        settle(&mut h);
        for page in ["Basic", "Inventory", "Lock", "Trap", "Scripts", "Advanced", "Visuals"] {
            h.get_by_label("Pages").get_by_label(page).click();
            settle(&mut h);
            if page_controls(&h, page) < 2 {
                found.push(format!("{page} at {width}: the page not drawn"));
            }
            // (Of the Right Dock's widgets: both of a pair named there.)
            let dock = right_dock_rect(&h);
            let named: Vec<String> = (h.query_all(egui_kittest::kittest::by()))
                .filter(|n| {
                    let b = n.accesskit_node().bounding_box();
                    b.is_some_and(|b| dock.contains(egui::pos2(b.x0 as f32, b.y0 as f32)))
                })
                .filter_map(|n| n.accesskit_node().label())
                .collect();
            let ours = |o: &String| {
                let quoted: Vec<&str> = o.split('"').skip(1).step_by(2).collect();
                quoted.iter().all(|q| named.iter().any(|n| n == q))
            };
            let over = overlapping(&h).into_iter().filter(ours);
            found.extend(over.map(|o| format!("{page} at {width}: {o}")));
        }
    }
    assert!(found.is_empty(), "drawn over each other:\n{}", found.join("\n"));
}

/// A drag on one of the panel's number fields (a chest's hit points)
/// moves nothing in the area beside it (deferred: "A drag on a number
/// field moving objects in the area behind", with the panel).
#[test]
fn a_drag_on_a_number_field_in_the_panel_moves_nothing_in_the_area() {
    let Some((mut h, area)) = props_area("props-drag", Mode::Floating, 1600.0) else { return };
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    let before = (waypoint(&mut h, area, 0), waypoint(&mut h, area, 1));
    let chests: Vec<(f32, f32)> = placeables(&mut h, area)
        .iter()
        .map(|p| (p.float("X").unwrap_or(0.0), p.float("Y").unwrap_or(0.0)))
        .collect();
    let field = h
        .get_all_by_role(egui::accesskit::Role::SpinButton)
        .next()
        .expect("a number field in the panel")
        .rect();
    let from = field.center();
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=6 {
        h.hover_at(from + egui::vec2(-60.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    press(&h, from + egui::vec2(-360.0, 0.0), false, egui::Modifiers::NONE);
    settle(&mut h);
    assert_eq!((waypoint(&mut h, area, 0), waypoint(&mut h, area, 1)), before);
    let now: Vec<(f32, f32)> = placeables(&mut h, area)
        .iter()
        .map(|p| (p.float("X").unwrap_or(0.0), p.float("Y").unwrap_or(0.0)))
        .collect();
    assert_eq!(now, chests, "the chests stay where they are");
}

/// Every kind's pages in the panel (a placeable's, a creature's, a
/// store's, an encounter's): shown one by one, at 280 (under 1400,
/// Properties chosen) and stacked at 320, they write nothing and draw
/// nothing over each other in the dock. Picture of the creature's
/// Inventory page at 320: `ui-area-props-kinds-floating/inventory.png`.
#[test]
fn every_kind_s_pages_write_nothing_and_fit() {
    let Some((mut h, area)) = props_area("props-kinds", Mode::Floating, 1280.0) else { return };
    h.get_by_label("Right Dock").get_by_label("Properties").click();
    settle(&mut h);
    let (at, label) = (revision(&h), undo_label(&h));
    let mut found = Vec::new();
    for (width, dock) in [(1280.0, 280.0), (1600.0, 320.0)] {
        h.set_size(egui::vec2(width, 900.0));
        right_dock_width(&mut h, dock);
        settle(&mut h);
        for (kind, t) in [
            (ObjectKind::Placeable, ResType::UTP),
            (ObjectKind::Creature, ResType::UTC),
            (ObjectKind::Store, ResType::UTM),
            (ObjectKind::Encounter, ResType::UTE),
        ] {
            select(&mut h, area, &[(kind, 0)]);
            for page in mg_ui::blueprint::instance_pages(t) {
                h.get_by_label("Pages").get_by_label(page).scroll_to_me();
                settle(&mut h);
                h.get_by_label("Pages").get_by_label(page).click();
                settle(&mut h);
                if page_controls(&h, page) < 2 {
                    found.push(format!("{kind:?} {page} at {width}: the page not drawn"));
                }
                let dock = right_dock_rect(&h);
                let named: Vec<String> = (h.query_all(egui_kittest::kittest::by()))
                    .filter(|n| {
                        let b = n.accesskit_node().bounding_box();
                        b.is_some_and(|b| dock.contains(egui::pos2(b.x0 as f32, b.y0 as f32)))
                    })
                    .filter_map(|n| n.accesskit_node().label())
                    .collect();
                let ours = |o: &String| {
                    let quoted: Vec<&str> = o.split('"').skip(1).step_by(2).collect();
                    quoted.iter().all(|q| named.iter().any(|n| n == q))
                };
                let over = overlapping(&h).into_iter().filter(ours);
                found.extend(over.map(|o| format!("{kind:?} {page} at {width}: {o}")));
                if kind == ObjectKind::Creature && page == "Inventory" && width == 1600.0 {
                    let dir = mg_testkit::test_output().join("ui-area-props-kinds-floating");
                    let _ = std::fs::create_dir_all(&dir);
                    h.render().unwrap().save(dir.join("inventory.png")).unwrap();
                }
            }
        }
    }
    assert_eq!((revision(&h), undo_label(&h)), (at, label), "shown: nothing written");
    assert!(found.is_empty(), "drawn over each other:\n{}", found.join("\n"));
}

/// An area dragged out into a window of its own has Properties in its
/// window's right pane (the main Right Dock its palettes): a change there
/// is one step; F6 stops there after the area's view; Open in Tab opens
/// the window, and the pane then leaves the fields to it.
#[test]
fn a_dragged_out_area_has_properties_in_its_window() {
    let Some((mut h, area)) = props_area("props-window", Mode::Floating, 1700.0) else { return };
    own_window(&mut h, &Tab::Area(area), egui::pos2(20.0, 40.0), egui::vec2(1200.0, 800.0));
    settle(&mut h);
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    let window = egui::Rect::from_min_size(egui::pos2(20.0, 40.0), egui::vec2(1200.0, 800.0));
    let basics: Vec<egui::Rect> = (h.query_all_by_label("Basic"))
        .map(|n| n.rect())
        // (The rail's: 24 square; Module Properties' page row has one too.)
        .filter(|r| r.width() == mg_uikit::pages::PAGE_BUTTON)
        .collect();
    assert_eq!(basics.len(), 1, "one panel: {basics:?}");
    assert!(window.contains_rect(basics[0]), "in the area's window: {basics:?}");
    let view = h.state().area_views[&area].rect;
    assert!(basics[0].left() >= view.right() - 1.0, "at its right: {basics:?} {view:?}");
    let before = placeables(&mut h, area);
    let label = undo_label(&h);
    retype(&mut h, &tag(&before[0]), "CHEST_WINDOW");
    assert_eq!(tag(&placeables(&mut h, area)[0]), "CHEST_WINDOW", "written");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(placeables(&mut h, area), before, "one step");
    assert_eq!(undo_label(&h), label);
    let mut stops = Vec::new();
    for _ in 0..10 {
        h.key_press(egui::Key::F6);
        settle(&mut h);
        stops.push(mg_ui::palette_view::arrow_keys_are(&h.ctx));
        if stops.last().map(String::as_str) == Some("Properties") {
            break;
        }
    }
    assert_eq!(stops.last().map(String::as_str), Some("Properties"), "F6: {stops:?}");
    h.get_by_label(OPEN_IN_TAB).click();
    settle(&mut h);
    let path = mg_edit::GffPath::root().item("Placeable List", 0);
    assert!(h.state().dock.find_tab(&Tab::Instance { area, path }).is_some(), "the window");
    h.get_by_label_contains("open in a tab or window of their own");
}

/// Every kind of placed object, each of its pages in the panel, at the
/// Right Dock's 280, 320 and 400 (a window under 1400, Properties chosen;
/// then stacked from 1600): no control reaches past the dock's right
/// edge (the review of U6 found the window's form, its fields 240 wide,
/// run past it at every width the panel has), none lies over another, and
/// each page draws its fields (a page's own named controls under its
/// heading).
#[test]
fn every_page_of_every_kind_fits_the_panel() {
    let Some((mut h, area)) = props_area("props-fit", Mode::Floating, 1280.0) else { return };
    // (The pictures in the light theme too, with `MG_LIGHT`.)
    if std::env::var_os("MG_LIGHT").is_some() {
        h.state_mut().settings.light_theme = true;
        settle(&mut h);
    }
    h.get_by_label("Right Dock").get_by_label("Properties").click();
    settle(&mut h);
    let mut found = Vec::new();
    // (One of each kind at every width; two edited together at the narrow
    // ones, where the pages of lists edited together are drawn.)
    let sizes = [
        (1280.0, 280.0, false),
        (1600.0, 320.0, false),
        (1600.0, 400.0, false),
        (1600.0, 480.0, false),
        (1600.0, 560.0, false),
        (1600.0, 640.0, false),
        (1280.0, 280.0, true),
        (1600.0, 400.0, true),
    ];
    for (width, dock, two) in sizes {
        h.set_size(egui::vec2(width, 900.0));
        right_dock_width(&mut h, dock);
        settle(&mut h);
        for kind in mg_area::ObjectKind::ALL {
            let pages = mg_ui::blueprint::instance_pages(kind.restype());
            let objects: &[(ObjectKind, usize)] =
                if two { &[(kind, 0), (kind, 1)] } else { &[(kind, 0)] };
            select(&mut h, area, objects);
            for page in pages {
                // (The rail scrolls where its pages are more than it holds.)
                h.get_by_label("Pages").get_by_label(page).scroll_to_me();
                settle(&mut h);
                h.get_by_label("Pages").get_by_label(page).click();
                settle(&mut h);
                let dock = right_dock_rect(&h);
                // (Each page's picture, the dock alone: `props-fit/`.)
                if std::env::var_os("MG_PAGE_SHOTS").is_some() {
                    let theme =
                        if std::env::var_os("MG_LIGHT").is_some() { "light" } else { "dark" };
                    let dir = mg_testkit::test_output().join("props-fit").join(theme);
                    let _ = std::fs::create_dir_all(&dir);
                    let img = h.render().unwrap();
                    let ppp = h.ctx.pixels_per_point();
                    let x = (dock.left() * ppp) as u32;
                    let w = ((dock.width() * ppp) as u32).min(img.width() - x);
                    let cut = image::imageops::crop_imm(&img, x, 0, w, img.height()).to_image();
                    let several = if two { "x2" } else { "" };
                    let page_name = page.replace(' ', "_");
                    let name = format!("{:.0}{several}-{kind:?}-{page_name}.png", dock.width());
                    cut.save(dir.join(name)).unwrap();
                }
                let heading = h.get_all_by_label(page).map(|n| n.rect()).find(|r| {
                    dock.contains(r.center()) && r.width() > mg_uikit::pages::PAGE_BUTTON
                });
                let Some(heading) = heading else {
                    found.push(format!(
                        "{kind:?} {page} at {dock_w}: no heading",
                        dock_w = dock.width()
                    ));
                    continue;
                };
                // (Named controls of the page, under its heading.)
                let mut drawn = 0;
                for n in h.query_all(egui_kittest::kittest::by()) {
                    let node = n.accesskit_node();
                    let Some(b) = node.bounding_box() else { continue };
                    let r = egui::Rect::from_min_max(
                        egui::pos2(b.x0 as f32, b.y0 as f32),
                        egui::pos2(b.x1 as f32, b.y1 as f32),
                    );
                    if r.top() <= heading.bottom()
                        || r.left() < dock.left()
                        || r.top() > dock.bottom()
                    {
                        continue;
                    }
                    let name = node.label().or_else(|| node.value()).unwrap_or_default();
                    // (A field counts empty too: by its role.)
                    use egui::accesskit::Role;
                    let field =
                        matches!(node.role(), Role::TextInput | Role::ComboBox | Role::SpinButton);
                    if (name.trim().is_empty() && !field) || node.children().next().is_some() {
                        continue;
                    }
                    drawn += 1;
                    if r.right() > dock.right() + 0.5 {
                        found.push(format!(
                            "{kind:?} {page} at {:.0}: {:?} {name:?} ({:.0}..{:.0}) past {:.0}",
                            dock.width(),
                            node.role(),
                            r.left(),
                            r.right(),
                            dock.right()
                        ));
                    }
                }
                if drawn < 2 {
                    found.push(format!("{kind:?} {page} at {:.0}: nothing drawn", dock.width()));
                }
                // (Nor over each other: a row wrapped in a cell of a form's
                // grid ran into the row under it.)
                let named: Vec<String> = (h.query_all(egui_kittest::kittest::by()))
                    .filter(|n| {
                        let b = n.accesskit_node().bounding_box();
                        b.is_some_and(|b| dock.contains(egui::pos2(b.x0 as f32, b.y0 as f32)))
                    })
                    .filter_map(|n| n.accesskit_node().label())
                    .collect();
                let ours = |o: &String| {
                    let quoted: Vec<&str> = o.split('"').skip(1).step_by(2).collect();
                    quoted.iter().all(|q| named.iter().any(|n| n == q))
                };
                let over = overlapping(&h).into_iter().filter(ours);
                found.extend(over.map(|o| format!("{kind:?} {page} at {:.0}: {o}", dock.width())));
            }
        }
    }
    assert!(found.is_empty(), "{} found:\n{}", found.len(), found.join("\n"));
}

/// A dragged out area's window too narrow for its view and the panel's
/// pane (the review of U6: views of 120 and 32 points in windows 450 and
/// 360 wide) keeps its view: the pane goes under the view's least (400
/// beside the pane's 280), and comes back as the window is widened.
#[test]
fn a_small_area_window_keeps_its_view() {
    let Some((mut h, area)) = props_area("props-small", Mode::Floating, 1700.0) else { return };
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    own_window(&mut h, &Tab::Area(area), egui::pos2(20.0, 40.0), egui::vec2(1200.0, 700.0));
    settle(&mut h);
    let rail = |h: &Harness<'_, Moonglow>| {
        (h.query_all_by_label("Basic")).any(|n| n.rect().width() == mg_uikit::pages::PAGE_BUTTON)
    };
    let mut seen = Vec::new();
    for width in [1200.0, 700.0, 450.0, 360.0, 1200.0] {
        let found = h.state().dock.find_tab(&Tab::Area(area)).unwrap();
        let surface = found.surface;
        let state = h.state_mut().dock.get_window_state_mut(surface).unwrap();
        state.set_size(egui::vec2(width, 700.0));
        settle(&mut h);
        let view = h.state().area_views[&area].rect.width();
        seen.push((width, view.round(), rail(&h)));
    }
    for &(width, view, pane) in &seen {
        // (The window's frame and the tool shelf aside: the view as wide as
        // a small window has it, and at least 400 where the pane is beside.)
        assert!(view >= 400.0f32.min(width - 60.0), "{width}: the view {view}: {seen:?}");
        assert!(!pane || view >= 400.0, "{width}: the pane over the view's least: {seen:?}");
    }
    assert!(seen[0].2 && seen[4].2, "the pane in a wide window, and back: {seen:?}");
    assert!(!seen[2].2 && !seen[3].2, "no pane in the narrow ones: {seen:?}");
    // A double click there, with no panel to show it: the window.
    let found = h.state().dock.find_tab(&Tab::Area(area)).unwrap();
    let state = h.state_mut().dock.get_window_state_mut(found.surface).unwrap();
    state.set_size(egui::vec2(450.0, 700.0));
    settle(&mut h);
    h.run_steps(40);
    let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
    }
    settle(&mut h);
    let path = mg_edit::GffPath::root().item("WaypointList", 1);
    assert!(h.state().dock.find_tab(&Tab::Instance { area, path }).is_some(), "the window");
}

/// A double click with Ctrl held (the first click selects, the second,
/// with Ctrl, would take it out again) keeps the object selected and
/// shows it in the panel (ruling Q1: a double click shows its
/// Properties, whatever the modifier).
#[test]
fn a_ctrl_double_click_keeps_the_object_in_the_panel() {
    let Some((mut h, area)) = props_area("props-ctrl-double", Mode::Floating, 1600.0) else {
        return;
    };
    h.run_steps(40);
    let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::COMMAND);
        press(&h, at, false, egui::Modifiers::COMMAND);
    }
    settle(&mut h);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 1)], "selected");
    h.get_by_label("Properties, Waypoint");
}

/// Text typed in the panel's field and an earlier object of its kind
/// deleted meanwhile (its objects renumbered): what was typed stays with
/// its object, now one place up its list, and is set on it when the field
/// is left; the panel's fields follow their object as a window's do.
#[test]
fn typed_text_follows_its_object_through_a_renumbering() {
    let Some((mut h, area)) = props_area("props-renumber", Mode::Floating, 1600.0) else {
        return;
    };
    select(&mut h, area, &[(ObjectKind::Placeable, 1)]);
    let before = placeables(&mut h, area);
    let old = tag(&before[1]);
    h.get_all_by_value(&old).find(is_input).expect("the field").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&old).find(is_input).unwrap().type_text("CHEST_SECOND");
    h.run_steps(2);
    // The first chest taken out, by another hand (a command of its own).
    let git = ResKey::new(area, ResType::GIT);
    let delete = mg_edit::Edit::remove_item(git, mg_edit::GffPath::root(), "Placeable List", 0);
    let command = mg_edit::Command::new("Delete", vec![delete]);
    h.state_mut().actions.push(mg_ui::Action::Apply(command));
    settle(&mut h);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Placeable, 0)], "followed");
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    let now = placeables(&mut h, area);
    assert_eq!(now.len(), 1, "one chest left");
    assert_eq!(tag(&now[0]), "CHEST_SECOND", "set on the chest it was typed for");
}

/// After F6 gives the panel the keys, Up and Down on its rail show the
/// page before or after, the keyboard going with them (the review of U6:
/// they left the panel, the next Tab on Module Properties' tab).
#[test]
fn the_rail_s_pages_by_up_and_down() {
    let Some((mut h, area)) = props_area("props-rail-keys", Mode::Floating, 1600.0) else {
        return;
    };
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    for _ in 0..8 {
        if mg_ui::palette_view::arrow_keys_are(&h.ctx) == "Properties" {
            break;
        }
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    let focused = |h: &Harness<'_, Moonglow>, page: &str| {
        h.get_by_label("Pages").get_by_label(page).accesskit_node().is_focused()
    };
    let on = |h: &Harness<'_, Moonglow>, page: &str| {
        let node = h.get_by_label("Pages").get_by_label(page);
        node.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
    };
    assert!(focused(&h, "Basic"), "F6: Basic has the keyboard");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert!(on(&h, "Inventory") && focused(&h, "Inventory"), "Down: the next page");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert!(on(&h, "Lock") && focused(&h, "Lock"), "Down again");
    h.key_press(egui::Key::ArrowUp);
    settle(&mut h);
    assert!(on(&h, "Inventory") && focused(&h, "Inventory"), "Up: back");
}

/// A double click on an object's row in the Outliner (as on the object in
/// the view, the coordinator's ruling at U6's review) selects it and
/// shows the Properties panel in front, the Right Dock unfolded; no
/// window opens.
#[test]
fn a_double_click_on_an_outliner_row_shows_the_panel() {
    let Some((mut h, area)) = props_area("props-outliner", Mode::Floating, 1600.0) else {
        return;
    };
    // (The row in sight: its kind opened as the Outliner follows a
    // selection, then nothing selected, the Right Dock folded.)
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    let chest = tag(&placeables(&mut h, area)[0]);
    select(&mut h, area, &[]);
    h.state_mut().settings.hide_palettes = true;
    settle(&mut h);
    let row = (h.get_by_label(mg_ui::OUTLINER).query_all_by_label_contains(chest.as_str()))
        .next()
        .expect("the chest's row")
        .rect();
    double_click_now(&mut h, row.center());
    settle(&mut h);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Placeable, 0)], "selected");
    assert!(!h.state().settings.hide_palettes, "the Right Dock unfolded");
    h.get_by_label("Properties, Placeable");
    let path = mg_edit::GffPath::root().item("Placeable List", 0);
    assert!(h.state().dock.find_tab(&Tab::Instance { area, path }).is_none(), "no window");
}

/// A double click on an object's row in the module tree while another tab
/// is in front (As Tabs: Module Properties) brings its area to the front,
/// selects the object and shows it in the Properties panel (the
/// coordinator's ruling at U6's review).
#[test]
fn a_double_click_on_a_tree_row_brings_the_area_and_the_panel() {
    let Some((mut h, area)) = props_area("props-tree-row", Mode::Tabs, 1600.0) else { return };
    tall_tree(&mut h);
    h.get_by_label_contains("Placed Objects in ").click();
    h.run_steps(3);
    h.get_by_label("Waypoints (2)").click();
    settle(&mut h);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    let tag = tag(&git.root.list("WaypointList").unwrap()[1]);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::ModuleProperties), "another tab in front");
    let rows = h.get_by_label("Module Tree Rows");
    let row = rows.query_all_by_label_contains(tag.as_str()).last().expect("its row").rect();
    double_click_now(&mut h, row.center());
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Area(area)), "the area in front");
    let selection = &h.state().area_views[&area].selection;
    assert!(selection.len() == 1 && selection[0].0 == ObjectKind::Waypoint, "{selection:?}");
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Properties), "the panel");
}

/// The panel's controls in the Right Dock named `name`, top to bottom.
fn in_dock(h: &Harness<'_, Moonglow>, name: &str) -> Vec<egui::Rect> {
    let dock = right_dock_rect(h);
    let mut found: Vec<egui::Rect> =
        h.get_all_by_label(name).map(|n| n.rect()).filter(|r| dock.contains_rect(*r)).collect();
    found.sort_by(|a, b| a.top().total_cmp(&b.top()));
    found
}

/// A localized name whose text is the talk table's, in the narrow panel
/// (U30's #1 and #15: "Plague Vi" beside "StrRef 5763" at 320, "Cabb" at
/// 280): its text takes the field's whole width, and "StrRef …" stands
/// on the row under it, beside the field's "…".
#[test]
fn a_name_s_strref_stands_under_its_field_in_the_narrow_panel() {
    let Some((mut h, area)) = props_area("props-strref", Mode::Floating, 1280.0) else { return };
    select(&mut h, area, &[(ObjectKind::Creature, 0)]);
    h.get_by_label("Right Dock").get_by_label("Properties").click();
    settle(&mut h);
    for (window, width) in [(1600.0, 320.0), (1280.0, 280.0)] {
        h.set_size(egui::vec2(window, 900.0));
        right_dock_width(&mut h, width);
        settle(&mut h);
        let dock = right_dock_rect(&h);
        assert!((dock.width() - width).abs() < 1.0, "at {width}: the dock {:.0}", dock.width());
        let chip = (h.query_all_by_label_contains("StrRef "))
            .map(|n| n.rect())
            .find(|r| dock.y_range().contains(r.center().y))
            .unwrap_or_else(|| panic!("at {width}: no strref"));
        assert!(dock.contains_rect(chip), "at {width}: {chip:?} past the panel {dock:?}");
        // (The first name's field: the panel's text field nearest over
        // or around the strref.)
        let field = (h.get_all_by_role(egui::accesskit::Role::TextInput))
            .filter(|n| dock.contains_rect(n.rect()) && n.rect().top() <= chip.top())
            .max_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
            .expect("the name's field");
        let (name, field) = (field.accesskit_node().value().unwrap_or_default(), field.rect());
        assert!(!name.is_empty(), "at {width}: the name shown");
        assert!(chip.top() >= field.bottom(), "at {width}: {chip:?} not under {field:?}");
        let languages = in_dock(&h, "…");
        let beside = languages.iter().find(|r| (r.center().y - chip.center().y).abs() < 4.0);
        let beside = beside.unwrap_or_else(|| panic!("at {width}: no … beside {chip:?}"));
        assert!(beside.left() >= chip.right(), "at {width}: … after the strref");
        // (Nothing of the name cut: its text and the field's margins.)
        let font = h.ctx.global_style().text_styles[&egui::TextStyle::Body].clone();
        let text = h.ctx.fonts_mut(|f| f.layout_no_wrap(name.clone(), font, egui::Color32::WHITE));
        let margins = mg_uikit::controls::FIELD_MARGIN.sum().x;
        assert!(
            text.size().x + margins <= field.width() + 0.5,
            "at {width}: {name:?} ({:.0}) cut in a field {:.0} wide",
            text.size().x,
            field.width()
        );
    }
}

/// A placeable's Static, Useable and Has inventory rows, and Inventory…
/// under them, stand at the form's own pitch, in the narrow panel and the
/// wide layout's (U30's #2: 47 to 51 apart, Useable's and Has inventory's
/// check boxes each in a scope of its own in the grid's cell).
#[test]
fn a_placeable_s_check_rows_stand_at_the_form_s_pitch() {
    let Some((mut h, area)) = props_area("props-pitch", Mode::Floating, 1280.0) else { return };
    select(&mut h, area, &[(ObjectKind::Placeable, 0)]);
    h.get_by_label("Right Dock").get_by_label("Properties").click();
    settle(&mut h);
    for width in [280.0, 320.0, 460.0] {
        right_dock_width(&mut h, width);
        settle(&mut h);
        let at = |name: &str| {
            let found = in_dock(&h, name);
            *found.first().unwrap_or_else(|| panic!("at {width}: no {name} in the panel"))
        };
        let rows = ["Static", "Useable", "Has inventory", "Inventory…"].map(at);
        let pitch: Vec<f32> = rows.windows(2).map(|w| w[1].center().y - w[0].center().y).collect();
        // (The check boxes' rows one pitch apart; Inventory…, a button
        // a few points higher than a check box, at most a few more.)
        assert!((pitch[0] - pitch[1]).abs() < 1.0, "at {width}: rows apart by {pitch:?}");
        assert!(pitch[0] < 34.0, "at {width}: rows apart by {pitch:?}");
        assert!(pitch[2] < pitch[0] + 6.0, "at {width}: Inventory… apart by {pitch:?}");
    }
}
