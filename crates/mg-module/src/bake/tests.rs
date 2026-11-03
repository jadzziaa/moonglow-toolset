use super::*;
use mg_gff::{Gff, Value};
use mg_resman::LayerClass;

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::new(ResRef::from_str(name).unwrap(), t)
}

/// A box of a model, kept as text: a root and one mesh.
const BOX: &str = "newmodel plc_box\nsetsupermodel plc_box NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom plc_box\n\
node dummy plc_box\n  parent NULL\nendnode\n\
node trimesh lid\n  parent plc_box\n  bitmap wood\n  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
  tverts 3\n    0 0 0\n    1 0 0\n    0 1 0\n  faces 1\n    0 1 2 1 0 1 2 0\nendnode\n\
node dummy hinge\n  parent plc_box\n  position 0 0.5 1\nendnode\n\
endmodelgeom plc_box\n\
newanim open plc_box\n  length 1\n  transtime 0.25\n  animroot plc_box\n\
  node dummy plc_box\n    parent NULL\n  endnode\n\
  node dummy hinge\n    parent plc_box\n    orientationkey 2\n      0 1 0 0 0\n      1 1 0 0 1.5\n    endlist\n  endnode\n\
doneanim open plc_box\n\
donemodel plc_box\n";

/// Its walkmesh: a 2 m square, its node 1 m north, and a use point.
const BOX_PWK: &str = "node trimesh plc_box_wg\n  parent plc_box_pwk\n  position 0 1 0\n\
  orientation 0 0 0 0\n  verts 4\n    -1 -1 0\n    1 -1 0\n    -1 1 0\n    1 1 0\n\
  faces 2\n    2 0 3  1  0 0 0  7\n    1 3 0  1  0 0 0  7\nendnode\n\
node dummy plc_box_pwk_use01\n  parent plc_box_pwk\n  position 0 -2 0\nendnode\n";

/// A table whose row 0 is the box, row 1 a chair with no walkmesh, rows 2
/// and 3 padding (`****`), row 4 a lamp.
const TABLE: &str = "2DA V2.0\n\n   Label ModelName Static\n0 Box plc_box 0\n1 Chair plc_chair 1\n\
2 **** **** ****\n3 **** **** ****\n4 Lamp plc_lamp 1\n";

fn game(hak: &[(&str, ResType, &[u8])]) -> ResMan {
    let mut g = MemContainer::new();
    g.insert(key("placeables", ResType::TWODA), TABLE.as_bytes().to_vec());
    g.insert(key("plc_box", ResType::MDL), BOX.as_bytes().to_vec());
    g.insert(key("plc_box", ResType::PWK), BOX_PWK.as_bytes().to_vec());
    g.insert(key("plc_chair", ResType::MDL), BOX.replace("plc_box", "plc_chair").into_bytes());
    let mut h = MemContainer::new();
    for (n, t, d) in hak {
        h.insert(key(n, *t), d.to_vec());
    }
    let mut rm = ResMan::new();
    rm.add(priority::KEY, "game", LayerClass::Key, g);
    rm.add(priority::HAK_USER, "hak:baked", LayerClass::Erf, h);
    rm
}

fn ready(rows: &str) -> Ready {
    Ready {
        rows: Rows::parse(rows).unwrap(),
        hak: "baked".into(),
        names: "mgbk_".into(),
        rows_file: DEFAULT_ROWS_FILE.into(),
    }
}

/// A uniform scale, a turn about the upright axis (degrees) and a move,
/// as `VisualTransform::matrix` makes them (the turn's sense aside).
fn request(appearance: u32, scale: f32, turn: f32, east: f32) -> Request {
    let (s, c) = turn.to_radians().sin_cos();
    Request {
        appearance,
        transform: Transform {
            scale: [scale; 3],
            rotate: [turn, 0.0, 0.0],
            translate: [east, 0.0, 0.0],
        },
        matrix: [
            [c * scale, s * scale, 0.0, 0.0],
            [-s * scale, c * scale, 0.0, 0.0],
            [0.0, 0.0, scale, 0.0],
            [east, 0.0, 0.0, 1.0],
        ],
    }
}

