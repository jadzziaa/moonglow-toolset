//! The area view's handles as the redesign draws them ("Fine Lines", the
//! UI redesign proposal's section 11): the rings as projected circles,
//! the rest of a set at half strength while one handle is lit, the grab
//! cursor over every handle and the name chip beside it. (What each
//! handle does, its reach and who gets which: `area_view.rs`' tests.)

use crate::common::*;
use glam::Vec3;
use mg_area::ObjectKind;

/// Where the chest stands.
const CHEST: Vec3 = Vec3::new(12.0, 20.0, 0.0);

/// [`area_harness`]'s area with a dynamic chest at [`CHEST`], selected,
/// seen as the Gizmos boards see theirs (yaw 0.75, pitch 0.72, 10 m).
fn chest_harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let chest = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::instances::{Placement, Placing, instance};
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: CHEST.to_array(), rotation: 0.0 };
        let mut item = instance(&placing, ResType::UTP, &bp, at, &[]).unwrap();
        item.set("Static", mg_gff::Value::Byte(0));
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        git.root.set("Placeable List", mg_gff::Value::List(vec![item]));
        m.set_gff(git_key, &git).unwrap();
    };
    let (mut h, area) = area_harness_with(name, "ttr01", None, 4, None, &chest)?;
    h.run_steps(3);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(ObjectKind::Placeable, 0)];
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.yaw, o.pitch, o.distance) = (CHEST + Vec3::Z * 0.5, 0.75, 0.72, 10.0);
    }
    h.run_steps(3);
    Some((h, area))
}

fn tokens() -> mg_uikit::Tokens {
    mg_uikit::Tokens::DARK
}

/// The shapes drawn over the area's view (not the Status Bar's glyphs,
/// in the same colors).
fn over_view(h: &Harness<'_, Moonglow>) -> Vec<egui::Shape> {
    let rect = h.state().area_views.values().next().unwrap().rect;
    drawn(h).into_iter().filter(|s| rect.contains_rect(s.visual_bounding_rect())).collect()
}

