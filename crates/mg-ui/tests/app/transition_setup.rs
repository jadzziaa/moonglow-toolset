//! Setup Area Transition with triggers, doors and waypoints, as Aurora's
//! (`mg_edit::transition`; Aurora's measurements in
//! `crates/mg-corpus-tests/tests/aurora_transitions.rs`): what the window
//! lists and offers, and what its OK writes, in one step.

use crate::common::*;
use crate::transition_triggers::{draw, harness as two_areas, instance_tab, show_area, square};
use egui::accesskit::Role;
use mg_edit::GffPath;
use mg_gff::{Struct, Value};

fn text(s: &str) -> Value {
    Value::String(s.as_bytes().to_vec())
}

/// The game's `newtransition` placed at (x, y) as a 6 m square, tagged
/// `tag`, of Trigger Type `kind` (1 an area transition, 0 generic, 2 a
/// trap).
fn trigger(game: &mg_rules::GameData, tag: &str, kind: i32, x: f32, y: f32) -> Struct {
    use mg_module::instances::{Placement, Placing, instance};
    let key = ResKey::parse("newtransition", ResType::UTT).unwrap();
    let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
    let outline = [[0.0, 0.0, 0.025], [6.0, 0.0, 0.025], [6.0, 6.0, 0.025], [0.0, 6.0, 0.025]];
    let mut s = instance(&placing, ResType::UTT, &bp, at, &outline).unwrap();
    s.set("Tag", text(tag));
    s.set("Type", Value::Int(kind));
    s
}

/// The game's `nw_door_normal` placed at (x, y), tagged `tag`.
fn door(game: &mg_rules::GameData, tag: &str, x: f32, y: f32) -> Struct {
    use mg_module::instances::{Placement, Placing, instance};
    let key = ResKey::parse("nw_door_normal", ResType::UTD).unwrap();
    let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
    let mut s = instance(&placing, ResType::UTD, &bp, at, &[]).unwrap();
    s.set("Tag", text(tag));
    s
}

/// A waypoint tagged `tag` at (x, y).
fn waypoint_at(tag: &str, x: f32, y: f32) -> Struct {
    mg_module::instances::walk_waypoint(tag.as_bytes(), [x, y, 0.0])
}

/// The fixture's two areas: `Field` (the harness's, with its two
/// waypoints) and `Second`.
///
/// The module has a transition blueprint of its own, `transit`.
/// Field: triggers TT_A1 (2, 2), TT_A2 (2, 12), the generic TT_GEN
/// (12, 2), the trap TT_TRAP (12, 12), TT_TWIN (2, 30) and TT_TWIN
/// (30, 30); the door TT_DA (34, 4).
/// Second: triggers TT_B1 (4, 4) and TT_B2 (4, 20), the door TT_DB
/// (30, 10), the waypoint WP_FAR (20, 30) and, with `clash`, a waypoint
/// WP_TT_B2 far from TT_B2 (30, 34).
struct World {
    h: Harness<'static, Moonglow>,
    field: ResRef,
    second: ResRef,
}

fn world_with(name: &str, clash: bool) -> Option<World> {
    let prepare = move |m: &mut Module, game: &mg_rules::GameData, field: ResRef| {
        use mg_module::new::{AreaSpec, add_area};
        let mut rng = fastrand::Rng::with_seed(11);
        let spec = AreaSpec {
            name: "Second".into(),
            tileset: ResRef::from_str("ttr01").unwrap(),
            width: 4,
            height: 4,
        };
        let second = add_area(m, game, &spec, &mut rng).unwrap();
        let set = |m: &mut Module, area: ResRef, lists: Vec<(&str, Vec<Struct>)>| {
            let key = ResKey::new(area, ResType::GIT);
            let mut git = m.gff(&key).unwrap().unwrap();
            for (list, mut items) in lists {
                let mut all = git.root.list(list).unwrap_or(&[]).to_vec();
                all.append(&mut items);
                git.root.set(list, Value::List(all));
            }
            m.set_gff(key, &git).unwrap();
        };
        let triggers = vec![
            trigger(game, "TT_A1", 1, 2.0, 2.0),
            trigger(game, "TT_A2", 1, 2.0, 12.0),
            trigger(game, "TT_GEN", 0, 12.0, 2.0),
            trigger(game, "TT_TRAP", 2, 12.0, 12.0),
            trigger(game, "TT_TWIN", 1, 2.0, 30.0),
            trigger(game, "TT_TWIN", 1, 30.0, 30.0),
        ];
        set(
            m,
            field,
            vec![("TriggerList", triggers), ("Door List", vec![door(game, "TT_DA", 34.0, 4.0)])],
        );
        // A transition blueprint of the module's own.
        let resref = ResRef::from_str("transit").unwrap();
        let mut utt = mg_module::blueprints::trigger(game, resref, "Transit", 0);
        utt.root.set("Type", Value::Int(1));
        m.set_gff(ResKey::new(resref, ResType::UTT), &utt).unwrap();
        let mut waypoints = vec![waypoint_at("WP_FAR", 20.0, 30.0)];
        if clash {
            waypoints.push(waypoint_at("WP_TT_B2", 30.0, 34.0));
        }
        set(
            m,
            second,
            vec![
                (
                    "TriggerList",
                    vec![trigger(game, "TT_B1", 1, 4.0, 4.0), trigger(game, "TT_B2", 1, 4.0, 20.0)],
                ),
                ("Door List", vec![door(game, "TT_DB", 30.0, 10.0)]),
                ("WaypointList", waypoints),
            ],
        );
    };
    let (h, field) = area_harness_with(name, "ttr01", None, 4, None, &prepare)?;
    let second = h.state().ws.as_ref()?.module.areas().ok()?.into_iter().find(|a| *a != field)?;
    Some(World { h, field, second })
}

