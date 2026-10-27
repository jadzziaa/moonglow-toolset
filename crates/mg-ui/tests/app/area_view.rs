//! The area view: selecting, placing, moving and turning objects, the
//! camera, copy and paste.

use crate::common::*;

/// GitHub issue 25: Alt + left drag off the selection moves the camera,
/// the ground following the pointer, and moves nothing in the area; on
/// the selection it raises it, as before.
#[test]
fn alt_and_a_drag_off_the_selection_moves_the_camera() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("alt-pan") else { return };
    let orbit = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap();
    let revision = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().revision();
    let alt = egui::Modifiers::ALT;
    let drag = |h: &mut Harness<'_, Moonglow>, from: Vec3, to: Vec3| {
        let (a, b) = (screen(h, area, from), screen(h, area, to));
        h.event(egui::Event::ModifiersChanged(alt));
        h.hover_at(a);
        // (Alt held on: the press does not let it go.)
        let button = egui::PointerButton::Primary;
        h.event(egui::Event::PointerButton { pos: a, button, pressed: true, modifiers: alt });
        for k in 1..=6 {
            h.hover_at(a + (b - a) * (k as f32 / 6.0));
            h.step();
        }
        // (Alt let go before the button: the drag goes on as it began.)
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        press(h, b, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    let (before, changed) = (orbit(&h), revision(&h));
    // Nothing selected, on bare ground: the ground follows the pointer.
    drag(&mut h, Vec3::new(10.0, 10.0, 0.0), Vec3::new(14.0, 10.0, 0.0));
    let moved = orbit(&h).target - before.target;
    assert!((moved.x + 4.0).abs() < 0.6 && moved.y.abs() < 0.6, "panned: {moved:?}");
    assert_eq!(revision(&h), changed, "nothing in the area moved");
    // A waypoint selected, the drag begun off it: the camera again, and
    // the waypoint stays.
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    h.run_steps(2);
    let before = orbit(&h);
    drag(&mut h, Vec3::new(5.0, 30.0, 0.0), Vec3::new(5.0, 34.0, 0.0));
    assert!((orbit(&h).target - before.target).length() > 2.0, "panned");
    assert_eq!(revision(&h), changed, "the waypoint stays");
}

/// GitHub issue 26: a sideways scroll pans the area's view left and right
/// as the screen has it, without zooming; Shift and the wheel (which egui
/// reports sideways) still zoom, finer.
#[test]
fn a_sideways_scroll_pans_the_area_view() {
    let Some((mut h, area)) = area_harness("sideways-scroll") else { return };
    let orbit = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap();
    let rect = h
        .state()
        .dock
        .iter_leaves()
        .find(|(_, l)| l.tabs.contains(&Tab::Area(area)))
        .map(|(_, l)| l.viewport)
        .unwrap();
    h.hover_at(rect.center());
    h.run_steps(2);
    let before = orbit(&h);
    let wheel = |h: &mut Harness<'_, Moonglow>, delta: egui::Vec2, modifiers: egui::Modifiers| {
        h.event_modifiers(
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta,
                phase: egui::TouchPhase::Move,
                modifiers,
            },
            modifiers,
        );
        h.run_steps(30);
    };
    wheel(&mut h, egui::vec2(120.0, 0.0), egui::Modifiers::NONE);
    let after = orbit(&h);
    assert!((after.distance - before.distance).abs() < 1e-3, "no zoom");
    // Looking north (the overview), screen left and right are west and east.
    let moved = after.target - before.target;
    assert!(moved.x.abs() > 0.5 && moved.y.abs() < 0.05, "panned sideways: {moved:?}");
    // The other way, back.
    wheel(&mut h, egui::vec2(-120.0, 0.0), egui::Modifiers::NONE);
    assert!((orbit(&h).target - before.target).length() < 0.05);
    // Shift and the wheel: a zoom, as before. (Shift held through the
    // frames the scroll is spread over.)
    let shift = egui::Modifiers::SHIFT;
    h.event(egui::Event::ModifiersChanged(shift));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        modifiers: shift,
    });
    h.run_steps(30);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    let shifted = orbit(&h);
    assert!(shifted.distance < before.distance - 0.1, "zoomed: {}", shifted.distance);
    assert!((shifted.target - before.target).length() < 0.05, "not panned");
}

#[test]
fn area_viewer_selects_moves_and_deletes() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("move") else { return };
    let view = &h.state().area_views[&area];
    let model = view.model.as_ref().expect("the area is read");
    assert_eq!((model.width, model.height, model.objects.len()), (4, 4, 2));
    assert!(model.problems.is_empty(), "{:?}", model.problems);
    let (x0, y0, turn) = waypoint(&mut h, area, 0).unwrap();

    // A click on the first waypoint's marker selects it.
    let on = screen(&h, area, Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(on);
    press(&h, on, true, egui::Modifiers::NONE);
    press(&h, on, false, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 0)]);

    // Dragged 5 m east (by its feet: the ground under the pointer moves it):
    // one command moves it, its orientation untouched.
    let on = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    let to = screen(&h, area, Vec3::new(25.0, 20.0, 0.02));
    press(&h, on, true, egui::Modifiers::NONE);
    for k in 1..=4 {
        h.hover_at(on + (to - on) * (k as f32 / 4.0));
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let (x, y, t) = waypoint(&mut h, area, 0).unwrap();
    assert!((x - 25.0).abs() < 0.05 && (y - y0).abs() < 0.05, "moved to {x}, {y}");
    assert_eq!(t, turn, "the orientation is not rewritten");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(waypoint(&mut h, area, 0).map(|w| (w.0, w.1)), Some((x0, y0)));

    // Ctrl + click adds the second; Delete deletes both.
    let second = screen(&h, area, Vec3::new(30.0, 20.0, 0.9));
    h.hover_at(second);
    press(&h, second, true, egui::Modifiers::COMMAND);
    press(&h, second, false, egui::Modifiers::COMMAND);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].selection.len(), 2);
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(waypoint(&mut h, area, 0), None, "both deleted");
    assert!(h.state().area_views[&area].selection.is_empty());
    // Undo brings them back; the second stays selectable.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(waypoint(&mut h, area, 1).is_some());
    h.hover_at(second);
    press(&h, second, true, egui::Modifiers::NONE);
    press(&h, second, false, egui::Modifiers::NONE);
    h.run_steps(2);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-move").join("area_view.png")).unwrap();
}

/// The placeable brush (Paint): two placeables chosen in the palette,
/// painted in one stroke that one Undo takes away; a dab where the circle
/// is full lays nothing; Q turns them; Shift + wheel sizes the circle
/// rather than zooming; Escape puts the brush away.
#[test]
fn area_viewer_paints_placeables() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("paint") else { return };
    let placeables = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("Placeable List").unwrap_or(&[]).to_vec()
    };
    let at = |s: &mg_gff::Struct| glam::Vec2::new(s.float("X").unwrap(), s.float("Y").unwrap());
    let tree = ResKey::parse("plc_tree", ResType::UTP).unwrap();
    let shrub = ResKey::parse("plc_shrub", ResType::UTP).unwrap();
    // B over the view turns Paint on; with nothing chosen, Escape puts it
    // away again.
    h.hover_at(screen(&h, area, Vec3::new(20.0, 20.0, 0.0)));
    h.run_steps(1);
    h.key_press(egui::Key::B);
    h.run_steps(1);
    assert!(h.state().paint, "B turns Paint on");
    h.get_by_label_contains("Paint: choose placeables in the palette");
    h.key_press(egui::Key::Escape);
    h.run_steps(1);
    assert!(!h.state().paint, "Escape puts it away");
    {
        let app = h.state_mut();
        // (The Placeables palette shown, not the tileset's.)
        app.palette.tiles = false;
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.selected = Some(tree);
        app.palette.chosen = vec![shrub];
        app.paint = true;
        app.settings.paint = mg_ui::settings::PaintSettings {
            radius_cm: 400,
            density: 20,
            spacing_cm: 100,
            flow: 100,
            turn_spread: 0,
            ..Default::default()
        };
    }
    h.run_steps(2);
    h.get_by_label_contains("Painting 2 placeables");
    h.get_by_label("Density");

    // A stroke: pressed at 12, 15 and dragged 8 m east, then let go.
    let from = screen(&h, area, Vec3::new(12.0, 15.0, 0.0));
    let to = screen(&h, area, Vec3::new(20.0, 15.0, 0.0));
    h.hover_at(from);
    h.run_steps(1);
    // Before the press, the dab it would lay shows, see-through.
    assert!(h.state().area_views[&area].paint.shown > 0, "the next dab shows");
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=8 {
        h.hover_at(from + (to - from) * (k as f32 / 8.0));
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let painted = placeables(&mut h);
    assert!(painted.len() >= 10, "painted {}", painted.len());
    for (i, p) in painted.iter().enumerate() {
        let q = at(p);
        assert!((7.9..=24.1).contains(&q.x) && (10.9..=19.1).contains(&q.y), "{q} off the stroke");
        let template = p.resref("TemplateResRef");
        assert!(template.is_some_and(|t| t == tree.resref || t == shrub.resref), "{template:?}");
        for o in &painted[i + 1..] {
            assert!(q.distance(at(o)) >= 0.99, "{q} and {} too near", at(o));
        }
    }
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paint Placeables"));
    // One Undo takes the whole stroke away.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(placeables(&mut h).is_empty(), "one stroke, one command");

    // Q turns the brush: a click lays the circle's share (10 of 20 a
    // tile on 50 m²), each turned 15°; a click there again, nothing more.
    let spot = screen(&h, area, Vec3::new(28.0, 28.0, 0.0));
    h.hover_at(spot);
    h.run_steps(1);
    h.key_press(egui::Key::Q);
    h.run_steps(1);
    let click = |h: &mut Harness<'_, Moonglow>| {
        press(h, spot, true, egui::Modifiers::NONE);
        press(h, spot, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    click(&mut h);
    let painted = placeables(&mut h);
    assert_eq!(painted.len(), 10);
    for p in &painted {
        let bearing = p.float("Bearing").unwrap();
        assert!((bearing - 15f32.to_radians()).abs() < 1e-3, "turned {bearing}");
    }
    click(&mut h);
    assert_eq!(placeables(&mut h).len(), 10, "the circle is full");

    // Shift and the wheel, the button held down: a bigger circle, no zoom.
    let distance = h.state().area_views[&area].orbit.unwrap().distance;
    let shift = egui::Modifiers::SHIFT;
    press(&h, spot, true, egui::Modifiers::NONE);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(shift));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        modifiers: shift,
    });
    h.run_steps(30);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    press(&h, spot, false, egui::Modifiers::NONE);
    h.run_steps(1);
    assert!(h.state().settings.paint.radius_cm > 450, "{}", h.state().settings.paint.radius_cm);
    let now = h.state().area_views[&area].orbit.unwrap().distance;
    assert!((now - distance).abs() < 1e-3, "no zoom");

    // Looked at from nearer and lower, the circle around what was painted.
    if let Some(o) = &mut h.state_mut().area_views.get_mut(&area).unwrap().orbit {
        o.target = Vec3::new(27.0, 26.0, 0.0);
        o.distance = 22.0;
        o.pitch = 0.75;
    }
    h.run_steps(2);
    h.hover_at(screen(&h, area, Vec3::new(28.0, 28.0, 0.0)));
    h.run_steps(2);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-paint").join("area_view.png")).unwrap();
    // Escape: Paint goes off; the placeables stay chosen.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(!h.state().paint);
    assert_eq!(h.state().palette.chosen, [shrub]);
}

/// The placeable brush's Row: a drag lays one placeable end to end along
/// it, each facing the same way, as one step.
#[test]
fn area_viewer_lays_a_row_of_placeables() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("paint-row") else { return };
    let placeables = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("Placeable List").unwrap_or(&[]).to_vec()
    };
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.selected = ResKey::parse("plc_statue1", ResType::UTP);
        app.paint = true;
        app.settings.paint.row = true;
        app.settings.paint.gap_cm = 50;
    }
    h.run_steps(2);
    h.get_by_label_contains("Laying a row of plc_statue1");
    h.get_by_label("Gap");
    let from = screen(&h, area, Vec3::new(8.0, 15.0, 0.0));
    let to = screen(&h, area, Vec3::new(32.0, 15.0, 0.0));
    h.hover_at(from);
    h.run_steps(1);
    assert_eq!(h.state().area_views[&area].paint.shown, 1, "a piece shows before the press");
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=6 {
        h.hover_at(from + (to - from) * (k as f32 / 6.0));
    }
    h.run_steps(1);
    let shown = h.state().area_views[&area].paint.shown;
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let row = placeables(&mut h);
    assert!(row.len() >= 3, "laid {}", row.len());
    assert_eq!(row.len(), shown, "what showed is what is laid");
    let x = |i: usize| row[i].float("X").unwrap();
    let step = x(1) - x(0);
    assert!(step > 0.5, "a piece's length and the gap: {step}");
    for (i, p) in row.iter().enumerate() {
        assert!((p.float("Y").unwrap() - 15.0).abs() < 0.05, "along the drag");
        assert_eq!(p.float("Bearing"), row[0].float("Bearing"), "facing alike");
        if i > 0 {
            assert!((x(i) - x(i - 1) - step).abs() < 0.01, "evenly");
        }
    }
    assert!(x(row.len() - 1) < 32.0 + step, "no further than the drag");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(placeables(&mut h).is_empty(), "one row, one command");
}

/// The placeable brush's Fill: a drag across both axes fills the
/// rectangle with rows and columns of one placeable, as one step (a
/// builder's ask: floors).
#[test]
fn area_viewer_fills_a_rectangle_with_placeables() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("paint-fill") else { return };
    let placeables = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("Placeable List").unwrap_or(&[]).to_vec()
    };
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.selected = ResKey::parse("plc_statue1", ResType::UTP);
        app.paint = true;
    }
    h.run_steps(2);
    // Fill, from the strip above the view.
    h.get_by_label("Fill").click();
    h.run_steps(2);
    assert!(h.state().settings.paint.fill && !h.state().settings.paint.row);
    h.get_by_label_contains("Filling with plc_statue1");
    let from = screen(&h, area, Vec3::new(10.0, 10.0, 0.0));
    let to = screen(&h, area, Vec3::new(18.0, 16.0, 0.0));
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=6 {
        h.hover_at(from + (to - from) * (k as f32 / 6.0));
    }
    h.run_steps(1);
    let shown = h.state().area_views[&area].paint.shown;
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let laid = placeables(&mut h);
    assert_eq!(laid.len(), shown, "what showed is what is laid");
    let distinct = |field: &str| {
        let mut v: Vec<i64> =
            laid.iter().map(|p| (p.float(field).unwrap() * 100.0).round() as i64).collect();
        v.sort();
        v.dedup();
        v.len()
    };
    let (columns, rows) = (distinct("X"), distinct("Y"));
    assert!(columns >= 2 && rows >= 2, "{columns} columns, {rows} rows");
    assert_eq!(laid.len(), columns * rows, "a full grid");
    for p in &laid {
        let (x, y) = (p.float("X").unwrap(), p.float("Y").unwrap());
        assert!((9.0..19.5).contains(&x) && (9.0..17.5).contains(&y), "in the rectangle: {x}, {y}");
        assert_eq!(p.float("Bearing"), laid[0].float("Bearing"), "facing alike");
    }
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(placeables(&mut h).is_empty(), "one fill, one command");
}

