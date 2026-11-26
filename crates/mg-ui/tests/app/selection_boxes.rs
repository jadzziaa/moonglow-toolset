//! The boxes of the objects selected or marked in the area view, drawn
//! in the 3D picture (`mg_render::Scene::boxes`): what gets one and how
//! it lies with what is drawn over the view (outlines, a bare light's
//! sun, the handles).

use crate::common::*;
use glam::Vec3;
use mg_area::ObjectKind;

/// Where each group stands (ttr01, 4 by 4 tiles): a trigger and an
/// encounter, each with a chest on its edge; a light with no model; a
/// sound, a waypoint and a placeable with no model; a door (with its
/// arrow, Options › Area); a row of chests.
const TRIGGER: Vec3 = Vec3::new(8.0, 8.0, 0.0);
const ENCOUNTER: Vec3 = Vec3::new(18.0, 8.0, 0.0);
const LIGHT: Vec3 = Vec3::new(31.0, 8.0, 1.0);
const SOUND: Vec3 = Vec3::new(11.0, 18.0, 0.0);
const WAYPOINT: Vec3 = Vec3::new(15.0, 18.0, 0.0);
const LOST: Vec3 = Vec3::new(13.0, 21.0, 0.0);
const DOOR: Vec3 = Vec3::new(26.0, 17.0, 0.0);
const CHESTS: Vec3 = Vec3::new(26.0, 28.0, 0.0);

/// The area of the groups above, everything but the sound and the
/// placeable with no model ([`LOST`]) selected.
fn boxes_harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let prepare = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::blueprints as bp;
        use mg_module::instances::{Placement, Placing, instance};
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = |p: Vec3| Placement { position: p.to_array(), rotation: 0.3 };
        let square = |half: f32| {
            [[-half, -half, 0.0], [half, -half, 0.0], [half, half, 0.0], [-half, half, 0.0]]
        };
        let chest = mg_gff::Gff::read(
            &game.resman.get(&ResKey::parse("plc_chest1", ResType::UTP).unwrap()).unwrap(),
        )
        .unwrap()
        .root;
        let mut chests: Vec<Vec3> = vec![TRIGGER + Vec3::X * 4.0, ENCOUNTER + Vec3::X * 4.0];
        chests.extend((0..4).map(|k| CHESTS + Vec3::new(2.0 * k as f32, (k % 2) as f32, 0.0)));
        let mut placed: Vec<mg_gff::Struct> = (chests.iter())
            .map(|&p| {
                let mut s = instance(&placing, ResType::UTP, &chest, at(p), &[]).unwrap();
                s.set("Static", mg_gff::Value::Byte(0));
                s
            })
            .collect();
        // (The game's "Light, White": placeables.2da row 15112.)
        let mut light = bp::placeable(ResRef::EMPTY, "Light", 0);
        light.root.set("Appearance", mg_gff::Value::Dword(15112));
        placed.push(instance(&placing, ResType::UTP, &light.root, at(LIGHT), &[]).unwrap());
        // (And one whose appearance has no row: no model, a marker's box.)
        let mut lost = bp::placeable(ResRef::EMPTY, "Lost", 0);
        lost.root.set("Appearance", mg_gff::Value::Dword(999_999));
        placed.push(instance(&placing, ResType::UTP, &lost.root, at(LOST), &[]).unwrap());
        let trigger = bp::trigger(game, ResRef::EMPTY, "Trigger", 0);
        let trigger =
            instance(&placing, ResType::UTT, &trigger.root, at(TRIGGER), &square(4.0)).unwrap();
        let encounter = bp::encounter(ResRef::EMPTY, "Encounter", 0, Vec::new());
        let encounter =
            instance(&placing, ResType::UTE, &encounter.root, at(ENCOUNTER), &square(4.0)).unwrap();
        let sound = bp::sound(ResRef::EMPTY, "Sound", 0, bp::SoundStyle::LoopingPositional, &[]);
        let sound = instance(&placing, ResType::UTS, &sound.root, at(SOUND), &[]).unwrap();
        let waypoint = mg_module::instances::walk_waypoint(b"MG_BOXED", WAYPOINT.to_array());
        let door = bp::door(ResRef::EMPTY, "Door", 0);
        let door = instance(&placing, ResType::UTD, &door.root, at(DOOR), &[]).unwrap();
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        let list = mg_gff::Value::List;
        git.root.set("Placeable List", list(placed));
        git.root.set("TriggerList", list(vec![trigger]));
        git.root.set("Encounter List", list(vec![encounter]));
        git.root.set("SoundList", list(vec![sound]));
        git.root.set("WaypointList", list(vec![waypoint]));
        git.root.set("Door List", list(vec![door]));
        m.set_gff(git_key, &git).unwrap();
    };
    let (mut h, area) = area_harness_with(name, "ttr01", None, 4, None, &prepare)?;
    settle(&mut h);
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    // (Every kind shown: sounds too.)
    view.show = [true; 9];
    let model = view.model.as_ref().unwrap();
    view.selection = (model.objects.iter())
        .filter(|o| o.kind != ObjectKind::Sound && o.position.distance(LOST) > 0.1)
        .map(|o| (o.kind, o.index))
        .collect();
    settle(&mut h);
    Some((h, area))
}

