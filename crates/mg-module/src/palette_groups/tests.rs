use super::*;
use crate::ModuleLocation;
use crate::project_file::{FILE_NAME, ProjectFile};

/// The proposal's example, as settled (2026-11-03).
const KIND: &str = r#"
[palette.kind]
title = "Kind"
types = ["utp", "uti"]
words_of = '^.*?:\s*(.*?)\s*(\(|$)'
from = "palette_kinds.toml"

[[palette.kind.group]]
name = "Tables"
types = ["utp"]
names = ["table", "desk", "counter"]
resrefs = ["dd_*_tbl*"]

[[palette.kind.group]]
name = "Food"
names = ["cheese", "bread", "ham", "Pâté"]

[[palette.kind.group]]
name = "Seats"
names = ["dining chair", "stool"]
"#;

fn read(source: &str) -> ProjectFile {
    let mut p = ProjectFile::default();
    p.apply(FILE_NAME, source, false);
    p
}

fn sorted_names(g: &Grouping, kind: BlueprintKind, entries: &[(&str, &str)]) -> Vec<Vec<String>> {
    let keyed: Vec<(ResRef, &str)> =
        entries.iter().map(|(r, n)| (ResRef::from_str(r).unwrap(), *n)).collect();
    let s = g.sort(kind, &keyed);
    let names = |at: &[usize]| at.iter().map(|&i| entries[i].0.to_string()).collect();
    s.groups.iter().map(|m| names(m)).chain(std::iter::once(names(&s.other))).collect()
}

#[test]
fn the_keys_are_read_in_the_file_s_order() {
    let p = read(KIND);
    assert_eq!(p.warnings, Vec::<String>::new());
    let g = p.palette.find("kind").expect("the grouping");
    assert_eq!((g.id.as_str(), g.title.as_str()), ("kind", "Kind"));
    assert_eq!(g.types, [BlueprintKind::Placeable, BlueprintKind::Item]);
    assert_eq!(g.from.as_deref(), Some("palette_kinds.toml"));
    let names: Vec<&str> = g.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["Tables", "Food", "Seats"], "as the file has them, not sorted");
    assert_eq!(g.groups[0].types, [BlueprintKind::Placeable]);
    assert_eq!(g.groups[0].resrefs.globs, ["dd_*_tbl*"]);
    assert!(g.offered(BlueprintKind::Item) && !g.offered(BlueprintKind::Creature));
    assert!(!g.group_offered(&g.groups[0], BlueprintKind::Item), "Tables: placeables only");
    // Found by its title too; a title left out is the id's.
    assert!(p.palette.find("KIND").is_some());
    let p = read("[palette.by_theme]\n[[palette.by_theme.group]]\nname = \"Ruin\"\n");
    assert_eq!(p.palette.list[0].title, "By theme");
    assert!(p.palette.list[0].types.is_empty() && p.palette.list[0].offered(BlueprintKind::Door));
}

#[test]
fn blueprints_go_under_every_group_their_words_put_them_in() {
    let p = read(KIND);
    let g = &p.palette.list[0];
    let entries = [
        ("dd_n2e_table2", "Tavern: Table 2 (N2E)"),
        ("dd_mkt_cheese", "Market: Cheese and bread board [64]"),
        // (The words come from the part after the colon, before the
        // brackets: "after HAM" is the pack's, not a ham.)
        ("dd_fern01", "Forest: Fern 01 (after HAM)"),
        // Whole words: a "tablet" is no table.
        ("dd_tablet", "Temple: Tablet"),
        // A ResRef pattern.
        ("dd_vil_tbl01", "Village: Plank"),
        // Case and accents ignored.
        ("dd_pate", "Kitchen: PATE en croute"),
        // Two words, in that order.
        ("dd_chair1", "Manor: Dining Chair"),
        ("dd_chair2", "Manor: Chair, dining"),
        ("dd_counter", "Shop: Counter, with bread"),
    ];
    let s = sorted_names(g, BlueprintKind::Placeable, &entries);
    assert_eq!(s[0], ["dd_n2e_table2", "dd_vil_tbl01", "dd_counter"], "Tables");
    assert_eq!(s[1], ["dd_mkt_cheese", "dd_pate", "dd_counter"], "Food");
    assert_eq!(s[2], ["dd_chair1"], "Seats");
    assert_eq!(s[3], ["dd_fern01", "dd_tablet", "dd_chair2"], "Other");
    // Tables isn't offered among the items: theirs go elsewhere.
    let s = sorted_names(g, BlueprintKind::Item, &entries[..1]);
    assert_eq!(s[0], Vec::<String>::new());
    assert_eq!(s[3], ["dd_n2e_table2"]);
    // A palette the grouping isn't offered in: all Other.
    let s = sorted_names(g, BlueprintKind::Creature, &entries[..2]);
    assert_eq!(s[3].len(), 2);
}