/// The lines drawn in `color` `width` wide, each its points.
fn paths(h: &Harness<'_, Moonglow>, color: egui::Color32, width: f32) -> Vec<Vec<egui::Pos2>> {
    over_view(h)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Path(p) if (p.stroke.width - width).abs() < 1e-3 => match p.stroke.color {
                egui::epaint::ColorMode::Solid(c) if c == color => Some(p.points),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// The words of the frame's text, in order.
fn words(h: &Harness<'_, Moonglow>) -> Vec<String> {
    over_view(h)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

/// The solid triangles of the frame filled with `color` (the arrows'
/// heads, the facing marks).
fn triangles(h: &Harness<'_, Moonglow>, color: egui::Color32) -> usize {
    over_view(h)
        .into_iter()
        .filter(|s| matches!(s, egui::Shape::Path(p) if p.points.len() == 3 && p.fill == color))
        .count()
}

fn shift(h: &mut Harness<'_, Moonglow>, on: bool) {
    let m = if on { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
    h.event(egui::Event::ModifiersChanged(m));
    h.run_steps(2);
}

/// How far `p` is from the polyline through `line` (closed: back to its
/// first point).
fn off_line(p: egui::Pos2, line: &[egui::Pos2], closed: bool) -> f32 {
    let n = line.len();
    let pieces = if closed { n } else { n - 1 };
    (0..pieces)
        .map(|k| {
            let (a, b) = (line[k], line[(k + 1) % n]);
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_sq().max(1e-9)).clamp(0.0, 1.0);
            p.distance(a + ab * t)
        })
        .fold(f32::MAX, f32::min)
}

/// The circle about `pivot` through `u` and `v` (units, `radius`) as the
/// camera projects it, at 2,000 points: by `mg_area::pick::project`, the
/// camera rebuilt from its orbit, not by the view's own projector.
fn true_circle(
    h: &Harness<'_, Moonglow>,
    area: ResRef,
    pivot: Vec3,
    (u, v): (Vec3, Vec3),
    radius: f32,
) -> Vec<egui::Pos2> {
    let view = &h.state().area_views[&area];
    let camera = view.orbit.unwrap().camera();
    let rect = view.rect;
    let aspect = rect.width() / rect.height();
    (0..2000)
        .map(|k| {
            let a = std::f32::consts::TAU * k as f32 / 2000.0;
            let p = pivot + (u * a.cos() + v * a.sin()) * radius;
            let at = mg_area::pick::project(&camera, aspect, p).expect("in front");
            rect.min + egui::vec2(at.x * rect.width(), at.y * rect.height())
        })
        .collect()
}

/// Each drawn line lies on the true circle, its pieces' middles too, and
/// covers it: within half a point both ways.
#[track_caller]
fn on_circle(lines: &[(Vec<egui::Pos2>, bool)], circle: &[egui::Pos2], what: &str) {
    assert!(!lines.is_empty(), "{what}: drawn");
    for (line, _) in lines {
        let mut points: Vec<egui::Pos2> = line.clone();
        points.extend(line.windows(2).map(|w| w[0] + (w[1] - w[0]) / 2.0));
        for p in points {
            let off = off_line(p, circle, true);
            assert!(off <= 0.5, "{what}: {p:?} is {off:.2} points off the circle");
        }
    }
    for &c in circle {
        let off = lines.iter().map(|(l, closed)| off_line(c, l, *closed)).fold(f32::MAX, f32::min);
        assert!(off <= 0.5, "{what}: the circle's {c:?} is {off:.2} points from what is drawn");
    }
}

/// The turning ring and the two tilt rings are the circles the camera
/// projects (the camera rebuilt from the view's orbit), within half a
/// point; each line over its dark edge two points wider; a tilt ring's
/// half behind its pivot at 40%.
#[test]
fn the_rings_are_projected_circles() {
    let Some((mut h, area)) = chest_harness("handles-rings") else { return };
    let t = tokens();
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let turn = paths(&h, t.handle_turn, 1.5);
    let lines: Vec<_> = turn.iter().map(|l| (l.clone(), true)).collect();
    let circle = true_circle(&h, area, CHEST, (Vec3::X, Vec3::Y), radius);
    on_circle(&lines, &circle, "the turning ring");
    let edge = paths(&h, egui::Color32::from_black_alpha(140), 3.5);
    assert!(edge.iter().any(|e| *e == turn[0]), "the turning ring over its edge");
    // With Shift, the tilt rings: about the model's X (red, upright in its
    // Y and Z) and its Y (green, in its X and Z); each in two halves, the
    // far one at 40%.
    // (The model's axes: turned by its bearing.)
    let turned = {
        let view = &h.state().area_views[&area];
        let o =
            view.model.as_ref().unwrap().objects.iter().find(|o| o.kind == ObjectKind::Placeable);
        glam::Quat::from_rotation_z(o.unwrap().rotation)
    };
    let (x, y) = (turned * Vec3::X, turned * Vec3::Y);
    shift(&mut h, true);
    for (color, plane, what) in
        [(t.handle_x, (y, Vec3::Z), "the red ring"), (t.handle_y, (x, Vec3::Z), "the green ring")]
    {
        // (Not the arrow's head line, of the same color and width.)
        let ring = |c| paths(&h, c, 1.5).into_iter().filter(|l| l.len() > 2).collect::<Vec<_>>();
        let (near, far) = (ring(color), ring(color.gamma_multiply(0.4)));
        assert!(!near.is_empty() && !far.is_empty(), "{what}: both halves ({near:?}, {far:?})");
        let lines: Vec<_> = near.iter().chain(&far).map(|l| (l.clone(), false)).collect();
        let circle = true_circle(&h, area, CHEST, plane, radius);
        on_circle(&lines, &circle, what);
        // The far half is the one away from the eye: each of its points
        // where the circle's point is behind the pivot as seen from it.
        let eye = h.state().area_views[&area].camera().unwrap().eye;
        let behind = |p: egui::Pos2, k: usize| {
            let a = std::f32::consts::TAU * k as f32 / 2000.0;
            let at = CHEST + (plane.0 * a.cos() + plane.1 * a.sin()) * radius;
            let _ = p;
            (at - CHEST).dot(eye - CHEST) <= 0.02 * radius * (eye - CHEST).length()
        };
        for (half, back) in [(&far, true), (&near, false)] {
            for line in half {
                for &p in &line[1..line.len() - 1] {
                    let k = (0..circle.len())
                        .min_by(|&a, &b| circle[a].distance(p).total_cmp(&circle[b].distance(p)))
                        .unwrap();
                    assert_eq!(
                        behind(p, k),
                        back,
                        "{what}: {p:?} in the {} half",
                        if back { "far" } else { "near" }
                    );
                }
            }
        }
    }
    shift(&mut h, false);
}

/// While one handle is under the pointer it is lit (brighter, thicker)
/// and the rest of its set draws at half strength; with none, all whole.
#[test]
fn the_rest_of_a_set_is_at_half_strength_while_one_is_lit() {
    let Some((mut h, area)) = chest_harness("handles-dimmed") else { return };
    let t = tokens();
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    shift(&mut h, true);
    assert_eq!(triangles(&h, t.handle_x), 1, "the X head, whole");
    assert_eq!(triangles(&h, t.handle_y), 1, "the Y head, whole");
    h.hover_at(screen(&h, area, CHEST + Vec3::X * radius * 1.45));
    h.run_steps(2);
    assert_eq!(triangles(&h, t.handle_x_lit), 1, "the X head lit");
    assert_eq!(triangles(&h, t.handle_y.gamma_multiply(0.5)), 1, "the Y head at half");
    assert_eq!(triangles(&h, t.handle_z.gamma_multiply(0.5)), 1, "the Z head at half");
    assert!(!paths(&h, t.handle_x_lit, 2.5).is_empty(), "the X head's line lit, thicker");
    let half_rings = paths(&h, t.handle_y.gamma_multiply(0.5), 1.5);
    assert!(!half_rings.is_empty(), "the green ring at half");
    shift(&mut h, false);
}

/// The pointer is a hand over every handle and closed while one is held;
/// not over the object itself, nor with Ctrl (the camera's).
#[test]
fn the_pointer_is_a_hand_over_every_handle() {
    let Some((mut h, area)) = chest_harness("handles-cursor") else { return };
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let cursor = |h: &Harness<'_, Moonglow>| h.output().platform_output.cursor_icon;
    let ring = CHEST + Vec3::new(0.3f32.cos(), 0.3f32.sin(), 0.0) * radius;
    let right = {
        let cam = h.state().area_views[&area].camera().unwrap();
        (cam.target - cam.eye).cross(Vec3::Z).normalize()
    };
    let square = CHEST + (right + Vec3::Z).normalize() * radius * 1.25;
    let over = |h: &mut Harness<'_, Moonglow>, p: Vec3, m: egui::Modifiers| {
        h.event(egui::Event::ModifiersChanged(m));
        h.hover_at(screen(h, area, p));
        h.run_steps(2);
        cursor(h)
    };
    let none = egui::Modifiers::NONE;
    let shifted = egui::Modifiers::SHIFT;
    assert_eq!(over(&mut h, ring, none), egui::CursorIcon::Grab, "the turning ring");
    for axis in 0..3 {
        let head = CHEST + Vec3::AXES[axis] * radius * 1.45;
        assert_eq!(over(&mut h, head, shifted), egui::CursorIcon::Grab, "arrow {axis}");
    }
    let tilt = CHEST + Vec3::new(0.0, 1.0f32.sin(), 1.0f32.cos()) * radius;
    assert_eq!(over(&mut h, tilt, shifted), egui::CursorIcon::Grab, "a tilt ring");
    assert_eq!(over(&mut h, square, shifted), egui::CursorIcon::Grab, "the scale square");
    let ctrl = egui::Modifiers::COMMAND;
    assert_ne!(over(&mut h, ring, ctrl), egui::CursorIcon::Grab, "Ctrl: the camera's");
    let grip = {
        let view = &h.state().area_views[&area];
        let i = view
            .model
            .as_ref()
            .unwrap()
            .objects
            .iter()
            .position(|o| o.kind == ObjectKind::Placeable);
        view.move_grip(i.unwrap()).unwrap()
    };
    h.event(egui::Event::ModifiersChanged(none));
    h.hover_at(grip);
    h.run_steps(2);
    assert_ne!(cursor(&h), egui::CursorIcon::Grab, "the chest itself");
    // Held, a closed hand.
    let at = screen(&h, area, ring);
    h.hover_at(at);
    h.run_steps(1);
    press(&h, at, true, none);
    for k in 1..=4 {
        h.hover_at(at + egui::vec2(0.0, 6.0 * k as f32));
    }
    h.run_steps(2);
    assert!(h.state().area_views[&area].is_dragging());
    assert_eq!(cursor(&h), egui::CursorIcon::Grabbing, "held");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    press(&h, at, false, none);
    h.run_steps(2);
}

/// A chip beside the pointer names the handle under it before it is
/// taken (the gizmo rulings' 4); none while one is dragged.
#[test]
fn a_chip_names_the_handle_under_the_pointer() {
    let Some((mut h, area)) = chest_harness("handles-names") else { return };
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let right = {
        let cam = h.state().area_views[&area].camera().unwrap();
        (cam.target - cam.eye).cross(Vec3::Z).normalize()
    };
    let named = |h: &mut Harness<'_, Moonglow>, p: Vec3, m: egui::Modifiers| {
        h.event(egui::Event::ModifiersChanged(m));
        h.hover_at(screen(h, area, p));
        h.run_steps(2);
        let names = [
            "Turn",
            "Move East or West",
            "Move North or South",
            "Raise or Lower",
            "Tilt (Y Rotation)",
            "Tilt (Z Rotation)",
            "Scale",
        ];
        let said: Vec<String> =
            words(h).into_iter().filter(|w| names.contains(&w.as_str())).collect();
        said
    };
    let (none, shifted) = (egui::Modifiers::NONE, egui::Modifiers::SHIFT);
    let ring = CHEST + Vec3::new(0.3f32.cos(), 0.3f32.sin(), 0.0) * radius;
    assert_eq!(named(&mut h, ring, none), ["Turn"]);
    for (axis, name) in
        [(0, "Move East or West"), (1, "Move North or South"), (2, "Raise or Lower")]
    {
        let head = CHEST + Vec3::AXES[axis] * radius * 1.45;
        assert_eq!(named(&mut h, head, shifted), [name]);
    }
    // (The red ring upright in Y and Z, the green in X and Z.)
    let red = CHEST + Vec3::new(0.0, -(1.1f32.sin()), 1.1f32.cos()) * radius;
    assert_eq!(named(&mut h, red, shifted), ["Tilt (Y Rotation)"]);
    let green = CHEST + Vec3::new(1.1f32.sin(), 0.0, 1.1f32.cos()) * radius;
    assert_eq!(named(&mut h, green, shifted), ["Tilt (Z Rotation)"]);
    let square = CHEST + (right + Vec3::Z).normalize() * radius * 1.25;
    assert_eq!(named(&mut h, square, shifted), ["Scale"]);
    // Off the handles, and with Ctrl, none.
    assert!(named(&mut h, CHEST + Vec3::new(3.0, -3.0, 0.0), none).is_empty());
    assert!(named(&mut h, ring, egui::Modifiers::COMMAND).is_empty());
    // Dragged, none (what it is doing shows instead).
    let at = screen(&h, area, ring);
    h.event(egui::Event::ModifiersChanged(none));
    h.hover_at(at);
    h.run_steps(1);
    press(&h, at, true, none);
    // (Along the ring: the pointer stays on a handle.)
    for k in 1..=4 {
        let a = 0.3 + 0.1 * k as f32;
        h.hover_at(screen(&h, area, CHEST + Vec3::new(a.cos(), a.sin(), 0.0) * radius));
    }
    h.run_steps(2);
    assert!(h.state().area_views[&area].is_dragging());
    assert!(!words(&h).iter().any(|w| w == "Turn"), "no name chip during the drag");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    press(&h, at, false, none);
    h.run_steps(2);
}

/// Presses at the first of `path` (window points) and leads the pointer
/// along the rest, `modifiers` held: a drag under way, not let go.
fn drag_along(h: &mut Harness<'_, Moonglow>, path: &[egui::Pos2], modifiers: egui::Modifiers) {
    h.event(egui::Event::ModifiersChanged(modifiers));
    h.hover_at(path[0]);
    h.run_steps(1);
    // (Not `press`: it puts back the harness's own modifiers, none, after
    // the press.)
    let button = egui::PointerButton::Primary;
    h.event(egui::Event::PointerButton { pos: path[0], button, pressed: true, modifiers });
    for p in &path[1..] {
        h.hover_at(*p);
        h.run_steps(1);
    }
    h.run_steps(2);
}

/// Drops the drag under way by Escape and lets go at `at`: nothing
/// changed.
#[track_caller]
fn drop_drag(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    press(h, at, false, egui::Modifiers::NONE);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(2);
    assert!(!h.state().area_views.values().next().unwrap().is_dragging(), "Escape drops the drag");
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "Escape changes nothing");
}

