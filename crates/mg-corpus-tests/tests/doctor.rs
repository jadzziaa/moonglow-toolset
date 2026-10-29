//! The content doctor on real content: every shipped module with its haks,
//! the haks taken as the user's (custom content, as a builder's would be).
//! What the doctor finds was checked by hand: the defects BioWare and
//! Beamdog left in are listed in `KNOWN`, by module, check and count; any
//! other finding is a false alarm to fix (calibrating this list removed the
//! row-level model check, hiding the game's own rows, and walkmeshes past
//! the tile, which the game's own tiles have).

use mg_module::Module;
use mg_module::doctor::{Severity, TalkTables, examine};
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResMan, priority};
use mg_testkit::{bundled_modules, corpus};

/// Findings in shipped content, checked by hand and real, counted by
/// module, check and source: (module, check, source, count).
const KNOWN: &[(&str, &str, &str, usize)] = &[
    // A tile whose model Beamdog never shipped.
    ("Kingmaker", "set-model", "hak:km_resources", 1),
    // tcm02.set: two tiles without models, a group starting with no tile,
    // tiles declaring doors without door sections, and door types (5001 on)
    // past the hak's own doortypes.2da (1451 rows, hiding the game's).
    ("Neverwinter Nights - Dark Dreams of Furiae", "set-model", "hak:ddf", 2),
    ("Neverwinter Nights - Dark Dreams of Furiae", "set-group", "hak:ddf", 1),
    ("Neverwinter Nights - Dark Dreams of Furiae", "set-data", "hak:ddf", 8),
    ("Neverwinter Nights - Dark Dreams of Furiae", "set-door", "hak:ddf", 17),
    // A blueprint for appearance 750, past the hak's 512-row appearance.2da;
    // world-map pin placeables whose models were never shipped.
    ("Neverwinter Nights - Darkness over Daggerford", "object-row", "module", 11),
    // tno01.set: two groups named "cliff_path1" start with the same tile.
    ("Neverwinter Nights - Darkness over Daggerford", "set-group", "hak:dodee_tno_addons", 1),
    // A placeable on a reserved placeables.2da row (model USER), placed twice
    // in two areas.
    ("Neverwinter Nights - Doom of Icewind Dale", "object-row", "module", 4),
    // A blueprint for appearance 15028, past the hak's appearance.2da.
    ("Neverwinter Nights - Tyrants of the Moonsea", "object-row", "module", 1),
    ("Neverwinter Nights - Tyrants of the Moonsea", "set-data", "hak:tm_race_2da", 2),
    // Placed trees, benches and braziers on placeables.2da rows (500 to 700)
    // that the hak's copy leaves empty, hiding the game's.
    ("Neverwinter Nights - Wyvern Crown of Cormyr", "object-row", "module", 29),
    // Models kept as text that the game misreads a little (`mdl-text`,
    // all warnings): a second node of a name, whose children hang from
    // the first (a model's root named twice; a creature's second head),
    // and a robe with more than four bones on a vertex.
    ("Neverwinter Nights - Darkness over Daggerford", "mdl-text", "hak:dodee_dev", 13),
    ("Neverwinter Nights - Darkness over Daggerford", "mdl-text", "hak:dodee_tno_addons", 8),
    ("Neverwinter Nights - Doom of Icewind Dale", "mdl-text", "hak:doiwd", 1),
    ("Neverwinter Nights - Pirates of the Sword Coast", "mdl-text", "hak:potsc_resources", 5),
    ("Neverwinter Nights - Tyrants of the Moonsea", "mdl-text", "hak:tm_race_2da", 1),
    // Two spells (rows 807 and 809) whose FeatID names feats 3000 and
    // 2589 of a feat.2da of 1607 rows.
    ("Neverwinter Nights - Wyvern Crown of Cormyr", "2da-ref", "hak:wc_2da_shared", 2),
];

