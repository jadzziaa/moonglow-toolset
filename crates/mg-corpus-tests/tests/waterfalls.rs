//! The waterfalls of Medieval Rural: their emitters point down the fall
//! and keep their particles in their own space; the water falls.

use glam::{Mat4, Vec3};
use mg_core::ResType;
use mg_mdl::{Model, NodeKind};
use mg_render::particles::Particles;
use mg_resman::{GameInstall, ResMan};
use mg_testkit::corpus;

#[test]
fn a_waterfall_s_water_falls() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    for name in ["ttr01_g07_01", "ttr01_g09_01", "ttr01_g13_01"] {
        let mut model = Model::read(&rm.get_named(name, ResType::MDL).unwrap()).unwrap();
        let pose = mg_render::rest_pose(&model);
        // (The emitters at the top of the fall alone: not the spray at
        // its foot.)
        for (i, n) in model.nodes.iter_mut().enumerate() {
            if matches!(n.kind, NodeKind::Emitter(_)) && pose[i].w_axis.z < 2.0 {
                n.kind = NodeKind::Dummy;
            }
        }
        let mut p = Particles::new(&model);
        for _ in 0..50 {
            p.update(&model, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        }
        let eye = Mat4::look_at_rh(Vec3::new(0.0, -20.0, 5.0), Vec3::ZERO, Vec3::Z);
        let batches = p.batches(&model, None, 0.0, &pose, Mat4::IDENTITY, eye);
        // The water's emitters (heavy, pointing out over the edge), and
        // how far their particles are from them: down, and back upstream.
        let water = |i: &usize| matches!(&model.nodes[*i].kind, NodeKind::Emitter(e) if e.texture.as_deref().is_some_and(|t| t.eq_ignore_ascii_case("fxpa_splash")));
        let at: Vec<Vec3> =
            (0..model.nodes.len()).filter(water).map(|i| pose[i].w_axis.truncate()).collect();
        let out: Vec<Vec3> = (0..model.nodes.len())
            .filter(water)
            .map(|i| pose[i].transform_vector3(Vec3::Z).normalize())
            .collect();
        assert!(!at.is_empty(), "{name}: no water");
        let top = at.iter().map(|p| p.z).fold(f32::MIN, f32::max);
        let splash = (batches.iter()).filter(|b| {
            b.texture.as_deref().is_some_and(|t| t.eq_ignore_ascii_case("fxpa_splash"))
        });
        let seen: Vec<Vec3> =
            splash.flat_map(|b| b.vertices.iter().map(|v| Vec3::from(v.pos))).collect();
        let low = seen.iter().map(|p| p.z).fold(f32::MAX, f32::min);
        // The furthest any particle is behind its emitters (against the
        // way they point): water pulled along the emitter's own axis ran
        // back up the stream.
        let behind = (seen.iter())
            .map(|p| at.iter().zip(&out).map(|(a, o)| (*a - *p).dot(*o)).fold(f32::MAX, f32::min))
            .fold(f32::MIN, f32::max);
        eprintln!("{name}: water from {top:.1} down to {low:.1}, {behind:.1} behind");
        assert!(low < top - 5.0 && behind < 2.5, "{name}");
    }
}