/// The readout chips over the view: each its words, its chip's fill and
/// its strip's color.
fn chips(h: &Harness<'_, Moonglow>) -> Vec<(String, egui::Color32, egui::Color32)> {
    let shapes = over_view(h);
    let rects: Vec<(egui::Rect, egui::Color32)> = shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r) => Some((r.rect, r.fill)),
            _ => None,
        })
        .collect();
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => {
                let at = t.visual_bounding_rect().center();
                // (The chip: the rect 18 high about the words; its strip
                // the 2 point rect at its left.)
                let chip = rects.iter().find(|(r, _)| r.height() == 18.0 && r.contains(at))?;
                let strip = rects
                    .iter()
                    .find(|(r, _)| r.width() == 2.0 && r.min == chip.0.min && r.height() == 18.0)?;
                Some((t.galley.text().to_owned(), chip.1, strip.1))
            }
            _ => None,
        })
        .collect()
}

/// Each handle's readout while it is dragged is a chip (the proposal's
/// section 11.5), in either theme: `135°` for the turning ring, `Y
/// Rotation 30°` for the red ring, `X 13.00 m` for the east arrow, `Z …
/// m` for the up arrow, `Scale 1.40` for the square, each with a strip in
/// its handle's color; and each drag's marks of where it began (the
/// turn's hollow triangle, the tilt's and the slide's hollow dots, the
/// slide's guide line, the scale's hollow square). Escape drops each,
/// nothing changed; the Status Bar hints the grid's snap for the east
/// arrow and none for the up one.
#[test]
fn each_handle_s_readout_is_a_chip() {
    let Some((mut h, area)) = chest_harness("handles-chips") else { return };
    h.state_mut().settings.snap_angle = Some(15);
    h.state_mut().settings.snap_grid = Some(100);
    h.run_steps(2);
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let (none, shifted) = (egui::Modifiers::NONE, egui::Modifiers::SHIFT);
    let ring = |a: f32| CHEST + Vec3::new(a.cos(), a.sin(), 0.0) * radius;
    let red = |a: f32| CHEST + Vec3::new(0.0, -a.sin(), a.cos()) * radius;
    let right = {
        let cam = h.state().area_views[&area].camera().unwrap();
        (cam.target - cam.eye).cross(Vec3::Z).normalize()
    };
    let square = CHEST + (right + Vec3::Z).normalize() * radius * 1.25;
    for light in [false, true] {
        h.state_mut().settings.light_theme = light;
        h.run_steps(2);
        let t = if light { mg_uikit::Tokens::LIGHT } else { mg_uikit::Tokens::DARK };
        let chip_of = |h: &Harness<'_, Moonglow>, words: &str| {
            let all = chips(h);
            let found = all.iter().find(|c| c.0 == words).cloned();
            found.unwrap_or_else(|| panic!("no chip {words:?}: {all:?}"))
        };
        let theme = if light { "light" } else { "dark" };
        // The turning ring, 45 degrees round from its facing at 90.
        let path: Vec<_> =
            (0..=6).map(|k| screen(&h, area, ring(0.3 + 0.785 * k as f32 / 6.0))).collect();
        drag_along(&mut h, &path, none);
        assert_eq!(chip_of(&h, "135°"), ("135°".into(), t.chip, t.handle_turn), "{theme}");
        let hollow = paths(&h, t.handle_turn.gamma_multiply(0.85), 1.0);
        assert!(hollow.iter().any(|p| p.len() == 3), "{theme}: where the turn began");
        drop_drag(&mut h, path[6]);
        // The red ring, a twelfth round.
        let path: Vec<_> = (0..=6)
            .map(|k| screen(&h, area, red(1.1 + std::f32::consts::FRAC_PI_6 * k as f32 / 6.0)))
            .collect();
        drag_along(&mut h, &path, shifted);
        let chip = chip_of(&h, "Y Rotation 30°");
        assert_eq!((chip.1, chip.2), (t.chip, t.handle_x), "{theme}");
        let dots = over_view(&h).into_iter().filter(|s| {
            matches!(s, egui::Shape::Circle(c) if c.stroke.color == t.handle_x.gamma_multiply(0.85))
        });
        assert_eq!(dots.count(), 1, "{theme}: where the tilt began");
        drop_drag(&mut h, path[6]);
        // The east arrow, a meter on, the grid's 1 m snap.
        let head = CHEST + Vec3::X * radius * 1.45;
        let path: Vec<_> =
            (0..=5).map(|k| screen(&h, area, head + Vec3::X * k as f32 / 5.0)).collect();
        drag_along(&mut h, &path, shifted);
        let chip = chip_of(&h, "X 13.00 m");
        assert_eq!((chip.1, chip.2), (t.chip, t.handle_x), "{theme}");
        // (Across the view and past it: of all the frame's shapes.)
        let guide = drawn(&h).into_iter().filter(|s| {
            matches!(s, egui::Shape::LineSegment { stroke, .. }
                if stroke.color == t.handle_x.gamma_multiply(0.35) && stroke.width == 1.0)
        });
        assert_eq!(guide.count(), 1, "{theme}: the guide line");
        let began = screen(&h, area, CHEST);
        let dot = over_view(&h).into_iter().any(|s| {
            matches!(s, egui::Shape::Circle(c) if c.center.distance(began) < 0.5
                && c.stroke.color == t.handle_x.gamma_multiply(0.85))
        });
        assert!(dot, "{theme}: a hollow dot where the slide began");
        assert!(h.state().status_said().iter().any(|s| s == "Snap 1 m"), "{theme}: the snap");
        drop_drag(&mut h, path[5]);
        // The up arrow, a meter up: no snap.
        let head = CHEST + Vec3::Z * radius * 1.45;
        let path: Vec<_> =
            (0..=5).map(|k| screen(&h, area, head + Vec3::Z * k as f32 / 5.0)).collect();
        drag_along(&mut h, &path, shifted);
        let up = chips(&h).into_iter().find(|c| c.0.starts_with("Z ") && c.0.ends_with(" m"));
        let up = up.unwrap_or_else(|| panic!("{theme}: the up arrow's chip: {:?}", chips(&h)));
        assert_eq!((up.1, up.2), (t.chip, t.handle_z), "{theme}");
        assert!(
            !h.state().status_said().iter().any(|s| s.starts_with("Snap")),
            "{theme}: no snap up and down: {:?}",
            h.state().status_said()
        );
        drop_drag(&mut h, path[5]);
        // The scale square, pulled out to 1.4 times as far.
        let (foot, at) = (screen(&h, area, CHEST), screen(&h, area, square));
        let path: Vec<_> = (0..=5).map(|k| at + (at - foot) * (0.08 * k as f32)).collect();
        drag_along(&mut h, &path, shifted);
        let chip = chip_of(&h, "Scale 1.40");
        assert_eq!((chip.1, chip.2), (t.chip, t.handle_scale), "{theme}");
        let hollow = paths(&h, t.handle_scale.gamma_multiply(0.85), 1.25);
        let square_at =
            |p: &Vec<egui::Pos2>| p.len() == 4 && (p[0] + (p[2] - p[0]) / 2.0).distance(at) < 0.5;
        assert!(hollow.iter().any(square_at), "{theme}: a hollow square where it began");
        drop_drag(&mut h, path[5]);
    }
}