#[test]
fn a_name_its_words_of_does_not_match_gives_its_words_whole() {
    let p = read(KIND);
    let g = &p.palette.list[0];
    let w = g.words_of.as_ref().unwrap();
    assert_eq!(w.of("Tavern: Table 2 (N2E)"), "Table 2");
    assert_eq!(w.of("A plain table"), "A plain table");
    let s = sorted_names(g, BlueprintKind::Placeable, &[("x", "A plain table")]);
    assert_eq!(s[0], ["x"]);
}

/// The `from` file's lists are added to the rules' groups; a group only it
/// names comes after the project file's, in its order; a ResRef with its
/// type is in that palette alone.
#[test]
fn the_from_file_adds_lists_and_groups() {
    let dir = mg_testkit::scratch_dir("palette-groups-from");
    let mut m = crate::Module::new();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join(FILE_NAME), KIND).unwrap();
    std::fs::write(
        dir.join("palette_kinds.toml"),
        "# written by the library build\n\"Lights\" = [\"dd_lamp01\"]\n\
         \"Food\" = [\"dd_din_fruitpapa.utp\", \"DD_PIE\"]\n\"Beds\" = [\"dd_bed\"]\n",
    )
    .unwrap();
    let m = crate::Module::open(&dir).unwrap();
    let p = &m.project_file;
    assert_eq!(p.warnings, Vec::<String>::new());
    let g = &p.palette.list[0];
    let names: Vec<&str> = g.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["Tables", "Food", "Seats", "Lights", "Beds"]);
    assert!(g.groups[3].from_file && !g.groups[1].from_file);
    let entries = [
        ("dd_din_fruitpapa", "Dining: Fruit"),
        ("dd_pie", "Bakery: Something"),
        ("dd_lamp01", "Street: Lamp"),
    ];
    let s = sorted_names(g, BlueprintKind::Placeable, &entries);
    assert_eq!(s[1], ["dd_din_fruitpapa", "dd_pie"], "Food: by its list, ResRefs' case ignored");
    assert_eq!(s[3], ["dd_lamp01"]);
    let s = sorted_names(g, BlueprintKind::Item, &entries);
    assert_eq!(s[1], ["dd_pie"], "the .utp is a placeable's alone");
}

/// A key not understood is said with its line, and the grouping works
/// without it.
#[test]
fn a_bad_key_is_said_and_the_rest_is_read() {
    let p = read(
        "[palette.kind]\ntypes = [\"utp\", \"xyz\"]\ncolour = 1\nwords_of = '('\n\
         [[palette.kind.group]]\nnames = [\"x\"]\n\
         [[palette.kind.group]]\nname = \"Other\"\n\
         [[palette.kind.group]]\nname = \"Tables\"\nnames = \"table\"\nresrefs = [\"a[\"]\n\
         [[palette.kind.group]]\nname = \"tables\"\nnames = [\"desk\", \"--\"]\nshape = 2\n\
         [palette]\n",
    );
    let g = &p.palette.list[0];
    assert_eq!(g.types, [BlueprintKind::Placeable]);
    assert!(g.words_of.is_none());
    assert_eq!(g.groups.len(), 1, "{:?}", g.groups);
    assert_eq!(g.groups[0].names, ["desk"], "the second Tables's rules added to the first's");
    let said = p.warnings.join("\n");
    for words in [
        "moonglow.toml:2: palette.kind types: \"xyz\" is none of utc,",
        "moonglow.toml:3: \"colour\" is not a key of [palette.kind]",
        "moonglow.toml:4: palette.kind words_of '(' is no expression",
        "moonglow.toml:5: [[palette.kind.group]] has no name",
        "moonglow.toml:7: [[palette.kind.group]] \"Other\": the blueprints no group takes",
        "moonglow.toml:11: palette.kind.group names is to be a list",
        "moonglow.toml:13: [[palette.kind.group]] \"tables\" is there twice",
        "moonglow.toml:15: palette.kind.group names: \"--\" has no word",
        "moonglow.toml:16: \"shape\" is not a key of [palette.kind.group]",
    ] {
        assert!(said.contains(words), "{words}\nin\n{said}");
    }
    assert_eq!(p.palette.problems, p.warnings, "and said by Verify");
    assert!(p.errors.is_empty());
    // A grouping that is no table; the builder's own file has none.
    let p = read("[palette]\nkind = 1\n");
    assert!(p.warnings[0].contains("palette.kind is to be a grouping"), "{:?}", p.warnings);
    let mut p = ProjectFile::default();
    p.apply("user.toml", KIND, true);
    assert!(p.palette.list.is_empty());
    assert_eq!(p.warnings, ["user.toml:2: [palette] is the team's: it goes in moonglow.toml"]);
}

