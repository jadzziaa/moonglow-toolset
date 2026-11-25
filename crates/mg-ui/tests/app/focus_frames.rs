//! Focus Selection (F, '.') frames what is drawn of the selection: the
//! middle of its models' boxes, not their pivots (their feet).

use crate::common::*;
use glam::Vec3;
use mg_area::ObjectKind;

/// Where the tree stands.
const TREE: Vec3 = Vec3::new(25.0, 25.0, 0.0);

/// [`area_harness`]'s area with a tree (`plc_tree`) at [`TREE`].
fn tree_harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let prepare = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        use mg_module::instances::{Placement, Placing, instance};
        let key = ResKey::parse("plc_tree", ResType::UTP).unwrap();
        let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: TREE.to_array(), rotation: 0.0 };
        let tree = instance(&placing, ResType::UTP, &bp, at, &[]).unwrap();
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        git.root.set("Placeable List", mg_gff::Value::List(vec![tree]));
        m.set_gff(git_key, &git).unwrap();
    };
    let user = std::env::var_os("MG_TEST_USER").map(std::path::PathBuf::from);
    area_harness_with(name, "ttr01", None, 4, user, &prepare)
}

/// The area's view alone, written to `target/test-output/focus-frames/`
/// with `MG_SHOT` set (to look at, before and after a change).
fn picture(h: &mut Harness<'_, Moonglow>, area: ResRef, name: &str) {
    if std::env::var_os("MG_SHOT").is_none() {
        return;
    }
    settle(h);
    let rect = h.state().area_views[&area].rect;
    let Ok(img) = h.render() else { return };
    let ppp = h.ctx.pixels_per_point();
    let out = mg_testkit::output_dir("focus-frames");
    crop_rect(&img, rect, ppp, 0.0).save(out.join(name)).unwrap();
}

/// The tree selected, F from close by and looking along the ground.
fn focus_tree(h: &mut Harness<'_, Moonglow>, area: ResRef) {
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        let o = view.orbit.as_mut().unwrap();
        o.target = Vec3::new(10.0, 10.0, 0.0);
        o.distance = 6.0;
        o.pitch = 0.3;
        view.selection = vec![(ObjectKind::Placeable, 0)];
    }
    settle(h);
    h.hover_at(h.state().area_views[&area].rect.center());
    h.run_steps(2);
    h.key_press(egui::Key::F);
    settle(h);
}

/// F on a tree frames the middle of its model's box, as the selection
/// box shows it (its trunk's foot is its position), from far enough to
/// see all of it; F on the tree and a waypoint (an arrow: no model of
/// its own, framed by its position) frames the box round both.
#[test]
fn focus_frames_the_model_not_the_pivot() {
    let Some((mut h, area)) = tree_harness("focus-frames") else { return };
    // (Its model loaded: the box the view selects it by.)
    until(&mut h, "the tree's model", |h| {
        let v = &h.state().area_views[&area];
        let i = v
            .model
            .as_ref()
            .and_then(|m| m.objects.iter().position(|o| o.kind == ObjectKind::Placeable));
        i.and_then(|i| v.box_of(i)).is_some_and(|(min, max)| max.z - min.z > 2.0)
    });
    let v = &h.state().area_views[&area];
    let i = v.model.as_ref().unwrap().objects.iter().position(|o| o.kind == ObjectKind::Placeable);
    let (min, max) = v.box_of(i.unwrap()).unwrap();
    let middle = TREE + (min + max) * 0.5;
    let half = ((max - min) * 0.5).length();
    focus_tree(&mut h, area);
    let o = h.state().area_views[&area].orbit.unwrap();
    assert!((o.target - middle).length() < 0.05, "the box's middle {middle}, not {}", o.target);
    assert!(o.target.z > 1.0, "above its foot: {}", o.target);
    assert!(o.distance >= half * 2.4 - 1e-3, "all of it seen: {} for {half}", o.distance);
    assert!(o.distance <= 25.0, "not far off: {}", o.distance);
    picture(&mut h, area, "tree.png");
    // The tree and the waypoint at (20, 20): the middle of both.
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(ObjectKind::Placeable, 0), (ObjectKind::Waypoint, 0)];
    }
    h.key_press(egui::Key::F);
    settle(&mut h);
    // (The waypoint's flag a little above where it stands.)
    let o = h.state().area_views[&area].orbit.unwrap();
    let low = (TREE + min).min(Vec3::new(20.0, 20.0, 0.0));
    let high = (TREE + max).max(Vec3::new(20.0, 20.0, 0.0));
    assert!((o.target - (low + high) * 0.5).truncate().length() < 0.6, "both: {}", o.target);
    assert!((o.target.z - (low.z + high.z) * 0.5).abs() < 0.05, "both: {}", o.target);
}