#[test]
fn area_viewer_places_draws_boxes_and_turns() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("place") else { return };
    let git_list = |h: &mut Harness<'_, Moonglow>, list: &str| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list(list).unwrap_or(&[]).to_vec()
    };
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
    };

    // A waypoint from the palette, placed on the ground where clicked.
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    h.run_steps(1);
    h.get_by_label_contains("Placing nw_waypoint001");
    click(&mut h, Vec3::new(15.0, 30.0, 0.0));
    h.run_steps(1);
    let waypoints = git_list(&mut h, "WaypointList");
    assert_eq!(waypoints.len(), 3);
    let (x, y) =
        (waypoints[2].float("XPosition").unwrap(), waypoints[2].float("YPosition").unwrap());
    assert!((x - 15.0).abs() < 0.1 && (y - 30.0).abs() < 0.1, "placed at {x}, {y}");
    assert_eq!(waypoints[2].float("YOrientation"), Some(1.0), "facing north, as Aurora");
    assert!(h.state().palette.selected.is_none(), "one click, one waypoint");
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 2)]);

    // A trigger, drawn: three corners, a double click on the fourth.
    h.state_mut().palette.selected = ResKey::parse("newgeneric", ResType::UTT);
    h.run_steps(1);
    for p in [[30.0, 30.0], [36.0, 30.0], [36.0, 36.0]] {
        click(&mut h, Vec3::new(p[0], p[1], 0.0));
    }
    let last = screen(&h, area, Vec3::new(30.0, 36.0, 0.0));
    h.hover_at(last);
    for _ in 0..2 {
        press(&h, last, true, egui::Modifiers::NONE);
        press(&h, last, false, egui::Modifiers::NONE);
    }
    h.run_steps(3);
    let triggers = git_list(&mut h, "TriggerList");
    assert_eq!(triggers.len(), 1, "the double click closes it");
    let points = triggers[0].list("Geometry").unwrap();
    assert_eq!(points.len(), 4);
    // It stands at its first corner; its points are relative to it.
    assert!((triggers[0].float("XPosition").unwrap() - 30.0).abs() < 0.1);
    assert_eq!(points[0].float("PointX"), Some(0.0));
    assert!((points[2].float("PointX").unwrap() - 6.0).abs() < 0.1);

    // Another, as a rectangle: one corner, then Alt and a click at the
    // corner opposite.
    h.state_mut().palette.selected = ResKey::parse("newgeneric", ResType::UTT);
    h.run_steps(1);
    click(&mut h, Vec3::new(10.0, 30.0, 0.0));
    let opposite = screen(&h, area, Vec3::new(14.0, 33.0, 0.0));
    h.hover_at(opposite);
    // (The right Alt of many keyboards, AltGr: Ctrl and Alt together, as
    // Windows reports it. The rectangle showed and the click did nothing.)
    let alt_gr = egui::Modifiers::ALT | egui::Modifiers::CTRL | egui::Modifiers::COMMAND;
    press(&h, opposite, true, alt_gr);
    press(&h, opposite, false, alt_gr);
    h.run_steps(3);
    let triggers = git_list(&mut h, "TriggerList");
    assert_eq!(triggers.len(), 2, "Alt and a click place the rectangle");
    let points = triggers[1].list("Geometry").unwrap();
    assert_eq!(points.len(), 4);
    assert!((points[2].float("PointX").unwrap() - 4.0).abs() < 0.2);
    assert!((points[2].float("PointY").unwrap() - 3.0).abs() < 0.2);

    // And with Alt pressed and let go before the click: many Linux window
    // managers keep an Alt + click to move the window, so the view never
    // hears of the click, and of Alt no longer. The rectangle stays.
    h.state_mut().palette.selected = ResKey::parse("newgeneric", ResType::UTT);
    h.run_steps(1);
    click(&mut h, Vec3::new(20.0, 10.0, 0.0));
    let opposite = screen(&h, area, Vec3::new(23.0, 12.0, 0.0));
    h.hover_at(opposite);
    h.run_steps(1);
    h.event_modifiers(egui::Event::PointerMoved(opposite), egui::Modifiers::ALT);
    h.run_steps(2);
    press(&h, opposite, true, egui::Modifiers::NONE);
    press(&h, opposite, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let triggers = git_list(&mut h, "TriggerList");
    assert_eq!(triggers.len(), 3, "Alt, let go, and a click place the rectangle");
    let points = triggers[2].list("Geometry").unwrap();
    assert_eq!(points.len(), 4);
    assert!((points[2].float("PointX").unwrap() - 3.0).abs() < 0.2);
    assert!((points[2].float("PointY").unwrap() - 2.0).abs() < 0.2);
    // Alt held to lead the view about (a middle drag) asks for none: the
    // next click is the outline's second corner.
    h.state_mut().palette.selected = ResKey::parse("newgeneric", ResType::UTT);
    h.run_steps(1);
    click(&mut h, Vec3::new(20.0, 20.0, 0.0));
    let (button, alt) = (egui::PointerButton::Middle, egui::Modifiers::ALT);
    for pressed in [true, false] {
        let event = egui::Event::PointerButton { pos: opposite, button, pressed, modifiers: alt };
        h.event_modifiers(event, alt);
    }
    h.run_steps(2);
    click(&mut h, Vec3::new(23.0, 22.0, 0.0));
    assert_eq!(h.state().area_views[&area].outline.len(), 2, "a corner, not a rectangle");
    assert_eq!(git_list(&mut h, "TriggerList").len(), 3);
    h.key_press(egui::Key::Escape);
    h.run_steps(2);

    // A box around the first two waypoints selects them.
    let (a, b) = (
        screen(&h, area, Vec3::new(17.0, 17.0, 0.0)),
        screen(&h, area, Vec3::new(33.0, 23.0, 0.0)),
    );
    h.hover_at(a);
    press(&h, a, true, egui::Modifiers::NONE);
    for k in 1..=4 {
        h.hover_at(a + (b - a) * (k as f32 / 4.0));
    }
    press(&h, b, false, egui::Modifiers::NONE);
    h.run_steps(2);
    let mut selected = h.state().area_views[&area].selection.clone();
    selected.sort();
    assert_eq!(selected, [(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)]);

    // Alt + right drag turns them: one command, orientations only. With
    // Shift (which shows their handles) a right drag turns the view, as
    // without it.
    let before = git_list(&mut h, "WaypointList");
    let (from, to) = (a, a + egui::vec2(-80.0, 0.0));
    let right_drag = |h: &mut Harness<'_, Moonglow>, modifiers: egui::Modifiers| {
        h.event(egui::Event::ModifiersChanged(modifiers));
        h.hover_at(from);
        let right = |pressed, pos| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers,
        };
        h.event(right(true, from));
        for k in 1..=4 {
            h.hover_at(from + (to - from) * (k as f32 / 4.0));
            h.run_steps(1);
        }
        h.event(right(false, to));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };
    let yaw = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().yaw;
    let (undo, was) = (h.state().ws.as_ref().unwrap().can_undo().map(str::to_string), yaw(&h));
    right_drag(&mut h, egui::Modifiers::SHIFT);
    assert!((yaw(&h) - was).abs() > 0.1, "Shift + right drag turns the view");
    let now = h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    assert_eq!(now, undo, "and nothing else");
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw = was;
    h.run_steps(2);
    right_drag(&mut h, egui::Modifiers::ALT);
    let after = git_list(&mut h, "WaypointList");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Rotate"));
    for i in 0..2 {
        assert_eq!(before[i].float("XPosition"), after[i].float("XPosition"));
        assert_ne!(before[i].float("XOrientation"), after[i].float("XOrientation"), "turned");
    }
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-place").join("area_view.png")).unwrap();
}

#[test]
fn placed_objects_open_their_properties() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("properties") else { return };
    let tag_of = |h: &mut Harness<'_, Moonglow>, i: usize| -> String {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let w = &git.root.list("WaypointList").unwrap()[i];
        String::from_utf8_lossy(w.string("Tag").unwrap()).into_owned()
    };
    // A double click on the second waypoint opens its Properties.
    let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    h.run_steps(40);
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
    }
    h.run_steps(3);
    let path = mg_edit::GffPath::root().item("WaypointList", 1);
    let tab = Tab::Instance { area, path };
    assert!(h.state().dock.find_tab(&tab).is_some(), "the Properties tab");
    // Its tag, edited there, changes that waypoint only: one command.
    let tag = tag_of(&mut h, 1);
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(&tag).find(is_input).expect("the tag field").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&tag).find(is_input).unwrap().type_text("WP_PLACED");
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
    assert_eq!(tag_of(&mut h, 1), "WP_PLACED");
    assert_eq!(tag_of(&mut h, 0), tag, "the other is untouched");
    // Blueprint-only fields are not offered.
    assert!(h.query_by_label("Comments").is_none());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(tag_of(&mut h, 1), tag);
}

#[test]
fn adjust_location_and_find_instance() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("adjust") else { return };
    let waypoint = |h: &mut Harness<'_, Moonglow>, i: usize| -> mg_gff::Struct {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").unwrap()[i].clone()
    };
    // Select the first waypoint; Adjust Location from its context menu.
    let at = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    h.hover_at(at);
    let right = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(right(true));
    h.event(right(false));
    h.run_steps(3);
    h.get_by_label("Adjust Location…").click();
    h.run_steps(3);
    let adjust = h.state_mut().adjust.as_mut().expect("the window");
    assert_eq!(adjust.objects, [(ObjectKind::Waypoint, 0)]);
    assert!((adjust.position.x - 20.0).abs() < 1e-3);
    // X to 12.5 and turned to face west (bearing 90): only those change.
    adjust.position.x = 12.5;
    adjust.bearing = 90.0;
    adjust.changed = [true, false, false, true];
    let before = waypoint(&mut h, 0);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let after = waypoint(&mut h, 0);
    assert_eq!(after.float("XPosition"), Some(12.5));
    assert_eq!(after.float("YPosition"), before.float("YPosition"));
    assert!((after.float("XOrientation").unwrap() + 1.0).abs() < 1e-5, "facing west");
    assert!(h.state().adjust.is_none());
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Adjust location"));

    // Find Instance: waypoints by tag, then go to one.
    let tag = String::from_utf8_lossy(waypoint(&mut h, 1).string("Tag").unwrap()).into_owned();
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Find Instance…").click();
    h.run_steps(2);
    type_into_hint(&mut h, "tag", &tag.to_lowercase());
    h.run_steps(2);
    h.get_by_label("Search").click();
    h.run_steps(2);
    let found = h.state().find_instance.as_ref().unwrap().results.clone();
    assert_eq!(found.len(), 2, "both waypoints share the blueprint's tag");
    assert!(found.iter().all(|f| f.kind == ObjectKind::Waypoint && f.area == area));
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    let second = h.get_all_by_label(&found[1].template).nth(1).expect("the second result").rect();
    h.run_steps(40);
    h.hover_at(second.center());
    for _ in 0..2 {
        press(&h, second.center(), true, egui::Modifiers::NONE);
        press(&h, second.center(), false, egui::Modifiers::NONE);
    }
    h.run_steps(4);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 1)]);
    // A tag in the game's codepage (é, one byte in Windows-1252) is found
    // as typed: it was read as UTF-8, and matched nothing.
    let edit = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::GIT),
        path: mg_edit::GffPath::root().item("WaypointList", 1),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(b"Caf\xe9".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
    h.run_steps(2);
    let f = h.state_mut().find_instance.as_mut().unwrap();
    f.tag = "café".into();
    h.run_steps(2);
    h.get_by_label("Search").click();
    h.run_steps(2);
    let found = h.state().find_instance.as_ref().unwrap().results.clone();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].index, found[0].tag.as_str()), (1, "Café"));
}

#[test]
fn copy_cut_and_paste_objects() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("paste") else { return };
    // (Options › General › Show duplicate tags warning: off by default.)
    h.state_mut().settings.warn_duplicate_tags = true;
    let waypoints = |h: &mut Harness<'_, Moonglow>| -> Vec<(f32, f32)> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root
            .list("WaypointList")
            .unwrap()
            .iter()
            .map(|w| (w.float("XPosition").unwrap(), w.float("YPosition").unwrap()))
            .collect()
    };
    // Both waypoints, copied.
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    let over = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(over);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    assert_eq!(h.state().object_clip.as_ref().map(|c| c.objects.len()), Some(2));
    // Pasted with the first at (15, 30): the second 10 m east of it.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting);
    // (Where that is now: the toolbar's lines wrap as its readout changes.)
    let over = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(over);
    press(&h, over, true, egui::Modifiers::NONE);
    press(&h, over, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let w = waypoints(&mut h);
    assert_eq!(w.len(), 4);
    let close = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.1 && (a.1 - b.1).abs() < 0.1;
    assert!(close(w[2], (15.0, 30.0)) && close(w[3], (25.0, 30.0)), "{w:?}");
    assert_eq!(
        h.state().area_views[&area].selection,
        [(ObjectKind::Waypoint, 2), (ObjectKind::Waypoint, 3)],
        "the copies are selected"
    );
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"));
    // The two waypoints' one tag is on four now, but it is their
    // blueprint's own: nothing asked.
    assert!(h.state().tag_clash.is_none());
    // Cut: the copies go (into the clipboard).
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::X);
    h.run_steps(2);
    assert_eq!(waypoints(&mut h).len(), 2);
    assert_eq!(h.state().object_clip.as_ref().map(|c| c.objects.len()), Some(2));
}

/// The classic clipboard keys in the area view: Ctrl+Insert copies the
/// objects selected, Shift+Insert pastes them, and Shift+Delete cuts them
/// (into the clipboard: Delete alone deletes, and Shift with it must not).
#[test]
fn insert_and_delete_keys_copy_cut_and_paste_objects() {
    use egui::{Key, Modifiers};
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("paste-cua") else { return };
    let waypoints = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").unwrap().len()
    };
    let clip = |h: &Harness<'_, Moonglow>| h.state().object_clip.as_ref().map(|c| c.objects.len());
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    let over = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(over);
    h.run_steps(1);
    press_now(&mut h, Modifiers::COMMAND, Key::Insert);
    assert_eq!(clip(&h), Some(2), "copied");
    // Shift+Insert asks the backend for the clipboard; its paste (the
    // text Moonglow put there) pastes the objects.
    let (_, asked) = press_now(&mut h, Modifiers::SHIFT, Key::Insert);
    assert!(asked);
    h.event(egui::Event::Paste("Moonglow Toolset: 2 objects".into()));
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting);
    let over = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(over);
    press(&h, over, true, egui::Modifiers::NONE);
    press(&h, over, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(waypoints(&mut h), 4);
    // Shift+Delete: the copies cut, into the clipboard (emptied first, to
    // tell a cut from a delete).
    h.state_mut().object_clip = None;
    press_now(&mut h, Modifiers::SHIFT, Key::Delete);
    h.run_steps(2);
    assert_eq!(waypoints(&mut h), 2);
    assert_eq!(clip(&h), Some(2), "cut, not deleted");
}

#[test]
fn context_menu_sets_states_mutes_and_adds_spawn_points() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("menu") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    // A chest, a sound and an encounter, placed.
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let square = [[-2.0, -2.0, 0.0], [2.0, -2.0, 0.0], [2.0, 2.0, 0.0], [-2.0, 2.0, 0.0]];
        let mut edits = Vec::new();
        for (name, t, x, outline) in [
            ("plc_chest1", ResType::UTP, 12.0, &[][..]),
            ("animalcriesday", ResType::UTS, 28.0, &[][..]),
            ("nw_verminbeet", ResType::UTE, 20.0, &square[..]),
        ] {
            let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
            let item = instance(&placing, t, &read(name, t), at, outline).unwrap();
            let (list, _) = mg_module::instances::git_list(t).unwrap();
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: list.into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let first = |h: &mut Harness<'_, Moonglow>, list: &str| -> mg_gff::Struct {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list(list).unwrap()[0].clone()
    };
    let right_click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    // The chest: Initial State › Opened.
    right_click(&mut h, Vec3::new(12.0, 30.0, 0.3));
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Placeable, 0)]);
    h.get_by_label_contains("Initial State").hover();
    h.run_steps(3);
    h.get_by_label("Opened").click();
    h.run_steps(3);
    assert_eq!(first(&mut h, "Placeable List").integer("AnimationState"), Some(1));
    // Inventory opens its Properties at the inventory page.
    right_click(&mut h, Vec3::new(12.0, 30.0, 0.3));
    h.get_by_label("Inventory").click();
    h.run_steps(3);
    let path = mg_edit::GffPath::root().item("Placeable List", 0);
    assert!(h.state().dock.find_tab(&Tab::Instance { area, path: path.clone() }).is_some());
    assert_eq!(h.state().blueprint_pages.get(&(git_key, path)), Some(&"Inventory"));
    close_windows(&mut h);
    h.run_steps(3);
    // The sound: Mute.
    right_click(&mut h, Vec3::new(28.0, 30.0, 1.9));
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Sound, 0)]);
    h.get_by_label("Mute").click();
    h.run_steps(3);
    assert_eq!(first(&mut h, "SoundList").integer("Active"), Some(0));
    // The encounter: a spawn point where the menu was opened.
    right_click(&mut h, Vec3::new(21.0, 31.0, 0.0));
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Encounter, 0)]);
    h.get_by_label("Add Spawn Point").click();
    h.run_steps(3);
    let spawns = first(&mut h, "Encounter List").list("SpawnPointList").unwrap().to_vec();
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].id, 2);
    assert!((spawns[0].float("X").unwrap() - 21.0).abs() < 0.1);
    assert!((spawns[0].float("Y").unwrap() - 31.0).abs() < 0.1);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-menu").join("area_view.png")).unwrap();
}

#[test]
fn doors_go_on_door_hooks() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    // A castle tile with three door hooks (east one at 20, 15, facing 270°).
    let Some((mut h, area)) = area_harness_on("doors", "tic01", Some(7)) else { return };
    let doors = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("Door List").unwrap_or(&[]).to_vec()
    };
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    // Far from any hook: nothing, and a word why.
    h.state_mut().palette.selected = ResKey::parse("nw_door_normal", ResType::UTD);
    h.run_steps(1);
    click(&mut h, Vec3::new(35.0, 35.0, 0.0));
    assert!(doors(&mut h).is_empty());
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("door hooks")));
    // Near the east hook, inside the tile: on the hook, turned as it is.
    click(&mut h, Vec3::new(18.5, 15.5, 0.0));
    let placed = doors(&mut h);
    assert_eq!(placed.len(), 1);
    let d = &placed[0];
    assert_eq!((d.float("X"), d.float("Y"), d.float("Z")), (Some(20.0), Some(15.0), Some(0.0)));
    assert!((d.float("Bearing").unwrap() + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Door, 0)]);
}

#[test]
fn several_objects_edited_together() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("multi") else { return };
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    let at = screen(&h, area, Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Properties").click();
    h.run_steps(3);
    let paths = vec![
        mg_edit::GffPath::root().item("WaypointList", 0),
        mg_edit::GffPath::root().item("WaypointList", 1),
    ];
    assert!(h.state().dock.find_tab(&Tab::Instances { area, paths }).is_some());
    h.get_by_label_contains("what you change is set on each");
    h.get_by_label("Advanced").click();
    h.run_steps(2);
    h.get_by_label("Waypoint Contains a Map Note").click();
    h.run_steps(3);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap().root.clone();
    let notes: Vec<_> =
        git.list("WaypointList").unwrap().iter().map(|w| w.integer("HasMapNote")).collect();
    assert_eq!(notes, [Some(1), Some(1)], "both");
    assert_eq!(ws.can_undo(), Some("Waypoint Contains a Map Note"));
}

#[test]
fn walkmesh_overlay_and_area_statistics() {
    let Some((mut h, area)) = area_harness("stats") else { return };
    h.get_by_label("👣 Walkmesh…").click();
    h.run_steps(3);
    assert!(h.state().area_views[&area].walkmesh);
    h.get_by_label("Build").click();
    h.run_steps(2);
    h.get_by_label("Area Statistics").click();
    h.run_steps(3);
    h.get_by_label("Resources Used");
    h.get_by_label("Triangles");
    h.get_by_label("16 (4 by 4)");
    h.get_by_label("Done").click();
    h.run_steps(2);
    assert!(h.state().area_stats.is_none());
    if std::env::var_os("MOONGLOW_SCREENS").is_some() {
        let image = h.render().expect("render");
        let out = mg_testkit::output_dir("screens");
        image.save(out.join("walkmesh.png")).unwrap();
    }
}

