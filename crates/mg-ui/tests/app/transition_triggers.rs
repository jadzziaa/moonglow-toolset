//! Area transition triggers the game shows and lets be clicked (a
//! builder's report: transitions linked by hand were invisible in the game
//! until their polygons were redrawn). The client draws one under the
//! pointer, and takes a click on it, only when its points run
//! counterclockwise and stand on the ground (`ZPosition` + `PointZ`; read
//! in `client_render/transition_outlines.rs`). So a trigger drawn, an Alt
//! rectangle and Redraw Polygon are saved in Aurora's order (counterclockwise
//! from the point of smallest X, the trigger standing at the first click),
//! Redraw Polygon puts the trigger at height 0, and a trigger moved (a
//! drag, the arrows, Adjust Location, a paste here or in another area)
//! goes back on the ground under its points; Trigger Type made Area
//! Transition gives it the transition cursor, as Aurora's does. Through the
//! real UI paths, each step undone and redone whole.
//!
//! The investigation's own tests (the reporter's sequence, saved to
//! `target/transition-triggers/` as the modules the client was shown) are
//! gone: run now, they would write fixed outlines under the names of the
//! broken ones. `the_module_for_the_after_picture` (ignored, by hand)
//! writes the module the client is to be shown with the fix in.

use crate::common::*;
use egui::accesskit::Role;
use glam::Vec3;
use mg_area::ObjectKind;
use mg_module::instances::{OUTLINE_LIFT, signed_area, trigger_xy};
use std::path::PathBuf;

/// A square's corners, anticlockwise (seen from above, x east, y north)
/// from its south-west corner.
pub(crate) fn square(x: f32, y: f32, side: f32) -> Vec<Vec3> {
    [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)]
        .iter()
        .map(|(dx, dy)| Vec3::new(x + dx, y + dy, 0.0))
        .collect()
}

fn click(h: &mut Harness<'_, Moonglow>, area: ResRef, p: Vec3) {
    let at = screen(h, area, p);
    h.hover_at(at);
    press(h, at, true, egui::Modifiers::NONE);
    press(h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
}

/// Clicks `corners` in the area's view, the last one twice (a double
/// click closes the outline), as a builder draws it.
fn outline(h: &mut Harness<'_, Moonglow>, area: ResRef, corners: &[Vec3]) {
    let (last, rest) = corners.split_last().unwrap();
    for p in rest {
        click(h, area, *p);
    }
    let at = screen(h, area, *last);
    h.hover_at(at);
    for _ in 0..2 {
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
    }
    h.run_steps(3);
    settle(h);
}

pub(crate) fn triggers(h: &mut Harness<'_, Moonglow>, area: ResRef) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    git.root.list("TriggerList").unwrap_or(&[]).to_vec()
}

pub(crate) fn show_area(h: &mut Harness<'_, Moonglow>, area: ResRef) {
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    h.run_steps(4);
    settle(h);
    h.run_steps(4);
}

/// Draws a trigger of blueprint `bp` along `corners` in `area`; its index.
pub(crate) fn draw(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    bp: &str,
    corners: &[Vec3],
) -> usize {
    show_area(h, area);
    let before = triggers(h, area).len();
    h.state_mut().palette.selected = ResKey::parse(bp, ResType::UTT);
    h.run_steps(2);
    outline(h, area, corners);
    let after = triggers(h, area).len();
    assert_eq!(after, before + 1, "{bp} drawn in {area}");
    before
}

/// A rectangle drawn with Alt: a click at `from`, Alt and a click at
/// `to`; its index.
fn rectangle(h: &mut Harness<'_, Moonglow>, area: ResRef, bp: &str, from: Vec3, to: Vec3) -> usize {
    let before = triggers(h, area).len();
    h.state_mut().palette.selected = ResKey::parse(bp, ResType::UTT);
    h.run_steps(2);
    click(h, area, from);
    let at = screen(h, area, to);
    h.hover_at(at);
    press(h, at, true, egui::Modifiers::ALT);
    press(h, at, false, egui::Modifiers::ALT);
    h.run_steps(3);
    settle(h);
    assert_eq!(triggers(h, area).len(), before + 1, "rectangle placed");
    before
}

/// Right-clicks inside trigger `index` (selected first) and clicks the
/// menu's item `item`.
fn menu_item(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize, inside: Vec3, item: &str) {
    show_area(h, area);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Trigger, index)];
    h.run_steps(2);
    let at = screen(h, area, inside);
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
    h.get_by_label(item).click();
    h.run_steps(2);
}

