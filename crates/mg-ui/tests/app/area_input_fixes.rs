//! The area view's input where a tool is on: Paint with no placeable
//! chosen places nothing but leaves selecting, Delete, copy and paste as
//! they are with Paint off (and shows no ghost where a click places
//! nothing) and leaves a paste's and a redrawn outline's clicks and Alt's
//! raise as they are; one Escape during Select Tiles' box drag drops the box and no
//! more; Adjust Location's OK and Apply on several objects whose values
//! differ keep each one's own.

use crate::common::*;
use glam::Vec3;
use mg_area::{ObjectKind, VisualTransform};

/// Clicks the shelf's tool `name`.
fn shelf_click(h: &mut Harness<'_, Moonglow>, name: &str) {
    h.get_by_label(AREA_TOOLS).get_by_label(name).click();
    h.run_steps(3);
}

/// A click at `at`, pressed and let go in one frame, the pointer brought
/// there first.
fn click_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    h.run_steps(1);
    for pressed in [true, false] {
        press(h, at, pressed, egui::Modifiers::NONE);
    }
    settle(h);
}

/// The length of the area's GIT list `list`.
fn listed(h: &mut Harness<'_, Moonglow>, area: ResRef, list: &str) -> usize {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&ResKey::new(area, ResType::GIT)).unwrap().root.list(list).map_or(0, <[_]>::len)
}

/// The area's GIT list `list`'s structs, as they are now.
fn structs(h: &mut Harness<'_, Moonglow>, area: ResRef, list: &str) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&ResKey::new(area, ResType::GIT)).unwrap().root.list(list).unwrap_or_default().to_vec()
}

/// Puts a dynamic chest at each of `at`, with the visual transform beside
/// it where one is given: they end in `Placeable List` in the order given.
fn chests(h: &mut Harness<'_, Moonglow>, area: ResRef, at: &[(Vec3, Option<VisualTransform>)]) {
    use mg_module::instances::{Placement, Placing, instance};
    let git_key = ResKey::new(area, ResType::GIT);
    let app = h.state_mut();
    let game = app.game.as_deref().unwrap();
    let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let mut edits = Vec::new();
    for (index, (p, visual)) in at.iter().enumerate() {
        let placement = Placement { position: p.to_array(), rotation: 0.3 * index as f32 };
        let mut item = instance(&placing, ResType::UTP, &chest, placement, &[]).unwrap();
        item.set("Static", mg_gff::Value::Byte(0));
        if let Some(v) = visual {
            item.set("VisTransformList", mg_area::edit::visual_transform_list(*v));
        }
        edits.push(mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index,
            item,
        });
    }
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.run_steps(3);
    settle(h);
}

/// Paint on with no placeable chosen: a click selects, Delete deletes,
/// copy and paste place a copy, and an object is dragged, as with Paint
/// off; Escape puts Paint away and keeps the selection.
#[test]
fn paint_with_nothing_chosen_leaves_the_rest_of_the_view_working() {
    let Some((mut h, area)) = area_harness("paint-none-works") else { return };
    settle(&mut h);
    chests(&mut h, area, &[(Vec3::new(12.0, 30.0, 0.0), None), (Vec3::new(20.0, 30.0, 0.0), None)]);
    shelf_click(&mut h, "Paint");
    assert!(h.state().paint);
    assert!(h.state().palette.selected.is_none());

    // A click on a chest selects it.
    let chest = screen(&h, area, Vec3::new(12.0, 30.0, 0.3));
    click_at(&mut h, chest);
    assert_eq!(
        h.state().area_views[&area].selection,
        [(ObjectKind::Placeable, 0)],
        "a click selects with Paint on"
    );

    // Copied and pasted: a copy where the next click is.
    h.hover_at(chest);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting, "Paste takes the copy in hand");
    let there = screen(&h, area, Vec3::new(16.0, 24.0, 0.0));
    click_at(&mut h, there);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"));
    assert_eq!(listed(&mut h, area, "Placeable List"), 3, "the copy placed");
    assert!(h.state().paint, "Paint still on");

    // Dragged: the chest moves.
    let second = Vec3::new(20.0, 30.0, 0.0);
    let was = structs(&mut h, area, "Placeable List")[1].float("X");
    let from = screen(&h, area, second + Vec3::Z * 0.3);
    let to = screen(&h, area, second + Vec3::X * 3.0);
    drag(&mut h, from, to);
    settle(&mut h);
    let now = structs(&mut h, area, "Placeable List")[1].float("X");
    assert_ne!(now, was, "a drag moves the object with Paint on");

    // Delete deletes what is selected.
    let n = listed(&mut h, area, "Placeable List");
    assert!(!h.state().area_views[&area].selection.is_empty());
    h.hover_at(screen(&h, area, Vec3::new(16.0, 18.0, 0.0)));
    h.run_steps(1);
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(listed(&mut h, area, "Placeable List"), n - 1, "Delete with Paint on");

    // Escape puts Paint away, and the selection stays.
    let first = screen(&h, area, Vec3::new(12.0, 30.0, 0.3));
    click_at(&mut h, first);
    let selected = h.state().area_views[&area].selection.clone();
    assert_eq!(selected.len(), 1);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().paint, "Escape puts Paint away");
    assert_eq!(h.state().area_views[&area].selection, selected, "and only that");
}