#[test]
fn rows_are_read_and_held_to_the_last_row_the_game_carries() {
    assert_eq!(Rows::parse("16500-16999"), Ok(Rows { from: 16500, to: 16999 }));
    assert_eq!(Rows::parse(" 16_500 \u{2013} 16_999 "), Ok(Rows { from: 16500, to: 16999 }));
    assert_eq!(Rows::parse("700"), Ok(Rows { from: 700, to: 700 }));
    assert_eq!(Rows::parse("65000-65535").map(|r| r.to), Ok(65535));
    let past = Rows::parse("65500-65600").unwrap_err();
    assert!(past.contains("past row 65535") && past.contains("reads 65536 as row 0"), "{past}");
    assert!(Rows::parse("9-3").unwrap_err().contains("first row is after the last"));
    assert!(Rows::parse("lots").unwrap_err().contains("is not rows"));
    assert!(Rows::parse("").is_err());
    assert_eq!(Rows { from: 1, to: 9 }.to_string(), "1-9");
}

#[test]
fn the_rules_say_why_nothing_can_be_baked() {
    let none = BakeRules::default();
    assert!(none.ready().unwrap_err().contains("no [bake] section"));
    let given = BakeRules { given: true, ..Default::default() };
    assert_eq!(given.missing().len(), 2);
    assert!(given.ready().unwrap_err().contains("has no rows"));
    let ok = BakeRules {
        given: true,
        rows: Some(Rows { from: 50000, to: 50999 }),
        hak: Some("mybaked".into()),
        ..Default::default()
    };
    let r = ok.ready().unwrap();
    assert_eq!((r.names.as_str(), r.rows_file.as_str()), (DEFAULT_NAMES, DEFAULT_ROWS_FILE));
    let wrong = BakeRules { problems: vec!["moonglow.toml:3: rows: …".into()], ..ok.clone() };
    assert!(wrong.ready().unwrap_err().contains("moonglow.toml:3"));
    assert_eq!(ok.rows_path(Path::new("p")), Some(Path::new("p").join(DEFAULT_ROWS_FILE)));
    assert!(BakeRules::names_problem("mgbk_").is_none());
    assert!(BakeRules::names_problem("a-b").unwrap().contains("letters, digits and _"));
    assert!(BakeRules::names_problem("thirteen_char").unwrap().contains("too long"));
    assert!(BakeRules::names_problem("").is_some());
}

#[test]
fn the_rows_file_reads_back_as_written() {
    let rows = RowsFile {
        rows: vec![
            BakedRow {
                row: 50001,
                copy_of: 4,
                model: "mgbk_0002".into(),
                label: "Baked: \"Lamp\"".into(),
            },
            BakedRow {
                row: 50000,
                copy_of: 0,
                model: "mgbk_0001".into(),
                label: "Baked: Box at 0.25".into(),
            },
        ],
    };
    let text = rows.text("baked_rows.toml");
    assert!(text.starts_with("# baked_rows.toml: written by Moonglow Toolset"), "{text}");
    assert!(text.contains("[[row]]\nrow = 50001\ncopy_of = 4\nmodel = \"mgbk_0002\"\n"), "{text}");
    let back = RowsFile::parse("baked_rows.toml", &text).unwrap();
    assert_eq!(back.rows.iter().map(|r| r.row).collect::<Vec<_>>(), [50000, 50001], "sorted");
    assert_eq!(back.row(50001).unwrap().label, "Baked: \"Lamp\"");
    assert_eq!(back.of_model("MGBK_0001").map(|r| r.row), Some(50000));
    assert_eq!(RowsFile::parse("x", ""), Ok(RowsFile::default()));
}