/// Redraw Polygon from the placed trigger's menu, then `corners`.
fn redraw(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    index: usize,
    inside: Vec3,
    corners: &[Vec3],
) {
    menu_item(h, area, index, inside, "Redraw Polygon");
    assert_eq!(h.state().area_views[&area].redraw, Some((ObjectKind::Trigger, index)));
    outline(h, area, corners);
    assert!(h.state().area_views[&area].redraw.is_none(), "closed");
}

pub(crate) fn instance_tab(area: ResRef, list: &str, index: usize) -> Tab {
    Tab::Instance { area, path: mg_edit::GffPath::root().item(list, index) }
}

/// The text field on the row of the label `label` (the first to its right).
fn field_beside<'h>(h: &'h Harness<'_, Moonglow>, label: &str) -> egui_kittest::Node<'h> {
    let row = h.get_all_by_label(label).last().unwrap().rect();
    h.get_all_by_role(Role::TextInput)
        .filter(|n| {
            (n.rect().center().y - row.center().y).abs() < 8.0 && n.rect().left() > row.left()
        })
        .min_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .unwrap_or_else(|| panic!("no field beside {label}"))
}

/// Types `text` into the field beside `label`, replacing it; Tab commits.
fn type_beside(h: &mut Harness<'_, Moonglow>, label: &str, text: &str) {
    field_beside(h, label).click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    field_beside(h, label).type_text(text);
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
    settle(h);
}

/// Gives a placed waypoint its tag in its Properties.
fn tag_waypoint(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize, tag: &str) {
    let tab = instance_tab(area, "WaypointList", index);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(6);
    type_beside(h, "Tag", tag);
    h.state_mut().actions.push(mg_ui::Action::CloseTab(tab));
    h.run_steps(3);
}

/// In the placed trigger's Properties: Trigger Type Area Transition when
/// `retype`, then the Area Transition page's Destination Tag typed and
/// Destination is a Waypoint.
fn link(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize, to: &str, retype: bool) {
    let tab = instance_tab(area, "TriggerList", index);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(6);
    if retype {
        retype_to(h, "Generic", "Area Transition");
        assert_eq!(triggers(h, area)[index].integer("Type"), Some(1), "retyped");
    }
    h.get_all_by_label("Area Transition").next().unwrap().click();
    h.run_steps(3);
    type_beside(h, "Destination Tag", to);
    let radio = h
        .get_all_by_label("Waypoint")
        .find(|n| n.accesskit_node().role() == Role::RadioButton)
        .expect("Destination is a Waypoint");
    radio.click();
    h.run_steps(3);
    settle(h);
    let t = &triggers(h, area)[index];
    assert_eq!(t.string("LinkedTo"), Some(to.as_bytes()), "linked");
    assert_eq!(t.integer("LinkedToFlags"), Some(2));
    h.state_mut().actions.push(mg_ui::Action::CloseTab(tab));
    h.run_steps(3);
}

/// In an open trigger's Properties (Basic), Trigger Type from `from` to
/// `to`.
fn retype_to(h: &mut Harness<'_, Moonglow>, from: &str, to: &str) {
    h.get_by_value(from).click();
    h.run_steps(2);
    h.get_all_by_label(to).last().unwrap().click();
    h.run_steps(3);
    settle(h);
}

/// The module of two areas: the harness's (two waypoints) and a second.
pub(crate) fn harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef, ResRef)> {
    two_areas(name, &[], &[])
}