fn world(name: &str) -> Option<World> {
    world_with(name, false)
}

fn git(area: ResRef) -> ResKey {
    ResKey::new(area, ResType::GIT)
}

/// Item `index` of list `list` of the area's GIT, as the workspace has it.
fn object(h: &mut Harness<'_, Moonglow>, area: ResRef, list: &str, index: usize) -> Struct {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let doc = ws.doc(&git(area)).unwrap();
    doc.root.list(list).unwrap()[index].clone()
}

fn list(h: &mut Harness<'_, Moonglow>, area: ResRef, list: &str) -> Vec<Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&git(area)).unwrap().root.list(list).unwrap_or(&[]).to_vec()
}

fn tag_of(s: &Struct) -> String {
    String::from_utf8_lossy(s.string("Tag").unwrap_or_default()).into_owned()
}

/// An object's `LinkedTo` and `LinkedToFlags`.
fn link(s: &Struct) -> (String, i64) {
    let to = String::from_utf8_lossy(s.string("LinkedTo").unwrap_or_default()).into_owned();
    (to, s.integer("LinkedToFlags").unwrap_or(0))
}

fn linked(to: &str, flags: i64) -> (String, i64) {
    (to.to_string(), flags)
}

fn tags(h: &mut Harness<'_, Moonglow>, area: ResRef, of: &str) -> Vec<String> {
    list(h, area, of).iter().map(tag_of).collect()
}

/// A waypoint's position, X and Y.
fn at(s: &Struct) -> (f32, f32) {
    (s.float("XPosition").unwrap(), s.float("YPosition").unwrap())
}

fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
}

/// Opens `tab`'s Properties on its Area Transition page and presses
/// Setup Area Transition…; then shows `area` (by its name) when given.
fn open_setup(h: &mut Harness<'_, Moonglow>, tab: Tab, area: Option<&str>) {
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
    settle(h);
    h.get_all_by_label("Area Transition").next().unwrap().click();
    settle(h);
    h.get_by_label("Setup Area Transition…").click();
    settle(h);
    if let Some(name) = area {
        show_target_area(h, name);
    }
}

/// Picks the target area `name` in the window's Target Area.
fn show_target_area(h: &mut Harness<'_, Moonglow>, name: &str) {
    // (Not one of the area toolbar's boxes behind the window on the same
    // row, where M3 T14's flush editor area moved the toolbar's rows.)
    let label = h.get_by_label("Target Area").rect();
    let combo = h
        .get_all_by_role(Role::ComboBox)
        .find(|n| {
            let r = n.rect();
            (r.center().y - label.center().y).abs() < 10.0
                && r.left() > label.left()
                && !in_group(n, "Area toolbar")
        })
        .unwrap_or_else(|| {
            let label = h.get_by_label("Target Area").rect();
            let all: Vec<String> = h
                .get_all_by_role(Role::ComboBox)
                .map(|n| format!("{:?} {:?}", n.rect(), n.accesskit_node().value()))
                .collect();
            panic!("the Target Area box: label {label:?}, boxes {all:?}")
        });
    let value = combo.accesskit_node().value().unwrap_or_default().to_string();
    let under = combo.rect();
    combo.click();
    settle(h);
    // (The row of the box's list, under the box: not the module tree's
    // area of the same name, which may come after it among the nodes.)
    let row = h.query_all_by_label(name).find(|n| {
        let r = n.rect();
        r.top() >= under.bottom() - 1.0 && under.x_range().contains(r.center().x)
    });
    let Some(row) = row else {
        let all: Vec<String> = h
            .get_all_by_role(Role::Button)
            .filter_map(|n| n.accesskit_node().label().map(|l| l.to_string()))
            .collect();
        panic!("no {name} (box {value:?}): {all:?}");
    };
    row.click();
    settle(h);
}

/// Clicks the row of the window's lists whose text starts with `tag`
/// (the `nth` of them).
fn pick(h: &mut Harness<'_, Moonglow>, tag: &str, nth: usize) {
    let rows: Vec<_> = h
        .get_all_by_role(Role::Button)
        .filter(|n| {
            let l = n.accesskit_node().label().unwrap_or_default();
            l == tag || l.starts_with(&format!("{tag}  "))
        })
        .collect();
    assert!(rows.len() > nth, "{} rows of {tag}", rows.len());
    rows[nth].click();
    settle(h);
}

