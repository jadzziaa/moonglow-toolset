//! Tileset authoring against the game's tilesets:
//!
//! - every `.set` the game ships, read as a `SetFile` and written back
//!   untouched, is the same bytes; a value changed is that line alone, and
//!   the tileset read again differs by that value alone;
//! - the palette generated from each set (`mg_module::tileset::palette`)
//!   holds what the game's own `<tileset>palstd.itp` does: its groups and
//!   features by their first tile's model, its terrains and crossers. The
//!   game's are arranged by hand, and leave a few groups out; those are
//!   counted, not failed.

use std::collections::BTreeSet;

use mg_core::{Codepage, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_set::Tileset;
use mg_set::edit::SetFile;
use mg_testkit::corpus;

/// A palette's leaves (`RESREF`s), lower case, anywhere in its tree, as
/// terrain leaves (under ID 2) and the others.
fn leaves(gff: &Gff) -> (BTreeSet<String>, BTreeSet<String>) {
    fn walk(s: &Struct, terrain: bool, out: &mut (BTreeSet<String>, BTreeSet<String>)) {
        let terrain = terrain || matches!(s.get("ID"), Some(Value::Byte(2)));
        if let Some(r) = s.resref("RESREF") {
            let name = String::from_utf8_lossy(r.as_bytes()).to_lowercase();
            if terrain {
                out.0.insert(name)
            } else {
                out.1.insert(name)
            };
        }
        for label in ["MAIN", "LIST"] {
            if let Some(Value::List(items)) = s.get(label) {
                for i in items {
                    walk(i, terrain, out);
                }
            }
        }
    }
    let mut out = Default::default();
    walk(&gff.root, false, &mut out);
    out
}

#[test]
fn shipped_tilesets_round_trip_and_palettes_hold_the_same() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let (mut sets, mut compared) = (0, 0);
    let (mut shipped_total, mut found, mut extra) = (0, 0, 0);
    let mut failures = Vec::new();
    for name in rm.list(ResType::SET) {
        let data = rm.get(&ResKey::new(name, ResType::SET)).unwrap();
        let text = Codepage::default().decode(&data).into_owned();
        let mut f = SetFile::parse(&text);
        if f.text() != text {
            failures.push(format!("{name}.set: not the same bytes written back"));
            continue;
        }
        let Ok(before) = Tileset::parse(&data, Codepage::default()) else { continue };
        sets += 1;
        // One value changed: that line, and that field.
        f.set("GENERAL", "Transition", "7.25");
        let changed = f.text();
        let differ = text.lines().zip(changed.lines()).filter(|(a, b)| a != b).count();
        if differ != 1 || text.lines().count() != changed.lines().count() {
            failures.push(format!("{name}.set: {differ} lines changed for one value"));
        }
        let after = Tileset::parse(changed.as_bytes(), Codepage::default()).unwrap();
        let mut expected = before.clone();
        expected.general.transition = 7.25;
        expected.ini = after.ini.clone();
        if after != expected {
            failures.push(format!("{name}.set: more than Transition changed"));
        }

        // The palette.
        let pal = ResKey::parse(&format!("{name}palstd"), ResType::ITP).unwrap();
        let Ok(shipped) = rm.get(&pal) else { continue };
        let Ok(shipped) = Gff::read(&shipped) else { continue };
        compared += 1;
        let (theirs_terrain, theirs) = leaves(&shipped);
        let (ours_terrain, ours) = leaves(&mg_module::tileset::palette(&before).0);
        // Terrains: the same, but for the special leaves a hand-made
        // palette may leave out or add.
        let special = |s: &&String| *s != "eraser" && *s != "raiselower";
        let t1: BTreeSet<_> = theirs_terrain.iter().filter(special).collect();
        let t2: BTreeSet<_> = ours_terrain.iter().filter(special).collect();
        if !t1.is_subset(&t2) {
            failures.push(format!(
                "{name}: terrains {:?} missing",
                t1.difference(&t2).collect::<Vec<_>>()
            ));
        }
        // Groups and features: every one the game's palette places that is
        // a group's first tile.
        let firsts: BTreeSet<String> = before
            .groups
            .iter()
            .filter_map(|g| g.tiles.first().copied().flatten())
            .filter_map(|t| before.tiles.get(t as usize))
            .map(|t| t.model.to_lowercase())
            .collect();
        let placed: BTreeSet<&String> = theirs.iter().filter(|m| firsts.contains(*m)).collect();
        shipped_total += placed.len();
        found += placed.iter().filter(|m| ours.contains(**m)).count();
        extra += ours.iter().filter(|m| !theirs.contains(*m)).count();
        let missing: Vec<_> = placed.iter().filter(|m| !ours.contains(**m)).collect();
        if !missing.is_empty() {
            failures.push(format!("{name}: groups {missing:?} missing"));
        }
    }
    println!(
        "{sets} sets; {compared} palettes: {found} of {shipped_total} placed groups generated, \
         {extra} groups the game's palettes leave out"
    );
    assert!(sets > 30 && compared > 30, "{sets} {compared}");
    failures.truncate(30);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
