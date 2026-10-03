//! Corpus tests for tilesets (SET) and soundsets (SSF).

use mg_core::{Codepage, ResType};
use mg_erf::Erf;
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_set::Tileset;
use mg_ssf::Ssf;
use mg_testkit::{bundled_haks, corpus};

/// Every resource of a type in the base game and the shipped haks.
fn shipped(root: &std::path::Path, restype: ResType) -> Vec<(String, Vec<u8>)> {
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let mut out: Vec<(String, Vec<u8>)> = rm
        .list(restype)
        .into_iter()
        .map(|r| {
            let key = ResKey::new(r, restype);
            (format!("base:{key}"), rm.get(&key).unwrap().into_owned())
        })
        .collect();
    for hak in bundled_haks(root) {
        let data = std::fs::read(&hak).unwrap();
        let erf = Erf::read(&data).unwrap();
        for e in erf.entries.iter().filter(|e| e.restype == restype) {
            out.push((
                format!("{}:{}", hak.display(), e.filename()),
                erf.data(e).unwrap().into_owned(),
            ));
        }
    }
    out
}

/// Tile models the shipped tilesets reference but the game does not ship
/// (checked case-insensitively across the keys and every shipped hak).
const KNOWN_MISSING_MODELS: &[&str] =
    &["tcm02_b92_02", "trs02_m00_00", "tcm02_f17_14", "tcm02_f27_14", "tcn01_a20_01b"];

/// Every shipped tileset parses, and every tile model exists in the base game
/// or a shipped hak (premium modules split tilesets across haks), apart from
/// the known holes above. Data inconsistencies in shipped tilesets are
/// reported as warnings, not errors.
#[test]
fn shipped_tilesets_parse_and_their_models_exist() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let hak_data: Vec<Vec<u8>> =
        bundled_haks(&root).iter().map(|h| std::fs::read(h).unwrap()).collect();
    let haks: Vec<Erf> = hak_data.iter().map(|d| Erf::read(d).unwrap()).collect();
    let sets = shipped(&root, ResType::SET);
    assert!(sets.len() > 30);
    let mut failures = Vec::new();
    let (mut rules, mut warnings) = (0, 0);
    for (name, bytes) in &sets {
        let t = match Tileset::parse(bytes, Codepage::WINDOWS_1252) {
            Ok(t) => t,
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        rules += t.primary_rules.len();
        warnings += t.warnings.len();
        for (i, tile) in t.tiles.iter().enumerate() {
            let key = ResKey::parse(&tile.model, ResType::MDL).unwrap();
            let known = KNOWN_MISSING_MODELS.iter().any(|m| m.eq_ignore_ascii_case(&tile.model));
            if !known
                && !rm.contains(&key)
                && !haks.iter().any(|h| h.find(&key.resref, key.restype).is_some())
            {
                failures.push(format!("{name}: tile {i} model {} missing", tile.model));
            }
        }
    }
    eprintln!("checked {} tilesets ({rules} primary rules, {warnings} data warnings)", sets.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// Custom tilesets have `Doors=` counts that are no counts (1869573190:
/// whatever was in their editor's memory), which the game shrugs off. Every
/// shipped tileset given such counts reads as fast, with the doors its
/// sections describe.
#[test]
fn tilesets_with_door_counts_that_are_no_counts_read_the_same() {
    let root = corpus!();
    let sets = shipped(&root, ResType::SET);
    let started = std::time::Instant::now();
    let mut doors = 0;
    for (name, bytes) in &sets {
        let text = String::from_utf8_lossy(bytes);
        let spoiled: String = text
            .split_inclusive('\n')
            .map(|l| {
                if l.trim_start().to_ascii_lowercase().starts_with("doors=") {
                    "Doors=1869573190\r\n"
                } else {
                    l
                }
            })
            .collect();
        let (Ok(plain), Ok(spoiled)) = (
            Tileset::parse(bytes, Codepage::WINDOWS_1252),
            Tileset::parse(spoiled.as_bytes(), Codepage::WINDOWS_1252),
        ) else {
            panic!("{name} does not parse");
        };
        for (i, (a, b)) in plain.tiles.iter().zip(&spoiled.tiles).enumerate() {
            // (All of a tile's door sections: those within its count first.)
            assert!(b.doors.starts_with(&a.doors), "{name}: tile {i}");
            doors += b.doors.len();
        }
        assert!(spoiled.warnings.len() <= plain.warnings.len() + spoiled.tiles.len(), "{name}");
    }
    eprintln!("{} tilesets, {doors} doors, {:?}", sets.len(), started.elapsed());
    assert!(doors > 1000);
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
}

/// Every shipped soundset parses and writes back byte for byte.
#[test]
fn shipped_soundsets_round_trip_exactly() {
    let root = corpus!();
    let ssfs = shipped(&root, ResType::SSF);
    assert!(ssfs.len() > 400);
    let failures: Vec<String> = ssfs
        .iter()
        .filter_map(|(name, bytes)| match Ssf::read(bytes) {
            Err(e) => Some(format!("{name}: {e}")),
            Ok(s) if s.to_bytes().unwrap() != *bytes => Some(format!("{name}: rewrite differs")),
            Ok(_) => None,
        })
        .collect();
    eprintln!("checked {} soundsets", ssfs.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