fn connection(h: &mut Harness<'_, Moonglow>, name: &str) {
    h.get_by_label(name).click();
    settle(h);
}

fn ok_button<'h>(h: &'h Harness<'_, Moonglow>) -> egui_kittest::Node<'h> {
    // (The window's: under its map. The Properties window has one too.)
    let map = h.get_by_label("The area's map").rect();
    let d = |n: &egui_kittest::Node<'_>| (n.rect().left() - map.left()).abs();
    (h.get_all_by_label("OK").filter(|n| n.rect().top() > map.bottom()))
        .min_by(|a, b| d(a).total_cmp(&d(b)))
        .expect("the window's OK")
}

/// The window's OK, pressed; the window closes. (Pressed where it is
/// once the window rests: centered, it moves as its lines change.)
fn ok(h: &mut Harness<'_, Moonglow>) {
    settle(h);
    let node = ok_button(h);
    assert!(!node.accesskit_node().is_disabled(), "OK can be pressed");
    node.click();
    settle(h);
    assert!(h.query_by_label("The area's map").is_none(), "closed");
}

/// The window's Cancel (under its map: the Properties window has one).
fn cancel(h: &mut Harness<'_, Moonglow>) {
    let map = h.get_by_label("The area's map").rect();
    let d = |n: &egui_kittest::Node<'_>| (n.rect().left() - map.left()).abs();
    let node = (h.get_all_by_label("Cancel").filter(|n| n.rect().top() > map.bottom()))
        .min_by(|a, b| d(a).total_cmp(&d(b)));
    node.expect("the window's Cancel").click();
    settle(h);
}

fn close(h: &mut Harness<'_, Moonglow>, tab: Tab) {
    h.state_mut().actions.push(mg_ui::Action::CloseTab(tab));
    settle(h);
}

fn trigger_tab(area: ResRef, index: usize) -> Tab {
    instance_tab(area, "TriggerList", index)
}

fn undo(h: &mut Harness<'_, Moonglow>) {
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(h);
}

fn redo(h: &mut Harness<'_, Moonglow>) {
    h.state_mut().actions.push(mg_ui::Action::Redo);
    settle(h);
}

/// Every trigger, door and waypoint of both areas.
fn everything(h: &mut Harness<'_, Moonglow>, areas: &[ResRef]) -> Vec<Vec<Struct>> {
    let mut out = Vec::new();
    for a in areas {
        for l in ["TriggerList", "Door List", "WaypointList"] {
            out.push(list(h, *a, l));
        }
    }
    out
}

/// The reporter's case: two unlinked transition triggers drawn from the
/// palette, one in each area, linked Both Ways. Each leads to a waypoint
/// made in the other's middle, tagged for it; the module saved and read
/// back has them; Undo takes back every object in one step and Redo puts
/// them back; the Properties window's Cancel after OK leaves the
/// transition whole.
#[test]
fn two_unlinked_triggers_are_linked_both_ways() {
    let Some((mut h, a1, a2)) = two_areas("setup-both-ways") else { return };
    let t1 = draw(&mut h, a1, "newtransition", &square(8.0, 8.0, 6.0));
    let t2 = draw(&mut h, a2, "newtransition", &square(24.0, 24.0, 6.0));
    // The first given a tag of its own, as in its Properties; the second
    // keeps its blueprint's (AreaTransition).
    let retag = mg_edit::Edit::SetField {
        key: git(a1),
        path: GffPath::root().item("TriggerList", t1),
        label: "Tag".into(),
        value: Some(text("TT_FIELD")),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![retag])));
    settle(&mut h);
    let before = everything(&mut h, &[a1, a2]);
    let wp_before =
        (list(&mut h, a1, "WaypointList").len(), list(&mut h, a2, "WaypointList").len());
    let tab = trigger_tab(a1, t1);
    open_setup(&mut h, tab.clone(), Some("second"));
    pick(&mut h, "AreaTransition", 0);
    h.get_by_label("Leads to the trigger tagged AreaTransition");
    // Both Ways, Aurora's default; and the lines of what OK does.
    assert!(
        h.get_by_label("Both Ways")
            .accesskit_node()
            .toggled()
            .is_some_and(|t| t == egui::accesskit::Toggled::True)
    );
    h.get_by_label_contains("Makes the waypoint WP_AreaTransition in second");
    h.get_by_label_contains("Makes the waypoint WP_TT_FIELD in field");
    h.get_by_label_contains("The target's tag (AreaTransition) is still its blueprint's");
    ok(&mut h);
    let (o1, o2) = (object(&mut h, a1, "TriggerList", t1), object(&mut h, a2, "TriggerList", t2));
    assert_eq!(link(&o1), linked("WP_AreaTransition", 2));
    assert_eq!(link(&o2), linked("WP_TT_FIELD", 2));
    let w1 = list(&mut h, a1, "WaypointList");
    let w2 = list(&mut h, a2, "WaypointList");
    assert_eq!((w1.len(), w2.len()), (wp_before.0 + 1, wp_before.1 + 1));
    assert_eq!(tag_of(w1.last().unwrap()), "WP_TT_FIELD");
    assert_eq!(tag_of(w2.last().unwrap()), "WP_AreaTransition");
    // (At the middle of each square: absolute, whatever the points' order.)
    assert!(near(at(w1.last().unwrap()), (11.0, 11.0)), "{:?}", at(w1.last().unwrap()));
    assert!(near(at(w2.last().unwrap()), (27.0, 27.0)), "{:?}", at(w2.last().unwrap()));
    let after = everything(&mut h, &[a1, a2]);
    // The Properties window's Cancel: the transition stays, both ends.
    h.get_all_by_label("Cancel").last().unwrap().click();
    settle(&mut h);
    assert_eq!(everything(&mut h, &[a1, a2]), after, "Cancel leaves it");
    // Saved and read back.
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    settle(&mut h);
    let path = h.state().ws.as_ref().unwrap().module.location.as_ref().unwrap().path().to_owned();
    let saved = Module::open(&path).unwrap();
    let read = |area: ResRef, l: &str| -> Vec<Struct> {
        saved.gff(&git(area)).unwrap().unwrap().root.list(l).unwrap_or(&[]).to_vec()
    };
    assert_eq!(link(&read(a1, "TriggerList")[t1]), linked("WP_AreaTransition", 2));
    assert_eq!(link(&read(a2, "TriggerList")[t2]), linked("WP_TT_FIELD", 2));
    assert_eq!(read(a1, "WaypointList"), w1);
    assert_eq!(read(a2, "WaypointList"), w2);
    // One step back, and again.
    undo(&mut h);
    assert_eq!(everything(&mut h, &[a1, a2]), before, "every object as it was");
    redo(&mut h);
    assert_eq!(everything(&mut h, &[a1, a2]), after, "every object again");
}

