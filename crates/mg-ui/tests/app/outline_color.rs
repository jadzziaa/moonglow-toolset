//! An encounter's own outline color (5.1.0, a builder's ask: overlapping
//! encounters told apart): Moonglow's `MG_OutlineColor` on the placed
//! encounter, drawn as its outline's and spawn points' color, kept when
//! it is selected (wider, over the dark edge), set in its Properties'
//! Basic page (Outline color, with swatches and Default), carried by
//! copy and paste and by undo. (The game's data and a GPU, else skipped.)

use crate::common::*;
use glam::Vec3;
use mg_area::{OUTLINE_COLOR, ObjectKind};
use mg_gff::Value;
use mg_ui::blueprint::{
    ENCOUNTER_OUTLINE, OUTLINE_COLOR_DEFAULT, OUTLINE_COLOR_LABEL, OUTLINE_SWATCHES,
};

/// Cyan, stored as the game stores colors (0x00BBGGRR).
const CYAN: u32 = 0x00E6_DC28;

fn rgb(c: u32) -> egui::Color32 {
    egui::Color32::from_rgb(c as u8, (c >> 8) as u8, (c >> 16) as u8)
}

fn orange() -> egui::Color32 {
    let [r, g, b] = ENCOUNTER_OUTLINE;
    egui::Color32::from_rgb(r, g, b)
}

/// Two encounters overlapping about (20, 30): the first in a color of
/// its own (`own`, where given), the second without, each with a spawn
/// point; placed as one step.
fn two_encounters(h: &mut Harness<'_, Moonglow>, area: ResRef, own: Option<u32>) {
    let app = h.state_mut();
    let game = app.game.as_deref().unwrap();
    let edits = encounters(game, own)
        .into_iter()
        .enumerate()
        .map(|(index, item)| mg_edit::Edit::InsertItem {
            key: ResKey::new(area, ResType::GIT),
            path: mg_edit::GffPath::root(),
            list: "Encounter List".into(),
            index,
            item,
        })
        .collect();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.run_steps(3);
    settle(h);
}

/// The two encounters of [`two_encounters`], as placed objects.
fn encounters(game: &mg_rules::GameData, own: Option<u32>) -> Vec<mg_gff::Struct> {
    use mg_module::instances::{Placement, Placing, instance};
    let key = ResKey::parse("nw_giantevil", ResType::UTE).unwrap();
    let blueprint = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let square = |a: f32, b: f32| [[a, a, 0.0], [b, a, 0.0], [b, b, 0.0], [a, b, 0.0]];
    let made = [(-4.0, 3.0, 19.0), (-1.0, 6.0, 22.0)].map(|(a, b, x)| {
        let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
        let mut e = instance(&placing, ResType::UTE, &blueprint, at, &square(a, b)).unwrap();
        let mut point = mg_gff::Struct::new(2);
        for (label, v) in [("X", x), ("Y", 30.0), ("Z", 0.0), ("Orientation", 0.0)] {
            point.set(label, Value::Float(v));
        }
        e.set("SpawnPointList", Value::List(vec![point]));
        e
    });
    let [mut first, second] = made;
    if let Some(c) = own {
        first.set(OUTLINE_COLOR, Value::Dword(c));
    }
    vec![first, second]
}

/// The widths of the lines drawn over `area`'s view in `color`.
fn widths(h: &Harness<'_, Moonglow>, area: ResRef, color: egui::Color32) -> Vec<f32> {
    let rect = h.state().area_views[&area].rect;
    let mut out = Vec::new();
    for cs in h.output().shapes.iter().filter(|cs| rect.intersects(cs.clip_rect)) {
        match &cs.shape {
            egui::Shape::LineSegment { stroke, .. } if stroke.color == color => {
                out.push(stroke.width);
            }
            egui::Shape::Path(p) if p.stroke.color == egui::epaint::ColorMode::Solid(color) => {
                out.push(p.stroke.width);
            }
            _ => {}
        }
    }
    out
}