/// What is wrong with a rows file is said, with where; nothing panics.
#[test]
fn a_bad_rows_file_is_an_error_that_says_where() {
    let one = "[[row]]\nrow = 50000\ncopy_of = 0\nmodel = \"mgbk_0001\"\nlabel = \"x\"\n";
    let bad = [
        (
            "[[row]]\nrow = 70000\ncopy_of = 0\nmodel = \"a\"\nlabel = \"x\"\n",
            "is not a row from 0 to 65535",
        ),
        ("[[row]]\nrow = \"5\"\ncopy_of = 0\nmodel = \"a\"\nlabel = \"x\"\n", "is not a row"),
        ("[[row]]\ncopy_of = 0\nmodel = \"a\"\nlabel = \"x\"\n", "row 1: no row"),
        ("[[row]]\nrow = 1\ncopy_of = 0\nmodel = 3\nlabel = \"x\"\n", "is not text in quotes"),
        (
            "[[row]]\nrow = 1\ncopy_of = 0\nmodel = \"a b\"\nlabel = \"x\"\n",
            "is not a model's name",
        ),
        (
            "[[row]]\nrow = 1\ncopy_of = 0\nmodel = \"seventeen_chars_x\"\nlabel = \"x\"\n",
            "is not a model's name",
        ),
        (
            "[[row]]\nrow = 1\ncopy_of = 0\nmodel = \"a\"\nlabel = \"x\"\nscale = 2\n",
            "\"scale\" is not a key of a row",
        ),
        (&format!("{one}{one}"), "row 2: row 50000 is in the file twice"),
        ("rows = 3\n", "\"rows\" is not a part of a rows file"),
        ("row = 3\n", "row is a list of tables"),
        ("[[row]]\nrow = 1\n[oops\n", "baked_rows.toml:3:"),
    ];
    for (text, said) in bad {
        let e = RowsFile::parse("baked_rows.toml", text).unwrap_err();
        assert!(e.contains(said), "{text:?}: {e}");
        assert!(e.starts_with("baked_rows.toml"), "{e}");
    }
    // Every prefix of a sound file reads or is an error.
    let text = RowsFile::parse(
        "f",
        &format!("{one}\n[[row]]\nrow = 7\ncopy_of = 1\nmodel = \"m\"\nlabel = \"é\"\n"),
    )
    .unwrap()
    .text("f");
    for end in (0..=text.len()).filter(|&e| text.is_char_boundary(e)) {
        let _ = RowsFile::parse("f", &text[..end]);
    }
}

#[test]
fn a_copy_is_the_model_under_another_name_with_its_part_numbers() {
    let rm = game(&[]);
    let copy = copy_model(&rm, "plc_box", "mgbk_0001").unwrap();
    assert!(mg_mdl::is_binary(&copy));
    let copy = mg_mdl::Model::read(&copy).unwrap();
    let original = mg_mdl::compile::compile_named(BOX.as_bytes(), "plc_box", &|_| None, &|_| None)
        .unwrap()
        .binary;
    let original = mg_mdl::Model::read(&original).unwrap();
    assert_eq!(copy.name, "mgbk_0001");
    assert_eq!(copy.nodes[0].name, "mgbk_0001");
    let names = |m: &mg_mdl::Model| m.nodes[1..].iter().map(|n| n.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&copy), names(&original));
    assert_eq!(
        mg_mdl::binary_write::PartNumbers::of(&copy),
        mg_mdl::binary_write::PartNumbers::of(&original)
    );
    let open = copy.animation("open").unwrap();
    assert_eq!(open.animroot, "mgbk_0001");
    assert!(open.nodes.iter().any(|n| n.name == "hinge"));
    assert_eq!(copy.supermodel, original.supermodel);
    assert!(copy_model(&rm, "plc_none", "x").unwrap_err().contains("plc_none.mdl is nowhere"));
}

