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