/// The middle of encounter `index`'s first outline edge, as the view
/// has it (on the ground).
fn edge_middle(h: &Harness<'_, Moonglow>, area: ResRef, index: usize) -> Vec3 {
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    let o = model.object(ObjectKind::Encounter, index).unwrap();
    (o.outline[0] + o.outline[1]) / 2.0
}

/// Whether a pixel `like` says is drawn within 3 points of where `at`
/// (on the ground) is on screen.
fn near(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    at: Vec3,
    like: &dyn Fn([u8; 3]) -> bool,
) -> bool {
    let p = screen(h, area, at);
    let scale = h.ctx.pixels_per_point();
    let image = h.render().expect("render");
    (-3..=3).any(|dx| {
        (-3..=3).any(|dy| {
            let x = ((p.x + dx as f32) * scale).round() as u32;
            let y = ((p.y + dy as f32) * scale).round() as u32;
            let Some(px) = image.get_pixel_checked(x, y) else { return false };
            like([px[0], px[1], px[2]])
        })
    })
}

fn encounters_now(h: &mut Harness<'_, Moonglow>, area: ResRef) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    git.root.list("Encounter List").unwrap_or(&[]).to_vec()
}

fn own_color(e: &mg_gff::Struct) -> Option<&Value> {
    e.get(OUTLINE_COLOR)
}

fn select(h: &mut Harness<'_, Moonglow>, area: ResRef, objects: &[(ObjectKind, usize)]) {
    h.state_mut().area_views.get_mut(&area).unwrap().selection = objects.to_vec();
    settle(h);
}

/// The Outline color row's Default button.
fn reset<'h>(h: &'h Harness<'_, Moonglow>) -> egui_kittest::Node<'h> {
    let small = |n: &egui_kittest::Node<'_>| n.rect().width() < 120.0;
    h.get_all_by_label(OUTLINE_COLOR_DEFAULT).find(small).expect("Default")
}

/// The Outline color button.
fn well<'h>(h: &'h Harness<'_, Moonglow>) -> egui_kittest::Node<'h> {
    h.get_all_by(|n| {
        n.role() == egui::accesskit::Role::ColorWell
            && n.label().as_deref() == Some(OUTLINE_COLOR_LABEL)
    })
    .next()
    .expect("the Outline color button")
}

/// Scrolls the Properties page to the Outline color row (the Basic
/// page's last).
fn to_row(h: &mut Harness<'_, Moonglow>) {
    well(h).scroll_to_me();
    settle(h);
}

fn undo_label(h: &Harness<'_, Moonglow>) -> Option<String> {
    h.state().ws.as_ref().unwrap().can_undo().map(str::to_string)
}