/// The lists: transition triggers of the area shown, linked or not, but
/// not the current one, nor generic and trap triggers; two of one tag as
/// two rows, the row picked deciding which is linked and where its
/// waypoint goes; picked on the map, a trigger's marker is at its
/// middle. In one area, the two waypoints one after the other, the
/// target's first.
#[test]
fn the_lists_hold_transition_triggers_and_two_of_one_tag_are_two() {
    let Some(World { mut h, field, .. }) = world("setup-lists") else { return };
    open_setup(&mut h, trigger_tab(field, 0), None);
    h.get_by_label("Available Triggers:");
    let rows = |h: &Harness<'_, Moonglow>, tag: &str| {
        h.get_all_by_role(Role::Button)
            .filter(|n| {
                n.accesskit_node()
                    .label()
                    .is_some_and(|l| l == tag || l.starts_with(&format!("{tag}  ")))
            })
            .count()
    };
    assert_eq!(rows(&h, "TT_A1"), 0, "the current trigger is left out");
    assert_eq!(rows(&h, "TT_A2"), 1);
    assert_eq!(rows(&h, "TT_TWIN"), 2, "two of one tag");
    assert_eq!(rows(&h, "TT_GEN"), 0, "a generic trigger");
    assert_eq!(rows(&h, "TT_TRAP"), 0, "a trap");
    assert_eq!(rows(&h, "TT_DA"), 1);
    // Triggers alone.
    h.get_by_label("Destinations Shown").get_by_label("Triggers").click();
    settle(&mut h);
    assert!(h.query_by_label("Available Doors:").is_none());
    assert_eq!(rows(&h, "TT_DA"), 0);
    h.get_by_label("Destinations Shown").get_by_label("All").click();
    settle(&mut h);
    // On the map: TT_A2's middle, (5, 15) of the area's 40 by 40 m.
    let map = h.get_by_label("The area's map").rect();
    let p = egui::pos2(
        map.left() + map.width() * 5.0 / 40.0,
        map.bottom() - map.height() * 15.0 / 40.0,
    );
    h.hover_at(p);
    settle(&mut h);
    // Its hover text says what it is.
    let tip = tip_at(&mut h, egui::Rect::from_center_size(p, egui::vec2(2.0, 2.0)));
    assert!(tip.as_deref().is_some_and(|t| t.contains("TT_A2 (trigger)")), "{tip:?}");
    press(&h, p, true, egui::Modifiers::NONE);
    press(&h, p, false, egui::Modifiers::NONE);
    settle(&mut h);
    h.get_by_label("Leads to the trigger tagged TT_A2");
    // The second TT_TWIN, at (30, 30).
    pick(&mut h, "TT_TWIN", 1);
    h.get_by_label("Leads to the trigger tagged TT_TWIN");
    let wp_before = list(&mut h, field, "WaypointList").len();
    let twin_first = object(&mut h, field, "TriggerList", 4);
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("WP_TT_TWIN", 2));
    assert_eq!(link(&object(&mut h, field, "TriggerList", 5)), linked("WP_TT_A1", 2));
    assert_eq!(object(&mut h, field, "TriggerList", 4), twin_first, "the other is unchanged");
    let w = list(&mut h, field, "WaypointList");
    assert_eq!(w.len(), wp_before + 2);
    assert_eq!(tag_of(&w[wp_before]), "WP_TT_TWIN", "the target's first");
    assert!(near(at(&w[wp_before]), (33.0, 33.0)), "in the second's middle");
    assert_eq!(tag_of(&w[wp_before + 1]), "WP_TT_A1");
    assert!(near(at(&w[wp_before + 1]), (5.0, 5.0)));
}

