//! Doors placed anywhere: on a door hook within reach (snapped, facing as
//! it does), else freely where the pointer is, turned by Q and E; with
//! Alt held, freely near a hook too. (Aurora places doors on hooks alone:
//! a divergence the parity checklist records.)

use crate::common::*;
use glam::Vec3;
use mg_area::ObjectKind;

/// Far from every door hook of the castle tile's area.
const FREE: Vec3 = Vec3::new(25.5, 24.5, 0.0);
/// Near the castle tile's east hook (at 20, 15, facing 270°), inside the
/// tile.
const NEAR_HOOK: Vec3 = Vec3::new(18.5, 15.5, 0.0);

/// The area's doors, as its GIT has them now.
fn doors(h: &mut Harness<'_, Moonglow>, area: ResRef) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    git.root.list("Door List").unwrap_or(&[]).to_vec()
}

/// A click at ground point `p` with `modifiers` held (sent before the
/// press, which `common::press` would drop again).
fn click(h: &mut Harness<'_, Moonglow>, area: ResRef, p: Vec3, modifiers: egui::Modifiers) {
    let at = screen(h, area, p);
    h.event(egui::Event::ModifiersChanged(modifiers));
    h.hover_at(at);
    h.run_steps(2);
    for pressed in [true, false] {
        let button = egui::PointerButton::Primary;
        h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
    }
    h.run_steps(3);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(1);
}

/// The castle tile's area (tic01, tile 7 at column 1, row 1: three door
/// hooks), seen from the south, a little above its middle.
fn door_harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let (mut h, area) = area_harness_on(name, "tic01", Some(7))?;
    {
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        o.target = Vec3::new(22.0, 20.0, 0.0);
        o.distance = 26.0;
        o.pitch = 0.95;
    }
    h.run_steps(2);
    Some((h, area))
}

/// Steps until the Status Bar says something that begins with `start`.
fn bar_says(h: &mut Harness<'_, Moonglow>, start: &str) {
    let started = std::time::Instant::now();
    while !h.state().status_said().iter().any(|s| s.starts_with(start)) {
        let said = h.state().status_said().to_vec();
        assert!(started.elapsed().as_secs() < 10, "never said {start}: {said:?}");
        h.step();
    }
}

/// The area's view alone, written to `target/test-output/free-doors/`
/// with `MG_SHOT` set (to look at, before and after a change).
fn picture(h: &mut Harness<'_, Moonglow>, area: ResRef, name: &str) {
    if std::env::var_os("MG_SHOT").is_none() {
        return;
    }
    settle(h);
    let rect = h.state().area_views[&area].rect;
    let Ok(img) = h.render() else { return };
    let ppp = h.ctx.pixels_per_point();
    let out = mg_testkit::output_dir("free-doors");
    crop_rect(&img, rect, ppp, 0.0).save(out.join(name)).unwrap();
}

/// A door chosen in the palette, clicked far from every hook, is placed
/// there freely, turned by Q as its ghost was; near a hook it still goes
/// on the hook, facing as it does; with Alt held near a hook, freely
/// where clicked, turned by Q and E, not by the hook.
#[test]
fn doors_go_on_hooks_in_reach_and_freely_elsewhere() {
    let Some((mut h, area)) = door_harness("free-doors") else { return };
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    assert!(model.hook_near(FREE, 4.0).is_none() && model.hook_near(NEAR_HOOK, 4.0).is_some());
    h.state_mut().palette.selected = ResKey::parse("nw_door_normal", ResType::UTD);
    h.hover_at(screen(&h, area, FREE));
    h.run_steps(3);
    // The ghost follows the pointer, and Q turns it (a free door turns).
    let ghost = h.state().area_views[&area].ghost_shown.clone().expect("a ghost off the hooks");
    assert!((ghost.position - FREE).truncate().length() < 0.6, "{:?}", ghost.position);
    h.key_press(egui::Key::Q);
    h.run_steps(2);
    let turn = h.state().area_views[&area].ghost_shown.as_ref().unwrap().rotation;
    assert!(turn.abs() > 0.1, "Q turned the free door's ghost: {turn}");
    bar_says(&mut h, "Free: no door hook in reach");
    picture(&mut h, area, "ghost_free.png");
    // Shift: placed, and the brush kept for the next ones.
    click(&mut h, area, FREE, egui::Modifiers::SHIFT);
    let placed = doors(&mut h, area);
    assert_eq!(placed.len(), 1, "placed off the hooks");
    let d = &placed[0];
    let at = Vec3::new(d.float("X").unwrap(), d.float("Y").unwrap(), d.float("Z").unwrap());
    assert!((at - FREE).truncate().length() < 0.6, "where clicked: {at:?}");
    assert!((d.float("Bearing").unwrap() - turn).abs() < 1e-4, "turned by Q");
    assert!(
        !h.state().log.entries.iter().any(|(_, m)| m.contains("door hooks")),
        "no warning: {:?}",
        h.state().log.entries
    );
    picture(&mut h, area, "placed_free.png");
    // Near the east hook: on the hook, turned as it is (Q's turn left out).
    h.hover_at(screen(&h, area, NEAR_HOOK));
    bar_says(&mut h, "On a door hook");
    picture(&mut h, area, "ghost_on_hook.png");
    click(&mut h, area, NEAR_HOOK, egui::Modifiers::SHIFT);
    let placed = doors(&mut h, area);
    assert_eq!(placed.len(), 2);
    let d = &placed[1];
    assert_eq!((d.float("X"), d.float("Y"), d.float("Z")), (Some(20.0), Some(15.0), Some(0.0)));
    assert!((d.float("Bearing").unwrap() + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    // With Alt near the hook: the ghost is free, and so is the door.
    // (Nothing selected, as most often: an Alt press is then the camera's
    // where it drags, not where it clicks.)
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::ALT));
    h.hover_at(screen(&h, area, NEAR_HOOK));
    h.run_steps(3);
    let ghost = h.state().area_views[&area].ghost_shown.clone().expect("a ghost with Alt");
    assert!((ghost.position - NEAR_HOOK).truncate().length() < 0.6, "{:?}", ghost.position);
    bar_says(&mut h, "Free: Alt keeps it off the door hook");
    picture(&mut h, area, "ghost_alt_near_hook.png");
    click(&mut h, area, NEAR_HOOK, egui::Modifiers::ALT);
    let placed = doors(&mut h, area);
    assert_eq!(placed.len(), 3);
    let d = &placed[2];
    let at = Vec3::new(d.float("X").unwrap(), d.float("Y").unwrap(), 0.0);
    assert!((at - NEAR_HOOK).truncate().length() < 0.6, "where clicked: {at:?}");
    assert!((d.float("Bearing").unwrap() - turn).abs() < 1e-4, "turned by Q, not the hook");
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Door, 2)]);
}