/// What a paste warns of by default (`paste_warning = "custom"`): a tag a
/// builder set (a door renamed), not a blueprint's own (a barrel as
/// placed from the palette), and never a static placeable's; with
/// `"any"`, the barrel too, the static one still not.
#[test]
fn a_paste_warns_of_builders_tags_not_scenery() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("paste-custom") else { return };
    // (Options › General › Show duplicate tags warning: off by default.)
    h.state_mut().settings.warn_duplicate_tags = true;
    let git = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = |x: f32| Placement { position: [x, 15.0, 0.0], rotation: 0.0 };
        let barrel =
            instance(&placing, ResType::UTP, &read("plc_barrel", ResType::UTP), at(10.0), &[]);
        let mut rock = barrel.clone().unwrap();
        rock.set("Static", mg_gff::Value::Byte(1));
        rock.set("XPosition", mg_gff::Value::Float(20.0));
        rock.set("Tag", mg_gff::Value::String(b"MG_BIG_ROCK".to_vec()));
        let mut door =
            instance(&placing, ResType::UTD, &read("nw_door_normal", ResType::UTD), at(30.0), &[])
                .unwrap();
        door.set("Tag", mg_gff::Value::String(b"KREZK_RENTAL_DOOR".to_vec()));
        let insert = |list: &str, index, item| mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: list.into(),
            index,
            item,
        };
        let edits = vec![
            insert("Placeable List", 0, barrel.unwrap()),
            insert("Placeable List", 1, rock),
            insert("Door List", 0, door),
        ];
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let paste = |h: &mut Harness<'_, Moonglow>, picked: (ObjectKind, usize), at: Vec3| {
        h.state_mut().tag_clash = None;
        h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![picked];
        let over = screen(h, area, at);
        h.hover_at(over);
        h.run_steps(1);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
        h.run_steps(1);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
        h.run_steps(2);
        let over = screen(h, area, at);
        h.hover_at(over);
        press(h, over, true, egui::Modifiers::NONE);
        press(h, over, false, egui::Modifiers::NONE);
        h.run_steps(3);
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"));
        h.state().tag_clash.clone()
    };
    let barrel = (ObjectKind::Placeable, 0);
    let rock = (ObjectKind::Placeable, 1);
    let door = (ObjectKind::Door, 0);
    assert!(paste(&mut h, barrel, Vec3::new(12.0, 30.0, 0.0)).is_none(), "the blueprint's tag");
    assert!(paste(&mut h, rock, Vec3::new(16.0, 30.0, 0.0)).is_none(), "a static placeable");
    let clash = paste(&mut h, door, Vec3::new(20.0, 30.0, 0.0)).expect("a renamed door");
    assert_eq!(clash.clashes[0].sentence(), "KREZK_RENTAL_DOOR is now on two objects");
    // "any": the barrel's too; the static rock's never.
    h.state_mut().ws.as_mut().unwrap().module.project_file.tags.paste_warning =
        Some(mg_module::project_file::PasteWarning::Any);
    let clash = paste(&mut h, barrel, Vec3::new(24.0, 30.0, 0.0)).expect("any tag");
    assert!(clash.clashes[0].sentence().ends_with("is now on three objects"), "{clash:?}");
    assert!(paste(&mut h, rock, Vec3::new(28.0, 30.0, 0.0)).is_none(), "a static placeable");
}

/// Every kind is weighed alike (Troubletcat's report, October 2026: the
/// window came up for waypoints and triggers, not for creatures): a
/// creature, a waypoint, a trigger and a store pasted with their
/// blueprint's tag ask nothing, each with a tag set by hand asks (a
/// store's blueprint found by its `ResRef`). With Options › General ›
/// Show duplicate tags warning off (the default), nothing asks unless the
/// project's moonglow.toml says to; the window's Don't warn again turns
/// the option off.
#[test]
fn a_paste_weighs_every_kind_alike_and_the_option_silences_it() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("paste-kinds") else { return };
    assert!(!h.state().settings.warn_duplicate_tags, "off by default");
    h.state_mut().settings.warn_duplicate_tags = true;
    let git = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = |x: f32| Placement { position: [x, 15.0, 0.0], rotation: 0.0 };
        let square = [[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [3.0, 3.0, 0.0], [0.0, 3.0, 0.0]];
        // Two of each: the blueprint's tag, and one set by hand.
        let two = |name: &str, t: ResType, x: f32, outline: &[[f32; 3]], tag: &[u8]| {
            let bp = read(name, t);
            let first = instance(&placing, t, &bp, at(x), outline).unwrap();
            let mut second = instance(&placing, t, &bp, at(x + 4.0), outline).unwrap();
            second.set("Tag", mg_gff::Value::String(tag.to_vec()));
            [first, second]
        };
        let creatures = two("nw_bandit001", ResType::UTC, 6.0, &[], b"MG_BANDIT_BOSS");
        let triggers = two("newgeneric", ResType::UTT, 14.0, &square, b"MG_AMBUSH");
        let stores = two("nw_storethief001", ResType::UTM, 24.0, &[], b"MG_FENCE");
        let mut edits = Vec::new();
        for (list, items) in
            [("Creature List", creatures), ("TriggerList", triggers), ("StoreList", stores)]
        {
            for (index, item) in items.into_iter().enumerate() {
                edits.push(mg_edit::Edit::InsertItem {
                    key: git,
                    path: mg_edit::GffPath::root(),
                    list: list.into(),
                    index,
                    item,
                });
            }
        }
        // (The harness's two waypoints have their blueprint's tag; the
        // second one a spawn system's.)
        edits.push(mg_edit::Edit::SetField {
            key: git,
            path: mg_edit::GffPath::root().item("WaypointList", 1),
            label: "Tag".into(),
            value: Some(mg_gff::Value::String(b"sca_kt_elite_war".to_vec())),
        });
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let paste = |h: &mut Harness<'_, Moonglow>, picked: (ObjectKind, usize), at: Vec3| {
        h.state_mut().tag_clash = None;
        h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![picked];
        let over = screen(h, area, at);
        h.hover_at(over);
        h.run_steps(1);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
        h.run_steps(1);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
        h.run_steps(2);
        let over = screen(h, area, at);
        h.hover_at(over);
        press(h, over, true, egui::Modifiers::NONE);
        press(h, over, false, egui::Modifiers::NONE);
        h.run_steps(3);
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"), "{picked:?}");
        h.state().tag_clash.clone()
    };
    let kinds =
        [ObjectKind::Creature, ObjectKind::Waypoint, ObjectKind::Trigger, ObjectKind::Store];
    for (i, kind) in kinds.into_iter().enumerate() {
        let at = Vec3::new(6.0 + 8.0 * i as f32, 26.0, 0.0);
        assert_eq!(paste(&mut h, (kind, 0), at), None, "{kind:?} with its blueprint's tag");
    }
    let set = ["MG_BANDIT_BOSS", "sca_kt_elite_war", "MG_AMBUSH", "MG_FENCE"];
    for (i, (kind, tag)) in kinds.into_iter().zip(set).enumerate() {
        let at = Vec3::new(6.0 + 8.0 * i as f32, 34.0, 0.0);
        let clash = paste(&mut h, (kind, 1), at).unwrap_or_else(|| panic!("{kind:?}: {tag}"));
        let said: Vec<String> = clash.clashes.iter().map(|c| c.sentence()).collect();
        assert_eq!(said, [format!("{tag} is now on two objects")], "{kind:?}");
        assert!(clash.from_option);
    }
    // The option off: a tag set by hand asks nothing.
    h.state_mut().settings.warn_duplicate_tags = false;
    let creature = (ObjectKind::Creature, 1);
    assert_eq!(paste(&mut h, creature, Vec3::new(6.0, 30.0, 0.0)), None, "the option off");
    // On, the window's Don't warn again turns it off, and says where it is.
    h.state_mut().settings.warn_duplicate_tags = true;
    let waypoint = (ObjectKind::Waypoint, 1);
    assert!(paste(&mut h, waypoint, Vec3::new(14.0, 30.0, 0.0)).is_some());
    h.get_by_label("Don't warn again").click();
    h.run_steps(2);
    assert!(!h.state().settings.warn_duplicate_tags);
    let said =
        |h: &Harness<'_, Moonglow>, s: &str| h.state().log.entries.iter().any(|e| e.1.contains(s));
    assert!(said(&h, "Options › General › Show duplicate tags warning turns it back on"));
    h.get_by_label("Keep Tags").click();
    h.run_steps(2);
    assert!(h.state().tag_clash.is_none());
    assert_eq!(paste(&mut h, waypoint, Vec3::new(22.0, 30.0, 0.0)), None, "not again");
    // The project's moonglow.toml asking for it wins over the option, and
    // the window offers no Don't warn again then.
    h.state_mut().ws.as_mut().unwrap().module.project_file.tags.paste_warning =
        Some(mg_module::project_file::PasteWarning::Custom);
    let store = (ObjectKind::Store, 1);
    let clash = paste(&mut h, store, Vec3::new(30.0, 30.0, 0.0)).expect("the project's rule");
    assert!(!clash.from_option);
    assert!(h.query_by_label("Don't warn again").is_none());
    assert!(
        h.query_by_label("The project's moonglow.toml asks for this warning ([tags]).").is_some()
    );
}

/// A paste that puts a tag on a second object says so and offers Retag:
/// the copy's tag with a number, unique in the module, one step to undo.
/// The project file can say to warn of none.
#[test]
fn a_paste_that_doubles_a_tag_offers_retag() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("paste-retag") else { return };
    // (Options › General › Show duplicate tags warning: off by default.)
    h.state_mut().settings.warn_duplicate_tags = true;
    let git = ResKey::new(area, ResType::GIT);
    let tags = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let g = ws.doc(&git).unwrap();
        let list = g.root.list("WaypointList").unwrap();
        list.iter()
            .map(|w| String::from_utf8_lossy(w.string("Tag").unwrap()).into_owned())
            .collect()
    };
    // The second waypoint's tag its own.
    let edit = mg_edit::Edit::SetField {
        key: git,
        path: mg_edit::GffPath::root().item("WaypointList", 1),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(b"TDOD_START_KREZK".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
    h.run_steps(2);
    let paste = |h: &mut Harness<'_, Moonglow>, at: Vec3| {
        h.state_mut().area_views.get_mut(&area).unwrap().selection =
            vec![(ObjectKind::Waypoint, 1)];
        let over = screen(h, area, at);
        h.hover_at(over);
        h.run_steps(1);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
        h.run_steps(1);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
        h.run_steps(2);
        let over = screen(h, area, at);
        h.hover_at(over);
        press(h, over, true, egui::Modifiers::NONE);
        press(h, over, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    paste(&mut h, Vec3::new(15.0, 30.0, 0.0));
    let clash = h.state().tag_clash.clone().expect("the Duplicate Tags window");
    assert_eq!(clash.clashes[0].sentence(), "TDOD_START_KREZK is now on two objects");
    // (In the window, and in the log.)
    assert!(h.query_by_label("• TDOD_START_KREZK is now on two objects").is_some());
    h.get_by_label("Retag").click();
    h.run_steps(2);
    assert!(h.state().tag_clash.is_none());
    let t = tags(&mut h);
    assert_eq!(&t[1..], ["TDOD_START_KREZK", "TDOD_START_KREZK_2"]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Retag"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(&tags(&mut h)[1..], ["TDOD_START_KREZK", "TDOD_START_KREZK"], "one undo");
    // The project's rules: warn of the unique ones only, and none here.
    {
        let rules = &mut h.state_mut().ws.as_mut().unwrap().module.project_file.tags;
        rules.paste_warning = Some(mg_module::project_file::PasteWarning::Unique);
        rules.unique = mg_module::project_file::Patterns::new(&["TDOD_START_*"]).unwrap();
    }
    h.state_mut().tag_clash = None;
    paste(&mut h, Vec3::new(25.0, 35.0, 0.0));
    let clash = h.state().tag_clash.clone().expect("a unique tag");
    assert_eq!(
        clash.clashes[0].sentence(),
        "TDOD_START_KREZK is now on three objects (the project wants TDOD_START_* unique)"
    );
    h.state_mut().tag_clash = None;
    h.state_mut().ws.as_mut().unwrap().module.project_file.tags.paste_warning =
        Some(mg_module::project_file::PasteWarning::Off);
    paste(&mut h, Vec3::new(25.0, 25.0, 0.0));
    assert!(h.state().tag_clash.is_none(), "the project wants no warning");
    assert_eq!(tags(&mut h).len(), 5);
}

#[test]
fn the_area_view_plays_the_area_s_sounds() {
    use mg_ui::audio::{Channel, Silence};
    let Some((mut h, area)) = area_harness("sounds") else { return };
    let speaker = std::rc::Rc::new(std::cell::RefCell::new(Silence::default()));
    h.state_mut().speaker = Box::new(speaker.clone());
    // The first rows of ambientsound.2da and ambientmusic.2da with a sound.
    let first = |table: &str| -> (i32, ResRef) {
        let t = h.state().game.as_deref().unwrap().table(table).unwrap();
        (0..t.len())
            .find_map(|r| {
                let name = ResRef::from_str(t.get(r, "Resource")?).ok()?;
                (!name.is_empty()).then_some((r as i32, name))
            })
            .unwrap()
    };
    let (sound_row, sound) = first("ambientsound");
    let (music_row, music) = first("ambientmusic");
    // A bell 8 m east of where the view looks, full volume within 5 m,
    // nothing past 18 m; the area's ambient sound and music by day.
    let target = h.state().area_views[&area].orbit.as_ref().unwrap().target;
    let key = ResKey::new(area, ResType::GIT);
    let mut git = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
    let mut bell = mg_gff::Struct::new(6);
    let mut entry = mg_gff::Struct::new(0);
    entry.set("Sound", mg_gff::Value::resref(ResRef::from_str("as_cv_bell1").unwrap()));
    bell.set("Sounds", mg_gff::Value::List(vec![entry]));
    for (label, v) in [("XPosition", target.x + 8.0), ("YPosition", target.y), ("ZPosition", 0.0)] {
        bell.set(label, mg_gff::Value::Float(v));
    }
    bell.set("MinDistance", mg_gff::Value::Float(5.0));
    bell.set("MaxDistance", mg_gff::Value::Float(18.0));
    for (label, v) in [("Active", 1), ("Positional", 1), ("Continuous", 1), ("Volume", 127)] {
        bell.set(label, mg_gff::Value::Byte(v));
    }
    git.root.set("SoundList", mg_gff::Value::List(vec![bell]));
    let props = git.root.child_mut("AreaProperties").unwrap();
    props.set("AmbientSndDay", mg_gff::Value::Int(sound_row));
    props.set("AmbientSndDayVol", mg_gff::Value::Byte(64));
    props.set("MusicDay", mg_gff::Value::Int(music_row));
    let edit = mg_edit::Edit::SetResource { key, data: git.to_bytes().ok() };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("sounds", vec![edit])));
    h.state_mut().area_views.get_mut(&area).unwrap().night = false;
    // (An area's sounds come up over its first three seconds.)
    h.run_steps(240);
    // Aurora's defaults: the placed sound only, at 1 − (8 − 5) / 13.
    let playing = |c: Channel| speaker.borrow().channels.get(&c).cloned();
    let (name, gain, looped) = playing(Channel::Placed(0)).expect("the bell");
    assert_eq!(name.to_string(), "as_cv_bell1");
    assert!((gain - 10.0 / 13.0).abs() < 1e-3, "{gain}");
    assert!(!looped);
    assert!(playing(Channel::Ambient).is_none() && playing(Channel::Music).is_none());
    // Ambient sound and music on: looped, at the area's and Aurora's volumes.
    h.state_mut().settings.ambient_sound = true;
    h.state_mut().settings.ambient_music = true;
    h.run_steps(2);
    let (a, a_volume, a_looped) = playing(Channel::Ambient).expect("the ambient sound");
    assert_eq!((a, a_looped), (sound, true));
    assert!((a_volume - 64.0 / 127.0).abs() < 1e-4);
    let (m, m_volume, _) = playing(Channel::Music).expect("the music");
    assert_eq!(m, music);
    assert!((m_volume - 92.0 / 127.0).abs() < 1e-4);
    // Placed sounds off; then the view closes and everything stops.
    h.state_mut().settings.no_placed_sounds = true;
    h.run_steps(2);
    assert!(playing(Channel::Placed(0)).is_none());
    let tab = h.state().dock.find_tab(&Tab::Area(area)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.run_steps(2);
    assert!(speaker.borrow().channels.is_empty(), "{:?}", speaker.borrow().channels);
}

#[test]
fn blueprints_drag_from_the_module_tree_into_the_area() {
    let Some((mut h, area)) = area_harness("drag-tree") else { return };
    // A waypoint blueprint of the module's own (the game's Tavern's copy).
    let key = ResKey::parse("mg_tree_wp", ResType::UTW).unwrap();
    let tavern = ResKey::parse("nw_wp_tavern", ResType::UTW).unwrap();
    let data = h.state().game.as_deref().unwrap().resman.get(&tavern).unwrap().into_owned();
    h.state_mut().ws.as_mut().unwrap().module.set(key, data);
    h.run_steps(2);
    let count = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").map_or(0, <[mg_gff::Struct]>::len)
    };
    let before = count(&mut h);
    h.get_by_label_contains("Waypoints (").click();
    h.run_steps(2);
    let from = h.get_by_label("mg_tree_wp.utw").rect().center();
    let to = screen(&h, area, glam::Vec3::new(25.0, 25.0, 0.0));
    let modifiers = egui::Modifiers::NONE;
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    press(&h, from, true, modifiers);
    h.run_steps(1);
    for t in [0.1, 0.4, 0.7, 1.0] {
        h.event(egui::Event::PointerMoved(from + (to - from) * t));
        h.run_steps(1);
    }
    // Before it is let go, its ghost shows where it would go; Shift + Q
    // turns it a quarter anticlockwise.
    let ghost = h.state().area_views[&area].ghost_shown.clone().expect("a ghost");
    assert_eq!(ghost.kind, mg_area::ObjectKind::Waypoint);
    let at = ghost.position;
    assert!((at.x - 25.0).abs() < 0.5 && (at.y - 25.0).abs() < 0.5, "the ghost is at {at}");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Q);
    h.run_steps(1);
    let turned = h.state().area_views[&area].ghost_shown.as_ref().unwrap().rotation;
    assert!((turned - ghost.rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    press(&h, to, false, modifiers);
    h.run_steps(3);
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    let placed = model
        .objects
        .iter()
        .find(|o| o.kind == mg_area::ObjectKind::Waypoint && o.index == before)
        .expect("the waypoint is placed");
    let d = (placed.rotation - turned).rem_euclid(std::f32::consts::TAU);
    assert!(d < 1e-3 || d > std::f32::consts::TAU - 1e-3, "placed at {}", placed.rotation);
    assert_eq!(count(&mut h), before + 1, "{:?}", h.state().log.entries);
    let (x, y, _) = waypoint(&mut h, area, before).unwrap();
    assert!((x - 25.0).abs() < 0.5 && (y - 25.0).abs() < 0.5, "dropped at {x}, {y}");
    // Placed: no ghost.
    assert!(h.state().area_views[&area].ghost_shown.is_none());
}

#[test]
fn a_placeable_dragged_over_the_area_shows_its_model_where_it_would_go() {
    let Some((mut h, area)) = area_harness("drag-ghost") else { return };
    let key = ResKey::parse("mg_ghost_chest", ResType::UTP).unwrap();
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let data = h.state().game.as_deref().unwrap().resman.get(&chest).unwrap().into_owned();
    h.state_mut().ws.as_mut().unwrap().module.set(key, data);
    h.run_steps(2);
    h.get_by_label_contains("Placeables (").click();
    h.run_steps(2);
    let from = h.get_by_label("mg_ghost_chest.utp").rect().center();
    let to = screen(&h, area, glam::Vec3::new(20.0, 22.0, 0.0));
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for t in [0.1, 0.5, 1.0] {
        h.event(egui::Event::PointerMoved(from + (to - from) * t));
        h.run_steps(1);
    }
    let ghost = h.state().area_views[&area].ghost_shown.clone().expect("a ghost");
    assert!(ghost.preview.is_some(), "the chest has a model to show");
    assert!((ghost.position.x - 20.0).abs() < 0.5 && (ghost.position.y - 22.0).abs() < 0.5);
    assert!(ghost.rotation.abs() < 1e-4);
    // E twice turns it clockwise by 15° each (the selection stays as it is).
    h.key_press(egui::Key::E);
    h.run_steps(1);
    h.key_press(egui::Key::E);
    h.run_steps(1);
    let turned = h.state().area_views[&area].ghost_shown.as_ref().unwrap().rotation;
    assert!((turned + 30f32.to_radians()).abs() < 1e-4, "turned to {}", turned.to_degrees());
    // Moved off the view (onto the module tree), it goes.
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(2);
    assert!(h.state().area_views[&area].ghost_shown.is_none());
    // Back over the view and let go: placed as the ghost was turned.
    h.event(egui::Event::PointerMoved(to));
    h.run_steps(2);
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    // (The last placeable: the copy keeps plc_chest1 as its template.)
    let placed = model
        .objects
        .iter()
        .filter(|o| o.kind == mg_area::ObjectKind::Placeable)
        .max_by_key(|o| o.index)
        .expect("the chest is placed");
    assert!((placed.rotation - turned).abs() < 1e-3, "placed at {}", placed.rotation.to_degrees());
}

#[test]
fn a_blueprint_about_to_be_placed_shows_in_the_views_picture() {
    // A dark stone statue, see-through over the ground, shows in the
    // picture: not only in the view's counts, which other tests hold.
    let Some((mut h, area)) = area_harness("ghost-picture") else { return };
    if let Some(o) = &mut h.state_mut().area_views.get_mut(&area).unwrap().orbit {
        o.target = glam::Vec3::new(20.0, 22.0, 1.0);
        (o.pitch, o.yaw, o.distance) = (40f32.to_radians(), -60f32.to_radians(), 8.0);
    }
    h.run_steps(2);
    let at = screen(&h, area, glam::Vec3::new(20.0, 22.0, 0.0));
    let top = screen(&h, area, glam::Vec3::new(20.0, 22.0, 2.0));
    let shot = |h: &mut Harness<'_, Moonglow>, chosen: Option<&str>| {
        h.state_mut().palette.selected = chosen.map(|k| ResKey::parse(k, ResType::UTP).unwrap());
        h.hover_at(at);
        h.run_steps(3);
        (h.render().expect("render"), h.state().area_views[&area].ghost_shown.is_some())
    };
    let (with, shown) = shot(&mut h, Some("plc_statue1"));
    assert!(shown, "the statue's ghost is under the pointer");
    let (without, shown) = shot(&mut h, None);
    assert!(!shown);
    let dir = mg_testkit::scratch_dir("ui-ghost-picture");
    with.save(dir.join("with.png")).unwrap();
    without.save(dir.join("without.png")).unwrap();
    // Around the statue, where it stands up from the ground (not its
    // box's cyan outline): how many pixels it changes, and by how much.
    let ppp = h.ctx.pixels_per_point();
    let (x0, x1) = ((at.x - 30.0) * ppp, (at.x + 30.0) * ppp);
    let (y0, y1) = ((top.y.min(at.y) - 10.0) * ppp, (top.y.max(at.y) + 10.0) * ppp);
    let luma = |p: &image::Rgba<u8>| {
        0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2])
    };
    let (mut changed, mut most) = (0, 0.0f32);
    for y in y0.max(0.0) as u32..(y1 as u32).min(with.height()) {
        for x in x0.max(0.0) as u32..(x1 as u32).min(with.width()) {
            let p = with.get_pixel(x, y);
            if i32::from(p[2]) - i32::from(p[0]) > 60 {
                continue;
            }
            let d = (luma(p) - luma(without.get_pixel(x, y))).abs();
            most = most.max(d);
            changed += usize::from(d > 12.0);
        }
    }
    // (About 4,300 pixels at 0.8 opaque, 3,300 at 0.6, 1,400 at 0.3;
    // 250, the outline's soft edges, with the statue not drawn.)
    assert!(changed > 2000, "the ghost changed {changed} pixels, {most} at most");
}