/// A door to a trigger and a trigger to a door, Both Ways: whatever
/// leads to a trigger names its waypoint (2), whatever leads to a door
/// names the door (1), and a door is given no waypoint. Each undone and
/// redone in one step.
#[test]
fn doors_and_triggers_lead_to_each_other() {
    let Some(World { mut h, field, second }) = world("setup-doors") else { return };
    let areas = [field, second];
    let start = everything(&mut h, &areas);
    let door_tab = instance_tab(field, "Door List", 0);
    open_setup(&mut h, door_tab.clone(), Some("second"));
    pick(&mut h, "TT_B1", 0);
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "Door List", 0)), linked("WP_TT_B1", 2));
    assert_eq!(link(&object(&mut h, second, "TriggerList", 0)), linked("TT_DA", 1));
    assert_eq!(tags(&mut h, second, "WaypointList"), ["WP_FAR", "WP_TT_B1"]);
    assert_eq!(list(&mut h, field, "WaypointList").len(), 2, "no waypoint for the door");
    let door_done = everything(&mut h, &areas);
    close(&mut h, door_tab);
    // A trigger to a door.
    let tab = trigger_tab(field, 1);
    open_setup(&mut h, tab.clone(), Some("second"));
    pick(&mut h, "TT_DB", 0);
    h.get_by_label("Leads to the door tagged TT_DB");
    h.get_by_label_contains("Makes the waypoint WP_TT_A2 in field");
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 1)), linked("TT_DB", 1));
    assert_eq!(link(&object(&mut h, second, "Door List", 0)), linked("WP_TT_A2", 2));
    let w = list(&mut h, field, "WaypointList");
    assert_eq!(tag_of(w.last().unwrap()), "WP_TT_A2");
    assert!(near(at(w.last().unwrap()), (5.0, 15.0)));
    let trigger_done = everything(&mut h, &areas);
    close(&mut h, tab);
    undo(&mut h);
    assert_eq!(everything(&mut h, &areas), door_done);
    undo(&mut h);
    assert_eq!(everything(&mut h, &areas), start);
    redo(&mut h);
    redo(&mut h);
    assert_eq!(everything(&mut h, &areas), trigger_done);
}

/// A waypoint is led to from the current end alone: Both Ways and Target
/// To Current grayed, Current To Target chosen; nothing made, the
/// waypoint unchanged.
#[test]
fn a_waypoint_takes_current_to_target_only() {
    let Some(World { mut h, field, second }) = world("setup-waypoint") else { return };
    let far = object(&mut h, second, "WaypointList", 0);
    open_setup(&mut h, trigger_tab(field, 0), Some("second"));
    pick(&mut h, "WP_FAR", 0);
    h.get_by_label("Leads to the waypoint tagged WP_FAR");
    assert!(h.get_by_label("Both Ways").accesskit_node().is_disabled());
    assert!(h.get_by_label("Target To Current").accesskit_node().is_disabled());
    assert!(!h.get_by_label("Current To Target").accesskit_node().is_disabled());
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("WP_FAR", 2));
    assert_eq!(object(&mut h, second, "WaypointList", 0), far);
    assert_eq!(list(&mut h, second, "WaypointList").len(), 1, "nothing made");
    assert_eq!(list(&mut h, field, "WaypointList").len(), 2, "nothing made");
}

/// A second run uses again the waypoint the first made; to another
/// target, the link replaced is said before OK, and the old partner is
/// left as it was (as Aurora's). The window opens again on the trigger
/// the object leads to (by its waypoint's tag).
#[test]
fn a_link_replaced_is_said_and_a_waypoint_used_again() {
    let Some(World { mut h, field, second }) = world("setup-relink") else { return };
    let tab = trigger_tab(field, 0);
    open_setup(&mut h, tab.clone(), Some("second"));
    pick(&mut h, "TT_B1", 0);
    ok(&mut h);
    let counts = |h: &mut Harness<'_, Moonglow>| {
        (list(h, field, "WaypointList").len(), list(h, second, "WaypointList").len())
    };
    let once = counts(&mut h);
    assert_eq!(once, (3, 2));
    // Again: opened on TT_B1, nothing made, nothing replaced.
    h.get_by_label("Setup Area Transition…").click();
    settle(&mut h);
    h.get_by_label("Leads to the trigger tagged TT_B1");
    h.get_by_label_contains("Uses the waypoint WP_TT_B1 already in second");
    h.get_by_label_contains("Uses the waypoint WP_TT_A1 already in field");
    assert!(h.query_by_label_contains("OK replaces").is_none());
    ok(&mut h);
    assert_eq!(counts(&mut h), once, "used again");
    // To TT_B2: what it replaces, said.
    h.get_by_label("Setup Area Transition…").click();
    settle(&mut h);
    pick(&mut h, "TT_B2", 0);
    h.get_by_label("TT_A1 already leads to WP_TT_B1: OK replaces that link");
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("WP_TT_B2", 2));
    assert_eq!(link(&object(&mut h, second, "TriggerList", 1)), linked("WP_TT_A1", 2));
    assert_eq!(link(&object(&mut h, second, "TriggerList", 0)), linked("WP_TT_A1", 2), "left");
    assert_eq!(counts(&mut h), (3, 3), "TT_B2's made, TT_A1's used again");
}

