//! What the window holds on the GPU: the 3D views' targets and scenes let
//! go of when they are not shown.

use crate::common::*;

/// Frames enough for a view to go unseen past
/// `model_view::UNSEEN_AFTER` (2 s) at the harness's 60 a second.
const PAST_UNSEEN: usize = 150;

/// An area's tab closed: its view lets go of its picture's targets and of
/// its scene (its models' meshes on the GPU) once unseen, and keeps its
/// camera; opened again, it is read and drawn anew where the camera was.
#[test]
fn a_closed_area_lets_go_of_its_targets_and_scene() {
    let Some((mut h, area)) = area_harness("closed-area-memory") else { return };
    settle(&mut h);
    let view = |h: &Harness<'_, Moonglow>| {
        let v = &h.state().area_views[&area];
        (v.holds_targets(), v.holds_scene())
    };
    assert_eq!(view(&h), (true, true), "drawn");
    let camera = h.state().area_views[&area].orbit;
    close_tab(h.state_mut(), &Tab::Area(area));
    h.run_steps(PAST_UNSEEN);
    assert_eq!(view(&h), (false, false), "closed: neither kept");
    assert_eq!(h.state().area_views[&area].orbit, camera, "the camera kept");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    settle(&mut h);
    assert_eq!(view(&h), (true, true), "opened again, drawn again");
    assert_eq!(h.state().area_views[&area].orbit, camera, "where it was");
}

/// An area's tab behind another: its view lets go of its targets once
/// unseen (drawn again into new ones when it is in front), and keeps its
/// scene, so that it is shown again at once.
#[test]
fn an_area_behind_another_tab_lets_go_of_its_targets_only() {
    let Some((mut h, area)) = area_harness("hidden-area-memory") else { return };
    settle(&mut h);
    let pictures = h.state().area_views[&area].pictures;
    // (The journal in front of it, in its pane.)
    let app = h.state_mut();
    let at = app.dock.find_tab(&Tab::Area(area)).unwrap();
    app.dock
        .set_focused_node_and_surface(egui_dock::NodePath { surface: at.surface, node: at.node });
    app.dock.push_to_focused_leaf(Tab::Journal);
    h.run_steps(PAST_UNSEEN);
    let v = &h.state().area_views[&area];
    assert!(!v.holds_targets(), "its targets let go of");
    assert!(v.holds_scene(), "its scene kept");
    let at = h.state().dock.find_tab(&Tab::Area(area)).unwrap();
    h.state_mut().dock.set_active_tab(at).expect("its tab");
    h.run_steps(3);
    let v = &h.state().area_views[&area];
    assert!(v.holds_targets() && v.pictures > pictures, "drawn again at once");
}