/// An encounter's own color is its outline's and its spawn points'; one
/// without is orange. Selected, the one of its own color keeps it, drawn
/// wider (over the dark edge); the one without takes the selection's
/// color as before. A color set later shows at once, without the area
/// read whole (a field of [`mg_area::PLACED_ONLY`]).
#[test]
fn an_encounter_is_drawn_in_its_own_color_selected_or_not() {
    let Some((mut h, area)) = area_harness("outline-color-draw") else { return };
    two_encounters(&mut h, area, Some(CYAN));
    let (cyan, orange) = (rgb(CYAN), orange());
    let mine = widths(&h, area, cyan);
    // (Its outline's 4 sides, a spawn post's 9 edges and its arrow's 3.)
    assert!(mine.len() >= 16 && mine.iter().all(|w| *w == 1.5), "{mine:?}");
    let theirs = widths(&h, area, orange);
    assert!(theirs.len() >= 16 && theirs.iter().all(|w| *w == 1.5), "{theirs:?}");
    // And in the picture: cyan on the first's south edge, orange on the
    // second's.
    if !no_adapter() {
        let [first, second] = [0, 1].map(|i| edge_middle(&h, area, i));
        // (A line 1.5 points wide over two rows of pixels: blended with
        // the ground, so its hue is looked for, not its exact color.)
        let cyanish = |[r, g, b]: [u8; 3]| g > 150 && b > 150 && r < 100;
        let orangeish = |[r, g, b]: [u8; 3]| r > 150 && (60..160).contains(&g) && b < 90;
        assert!(near(&mut h, area, first, &cyanish), "cyan drawn at {first}");
        assert!(!near(&mut h, area, first, &orangeish), "not orange at {first}");
        assert!(near(&mut h, area, second, &orangeish), "orange drawn at {second}");
    }

    select(&mut h, area, &[(ObjectKind::Encounter, 0)]);
    let mine = widths(&h, area, cyan);
    assert!(mine.len() >= 16, "{mine:?}");
    assert!(mine.iter().all(|w| *w == 2.5), "selected: wider, its own color kept: {mine:?}");
    assert_eq!(widths(&h, area, orange).len(), theirs.len(), "the other as it was");

    // Without a color of its own, the selection's color as before.
    select(&mut h, area, &[(ObjectKind::Encounter, 1)]);
    assert!(widths(&h, area, orange).is_empty(), "the selection's color, not orange");
    assert!(widths(&h, area, cyan).iter().all(|w| *w == 1.5));

    // Set on the other: drawn at once, the area not read whole.
    let reads = h.state().area_views[&area].whole_reads;
    let edit = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::GIT),
        path: mg_edit::GffPath::root().item("Encounter List", 1),
        label: OUTLINE_COLOR.into(),
        value: Some(Value::Dword(0x0032_DCFA)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Color", vec![edit])));
    settle(&mut h);
    assert_eq!(h.state().area_views[&area].whole_reads, reads, "not read whole");
    let yellow = widths(&h, area, rgb(0x0032_DCFA));
    assert!(yellow.len() >= 16 && yellow.iter().all(|w| *w == 2.5), "{yellow:?}");
}

/// Properties › Basic › Outline color: a swatch picked is stored when
/// the picker closes, one step undone and redone; Default takes the field
/// off (and is off with none set). Two encounters selected take it both.
#[test]
fn the_outline_color_row_sets_undoes_and_resets() {
    let Some((mut h, area)) = area_harness("outline-color-row") else { return };
    wide_window(&mut h);
    two_encounters(&mut h, area, None);
    select(&mut h, area, &[(ObjectKind::Encounter, 1)]);
    h.get_by_label("Properties, Encounter");
    to_row(&mut h);
    assert!(reset(&h).accesskit_node().is_disabled(), "nothing to reset");
    well(&h).click();
    settle(&mut h);
    let (cyan_name, cyan) = OUTLINE_SWATCHES[2];
    assert!(h.query_by_label("Paste Color").is_some(), "the picker open");
    h.get_by_label(cyan_name).click();
    settle(&mut h);
    let stored = u32::from(cyan[0]) | u32::from(cyan[1]) << 8 | u32::from(cyan[2]) << 16;
    let now = encounters_now(&mut h, area);
    assert_eq!(own_color(&now[1]), Some(&Value::Dword(stored)), "stored at once");
    // (Escape, the picker closed: as it was stored.)
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label("Paste Color").is_none(), "the picker closed");
    let now = encounters_now(&mut h, area);
    assert_eq!(own_color(&now[1]), Some(&Value::Dword(stored)));
    assert!(own_color(&now[0]).is_none(), "the other untouched");
    assert_eq!(undo_label(&h).as_deref(), Some(OUTLINE_COLOR_LABEL));
    assert!(!reset(&h).accesskit_node().is_disabled());

    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert!(own_color(&encounters_now(&mut h, area)[1]).is_none(), "undone: the field gone");
    h.state_mut().actions.push(mg_ui::Action::Redo);
    settle(&mut h);
    assert_eq!(own_color(&encounters_now(&mut h, area)[1]), Some(&Value::Dword(stored)));

    reset(&h).click();
    settle(&mut h);
    let now = encounters_now(&mut h, area);
    assert!(own_color(&now[1]).is_none(), "Default takes the field off: {:?}", now[1]);
    assert!(reset(&h).accesskit_node().is_disabled());

    // Both selected: a swatch sets both, Default takes it off both.
    select(&mut h, area, &[(ObjectKind::Encounter, 0), (ObjectKind::Encounter, 1)]);
    to_row(&mut h);
    well(&h).click();
    settle(&mut h);
    h.get_by_label(OUTLINE_SWATCHES[0].0).click();
    settle(&mut h);
    to_row(&mut h);
    well(&h).click();
    settle(&mut h);
    let now = encounters_now(&mut h, area);
    assert!(now.iter().all(|e| own_color(e).is_some()), "both: {now:?}");
    reset(&h).click();
    settle(&mut h);
    assert!(encounters_now(&mut h, area).iter().all(|e| own_color(e).is_none()));
}