#[test]
fn shipped_modules_have_only_known_findings() {
    let root = corpus!();
    let gi = GameInstall::new(&root, None, "en");
    let base =
        mg_tlk::Tlk::read(&std::fs::read(gi.talk_table(false)).unwrap()).unwrap().entries.len();
    let mut unexpected = Vec::new();
    let mut total = 0;
    for path in bundled_modules(&root) {
        let m = Module::open(&path).unwrap();
        let mut rm = ResMan::for_game(&gi).unwrap();
        for hak in m.haks().unwrap() {
            let p = root.join("data/hk").join(format!("{hak}.hak"));
            match ErfContainer::open(&p) {
                Ok(c) => rm.add(priority::HAK_USER, format!("hak:{hak}"), LayerClass::Erf, c),
                Err(_) => eprintln!("{}: hak {hak} not found", path.display()),
            }
        }
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        let custom = m.custom_tlk().ok().flatten().and_then(|name| {
            gi.tlk_dirs().iter().find_map(|d| {
                let data = std::fs::read(d.join(format!("{name}.tlk"))).ok()?;
                Some(mg_tlk::Tlk::read(&data).ok()?.entries.len())
            })
        });
        let found = examine(&m, &rm, TalkTables { base, custom });
        total += found.len();
        let label = path.file_stem().unwrap().to_string_lossy().to_string();
        let mut counts: std::collections::BTreeMap<(&str, &str), usize> = Default::default();
        for f in &found {
            *counts.entry((f.check.as_ref(), f.source.as_str())).or_default() += 1;
        }
        for ((check, source), n) in &counts {
            let known =
                KNOWN.iter().any(|k| k.0 == label && k.1 == *check && k.2 == *source && k.3 == *n);
            if !known {
                unexpected.push(format!("(\"{label}\", \"{check}\", \"{source}\", {n}),"));
                for f in found.iter().filter(|f| f.check == *check && f.source == *source).take(3) {
                    let sev = if f.severity == Severity::Error { "error" } else { "warning" };
                    unexpected.push(format!("    // {sev} {} {}: {}", f.resource, f.at, f.message));
                }
            }
        }
    }
    eprintln!("{total} findings in {} modules", bundled_modules(&root).len());
    assert!(
        unexpected.is_empty(),
        "{} unexpected findings:\n{}",
        unexpected.len(),
        unexpected.join("\n")
    );
}