/// A waypoint made with a tag another object has already is weighed as
/// Duplicate Tags weighs what is placed (the option on).
#[test]
fn a_waypoint_made_with_a_tag_in_use_is_weighed() {
    let Some(World { mut h, field, second }) = world_with("setup-clash", true) else { return };
    h.state_mut().settings.warn_duplicate_tags = true;
    open_setup(&mut h, trigger_tab(field, 0), Some("second"));
    pick(&mut h, "TT_B2", 0);
    h.get_by_label_contains("Makes the waypoint WP_TT_B2 in second");
    ok(&mut h);
    assert_eq!(tags(&mut h, second, "WaypointList"), ["WP_FAR", "WP_TT_B2", "WP_TT_B2"]);
    let clash = h.state().tag_clash.clone().expect("the Duplicate Tags window");
    let listed: Vec<&str> = clash.clashes.iter().map(|c| c.tag.as_str()).collect();
    assert!(listed.contains(&"WP_TT_B2"), "{listed:?}");
}

/// Several objects edited together: Current To Target alone, said, each
/// given the link, the target's waypoint made once and the target left
/// as it was.
#[test]
fn several_objects_take_current_to_target_only() {
    let Some(World { mut h, field, second }) = world("setup-several") else { return };
    let paths =
        vec![GffPath::root().item("TriggerList", 0), GffPath::root().item("TriggerList", 1)];
    let b1 = object(&mut h, second, "TriggerList", 0);
    open_setup(&mut h, Tab::Instances { area: field, paths }, Some("second"));
    pick(&mut h, "TT_B1", 0);
    assert!(h.get_by_label("Both Ways").accesskit_node().is_disabled());
    assert!(h.get_by_label("Target To Current").accesskit_node().is_disabled());
    h.get_by_label_contains("can't be led back to");
    ok(&mut h);
    for i in 0..2 {
        assert_eq!(link(&object(&mut h, field, "TriggerList", i)), linked("WP_TT_B1", 2));
    }
    assert_eq!(object(&mut h, second, "TriggerList", 0), b1);
    assert_eq!(tags(&mut h, second, "WaypointList"), ["WP_FAR", "WP_TT_B1"]);
}

/// Current To Target and Target To Current between two triggers: one end
/// written each, the waypoint beside the one led to.
#[test]
fn one_way_connections_write_one_end() {
    let Some(World { mut h, field, second }) = world("setup-one-way") else { return };
    let tab = trigger_tab(field, 0);
    open_setup(&mut h, tab.clone(), Some("second"));
    pick(&mut h, "TT_B1", 0);
    connection(&mut h, "Target To Current");
    h.get_by_label_contains("Makes the waypoint WP_TT_A1 in field");
    assert!(h.query_by_label_contains("WP_TT_B1").is_none());
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("", 0), "unchanged");
    assert_eq!(link(&object(&mut h, second, "TriggerList", 0)), linked("WP_TT_A1", 2));
    assert_eq!(tags(&mut h, second, "WaypointList"), ["WP_FAR"]);
    h.get_by_label("Setup Area Transition…").click();
    settle(&mut h);
    show_target_area(&mut h, "second");
    pick(&mut h, "TT_B2", 0);
    connection(&mut h, "Current To Target");
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("WP_TT_B2", 2));
    assert_eq!(link(&object(&mut h, second, "TriggerList", 1)), linked("", 0), "unchanged");
    assert_eq!(tags(&mut h, second, "WaypointList"), ["WP_FAR", "WP_TT_B2"]);
}

/// The two areas of [`world`] in a project folder whose claims table
/// says `cd` holds `Second`, the builder being `ab`: Second is read-only
/// here.
fn read_only_world(name: &str) -> Option<World> {
    let World { h, field, second } = world(name)?;
    let mut m = h.state().ws.as_ref()?.module.clone();
    drop(h);
    let dir = mg_testkit::scratch_dir(&format!("ui-area-{name}-project"));
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let rules = "[claims]\nfile = \"claims.2da\"\nothers = \"read-only\"\n";
    std::fs::write(dir.join("moonglow.toml"), rules).unwrap();
    let table = format!(
        "2DA V2.0\n\n    Area             Builder  Status   Date        Tag\n\
         0   {:<16} cd       claimed  2026-10-20  ****\n",
        second.to_string()
    );
    std::fs::write(dir.join("claims.2da"), table).unwrap();
    mg_module::project_file::ProjectFile::set_builder_code(&dir, Some("ab")).unwrap();
    let root = mg_testkit::nwn_root()?;
    let rs = gpu()?;
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.settings.game_shaders = Some(false);
    app.open_module(&dir);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(field)));
    let mut h = area_gpu_harness(app, rs, 1100.0, 800.0);
    settle(&mut h);
    settle(&mut h);
    Some(World { h, field, second })
}