#[test]
fn a_bake_takes_the_next_free_row_and_name_and_moves_the_walkmesh() {
    // The hak has a model of the first name already.
    let rm = game(&[("mgbk_0001", ResType::MDL, b"x")]);
    // Rows 2 to 5: 2 and 3 are padding (free), 4 is the lamp's (held), and
    // the rows file has 2.
    let mut rows = RowsFile::default();
    rows.insert(BakedRow { row: 2, copy_of: 1, model: "mgbk_0009".into(), label: "x".into() });
    let made = plan(&ready("2-5"), &rows, &rm, Codepage::default(), &[request(0, 0.5, 90.0, 0.0)])
        .unwrap();
    let b = made.each[0].as_ref().unwrap();
    assert_eq!((b.row, b.copy_of, b.model.as_str(), b.reused), (3, 0, "mgbk_0002", false));
    assert_eq!(b.original, "plc_box");
    assert_eq!(b.label, "Baked: Box at 0.5, turned 90 0 0");
    let names: Vec<String> = made.files.iter().map(|(k, _)| k.to_string()).collect();
    assert_eq!(names, ["mgbk_0002.mdl", "mgbk_0002.pwk"]);
    assert!(made.rows_changed);
    assert_eq!(made.rows.row(3).map(|r| r.model.as_str()), Some("mgbk_0002"));
    // The walkmesh: the square (1 m north) at half size, turned a quarter
    // anticlockwise: 1 m across, half a meter west; the use point too.
    let pwk = Walkmesh::read(&made.files[1].1, WalkmeshKind::Placeable).unwrap();
    let (lo, hi) = pwk
        .object_faces(None)
        .iter()
        .flat_map(|(c, _)| *c)
        .fold(([f32::MAX; 2], [f32::MIN; 2]), |(lo, hi), p| {
            ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
        });
    let near = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5;
    assert!(near(lo, [-1.0, -0.5]) && near(hi, [0.0, 0.5]), "{lo:?} {hi:?}");
    let used = pwk.use_points()[0];
    assert!(near([used[0], used[1]], [1.0, 0.0]), "{used:?}");
    assert_eq!(b.before.len(), 2);
    assert!(made.describe().contains("plc_box is copied as mgbk_0002"), "{}", made.describe());
    // No row left.
    let full =
        plan(&ready("4"), &rows, &rm, Codepage::default(), &[request(0, 0.5, 0.0, 0.0)]).unwrap();
    assert!(full.each[0].as_ref().unwrap_err().contains("every row of [bake] rows 4-4 is taken"));
    // A model without a walkmesh: nothing to bake.
    let chair =
        plan(&ready("2-5"), &rows, &rm, Codepage::default(), &[request(1, 0.5, 0.0, 0.0)]).unwrap();
    assert!(chair.each[0].as_ref().unwrap_err().contains("has no walkmesh (.pwk)"));
    assert!(chair.files.is_empty() && !chair.rows_changed);
    let empty =
        plan(&ready("2-5"), &rows, &rm, Codepage::default(), &[request(3, 0.5, 0.0, 0.0)]).unwrap();
    assert!(empty.each[0].as_ref().unwrap_err().contains("row 3 of placeables.2da has no model"));
}

