//! The area view's Player's Light (the Lighting popover): in a dim area,
//! the game's player's vision light at the module's start location
//! (`mg_area::player_light`), off by default; what it lights, where it is
//! out, how it follows the start location, and that it is kept.

use crate::common::*;
use glam::Vec3;

/// Where the module starts (ttr01, 4 by 4 tiles), facing north, and a
/// place beyond the light's reach (22 m) from it.
const START: Vec3 = Vec3::new(12.0, 12.0, 0.0);
const FAR: Vec3 = Vec3::new(34.0, 34.0, 0.0);

/// A dark sun (the readings' dark scene: ambient 0x080808, no diffuse)
/// and a bright one (a stock day's: 0x606060 and 0xC0C0C0).
const DARK: (u32, u32) = (0x080808, 0);
const BRIGHT: (u32, u32) = (0x606060, 0xC0C0C0);

/// The area lit by `sun` by day and `moon` at night (shown by day), the
/// start location at [`START`] in it, the camera looking down over it,
/// still (no animations, no grass, no fog).
fn lit_harness(
    name: &str,
    sun: (u32, u32),
    moon: (u32, u32),
) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let prepare = move |m: &mut mg_module::Module, _: &mg_rules::GameData, area: ResRef| {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        for (label, v) in [
            ("SunAmbientColor", sun.0),
            ("SunDiffuseColor", sun.1),
            ("MoonAmbientColor", moon.0),
            ("MoonDiffuseColor", moon.1),
            ("SunFogAmount", 0),
            ("MoonFogAmount", 0),
        ] {
            are.root.set(label, mg_gff::Value::Dword(v));
        }
        are.root.set("IsNight", mg_gff::Value::Byte(0));
        are.root.set("DayNightCycle", mg_gff::Value::Byte(0));
        m.set_gff(key, &are).unwrap();
        let ifo_key = ResKey::parse("module", ResType::IFO).unwrap();
        let mut ifo = m.gff(&ifo_key).unwrap().unwrap();
        ifo.root.set("Mod_Entry_Area", mg_gff::Value::resref(area));
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
    let (mut h, area) = area_harness_with(name, "ttr01", None, 4, None, &prepare)?;
    {
        let s = &mut h.state_mut().settings;
        (s.still_objects, s.no_grass) = (true, true);
    }
    settle(&mut h);
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    (view.night, view.fog, view.show_start) = (false, false, false);
    let o = view.orbit.as_mut().unwrap();
    (o.target, o.yaw, o.pitch, o.distance) = (Vec3::new(22.0, 22.0, 0.0), 0.0, 1.45, 62.0);
    settle(&mut h);
    Some((h, area))
}

/// Player's Light set `on`, the picture drawn again and settled.
fn switch(h: &mut Harness<'_, Moonglow>, on: bool) {
    h.state_mut().settings.player_light = on;
    settle(h);
}

/// The view's picture (its clock held, so two compare).
fn picture(h: &mut Harness<'_, Moonglow>, area: ResRef) -> image::RgbaImage {
    h.state_mut().area_views.get_mut(&area).unwrap().set_clock(10.0);
    h.run_steps(2);
    let ppp = h.ctx.pixels_per_point();
    let rect = h.state().area_views[&area].rect;
    let shot = h.render().expect("render");
    let (x, y) = ((rect.left() * ppp) as u32, (rect.top() * ppp) as u32);
    let (w, hh) = ((rect.width() * ppp) as u32, (rect.height() * ppp) as u32);
    image::imageops::crop_imm(&shot, x, y, w, hh).to_image()
}