/// A press on the selection's door at screen point `from`, led to `to`
/// and let go, with Alt pressed once the drag has begun (Alt held at the
/// press raises the selection instead) when `alt`.
fn drag_door(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2, alt: bool) {
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(1);
    let modifiers = if alt { egui::Modifiers::ALT } else { egui::Modifiers::NONE };
    for step in 1..=8 {
        if step == 4 && alt {
            let moving = h.state().area_views.values().any(|v| v.door_drag_hooked().is_some());
            assert!(moving, "a door's move under way before Alt");
            h.event(egui::Event::ModifiersChanged(modifiers));
        }
        h.hover_at(from + (to - from) * (step as f32 / 8.0));
        h.run_steps(1);
    }
    h.drop_at(to);
    h.run_steps(3);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(1);
}

/// Where the area's first door stands and how it faces.
fn first_door(h: &mut Harness<'_, Moonglow>, area: ResRef) -> (Vec3, f32) {
    let d = &doors(h, area)[0];
    let at = Vec3::new(d.float("X").unwrap(), d.float("Y").unwrap(), d.float("Z").unwrap());
    (at, d.float("Bearing").unwrap())
}

/// A door placed freely and dragged within reach of a hook goes onto it,
/// facing as it does (the Status Bar saying so while it is held); dragged
/// with Alt held, it goes where the pointer leads it, off the hook, and
/// keeps its facing.
#[test]
fn a_placed_door_snaps_to_hooks_in_reach_and_alt_keeps_it_free() {
    let Some((mut h, area)) = door_harness("free-door-drag") else { return };
    h.state_mut().palette.selected = ResKey::parse("nw_door_normal", ResType::UTD);
    click(&mut h, area, FREE, egui::Modifiers::NONE);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Door, 0)]);
    assert_eq!(h.state().palette.selected, None, "the brush let go");
    let (at, facing) = first_door(&mut h, area);
    assert!((at - FREE).truncate().length() < 0.6, "{at:?}");
    assert!(facing.abs() < 1e-4, "unturned: {facing}");
    // Onto the east hook.
    let grip = h.state().area_views[&area].move_grip(0).expect("a grip on the door");
    let offset = screen(&h, area, at) - grip;
    let to = screen(&h, area, NEAR_HOOK) - offset;
    drag_door(&mut h, grip, to, false);
    let (at, facing) = first_door(&mut h, area);
    assert!((at.truncate() - glam::Vec2::new(20.0, 15.0)).length() < 1e-4, "on the hook: {at:?}");
    assert!((facing + std::f32::consts::FRAC_PI_2).abs() < 1e-5, "facing as it: {facing}");
    // With Alt, off it again, 1.5 m west of the hook: where led, facing
    // as it did.
    let off = Vec3::new(18.5, 15.0, 0.0);
    let grip = h.state().area_views[&area].move_grip(0).expect("a grip on the door");
    let offset = screen(&h, area, at) - grip;
    let to = screen(&h, area, off) - offset;
    drag_door(&mut h, grip, to, true);
    let (now, facing) = first_door(&mut h, area);
    assert!((now - at).truncate().length() > 0.5, "moved off the hook: {now:?}");
    assert!((now - off).truncate().length() < 0.6, "where led: {now:?}");
    assert!((facing + std::f32::consts::FRAC_PI_2).abs() < 1e-5, "its facing kept: {facing}");
    // Alt let go after the drag opened no menu (Alt alone does), and Q
    // turns the free door, as any object.
    assert!(!egui::Popup::is_any_open(&h.ctx), "no menu opened");
    h.hover_at(screen(&h, area, Vec3::new(25.0, 25.0, 0.0)));
    h.run_steps(3);
    h.key_press(egui::Key::Q);
    h.run_steps(2);
    rest_turn_keys(&mut h, area);
    let (_, turned) = first_door(&mut h, area);
    assert!((turned - facing).abs() > 0.1, "Q turned the free door: {turned}");
    picture(&mut h, area, "dragged_off_hook.png");
}