#[test]
fn blueprints_drag_from_the_palette_into_the_area() {
    let Some((mut h, area)) = area_harness("drag-place") else { return };
    // The standard Tavern waypoint, in the palette beside the area.
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    h.state_mut().palette.kind = mg_module::palette::BlueprintKind::Waypoint;
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.filter = "nw_wp_tavern".into();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    let count = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").map_or(0, <[mg_gff::Struct]>::len)
    };
    assert_eq!(count(&mut h), 2);
    // Chosen in the palette (a click), then pressed on, dragged over the
    // area, let go there.
    h.get_by_label(&tavern).click();
    h.run_steps(1);
    let from = h.get_by_label(&tavern).rect().center();
    let to = screen(&h, area, glam::Vec3::new(25.0, 25.0, 0.0));
    let modifiers = egui::Modifiers::NONE;
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    press(&h, from, true, modifiers);
    h.run_steps(1);
    for t in [0.1, 0.4, 0.7, 1.0] {
        h.event(egui::Event::PointerMoved(from + (to - from) * t));
        h.run_steps(1);
    }
    press(&h, to, false, modifiers);
    h.run_steps(3);
    assert_eq!(count(&mut h), 3, "{:?}", h.state().log.entries);
    // Remembered in the palette's Recent.
    assert_eq!(
        h.state().settings.palette_recent.first().map(String::as_str),
        Some("utw:nw_wp_tavern")
    );
    let (x, y, _) = waypoint(&mut h, area, 2).unwrap();
    assert!((x - 25.0).abs() < 0.5 && (y - 25.0).abs() < 0.5, "dropped at {x}, {y}");
    // What was dropped moves at once, like anything else: dragged, not
    // placed again.
    let at = screen(&h, area, glam::Vec3::new(x, y, 0.0));
    let to = screen(&h, area, glam::Vec3::new(30.0, 25.0, 0.0));
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    press(&h, at, true, modifiers);
    h.run_steps(1);
    for t in [0.2, 0.6, 1.0] {
        h.event(egui::Event::PointerMoved(at + (to - at) * t));
        h.run_steps(1);
    }
    press(&h, to, false, modifiers);
    h.run_steps(3);
    assert_eq!(count(&mut h), 3, "moved, not placed again");
    let (x, _, _) = waypoint(&mut h, area, 2).unwrap();
    assert!((x - 30.0).abs() < 0.5, "moved to {x}");
}

#[test]
fn the_start_location_shows_on_raised_ground() {
    // Medieval Rural 2 builds its ground 5 m up; a new module's start
    // location is stored at height 0.
    let Some((mut h, area)) = area_harness_on("start-raised", "trm02", None) else { return };
    h.run_steps(3);
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let stored = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let r = ws.doc(&ifo).unwrap().root.clone();
        (r.read(&ifo::MOD_ENTRY_AREA), r.read(&ifo::MOD_ENTRY_Z))
    };
    assert_eq!(stored(&mut h), (area, 0.0));
    // (The button is at the end of a toolbar's row, which ends at the
    // tab's edge in a window this narrow too.)
    h.get_by_label("🏃 Go to Start Location").click();
    h.run_steps(2);
    let target = h.state().area_views[&area].orbit.unwrap().target;
    assert!((target.z - 5.0).abs() < 0.5, "looks at the ground, not under it: {target}");
    assert_eq!(stored(&mut h), (area, 0.0), "the module's start is as it was");
}

/// The main toolbar's Start opens the area the module starts in, with its
/// view at the start location (as Aurora's Go to Start Location does).
#[test]
fn go_to_start_location_opens_the_start_s_area() {
    let Some((mut h, area)) = area_harness_on("start-go", "trm02", None) else { return };
    h.run_steps(3);
    let start = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let ifo = ResKey::parse("module", ResType::IFO).unwrap();
        let r = ws.doc(&ifo).unwrap().root.clone();
        glam::Vec2::new(r.read(&ifo::MOD_ENTRY_X), r.read(&ifo::MOD_ENTRY_Y))
    };
    // The view looks elsewhere, and its tab is closed.
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().target =
        glam::Vec3::new(start.x + 25.0, start.y + 25.0, 0.0);
    close_tab(h.state_mut(), &Tab::Area(area));
    h.run_steps(2);
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_none());
    h.get_by_label("🏃 Start").click();
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_some(), "the area is opened");
    let target = h.state().area_views[&area].orbit.unwrap().target;
    assert!(target.truncate().distance(start) < 0.01, "{target} is not at {start}");
    // (The command is in the View menu and the command palette too.)
    assert!(mg_ui::commands::Id::all().contains(&mg_ui::commands::Id::GoToStart));
}

/// A trigger's outline drawn as a rectangle by two clicks: the first
/// corner, then Alt and a click at the corner opposite.
#[test]
fn an_outline_is_drawn_as_a_rectangle_by_two_clicks() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("outline-rect") else { return };
    h.run_steps(3);
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
    h.run_steps(3);
    h.state_mut().area_views.get_mut(&area).unwrap().redraw =
        Some((mg_area::ObjectKind::Trigger, 0));
    h.run_steps(2);
    let (first, opposite) = (Vec3::new(18.0, 18.0, 0.0), Vec3::new(24.0, 21.0, 0.0));
    let click = |h: &mut Harness<'_, Moonglow>, at: Vec3, modifiers: egui::Modifiers| {
        let on = screen(h, area, at);
        h.hover_at(on);
        press(h, on, true, modifiers);
        press(h, on, false, modifiers);
        h.run_steps(3);
    };
    click(&mut h, first, egui::Modifiers::NONE);
    assert!(h.state().area_views[&area].redraw.is_some(), "one corner: not closed");
    click(&mut h, opposite, egui::Modifiers::ALT);
    assert!(h.state().area_views[&area].redraw.is_none(), "closed by the second click");
    let corners: Vec<glam::Vec2> = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let t = ws.doc(&git).unwrap().root.list("TriggerList").unwrap()[0].clone();
        let at = glam::Vec2::new(t.float("XPosition").unwrap(), t.float("YPosition").unwrap());
        t.list("Geometry")
            .unwrap()
            .iter()
            .map(|p| at + glam::Vec2::new(p.float("PointX").unwrap(), p.float("PointY").unwrap()))
            .collect()
    };
    assert_eq!(corners.len(), 4, "{corners:?}");
    let near = |a: glam::Vec2, x: f32, y: f32| a.distance(glam::Vec2::new(x, y)) < 0.2;
    assert!(near(corners[0], 18.0, 18.0) && near(corners[2], 24.0, 21.0), "{corners:?}");
    // Its sides run east and north: the other two corners share theirs.
    assert!(
        (corners[1].y - corners[0].y).abs() < 1e-4 && (corners[1].x - corners[2].x).abs() < 1e-4
    );
    assert!(
        (corners[3].x - corners[0].x).abs() < 1e-4 && (corners[3].y - corners[2].y).abs() < 1e-4
    );
}

/// A picture: the turning ring around a trigger whose outline is on
/// ground built 5 m up while its own place is stored at height 0 (as
/// Aurora stores it), before and after a quarter turn. To
/// `target/test-output/ring-trigger/`. (And held: where the ring is, and
/// that its mark goes round.)
#[test]
fn a_trigger_s_turning_ring_in_a_picture() {
    let Some((mut h, area)) = area_harness_on("ring-trigger", "trm02", None) else { return };
    h.run_steps(3);
    let git = ResKey::new(area, ResType::GIT);
    let mut trigger = mg_gff::Struct::new(1);
    for (label, v) in [("XPosition", 18.0), ("YPosition", 20.0), ("ZPosition", 0.0)] {
        trigger.set(label, mg_gff::Value::Float(v));
    }
    let corner = |x: f32, y: f32| {
        let mut p = mg_gff::Struct::new(3);
        p.set("PointX", mg_gff::Value::Float(x));
        p.set("PointY", mg_gff::Value::Float(y));
        p.set("PointZ", mg_gff::Value::Float(5.0));
        p
    };
    let corners = vec![corner(0.0, 0.0), corner(8.0, 0.0), corner(8.0, 1.5), corner(0.0, 1.5)];
    trigger.set("Geometry", mg_gff::Value::List(corners));
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
    h.run_steps(3);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(mg_area::ObjectKind::Trigger, 0)];
        if let Some(o) = &mut view.orbit {
            o.target = glam::Vec3::new(22.0, 20.75, 5.0);
            o.distance = 22.0;
            o.pitch = 40f32.to_radians();
        }
    }
    h.run_steps(6);
    // The ring is around the outline's middle at the outline's own height
    // (not the trigger's stored 0), marked toward its first corner.
    let rings = h.state().area_views[&area].ring_places();
    assert_eq!(rings.len(), 1);
    let (pivot, mark) = rings[0];
    assert!(pivot.distance(glam::Vec3::new(22.0, 20.75, 5.0)) < 1e-3, "{pivot}");
    let to_first = glam::Vec2::new(18.0 - 22.0, 20.0 - 20.75);
    assert!((mark - to_first.y.atan2(to_first.x)).abs() < 1e-3, "{mark}");
    let dir = mg_testkit::output_dir("ring-trigger");
    h.render().unwrap().save(dir.join("before-turn.png")).unwrap();
    // (The keys are the view's with the pointer over it.)
    h.hover_at(egui::pos2(700.0, 560.0));
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(6);
    h.render().unwrap().save(dir.join("after-turn.png")).unwrap();
    // Turned, the mark has gone round with the outline.
    let turned = h.state().area_views[&area].ring_places()[0].1;
    assert!((turned - mark).abs() > 1.0, "{mark} then {turned}");
}

#[test]
fn w_a_s_d_drive_the_camera_like_the_arrows() {
    let Some((mut h, area)) = area_harness("wasd") else { return };
    h.run_steps(3);
    let target = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().target;
    let centre = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(centre);
    h.run_steps(1);
    // Held for a few frames, with modifiers.
    let hold = |h: &mut Harness<'_, Moonglow>, key: egui::Key, modifiers: egui::Modifiers| {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        // The modifiers held down all the while, as a hand holds them.
        h.event(egui::Event::ModifiersChanged(modifiers));
        h.event(event(true));
        h.run_steps(4);
        h.event(event(false));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(1);
    };
    let none = egui::Modifiers::NONE;
    let start = target(&h);
    hold(&mut h, egui::Key::ArrowUp, none);
    let by_arrow = target(&h) - start;
    assert!(by_arrow.length() > 0.1, "the arrow moves the view");
    let start = target(&h);
    hold(&mut h, egui::Key::W, none);
    let by_w = target(&h) - start;
    assert!((by_w - by_arrow).length() < 0.05 * by_arrow.length(), "{by_w} like {by_arrow}");
    // Ctrl+W is not a move.
    let start = target(&h);
    hold(&mut h, egui::Key::W, egui::Modifiers::COMMAND);
    assert_eq!(target(&h), start);
    // The pointer gone to the module tree: the keys still drive the view.
    h.hover_at(egui::pos2(60.0, 300.0));
    h.run_steps(2);
    let start = target(&h);
    hold(&mut h, egui::Key::W, none);
    let off_view = target(&h) - start;
    assert!((off_view - by_w).length() < 0.05 * by_w.length(), "{off_view} like {by_w}");
}

#[test]
fn a_right_drag_turns_the_camera_and_w_a_s_d_fly_while_held() {
    let Some((mut h, area)) = area_harness("right-drag") else { return };
    h.run_steps(3);
    let orbit = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap();
    let selection = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].selection.clone();
    let before = orbit(&h);
    let at = screen(&h, area, glam::Vec3::new(20.0, 10.0, 0.0));
    let button = egui::PointerButton::Secondary;
    let modifiers = egui::Modifiers::NONE;
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: at, button, pressed: true, modifiers });
    h.run_steps(1);
    for dx in [20.0, 40.0, 60.0] {
        h.event(egui::Event::PointerMoved(at + egui::vec2(dx, 0.0)));
        h.run_steps(1);
    }
    let turned = orbit(&h);
    assert!((turned.yaw - before.yaw).abs() > 0.3, "turned: {} → {}", before.yaw, turned.yaw);
    // W, with the button still held: the camera moves.
    let key = |pressed| egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    };
    h.event(key(true));
    h.run_steps(4);
    h.event(key(false));
    h.run_steps(1);
    assert!((orbit(&h).target - turned.target).length() > 0.1, "moved while turning");
    let end = at + egui::vec2(60.0, 0.0);
    h.event(egui::Event::PointerButton { pos: end, button, pressed: false, modifiers });
    h.run_steps(2);
    assert!(selection(&h).is_empty(), "nothing selected or turned");
    assert!(h.query_by_label("Properties").is_none(), "no context menu after a drag");
}

/// An object held with the left button stays in hand while the middle
/// button swings the camera, and after that button is let go.
#[test]
fn a_held_object_stays_in_hand_through_a_camera_swing() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("held-swing") else { return };
    let orbit = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap();
    let (x0, y0, _) = waypoint(&mut h, area, 0).unwrap();
    let none = egui::Modifiers::NONE;
    let button = |h: &Harness<'_, Moonglow>, pos, button, pressed| {
        h.event(egui::Event::PointerButton { pos, button, pressed, modifiers: none });
    };
    let (left, middle) = (egui::PointerButton::Primary, egui::PointerButton::Middle);
    let mut at = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    button(&h, at, left, true);
    h.run_steps(1);
    for _ in 0..4 {
        at += egui::vec2(6.0, 0.0);
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
    }
    // The middle button too: a hard swing, to and fro.
    let before = orbit(&h);
    button(&h, at, middle, true);
    h.run_steps(1);
    for step in [egui::vec2(90.0, -40.0), egui::vec2(120.0, -60.0), egui::vec2(-150.0, 70.0)] {
        at += step;
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
    }
    assert!((orbit(&h).yaw - before.yaw).abs() > 0.3, "the camera swung");
    button(&h, at, middle, false);
    h.run_steps(2);
    assert_eq!(waypoint(&mut h, area, 0).map(|w| (w.0, w.1)), Some((x0, y0)), "still held");
    // Still in hand: it goes on following the pointer, and is put down
    // under it when the left button is let go.
    let swung = orbit(&h);
    at += egui::vec2(-30.0, 20.0);
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    assert_eq!(orbit(&h).yaw, swung.yaw, "the camera is let go");
    button(&h, at, left, false);
    h.run_steps(3);
    let (x, y, _) = waypoint(&mut h, area, 0).unwrap();
    let view = &h.state().area_views[&area];
    let model = view.model.as_ref().unwrap();
    let z = model.objects.iter().find(|o| o.index == 0).unwrap().position.z;
    let shown = view.screen_pos(Vec3::new(x, y, z)).expect("in view");
    assert!((shown - at).length() < 3.0, "put down at {shown:?}, the pointer at {at:?}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(waypoint(&mut h, area, 0).map(|w| (w.0, w.1)), Some((x0, y0)), "one move");
}

