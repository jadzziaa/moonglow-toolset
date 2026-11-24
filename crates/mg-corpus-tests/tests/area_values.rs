//! Every shipped area built from values (`mg_area::values`, what a game
//! client has of an area) is the area read from its files: the reading
//! goes through the values, and nothing of it goes round them.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use mg_area::AreaModel;
use mg_area::values::Area;
use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_resman::ResKey;
use mg_set::Tileset;
use mg_testkit::corpus;

use crate::area_common::each_module;

/// Each area's model read from its ARE and GIT, and built from the values
/// read of them, are one (compared as printed: a position that is no
/// number is no number in both). With `AREA_DUMP` set to a file, each
/// model hashed is written there, to hold a change of the reading to the
/// models it made before (a whole install's areas were the same before
/// and after the reading went through the values).
#[test]
fn every_shipped_area_is_its_values() {
    let root = corpus!();
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    let (mut areas, mut objects) = (0, 0);
    each_module(&root, |opened| {
        let (name, m, game) = (&opened.name, &opened.module, &opened.game);
        let mut tilesets: HashMap<ResRef, Option<Tileset>> = HashMap::new();
        for area in m.areas().unwrap() {
            let gff = |t: ResType| {
                let data = game.resman.get(&ResKey::new(area, t)).ok()?;
                Gff::read(&data).ok()
            };
            let (Some(are), Some(git)) = (gff(ResType::ARE), gff(ResType::GIT)) else { continue };
            let tileset = are.root.resref("Tileset").and_then(|r| {
                tilesets.entry(r).or_insert_with(|| mg_area::tileset(game, r).ok()).clone()
            });
            let read =
                format!("{:?}", AreaModel::read(game, &are.root, &git.root, tileset.as_ref()));
            let values = Area::read(game, &are.root, &git.root);
            objects += values.objects.len();
            let built = format!("{:?}", AreaModel::from_values(game, values, tileset.as_ref()));
            areas += 1;
            if built != read {
                failures.push(format!("{name}/{area}"));
            }
            let mut h = std::collections::hash_map::DefaultHasher::new();
            read.hash(&mut h);
            lines.push(format!("{name}/{area} {:016x}", h.finish()));
        }
    });
    eprintln!("{areas} areas, {objects} objects");
    if let Some(out) = std::env::var_os("AREA_DUMP") {
        lines.sort();
        std::fs::write(out, lines.join("\n")).unwrap();
    }
    assert!(areas > mg_testkit::at_least(1000, 6), "only {areas} areas");
    assert!(failures.is_empty(), "built otherwise than read: {failures:?}");
}

/// What a game client's frame costs on the shipped area of the most
/// objects (of the modules swept) when a creature walks, changes its look,
/// and an object comes and goes, against showing the area anew. Run by
/// hand: `cargo test -p mg-corpus-tests --release --test areas
/// a_creature_walking -- --ignored --nocapture`.
#[test]
#[ignore = "timed by hand, in release"]
fn a_creature_walking_on_a_large_area_costs_little() {
    use mg_area::ObjectKind;
    let root = corpus!();
    mg_testkit::gpu::hold();
    static BASE: std::sync::OnceLock<Option<mg_render::Gpu>> = std::sync::OnceLock::new();
    let Some(gpu) = BASE
        .get_or_init(mg_render::Gpu::headless)
        .as_ref()
        .map(|g| mg_render::Gpu::new(g.device.clone(), g.queue.clone()))
    else {
        eprintln!("skipped: no GPU");
        return;
    };
    // (The area of the most objects, then its module opened again to time it.)
    let read = |game: &mg_rules::GameData, area: ResRef| {
        let gff = |t: ResType| {
            let data = game.resman.get(&ResKey::new(area, t)).ok()?;
            Gff::read(&data).ok()
        };
        Some((gff(ResType::ARE)?, gff(ResType::GIT)?))
    };
    let mut largest: Option<(usize, String, ResRef)> = None;
    each_module(&root, |opened| {
        for area in opened.module.areas().unwrap() {
            let Some((_, git)) = read(&opened.game, area) else { continue };
            let list = |k: &ObjectKind| git.root.list(k.list()).map_or(0, <[_]>::len);
            let count = ObjectKind::ALL.iter().map(list).sum::<usize>();
            let creatures = list(&ObjectKind::Creature);
            if creatures > 0 && largest.as_ref().is_none_or(|l| l.0 < count) {
                largest = Some((count, opened.name.clone(), area));
            }
        }
    });
    let Some((_, module, resref)) = largest else { panic!("no area with a creature") };
    each_module(&root, |opened| {
        if opened.name != module {
            return;
        }
        let game = &opened.game;
        let (are, git) = read(game, resref).unwrap();
        let set = are.root.resref("Tileset").and_then(|r| mg_area::tileset(game, r).ok());
        let area = AreaModel::read(game, &are.root, &git.root, set.as_ref());
        time_a_client_s_frames(&gpu, game, area, &format!("{module}/{resref}"));
    });
}

fn time_a_client_s_frames(
    gpu: &mg_render::Gpu,
    game: &mg_rules::GameData,
    mut area: AreaModel,
    at: &str,
) {
    use std::time::{Duration, Instant};

    use mg_area::{AreaScene, ObjectKind, View};
    let t = Instant::now();
    let mut loaded = AreaScene::new(gpu, game, &area);
    let built = t.elapsed();
    let view = View { fog: true, ..View::of(&area) };
    let walker = area.objects.iter().position(|o| o.kind == ObjectKind::Creature).unwrap();
    let start = area.objects[walker].position;
    let looks: Vec<_> = area.objects.iter().filter_map(|o| o.preview.clone()).take(2).collect();
    let median = |mut took: Vec<Duration>| {
        took.sort();
        took[took.len() / 2].as_secs_f64() * 1000.0
    };
    let (mut scenes, mut changed, mut added, mut removed) = (vec![], vec![], vec![], vec![]);
    for f in 0..100 {
        area.objects[walker].move_to(start + glam::Vec3::X * (f as f32 * 0.02), 0.0);
        let t = Instant::now();
        let scene = loaded.scene(&area, &view);
        scenes.push(t.elapsed());
        assert!(!scene.instances.is_empty());
        area.objects[walker].preview = Some(looks[f % looks.len()].clone());
        let t = Instant::now();
        loaded.update_objects(gpu, game, &area, &[walker]);
        changed.push(t.elapsed());
        let mut o = area.objects[walker].clone();
        o.index = 1 << 20;
        area.objects.push(o);
        let t = Instant::now();
        loaded.update_objects(gpu, game, &area, &[]);
        added.push(t.elapsed());
        let last = area.objects.len() - 1;
        area.objects.remove(last);
        let t = Instant::now();
        loaded.remove_object(&area, last);
        removed.push(t.elapsed());
    }
    let t = Instant::now();
    loaded.update(gpu, game, &area);
    let whole = t.elapsed();
    eprintln!(
        "{at}: {} tiles, {} objects; built {:.0} ms. A frame's scene {:.2} ms (the creature \
         moved: no update); a look changed (update_objects) {:.3} ms; an object added \
         (update_objects) {:.3} ms, taken out (remove_object) {:.3} ms; the area shown anew \
         (update) {:.2} ms",
        area.tiles.len(),
        area.objects.len(),
        built.as_secs_f64() * 1000.0,
        median(scenes),
        median(changed),
        median(added),
        median(removed),
        whole.as_secs_f64() * 1000.0
    );
}