/// The camera on `at`, from `distance` m, `pitch` down.
fn look(h: &mut Harness<'_, Moonglow>, area: ResRef, at: Vec3, distance: f32, pitch: f32) {
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    let o = view.orbit.as_mut().unwrap();
    (o.target, o.yaw, o.pitch, o.distance) = (at + Vec3::Z * 0.5, 0.6, pitch, distance);
    settle(h);
}

/// The area's view alone, as the window shows it (the area's animations
/// at one clock, so that two runs' pictures compare).
fn view_picture(h: &mut Harness<'_, Moonglow>, area: ResRef) -> image::RgbaImage {
    h.state_mut().area_views.get_mut(&area).unwrap().set_clock(10.0);
    h.run_steps(1);
    let ppp = h.ctx.pixels_per_point();
    let rect = h.state().area_views[&area].rect;
    let shot = h.render().expect("render");
    let (x, y) = ((rect.left() * ppp) as u32, (rect.top() * ppp) as u32);
    let (w, hh) = ((rect.width() * ppp) as u32, (rect.height() * ppp) as u32);
    image::imageops::crop_imm(&shot, x, y, w, hh).to_image()
}

/// Of `picture` (the view's), 60 points about where `p` is seen, three
/// times as large: where a box crosses what is drawn over the view.
fn close_up(
    h: &Harness<'_, Moonglow>,
    area: ResRef,
    picture: &image::RgbaImage,
    p: Vec3,
) -> image::RgbaImage {
    let ppp = h.ctx.pixels_per_point();
    let view = &h.state().area_views[&area];
    let c = (screen(h, area, p) - view.rect.min) * ppp;
    let side = 60.0 * ppp;
    let x = (c.x - side / 2.0).clamp(0.0, picture.width() as f32 - side) as u32;
    let y = (c.y - side / 2.0).clamp(0.0, picture.height() as f32 - side) as u32;
    let part = image::imageops::crop_imm(picture, x, y, side as u32, side as u32).to_image();
    let filter = image::imageops::FilterType::Nearest;
    image::imageops::resize(&part, 3 * side as u32, 3 * side as u32, filter)
}

/// A picture: its name, where the camera looks, from how far, how far
/// down, and the close-ups made of it (each its name and the point).
type Shot<'a> = (&'a str, Vec3, f32, f32, &'a [(&'a str, Vec3)]);