/// A `from` file missing, or no TOML, or with entries that aren't
/// ResRefs, is said; the grouping works with its rules (no error: the
/// project's other rules stay).
#[test]
fn a_from_file_missing_or_bad_is_said() {
    let dir = mg_testkit::scratch_dir("palette-groups-from-bad");
    let mut m = crate::Module::new();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join(FILE_NAME), KIND).unwrap();
    let mut m = crate::Module::open(&dir).unwrap();
    let p = &m.project_file;
    assert!(p.errors.is_empty());
    assert_eq!(
        p.warnings,
        ["palette_kinds.toml: not there ([palette.kind] from): the grouping has only its rules"]
    );
    assert_eq!(p.palette.problems, p.warnings);
    assert_eq!(p.palette.list[0].groups.len(), 3);
    // Made: read again (a change to the file is one to the rules).
    let before = p.palette.fingerprint;
    let kinds = dir.join("palette_kinds.toml");
    std::fs::write(&kinds, "\"Tables\" = [").unwrap();
    assert!(m.reread_project_file(), "the from file made");
    let p = &m.project_file;
    assert!(p.warnings[0].starts_with("palette_kinds.toml:1: "), "{:?}", p.warnings);
    assert!(p.warnings[0].ends_with("the grouping has only its rules"));
    assert!(p.errors.is_empty() && p.palette.list[0].groups.len() == 3);
    std::fs::write(
        &kinds,
        "\"Tables\" = [\"ok\", 3, \"\", \"waytoolongforaresref\", \"a.xyz\", \"b.uti\"]\n\
         \"Beds\" = \"bed\"\n\"\" = []\n\"other\" = [\"x\"]\n",
    )
    .unwrap();
    assert!(m.reread_project_file());
    let p = &m.project_file;
    let said = p.warnings.join("\n");
    for words in [
        "palette_kinds.toml:1: \"Tables\": not text",
        "palette_kinds.toml:1: \"Tables\": an empty ResRef",
        "palette_kinds.toml:1: \"Tables\": \"waytoolongforaresref\"",
        "palette_kinds.toml:1: \"Tables\": \"a.xyz\": .xyz is none of",
        "palette_kinds.toml:1: \"Tables\": \"b.uti\": the group isn't offered in the item palette",
        "palette_kinds.toml:2: \"Beds\" is to be a list of ResRefs",
        "palette_kinds.toml:3: a group's name is empty",
        "palette_kinds.toml:4: \"other\": the blueprints no group takes",
    ] {
        assert!(said.contains(words), "{words}\nin\n{said}");
    }
    let tables = &p.palette.list[0].groups[0];
    assert_eq!(tables.listed.len(), 2, "ok and b.uti: {:?}", tables.listed);
    assert_ne!(p.palette.fingerprint, before);
    assert!(!m.reread_project_file(), "unchanged");
    std::fs::remove_file(&kinds).unwrap();
    assert!(m.reread_project_file(), "deleted");
}