/// [`harness`] with tiles `raised` of the first area and `raised_second`
/// of the second a height step up (tile `i` at column `i % 4`, row
/// `i / 4`: 10 m squares).
fn two_areas(
    name: &str,
    raised: &'static [usize],
    raised_second: &'static [usize],
) -> Option<(Harness<'static, Moonglow>, ResRef, ResRef)> {
    let raise = |m: &mut Module, area: ResRef, tiles: &[usize]| {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        for &i in tiles {
            are.root.list_mut("Tile_List").unwrap()[i].set("Tile_Height", mg_gff::Value::Int(1));
        }
        m.set_gff(key, &are).unwrap();
    };
    let second = move |m: &mut Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::new::{AreaSpec, add_area};
        let mut rng = fastrand::Rng::with_seed(11);
        let spec = AreaSpec {
            name: "Second".into(),
            tileset: ResRef::from_str("ttr01").unwrap(),
            width: 4,
            height: 4,
        };
        let other = add_area(m, game, &spec, &mut rng).unwrap();
        raise(m, area, raised);
        raise(m, other, raised_second);
    };
    let (h, area) = area_harness_with(name, "ttr01", None, 4, None, &second)?;
    let other = h.state().ws.as_ref()?.module.areas().ok()?.into_iter().find(|a| *a != area)?;
    Some((h, area, other))
}

/// Trigger `t`'s points, each where the game takes it: its position plus
/// the point (X and Y), and `ZPosition` + `PointZ`.
fn points(t: &mg_gff::Struct) -> Vec<Vec3> {
    let f = |s: &mg_gff::Struct, l: &str| s.float(l).unwrap_or(0.0);
    let at = Vec3::new(f(t, "XPosition"), f(t, "YPosition"), f(t, "ZPosition"));
    let geometry = t.list("Geometry").unwrap_or(&[]);
    geometry
        .iter()
        .map(|p| at + Vec3::new(f(p, "PointX"), f(p, "PointY"), f(p, "PointZ")))
        .collect()
}

/// How far each of trigger `t`'s points is above where Aurora puts it,
/// 0.025 above the ground of `area` under it (the area view's ground).
fn off_ground(h: &Harness<'_, Moonglow>, area: ResRef, t: &mg_gff::Struct) -> Vec<f32> {
    let app = h.state();
    let model = app.area_views[&area].model.as_ref().expect("the area read");
    let ground = mg_area::walk::Ground::new(app.game.as_deref().unwrap(), model);
    points(t)
        .iter()
        .map(|p| p.z - ground.height(p.truncate(), p.z).expect("ground under it") - OUTLINE_LIFT)
        .collect()
}

/// Asserts trigger `t`'s points all on the ground of `area`.
fn on_ground(h: &Harness<'_, Moonglow>, area: ResRef, t: &mg_gff::Struct, what: &str) {
    let off = off_ground(h, area, t);
    assert!(off.iter().all(|d| d.abs() < 1e-3), "{what}: off the ground by {off:?}");
}

/// Asserts trigger `t` saved in Aurora's order: counterclockwise from its
/// point of smallest X (then Y), standing at `first` (the first click).
fn in_aurora_s_order(t: &mg_gff::Struct, first: Vec3, what: &str) {
    let xy = trigger_xy(t.list("Geometry").unwrap());
    assert!(signed_area(&xy) > 0.0, "{what}: counterclockwise {xy:?}");
    let least = xy
        .iter()
        .copied()
        .fold([f32::MAX, f32::MAX], |a, b| if (b[0], b[1]) < (a[0], a[1]) { b } else { a });
    assert_eq!(xy[0], least, "{what}: from the point of smallest X {xy:?}");
    let at = (t.float("XPosition").unwrap(), t.float("YPosition").unwrap());
    assert!((at.0 - first.x).abs() < 0.1 && (at.1 - first.y).abs() < 0.1, "{what}: at {at:?}");
    assert!(xy.iter().any(|p| p[0] == 0.0 && p[1] == 0.0), "{what}: the first click among them");
}

/// Undo and redo of the last step: what undo leaves is `before`, what
/// redo leaves is `after` (trigger `index` of `area`).
fn undo_redo(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    index: usize,
    before: &mg_gff::Struct,
    after: &mg_gff::Struct,
) {
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(h);
    assert_eq!(&triggers(h, area)[index], before, "undone whole");
    h.state_mut().actions.push(mg_ui::Action::Redo);
    settle(h);
    assert_eq!(&triggers(h, area)[index], after, "redone whole");
}