/// The same model and transform, within a millimeter and a tenth of a
/// degree, share one bake: among those baked together, and with the rows
/// file's; another transform is another bake.
#[test]
fn the_same_bake_is_taken_again() {
    let rm = game(&[]);
    let rules = ready("50000-50999");
    let cp = Codepage::default();
    let together = plan(
        &rules,
        &RowsFile::default(),
        &rm,
        cp,
        &[request(0, 0.5, 30.0, 0.0), request(0, 0.5, 30.05, 0.0005), request(0, 0.5, 31.0, 0.0)],
    )
    .unwrap();
    let rows: Vec<(u32, bool)> = together.bakes().map(|b| (b.row, b.reused)).collect();
    assert_eq!(rows, [(50000, false), (50000, true), (50001, false)]);
    assert_eq!(together.files.len(), 4);
    assert_eq!(together.rows.rows.len(), 2);
    // Written, it is found again by its walkmesh in the hak.
    let hak: Vec<(&str, ResType, &[u8])> = together
        .files
        .iter()
        .map(|(k, d)| {
            (Box::leak(k.resref.to_string().into_boxed_str()) as &str, k.restype, d.as_slice())
        })
        .collect();
    let rm = game(&hak);
    let again = plan(&rules, &together.rows, &rm, cp, &[request(0, 0.5, 30.0, 0.0)]).unwrap();
    let b = again.each[0].as_ref().unwrap();
    assert_eq!((b.row, b.model.as_str(), b.reused), (50000, "mgbk_0001", true));
    assert!(again.files.is_empty() && !again.rows_changed);
    assert!(again.describe().contains("is baked already as mgbk_0001 (row 50000)"));
    // A baked object baked again bakes from the row it copies.
    let rebaked = plan(&rules, &together.rows, &rm, cp, &[request(50000, 0.5, 31.0, 0.0)]).unwrap();
    let b = rebaked.each[0].as_ref().unwrap();
    assert_eq!((b.row, b.copy_of, b.reused), (50001, 0, true));
    // Its model gone from the hak (the walkmesh still there): written again.
    let rm = game(&hak[1..]);
    let again = plan(&rules, &together.rows, &rm, cp, &[request(0, 0.5, 30.0, 0.0)]).unwrap();
    let names: Vec<String> = again.files.iter().map(|(k, _)| k.to_string()).collect();
    assert_eq!(names, ["mgbk_0001.mdl"]);
}

#[test]
fn same_bake_is_a_millimeter_and_a_tenth_of_a_degree() {
    let at = |x: f32| vec![[[x, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]]];
    assert!(same_bake(&at(1.0), &at(1.0009)));
    assert!(!same_bake(&at(1.0), &at(1.003)));
    // Ten meters out, a tenth of a degree is 17 mm.
    assert!(same_bake(&at(10.0), &at(10.015)));
    assert!(!same_bake(&at(10.0), &at(10.03)));
    assert!(!same_bake(&at(1.0), &[]));
}

#[test]
fn rows_the_table_lacks_are_pending_and_drawn_as_the_build_will_add_them() {
    let table = TwoDa::parse(TABLE.as_bytes(), Codepage::default()).unwrap();
    let rows = RowsFile {
        rows: vec![
            BakedRow { row: 3, copy_of: 0, model: "mgbk_0001".into(), label: "Baked: Box".into() },
            BakedRow { row: 4, copy_of: 0, model: "mgbk_0002".into(), label: "x".into() },
            BakedRow { row: 7, copy_of: 1, model: "mgbk_0003".into(), label: "y".into() },
        ],
    };
    assert_eq!(row_state(&table, &rows.rows[0]), RowState::Pending);
    assert_eq!(row_state(&table, &rows.rows[1]), RowState::Other(Some("plc_lamp".into())));
    assert_eq!(row_state(&table, &rows.rows[2]), RowState::Pending);
    let with = pending_table(&table, &rows).unwrap();
    assert_eq!(with.get(3, "ModelName"), Some("mgbk_0001"));
    assert_eq!(with.get(3, "Static"), Some("0"));
    assert_eq!(with.get(3, "Label"), Some("Baked:_Box"));
    assert_eq!(with.get(4, "ModelName"), Some("plc_lamp"), "another's row stays");
    assert_eq!((with.len(), with.get(7, "Static")), (8, Some("1")));
    assert_eq!(row_state(&with, &rows.rows[0]), RowState::Added);
    assert!(pending_table(&with, &RowsFile { rows: rows.rows[..1].to_vec() }).is_none());
    // As a layer over the haks, taken out once nothing is pending.
    let mut rm = game(&[]);
    assert!(layer_pending(&mut rm, Codepage::default(), Some(&rows)));
    assert_eq!(rm.origin(&crate::lights::placeables_key()), Some(PENDING_LABEL));
    let (under, from) = table_under(&rm, Codepage::default()).unwrap();
    assert_eq!((under.len(), from.as_str()), (5, "game"), "read under the pending rows");
    assert!(layer_pending(&mut rm, Codepage::default(), None));
    assert_eq!(rm.origin(&crate::lights::placeables_key()), Some("game"));
    assert!(!layer_pending(&mut rm, Codepage::default(), None));
    assert_eq!(rows_held(&Rows { from: 2, to: 9 }, &table, &rows), [4]);
}