/// Parsers never panic: every cut of a `from` file, and bytes that are no
/// text.
#[test]
fn every_cut_of_a_from_file_reads_without_a_panic() {
    let full = "# lists\n\"Tables\" = [\"dd_n2e_table2\", \"dd_vil_tbl01.utp\"]\n\
                \"Food\" = [\"dd_din_fruitpapa.utp\"]\n\"Ñandú\" = [\"x\"]\n";
    let rules = read(KIND).palette.list[0].clone();
    for end in (0..=full.len()).filter(|&i| full.is_char_boundary(i)) {
        let mut g = rules.clone();
        let mut warnings = Vec::new();
        read_list("palette_kinds.toml", &full[..end], &mut g, &mut warnings);
        let _ = g.sort(BlueprintKind::Placeable, &[(ResRef::from_str("x").unwrap(), "A: x")]);
    }
    let mut g = rules.clone();
    let mut warnings = Vec::new();
    let junk = String::from_utf8_lossy(&[0xFF, b'[', 0, b'"', 0xC3, b'=']);
    read_list("palette_kinds.toml", &junk, &mut g, &mut warnings);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(g.groups.len(), 3);
    // And the project file's keys cut anywhere.
    for end in (0..=KIND.len()).filter(|&i| KIND.is_char_boundary(i)) {
        let p = read(&KIND[..end]);
        for g in &p.palette.list {
            let _ = g.sort(BlueprintKind::Item, &[(ResRef::from_str("x").unwrap(), "x")]);
        }
    }
}

fn node(
    name: &str,
    id: Option<u8>,
    children: Vec<PaletteNode>,
    bps: &[(&str, &str)],
) -> PaletteNode {
    PaletteNode {
        name: PaletteName::Text(name.as_bytes().to_vec()),
        id,
        children,
        blueprints: bps
            .iter()
            .map(|(r, n)| PaletteBlueprint {
                resref: ResRef::from_str(r).unwrap(),
                name: PaletteName::Text(n.as_bytes().to_vec()),
                cr: None,
                faction: None,
            })
            .collect(),
    }
}

fn text(n: &PaletteName) -> String {
    match n {
        PaletteName::Text(t) => String::from_utf8_lossy(t).into_owned(),
        PaletteName::StrRef(s) => format!("string {s}"),
    }
}

/// A palette grouped: a node for each group with blueprints, Other last;
/// in each the categories they are in (pruned to them), or a list.
#[test]
fn a_palette_grouped_has_its_categories_in_each_group() {
    let palette = Palette {
        nodes: vec![node(
            "Domains of Dread",
            None,
            vec![
                node(
                    "Tavern",
                    Some(1),
                    vec![],
                    &[("dd_t1", "Tavern: Table 1"), ("dd_t2", "Tavern: Stool")],
                ),
                node(
                    "Manor",
                    Some(2),
                    vec![],
                    &[("dd_m1", "Manor: Table, oak"), ("dd_m2", "Manor: Cheese table")],
                ),
                node("Ruin", Some(3), vec![], &[("dd_r1", "Ruin: Rubble")]),
            ],
            &[],
        )],
    };
    let p = read(KIND);
    let g = &p.palette.list[0];
    let grouped = group_palette(&palette, BlueprintKind::Placeable, g, &text, true);
    let top: Vec<String> = grouped.palette.nodes.iter().map(|n| text(&n.name)).collect();
    assert_eq!(top, ["Tables", "Food", "Seats", "Other"]);
    assert_eq!(grouped.groups, [Some(0), Some(1), Some(2), None]);
    let tables = &grouped.palette.nodes[0];
    assert!(tables.blueprints.is_empty() && tables.id.is_none());
    let dod = &tables.children[0];
    let cats: Vec<(String, Option<u8>, usize)> =
        dod.children.iter().map(|c| (text(&c.name), c.id, c.blueprints.len())).collect();
    assert_eq!(cats, [("Tavern".into(), Some(1), 1), ("Manor".into(), Some(2), 2)]);
    // Each category's place in the palette it came from.
    assert_eq!(grouped.origin[&vec![0, 0]], [0]);
    assert_eq!(grouped.origin[&vec![0, 0, 1]], [0, 1]);
    assert_eq!(grouped.origin[&vec![3, 0, 0]], [0, 2], "Other's Ruin");
    // A blueprint in two groups is under both (Manor's cheese table).
    let food = &grouped.palette.nodes[1].children[0].children[0];
    assert_eq!(
        (text(&food.name), food.blueprints[0].resref.to_string()),
        ("Manor".into(), "dd_m2".into())
    );
    // Flat: the group's blueprints in the palette's order.
    let flat = group_palette(&palette, BlueprintKind::Placeable, g, &text, false);
    let tables: Vec<String> =
        flat.palette.nodes[0].blueprints.iter().map(|b| b.resref.to_string()).collect();
    assert_eq!(tables, ["dd_t1", "dd_m1", "dd_m2"]);
    assert!(flat.palette.nodes[0].children.is_empty() && flat.origin.is_empty());
    // A group no blueprint falls in is left out.
    let few = Palette { nodes: vec![node("Ruin", Some(3), vec![], &[("dd_r1", "Ruin: Rubble")])] };
    let grouped = group_palette(&few, BlueprintKind::Placeable, g, &text, true);
    assert_eq!(grouped.groups, [None]);
    assert_eq!(grouped.group_name(g, 0), OTHER);
}