/// The ring around a selected object: taken anywhere on it (or just off
/// it) and led round, it turns the object with the pointer, as one Rotate.
#[test]
fn the_turning_ring_spins_the_selection() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("turn-ring") else { return };
    let facing = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let w = &git.root.list("WaypointList").unwrap()[0];
        w.float("YOrientation").unwrap().atan2(w.float("XOrientation").unwrap())
    };
    let none = egui::Modifiers::NONE;
    let on = screen(&h, area, Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(on);
    press(&h, on, true, none);
    press(&h, on, false, none);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 0)]);

    // A place on the ring, away from where the waypoint faces.
    let before = facing(&mut h);
    let view = &h.state().area_views[&area];
    let pivot =
        view.model.as_ref().unwrap().objects.iter().find(|o| o.index == 0).unwrap().position;
    let eye = view.orbit.unwrap().camera().eye;
    let radius = ((eye - pivot).length() * 0.0765).max(0.5);
    // (A little outside its line.)
    let ring = |a: f32| pivot + Vec3::new(a.cos(), a.sin(), 0.0) * radius * 1.06;
    let from = before + 2.0;
    let handle = screen(&h, area, ring(from));
    // A click on it keeps the selection.
    h.hover_at(handle);
    press(&h, handle, true, none);
    press(&h, handle, false, none);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 0)]);

    // Led a quarter of the way round: the waypoint faces there.
    let quarter = std::f32::consts::FRAC_PI_2;
    press(&h, handle, true, none);
    let mut at = handle;
    for k in 1..=6 {
        at = screen(&h, area, ring(from + quarter * k as f32 / 6.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-turn-ring").join("turn_ring.png")).unwrap();
    press(&h, at, false, none);
    h.run_steps(3);
    let after = facing(&mut h);
    let turned = (after - before).rem_euclid(std::f32::consts::TAU);
    assert!((turned - quarter).abs() < 0.03, "turned {turned} from {before} to {after}");
    assert_eq!(waypoint(&mut h, area, 0).map(|w| (w.0, w.1)), Some((20.0, 20.0)), "in place");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Rotate"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!((facing(&mut h) - before).abs() < 1e-4, "one undo");

    // Switched off in Options › Area: the same drag turns nothing.
    h.state_mut().settings.no_turn_ring = true;
    h.run_steps(2);
    h.hover_at(handle);
    press(&h, handle, true, none);
    for k in 1..=6 {
        at = screen(&h, area, ring(from + quarter * k as f32 / 6.0));
        h.hover_at(at);
    }
    press(&h, at, false, none);
    h.run_steps(3);
    assert!((facing(&mut h) - before).abs() < 1e-4, "no ring, no turn");
}

/// With Shift held, the rings for tilting a model: one led round turns
/// the visual transform about its axis, as one Tilt. A static placeable
/// has none.
#[test]
fn a_tilt_ring_tilts_a_placeable_s_model_while_shift_is_held() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("tilt-ring") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        // Two chests: the first static, the second (put before it) not.
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut edits = Vec::new();
        for (x, y, fixed) in [(28.0, 30.0, 1), (12.0, 20.0, 0)] {
            let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
            let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
            item.set("Static", mg_gff::Value::Byte(fixed));
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: "Placeable List".into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let rotate = |h: &mut Harness<'_, Moonglow>, index: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let chest = &git.root.list("Placeable List").unwrap()[index];
        mg_area::VisualTransform::read(chest).map(|v| v.rotate)
    };
    let look_at = |h: &mut Harness<'_, Moonglow>, index: usize, target: Vec3| {
        // From the east and above: the ring about X faces the camera.
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(ObjectKind::Placeable, index)];
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.yaw, o.pitch, o.distance) = (target, 0.3, 0.5, 14.0);
    };
    // Leads the ring about X a twelfth of the way round, Shift held.
    let lead = |h: &mut Harness<'_, Moonglow>, pivot: Vec3, shot: bool| {
        let eye = h.state().area_views[&area].orbit.unwrap().camera().eye;
        let radius = ((eye - pivot).length() * 0.0765).max(0.5);
        let ring = |a: f32| pivot + Vec3::new(0.0, a.cos(), a.sin()) * radius;
        let shift = egui::Modifiers::SHIFT;
        h.event(egui::Event::ModifiersChanged(shift));
        let mut at = screen(h, area, ring(1.0));
        h.hover_at(at);
        h.run_steps(1);
        let left = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: shift,
        };
        h.event(left(at, true));
        for k in 1..=6 {
            at = screen(h, area, ring(1.0 + std::f32::consts::FRAC_PI_6 * k as f32 / 6.0));
            h.hover_at(at);
        }
        h.run_steps(1);
        if shot {
            let img = h.render().expect("render");
            img.save(mg_testkit::output_dir("ui-area-tilt-ring").join("tilt_ring.png")).unwrap();
        }
        h.event(left(at, false));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };

    let pivot = Vec3::new(12.0, 20.0, 0.0);
    look_at(&mut h, 0, pivot);
    h.run_steps(2);
    assert_eq!(rotate(&mut h, 0), None, "no visual transform yet");
    lead(&mut h, pivot, true);
    // (About the model's X: Y Rotation, as the game reads it.)
    let tilted = rotate(&mut h, 0).expect("a visual transform");
    assert!((tilted.y - 30.0).abs() < 1.0, "tilted {tilted:?}");
    assert_eq!((tilted.x, tilted.z), (0.0, 0.0), "about X alone");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tilt"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(rotate(&mut h, 0), None, "one undo");

    // The static chest has no rings: the same drag tilts nothing.
    let pivot = Vec3::new(28.0, 30.0, 0.0);
    look_at(&mut h, 1, pivot);
    h.run_steps(2);
    lead(&mut h, pivot, false);
    assert_eq!(rotate(&mut h, 1), None, "a static placeable is not tilted");

    // (Without rings, the drag was the usual one: it moved the chest.)
    if h.state().ws.as_ref().unwrap().can_undo() == Some("Move") {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(2);
    }

    // Its menu makes it dynamic; then it tilts.
    let at = screen(&h, area, pivot + Vec3::Z * 0.3);
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    assert!(h.query_by_label("Make Static").is_none(), "it is static");
    h.get_by_label("Make Dynamic").click();
    h.run_steps(3);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Make Dynamic"));
    look_at(&mut h, 1, pivot);
    h.run_steps(2);
    lead(&mut h, pivot, false);
    let tilted = rotate(&mut h, 1).expect("a visual transform");
    assert!((tilted.y - 30.0).abs() < 1.0, "tilted {tilted:?}");

    // The arrow over it, Shift held: led up a meter, the chest rises one.
    let height = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        git.root.list("Placeable List").unwrap()[1].float("Z").unwrap()
    };
    let z0 = height(&mut h);
    let eye = h.state().area_views[&area].orbit.unwrap().camera().eye;
    let radius = ((eye - pivot).length() * 0.0765).max(0.5);
    let shift = egui::Modifiers::SHIFT;
    let left = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: shift,
    };
    h.event(egui::Event::ModifiersChanged(shift));
    let head = pivot + Vec3::Z * radius * 1.45;
    let mut at = screen(&h, area, head);
    h.hover_at(at);
    h.run_steps(1);
    h.event(left(at, true));
    for k in 1..=5 {
        at = screen(&h, area, head + Vec3::Z * (k as f32 / 5.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    let img = h.render().expect("render");
    img.save(mg_testkit::output_dir("ui-area-tilt-ring").join("height_arrow.png")).unwrap();
    h.event(left(at, false));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let z = height(&mut h);
    assert!((z - z0 - 1.0).abs() < 0.03, "raised from {z0} to {z}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Raise"));

    // The green arrow, north of it: led 2 m along, the chest goes north
    // and nowhere else.
    let place = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let chest = &git.root.list("Placeable List").unwrap()[1];
        (chest.float("X").unwrap(), chest.float("Y").unwrap(), chest.float("Z").unwrap())
    };
    let (x0, y0, z0) = place(&mut h);
    let stands = Vec3::new(x0, y0, z0);
    h.event(egui::Event::ModifiersChanged(shift));
    let head = stands + Vec3::Y * radius * 1.45;
    let mut at = screen(&h, area, head);
    h.hover_at(at);
    h.run_steps(2);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-shift-tools").join("shift_tools.png")).unwrap();
    h.event(left(at, true));
    for k in 1..=5 {
        at = screen(&h, area, head + Vec3::Y * (2.0 * k as f32 / 5.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    let img = h.render().expect("render");
    img.save(mg_testkit::output_dir("ui-area-tilt-ring").join("axis_arrow.png")).unwrap();
    h.event(left(at, false));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let (x, y, z) = place(&mut h);
    assert!((y - y0 - 2.0).abs() < 0.05, "north from {y0} to {y}");
    assert_eq!(x, x0, "not east or west");
    assert!((z - z0).abs() < 1e-3, "as high above the ground as before: {z0} to {z}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move"));
    let pivot = Vec3::new(x, y, 0.0);

    // Dynamic now, its menu makes it static again: the tilt goes.
    let at = screen(&h, area, pivot + Vec3::Z * 1.3);
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    assert!(h.query_by_label("Make Dynamic").is_none(), "it is dynamic");
    h.get_by_label("Make Static").click();
    h.run_steps(3);
    assert_eq!(rotate(&mut h, 1), None, "a static placeable has no visual transform");
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&git_key).unwrap();
    let chest = &git.root.list("Placeable List").unwrap()[1];
    assert_eq!(chest.integer("Static"), Some(1));
}

/// An area opens again with its camera where it was left; Tile
/// Properties (of the tiles it was of), a tile's view and a load order
/// layer's copy of a resource open again with it.
#[test]
fn an_area_opens_again_with_its_camera_and_tile_tools() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("open-tabs-camera") else { return };
    let path = h.state().module_path().expect("saved");
    h.state_mut().open_tabs_dir = Some(mg_testkit::scratch_dir("ui-open-tabs-camera"));
    // Tile Properties of the tile at column 1, row 1.
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(2);
    h.get_by_label("Tile Properties…").click();
    h.run_steps(2);
    assert_eq!(h.state().tile_props.as_ref().map(|p| p.tiles.clone()), Some(vec![5]));
    // The camera turned, tilted and nearer.
    let orbit = {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        let o = view.orbit.as_mut().unwrap();
        o.yaw += 0.7;
        o.pitch = 0.9;
        o.distance = 22.0;
        o.target += Vec3::new(3.0, 2.0, 0.0);
        *o
    };
    let tile = Tab::Tile { tileset: ResRef::from_str("ttr01").unwrap(), tile: 3 };
    let key = ResKey::parse("appearance", ResType::TWODA).unwrap();
    let layer = {
        let rm = &h.state().game.as_ref().unwrap().resman;
        let i = (0..rm.layers().len()).find(|&i| rm.get_in(i, &key).is_ok()).unwrap();
        rm.layers()[i].label.clone()
    };
    let copy = Tab::ResourceCopy { key, layer };
    for tab in [tile.clone(), copy.clone()] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run_steps(2);
    }
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path));
    h.run_steps(8);
    let app = h.state();
    assert!(app.dock.find_tab(&Tab::Area(area)).is_some());
    let now = app.area_views[&area].orbit.expect("drawn");
    assert!((now.target - orbit.target).length() < 0.01, "{now:?} {orbit:?}");
    assert!((now.yaw - orbit.yaw).abs() < 1e-4 && (now.pitch - orbit.pitch).abs() < 1e-4);
    assert!((now.distance - orbit.distance).abs() < 1e-3, "{now:?} {orbit:?}");
    assert!(app.dock.find_tab(&tile).is_some(), "the tile's view");
    assert!(app.dock.find_tab(&copy).is_some(), "the layer's copy");
    assert_eq!(app.tile_props.as_ref().map(|p| p.tiles.clone()), Some(vec![5]));
    assert!(app.dock.find_tab(&Tab::TileProperties).is_some());
}

/// Make Placeables Static: those that lose nothing by it, as one command;
/// one with a variable stays dynamic, and the log says why.
#[test]
fn an_area_s_placeables_are_made_static_together() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("static-placeables") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        // Four chests: plain, Useable, tilted, and with a variable.
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut edits = Vec::new();
        for (index, useable) in [(0, 0), (1, 1), (2, 0), (3, 0)] {
            let at = Placement { position: [10.0 + 5.0 * index as f32, 20.0, 0.0], rotation: 0.0 };
            let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
            for (label, value) in [("Static", 0), ("Useable", useable), ("HasInventory", 0)] {
                item.set(label, mg_gff::Value::Byte(value));
            }
            // (The blueprint's scripts would keep it dynamic.)
            let scripts: Vec<String> = item
                .fields
                .iter()
                .filter(|f| matches!(f.value, mg_gff::Value::ResRef(_)))
                .map(|f| f.label.to_string())
                .filter(|l| l.starts_with("On"))
                .collect();
            for label in scripts {
                item.set(&label, mg_gff::Value::resref(ResRef::EMPTY));
            }
            item.set("Conversation", mg_gff::Value::resref(ResRef::EMPTY));
            item.set("AnimationState", mg_gff::Value::Byte(0));
            item.set("TrapFlag", mg_gff::Value::Byte(0));
            item.set("Tag", mg_gff::Value::String(format!("CHEST_{index}").into_bytes()));
            if index == 3 {
                let mut v = mg_gff::Struct::new(0);
                v.set("Name", mg_gff::Value::String(b"tdod_lore".to_vec()));
                v.set("Type", mg_gff::Value::Dword(3));
                v.set("Value", mg_gff::Value::String(b"stone".to_vec()));
                item.set("VarTable", mg_gff::Value::List(vec![v]));
            } else {
                item.remove("VarTable");
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
    }
    h.run_steps(3);
    {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let s = ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[2].clone();
        let game = h.state().game.as_deref().unwrap();
        let o = mg_area::AreaObject::read(game, mg_area::ObjectKind::Placeable, 2, &s);
        let tilt = mg_area::VisualTransform { rotate: glam::Vec3::X * 20.0, ..Default::default() };
        let edits = mg_area::edit::visual_transform_edits(git_key, &o, &s, tilt);
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tilt", edits)));
    }
    h.run_steps(3);
    let fixed = |h: &mut Harness<'_, Moonglow>| -> Vec<i64> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let list = git.root.list("Placeable List").unwrap();
        list.iter().map(|p| p.integer("Static").unwrap()).collect()
    };
    assert_eq!(fixed(&mut h), [0, 0, 0, 0]);
    h.state_mut().actions.push(mg_ui::Action::StaticPlaceables(vec![area]));
    h.run_steps(3);
    assert_eq!(fixed(&mut h), [1, 0, 0, 0], "the Useable, tilted and noted ones stay dynamic");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Make Placeables Static"));
    let entries = &h.state().log.entries;
    let said = &entries[entries.len() - 2].1;
    assert!(said.starts_with("Made 1 placeable static in 1 area"), "{said}");
    assert!(said.contains("1 Useable, 1 with a visual transform"), "{said}");
    assert!(said.contains("1 with variables"), "{said}");
    let why = &entries.last().unwrap().1;
    assert_eq!(why, &format!("Left dynamic: CHEST_3 in {area}: it has the variable tdod_lore"));
    // A project that names its own variables: this one isn't.
    let p = |g: &[&str]| mg_module::project_file::Patterns::new(g).unwrap();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(fixed(&mut h), [0, 0, 0, 0], "one undo");
    h.state_mut().ws.as_mut().unwrap().module.project_file.make_static.keep_variables = p(&["x_*"]);
    h.state_mut().actions.push(mg_ui::Action::StaticPlaceables(vec![area]));
    h.run_steps(3);
    assert_eq!(fixed(&mut h), [1, 0, 0, 1]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    h.state_mut().ws.as_mut().unwrap().module.project_file = Default::default();

    // And back, every one at once.
    h.state_mut().actions.push(mg_ui::Action::StaticPlaceables(vec![area]));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::DynamicPlaceables(vec![area]));
    h.run_steps(3);
    assert_eq!(fixed(&mut h), [0, 0, 0, 0]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Make Placeables Dynamic"));
    assert_eq!(h.state().log.entries.last().unwrap().1, "Made 1 placeable dynamic in 1 area");
}