/// A pick is stored on its object at once: the selection changed after
/// it writes nothing more, and coming back to the object later writes
/// nothing either (the review's probe: the color had waited, and landed
/// as a new step when the object was next shown).
#[test]
fn a_pick_is_stored_at_once_and_never_later() {
    let Some((mut h, area)) = area_harness("outline-color-at-once") else { return };
    wide_window(&mut h);
    two_encounters(&mut h, area, None);
    select(&mut h, area, &[(ObjectKind::Encounter, 1)]);
    h.get_by_label("Properties, Encounter");
    to_row(&mut h);
    well(&h).click();
    settle(&mut h);
    h.get_by_label(OUTLINE_SWATCHES[2].0).click();
    settle(&mut h);
    let revision = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().revision();
    let at = revision(&h);
    let now = encounters_now(&mut h, area);
    assert!(own_color(&now[1]).is_some() && own_color(&now[0]).is_none(), "{now:?}");
    assert_eq!(undo_label(&h).as_deref(), Some(OUTLINE_COLOR_LABEL));
    select(&mut h, area, &[(ObjectKind::Encounter, 0)]);
    select(&mut h, area, &[]);
    select(&mut h, area, &[(ObjectKind::Encounter, 1)]);
    h.get_by_label("Properties, Encounter");
    assert_eq!(revision(&h), at, "nothing written after");
    assert!(own_color(&encounters_now(&mut h, area)[0]).is_none(), "nor on the other");
}

/// A drag on the picker is stored when the button is let go; held while
/// its encounter is deleted, it is written nowhere (not on the other
/// encounter, not when the deletion is undone).
#[test]
fn a_drag_held_while_its_encounter_is_deleted_writes_nothing() {
    let Some((mut h, area)) = area_harness("outline-color-drag") else { return };
    wide_window(&mut h);
    two_encounters(&mut h, area, None);
    select(&mut h, area, &[(ObjectKind::Encounter, 1)]);
    h.get_by_label("Properties, Encounter");
    to_row(&mut h);
    well(&h).click();
    settle(&mut h);
    // (The picker's square, over its Paste Color button.)
    let paste = h.get_by_label("Paste Color").rect();
    let (a, b) = (
        egui::pos2(paste.left(), paste.top() - 150.0),
        egui::pos2(paste.left() + 40.0, paste.top() - 120.0),
    );
    h.hover_at(a);
    press(&h, a, true, egui::Modifiers::NONE);
    h.run_steps(2);
    h.hover_at(b);
    h.run_steps(2);
    assert!(own_color(&encounters_now(&mut h, area)[1]).is_none(), "held: not yet stored");
    // Its encounter deleted while the button is held.
    let edit = mg_edit::Edit::remove_item(
        ResKey::new(area, ResType::GIT),
        mg_edit::GffPath::root(),
        "Encounter List",
        1,
    );
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Delete", vec![edit])));
    select(&mut h, area, &[]);
    press(&h, b, false, egui::Modifiers::NONE);
    settle(&mut h);
    let now = encounters_now(&mut h, area);
    assert_eq!(now.len(), 1);
    assert!(own_color(&now[0]).is_none(), "not on the other: {:?}", now[0]);
    assert_eq!(undo_label(&h).as_deref(), Some("Delete"));
    select(&mut h, area, &[(ObjectKind::Encounter, 0)]);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    select(&mut h, area, &[(ObjectKind::Encounter, 1)]);
    h.get_by_label("Properties, Encounter");
    let now = encounters_now(&mut h, area);
    assert!(now.iter().all(|e| own_color(e).is_none()), "written nowhere: {now:?}");
}