/// The mean of `picture`'s red, green and blue, 0 to 255, over 15 by 15
/// pixels about where `p` is seen.
fn level(h: &Harness<'_, Moonglow>, area: ResRef, picture: &image::RgbaImage, p: Vec3) -> f32 {
    let ppp = h.ctx.pixels_per_point();
    let view = &h.state().area_views[&area];
    let c = (screen(h, area, p) - view.rect.min) * ppp;
    let (mut sum, mut n) = (0.0, 0.0);
    for dy in -7..=7 {
        for dx in -7..=7 {
            let (x, y) = (c.x as i32 + dx, c.y as i32 + dy);
            if let Some(px) = picture.get_pixel_checked(x as u32, y as u32) {
                sum += (u32::from(px[0]) + u32::from(px[1]) + u32::from(px[2])) as f32 / 3.0;
                n += 1.0;
            }
        }
    }
    assert!(n > 0.0, "{p} is in the picture");
    sum / n
}

/// How many pixels of two pictures differ by more than 2 in a channel.
fn differ(a: &image::RgbaImage, b: &image::RgbaImage) -> usize {
    a.pixels().zip(b.pixels()).filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 2)).count()
}

/// The start location set to `at` facing `facing` (radians), as its
/// marker's drag and the Start Location's fields apply it.
fn set_start(h: &mut Harness<'_, Moonglow>, at: Vec3, facing: f32) {
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let set = |label: &str, v: f32| mg_edit::Edit::SetField {
        key: ifo,
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(mg_gff::Value::Float(v)),
    };
    let edits = vec![
        set("Mod_Entry_X", at.x),
        set("Mod_Entry_Y", at.y),
        set("Mod_Entry_Dir_X", facing.cos()),
        set("Mod_Entry_Dir_Y", facing.sin()),
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Move", edits)));
    settle(h);
}

/// Off (as it starts), nothing of it is drawn; on, in a dark area, the
/// ground about the start location is lit (ambient only, pale blue) and
/// beyond its 22 m nothing changes. Writes `off.png` and `on.png` to
/// `target/test-output/ui-area-player-light-dark/`, to look at.
#[test]
fn in_a_dim_area_the_start_location_is_lit() {
    let Some((mut h, area)) = lit_harness("player-light-dark", DARK, DARK) else { return };
    assert!(!h.state().settings.player_light, "off by default");
    let off = picture(&mut h, area);
    assert_eq!(h.state().area_views[&area].player_drawn(), None, "off: no player's light");
    switch(&mut h, true);
    let on = picture(&mut h, area);
    let drawn = h.state().area_views[&area].player_drawn().expect("the player's light drawn");
    assert!(drawn.0.truncate().distance(START.truncate()) < 1e-3, "{drawn:?}");
    assert!((drawn.1 - std::f32::consts::FRAC_PI_2).abs() < 1e-4, "facing north: {drawn:?}");
    let dir = mg_testkit::scratch_dir("ui-area-player-light-dark");
    off.save(dir.join("off.png")).unwrap();
    on.save(dir.join("on.png")).unwrap();
    let (was, now) = (level(&h, area, &off, START), level(&h, area, &on, START));
    assert!(now > was * 2.0 + 4.0, "the ground at the start lit: {was:.1} to {now:.1}");
    let (far_was, far_now) = (level(&h, area, &off, FAR), level(&h, area, &on, FAR));
    assert!((far_now - far_was).abs() < 1.0, "beyond its reach: {far_was:.1} to {far_now:.1}");
    // Close, as a player would see it (`close-off.png`, `close-on.png`).
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    view.show_start = true;
    let o = view.orbit.as_mut().unwrap();
    (o.target, o.yaw, o.pitch, o.distance) = (START, 0.4, 0.75, 16.0);
    settle(&mut h);
    let close_on = picture(&mut h, area);
    switch(&mut h, false);
    let close_off = picture(&mut h, area);
    close_on.save(dir.join("close-on.png")).unwrap();
    close_off.save(dir.join("close-off.png")).unwrap();
    assert!(differ(&close_off, &close_on) > 1000, "close, lit");
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    view.show_start = false;
    let o = view.orbit.as_mut().unwrap();
    (o.target, o.yaw, o.pitch, o.distance) = (Vec3::new(22.0, 22.0, 0.0), 0.0, 1.45, 62.0);
    settle(&mut h);
    // Off again: as it was.
    switch(&mut h, false);
    let again = picture(&mut h, area);
    assert_eq!(differ(&off, &again), 0, "off again, the picture as it was");
    // With the area's lighting off (the working light), none.
    h.state_mut().area_views.get_mut(&area).unwrap().lit = false;
    switch(&mut h, true);
    let unlit_on = picture(&mut h, area);
    switch(&mut h, false);
    let unlit_off = picture(&mut h, area);
    assert_eq!(differ(&unlit_on, &unlit_off), 0, "with Lighting off, no player's light");
}

/// In a bright area the light is out: the picture is the same with the
/// switch on and off.
#[test]
fn in_a_bright_area_nothing_changes() {
    let Some((mut h, area)) = lit_harness("player-light-bright", BRIGHT, BRIGHT) else { return };
    let off = picture(&mut h, area);
    switch(&mut h, true);
    let on = picture(&mut h, area);
    assert_eq!(differ(&off, &on), 0, "a bright area, the light out");
}

/// By day under a bright sun the light is out; the same area's night,
/// dark, has it: the view's time of day decides.
#[test]
fn a_dark_night_has_it_where_a_bright_day_does_not() {
    let Some((mut h, area)) = lit_harness("player-light-night", BRIGHT, DARK) else { return };
    let day_off = picture(&mut h, area);
    switch(&mut h, true);
    let day_on = picture(&mut h, area);
    assert_eq!(differ(&day_off, &day_on), 0, "by day, out");
    h.state_mut().area_views.get_mut(&area).unwrap().night = true;
    settle(&mut h);
    let night_on = picture(&mut h, area);
    switch(&mut h, false);
    let night_off = picture(&mut h, area);
    let (was, now) = (level(&h, area, &night_off, START), level(&h, area, &night_on, START));
    assert!(now > was * 2.0 + 4.0, "at night, lit: {was:.1} to {now:.1}");
}

/// Moved, the light goes with the start location (the old place dark as
/// it was, the new one lit); turned, it turns with it; in an area the
/// start location isn't in, there is none.
#[test]
fn the_light_follows_the_start_location() {
    let Some((mut h, area)) = lit_harness("player-light-moved", DARK, DARK) else { return };
    let off = picture(&mut h, area);
    switch(&mut h, true);
    let moved_to = Vec3::new(28.0, 26.0, 0.0);
    set_start(&mut h, moved_to, 0.0);
    let drawn = h.state().area_views[&area].player_drawn().expect("drawn");
    assert!(drawn.0.truncate().distance(moved_to.truncate()) < 1e-3, "moved: {drawn:?}");
    assert!(drawn.1.abs() < 1e-4, "facing east: {drawn:?}");
    let moved = picture(&mut h, area);
    let (old_was, old_now) = (level(&h, area, &off, START), level(&h, area, &moved, START));
    // (The old place 17 m from the new light, still in its reach: lit a
    // little, far less than under it.)
    let (new_was, new_now) = (level(&h, area, &off, moved_to), level(&h, area, &moved, moved_to));
    assert!(new_now > new_was * 2.0 + 4.0, "the new place lit: {new_was:.1} to {new_now:.1}");
    assert!(old_now - old_was < (new_now - new_was) / 2.0, "the old place less: {old_now:.1}");
    // Turned west: the light 1.28 m from where it was, the picture changed.
    set_start(&mut h, moved_to, std::f32::consts::PI);
    let drawn = h.state().area_views[&area].player_drawn().expect("drawn");
    assert!((drawn.1.cos() + 1.0).abs() < 1e-4 && drawn.1.sin().abs() < 1e-3, "west: {drawn:?}");
    let turned = picture(&mut h, area);
    assert!(differ(&moved, &turned) > 100, "turned, the light moved");
    // The start location in another area: none here.
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let elsewhere = mg_edit::Edit::SetField {
        key: ifo,
        path: mg_edit::GffPath::root(),
        label: "Mod_Entry_Area".into(),
        value: Some(mg_gff::Value::resref(ResRef::from_str("elsewhere").unwrap())),
    };
    h.state_mut()
        .actions
        .push(mg_ui::Action::Apply(mg_edit::Command::new("Elsewhere", vec![elsewhere])));
    settle(&mut h);
    assert_eq!(h.state().area_views[&area].player_drawn(), None, "not this area's start");
    let away = picture(&mut h, area);
    assert_eq!(differ(&off, &away), 0, "the start elsewhere, nothing lit");
}

/// While the start location's marker is dragged, the light goes with it,
/// before the drag ends.
#[test]
fn the_light_goes_with_the_marker_while_it_is_dragged() {
    let Some((mut h, area)) = lit_harness("player-light-drag", DARK, DARK) else { return };
    h.state_mut().area_views.get_mut(&area).unwrap().show_start = true;
    switch(&mut h, true);
    let none = egui::Modifiers::NONE;
    // By the ring's west side, four meters east.
    let from = screen(&h, area, START - Vec3::X);
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, none);
    let mut at = from;
    for k in 1..=4 {
        at = screen(&h, area, START + Vec3::new(k as f32 - 1.0, 0.0, 0.0));
        h.hover_at(at);
        h.run_steps(1);
    }
    h.run_steps(2);
    let held = h.state().area_views[&area].player_drawn().expect("drawn");
    assert!(held.0.x > START.x + 2.0, "held, the light moved east: {held:?}");
    press(&h, at, false, none);
    settle(&mut h);
    let dropped = h.state().area_views[&area].player_drawn().expect("drawn");
    assert!(dropped.0.x > START.x + 2.0, "dropped: {dropped:?}");
}