/// A turn of the ring draws a wedge from where it began, and with the
/// Turn snap set ticks on the ring at its steps (18 at 20 degrees, at the
/// facings a snapped turn lands on); without it, none.
#[test]
fn a_turn_s_wedge_and_the_snap_s_ticks() {
    let Some((mut h, area)) = chest_harness("handles-ticks") else { return };
    let t = tokens();
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let ring = |a: f32| CHEST + Vec3::new(a.cos(), a.sin(), 0.0) * radius;
    let ticks = |h: &Harness<'_, Moonglow>| -> Vec<egui::Pos2> {
        let lines = paths(h, t.handle_turn, 1.0);
        lines
            .into_iter()
            .filter(|l| l.len() == 2 && (l[0].distance(l[1]) - 5.0).abs() < 0.01)
            .map(|l| l[0])
            .collect()
    };
    let wedges = |h: &Harness<'_, Moonglow>| {
        let fill = t.handle_turn.gamma_multiply(0.18);
        over_view(h)
            .into_iter()
            .filter(
                |s| matches!(s, egui::Shape::Mesh(m) if m.vertices.iter().all(|v| v.color == fill)),
            )
            .count()
    };
    // (At 20 degrees: the facings a snapped turn lands on are not the
    // multiples of the step from east, as at 15.)
    for snap in [Some(20u16), None] {
        h.state_mut().settings.snap_angle = snap;
        h.run_steps(2);
        assert!(ticks(&h).is_empty(), "no ticks before the turn");
        let path: Vec<_> =
            (0..=6).map(|k| screen(&h, area, ring(0.3 + 0.785 * k as f32 / 6.0))).collect();
        drag_along(&mut h, &path, egui::Modifiers::NONE);
        assert_eq!(wedges(&h), 1, "the turn's wedge ({snap:?})");
        let at = ticks(&h);
        match snap {
            Some(step) => {
                assert_eq!(at.len(), 360 / step as usize, "a tick a step");
                for n in 0..360 / step {
                    let a = std::f32::consts::FRAC_PI_2 + f32::from(n * step).to_radians();
                    let on = screen(&h, area, ring(a));
                    let facing = 90 + n * step;
                    assert!(at.iter().any(|p| p.distance(on) < 0.5), "a tick at {facing}°");
                }
            }
            None => assert!(at.is_empty(), "no snap, no ticks: {at:?}"),
        }
        drop_drag(&mut h, path[6]);
    }
}