/// Paint on with a waypoint chosen in the palette: a click places none,
/// changes no selection, and no ghost follows the pointer; with Paint off
/// the same choice shows its ghost and the click places it.
#[test]
fn paint_with_a_waypoint_chosen_places_nothing_and_shows_no_ghost() {
    let Some((mut h, area)) = area_harness("paint-waypoint-none") else { return };
    settle(&mut h);
    let spot = screen(&h, area, Vec3::new(14.0, 14.0, 0.0));
    let waypoints = listed(&mut h, area, "WaypointList");

    shelf_click(&mut h, "Paint");
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    for _ in 0..3 {
        h.hover_at(spot);
        h.run_steps(1);
    }
    assert!(h.state().area_views[&area].ghost_shown.is_none(), "no ghost with Paint on");
    click_at(&mut h, spot);
    assert_eq!(listed(&mut h, area, "WaypointList"), waypoints, "nothing placed");
    assert_eq!(
        h.state().area_views[&area].selection,
        [(ObjectKind::Waypoint, 0)],
        "the placing click is swallowed whole"
    );

    // Paint off, the waypoint still chosen: its ghost, and a click places it.
    shelf_click(&mut h, "Select Objects");
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    for _ in 0..3 {
        h.hover_at(spot);
        h.run_steps(1);
    }
    assert!(h.state().area_views[&area].ghost_shown.is_some(), "a ghost with Paint off");
    click_at(&mut h, spot);
    assert_eq!(listed(&mut h, area, "WaypointList"), waypoints + 1, "placed with Paint off");
}

/// Paint on with a waypoint chosen in the palette (whose placing click is
/// swallowed): a paste's click still pastes, one Escape drops a paste and
/// leaves Paint on, and a trigger's outline drawn anew takes its clicks.
#[test]
fn paint_with_a_waypoint_chosen_still_pastes_and_redraws() {
    let Some((mut h, area)) = area_harness("paint-waypoint-paste") else { return };
    settle(&mut h);
    chests(&mut h, area, &[(Vec3::new(12.0, 30.0, 0.0), None)]);
    shelf_click(&mut h, "Paint");
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    let waypoints = listed(&mut h, area, "WaypointList");

    // Copied and pasted: the paste's click places the copy, no waypoint.
    // (Selected by hand: a click on the chest is the swallowed one.)
    let chest = screen(&h, area, Vec3::new(12.0, 30.0, 0.3));
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    h.hover_at(chest);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting, "Paste takes the copy in hand");
    let there = screen(&h, area, Vec3::new(16.0, 24.0, 0.0));
    click_at(&mut h, there);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"), "the paste lands");
    assert_eq!(listed(&mut h, area, "Placeable List"), 2, "the copy placed");
    assert_eq!(listed(&mut h, area, "WaypointList"), waypoints, "no waypoint placed");
    assert!(h.state().paint, "Paint still on");

    // Pasted again and Escape: the paste dropped, Paint still on.
    h.hover_at(chest);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().area_views[&area].pasting, "one Escape drops the paste");
    assert!(h.state().paint, "and leaves Paint on");

    // A trigger's outline drawn anew: its clicks are its corners.
    let git = ResKey::new(area, ResType::GIT);
    let mut trigger = mg_gff::Struct::new(1);
    for (label, v) in [("XPosition", 10.0), ("YPosition", 10.0), ("ZPosition", 0.0)] {
        trigger.set(label, mg_gff::Value::Float(v));
    }
    trigger.set("Geometry", mg_gff::Value::List(Vec::new()));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Trigger",
        vec![mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "TriggerList".into(),
            index: 0,
            item: trigger,
        }],
    )));
    settle(&mut h);
    h.state_mut().area_views.get_mut(&area).unwrap().redraw = Some((ObjectKind::Trigger, 0));
    h.run_steps(2);
    let first = screen(&h, area, Vec3::new(18.0, 18.0, 0.0));
    click_at(&mut h, first);
    assert_eq!(h.state().area_views[&area].outline.len(), 1, "the click is the first corner");
    let opposite = screen(&h, area, Vec3::new(24.0, 21.0, 0.0));
    h.hover_at(opposite);
    h.run_steps(1);
    for pressed in [true, false] {
        press(&h, opposite, pressed, egui::Modifiers::ALT);
    }
    settle(&mut h);
    assert!(h.state().area_views[&area].redraw.is_none(), "closed by the second click");
    assert_ne!(h.state().ws.as_ref().unwrap().can_undo(), Some("Trigger"), "the outline redrawn");
    assert_eq!(listed(&mut h, area, "WaypointList"), waypoints, "no waypoint placed");
    assert!(h.state().paint, "Paint still on");
}