/// A drag on the picker, let go, is stored then, one step.
#[test]
fn a_drag_on_the_picker_is_stored_when_let_go() {
    let Some((mut h, area)) = area_harness("outline-color-drag-go") else { return };
    wide_window(&mut h);
    two_encounters(&mut h, area, None);
    select(&mut h, area, &[(ObjectKind::Encounter, 1)]);
    h.get_by_label("Properties, Encounter");
    to_row(&mut h);
    well(&h).click();
    settle(&mut h);
    let paste = h.get_by_label("Paste Color").rect();
    let (a, b) = (
        egui::pos2(paste.left(), paste.top() - 150.0),
        egui::pos2(paste.left() + 40.0, paste.top() - 120.0),
    );
    h.hover_at(a);
    press(&h, a, true, egui::Modifiers::NONE);
    h.run_steps(2);
    h.hover_at(b);
    h.run_steps(2);
    assert!(own_color(&encounters_now(&mut h, area)[1]).is_none(), "held: not yet stored");
    press(&h, b, false, egui::Modifiers::NONE);
    settle(&mut h);
    assert!(own_color(&encounters_now(&mut h, area)[1]).is_some(), "let go: stored");
    assert_eq!(undo_label(&h).as_deref(), Some(OUTLINE_COLOR_LABEL));
}

/// A field of another type (another tool's) reads as no color, and
/// Default still takes it off.
#[test]
fn default_takes_off_a_field_of_another_type() {
    let Some((mut h, area)) = area_harness("outline-color-typed") else { return };
    wide_window(&mut h);
    two_encounters(&mut h, area, None);
    let edit = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::GIT),
        path: mg_edit::GffPath::root().item("Encounter List", 1),
        label: OUTLINE_COLOR.into(),
        value: Some(Value::String(b"cyan".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Typed", vec![edit])));
    settle(&mut h);
    select(&mut h, area, &[(ObjectKind::Encounter, 1)]);
    h.get_by_label("Properties, Encounter");
    to_row(&mut h);
    assert!(!reset(&h).accesskit_node().is_disabled(), "Default on for a field of any type");
    reset(&h).click();
    settle(&mut h);
    assert!(own_color(&encounters_now(&mut h, area)[1]).is_none(), "taken off");
}

/// The row is the placed encounter's alone: an encounter blueprint's
/// Basic page has none.
#[test]
fn an_encounter_blueprint_has_no_outline_color() {
    let ute = |m: &mut mg_module::Module, game: &mg_rules::GameData, _: ResRef| {
        let key = ResKey::parse("nw_giantevil", ResType::UTE).unwrap();
        let gff = Gff::read(&game.resman.get(&key).unwrap()).unwrap();
        m.set_gff(ResKey::parse("goblins", ResType::UTE).unwrap(), &gff).unwrap();
    };
    let Some((mut h, _)) = area_harness_with("outline-color-ute", "ttr01", None, 4, None, &ute)
    else {
        return;
    };
    wide_window(&mut h);
    let key = ResKey::parse("goblins", ResType::UTE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
    settle(&mut h);
    h.get_by_label("Maximum creatures");
    assert!(h.query_by_label(OUTLINE_COLOR_LABEL).is_none(), "no Outline color on a blueprint");
}

/// In an area claimed by another builder (read-only), the row is shown
/// and changes nothing: neither a swatch nor Default.
#[test]
fn a_read_only_area_s_outline_color_is_looked_at() {
    let put = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        let key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&key).unwrap().unwrap();
        git.root.set("Encounter List", Value::List(encounters(game, Some(CYAN))));
        m.set_gff(key, &git).unwrap();
    };
    let Some((mut h, area)) = read_only_area_harness_prepared("outline-color-ro", "", &put) else {
        return;
    };
    wide_window(&mut h);
    select(&mut h, area, &[(ObjectKind::Encounter, 0)]);
    h.get_by_label_contains("read-only here, shown to look at");
    let before = encounters_now(&mut h, area);
    assert_eq!(own_color(&before[0]), Some(&Value::Dword(CYAN)));
    h.get_all_by_label(OUTLINE_COLOR_LABEL).next().expect("the row shown");
    to_row(&mut h);
    let at = reset(&h).rect().center();
    click_pos(&h, at);
    settle(&mut h);
    let at = well(&h).rect().center();
    click_pos(&h, at);
    settle(&mut h);
    assert!(h.query_by_label("Paste Color").is_some(), "the picker opens, to look at");
    h.get_by_label(OUTLINE_SWATCHES[0].0).click();
    settle(&mut h);
    click_pos(&h, at);
    settle(&mut h);
    assert_eq!(encounters_now(&mut h, area), before, "nothing changed");
}