/// A read-only target area: a connection that would write there (a link
/// on its object, a waypoint in it) is refused with a line saying so,
/// OK grayed, nothing written; one that writes nothing there stays.
#[test]
fn a_read_only_target_area_takes_nothing() {
    let Some(World { mut h, field, second }) = read_only_world("setup-read-only") else { return };
    let areas = [field, second];
    let start = everything(&mut h, &areas);
    let tab = trigger_tab(field, 0);
    open_setup(&mut h, tab.clone(), Some("second"));
    // A trigger there: each connection writes there (its link, or its
    // waypoint).
    pick(&mut h, "TT_B1", 0);
    for c in ["Both Ways", "Current To Target", "Target To Current"] {
        connection(&mut h, c);
        h.get_by_label_contains("is read-only here");
        assert!(ok_button(&h).accesskit_node().is_disabled(), "{c}");
    }
    // A door there: Current To Target writes only here.
    pick(&mut h, "TT_DB", 0);
    connection(&mut h, "Both Ways");
    h.get_by_label_contains("is read-only here");
    assert!(ok_button(&h).accesskit_node().is_disabled());
    // (Enter, double clicks: nothing either.)
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(everything(&mut h, &areas), start, "nothing written");
    connection(&mut h, "Current To Target");
    assert!(h.query_by_label_contains("is read-only here").is_none());
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("TT_DB", 1));
    assert_eq!(list(&mut h, second, "Door List"), start[4], "the door unchanged");
    cancel_check(&mut h);
}

/// (Nothing else open: the window closed.)
fn cancel_check(h: &mut Harness<'_, Moonglow>) {
    assert!(h.query_by_label("The area's map").is_none());
}

/// The window, looked at (kept in `target/transition-triggers/window/`
/// with `MG_SHOT=1`):
/// a trigger chosen on the second area's map, with its lines (a waypoint
/// made, one used again, a link replaced), at the window's size as it
/// opens and dragged down to its least; at both, the lines and OK stand
/// under the map, inside the window.
#[test]
fn the_window_with_triggers_on_the_map() {
    let Some(World { mut h, field, .. }) = world("setup-look") else { return };
    show_area(&mut h, field);
    open_setup(&mut h, trigger_tab(field, 0), Some("second"));
    pick(&mut h, "TT_B1", 0);
    ok(&mut h);
    h.get_by_label("Setup Area Transition…").click();
    settle(&mut h);
    pick(&mut h, "TT_B2", 0);
    h.get_by_label_contains("OK replaces that link");
    settle(&mut h);
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/transition-triggers/window");
    if std::env::var_os("MG_SHOT").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
    }
    // The window: the smallest area shown that holds the map.
    let window = |h: &Harness<'_, Moonglow>| {
        let map = h.get_by_label("The area's map").rect();
        let screen = h.ctx.content_rect();
        h.ctx.memory(|m| {
            let layers: Vec<egui::LayerId> = m.areas().visible_layer_ids().into_iter().collect();
            layers
                .iter()
                .filter_map(|l| m.area_rect(l.id))
                .filter(|r| r.contains_rect(map) && *r != screen)
                .min_by(|a, b| a.area().total_cmp(&b.area()))
                .unwrap()
        })
    };
    let fits = |h: &Harness<'_, Moonglow>, size: &str| {
        let (map, win) = (h.get_by_label("The area's map").rect(), window(h));
        let lines = [
            h.get_by_label_contains("Leads to the trigger").rect(),
            h.get_by_label_contains("OK replaces that link").rect(),
            ok_button(h).rect(),
        ];
        for r in lines {
            assert!(r.top() >= map.bottom() - 0.5, "{size}: {r:?} under the map {map:?}");
            assert!(win.expand(1.0).contains_rect(r), "{size}: {r:?} inside {win:?}");
        }
    };
    // (Kept only when asked for, `MG_SHOT=1`: the folder is outside the
    // test output, shared by the runs.)
    let shot = std::env::var_os("MG_SHOT").is_some();
    if shot && let Ok(img) = h.render() {
        img.save(dir.join("setup-area-transition.png")).unwrap();
    }
    fits(&h, "as it opens");
    // Dragged by its corner up and left, as far as it goes.
    let corner = window(&h).right_bottom() - egui::vec2(3.0, 3.0);
    h.hover_at(corner);
    settle(&mut h);
    press(&h, corner, true, egui::Modifiers::NONE);
    for i in 1..=10 {
        let at = corner - egui::vec2(50.0, 40.0) * i as f32;
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
    }
    press(&h, corner - egui::vec2(500.0, 400.0), false, egui::Modifiers::NONE);
    settle(&mut h);
    let small = window(&h);
    assert!(small.height() < 640.0 && small.width() < 760.0, "made smaller: {small:?}");
    if shot && let Ok(img) = h.render() {
        img.save(dir.join("setup-area-transition-least.png")).unwrap();
    }
    fits(&h, "at its least");
    cancel(&mut h);
}