/// Adjust Location's Visual Transforms (xtul's mirror: Scale 0.46, X
/// Rotation -50.8): Apply writes them and keeps the window open, OK writes
/// what changed since and closes it, each one step to undo; a pasted copy
/// has them too; saved, the module reopened shows them again in the
/// window, and the area view keeps the model upright (X Rotation turns it
/// about its upright axis, as the game does). A static placeable is
/// offered none, and the window says why and that its file's transform
/// is not drawn.
#[test]
fn adjust_location_s_visual_transforms_are_saved_and_reopened() {
    use glam::Vec3;
    use mg_area::{ObjectKind, VisualTransform};
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("visual-saved") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    let mirror = VisualTransform {
        scale: Vec3::splat(0.46),
        rotate: Vec3::new(-50.8, 0.0, 0.0),
        translate: Vec3::ZERO,
    };
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut edits = Vec::new();
        // (Inserted at 0 each: the dynamic one ends first.)
        for (x, fixed) in [(28.0, 1), (12.0, 0)] {
            let at = Placement { position: [x, 30.0, 0.0], rotation: 1.5643 };
            let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
            item.set("Static", mg_gff::Value::Byte(fixed));
            if fixed == 1 {
                item.set("VisTransformList", mg_area::edit::visual_transform_list(mirror));
            }
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: "Placeable List".into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let visual = |h: &mut Harness<'_, Moonglow>, index: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        VisualTransform::read(
            &ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[index],
        )
    };
    let open = |h: &mut Harness<'_, Moonglow>, at: Vec3| {
        let at = screen(h, area, at + Vec3::Z * 0.3);
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
        h.get_by_label("Adjust Location…").click();
        h.run_steps(3);
    };
    let dynamic = Vec3::new(12.0, 30.0, 0.0);
    open(&mut h, dynamic);
    assert!(h.query_by_label("Visual Transforms").is_some());
    let a = h.state_mut().adjust.as_mut().expect("the window");
    assert_eq!(a.objects, [(ObjectKind::Placeable, 0)]);
    a.visual = mirror;
    a.visual_changed = true;
    h.get_by_label("Apply").click();
    h.run_steps(3);
    assert_eq!(visual(&mut h, 0), Some(mirror), "Apply wrote it");
    assert!(h.state().adjust.is_some(), "Apply keeps the window open");
    // Then the scale alone, and OK.
    let a = h.state_mut().adjust.as_mut().unwrap();
    a.visual.scale = Vec3::splat(0.5);
    a.visual_changed = true;
    h.get_by_label("OK").click();
    h.run_steps(3);
    let half = VisualTransform { scale: Vec3::splat(0.5), ..mirror };
    assert_eq!(visual(&mut h, 0), Some(half), "OK wrote it");
    assert!(h.state().adjust.is_none());
    // Each a step to undo.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(visual(&mut h, 0), Some(mirror));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(visual(&mut h, 0), None);
    for _ in 0..2 {
        h.state_mut().actions.push(mg_ui::Action::Redo);
        h.run_steps(3);
    }
    assert_eq!(visual(&mut h, 0), Some(half));
    // Drawn upright: X Rotation turns the model about its upright axis.
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    let o = model.object(ObjectKind::Placeable, 0).unwrap();
    let up = o.model_transform().transform_vector3(Vec3::Z);
    assert!((up - Vec3::Z * 0.5).length() < 1e-4, "upright, half size: {up}");
    // Copied and pasted: the copy has it too.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    h.hover_at(screen(&h, area, Vec3::new(20.0, 25.0, 0.0)));
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(2);
    let over = screen(&h, area, Vec3::new(20.0, 25.0, 0.0));
    h.hover_at(over);
    press(&h, over, true, egui::Modifiers::NONE);
    press(&h, over, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"));
    assert_eq!(visual(&mut h, 2), Some(half), "the pasted copy's");

    // Saved, and the module opened again: the window shows them.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    let path =
        h.state().ws.as_ref().unwrap().module.location.as_ref().unwrap().path().to_path_buf();
    let m = mg_module::Module::open(&path).unwrap();
    let git = m.gff(&git_key).unwrap().unwrap();
    let saved = &git.root.list("Placeable List").unwrap()[0];
    assert_eq!(VisualTransform::read(saved), Some(half), "in the saved module");
    h.state_mut().open_module(&path);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    h.run_steps(3);
    open(&mut h, dynamic);
    assert_eq!(h.state().adjust.as_ref().expect("the window").visual, half);
    h.get_by_label("Cancel").click();
    h.run_steps(2);

    // The static one: none offered, and why.
    open(&mut h, Vec3::new(28.0, 30.0, 0.0));
    assert!(h.query_by_label("Visual Transforms").is_none());
    let note = mg_ui::area_tools::static_note(h.state().adjust.as_ref().unwrap()).unwrap();
    assert!(note.starts_with("This placeable is static"), "{note}");
    assert!(note.contains("which the game doesn't draw"), "{note}");
    assert!(h.query_by_label(&note).is_some(), "the window says so");
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-adjust-static").join("window.png")).unwrap();
    h.get_by_label("Cancel").click();
    h.run_steps(2);
    assert_eq!(visual(&mut h, 1), Some(mirror), "its file's transform is kept");
}

/// Static ticked in the Properties of two placed placeables edited
/// together takes their visual transforms off, as Aurora does: one step,
/// which Undo takes back, transforms and all. Unticked, none comes back.
#[test]
fn static_ticked_takes_the_visual_transform_off() {
    use mg_area::VisualTransform;
    use mg_module::instances::{Placement, Placing, instance};
    let tilted = VisualTransform {
        scale: glam::Vec3::splat(0.46),
        rotate: glam::Vec3::new(-50.8, 10.0, 0.0),
        translate: glam::Vec3::ZERO,
    };
    let prepare = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let chests: Vec<_> = [12.0, 28.0]
            .map(|x| {
                let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
                let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
                item.set("Static", mg_gff::Value::Byte(0));
                item.set("VisTransformList", mg_area::edit::visual_transform_list(tilted));
                item
            })
            .into();
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        git.root.set("Placeable List", mg_gff::Value::List(chests));
        m.set_gff(git_key, &git).unwrap();
    };
    let Some((mut h, area)) = area_harness_with("static-ticked", "ttr01", None, 4, None, &prepare)
    else {
        return;
    };
    let git_key = ResKey::new(area, ResType::GIT);
    let chests = |h: &mut Harness<'_, Moonglow>| -> Vec<(Option<i64>, Option<VisualTransform>)> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let list = ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap().to_vec();
        list.iter().map(|p| (p.integer("Static"), VisualTransform::read(p))).collect()
    };
    let paths = (0..2).map(|i| mg_edit::GffPath::root().item("Placeable List", i)).collect();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Instances { area, paths }));
    h.run_steps(4);
    let before = chests(&mut h);
    assert_eq!(before, [(Some(0), Some(tilted)); 2]);
    h.get_by_label("Static").click();
    h.run_steps(3);
    assert_eq!(chests(&mut h), [(Some(1), None); 2], "static, and their transforms gone");
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-static-ticked").join("properties.png")).unwrap();
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Static"));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(chests(&mut h), before, "one undo: dynamic and tilted again");
    h.state_mut().actions.push(mg_ui::Action::Redo);
    h.run_steps(3);
    h.get_by_label("Static").click();
    h.run_steps(3);
    assert_eq!(chests(&mut h), [(Some(0), None); 2], "unticked: no transform comes back");
}

/// From straight above, where a tilt ring is seen edge-on and the up arrow
/// end on, both are still led, by the pointer's travel on screen. Adjust
/// Location offers a static placeable no visual transform. The Lighting
/// switch is kept in the settings.
#[test]
fn rings_and_arrows_are_led_from_straight_above() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("from-above") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut edits = Vec::new();
        for (x, y, fixed) in [(28.0, 30.0, 1), (12.0, 30.0, 0)] {
            let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
            let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
            item.set("Static", mg_gff::Value::Byte(fixed));
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: "Placeable List".into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let chest = |h: &mut Harness<'_, Moonglow>, index: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[index].clone()
    };
    let adjust = |h: &mut Harness<'_, Moonglow>, at: Vec3| -> bool {
        let at = screen(h, area, at + Vec3::Z * 0.3);
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
        h.get_by_label("Adjust Location…").click();
        h.run_steps(3);
        let offered = h.query_by_label("Visual Transforms").is_some();
        h.get_by_label("Cancel").click();
        h.run_steps(2);
        offered
    };
    assert!(!adjust(&mut h, Vec3::new(28.0, 30.0, 0.0)), "a static placeable has none");
    let pivot = Vec3::new(12.0, 30.0, 0.0);
    assert!(adjust(&mut h, pivot), "a dynamic one has");
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Placeable, 0)]);

    // Shift + the arrow or ring at `from`, led to `to`.
    let lead = |h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2| {
        let shift = egui::Modifiers::SHIFT;
        let left = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: shift,
        };
        h.event(egui::Event::ModifiersChanged(shift));
        h.hover_at(from);
        h.run_steps(1);
        h.event(left(from, true));
        for k in 1..=5 {
            h.hover_at(from + (to - from) * (k as f32 / 5.0));
        }
        h.run_steps(1);
        h.event(left(to, false));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };
    // The camera looks straight down, as the view opens.
    let eye = h.state().area_views[&area].orbit.unwrap().camera().eye;
    let radius = ((eye - pivot).length() * 0.0765).max(0.5);

    // The up arrow, end on: up the screen raises the chest.
    let head = screen(&h, area, pivot + Vec3::Z * radius * 1.45);
    lead(&mut h, head, head + egui::vec2(0.0, -40.0));
    let z = chest(&mut h, 0).float("Z").unwrap();
    assert!(z > 0.05, "raised to {z}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Raise"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);

    // The ring about X, edge-on (a line north and south over the chest):
    // led along that line, it tilts the model: Y Rotation, which the game
    // tips a model about its X with.
    let ring = |a: f32| pivot + Vec3::new(0.0, a.cos(), a.sin()) * radius;
    let (from, to) = (screen(&h, area, ring(1.05)), screen(&h, area, ring(0.52)));
    lead(&mut h, from, to);
    let tilted = mg_area::VisualTransform::read(&chest(&mut h, 0)).expect("a visual transform");
    assert!((-40.0..-10.0).contains(&tilted.rotate.y), "tilted {:?}", tilted.rotate);
    assert_eq!((tilted.rotate.x, tilted.rotate.z), (0.0, 0.0));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tilt"));

    // Lighting, switched off, stays off for views opened later.
    assert!(!h.state().settings.unlit_areas);
    h.get_by_label("💡 Lighting").click();
    h.run_steps(2);
    assert!(h.state().settings.unlit_areas);
}

/// A selected encounter's spawn points: the tip of a point's arrow, led
/// round, turns the way what spawns there faces.
#[test]
fn a_spawn_point_s_arrow_turns_its_facing() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("spawn-facing") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("nw_giantevil", ResType::UTE).unwrap();
        let blueprint = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let outline = [[-3.0, -3.0, 0.0], [3.0, -3.0, 0.0], [3.0, 3.0, 0.0], [-3.0, 3.0, 0.0]];
        let at = Placement { position: [12.0, 30.0, 0.0], rotation: 0.0 };
        let mut enc = instance(&placing, ResType::UTE, &blueprint, at, &outline).unwrap();
        let mut point = mg_gff::Struct::new(2);
        for (label, v) in [("X", 12.0), ("Y", 30.0), ("Z", 0.0), ("Orientation", 0.0)] {
            point.set(label, mg_gff::Value::Float(v));
        }
        enc.set("SpawnPointList", mg_gff::Value::List(vec![point]));
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Encounter List".into(),
            index: 0,
            item: enc,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    h.run_steps(3);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Encounter, 0)];
    h.run_steps(2);
    let facing = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let enc = &git.root.list("Encounter List").unwrap()[0];
        enc.list("SpawnPointList").unwrap()[0].float("Orientation").unwrap()
    };
    let at = Vec3::new(12.0, 30.0, 0.1);
    let tip = |a: f32| at + Vec3::new(a.cos(), a.sin(), 0.0);
    let none = egui::Modifiers::NONE;
    let from = screen(&h, area, tip(0.0));
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, none);
    let mut to = from;
    for k in 1..=6 {
        to = screen(&h, area, tip(std::f32::consts::FRAC_PI_2 * k as f32 / 6.0));
        h.hover_at(to);
    }
    h.run_steps(1);
    press(&h, to, false, none);
    h.run_steps(3);
    let turned = facing(&mut h);
    assert!((turned - std::f32::consts::FRAC_PI_2).abs() < 0.03, "faces {turned}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Turn Spawn Point"));
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Encounter, 0)]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(facing(&mut h), 0.0, "one undo");
    // The foot of its post, led over the ground, moves the point alone.
    let place = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let enc = &git.root.list("Encounter List").unwrap()[0];
        let p = &enc.list("SpawnPointList").unwrap()[0];
        let outline = enc.float("XPosition").unwrap();
        (p.float("X").unwrap(), p.float("Y").unwrap(), outline)
    };
    let foot = screen(&h, area, Vec3::new(12.0, 30.0, 0.0));
    h.hover_at(foot);
    h.run_steps(1);
    press(&h, foot, true, none);
    let mut to = foot;
    for k in 1..=6 {
        to = screen(&h, area, Vec3::new(12.0 + 0.4 * k as f32, 30.0 - 0.2 * k as f32, 0.0));
        h.hover_at(to);
        h.run_steps(1);
    }
    press(&h, to, false, none);
    h.run_steps(3);
    let (x, y, encounter) = place(&mut h);
    assert!((x - 14.4).abs() < 0.2 && (y - 28.8).abs() < 0.2, "moved to {x}, {y}");
    assert_eq!(encounter, 12.0, "the encounter stays");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move Spawn Point"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(place(&mut h), (12.0, 30.0, 12.0), "one undo");
    // G on an encounter raised off the ground brings it down, its lowest
    // corner onto the ground there.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Raise",
        vec![mg_edit::Edit::SetField {
            key: git_key,
            path: mg_edit::GffPath::root().item("Encounter List", 0),
            label: "ZPosition".into(),
            value: Some(mg_gff::Value::Float(3.0)),
        }],
    )));
    h.run_steps(2);
    let inside = screen(&h, area, Vec3::new(10.5, 31.5, 0.0));
    h.hover_at(inside);
    h.run_steps(1);
    h.key_press(egui::Key::G);
    h.run_steps(2);
    let z = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        git.root.list("Encounter List").unwrap()[0].float("ZPosition").unwrap()
    };
    assert!(z.abs() < 0.5, "on the ground: {z}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Drop to Ground"));
}

/// The area's menu sets the module's start location where it was opened.
#[test]
fn the_start_location_is_set_from_the_area_s_menu() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("set-start") else { return };
    let entry = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let info = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap().root.clone();
        let f = |label: &str| info.float(label).unwrap();
        (info.resref("Mod_Entry_Area").unwrap(), f("Mod_Entry_X"), f("Mod_Entry_Y"))
    };
    let (_, x0, y0) = entry(&mut h);
    let at = screen(&h, area, Vec3::new(12.0, 31.0, 0.0));
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Set Start Location Here").click();
    h.run_steps(3);
    let (in_area, x, y) = entry(&mut h);
    assert_eq!(in_area, area);
    assert!((x - 12.0).abs() < 0.1 && (y - 31.0).abs() < 0.1, "at {x}, {y}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Set start location"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    let (_, x, y) = entry(&mut h);
    assert_eq!((x, y), (x0, y0), "one undo");
}

/// The start location's marker is dragged by its ring and turned by its
/// arrow's tip.
#[test]
fn the_start_location_is_dragged_and_turned_in_the_view() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("drag-start") else { return };
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    {
        let set = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
            key: ifo,
            path: mg_edit::GffPath::root(),
            label: label.into(),
            value: Some(value),
        };
        let edits = vec![
            set("Mod_Entry_Area", mg_gff::Value::resref(area)),
            set("Mod_Entry_X", mg_gff::Value::Float(12.0)),
            set("Mod_Entry_Y", mg_gff::Value::Float(31.0)),
            set("Mod_Entry_Dir_X", mg_gff::Value::Float(1.0)),
            set("Mod_Entry_Dir_Y", mg_gff::Value::Float(0.0)),
        ];
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let entry = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let r = ws.doc(&ifo).unwrap().root.clone();
        let f = |label: &str| r.float(label).unwrap();
        (f("Mod_Entry_X"), f("Mod_Entry_Y"), f("Mod_Entry_Dir_Y").atan2(f("Mod_Entry_Dir_X")))
    };
    let none = egui::Modifiers::NONE;
    let drag = |h: &mut Harness<'_, Moonglow>, path: &[Vec3]| {
        let from = screen(h, area, path[0]);
        h.hover_at(from);
        h.run_steps(1);
        press(h, from, true, none);
        let mut to = from;
        for p in &path[1..] {
            to = screen(h, area, *p);
            h.hover_at(to);
            h.run_steps(1);
        }
        press(h, to, false, none);
        h.run_steps(3);
    };
    // By the ring's west side, four meters east and north.
    let west = Vec3::new(11.0, 31.0, 0.0);
    let steps: Vec<Vec3> = (0..=4).map(|k| west + Vec3::new(k as f32, k as f32, 0.0)).collect();
    drag(&mut h, &steps);
    let (x, y, facing) = entry(&mut h);
    assert!((x - 16.0).abs() < 0.15 && (y - 35.0).abs() < 0.15, "moved to {x}, {y}");
    assert!(facing.abs() < 1e-3, "faces as it did: {facing}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move Start Location"));
    assert!(h.state().area_views[&area].start_selected, "held, it is selected");
    // A click on the ground lets it go; one on its ring selects it.
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let pos = screen(h, area, p);
        h.hover_at(pos);
        h.run_steps(1);
        press(h, pos, true, none);
        press(h, pos, false, none);
        h.run_steps(2);
    };
    click(&mut h, Vec3::new(x + 6.0, y, 0.0));
    assert!(!h.state().area_views[&area].start_selected);
    click(&mut h, Vec3::new(x - 1.0, y, 0.0));
    assert!(h.state().area_views[&area].start_selected, "selected by a click");
    assert_eq!(entry(&mut h).0, x, "a click moves nothing");
    // By the arrow's tip, a quarter turn to the north (it is selected).
    let middle = Vec3::new(x, y, 0.0);
    let tip = |a: f32| middle + Vec3::new(a.cos(), a.sin(), 0.0) * 0.75;
    let round: Vec<Vec3> =
        (0..=6).map(|k| tip(std::f32::consts::FRAC_PI_2 * k as f32 / 6.0)).collect();
    drag(&mut h, &round);
    let (x1, y1, facing) = entry(&mut h);
    assert_eq!((x1, y1), (x, y), "turned where it is");
    assert!((facing - std::f32::consts::FRAC_PI_2).abs() < 0.05, "faces {facing}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Turn Start Location"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(entry(&mut h).2.abs() < 1e-3, "one undo");
    // With the marker hidden, there is nothing to take hold of.
    h.state_mut().area_views.get_mut(&area).unwrap().show_start = false;
    h.run_steps(2);
    let steps: Vec<Vec3> = (0..=4).map(|k| Vec3::new(15.0 + k as f32, 35.0, 0.0)).collect();
    drag(&mut h, &steps);
    assert_eq!(entry(&mut h).0, x, "not moved");
}

