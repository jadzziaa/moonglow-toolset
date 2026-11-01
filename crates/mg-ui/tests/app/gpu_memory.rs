//! What the window asks of the GPU: the 3D views' targets and scenes let
//! go of when they are not shown, and no frames behind another window.

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

/// How soon the window asks to be drawn again after its last frame.
fn again(h: &Harness<'_, Moonglow>) -> std::time::Duration {
    let out = &h.output().viewport_output;
    out.get(&egui::ViewportId::ROOT).map_or(std::time::Duration::MAX, |v| v.repaint_delay)
}

/// An area's view steps its animations 25 times a second while the window
/// has the focus; behind another program's window it asks for no frame
/// (its picture kept as it is), and steps again once it has the focus.
/// With Keep animating behind other windows, it goes on there too.
#[test]
fn an_area_s_animations_wait_while_the_window_has_not_the_focus() {
    let Some((mut h, _)) = area_harness("unfocused-area") else { return };
    settle(&mut h);
    h.run_steps(3);
    assert!(again(&h) <= std::time::Duration::from_millis(40), "focused: {:?}", again(&h));
    h.input_mut().focused = false;
    h.run_steps(3);
    assert!(again(&h) > std::time::Duration::from_secs(1), "in the background: {:?}", again(&h));
    h.input_mut().focused = true;
    h.run_steps(3);
    assert!(again(&h) <= std::time::Duration::from_millis(40), "focused again: {:?}", again(&h));
    // Options › Area › Keep animating behind other windows: they go on.
    h.state_mut().settings.animate_unfocused = true;
    h.input_mut().focused = false;
    h.run_steps(3);
    assert!(again(&h) <= std::time::Duration::from_millis(40), "kept going: {:?}", again(&h));
}

/// Options › Area › Largest textures: the area's textures read again at
/// most so large a side, taking less of the GPU's memory, and its picture
/// drawn anew with them.
#[test]
fn the_largest_textures_option_reads_the_textures_again_smaller() {
    let Some((mut h, area)) = area_harness("texture-limit") else { return };
    settle(&mut h);
    let memory =
        |h: &Harness<'_, Moonglow>| h.state().viewport.as_ref().unwrap().renderer.texture_memory();
    let (before, pictures) = (memory(&h), h.state().area_views[&area].pictures);
    assert!(before.bytes > 0, "{before:?}");
    h.state_mut().settings.texture_limit = Some(64);
    settle(&mut h);
    h.run_steps(3);
    assert_eq!(h.state().viewport.as_ref().unwrap().gpu.texture_limit(), Some(64));
    let after = memory(&h);
    assert!(after.bytes > 0 && after.bytes < before.bytes / 2, "{after:?} of {before:?}");
    assert!(h.state().area_views[&area].pictures > pictures, "drawn again");
    h.state_mut().settings.texture_limit = None;
    settle(&mut h);
    h.run_steps(3);
    assert_eq!(memory(&h).bytes, before.bytes, "as they were");
}

/// Options › Area › Area view resolution: the area's picture drawn at that
/// share of the view's pixels each way (shown stretched over the same
/// view), and at all of them again once Full is chosen.
#[test]
fn the_area_view_resolution_draws_the_picture_at_a_share_of_its_pixels() {
    let Some((mut h, area)) = area_harness("area-resolution") else { return };
    settle(&mut h);
    let size = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].picture_size().unwrap();
    let (rect, (w, hh)) = (h.state().area_views[&area].rect, size(&h));
    h.state_mut().settings.area_resolution = Some(50);
    h.run_steps(3);
    let (w2, h2) = size(&h);
    assert!(w2.abs_diff(w / 2) <= 1 && h2.abs_diff(hh / 2) <= 1, "{w2}x{h2} of {w}x{hh}");
    assert_eq!(h.state().area_views[&area].rect, rect, "over the same view");
    h.state_mut().settings.area_resolution = None;
    h.run_steps(3);
    assert_eq!(size(&h), (w, hh));
}

/// The model viewer playing an animation asks for frames while the window
/// has the focus, and none behind another program's window.
#[test]
fn a_playing_model_waits_while_the_window_has_not_the_focus() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    app.settings.game_shaders = Some(false);
    let key = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = gpu_harness(app, rs, 900.0, 700.0);
    h.run_steps(3);
    let source = mg_ui::model_view::Source::Resource(key);
    let view = h.state_mut().model_views.get_mut(&source).unwrap();
    (view.animation, view.playing) = (Some("open".into()), true);
    h.run_steps(3);
    assert!(again(&h) <= std::time::Duration::from_millis(40), "playing: {:?}", again(&h));
    h.input_mut().focused = false;
    h.run_steps(3);
    assert!(again(&h) > std::time::Duration::from_secs(1), "in the background: {:?}", again(&h));
}