/// Clicked clockwise, a trigger is saved counterclockwise in Aurora's
/// order, standing at the first click; so is an Alt rectangle dragged
/// each of the four ways; undone, it goes, and redone it comes back as
/// it was.
#[test]
fn a_trigger_drawn_clockwise_is_saved_in_aurora_s_order() {
    let Some((mut h, field, _)) = harness("tt-order") else { return };
    // P1 (8, 8), P2 (9, 13), P3 (14, 12), P4 (13, 8): clockwise, as
    // Aurora's own probe was drawn (saved P1 P4 P3 P2).
    let clicks: Vec<Vec3> = [(8.0, 8.0), (9.0, 13.0), (14.0, 12.0), (13.0, 8.0)]
        .map(|(x, y)| Vec3::new(x, y, 0.0))
        .into();
    let i = draw(&mut h, field, "newtransition", &clicks);
    let t = triggers(&mut h, field)[i].clone();
    in_aurora_s_order(&t, clicks[0], "clicked clockwise");
    let saved: Vec<Vec3> = points(&t);
    for (k, p) in [0, 3, 2, 1].into_iter().enumerate() {
        assert!(saved[k].truncate().distance(clicks[p].truncate()) < 0.1, "{saved:?}");
    }
    on_ground(&h, field, &t, "drawn");
    // Undone and redone: gone, then back as it was.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert_eq!(triggers(&mut h, field).len(), i);
    h.state_mut().actions.push(mg_ui::Action::Redo);
    settle(&mut h);
    assert_eq!(triggers(&mut h, field)[i], t);
    // Alt rectangles, dragged north-east, north-west, south-east and
    // south-west.
    for (what, from, to) in [
        ("north-east", (20.0, 6.0), (26.0, 10.0)),
        ("north-west", (34.0, 6.0), (28.0, 10.0)),
        ("south-east", (20.0, 20.0), (26.0, 16.0)),
        ("south-west", (34.0, 20.0), (28.0, 16.0)),
    ] {
        let from = Vec3::new(from.0, from.1, 0.0);
        let r = rectangle(&mut h, field, "newtransition", from, Vec3::new(to.0, to.1, 0.0));
        let t = triggers(&mut h, field)[r].clone();
        in_aurora_s_order(&t, from, what);
        assert_eq!(t.list("Geometry").unwrap().len(), 4);
        on_ground(&h, field, &t, what);
    }
}