/// What the area view's toolbar hides stays hidden in every area view,
/// as in Aurora (a builder's report): in the other open ones, and in one
/// opened later (from the settings, so in the next run too).
#[test]
fn what_the_area_toolbar_hides_is_hidden_in_every_area() {
    use mg_area::ObjectKind;
    use mg_module::new::{AreaSpec, add_area};
    let Some((mut h, first)) = area_harness("shown-together") else { return };
    let second = {
        let app = h.state_mut();
        let game = app.game.clone().unwrap();
        let spec = AreaSpec {
            name: "Second".into(),
            tileset: ResRef::from_str("ttr01").unwrap(),
            width: 4,
            height: 4,
        };
        let ws = app.ws.as_mut().unwrap();
        add_area(&mut ws.module, &game, &spec, &mut fastrand::Rng::with_seed(9)).unwrap()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(second)));
    h.run_steps(3);
    let shown = |h: &Harness<'_, Moonglow>, area: ResRef, kind: ObjectKind| {
        h.state().area_views[&area].show[kind.index()]
    };
    assert!(shown(&h, first, ObjectKind::Trigger) && shown(&h, second, ObjectKind::Trigger));
    h.get_by_label("⚡ Triggers").click();
    h.run_steps(2);
    assert!(!shown(&h, second, ObjectKind::Trigger), "hidden where it was clicked");
    assert!(!shown(&h, first, ObjectKind::Trigger), "and in the other open area");
    assert!(shown(&h, first, ObjectKind::Encounter), "the rest shown");
    assert_eq!(h.state().settings.hidden_in_areas, ["Triggers"]);
    // An area view made anew (closed and opened again) hides them too.
    h.state_mut().area_views.remove(&first);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(first)));
    h.run_steps(3);
    assert!(!shown(&h, first, ObjectKind::Trigger));
    // Show All: everywhere, and nothing kept hidden.
    h.get_by_label("All").click();
    h.run_steps(2);
    assert!(shown(&h, first, ObjectKind::Trigger) && shown(&h, second, ObjectKind::Trigger));
    assert!(h.state().settings.hidden_in_areas.is_empty());
}

/// Delete with a text field holding the keyboard is the field's: the
/// selected objects of the area under the pointer are not deleted too (a
/// code review's finding; the tile selection's Delete had the check).
#[test]
fn delete_typed_in_a_field_deletes_no_object_under_the_pointer() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("delete-typing") else { return };
    let count = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").map_or(0, <[_]>::len)
    };
    assert_eq!(count(&mut h), 2);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    // The module tree's filter takes the keyboard, with text to delete.
    let filter = h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap();
    filter.click();
    h.run_steps(2);
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text("ab");
    h.run_steps(2);
    h.key_press(egui::Key::Home);
    h.run_steps(1);
    // The pointer over the area's view, and Delete.
    let at = screen(&h, area, glam::Vec3::new(25.0, 25.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(count(&mut h), 2, "the waypoint stays");
    // The field let go of, Delete deletes it.
    h.ctx.memory_mut(egui::Memory::stop_text_input);
    h.run_steps(2);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    h.run_steps(1);
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(count(&mut h), 1);
}

/// Every selected object has its turning ring: the ring of any of them,
/// led round, turns them all.
#[test]
fn each_selected_object_has_a_ring_and_any_turns_them_all() {
    use glam::Vec3;
    use mg_area::ObjectKind::Waypoint;
    let Some((mut h, area)) = area_harness("rings-all") else { return };
    let facings = |h: &mut Harness<'_, Moonglow>| -> Vec<f32> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let list = git.root.list("WaypointList").unwrap();
        let facing = |w: &mg_gff::Struct| {
            w.float("YOrientation").unwrap().atan2(w.float("XOrientation").unwrap())
        };
        list.iter().map(facing).collect()
    };
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(Waypoint, 0), (Waypoint, 1)];
    h.run_steps(2);
    let before = facings(&mut h);
    // The ring around the second waypoint (the first selected is the
    // other), taken away from where it faces.
    let view = &h.state().area_views[&area];
    let second = view.model.as_ref().unwrap().objects.iter().find(|o| o.index == 1).unwrap();
    let pivot = second.position;
    let eye = view.orbit.unwrap().camera().eye;
    let radius = ((eye - pivot).length() * 0.0765).max(0.5);
    let ring = |a: f32| pivot + Vec3::new(a.cos(), a.sin(), 0.0) * radius;
    let from = before[1] + 2.0;
    let quarter = std::f32::consts::FRAC_PI_2;
    let none = egui::Modifiers::NONE;
    let mut at = screen(&h, area, ring(from));
    h.hover_at(at);
    press(&h, at, true, none);
    for k in 1..=6 {
        at = screen(&h, area, ring(from + quarter * k as f32 / 6.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    press(&h, at, false, none);
    h.run_steps(3);
    let after = facings(&mut h);
    for (was, now) in before.iter().zip(&after) {
        let turned = (now - was).rem_euclid(std::f32::consts::TAU);
        assert!((turned - quarter).abs() < 0.03, "turned {turned}: {before:?} to {after:?}");
    }
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Rotate"));
}

/// The wheel over a window that lies on the area view is the window's:
/// the area's camera stays where it is.
#[test]
fn the_wheel_over_a_window_leaves_the_area_s_camera_be() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, area)) = area_harness("wheel-window") else { return };
    {
        let app = h.state_mut();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
        app.palette.kind = BlueprintKind::Placeable;
        app.palette.tiles = false;
    }
    h.run_steps(3);
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…").click();
    h.run_steps(3);
    let distance = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().distance;
    let before = distance(&h);
    // (A row of the window, which lies over the area's view.)
    let view = h.state().area_views[&area].rect;
    let row = h
        .query_all_by_label_contains("  (category")
        .map(|n| n.rect().center())
        .find(|p| view.contains(*p))
        .expect("a row over the view");
    h.hover_at(row);
    h.run_steps(3);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -3.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(20);
    assert_eq!(distance(&h), before, "the area isn't zoomed from under the window");
}

#[test]
fn placement_tools_turn_ground_arrange_lock_and_make_prefabs() {
    use glam::Vec3;
    use mg_area::ObjectKind::Waypoint;
    let Some((mut h, area)) = area_harness("placement") else { return };
    // (Options › General › Show duplicate tags warning: off by default.)
    h.state_mut().settings.warn_duplicate_tags = true;
    let prefabs = mg_testkit::scratch_dir("ui-prefabs");
    h.state_mut().prefab_dir = Some(prefabs.clone());
    h.run_steps(3);
    let waypoints = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&ResKey::new(area, ResType::GIT))
            .unwrap()
            .root
            .list("WaypointList")
            .unwrap()
            .to_vec()
    };
    let facing = |s: &mg_gff::Struct| {
        s.float("YOrientation").unwrap().atan2(s.float("XOrientation").unwrap())
    };
    let right_click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    let a = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    let both = vec![(Waypoint, 0), (Waypoint, 1)];
    h.state_mut().area_views.get_mut(&area).unwrap().selection = both.clone();
    h.hover_at(a);
    h.run_steps(1);

    // Q turns the selection 15° to the left, Shift+E 90° to the right.
    let before: Vec<f32> = waypoints(&mut h).iter().map(facing).collect();
    h.key_press(egui::Key::Q);
    h.run_steps(2);
    let after: Vec<f32> = waypoints(&mut h).iter().map(facing).collect();
    for (b, a) in before.iter().zip(&after) {
        assert!((a - b - 15f32.to_radians()).abs() < 1e-4, "{b} → {a}");
    }
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(2);
    let turned: Vec<f32> = waypoints(&mut h).iter().map(facing).collect();
    assert!((after[0] - turned[0] - 90f32.to_radians()).abs() < 1e-4);

    // Together: the two turn about their middle as one, each keeping
    // where the other is, seen from the way it faces.
    let seen = |w: &[mg_gff::Struct]| {
        let at = |s: &mg_gff::Struct| {
            glam::Vec2::new(s.float("XPosition").unwrap(), s.float("YPosition").unwrap())
        };
        let to = at(&w[1]) - at(&w[0]);
        (to.y.atan2(to.x) - facing(&w[0])).rem_euclid(std::f32::consts::TAU)
    };
    let was = waypoints(&mut h);
    h.state_mut().settings.turn_together = true;
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(2);
    let now = waypoints(&mut h);
    assert!((facing(&was[0]) - facing(&now[0]) - 90f32.to_radians()).abs() < 1e-4, "turned");
    let moved = (was[0].float("XPosition").unwrap() - now[0].float("XPosition").unwrap()).abs()
        + (was[0].float("YPosition").unwrap() - now[0].float("YPosition").unwrap()).abs();
    assert!(moved > 0.1, "led round the middle");
    assert!((seen(&was) - seen(&now)).abs() < 1e-3, "{} → {}", seen(&was), seen(&now));
    h.state_mut().settings.turn_together = false;
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);

    // A trigger turns by its outline: alone about the outline's middle,
    // and Together with the rest about theirs.
    let git = ResKey::new(area, ResType::GIT);
    let mut trigger = mg_gff::Struct::new(1);
    for (label, v) in [("XPosition", 22.0), ("YPosition", 20.0), ("ZPosition", 0.0)] {
        trigger.set(label, mg_gff::Value::Float(v));
    }
    let corner = |x: f32, y: f32| {
        let mut p = mg_gff::Struct::new(3);
        p.set("PointX", mg_gff::Value::Float(x));
        p.set("PointY", mg_gff::Value::Float(y));
        p.set("PointZ", mg_gff::Value::Float(0.0));
        p
    };
    let corners = vec![corner(0.0, 0.0), corner(4.0, 0.0), corner(4.0, 2.0), corner(0.0, 2.0)];
    trigger.set("Geometry", mg_gff::Value::List(corners));
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
    h.run_steps(3);
    // The outline's corners in the area, from the GIT.
    let outline = |h: &mut Harness<'_, Moonglow>| -> Vec<glam::Vec2> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let t = ws.doc(&git).unwrap().root.list("TriggerList").unwrap()[0].clone();
        let at = glam::Vec2::new(t.float("XPosition").unwrap(), t.float("YPosition").unwrap());
        t.list("Geometry")
            .unwrap()
            .iter()
            .map(|p| at + glam::Vec2::new(p.float("PointX").unwrap(), p.float("PointY").unwrap()))
            .collect()
    };
    let near = |a: glam::Vec2, b: glam::Vec2| a.distance(b) < 1e-3;
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Trigger, 0)];
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(2);
    // (22..26 by 20..22, a quarter turn to the right about 24, 21.)
    let now = outline(&mut h);
    assert!(near(now[0], glam::Vec2::new(23.0, 23.0)), "{now:?}");
    assert!(near(now[2], glam::Vec2::new(25.0, 19.0)), "{now:?}");
    let alone = now;
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Trigger, 0), (Waypoint, 0)];
    h.state_mut().settings.turn_together = true;
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(2);
    let together = outline(&mut h);
    let side = |o: &[glam::Vec2]| o[1] - o[0];
    assert!(near(side(&together), -side(&alone).perp()), "{alone:?} → {together:?}");
    h.state_mut().settings.turn_together = false;
    for _ in 0..3 {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(2);
    }
    h.state_mut().area_views.get_mut(&area).unwrap().selection = both.clone();
    h.run_steps(2);

    // G drops a raised object to the ground.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Raise",
        vec![mg_edit::Edit::SetField {
            key: git,
            path: mg_edit::GffPath::root().item("WaypointList", 0),
            label: "ZPosition".into(),
            value: Some(mg_gff::Value::Float(3.0)),
        }],
    )));
    h.run_steps(2);
    h.key_press(egui::Key::G);
    h.run_steps(2);
    let ground = waypoints(&mut h)[0].float("ZPosition").unwrap();
    assert!(ground.abs() < 1.0, "on the ground: {ground}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Drop to Ground"));

    // Arrange › Mirror West–East swaps them.
    right_click(&mut h, a);
    h.get_by_label_contains("Arrange").click();
    h.run_steps(2);
    h.get_by_label("Mirror West–East").click();
    h.run_steps(3);
    let xs: Vec<f32> = waypoints(&mut h).iter().map(|s| s.float("XPosition").unwrap()).collect();
    assert!((xs[0] - 30.0).abs() < 1e-3 && (xs[1] - 20.0).abs() < 1e-3, "{xs:?}");

    // Save as Prefab, then place it again elsewhere.
    let b = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    right_click(&mut h, b);
    h.get_by_label("Save as Prefab…").click();
    h.run_steps(3);
    // The name field has the focus; Enter saves.
    h.event(egui::Event::Text("Two Posts".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(2);
    assert!(prefabs.join("Two Posts.prefab.json").is_file(), "{:?}", h.state().log.entries);
    h.state_mut().actions.push(mg_ui::Action::PlacePrefab("Two Posts".into()));
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting);
    let c = screen(&h, area, Vec3::new(25.0, 32.0, 0.02));
    h.hover_at(c);
    h.run_steps(2);
    press(&h, c, true, egui::Modifiers::NONE);
    press(&h, c, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(waypoints(&mut h).len(), 4, "{:?}", h.state().log.entries);
    // (The posts' tags are their blueprint's: nothing asked.)
    assert!(h.state().tag_clash.is_none());

    // Lock: clicks pass locked objects by; Unlock All frees them.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = both;
    right_click(&mut h, b);
    h.get_by_label("Lock").click();
    h.run_steps(3);
    let locked = waypoints(&mut h);
    assert!(locked[..2].iter().all(|s| s.byte(mg_area::LOCKED) == Some(1)));
    press(&h, b, true, egui::Modifiers::NONE);
    press(&h, b, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert!(h.state().area_views[&area].selection.is_empty(), "a locked object isn't picked");
    let empty = screen(&h, area, Vec3::new(10.0, 35.0, 0.0));
    right_click(&mut h, empty);
    h.get_by_label("Unlock All (2)").click();
    h.run_steps(3);
    assert!(waypoints(&mut h).iter().all(|s| s.get(mg_area::LOCKED).is_none()));

    // With a 1 m grid, a dragged object lands on whole meters.
    h.state_mut().settings.snap_grid = Some(100);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(Waypoint, 1)];
    let from = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    let to = from + egui::vec2(37.0, 23.0);
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=4 {
        h.hover_at(from + (to - from) * (k as f32 / 4.0));
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let moved = &waypoints(&mut h)[1];
    let (x, y) = (moved.float("XPosition").unwrap(), moved.float("YPosition").unwrap());
    assert!((x, y) != (20.0, 20.0), "it moved");
    assert!((x - x.round()).abs() < 1e-4 && (y - y.round()).abs() < 1e-4, "on the grid: {x}, {y}");
}

/// A drag begun in a Properties window lying over the area's view moves
/// nothing in the area.
#[test]
fn a_drag_in_a_window_over_the_area_moves_nothing_there() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("drag-over") else { return };
    h.run_steps(40);
    let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
    }
    h.run_steps(5);
    let before = (waypoint(&mut h, area, 0), waypoint(&mut h, area, 1));
    // Where the window covers a waypoint: a drag from there.
    let covered = h
        .state()
        .dock
        .iter_leaves()
        .find(|(p, _)| !p.surface.is_main())
        .map(|(_, leaf)| leaf.rect)
        .expect("the Properties window");
    let feet = [Vec3::new(20.0, 20.0, 0.02), Vec3::new(30.0, 20.0, 0.02)];
    let from = feet.map(|p| screen(&h, area, p)).into_iter().find(|p| covered.contains(*p));
    let from = from.unwrap_or_else(|| panic!("no waypoint under {covered:?}"));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=6 {
        h.hover_at(from + egui::vec2(8.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    press(&h, from + egui::vec2(48.0, 0.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!((waypoint(&mut h, area, 0), waypoint(&mut h, area, 1)), before);
}

/// The game's invisible object is selected by the box its model holds
/// undrawn (0.6 m across, 0.18 m tall), as Aurora outlines it: it was a
/// marker's, a metre each way.
#[test]
fn an_invisible_object_is_selected_by_its_own_box() {
    let Some((mut h, area)) = area_harness("invisible-object") else { return };
    h.run_steps(3);
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let mut utp = {
        let game = h.state().game.as_deref().unwrap();
        Gff::read(&game.resman.get(&chest).unwrap()).unwrap().root
    };
    utp.set("Appearance", mg_gff::Value::Dword(157));
    for (label, v) in [("X", 15.0), ("Y", 15.0), ("Z", 0.0), ("Bearing", 0.0)] {
        utp.set(label, mg_gff::Value::Float(v));
    }
    let edit = mg_edit::Edit::InsertItem {
        key: ResKey::new(area, ResType::GIT),
        path: mg_edit::GffPath::root(),
        list: "Placeable List".into(),
        index: 0,
        item: utp,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Place", vec![edit])));
    h.run_steps(5);
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    let placeable = |o: &mg_area::AreaObject| o.kind == mg_area::ObjectKind::Placeable;
    let i = view.model.as_ref().unwrap().objects.iter().position(placeable).unwrap();
    view.selection = vec![(mg_area::ObjectKind::Placeable, 0)];
    let (min, max) = view.box_of(i).expect("a box");
    let across = max - min;
    assert!((across.x - 0.6).abs() < 0.05 && (across.y - 0.6).abs() < 0.05, "{across:?}");
    assert!(across.z < 0.3, "{across:?}");
    h.run_steps(3);
    h.render().unwrap().save(mg_testkit::scratch_dir("ui-invisible-object").join("a.png")).unwrap();
}

/// A right click on the ground offers a light: one of the game's
/// invisible light placeables is put there, a little above the ground,
/// and its menu gives it another color.
#[test]
fn a_light_is_added_from_the_area_s_menu_and_recolored() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("add-light") else { return };
    h.set_size(egui::vec2(1400.0, 1000.0));
    h.run_steps(3);
    let git_key = ResKey::new(area, ResType::GIT);
    let placed = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        git.root.list("Placeable List").map(<[_]>::to_vec).unwrap_or_default()
    };
    let right_click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    right_click(&mut h, at);
    h.get_by_label_contains("Add Light Here").hover();
    h.run_steps(3);
    h.get_by_label("White").click();
    h.run_steps(3);
    let lights = placed(&mut h);
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].integer("Appearance"), Some(15112), "Light, White");
    let (x, z) = (lights[0].float("X").unwrap(), lights[0].float("Z").unwrap());
    assert!((x - 15.0).abs() < 0.5 && (z - 1.5).abs() < 0.3, "at {x}, up {z}");
    assert_eq!(lights[0].integer("Static"), Some(1));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add Light, White"));
    assert_eq!(h.state().area_views[&area].selection, [(mg_area::ObjectKind::Placeable, 0)]);
    h.render().unwrap().save(mg_testkit::scratch_dir("ui-add-light").join("a.png")).unwrap();
    // Its own menu: another color.
    let on_it = screen(&h, area, Vec3::new(x, lights[0].float("Y").unwrap(), z + 0.5));
    right_click(&mut h, on_it);
    h.get_by_label_contains("Light Color").hover();
    h.run_steps(3);
    h.get_by_label("Red").click();
    h.run_steps(3);
    assert_eq!(placed(&mut h)[0].integer("Appearance"), Some(15111), "Light, Red");
}

/// With Shift held, a selected placeable has a handle that scales its
/// model about its feet: pulled twice as far from the object, twice the
/// size, in one undoable step; Escape drops the drag.
#[test]
fn a_scale_handle_scales_a_placeable_s_model_while_shift_is_held() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("scale-handle") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    let pivot = Vec3::new(12.0, 20.0, 0.0);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [pivot.x, pivot.y, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    h.run_steps(3);
    let scale = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let chest = &git.root.list("Placeable List").unwrap()[0];
        mg_area::VisualTransform::read(chest).map(|v| v.scale)
    };
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(ObjectKind::Placeable, 0)];
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.yaw, o.pitch, o.distance) = (pivot, 0.3, 0.5, 14.0);
    }
    h.run_steps(2);
    // The handle: up and to the camera's right of the chest's feet.
    let camera = h.state().area_views[&area].orbit.unwrap().camera();
    let radius = ((camera.eye - pivot).length() * 0.0765).max(0.5);
    let right = (camera.target - camera.eye).cross(Vec3::Z).normalize();
    let handle = screen(&h, area, pivot + (right + Vec3::Z).normalize() * radius * 1.25);
    let middle = screen(&h, area, pivot);
    let shift = egui::Modifiers::SHIFT;
    let left = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: shift,
    };
    // Pulled to `times` as far from the chest; let go, or dropped.
    let pull = |h: &mut Harness<'_, Moonglow>, times: f32, escape: bool, shot: bool| {
        h.event(egui::Event::ModifiersChanged(shift));
        h.hover_at(handle);
        h.run_steps(1);
        h.event(left(handle, true));
        let to = middle + (handle - middle) * times;
        for k in 1..=6 {
            h.hover_at(handle + (to - handle) * (k as f32 / 6.0));
            h.run_steps(1);
        }
        if shot {
            let img = h.render().expect("render");
            img.save(mg_testkit::scratch_dir("ui-area-scale").join("scale_handle.png")).unwrap();
        }
        if escape {
            h.key_press(egui::Key::Escape);
            h.run_steps(1);
        }
        h.event(left(to, false));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };
    assert_eq!(scale(&mut h), None, "no visual transform yet");
    pull(&mut h, 2.0, true, false);
    assert_eq!(scale(&mut h), None, "Escape dropped it");
    pull(&mut h, 2.0, false, true);
    let twice = scale(&mut h).expect("a visual transform");
    assert!((twice.x - 2.0).abs() < 0.05, "scaled {twice:?}");
    assert_eq!((twice.y, twice.z), (twice.x, twice.x), "the same every way");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Scale"));
    // Pushed halfway in: half of that, from what it has.
    pull(&mut h, 0.5, false, false);
    let back = scale(&mut h).unwrap();
    assert!((back.x - 1.0).abs() < 0.05, "scaled back {back:?}");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(scale(&mut h), None, "an undo each");
}