#[test]
fn row_columns_hold_for_the_game_s_tables() {
    // Every column the checks take for a row of another table names a
    // row that is there, and not a blank one, in the game's own tables: a
    // column that did not would be no rule to hold a hak's table to. (But
    // appearance.FOOTSTEPTYPE, which the game breaks in seven rows.)
    use mg_module::doctor::{ColumnKind, column_kinds, column_kinds_as_named, row_named};
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let table = |name: &str| {
        let key = mg_resman::ResKey::parse(name, mg_core::ResType::TWODA).unwrap();
        mg_2da::TwoDa::parse(&rm.get(&key).unwrap(), mg_core::Codepage::default()).unwrap()
    };
    let blank =
        |t: &mg_2da::TwoDa, row: usize| (0..t.columns().len()).all(|c| t.cell(row, c).is_none());
    let (mut columns, mut cells, mut broken) = (0, 0, Vec::new());
    let (mut named_tables, mut table_cells) = (0, 0);
    for name in [
        "appearance",
        "baseitems",
        "classes",
        "domains",
        "feat",
        "genericdoors",
        "itempropdef",
        "iprp_costtable",
        "iprp_paramtable",
        "packages",
        "placeables",
        "polymorph",
        "racialtypes",
        "spells",
    ] {
        let t = table(name);
        for (column, kind) in column_kinds(name) {
            assert!(t.column(column).is_some(), "{name}.2da has a column {column}");
            // A table it names is there.
            if kind == ColumnKind::Table {
                named_tables += 1;
                for row in 0..t.len() {
                    let Some(other) = t.get(row, column) else { continue };
                    table_cells += 1;
                    let key = mg_resman::ResKey::parse(other.trim(), mg_core::ResType::TWODA);
                    assert!(
                        key.is_some_and(|k| rm.contains(&k)),
                        "{name} row {row}, {column}: {other}"
                    );
                }
                continue;
            }
            let ColumnKind::Row(target) = kind else { continue };
            let other = table(target);
            columns += 1;
            for row in 0..t.len() {
                let named = t.get_int(row, column).and_then(|v| row_named(name, column, v));
                if let Some(named) = named {
                    cells += 1;
                    if named >= other.len() {
                        broken.push(format!("{name} row {row}, {column}: {named}"));
                        continue;
                    }
                    assert!(!blank(&other, named), "{name} row {row}, {column}: {named} is blank");
                }
            }
        }
    }
    assert_eq!(columns, 64);
    assert!(cells > 5000, "{cells}");
    assert_eq!((named_tables, table_cells > 600), (17, true), "{table_cells}");
    let footsteps: Vec<String> =
        (562..=568).map(|r| format!("appearance row {r}, FOOTSTEPTYPE: 30")).collect();
    assert_eq!(broken, footsteps, "the game's own, and only those");

    // The tables a class, a race or a package names: each is known by
    // its name, has the column, and names rows that are there.
    let (mut tables, mut cells) = (0, 0);
    for (naming, by) in [
        ("classes", "FeatsTable"),
        ("classes", "SkillsTable"),
        ("classes", "PreReqTable"),
        ("racialtypes", "FeatsTable"),
        ("packages", "FeatPref2DA"),
        ("packages", "SkillPref2DA"),
        ("packages", "SpellPref2DA"),
    ] {
        let t = table(naming);
        let mut names: Vec<String> =
            (0..t.len()).filter_map(|r| t.get(r, by)).map(str::to_lowercase).collect();
        names.sort();
        names.dedup();
        for name in names {
            let kinds = column_kinds(&name);
            let [(column, kind @ (ColumnKind::Row(_) | ColumnKind::RowBy { .. }))] =
                kinds.as_slice()
            else {
                panic!("{name}, which {naming}.{by} names: {kinds:?}")
            };
            let named = table(&name);
            assert!(named.column(column).is_some(), "{name}.2da has a column {column}");
            tables += 1;
            for row in 0..named.len() {
                // (A requirement's by its kind: some name no row.)
                let Some(target) = kind.row_table(&named, row) else { continue };
                let other = table(target);
                if let Some(v) = named.get_int(row, column).and_then(|v| usize::try_from(v).ok()) {
                    cells += 1;
                    assert!(v < other.len(), "{name} row {row}, {column}: {v} of {target}");
                    assert!(!blank(&other, v), "{name} row {row}, {column}: {v} is blank");
                }
            }
        }
    }
    assert!(tables > 200 && cells > 20000, "{tables} tables, {cells} cells");

    // The tables an item property names: each has a Name, a string (and
    // known by the column that names it, not its own name).
    let read = |name: &str| {
        let key = mg_resman::ResKey::parse(name, mg_core::ResType::TWODA)?;
        let t = mg_2da::TwoDa::parse(&rm.get(&key).ok()?, mg_core::Codepage::default()).ok()?;
        Some(std::sync::Arc::new(t))
    };
    let mut named = 0;
    for (naming, by) in [
        ("itempropdef", "SubTypeResRef"),
        ("iprp_costtable", "Name"),
        ("iprp_paramtable", "TableResRef"),
    ] {
        let t = table(naming);
        let mut names: Vec<String> =
            (0..t.len()).filter_map(|r| t.get(r, by)).map(str::to_lowercase).collect();
        names.sort();
        names.dedup();
        for name in names {
            let kinds = column_kinds_as_named(&name, &read);
            assert!(kinds.contains(&("Name", ColumnKind::StrRef)), "{name}: {kinds:?}");
            assert!(table(&name).column("Name").is_some(), "{name}");
            named += 1;
        }
    }
    assert!(named > 60, "{named}");
}

