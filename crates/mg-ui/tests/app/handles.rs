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

/// Where the second chest of [`two_chests`] stands, turned by
/// [`SECOND_TURN`].
const SECOND: Vec3 = Vec3::new(15.0, 20.0, 0.0);
const SECOND_TURN: f32 = 0.3;

/// Where the encounter's spawn point stands (the encounter about 6, 6),
/// and the start location (facing north).
const SPAWN: Vec3 = Vec3::new(5.5, 5.5, 0.0);
const START: Vec3 = Vec3::new(30.0, 12.0, 0.0);

/// [`area_harness`]'s area with a dynamic chest at [`CHEST`], selected,
/// seen as the Gizmos boards see theirs (yaw 0.75, pitch 0.72, 10 m); an
/// encounter with a spawn point at [`SPAWN`] and the start location at
/// [`START`] aside.
fn chest_harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    scene_harness(name, &[(CHEST, 0.0)])
}

/// [`chest_harness`] with a second chest at [`SECOND`], turned, both
/// selected.
fn two_chests(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    scene_harness(name, &[(CHEST, 0.0), (SECOND, SECOND_TURN)])
}

fn scene_harness(
    name: &str,
    chests: &[(Vec3, f32)],
) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let chests = chests.to_vec();
    let n = chests.len();
    let scene = move |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::instances::{Placement, Placing, instance};
        let bp = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let chest = bp("plc_chest1", ResType::UTP);
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let items = (chests.iter())
            .map(|(p, r)| {
                let at = Placement { position: p.to_array(), rotation: *r };
                let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
                item.set("Static", mg_gff::Value::Byte(0));
                item
            })
            .collect();
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        git.root.set("Placeable List", mg_gff::Value::List(items));
        let ute = bp("nw_giantevil", ResType::UTE);
        let outline = [[-2.0, -2.0, 0.0], [2.0, -2.0, 0.0], [2.0, 2.0, 0.0], [-2.0, 2.0, 0.0]];
        let at = Placement { position: [6.0, 6.0, 0.0], rotation: 0.0 };
        let mut enc = instance(&placing, ResType::UTE, &ute, at, &outline).unwrap();
        let mut spawn = mg_gff::Struct::new(2);
        for (label, v) in [("X", SPAWN.x), ("Y", SPAWN.y), ("Z", 0.0), ("Orientation", 0.5)] {
            spawn.set(label, mg_gff::Value::Float(v));
        }
        enc.set("SpawnPointList", mg_gff::Value::List(vec![spawn]));
        git.root.set("Encounter List", mg_gff::Value::List(vec![enc]));
        m.set_gff(git_key, &git).unwrap();
        let ifo_key = ResKey::parse("module", ResType::IFO).unwrap();
        let mut ifo = m.gff(&ifo_key).unwrap().unwrap();
        ifo.root.set("Mod_Entry_Area", mg_gff::Value::ResRef(area.to_string().into_bytes()));
        for (label, v) in [
            ("Mod_Entry_X", START.x),
            ("Mod_Entry_Y", START.y),
            ("Mod_Entry_Z", 0.0),
            ("Mod_Entry_Dir_X", 0.0),
            ("Mod_Entry_Dir_Y", 1.0),
        ] {
            ifo.root.set(label, mg_gff::Value::Float(v));
        }
        m.set_gff(ifo_key, &ifo).unwrap();
    };
    let (mut h, area) = area_harness_with(name, "ttr01", None, 4, None, &scene)?;
    h.run_steps(3);
    look(&mut h, area, CHEST);
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    view.selection = (0..n).map(|i| (ObjectKind::Placeable, i)).collect();
    view.show_start = true;
    h.run_steps(3);
    Some((h, area))
}

/// The camera on `at` as the boards have theirs.
fn look(h: &mut Harness<'_, Moonglow>, area: ResRef, at: Vec3) {
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    let o = view.orbit.as_mut().unwrap();
    (o.target, o.yaw, o.pitch, o.distance) = (at + Vec3::Z * 0.5, 0.75, 0.72, 10.0);
    h.run_steps(2);
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
    // (In 96 pieces, closed: 96 points.)
    assert_eq!(turn.iter().map(Vec::len).collect::<Vec<_>>(), [96], "the turning ring's points");
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
        // (Each half 48 of the 96 pieces: 49 points.)
        let counts: Vec<usize> = near.iter().chain(&far).map(Vec::len).collect();
        assert_eq!(counts, [49, 49], "{what}: its halves' points");
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
    // The pivot dot stays whole (neither section 11.3 nor the board dims
    // it).
    let dot = over_view(&h).into_iter().any(
        |s| matches!(s, egui::Shape::Circle(c) if c.radius == 2.0 && c.fill == t.handle_scale),
    );
    assert!(dot, "the pivot dot whole");
    shift(&mut h, false);
}