/// A look (`MG_SHOT=1`): a dragon selected in an area, its box around it
/// as it stands.
#[test]
fn a_dragon_s_box_in_an_area() {
    use mg_module::instances::{Placement, Placing, instance};
    if std::env::var_os("MG_SHOT").is_none() {
        return;
    }
    let Some((mut h, area)) = area_harness("dragon-box") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("nw_drgred001", ResType::UTC).unwrap();
        let dragon = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [20.0, 20.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTC, &dragon, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Creature List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    h.run_steps(3);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(mg_area::ObjectKind::Creature, 0)];
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.yaw, o.pitch, o.distance) =
            (glam::Vec3::new(20.0, 20.0, 2.0), 0.6, 0.45, 28.0);
    }
    h.run_steps(6);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-dragon-box").join("dragon.png")).unwrap();
}

/// Ctrl + wheel over the view scales the selected placeable's model, as
/// Aurora's does: shown as the wheel turns, one command when Ctrl is let
/// go, and Undo takes it back whole (or drops it while it is under way).
/// A static placeable takes no scale, and with nothing selected that
/// does the wheel zooms, as it did; so does it with the pointer off the
/// selection (a builder's report: the view moved with Ctrl + drag, the
/// wheel turned to zoom scaled the placeable selected).
#[test]
fn ctrl_and_the_wheel_scale_what_is_selected() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("scale-wheel") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    let pivot = Vec3::new(12.0, 20.0, 0.0);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [pivot.x, pivot.y, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    h.run_steps(3);
    let scale = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let chest = &git.root.list("Placeable List").unwrap()[0];
        mg_area::VisualTransform::read(chest).map(|v| v.scale.x)
    };
    let distance = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().distance;
    let ctrl = egui::Modifiers::COMMAND;
    // The wheel turned with Ctrl held (and still held after).
    let turn = |h: &mut Harness<'_, Moonglow>, by: f32| {
        h.event(egui::Event::ModifiersChanged(ctrl));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, by),
            phase: egui::TouchPhase::Move,
            modifiers: ctrl,
        });
        h.run_steps(12);
    };
    let let_go = |h: &mut Harness<'_, Moonglow>| {
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };
    let over = screen(&h, area, pivot);
    h.hover_at(over);
    h.run_steps(2);
    // Nothing selected: the wheel zooms, slowly.
    let far = distance(&h);
    turn(&mut h, 200.0);
    let_go(&mut h);
    assert!(distance(&h) < far, "zoomed in");
    assert_eq!(scale(&mut h), None);
    // The chest selected, the pointer off it: the wheel zooms still.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    h.hover_at(screen(&h, area, pivot) + egui::vec2(160.0, 0.0));
    h.run_steps(2);
    let far = distance(&h);
    turn(&mut h, 200.0);
    let_go(&mut h);
    assert!(distance(&h) < far, "zoomed, the pointer off the chest");
    assert_eq!(scale(&mut h), None, "the chest keeps its size");
    // The pointer on it: it grows as the wheel turns, shown before it is
    // a command, and the view stays where it is.
    h.hover_at(screen(&h, area, pivot));
    h.run_steps(2);
    let far = distance(&h);
    turn(&mut h, 200.0);
    assert_eq!(scale(&mut h), None, "not a command while Ctrl is held");
    assert!(h.query_by_label_contains("Scale 1.").is_some(), "the status line says how large");
    turn(&mut h, 200.0);
    let_go(&mut h);
    let grown = scale(&mut h).expect("a visual transform");
    assert!(grown > 1.2 && grown < 2.0, "scaled {grown}");
    assert_eq!(distance(&h), far, "the wheel was the chest's, not the camera's");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Scale"));
    // One undo for the wheel's whole turn.
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(scale(&mut h), None, "one undo");
    // Turned the other way: smaller. Left to rest, it is a command too.
    turn(&mut h, -200.0);
    h.run_steps(40);
    let shrunk = scale(&mut h).expect("made a command when the wheel rests");
    assert!(shrunk < 0.9, "scaled {shrunk}");
    let_go(&mut h);
    // Saved while a turn is under way: the turn is part of what is saved.
    let_go(&mut h);
    turn(&mut h, 200.0);
    // (The wheel's turning is smoothed over some frames: all of it in.)
    h.run_steps(8);
    assert!(h.state().area_views[&area].wheel_scaling(), "still under way");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert!(!h.state().area_views[&area].wheel_scaling(), "made a command by the save");
    let saved = scale(&mut h).expect("a command before the save");
    assert!(saved > shrunk, "scaled {saved}");
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "and saved");
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run_steps(3);
    let_go(&mut h);
    assert_eq!(scale(&mut h), Some(shrunk));
    // Undo while a turn is under way drops that, and no more.
    turn(&mut h, 200.0);
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run_steps(3);
    let_go(&mut h);
    assert_eq!(scale(&mut h), Some(shrunk), "the turn under way was dropped");
    // A static placeable takes no scale: the wheel zooms, and the status
    // line says why.
    let set = mg_edit::Edit::SetField {
        key: git_key,
        path: mg_edit::GffPath::root().item("Placeable List", 0),
        label: "Static".into(),
        value: Some(mg_gff::Value::Byte(1)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Static", vec![set])));
    h.run_steps(3);
    let far = distance(&h);
    turn(&mut h, 200.0);
    let_go(&mut h);
    assert!(distance(&h) < far, "zoomed");
    assert!(h.query_by_label_contains("static placeable takes no scale").is_some());
}

/// Replace Selected with This keeps what Update Instances keeps (Aurora's
/// remaking of a placed object): a trigger is replaced with its outline
/// where it was, a tilted placeable keeps its place, facing and visual
/// transform; the tag and the variables are the new blueprint's. Each
/// replacement is one step to undo.
#[test]
fn replaced_objects_keep_their_outline_and_transform() {
    use mg_area::{ObjectKind, VisualTransform};
    use mg_module::instances::{Placement, Placing, instance};
    let tilted = VisualTransform {
        scale: glam::Vec3::splat(0.7),
        rotate: glam::Vec3::new(30.0, 15.0, 0.0),
        translate: glam::Vec3::ZERO,
    };
    let outline = [[0.0, 0.0, 0.025], [6.0, 0.0, 0.025], [6.0, 4.0, 0.025], [0.0, 4.0, 0.025]];
    let prepare = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap()
        };
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut var = mg_gff::Struct::new(0);
        var.set("Name", mg_gff::Value::String(b"nOld".to_vec()));
        var.set("Type", mg_gff::Value::Dword(1));
        var.set("Value", mg_gff::Value::Int(1));
        let old = |mut s: mg_gff::Struct| {
            s.set("Tag", mg_gff::Value::String(b"OLD_ONE".to_vec()));
            s.set("VarTable", mg_gff::Value::List(vec![var.clone()]));
            s
        };
        let at = Placement { position: [10.0, 26.0, 0.0], rotation: 0.0 };
        let trigger = read("newgeneric", ResType::UTT).root;
        let trigger = old(instance(&placing, ResType::UTT, &trigger, at, &outline).unwrap());
        let at = Placement { position: [28.0, 30.0, 0.0], rotation: 1.2 };
        let chest = read("plc_chest1", ResType::UTP).root;
        let mut chest = old(instance(&placing, ResType::UTP, &chest, at, &[]).unwrap());
        chest.set("Static", mg_gff::Value::Byte(0));
        chest.set("VisTransformList", mg_area::edit::visual_transform_list(tilted));
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        git.root.set("TriggerList", mg_gff::Value::List(vec![trigger]));
        git.root.set("Placeable List", mg_gff::Value::List(vec![chest]));
        m.set_gff(git_key, &git).unwrap();
        // The blueprints to replace them with, in the custom palette.
        for (resref, from, t, name, tag) in [
            ("mg_zone", "newgeneric", ResType::UTT, "Mg Zone", "MG_ZONE"),
            ("mg_crate", "plc_chest1", ResType::UTP, "Mg Crate", "MG_CRATE"),
        ] {
            let key = ResKey::parse(resref, t).unwrap();
            let mut g = read(from, t);
            g.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
            let label = if t == ResType::UTT { "LocalizedName" } else { "LocName" };
            let ls = LocString::from_text(Language::ENGLISH, Gender::Male, name);
            g.root.set(label, mg_gff::Value::LocString(ls));
            g.root.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
            g.root.set("PaletteID", mg_gff::Value::Byte(0));
            if t == ResType::UTP {
                g.root.set("Static", mg_gff::Value::Byte(0));
            }
            m.set_gff(key, &g).unwrap();
        }
        mg_module::palette::rebuild_custom_palettes(m, game).unwrap();
    };
    let Some((mut h, area)) = area_harness_with("replace-kept", "ttr01", None, 4, None, &prepare)
    else {
        return;
    };
    let git_key = ResKey::new(area, ResType::GIT);
    let placed = |h: &mut Harness<'_, Moonglow>, list: &str| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list(list).unwrap()[0].clone()
    };
    let (trigger, chest) = (placed(&mut h, "TriggerList"), placed(&mut h, "Placeable List"));
    let before = h.render().expect("render");
    before.save(mg_testkit::output_dir("ui-replace-kept").join("before.png")).unwrap();
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Trigger, 0), (ObjectKind::Placeable, 0)];
    let replace = |h: &mut Harness<'_, Moonglow>, kind, name: &str| {
        let app = h.state_mut();
        app.palette.kind = kind;
        (app.palette.tiles, app.palette.custom) = (false, true);
        app.palette.filter = name.into();
        h.run_steps(3);
        h.get_by_label(name).click_secondary();
        h.run_steps(2);
        h.get_by_label("Replace Selected with This").click();
        h.run_steps(3);
    };
    use mg_module::palette::BlueprintKind;
    replace(&mut h, BlueprintKind::Trigger, "Mg Zone");
    let zone = placed(&mut h, "TriggerList");
    assert_eq!(zone.resref("TemplateResRef").unwrap().to_string(), "mg_zone");
    assert_eq!(zone.string("Tag"), Some(&b"MG_ZONE"[..]), "the blueprint's tag");
    assert!(zone.list("VarTable").unwrap_or(&[]).is_empty(), "the old one's variables go");
    assert_eq!(zone.list("Geometry"), trigger.list("Geometry"), "its outline");
    for f in ["XPosition", "YPosition", "ZPosition"] {
        assert_eq!(zone.float(f), trigger.float(f), "{f}");
    }
    assert_eq!(placed(&mut h, "Placeable List"), chest, "the chest is another type's");
    replace(&mut h, BlueprintKind::Placeable, "Mg Crate");
    let crate_ = placed(&mut h, "Placeable List");
    assert_eq!(crate_.string("Tag"), Some(&b"MG_CRATE"[..]));
    for f in ["X", "Y", "Z", "Bearing"] {
        assert_eq!(crate_.float(f), chest.float(f), "{f}");
    }
    assert_eq!(VisualTransform::read(&crate_), Some(tilted), "its visual transform");
    let after = h.render().expect("render");
    after.save(mg_testkit::output_dir("ui-replace-kept").join("after.png")).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(placed(&mut h, "Placeable List"), chest, "one undo for the chest");
    assert_eq!(placed(&mut h, "TriggerList"), zone);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(placed(&mut h, "TriggerList"), trigger, "and one for the trigger");
}

/// Drop onto Object (Shift+G) sets the selection on the placeable below
/// it, by the placeable's model (a builder's ask: a skeleton on an altar);
/// with nothing below, on the ground.
#[test]
fn shift_g_drops_the_selection_onto_the_placeable_below() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("drop-onto") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let chest = {
        let game = h.state().game.clone().unwrap();
        let bp = game.resman.get(&ResKey::parse("plc_chest1", ResType::UTP).unwrap()).unwrap();
        let bp = Gff::read(&bp).unwrap();
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let at = Placement { position: [25.0, 25.0, 0.0], rotation: 0.0 };
        instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap()
    };
    let set = |label: &str, value: f32| mg_edit::Edit::SetField {
        key: git,
        path: mg_edit::GffPath::root().item("WaypointList", 0),
        label: label.into(),
        value: Some(mg_gff::Value::Float(value)),
    };
    let edits = vec![
        mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item: chest,
        },
        set("XPosition", 25.0),
        set("YPosition", 25.0),
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.run_steps(6);
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    let at = screen(&h, area, glam::Vec3::new(25.0, 25.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    let z = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git).unwrap().root.list("WaypointList").unwrap()[0].float("ZPosition").unwrap()
    };
    let ground = z(&mut h);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::G);
    h.run_steps(3);
    let top = z(&mut h);
    assert!(top > ground + 0.3 && top < ground + 2.0, "on the chest: {ground} → {top}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Drop onto Object"));
    // Beside it, on the ground.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Away",
        vec![set("XPosition", 15.0), set("ZPosition", 1.5)],
    )));
    h.run_steps(4);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::G);
    h.run_steps(3);
    assert!((z(&mut h) - ground).abs() < 0.3, "on the ground: {}", z(&mut h));
    // The chest's row in the module tree has Preview: the model viewer, as
    // it is placed. (The area is opened out there: its waypoint was
    // selected.)
    h.get_by_label("Placeables (1)").click();
    h.run_steps(6);
    let row = h.get_by_label_contains("Chest").rect().center();
    h.hover_at(row);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Preview").click();
    h.run_steps(4);
    let path = mg_edit::GffPath::root().item("Placeable List", 0);
    assert!(h.state().dock.find_tab(&Tab::InstanceModel { area, path }).is_some());
}

/// Properties is the last item of every right-click menu that has one,
/// with Variables just before it where the menu has that, as Aurora's
/// menus end (a builder's ask: Jasperre's): a placed object's, a tile's,
/// a placed object's row in the module tree and an area's row there.
#[test]
fn properties_is_the_last_item_of_the_area_and_tree_menus() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("properties-last") else { return };
    let right_click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    let tail = |items: &[String], n: usize| items[items.len().saturating_sub(n)..].to_vec();
    // A waypoint in the area view.
    let on_it = screen(&h, area, Vec3::new(20.0, 20.0, 0.0));
    right_click(&mut h, on_it);
    let items = menu_items(&h, "Properties");
    assert_eq!(tail(&items, 2), ["Variables…", "Properties"], "{items:?}");
    assert!(items.contains(&"Delete".to_string()), "{items:?}");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    // Its row in the module tree.
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    h.run_steps(6);
    let row = h.query_all_by_label_contains("WAYPOINT").next().expect("a waypoint's row").rect();
    right_click(&mut h, row.center());
    let items = menu_items(&h, "Properties");
    // (A waypoint has no Preview.)
    assert_eq!(items, ["Go To", "Copy", "Delete", "Variables…", "Properties"]);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    // The area's row in the module tree (left of the area's tab).
    let name = area.to_string();
    let row =
        h.get_all_by_label(&name).map(|n| n.rect()).min_by(|a, b| a.left().total_cmp(&b.left()));
    right_click(&mut h, row.expect("the area's row").center());
    let items = menu_items(&h, "Properties");
    assert_eq!(items.last().map(String::as_str), Some("Properties"), "{items:?}");
    assert!(items.contains(&"View Area".to_string()), "the area's menu: {items:?}");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    // A tile, in tile mode.
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    let tile = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    right_click(&mut h, tile);
    let items = menu_items(&h, "Tile Properties…");
    assert_eq!(
        items,
        [
            "Next Tile Appearance",
            "Previous Tile Appearance",
            "Rotate Tile",
            "Delete",
            "Tile Properties…"
        ]
    );
}