/// Paint on with nothing chosen is Select Objects: Alt and a drag raise
/// the selection, not move the view.
#[test]
fn paint_with_nothing_chosen_alt_drag_raises_the_selection() {
    let Some((mut h, area)) = area_harness("paint-none-alt") else { return };
    settle(&mut h);
    shelf_click(&mut h, "Paint");
    assert!(h.state().paint);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    h.run_steps(2);
    let git = ResKey::new(area, ResType::GIT);
    let z = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git).unwrap().root.list("WaypointList").unwrap()[0].float("ZPosition").unwrap()
    };
    let z0 = z(&mut h);
    let alt = egui::Modifiers::ALT;
    let a = screen(&h, area, Vec3::new(5.0, 30.0, 0.0));
    h.event(egui::Event::ModifiersChanged(alt));
    h.hover_at(a);
    h.step();
    let button = egui::PointerButton::Primary;
    h.event(egui::Event::PointerButton { pos: a, button, pressed: true, modifiers: alt });
    h.step();
    let end = a + egui::vec2(0.0, -40.0);
    h.hover_at(end);
    h.step();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    press(&h, end, false, egui::Modifiers::NONE);
    settle(&mut h);
    assert!(z(&mut h) - z0 > 0.1, "Alt + drag raises the selection with Paint on");
    assert!(h.state().paint, "Paint still on");
}