/// The pictures of each group, to look at (before and after a change of
/// how the boxes are drawn): `target/test-output/ui-selection-boxes/`.
#[test]
fn the_boxes_of_each_kind_drawn_in_the_picture() {
    let Some((mut h, area)) = boxes_harness("selection-boxes") else { return };
    let out = mg_testkit::output_dir("ui-selection-boxes");
    let near_trigger = TRIGGER + Vec3::X * 4.0;
    let near_encounter = ENCOUNTER + Vec3::X * 4.0;
    let shots: [Shot<'_>; 6] = [
        ("overview", Vec3::new(20.0, 20.0, 0.0), 50.0, 1.55, &[]),
        (
            "outlines",
            (TRIGGER + ENCOUNTER) / 2.0,
            26.0,
            1.0,
            &[("trigger", near_trigger), ("encounter", near_encounter)],
        ),
        ("light", LIGHT, 10.0, 0.8, &[("light", LIGHT)]),
        (
            "markers",
            (SOUND + WAYPOINT + LOST) / 3.0,
            10.0,
            1.0,
            &[("lost", LOST), ("waypoint", WAYPOINT)],
        ),
        ("door", DOOR, 10.0, 0.9, &[("door", DOOR)]),
        ("placeables", CHESTS + Vec3::X * 3.0, 12.0, 0.8, &[]),
    ];
    for (name, at, distance, pitch, close) in shots {
        look(&mut h, area, at, distance, pitch);
        let picture = view_picture(&mut h, area);
        picture.save(out.join(format!("{name}.png"))).unwrap();
        for (part, p) in close {
            let file = out.join(format!("{name}-{part}-close.png"));
            close_up(&h, area, &picture, *p).save(file).unwrap();
        }
    }
    // At half the resolution (Options › Area › Area view resolution): the
    // boxes are drawn in the picture, stretched with it.
    h.state_mut().settings.area_resolution = Some(50);
    look(&mut h, area, CHESTS + Vec3::X * 3.0, 12.0, 0.8);
    view_picture(&mut h, area).save(out.join("placeables-half.png")).unwrap();
}

/// The box the picture has for object `i` (its place in the model's
/// objects), as the view poses it: its model's box, from the unit cube.
fn copy_of(h: &Harness<'_, Moonglow>, area: ResRef, i: usize) -> glam::Mat4 {
    let view = &h.state().area_views[&area];
    let (min, max) = view.box_of(i).expect("a box");
    let o = view.shown_object(i).expect("the object");
    o.model_transform() * glam::Mat4::from_translation(min) * glam::Mat4::from_scale(max - min)
}

/// Whether the last picture has a box for object `i` where the view has
/// it this frame.
fn drawn_box(h: &Harness<'_, Moonglow>, area: ResRef, i: usize) -> bool {
    let t = copy_of(h, area, i);
    let view = &h.state().area_views[&area];
    view.picture_boxes().iter().any(|c| c.transform.abs_diff_eq(t, 1e-4))
}

/// Where in the view's picture (pixels) a point three quarters up one of
/// object `i`'s box's upright edges is: clear of its handles (on the
/// ground) and of its marker's model.
fn on_edge(h: &Harness<'_, Moonglow>, area: ResRef, i: usize) -> (u32, u32) {
    let p = copy_of(h, area, i).transform_point3(Vec3::new(0.0, 0.0, 0.75));
    let ppp = h.ctx.pixels_per_point();
    let at = (screen(h, area, p) - h.state().area_views[&area].rect.min) * ppp;
    (at.x as u32, at.y as u32)
}

/// How far two pictures differ about `at` (the most of any pixel within
/// two, its channels' differences summed).
fn differ(a: &image::RgbaImage, b: &image::RgbaImage, (x, y): (u32, u32)) -> u32 {
    let mut most = 0;
    for py in y.saturating_sub(2)..=y + 2 {
        for px in x.saturating_sub(2)..=x + 2 {
            let (p, q) = (a.get_pixel(px, py), b.get_pixel(px, py));
            let d = (0..3).map(|c| u32::from(p[c].abs_diff(q[c]))).sum();
            most = most.max(d);
        }
    }
    most
}