/// A blueprint leads to a trigger: Current To Target alone, the
/// blueprint given the link to the trigger's waypoint, which is made; the
/// trigger left as it was.
#[test]
fn a_blueprint_leads_to_a_trigger_s_waypoint() {
    let Some(World { mut h, second, .. }) = world("setup-blueprint") else { return };
    let key = ResKey::parse("transit", ResType::UTT).unwrap();
    let b1 = object(&mut h, second, "TriggerList", 0);
    open_setup(&mut h, Tab::Blueprint(key), Some("second"));
    pick(&mut h, "TT_B1", 0);
    assert!(h.get_by_label("Both Ways").accesskit_node().is_disabled());
    h.get_by_label_contains("Makes the waypoint WP_TT_B1 in second");
    ok(&mut h);
    let root = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().root.clone();
    assert_eq!(link(&root), linked("WP_TT_B1", 2));
    assert_eq!(object(&mut h, second, "TriggerList", 0), b1);
    assert_eq!(tags(&mut h, second, "WaypointList"), ["WP_FAR", "WP_TT_B1"]);
}

/// The tags that name a link: this object's still its blueprint's is said
/// where the target is to lead back to it; without a tag, nothing can
/// lead back to it (Both Ways and Target To Current grayed, why said).
#[test]
fn this_object_s_tag_is_weighed_where_it_names_a_link() {
    let Some(World { mut h, field, .. }) = world("setup-own-tag") else { return };
    let retag = |h: &mut Harness<'_, Moonglow>, tag: &str| {
        let edit = mg_edit::Edit::SetField {
            key: git(field),
            path: GffPath::root().item("TriggerList", 0),
            label: "Tag".into(),
            value: Some(text(tag)),
        };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
        settle(h);
    };
    // Its blueprint's (newtransition's).
    retag(&mut h, "AreaTransition");
    let tab = trigger_tab(field, 0);
    open_setup(&mut h, tab.clone(), Some("second"));
    pick(&mut h, "TT_B1", 0);
    h.get_by_label_contains("This object's tag (AreaTransition) is still its blueprint's");
    assert!(h.query_by_label_contains("The target's tag").is_none(), "TT_B1 is its own");
    connection(&mut h, "Current To Target");
    assert!(h.query_by_label_contains("This object's tag").is_none(), "names no link");
    cancel(&mut h);
    close(&mut h, tab.clone());
    // None at all.
    retag(&mut h, "");
    open_setup(&mut h, tab, Some("second"));
    pick(&mut h, "TT_B1", 0);
    assert!(h.get_by_label("Both Ways").accesskit_node().is_disabled());
    assert!(h.get_by_label("Target To Current").accesskit_node().is_disabled());
    h.get_by_label("This object has no tag for the target to lead back to");
    ok(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("WP_TT_B1", 2));
}

/// The Properties window's Cancel after the transition is set up, where
/// the window had changed the link before: the transition stays whole
/// (set back on this end alone, the target would lead to an object that
/// leads elsewhere).
#[test]
fn the_properties_cancel_leaves_the_transition_whole() {
    let Some(World { mut h, field, second }) = world("setup-cancel") else { return };
    let areas = [field, second];
    let tab = trigger_tab(field, 0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(&mut h);
    h.get_all_by_label("Area Transition").next().unwrap().click();
    settle(&mut h);
    // Destination is a Door, changed in the window first.
    let door = h
        .get_all_by_label("Door")
        .find(|n| n.accesskit_node().role() == Role::RadioButton)
        .expect("Destination is a Door");
    door.click();
    settle(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("", 1));
    h.get_by_label("Setup Area Transition…").click();
    settle(&mut h);
    show_target_area(&mut h, "second");
    pick(&mut h, "TT_B1", 0);
    ok(&mut h);
    let set_up = everything(&mut h, &areas);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("WP_TT_B1", 2));
    // The Properties' Cancel (the only one left).
    h.get_all_by_label("Cancel").last().unwrap().click();
    settle(&mut h);
    assert!(h.query_by_label("Setup Area Transition…").is_none(), "the window closed");
    assert_eq!(everything(&mut h, &areas), set_up, "left whole");
    // Undo takes it back whole, to the Door set before it.
    undo(&mut h);
    assert_eq!(link(&object(&mut h, field, "TriggerList", 0)), linked("", 1));
    assert_eq!(link(&object(&mut h, second, "TriggerList", 0)), linked("", 0));
    assert_eq!(tags(&mut h, second, "WaypointList"), ["WP_FAR"]);
}