/// Any number of groupings, each with its own `from` file, each offered
/// where its `types` say (TDoD's: `kind` in every palette but the doors',
/// `role` among the items alone); a group's `types` narrow its
/// grouping's, and a listed `dd_x.uti` is an item's alone.
#[test]
fn two_groupings_each_with_its_own_list() {
    let dir = mg_testkit::scratch_dir("palette-groups-two");
    let mut m = crate::Module::new();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(
        dir.join(FILE_NAME),
        "[palette.kind]\ntypes = [\"utc\", \"ute\", \"uti\", \"utp\", \"uts\", \"utm\", \"utt\", \
         \"utw\"]\nfrom = \"palette_kinds.toml\"\n[[palette.kind.group]]\nname = \"Tables\"\n\
         types = [\"utp\"]\nnames = [\"table\"]\n\
         [palette.role]\ntitle = \"Role\"\ntypes = [\"uti\"]\nfrom = \"palette_roles.toml\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("palette_kinds.toml"), "\"Food\" = [\"dd_x.uti\", \"dd_pie\"]\n")
        .unwrap();
    std::fs::write(dir.join("palette_roles.toml"), "\"Weapons\" = [\"dd_sword\"]\n").unwrap();
    let mut m = crate::Module::open(&dir).unwrap();
    let p = &m.project_file;
    assert_eq!(p.warnings, Vec::<String>::new());
    let titles = |k: BlueprintKind| -> Vec<&str> {
        p.palette.offered(k).map(|g| g.title.as_str()).collect()
    };
    assert_eq!(titles(BlueprintKind::Item), ["Kind", "Role"]);
    assert_eq!(titles(BlueprintKind::Placeable), ["Kind"]);
    assert_eq!(titles(BlueprintKind::Door), Vec::<&str>::new(), "doors: Category alone");
    let (kind, role) = (&p.palette.list[0], &p.palette.list[1]);
    let entries = [("dd_x", "A table"), ("dd_pie", "Pie"), ("dd_sword", "Sword")];
    let s = sorted_names(kind, BlueprintKind::Item, &entries);
    assert_eq!(s[0], Vec::<String>::new(), "Tables: placeables alone");
    assert_eq!(s[1], ["dd_x", "dd_pie"]);
    let s = sorted_names(kind, BlueprintKind::Placeable, &entries);
    assert_eq!(s[0], ["dd_x"]);
    assert_eq!(s[1], ["dd_pie"], "dd_x.uti is an item's alone");
    let s = sorted_names(role, BlueprintKind::Item, &entries);
    assert_eq!(s[0], ["dd_sword"]);
    assert_eq!(s[1], ["dd_x", "dd_pie"], "Other");
    // Each list is watched: a change to the second is seen.
    assert!(!m.reread_project_file());
    std::fs::write(dir.join("palette_roles.toml"), "\"Weapons\" = [\"dd_axe\"]\n").unwrap();
    assert!(m.reread_project_file());
    assert_eq!(m.project_file.palette.list[1].groups[0].listed[0].0.to_string(), "dd_axe");
}