/// Redraw Polygon clockwise or counterclockwise saves Aurora's order; on
/// a trigger at another height (`ZPosition` 2: lifted, or pasted by
/// Aurora) it puts the trigger at height 0 with its points on the ground,
/// in one step that undoes whole.
#[test]
fn redraw_polygon_saves_aurora_s_order_and_puts_the_trigger_down() {
    let Some((mut h, field, _)) = harness("tt-redraw") else { return };
    let corners = square(8.0, 8.0, 6.0);
    let i = draw(&mut h, field, "newtransition", &corners);
    let inside = Vec3::new(11.0, 11.0, 0.0);
    for (what, ccw) in [("clockwise", false), ("counterclockwise", true)] {
        let mut c = square(8.0, 8.0, 6.0);
        if !ccw {
            c.reverse();
        }
        redraw(&mut h, field, i, inside, &c);
        let t = triggers(&mut h, field)[i].clone();
        in_aurora_s_order(&t, corners[0], what);
        on_ground(&h, field, &t, what);
    }
    // Lifted 2 m, its points kept: 2 m off the ground.
    let edit = mg_edit::Edit::SetField {
        key: ResKey::new(field, ResType::GIT),
        path: mg_edit::GffPath::root().item("TriggerList", i),
        label: "ZPosition".into(),
        value: Some(mg_gff::Value::Float(2.0)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Lift", vec![edit])));
    settle(&mut h);
    let lifted = triggers(&mut h, field)[i].clone();
    assert!(off_ground(&h, field, &lifted).iter().all(|d| (d - 2.0).abs() < 1e-3));
    let mut c = square(8.0, 8.0, 6.0);
    c.reverse();
    redraw(&mut h, field, i, Vec3::new(11.0, 11.0, 2.0), &c);
    let t = triggers(&mut h, field)[i].clone();
    assert_eq!(t.float("ZPosition"), Some(0.0), "at height 0, as Aurora's Redraw Polygon puts it");
    in_aurora_s_order(&t, corners[0], "redrawn lifted");
    on_ground(&h, field, &t, "redrawn lifted");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Redraw Polygon"));
    undo_redo(&mut h, field, i, &lifted, &t);
}

/// A trigger moved onto ground of another height (tile 1 of the area,
/// 10 to 20 m east, 0 to 10 north, a height step up), or off it onto
/// the second area's flat ground, keeps every point on the ground (`ZPosition` + `PointZ` 0.025
/// above it): dragged, by the X arrow, by Adjust Location's X and Y,
/// pasted in the area and into the other; each a step that undoes and
/// redoes whole.
#[test]
fn a_trigger_moved_stays_on_the_ground() {
    let Some((mut h, field, second)) = two_areas("tt-moved", &[1], &[]) else { return };
    let raised = h.state().area_views[&field].model.as_ref().unwrap().tiles[1].position.z;
    assert!(raised > 0.5, "tile 1 is raised: {raised}");
    // A 3 m square on flat ground, at (2, 2), and its copy's place.
    let i = draw(&mut h, field, "newtransition", &square(2.0, 2.0, 3.0));
    let t0 = triggers(&mut h, field)[i].clone();
    on_ground(&h, field, &t0, "drawn");
    h.state_mut().area_views.get_mut(&field).unwrap().selection = vec![(ObjectKind::Trigger, i)];
    h.run_steps(2);

    // Dragged with the mouse from inside it 10 m east, onto the raised
    // tile.
    let (from, to) = (Vec3::new(3.5, 3.5, 0.0), Vec3::new(13.5, 3.5, raised));
    let (a, b) = (screen(&h, field, from), screen(&h, field, to));
    h.hover_at(a);
    h.run_steps(1);
    press(&h, a, true, egui::Modifiers::NONE);
    for k in 1..=6 {
        h.hover_at(a + (b - a) * (k as f32 / 6.0));
        h.step();
    }
    press(&h, b, false, egui::Modifiers::NONE);
    h.run_steps(3);
    settle(&mut h);
    let dragged = triggers(&mut h, field)[i].clone();
    let x = dragged.float("XPosition").unwrap();
    assert!(x > 11.0, "dragged east: {x}");
    assert_eq!(dragged.float("ZPosition"), Some(0.0), "its height kept, as Aurora's drag");
    on_ground(&h, field, &dragged, "dragged");
    undo_redo(&mut h, field, i, &t0, &dragged);

    // Back west by the X arrow (Shift held), 8 m.
    let shift = egui::Modifiers::SHIFT;
    let left = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: shift,
    };
    // (The arrows stand at where the trigger stands, its first click.)
    let f = |l: &str| dragged.float(l).unwrap();
    let stands = Vec3::new(f("XPosition"), f("YPosition"), f("ZPosition"));
    let radius = h.state().area_views[&field].handle_radius_at(stands).unwrap();
    h.event(egui::Event::ModifiersChanged(shift));
    let head = stands + Vec3::X * radius * 1.45;
    let mut at = screen(&h, field, head);
    h.hover_at(at);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&field].arrow_axis_at(at), Some(0), "on the X arrow");
    h.event(left(at, true));
    for k in 1..=6 {
        at = screen(&h, field, head - Vec3::X * (8.0 * k as f32 / 6.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    h.event(left(at, false));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    settle(&mut h);
    let arrowed = triggers(&mut h, field)[i].clone();
    let x = arrowed.float("XPosition").unwrap();
    assert!(x < 10.0, "led west: {x}");
    assert_eq!(arrowed.float("YPosition"), dragged.float("YPosition"), "along X alone");
    on_ground(&h, field, &arrowed, "by the arrow");
    undo_redo(&mut h, field, i, &dragged, &arrowed);

    // Adjust Location: X and Y, onto the raised tile again.
    menu_item(&mut h, field, i, points(&arrowed)[0] + Vec3::new(1.5, 1.5, 0.0), "Adjust Location…");
    {
        let adjust = h.state_mut().adjust.as_mut().expect("the window");
        assert_eq!(adjust.objects, [(ObjectKind::Trigger, i)]);
        adjust.position.x = 14.0;
        adjust.position.y = 4.0;
        adjust.changed = [true, true, false, false];
    }
    h.get_by_label("OK").click();
    h.run_steps(3);
    settle(&mut h);
    let adjusted = triggers(&mut h, field)[i].clone();
    assert_eq!(adjusted.float("XPosition"), Some(14.0));
    on_ground(&h, field, &adjusted, "by Adjust Location");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Adjust Location"));
    undo_redo(&mut h, field, i, &arrowed, &adjusted);

    // Copied, and pasted on flat ground in this area, then on the second
    // area's raised tile 0.
    let paste = |h: &mut Harness<'_, Moonglow>, area: ResRef, at: Vec3| -> mg_gff::Struct {
        show_area(h, area);
        let count = triggers(h, area).len();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
        h.run_steps(2);
        let over = screen(h, area, at);
        h.hover_at(over);
        h.run_steps(1);
        let over = screen(h, area, at);
        h.hover_at(over);
        press(h, over, true, egui::Modifiers::NONE);
        press(h, over, false, egui::Modifiers::NONE);
        h.run_steps(3);
        settle(h);
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"));
        let all = triggers(h, area);
        assert_eq!(all.len(), count + 1, "pasted");
        all[count].clone()
    };
    show_area(&mut h, field);
    h.state_mut().area_views.get_mut(&field).unwrap().selection = vec![(ObjectKind::Trigger, i)];
    let over = screen(&h, field, Vec3::new(15.0, 5.0, raised));
    h.hover_at(over);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    let here = paste(&mut h, field, Vec3::new(4.0, 24.0, 0.0));
    assert!((here.float("YPosition").unwrap() - 24.0).abs() < 0.5, "pasted where clicked");
    on_ground(&h, field, &here, "pasted here");
    // (From the raised tile onto the second area's flat ground: as it
    // was, it would be a step up in the air.)
    let there = paste(&mut h, second, Vec3::new(24.0, 24.0, 0.0));
    on_ground(&h, second, &there, "pasted into the second area");
    let mut unchanged = adjusted.clone();
    for l in ["XPosition", "YPosition"] {
        unchanged.set(l, there.get(l).unwrap().clone());
    }
    let off = off_ground(&h, second, &unchanged);
    assert!(off.iter().all(|d| *d > 0.5), "unchanged, it would be off: {off:?}");
    let pasted = triggers(&mut h, second).len() - 1;
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert_eq!(triggers(&mut h, second).len(), pasted, "the paste undone");
    h.state_mut().actions.push(mg_ui::Action::Redo);
    settle(&mut h);
    assert_eq!(triggers(&mut h, second)[pasted], there, "and redone");
}

/// Trigger Type made Area Transition gives a trigger the transition
/// cursor (`Cursor` 1) where it had none, in the same step (Aurora's
/// change, captured); made Generic again, only the type changes.
#[test]
fn trigger_type_area_transition_takes_the_transition_cursor() {
    let Some((mut h, field, _)) = harness("tt-type") else { return };
    let i = draw(&mut h, field, "newgeneric", &square(8.0, 8.0, 4.0));
    let before = triggers(&mut h, field)[i].clone();
    assert_eq!((before.integer("Type"), before.integer("Cursor")), (Some(0), Some(0)));
    let tab = instance_tab(field, "TriggerList", i);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(6);
    settle(&mut h);
    retype_to(&mut h, "Generic", "Area Transition");
    let after = triggers(&mut h, field)[i].clone();
    assert_eq!((after.integer("Type"), after.integer("Cursor")), (Some(1), Some(1)));
    assert_eq!(after.get("Cursor").map(mg_gff::Value::field_type), Some(mg_gff::FieldType::Byte));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Trigger type"));
    undo_redo(&mut h, field, i, &before, &after);
    // Back to Generic: the cursor stays.
    retype_to(&mut h, "Area Transition", "Generic");
    let generic = triggers(&mut h, field)[i].clone();
    assert_eq!((generic.integer("Type"), generic.integer("Cursor")), (Some(0), Some(1)));
    h.state_mut().actions.push(mg_ui::Action::CloseTab(tab));
    h.run_steps(3);
}

/// The module for the client's "after" picture, by hand: with the fix in,
/// in a flat 4 × 4 `ttr01` area, a transition drawn clockwise by clicks
/// (TT_CW, at (8, 8)), an Alt rectangle dragged south-east (TT_SE, from
/// (20, 30) to (26, 24)), both linked to the waypoint `WP_AFTER` in the
/// second area, and a third (TT_MOVED, drawn at (8, 24), dragged 10 m
/// east) linked to it too; saved to
/// `target/transition-triggers/after/after-fix.mod`, with `after-fix.txt`
/// saying what each trigger holds.
/// `cargo test -p mg-ui --test app transition_triggers::the_module -- --ignored`
#[test]
#[ignore = "makes the module the client is shown, by hand"]
fn the_module_for_the_after_picture() {
    let Some((mut h, field, second)) = harness("tt-after") else { return };
    let tag = |h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize, text: &str| {
        let tab = instance_tab(area, "TriggerList", index);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
        h.run_steps(6);
        type_beside(h, "Tag", text);
        h.state_mut().actions.push(mg_ui::Action::CloseTab(tab));
        h.run_steps(3);
    };
    let mut clockwise = square(8.0, 8.0, 6.0);
    clockwise.reverse();
    let cw = draw(&mut h, field, "newtransition", &clockwise);
    let (from, to) = (Vec3::new(20.0, 30.0, 0.0), Vec3::new(26.0, 24.0, 0.0));
    let se = rectangle(&mut h, field, "newtransition", from, to);
    let moved = draw(&mut h, field, "newtransition", &square(8.0, 24.0, 4.0));
    h.state_mut().area_views.get_mut(&field).unwrap().selection =
        vec![(ObjectKind::Trigger, moved)];
    h.run_steps(2);
    let (a, b) = (
        screen(&h, field, Vec3::new(10.0, 26.0, 0.0)),
        screen(&h, field, Vec3::new(20.0, 26.0, 0.0)),
    );
    h.hover_at(a);
    h.run_steps(1);
    press(&h, a, true, egui::Modifiers::NONE);
    for k in 1..=6 {
        h.hover_at(a + (b - a) * (k as f32 / 6.0));
        h.step();
    }
    press(&h, b, false, egui::Modifiers::NONE);
    h.run_steps(3);
    settle(&mut h);
    assert!(triggers(&mut h, field)[moved].float("XPosition").unwrap() > 15.0, "dragged");
    show_area(&mut h, second);
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    h.run_steps(2);
    click(&mut h, second, Vec3::new(20.0, 10.0, 0.0));
    settle(&mut h);
    tag_waypoint(&mut h, second, 0, "WP_AFTER");
    for (index, name) in [(cw, "TT_CW"), (se, "TT_SE"), (moved, "TT_MOVED")] {
        tag(&mut h, field, index, name);
        link(&mut h, field, index, "WP_AFTER", false);
        let t = triggers(&mut h, field)[index].clone();
        let xy = trigger_xy(t.list("Geometry").unwrap());
        assert!(signed_area(&xy) > 0.0, "{name} counterclockwise");
        on_ground(&h, field, &t, name);
    }
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(4);
    settle(&mut h);
    let ws = h.state().ws.as_ref().unwrap();
    assert!(!ws.is_modified(), "saved");
    let saved = ws.module.location.as_ref().unwrap().path().to_path_buf();
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/transition-triggers/after");
    std::fs::create_dir_all(&dir).unwrap();
    let to = dir.join("after-fix.mod");
    std::fs::copy(&saved, &to).unwrap();
    // What each trigger holds, read back from the file.
    let m = Module::open(&to).unwrap();
    let git = m.gff(&ResKey::new(field, ResType::GIT)).unwrap().unwrap();
    let mut said = format!("# {}\n", to.display());
    for t in git.root.list("TriggerList").unwrap() {
        let s = |l: &str| String::from_utf8_lossy(t.string(l).unwrap_or_default()).into_owned();
        let xy = trigger_xy(t.list("Geometry").unwrap());
        said.push_str(&format!(
            "{} linked to {} (flags {:?}), Type {:?}, Cursor {:?}, at ({:.3}, {:.3}, {:.3}), \
             signed area {:.3}\n",
            s("Tag"),
            s("LinkedTo"),
            t.integer("LinkedToFlags"),
            t.integer("Type"),
            t.integer("Cursor"),
            t.float("XPosition").unwrap_or(0.0),
            t.float("YPosition").unwrap_or(0.0),
            t.float("ZPosition").unwrap_or(0.0),
            signed_area(&xy),
        ));
        for p in t.list("Geometry").unwrap() {
            let f = |l: &str| p.float(l).unwrap_or(f32::NAN);
            said.push_str(&format!("  {:.4} {:.4} {:.4}\n", f("PointX"), f("PointY"), f("PointZ")));
        }
    }
    std::fs::write(dir.join("after-fix.txt"), said).unwrap();
}