/// Only the hovered object's own set dims (section 11.3, the owner's
/// ruling on U7's review): the other selected objects' sets stay whole.
#[test]
fn only_the_hovered_object_s_set_dims() {
    let Some((mut h, area)) = two_chests("handles-dim-own-set") else { return };
    let t = tokens();
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    shift(&mut h, true);
    assert_eq!(triangles(&h, t.handle_y), 2, "both Y heads, whole");
    h.hover_at(screen(&h, area, CHEST + Vec3::X * radius * 1.45));
    h.run_steps(2);
    assert_eq!(triangles(&h, t.handle_x_lit), 1, "the first's X head lit");
    assert_eq!(triangles(&h, t.handle_y.gamma_multiply(0.5)), 1, "the first's Y head at half");
    assert_eq!(triangles(&h, t.handle_y), 1, "the second's Y head whole");
    assert_eq!(triangles(&h, t.handle_x), 1, "the second's X head whole");
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
    let alt = egui::Modifiers::ALT;
    assert_ne!(over(&mut h, ring, alt), egui::CursorIcon::Grab, "Alt: the drag raises");
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
    // Its bottom-left corner 12 right of the pointer and 12 up.
    let pointer = screen(&h, area, ring);
    let corner = chip_corner(&h, "Turn");
    assert!(corner.distance(pointer + egui::vec2(12.0, -12.0)) < 0.5, "{corner:?} {pointer:?}");
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
    assert!(named(&mut h, ring, egui::Modifiers::ALT).is_empty(), "Alt: the drag raises");
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

/// Each chip over the view: its words and its rect.
fn chip_rects(h: &Harness<'_, Moonglow>) -> Vec<(String, egui::Rect)> {
    let shapes = over_view(h);
    let rects: Vec<egui::Rect> = (shapes.iter())
        .filter_map(|s| match s {
            egui::Shape::Rect(r) if r.rect.height() == 18.0 => Some(r.rect),
            _ => None,
        })
        .collect();
    (shapes.iter())
        .filter_map(|s| match s {
            egui::Shape::Text(t) => {
                let at = t.visual_bounding_rect().center();
                let chip = rects.iter().find(|r| r.contains(at) && r.width() > 2.0)?;
                Some((t.galley.text().to_owned(), *chip))
            }
            _ => None,
        })
        .collect()
}

/// The bottom-left corner of the chip saying `words`.
#[track_caller]
fn chip_corner(h: &Harness<'_, Moonglow>, words: &str) -> egui::Pos2 {
    let all = chip_rects(h);
    let found = all.iter().find(|(w, _)| w == words);
    found.unwrap_or_else(|| panic!("no chip {words:?}: {all:?}")).1.left_bottom()
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
        // (Its corner 10 right and 10 up of the facing's triangle on the
        // ring, section 11.5.)
        let mark = screen(&h, area, ring(135f32.to_radians()));
        let corner = chip_corner(&h, "135°");
        assert!(corner.distance(mark + egui::vec2(10.0, -10.0)) < 0.5, "{theme}: {corner:?}");
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
        // (10 right and 10 up of the grip's dot.)
        let grip = over_view(&h).into_iter().find_map(|s| match s {
            egui::Shape::Circle(c) if c.radius == 3.5 && c.fill == t.handle_x_lit => Some(c.center),
            _ => None,
        });
        let grip = grip.unwrap_or_else(|| panic!("{theme}: the grip's dot"));
        let corner = chip_corner(&h, "Y Rotation 30°");
        assert!(corner.distance(grip + egui::vec2(10.0, -10.0)) < 0.5, "{theme}: {corner:?}");
        let fill = t.handle_x.gamma_multiply(0.18);
        let wedge = over_view(&h).into_iter().any(|s| {
            matches!(s, egui::Shape::Mesh(m) if !m.vertices.is_empty()
                && m.vertices.iter().all(|v| v.color == fill))
        });
        assert!(wedge, "{theme}: the tilt's wedge");
        // The dragged ring alone, as the board draws it.
        let green = paths(&h, t.handle_y, 1.5).into_iter().chain(paths(&h, t.handle_y, 2.5));
        let green = green.chain(paths(&h, t.handle_y.gamma_multiply(0.4), 1.5));
        let green = green.chain(paths(&h, t.handle_y.gamma_multiply(0.5), 1.5));
        assert_eq!(green.filter(|l| l.len() > 2).count(), 0, "{theme}: no green ring");
        let said = h.state().status_said().to_vec();
        assert!(said.iter().any(|s| s == "Turn 15°"), "{theme}: the tilt's snap: {said:?}");
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
        // (10 right and 4 up of the arrow's tip, where the chest is now.)
        let now = CHEST + Vec3::X;
        let r = h.state().area_views[&area].handle_radius_at(now).unwrap();
        let tip = screen(&h, area, now + Vec3::X * r * 1.7);
        let corner = chip_corner(&h, "X 13.00 m");
        assert!(corner.distance(tip + egui::vec2(10.0, -4.0)) < 0.5, "{theme}: {corner:?} {tip:?}");
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
        // (12 right and 12 up of the pointer.)
        let corner = chip_corner(&h, "Scale 1.40");
        assert!(corner.distance(path[5] + egui::vec2(12.0, -12.0)) < 0.5, "{theme}: {corner:?}");
        let said = h.state().status_said().to_vec();
        assert!(said.iter().any(|s| s == "Snap 0.05"), "{theme}: the scale's snap: {said:?}");
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
    // multiples of the step from east, as at 15; at 25, a step that does
    // not divide 360, 15 of them within half a turn either way.)
    for snap in [Some(20u16), Some(25), None] {
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
                // (The chest's rotation is 0: the multiples of the step
                // from -180 to under 180, each a quarter turn on from it.)
                let step = i32::from(step);
                let turns: Vec<i32> =
                    (-180 / step..=180 / step).map(|n| n * step).filter(|a| *a < 180).collect();
                assert_eq!(at.len(), turns.len(), "a tick a step ({step})");
                for turn in turns {
                    let a = std::f32::consts::FRAC_PI_2 + (turn as f32).to_radians();
                    let on = screen(&h, area, ring(a));
                    let facing = 90 + turn;
                    assert!(
                        at.iter().any(|p| p.distance(on) < 0.5),
                        "a tick at {facing}° ({step})"
                    );
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

/// With two objects at different bearings selected, each ring's ticks are
/// at its own facing turned by the turns the first object's snapped
/// rotation lands on (not at the first's facings on every ring).
#[test]
fn each_ring_s_ticks_are_its_own() {
    let Some((mut h, area)) = two_chests("handles-ticks-two") else { return };
    let t = tokens();
    h.state_mut().settings.snap_angle = Some(20);
    h.run_steps(2);
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let ring = |a: f32| CHEST + Vec3::new(a.cos(), a.sin(), 0.0) * radius;
    let path: Vec<_> =
        (0..=6).map(|k| screen(&h, area, ring(0.3 + 0.785 * k as f32 / 6.0))).collect();
    drag_along(&mut h, &path, egui::Modifiers::NONE);
    let ticks: Vec<egui::Pos2> = (paths(&h, t.handle_turn, 1.0).into_iter())
        .filter(|l| l.len() == 2 && (l[0].distance(l[1]) - 5.0).abs() < 0.01)
        .map(|l| l[0])
        .collect();
    let second = h.state().area_views[&area].handle_radius_at(SECOND).unwrap();
    // (The second's facing at the press: its rotation and a quarter turn;
    // the first's rotation is 0, so the turns are the step's multiples.)
    let facing = SECOND_TURN + std::f32::consts::FRAC_PI_2;
    for n in -9..9 {
        let a = facing + (n as f32 * 20.0).to_radians();
        let on = screen(&h, area, SECOND + Vec3::new(a.cos(), a.sin(), 0.0) * second);
        assert!(ticks.iter().any(|p| p.distance(on) < 0.5), "the second's tick {n}: {on:?}");
    }
    assert_eq!(ticks.len(), 36, "18 on each ring");
    drop_drag(&mut h, path[6]);
}

/// A tilt from 170 degrees on by 30 reads -160 (the field's value, as the
/// Properties and Adjust Location have it, within half a turn) and its
/// wedge spans the 30 degrees turned, not the 330 the other way round.
#[test]
fn a_tilt_across_half_a_turn_goes_the_short_way() {
    let Some((mut h, area)) = chest_harness("handles-tilt-across") else { return };
    let t = tokens();
    h.state_mut().settings.snap_angle = Some(10);
    h.run_steps(2);
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let red = |a: f32| CHEST + Vec3::new(0.0, -a.sin(), a.cos()) * radius;
    let shifted = egui::Modifiers::SHIFT;
    // First to 170, in small steps, let go.
    let path: Vec<_> = (0..=34)
        .map(|k| screen(&h, area, red(1.1 + 170f32.to_radians() * k as f32 / 34.0)))
        .collect();
    drag_along(&mut h, &path, shifted);
    let last = *path.last().unwrap();
    let button = egui::PointerButton::Primary;
    h.event(egui::Event::PointerButton { pos: last, button, pressed: false, modifiers: shifted });
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let git = ResKey::new(area, ResType::GIT);
    let rotate = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let chest = ws.doc(&git).unwrap().root.list("Placeable List").unwrap()[0].clone();
        mg_area::VisualTransform::read(&chest).map(|v| v.rotate.y)
    };
    assert_eq!(rotate(&mut h), Some(170.0), "tilted to 170 first");
    // Then 30 more.
    let from = 1.1 + 170f32.to_radians();
    let path: Vec<_> = (0..=6)
        .map(|k| screen(&h, area, red(from + 30f32.to_radians() * k as f32 / 6.0)))
        .collect();
    drag_along(&mut h, &path, shifted);
    let all = chip_rects(&h);
    assert!(all.iter().any(|(w, _)| w == "Y Rotation -160°"), "the field's value: {all:?}");
    let fill = t.handle_x.gamma_multiply(0.18);
    let wedge: Vec<usize> = (over_view(&h).into_iter())
        .filter_map(|s| match s {
            egui::Shape::Mesh(m) if m.vertices.iter().all(|v| v.color == fill) => {
                Some(m.vertices.len())
            }
            _ => None,
        })
        .collect();
    // (30 degrees in pieces of at most 3.75: 8 or 9, so 10 or 11 corners
    // with the pivot; the long way, 330 degrees, would take 90.)
    assert!(matches!(wedge.as_slice(), [10 | 11]), "the wedge spans 30 degrees: {wedge:?}");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    h.event(egui::Event::PointerButton {
        pos: path[6],
        button,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(2);
}

/// Where a press would not take a handle, none is lit, named or given the
/// hand: with the palette's blueprint chosen, an appearance or a light
/// being placed by a click, an outline drawn anew (each mode takes the
/// press first, `PressTo`).
#[test]
fn handles_answer_only_where_a_press_takes_them() {
    let Some((mut h, area)) = chest_harness("handles-modes") else { return };
    let t = tokens();
    let radius = h.state().area_views[&area].handle_radius_at(CHEST).unwrap();
    let ring = screen(&h, area, CHEST + Vec3::new(0.3f32.cos(), 0.3f32.sin(), 0.0) * radius);
    let answers = |h: &mut Harness<'_, Moonglow>| {
        h.hover_at(ring + egui::vec2(1.0, 0.0));
        h.run_steps(2);
        h.hover_at(ring);
        h.run_steps(2);
        let grab = h.output().platform_output.cursor_icon == egui::CursorIcon::Grab;
        let named = words(h).iter().any(|w| w == "Turn");
        let lit = paths(h, t.handle_turn_lit, 2.5).into_iter().any(|l| l.len() > 40);
        (grab, named, lit)
    };
    assert_eq!(answers(&mut h), (true, true, true), "no mode in hand");
    type Set = fn(&mut Moonglow, ResRef);
    let modes: [(&str, Set, Set); 4] = [
        (
            "a blueprint chosen",
            |app, _| app.palette.selected = ResKey::parse("plc_chest1", ResType::UTP),
            |app, _| app.palette.selected = None,
        ),
        (
            "an appearance placed",
            |app, _| {
                let taken =
                    mg_ui::appearance_gallery::DraggedAppearance { row: 0, name: "a".into() };
                app.palette.placing_appearance = Some(taken);
            },
            |app, _| app.palette.placing_appearance = None,
        ),
        (
            "a light placed",
            |app, _| app.palette.placing_light = Some(0),
            |app, _| app.palette.placing_light = None,
        ),
        (
            "an outline drawn anew",
            |app, area| {
                app.area_views.get_mut(&area).unwrap().redraw = Some((ObjectKind::Encounter, 0));
            },
            |app, area| app.area_views.get_mut(&area).unwrap().redraw = None,
        ),
    ];
    for (what, on, off) in modes {
        on(h.state_mut(), area);
        h.run_steps(2);
        assert_eq!(answers(&mut h), (false, false, false), "{what}");
        off(h.state_mut(), area);
        h.run_steps(2);
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(ObjectKind::Placeable, 0)];
        view.outline.clear();
        h.run_steps(2);
    }
    assert_eq!(answers(&mut h), (true, true, true), "back in object mode");
}

/// The encounter's spawn point's handles answer without Shift (Shift
/// takes the selection's other handles) and the start location's marker
/// with its own: its name chip in the turning ring's amber, as its
/// readout; its ring and arrow lit, a point thicker, under the pointer;
/// the Status Bar hints the Turn snap while its tip turns it.
#[test]
fn the_spawn_point_and_the_start_location_answer_as_handles() {
    let Some((mut h, area)) = chest_harness("handles-spawn-start") else { return };
    let t = tokens();
    look(&mut h, area, SPAWN);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Encounter, 0)];
    h.run_steps(3);
    let tip = SPAWN + Vec3::Z * 0.1 + Vec3::new(0.5f32.cos(), 0.5f32.sin(), 0.0);
    let named = |h: &mut Harness<'_, Moonglow>, p: Vec3, m: egui::Modifiers, words_: &str| {
        h.event(egui::Event::ModifiersChanged(m));
        h.hover_at(screen(h, area, p));
        h.run_steps(2);
        let grab = h.output().platform_output.cursor_icon == egui::CursorIcon::Grab;
        (words(h).iter().any(|w| w == words_), grab)
    };
    let (none, shifted) = (egui::Modifiers::NONE, egui::Modifiers::SHIFT);
    assert_eq!(named(&mut h, tip, none, "Turn Spawn Point"), (true, true));
    assert_eq!(named(&mut h, SPAWN, none, "Move Spawn Point"), (true, true));
    assert_eq!(named(&mut h, tip, shifted, "Turn Spawn Point"), (false, false), "with Shift");
    h.event(egui::Event::ModifiersChanged(none));
    // The start location, selected.
    look(&mut h, area, START);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection.clear();
        view.start_selected = true;
    }
    h.hover_at(egui::pos2(5.0, 5.0));
    h.run_steps(3);
    let ring = |h: &Harness<'_, Moonglow>, color, width| {
        paths(h, color, width).into_iter().filter(|l| l.len() > 10).count()
    };
    assert_eq!(ring(&h, t.handle_start_ring, 3.0), 1, "its ring, idle");
    let start_tip = START + Vec3::new(0.0, 0.75, 0.05);
    assert_eq!(named(&mut h, start_tip, none, "Turn Start Location"), (true, true));
    let strip = chips(&h).into_iter().find(|c| c.0 == "Turn Start Location").map(|c| c.2);
    assert_eq!(strip, Some(t.handle_turn), "its name chip's strip, the readout's");
    // On its ring: lit, a point thicker.
    let on_ring = START + Vec3::new(-1.0, 0.0, 0.05);
    assert_eq!(named(&mut h, on_ring, none, "Move Start Location"), (true, true));
    assert_eq!(ring(&h, t.handle_start_ring_lit, 4.0), 1, "its ring lit");
    assert_eq!(ring(&h, t.handle_start_ring, 3.0), 0, "not idle");
    // Its tip turned with the Turn snap: the Status Bar says the snap.
    h.state_mut().settings.snap_angle = Some(15);
    h.run_steps(2);
    let at = screen(&h, area, start_tip);
    let path: Vec<_> = (0..=5).map(|k| at + egui::vec2(-8.0 * k as f32, 4.0 * k as f32)).collect();
    drag_along(&mut h, &path, none);
    let said = h.state().status_said().to_vec();
    assert!(said.iter().any(|s| s == "Turn 15°"), "the start's turn snaps: {said:?}");
    drop_drag(&mut h, path[5]);
}