/// Select Tiles: Escape while a box is dragged drops the box, and only
/// that; the tiles selected stay selected, Select Tiles stays on, and
/// letting go selects nothing.
#[test]
fn escape_in_a_tile_box_drag_only_drops_the_box() {
    let Some((mut h, area)) = area_harness("tile-box-escape") else { return };
    settle(&mut h);
    shelf_click(&mut h, "Select Tiles");
    assert!(h.state().area_views[&area].tile_mode);
    let from = screen(&h, area, Vec3::new(5.0, 5.0, 0.0));
    let to = screen(&h, area, Vec3::new(25.0, 25.0, 0.0));

    for kept in [vec![(3, 3)], vec![]] {
        h.state_mut().area_views.get_mut(&area).unwrap().tile_selection = kept.clone();
        h.hover_at(from);
        h.run_steps(1);
        press(&h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for step in 1..=6 {
            h.hover_at(from + (to - from) * (step as f32 / 6.0));
            h.run_steps(1);
        }
        h.key_press(egui::Key::Escape);
        h.run_steps(1);
        let view = &h.state().area_views[&area];
        assert!(view.tile_mode, "Select Tiles stays on ({kept:?} selected)");
        assert_eq!(view.tile_selection, kept, "the tiles selected stay");
        press(&h, to, false, egui::Modifiers::NONE);
        settle(&mut h);
        let view = &h.state().area_views[&area];
        assert_eq!(view.tile_selection, kept, "letting go selects nothing");
        assert!(view.tile_mode);
    }
}

/// Adjust Location on three chests whose scale, turns and shifts differ,
/// and a waypoint: Apply with one visual field changed, then OK with Y
/// changed by the field's arrow, each keeps every object's own other
/// values (its place, bearing and the rest of its transform), the
/// waypoint taking only the Y; one Undo takes back each.
#[test]
fn adjust_location_on_several_keeps_each_object_s_own_values() {
    let Some((mut h, area)) = area_harness("adjust-several-write") else { return };
    settle(&mut h);
    let v = |s: f32, r: [f32; 3], t: [f32; 3]| VisualTransform {
        scale: Vec3::splat(s),
        rotate: Vec3::from(r),
        translate: Vec3::from(t),
    };
    let own = [
        v(0.5, [10.0, 5.0, -3.0], [0.1, 0.2, 0.3]),
        v(1.25, [-40.0, 20.0, 7.0], [-0.5, 0.0, 1.0]),
        v(2.0, [90.0, -15.0, 30.0], [0.0, -0.75, 0.25]),
    ];
    chests(
        &mut h,
        area,
        &[
            (Vec3::new(12.0, 30.0, 0.0), Some(own[0])),
            (Vec3::new(16.0, 32.0, 0.0), Some(own[1])),
            (Vec3::new(20.0, 34.0, 0.0), Some(own[2])),
        ],
    );
    let chosen = vec![
        (ObjectKind::Placeable, 0),
        (ObjectKind::Placeable, 1),
        (ObjectKind::Placeable, 2),
        (ObjectKind::Waypoint, 0),
    ];
    let start_p = structs(&mut h, area, "Placeable List");
    let start_w = structs(&mut h, area, "WaypointList");
    {
        let app = h.state_mut();
        let view = app.area_views.get_mut(&area).unwrap();
        view.selection = chosen.clone();
        app.adjust = mg_ui::area_tools::adjust(&app.area_views[&area]);
    }
    h.run_steps(3);
    assert_eq!(h.state().adjust.as_ref().expect("the window").objects, chosen);

    // Apply: the Y rotation alone, on each chest.
    {
        let a = h.state_mut().adjust.as_mut().unwrap();
        a.visual.rotate.y = 12.5;
        a.visual_changed.rotate[1] = true;
    }
    h.get_by_label("Apply").click();
    h.run_steps(3);
    settle(&mut h);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Adjust Location"));
    let applied_p = structs(&mut h, area, "Placeable List");
    for (i, s) in applied_p.iter().enumerate() {
        let mut want = own[i];
        want.rotate.y = 12.5;
        assert_eq!(VisualTransform::read(s), Some(want), "chest {i}'s own transform, Y turned");
        for f in ["X", "Y", "Z", "Bearing"] {
            assert_eq!(s.float(f), start_p[i].float(f), "chest {i}'s {f} kept");
        }
    }
    assert_eq!(structs(&mut h, area, "WaypointList"), start_w, "the waypoint untouched");
    assert!(h.state().adjust.is_some(), "Apply keeps the window open");

    // OK: Y up by the field's arrow, twice.
    let y_was = h.state().adjust.as_ref().unwrap().position.y;
    let at = h.get_by_label("Y: One More").rect().center();
    for _ in 0..2 {
        h.hover_at(at);
        for pressed in [true, false] {
            press(&h, at, pressed, egui::Modifiers::NONE);
        }
        h.run_steps(2);
    }
    let a = h.state().adjust.as_ref().unwrap();
    assert_eq!(a.changed, [false, true, false, false], "only Y changed");
    assert!(!a.visual_changed.any() && !a.speed_changed);
    let y = a.position.y;
    assert!((y - (y_was + 0.2)).abs() < 1e-4, "{y} from {y_was}");
    h.get_by_label("OK").click();
    h.run_steps(3);
    settle(&mut h);
    assert!(h.state().adjust.is_none());
    let ok_p = structs(&mut h, area, "Placeable List");
    for (i, s) in ok_p.iter().enumerate() {
        assert_eq!(s.float("Y"), Some(y), "chest {i} at the new Y");
        for f in ["X", "Bearing"] {
            assert_eq!(s.float(f), applied_p[i].float(f), "chest {i}'s {f} kept");
        }
        assert_eq!(
            VisualTransform::read(s),
            VisualTransform::read(&applied_p[i]),
            "chest {i}'s transform kept"
        );
    }
    let ok_w = structs(&mut h, area, "WaypointList");
    assert_eq!(ok_w[0].float("YPosition"), Some(y), "the waypoint at the new Y");
    for f in ["XPosition", "XOrientation", "YOrientation"] {
        assert_eq!(ok_w[0].float(f), start_w[0].float(f), "the waypoint's {f} kept");
    }

    // One Undo each.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(structs(&mut h, area, "Placeable List"), applied_p, "Undo takes back OK");
    assert_eq!(structs(&mut h, area, "WaypointList"), start_w);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(structs(&mut h, area, "Placeable List"), start_p, "Undo takes back Apply");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Setup"));
}