/// A group of several types (TDoD's Animals: placeables and creatures)
/// shows in each palette it has blueprints of; one with none there
/// (Lights, among the creatures) isn't drawn.
#[test]
fn a_group_of_several_types_shows_where_it_has_blueprints() {
    let mut g = read("[palette.kind]\ntypes = [\"utp\", \"utc\"]\n").palette.list[0].clone();
    let mut warnings = Vec::new();
    read_list(
        "palette_kinds.toml",
        "\"Animals\" = [\"dd_cat.utp\", \"dd_wolf.utc\"]\n\"Lights\" = [\"dd_lamp.utp\"]\n",
        &mut g,
        &mut warnings,
    );
    assert_eq!(warnings, Vec::<String>::new());
    let shown = |kind: BlueprintKind, bps: &[(&str, &str)]| -> Vec<String> {
        let palette = Palette { nodes: vec![node("All", Some(1), vec![], bps)] };
        let grouped = group_palette(&palette, kind, &g, &text, true);
        (0..grouped.groups.len()).map(|i| grouped.group_name(&g, i).to_string()).collect()
    };
    let placeables = [("dd_cat", "Cat"), ("dd_lamp", "Lamp"), ("dd_rock", "Rock")];
    assert_eq!(shown(BlueprintKind::Placeable, &placeables), ["Animals", "Lights", "Other"]);
    assert_eq!(shown(BlueprintKind::Creature, &[("dd_wolf", "Wolf")]), ["Animals"]);
    // (dd_cat.utp is a placeable's: a creature of that ResRef is Other's.)
    assert_eq!(shown(BlueprintKind::Creature, &[("dd_cat", "Cat")]), ["Other"]);
}

/// `order = "name"`: a grouping's groups by name, the project file's and
/// those its list alone names together, case and accents aside (TDoD's
/// ask: rules for a few kinds put them before the alphabetical rest). As
/// the file gives them without it; a value it doesn't know is said, and
/// the file's order kept. A list changed is put in order again.
#[test]
fn a_grouping_s_groups_are_ordered_by_name_when_asked() {
    let dir = mg_testkit::scratch_dir("palette-groups-by-name");
    let mut m = crate::Module::new();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let rules = |order: &str| {
        format!(
            "[palette.kind]\n{order}from = \"kinds.toml\"\n\
             [[palette.kind.group]]\nname = \"Tables\"\nnames = [\"table\"]\n\
             [[palette.kind.group]]\nname = \"beds\"\nnames = [\"bed\"]\n"
        )
    };
    std::fs::write(
        dir.join("kinds.toml"),
        "\"Seats\" = [\"dd_s\"]\n\"Échelles\" = [\"dd_l\"]\n\"Animals\" = [\"dd_a\"]\n",
    )
    .unwrap();
    let names = |m: &crate::Module| -> Vec<String> {
        m.project_file.palette.list[0].groups.iter().map(|g| g.name.clone()).collect()
    };
    std::fs::write(dir.join(FILE_NAME), rules("")).unwrap();
    let m = crate::Module::open(&dir).unwrap();
    assert_eq!(
        names(&m),
        ["Tables", "beds", "Seats", "Échelles", "Animals"],
        "as the files give them"
    );
    std::fs::write(dir.join(FILE_NAME), rules("order = \"name\"\n")).unwrap();
    let mut m = crate::Module::open(&dir).unwrap();
    assert_eq!(m.project_file.warnings, Vec::<String>::new());
    assert_eq!(names(&m), ["Animals", "beds", "Échelles", "Seats", "Tables"]);
    // What each holds goes with it.
    let kind = &m.project_file.palette.list[0];
    let entries = [("dd_a", "A rat"), ("dd_x", "A table"), ("dd_l", "Ladder")];
    let s = sorted_names(kind, BlueprintKind::Placeable, &entries);
    assert_eq!(
        (s[0].clone(), s[2].clone(), s[4].clone()),
        (vec!["dd_a".to_string()], vec!["dd_l".to_string()], vec!["dd_x".to_string()])
    );
    // A list changed: in order again.
    std::fs::write(dir.join("kinds.toml"), "\"Dishes\" = [\"dd_d\"]\n\"Animals\" = [\"dd_a\"]\n")
        .unwrap();
    assert!(m.reread_project_file());
    assert_eq!(names(&m), ["Animals", "beds", "Dishes", "Tables"]);
    // A value it doesn't know: said, and the file's order kept.
    std::fs::write(dir.join(FILE_NAME), rules("order = \"size\"\n")).unwrap();
    let m = crate::Module::open(&dir).unwrap();
    assert!(
        m.project_file.warnings.iter().any(|w| w.contains("order \"size\" is neither")),
        "{:?}",
        m.project_file.warnings
    );
    assert_eq!(names(&m), ["Tables", "beds", "Dishes", "Animals"]);
}