/// Every column the engine runs a script from (`SCRIPT_COLUMNS`) is in
/// the game's own table, known as a script's, and names the game's own
/// scripts (all but a few: spells.2da names 26 scripts the game doesn't
/// have, vfx_persistent.2da 7).
#[test]
fn script_columns_name_the_game_s_scripts() {
    use mg_module::doctor::{ColumnKind, SCRIPT_COLUMNS, column_kinds};
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    for (name, column) in SCRIPT_COLUMNS {
        let key = mg_resman::ResKey::parse(name, mg_core::ResType::TWODA).unwrap();
        let t = mg_2da::TwoDa::parse(&rm.get(&key).unwrap(), mg_core::Codepage::default()).unwrap();
        assert!(t.column(column).is_some(), "{name}.2da has a column {column}");
        assert!(column_kinds(name).contains(&(column, ColumnKind::Script)), "{name} {column}");
        let cells: Vec<&str> = (0..t.len()).filter_map(|r| t.get(r, column)).collect();
        let there = cells
            .iter()
            .filter(|c| {
                mg_resman::ResKey::parse(c.trim(), mg_core::ResType::NCS)
                    .is_some_and(|k| rm.contains(&k))
            })
            .count();
        assert!(
            !cells.is_empty() && there * 10 >= cells.len() * 8,
            "{name} {column}: {there} of {}",
            cells.len()
        );
    }
    // And areaeffects.2da, which the engine doesn't read, names none.
    let key = mg_resman::ResKey::parse("areaeffects", mg_core::ResType::TWODA).unwrap();
    let t = mg_2da::TwoDa::parse(&rm.get(&key).unwrap(), mg_core::Codepage::default()).unwrap();
    for row in 0..t.len() {
        let script = t.get(row, "ONENTER").unwrap();
        let key = mg_resman::ResKey::parse(script, mg_core::ResType::NCS).unwrap();
        assert!(!rm.contains(&key), "areaeffects.2da row {row}: {script}");
    }
}

#[test]
fn the_game_s_own_tables_have_only_their_known_cell_faults() {
    // What the 2DA editor marks wrong as a table is typed must find
    // nothing in the game's own tables.
    use mg_module::doctor::{TalkTables, column_kinds_as_named, table_faults};
    let root = corpus!();
    let gi = GameInstall::new(&root, None, "en");
    let rm = ResMan::for_game(&gi).unwrap();
    let base =
        mg_tlk::Tlk::read(&std::fs::read(gi.talk_table(false)).unwrap()).unwrap().entries.len();
    let read = |name: &str| {
        let key = mg_resman::ResKey::parse(name, mg_core::ResType::TWODA)?;
        let t = mg_2da::TwoDa::parse(&rm.get(&key).ok()?, mg_core::Codepage::default()).ok()?;
        Some(std::sync::Arc::new(t))
    };
    let mut wrong = Vec::new();
    let mut checked = 0;
    for name in rm.list(mg_core::ResType::TWODA) {
        let name = name.to_string().to_lowercase();
        let kinds = column_kinds_as_named(&name, &read);
        if kinds.is_empty() {
            continue;
        }
        let Some(t) = read(&name) else { continue };
        checked += 1;
        let tlk = TalkTables { base, custom: None };
        for f in table_faults(&name, &t, &kinds, Some(&read), tlk) {
            wrong.push(format!("{name} row {}, {}: {}", f.row, t.columns()[f.column], f.message));
        }
    }
    assert!(checked > 200, "{checked}");
    // (The game's: a bard's spell preferences name spell 2544 of 840, and
    // seven appearances footstep 30 of 18.)
    let mut known: Vec<String> = (562..=568)
        .map(|r| {
            format!(
                "appearance row {r}, FOOTSTEPTYPE: row 30 of footstepsounds.2da doesn't exist: it \
                 has 18 rows"
            )
        })
        .collect();
    known.push(
        "packspbar2 row 40, SpellIndex: row 2544 of spells.2da doesn't exist: it has 840 rows"
            .to_string(),
    );
    assert_eq!(wrong, known);
}