/// The Lighting popover's Player's light turns it on and off, and the
/// setting is kept for the views opened after (it is the settings').
#[test]
fn the_switch_is_in_the_lighting_popover_and_kept() {
    let Some((mut h, area)) = lit_harness("player-light-switch", DARK, DARK) else { return };
    area_click(&mut h, "Player's light");
    settle(&mut h);
    assert!(h.state().settings.player_light, "checked, on");
    assert!(h.state().area_views[&area].player_drawn().is_some(), "and drawn");
    let saved = h.state().settings.saved_text();
    let (read, _) = mg_ui::Settings::read_fields(&saved).expect("it reads");
    assert!(read.player_light, "kept in the saved settings");
    area_click(&mut h, "Player's light");
    settle(&mut h);
    assert!(!h.state().settings.player_light, "unchecked, off");
    assert_eq!(h.state().area_views[&area].player_drawn(), None);
}

/// With the game's own shaders (the Shaders switch, as the area view
/// opens), the same: the start lit, nothing beyond the light's reach.
/// Writes `on.png` and `off.png` to
/// `target/test-output/ui-area-player-light-shaders/`.
#[test]
fn with_the_game_s_shaders_the_start_location_is_lit_too() {
    let Some((mut h, area)) = lit_harness("player-light-shaders", DARK, DARK) else { return };
    h.state_mut().set_game_shaders(true);
    let drawn = |h: &mut Harness<'_, Moonglow>| {
        let start = std::time::Instant::now();
        loop {
            settle(h);
            let r = &h.state().viewport.as_ref().unwrap().renderer;
            if r.drawn.game_pending == 0 && r.drawn.textures_pending == 0 {
                break;
            }
            assert!(start.elapsed().as_secs() < 120, "the game's programs never made");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    drawn(&mut h);
    let off = picture(&mut h, area);
    switch(&mut h, true);
    drawn(&mut h);
    let on = picture(&mut h, area);
    let dir = mg_testkit::scratch_dir("ui-area-player-light-shaders");
    off.save(dir.join("off.png")).unwrap();
    on.save(dir.join("on.png")).unwrap();
    let (was, now) = (level(&h, area, &off, START), level(&h, area, &on, START));
    assert!(now > was * 2.0 + 4.0, "the ground at the start lit: {was:.1} to {now:.1}");
    let (far_was, far_now) = (level(&h, area, &off, FAR), level(&h, area, &on, FAR));
    assert!((far_now - far_was).abs() < 1.0, "beyond its reach: {far_was:.1} to {far_now:.1}");
}