/// The boxes are drawn in the area's picture (`Scene::boxes`, as many as
/// the renderer's `Drawn::boxes` counts): one round each object selected
/// that is not an outline (its outline is drawn instead), and a marker's
/// round an object the scene does not draw ([`LOST`]), which alone stays
/// once the selection is let go of. In the picture: an edge of each,
/// clear of the handles, is gone from the picture without it.
#[test]
fn the_boxes_of_the_selection_and_the_markers_are_drawn_in_the_picture() {
    let Some((mut h, area)) = boxes_harness("boxes-drawn") else { return };
    look(&mut h, area, (SOUND + WAYPOINT + LOST) / 3.0, 10.0, 1.0);
    let view = &h.state().area_views[&area];
    let model = view.model.as_ref().unwrap();
    let lost = model.objects.iter().position(|o| o.position.distance(LOST) < 0.1).unwrap();
    let waypoint = model.objects.iter().position(|o| o.kind == ObjectKind::Waypoint).unwrap();
    let boxed = view.selection.iter().filter(|(k, _)| !k.has_outline()).count();
    let count = |h: &Harness<'_, Moonglow>| {
        let drawn = h.state().viewport.as_ref().unwrap().renderer.drawn;
        (h.state().area_views[&area].picture_boxes().len(), drawn.boxes)
    };
    let selected = view_picture(&mut h, area);
    assert_eq!(count(&h), (boxed + 1, boxed + 1), "the selection's and the marker's");
    assert!(drawn_box(&h, area, waypoint) && drawn_box(&h, area, lost), "each where it stands");
    let (waypoint_edge, lost_edge) = (on_edge(&h, area, waypoint), on_edge(&h, area, lost));
    // At rest, the same boxes frame after frame, each a number (they are
    // the picture's key: one unequal to itself would draw it again every
    // frame).
    let boxes = h.state().area_views[&area].picture_boxes().to_vec();
    h.run_steps(4);
    assert_eq!(h.state().area_views[&area].picture_boxes(), &boxes[..], "the same at rest");
    assert!(boxes.iter().all(|c| c.transform.is_finite()), "each a number");
    // Let go of: the marker's alone.
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    settle(&mut h);
    let none = view_picture(&mut h, area);
    assert_eq!(count(&h), (1, 1), "the marker's alone");
    assert!(drawn_box(&h, area, lost) && !drawn_box(&h, area, waypoint));
    assert!(differ(&selected, &none, waypoint_edge) > 60, "the waypoint's box seen, then gone");
    assert!(differ(&selected, &none, lost_edge) < 20, "the marker's box stays as it was");
    // Placeables hidden: the marker's gone too.
    h.state_mut().area_views.get_mut(&area).unwrap().show[ObjectKind::Placeable.index()] = false;
    settle(&mut h);
    let hidden = view_picture(&mut h, area);
    assert_eq!(count(&h), (0, 0), "nothing boxed");
    assert!(differ(&none, &hidden, lost_edge) > 60, "the marker's box seen, then gone");
}

/// In a drag, each frame's picture has the boxes where that frame has the
/// objects it moves: drawn with them, not a frame behind.
#[test]
fn the_boxes_follow_a_drag_in_the_same_frame() {
    let Some((mut h, area)) = boxes_harness("boxes-dragged") else { return };
    let chests: Vec<usize> = {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        let model = view.model.as_ref().unwrap();
        let chests: Vec<usize> = (model.objects.iter().enumerate())
            .filter(|(_, o)| o.kind == ObjectKind::Placeable && o.position.y > CHESTS.y - 0.5)
            .map(|(i, _)| i)
            .collect();
        view.selection =
            chests.iter().map(|&i| (model.objects[i].kind, model.objects[i].index)).collect();
        chests
    };
    assert_eq!(chests.len(), 4, "the row of chests");
    look(&mut h, area, CHESTS + Vec3::X * 3.0, 12.0, 0.8);
    let first = chests[0];
    let grip = h.state().area_views[&area].move_grip(first).expect("a grip");
    h.hover_at(grip);
    h.run_steps(1);
    press(&h, grip, true, egui::Modifiers::NONE);
    h.run_steps(1);
    // (The drag begins with the first move, posing them from the next.)
    h.hover_at(grip + egui::vec2(12.0, 0.0));
    h.run_steps(1);
    let mut was = h.state().area_views[&area].shown_object(first).unwrap().position;
    for k in 2..=6 {
        let pictures = h.state().area_views[&area].pictures;
        h.hover_at(grip + egui::vec2(12.0 * k as f32, 0.0));
        h.run_steps(1);
        let view = &h.state().area_views[&area];
        assert!(view.pictures > pictures, "step {k}: the picture drawn in its frame");
        let at = view.shown_object(first).unwrap().position;
        assert!(at.distance(was) > 0.01, "step {k}: moved ({was} to {at})");
        was = at;
        for &i in &chests {
            assert!(drawn_box(&h, area, i), "step {k}: object {i}'s box where it is");
        }
    }
    press(&h, grip + egui::vec2(72.0, 0.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    let undo = h.state().ws.as_ref().and_then(|w| w.can_undo().map(str::to_owned));
    assert_eq!(undo.as_deref(), Some("Move"), "the drag moved them");
    settle(&mut h);
    for &i in &chests {
        assert!(drawn_box(&h, area, i), "object {i}'s box where it was put");
    }
}