/// Copied and pasted, the encounter's copy has its color; deleted and
/// brought back by Undo, it has it again.
#[test]
fn copy_paste_and_undo_carry_the_outline_color() {
    let Some((mut h, area)) = area_harness("outline-color-copy") else { return };
    two_encounters(&mut h, area, Some(CYAN));
    select(&mut h, area, &[(ObjectKind::Encounter, 0)]);
    let over = screen(&h, area, Vec3::new(30.0, 15.0, 0.0));
    h.hover_at(over);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting);
    let over = screen(&h, area, Vec3::new(30.0, 15.0, 0.0));
    h.hover_at(over);
    press(&h, over, true, egui::Modifiers::NONE);
    press(&h, over, false, egui::Modifiers::NONE);
    settle(&mut h);
    let now = encounters_now(&mut h, area);
    assert_eq!(now.len(), 3, "pasted");
    assert_eq!(own_color(&now[2]), Some(&Value::Dword(CYAN)), "the copy's color");

    select(&mut h, area, &[(ObjectKind::Encounter, 0)]);
    let at = screen(&h, area, Vec3::new(30.0, 15.0, 0.0));
    h.hover_at(at);
    h.run_steps(1);
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    let now = encounters_now(&mut h, area);
    assert_eq!(now.len(), 2, "deleted");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    let now = encounters_now(&mut h, area);
    assert_eq!(now.len(), 3);
    assert_eq!(own_color(&now[0]), Some(&Value::Dword(CYAN)), "back with its color");
}

/// Pictures (not a check): two overlapping encounters before (both
/// orange) and after (the first cyan), unselected and with each selected,
/// in the theme `MG_LIGHT` says; and the Properties row with its picker
/// open. Into `MOONGLOW_TEST_OUTPUT`'s `outline-color/`.
#[test]
#[ignore]
fn outline_color_pictures() {
    let light = std::env::var_os("MG_LIGHT").is_some();
    let theme = if light { "light" } else { "dark" };
    let dir = mg_testkit::output_dir("outline-color");
    for (when, own) in [("before", None), ("after", Some(CYAN))] {
        let Some((mut h, area)) = area_harness(&format!("outline-color-pic-{when}")) else {
            return;
        };
        h.state_mut().settings.light_theme = light;
        two_encounters(&mut h, area, own);
        if let Some(o) = &mut h.state_mut().area_views.get_mut(&area).unwrap().orbit {
            o.target = Vec3::new(21.0, 30.5, 0.0);
            o.pitch = 55f32.to_radians();
            o.distance = 22.0;
        }
        settle(&mut h);
        let shoot = |h: &mut Harness<'_, Moonglow>, name: &str| {
            h.run_steps(3);
            let image = h.render().expect("render");
            image.save(dir.join(format!("{name}-{theme}.png"))).unwrap();
        };
        shoot(&mut h, &format!("{when}-unselected"));
        for i in 0..2 {
            select(&mut h, area, &[(ObjectKind::Encounter, i)]);
            shoot(&mut h, &format!("{when}-selected-{i}"));
        }
        if own.is_some() {
            wide_window(&mut h);
            select(&mut h, area, &[(ObjectKind::Encounter, 0)]);
            to_row(&mut h);
            shoot(&mut h, "properties-row");
            well(&h).click();
            settle(&mut h);
            shoot(&mut h, "properties-picker");
        }
    }
}