/// During a scale drag the turning ring is drawn at half strength (as
/// today, and as the Gizmos-scale-drag board draws it); during an
/// arrow's or a tilt ring's, it is not drawn.
#[test]
fn the_turning_ring_is_at_half_strength_during_a_scale_drag_alone() {
    let Some((mut h, area)) = chest_harness("handles-scale-ring") else { return };
    let t = tokens();
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let shifted = egui::Modifiers::SHIFT;
    let turning = |h: &Harness<'_, Moonglow>, color: egui::Color32| {
        paths(h, color, 1.5).into_iter().filter(|l| l.len() > 40).count()
    };
    let half = t.handle_turn.gamma_multiply(0.5);
    let right = {
        let cam = h.state().area_views[&area].camera().unwrap();
        (cam.target - cam.eye).cross(Vec3::Z).normalize()
    };
    let (foot, at) = (
        screen(&h, area, CHEST),
        screen(&h, area, CHEST + (right + Vec3::Z).normalize() * radius * 1.25),
    );
    let path: Vec<_> = (0..=5).map(|k| at + (at - foot) * (0.08 * k as f32)).collect();
    drag_along(&mut h, &path, shifted);
    assert_eq!(turning(&h, half), 1, "the turning ring at half during a scale drag");
    assert_eq!(turning(&h, t.handle_turn), 0, "not whole");
    drop_drag(&mut h, path[5]);
    let head = CHEST + Vec3::X * radius * 1.45;
    let path: Vec<_> = (0..=5).map(|k| screen(&h, area, head + Vec3::X * k as f32 / 5.0)).collect();
    drag_along(&mut h, &path, shifted);
    assert!(h.state().area_views[&area].is_dragging());
    assert_eq!(turning(&h, half) + turning(&h, t.handle_turn), 0, "none during an arrow's");
    drop_drag(&mut h, path[5]);
    let red = |a: f32| CHEST + Vec3::new(0.0, -a.sin(), a.cos()) * radius;
    let path: Vec<_> = (0..=6)
        .map(|k| screen(&h, area, red(1.1 + std::f32::consts::FRAC_PI_6 * k as f32 / 6.0)))
        .collect();
    drag_along(&mut h, &path, shifted);
    assert!(h.state().area_views[&area].is_dragging());
    assert_eq!(turning(&h, half) + turning(&h, t.handle_turn), 0, "none during a tilt's");
    drop_drag(&mut h, path[6]);
}

/// The ring under the pointer stays lit, and the rest of the set whole,
/// while E turns the object before the keys rest (the rings drawn turned,
/// the handle found under the pointer of the object as the area has it).
#[test]
fn a_ring_stays_lit_while_e_turns_it() {
    let Some((mut h, area)) = chest_harness("handles-lit-turned") else { return };
    let t = tokens();
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let rings = |h: &Harness<'_, Moonglow>, color, width| {
        paths(h, color, width).into_iter().filter(|l| l.len() > 40).count()
    };
    h.hover_at(screen(&h, area, CHEST + Vec3::new(0.3f32.cos(), 0.3f32.sin(), 0.0) * radius));
    h.run_steps(2);
    assert_eq!(rings(&h, t.handle_turn_lit, 2.5), 1, "lit under the pointer");
    h.key_press(egui::Key::E);
    h.run_steps(2);
    assert!(h.state().area_views[&area].key_turning(), "turned, not yet a command");
    assert_eq!(rings(&h, t.handle_turn_lit, 2.5), 1, "still lit while turned");
    assert_eq!(rings(&h, t.handle_turn.gamma_multiply(0.5), 1.5), 0, "not at half");
    rest_turn_keys(&mut h, area);
}