#[test]
fn rows_and_models_no_placeable_uses_are_found_and_tidied() {
    let dir = mg_testkit::scratch_dir("bake-unused");
    let mut m = Module::new();
    let mut bp = Gff::new(*b"UTP ");
    bp.root.set("Appearance", Value::Dword(50001));
    m.set_gff(key("my_box", ResType::UTP), &bp).unwrap();
    let mut git = Gff::new(*b"GIT ");
    let mut placed = mg_gff::Struct::new(9);
    placed.set("Appearance", Value::Dword(50000));
    git.root.set("Placeable List", Value::List(vec![placed.clone(), placed]));
    m.set_gff(key("area", ResType::GIT), &git).unwrap();
    let used = appearances(&m);
    assert_eq!(used.get(&50000), Some(&2));
    assert_eq!(used.get(&50001), Some(&1));
    let row = |row: u32, model: &str| BakedRow {
        row,
        copy_of: 0,
        model: model.into(),
        label: "x".into(),
    };
    let mut rows = RowsFile {
        rows: vec![row(50000, "mgbk_0001"), row(50001, "mgbk_0002"), row(50002, "mgbk_0003")],
    };
    let in_hak: Vec<String> =
        ["mgbk_0001", "mgbk_0003", "mgbk_0004", "plc_other"].map(String::from).to_vec();
    let unused = unused(&m, &rows, "mgbk_", &in_hak);
    assert_eq!(unused.rows, [row(50002, "mgbk_0003")]);
    assert_eq!(unused.models, ["mgbk_0003", "mgbk_0004"]);
    // Tidied: the rows file and the hak.
    let hak = dir.join("baked.hak");
    let files = ["mgbk_0001", "mgbk_0003", "mgbk_0004"]
        .iter()
        .flat_map(|n| {
            [ResType::MDL, ResType::PWK]
                .map(|t| (key(n, t), crate::hak_edit::Source::Bytes(b"x".as_slice().into())))
        })
        .collect();
    crate::hak_edit::write_into(&hak, files).unwrap();
    tidy(&unused, &mut rows, Some(&hak)).unwrap();
    assert_eq!(rows.rows.len(), 2);
    let left: Vec<String> = crate::hak_edit::Hak::open(&hak)
        .unwrap()
        .items()
        .iter()
        .map(|i| i.key.to_string())
        .collect();
    assert_eq!(left, ["mgbk_0001.mdl", "mgbk_0001.pwk"]);
    assert!(crate::sibling(&hak, ".bak").exists(), "kept beside itself");
    // Its models listed from the hak's layer.
    let rm = game(&[("mgbk_0002", ResType::MDL, b"x"), ("mgbk_0001", ResType::MDL, b"x")]);
    assert_eq!(hak_models(&rm, "baked"), ["mgbk_0001", "mgbk_0002"]);
    assert!(hak_models(&rm, "other").is_empty());
}

#[test]
fn labels_say_the_transform_briefly() {
    let t = |s: f32, r: [f32; 3], m: [f32; 3]| Transform { scale: [s; 3], rotate: r, translate: m };
    assert_eq!(label("Boulder_03", &t(0.25, [0.0; 3], [0.0; 3])), "Baked: Boulder 03 at 0.25");
    assert_eq!(label("Box", &t(1.0, [0.0; 3], [0.0; 3])), "Baked: Box");
    assert_eq!(
        label("Box", &t(1.0 / 3.0, [45.0, -0.0, 12.5], [0.0, 1.5, 0.0])),
        "Baked: Box at 0.33, turned 45 0 12.5, moved 0 1.5 0"
    );
}
