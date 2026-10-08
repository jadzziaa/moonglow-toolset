//! What a plugin reaches outside the module, and the terrain it paints:
//! a file and a folder the user chose, resources for a hak they agreed
//! to, pictures, and areas made and painted as the area editor does it.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use mg_edit::{Command, Workspace};
use mg_gff::Gff;
use mg_module::Module;
use mg_plugin::{
    Answer, HakData, Host, Input, Level, Outcome, Plugin, PluginError, Question, run_command,
    run_console,
};
use mg_resman::ResKey;
use mg_rules::GameData;

#[derive(Default)]
struct TestHost {
    log: RefCell<Vec<String>>,
    answers: RefCell<VecDeque<Option<Answer>>>,
    asked: RefCell<Vec<Question>>,
}

impl Host for TestHost {
    fn log(&self, _: Level, text: &str) {
        self.log.borrow_mut().push(text.to_string());
    }

    fn ask(&self, question: &Question) -> Option<Answer> {
        self.asked.borrow_mut().push(question.clone());
        self.answers.borrow_mut().pop_front().flatten()
    }
}

fn host(answers: impl IntoIterator<Item = Option<Answer>>) -> Rc<TestHost> {
    let host = TestHost::default();
    host.answers.borrow_mut().extend(answers);
    Rc::new(host)
}

fn bare() -> Module {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m
}

fn console(code: &str, host: &Rc<TestHost>) -> Result<Outcome, PluginError> {
    run_console(code, Input { module: bare(), game: None }, host.clone())
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mg-plugin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_file_the_user_chose_is_read() {
    let dir = scratch("file");
    std::fs::write(dir.join("notes.txt"), "from outside").unwrap();
    let h = host([Some(Answer::Path(dir.join("notes.txt")))]);
    let code = r#"
        local file = ctx.ui:open_file({ title = "Notes", extensions = { ".TXT" } })
        return file.name, file.bytes, ctx.ui:open_file()
    "#;
    console(code, &h).unwrap();
    assert_eq!(h.log.borrow().as_slice(), ["\"notes.txt\"", "\"from outside\"", "nil"]);
    assert_eq!(
        h.asked.borrow()[0],
        Question::File { title: "Notes".into(), extensions: vec!["txt".into()] }
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_folder_the_user_chose_is_read_and_nothing_outside_it() {
    let dir = scratch("folder");
    std::fs::create_dir_all(dir.join("export/sub")).unwrap();
    std::fs::write(dir.join("export/tiles.set"), "[GENERAL]\n").unwrap();
    std::fs::write(dir.join("export/sub/deep.txt"), "deep").unwrap();
    std::fs::write(dir.join("private.txt"), "private").unwrap();
    let h = host([Some(Answer::Path(dir.join("export")))]);
    let code = r#"
        local folder = ctx.ui:open_folder({ title = "Export" })
        local out = pcall(function() return folder:bytes("../private.txt") end)
        return folder.name, table.concat(folder:files(), " "), folder:text("sub/deep.txt"), out
    "#;
    console(code, &h).unwrap();
    assert_eq!(
        h.log.borrow().as_slice(),
        ["\"export\"", "\"sub/deep.txt tiles.set\"", "\"deep\"", "false"]
    );
    // Nobody to ask (a check, the command line without --file): nothing.
    let h = host([]);
    console("return ctx.ui:open_folder()", &h).unwrap();
    assert_eq!(h.log.borrow().as_slice(), ["nil"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_hak_takes_resources_with_the_user_s_leave() {
    let dir = scratch("hak");
    std::fs::write(dir.join("big.tga"), "pixels").unwrap();
    let code = r#"
        ctx.hak:write("My_Tiles", "tiles.set", "[GENERAL]")
        ctx.hak:write("my_tiles", "tiles.set", "[GENERAL]\nName=tiles")
        local folder = ctx.ui:open_folder()
        folder:to_hak("my_tiles", "big.tga", "small.tga")
        ctx.hak:attach("my_tiles")
        -- What the job put into a hak, the game would load.
        return ctx.game:has("tiles.set"), ctx.game:text("small.tga")
    "#;
    // Refused: the job fails, and nothing is handed back.
    let h = host([Some(Answer::No)]);
    let refused = console(code, &h).unwrap_err().to_string();
    assert!(refused.contains("writing the hak my_tiles was not allowed"), "{refused}");
    let Question::Confirm(asked) = &h.asked.borrow()[0] else { panic!("no question") };
    assert!(asked.contains("my_tiles.hak in your hak folder"), "{asked}");

    // Allowed: asked once for the hak, the resources handed back.
    let h = host([Some(Answer::Yes), Some(Answer::Path(dir.clone()))]);
    let outcome = console(code, &h).unwrap();
    assert_eq!(h.asked.borrow().len(), 2);
    assert_eq!(h.log.borrow().as_slice(), ["true", "\"pixels\""]);
    assert_eq!(outcome.haks.len(), 1);
    let hak = &outcome.haks[0];
    assert_eq!(hak.name, "my_tiles");
    assert_eq!(hak.files.len(), 2);
    assert_eq!(hak.files[0].1, HakData::Bytes(b"[GENERAL]\nName=tiles".as_slice().into()));
    assert!(matches!(&hak.files[1].1, HakData::File(p) if p.ends_with("big.tga")));
    // The module lists the hak.
    let mut ws = Workspace::new(bare());
    ws.apply(Command::new(outcome.label.clone(), outcome.edits.clone())).unwrap();
    ws.flush().unwrap();
    assert_eq!(ws.module.haks().unwrap(), ["my_tiles"]);
    // Written where the host says haks go.
    assert_eq!(hak.write(&dir.join("hak")).unwrap(), (2, 0));
    let written = mg_module::hak_edit::Hak::open(&dir.join("hak/my_tiles.hak")).unwrap();
    assert_eq!(written.data(ResKey::from_filename("small.tga").unwrap()).unwrap(), b"pixels");

    // A name that leads elsewhere, and a file no hak holds.
    for bad in [
        r#"ctx.hak:write("../elsewhere", "a.txt", "")"#,
        r#"ctx.hak:write("fine", "a_name_far_too_long_for_it.txt", "")"#,
        r#"ctx.hak:write("fine", "notes.docx", "")"#,
    ] {
        let h = host([Some(Answer::Yes)]);
        assert!(console(bad, &h).is_err(), "{bad}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_picture_is_decoded() {
    let mut png_bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png_bytes, 2, 1);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&[10, 200]).unwrap();
    }
    let dir = scratch("picture");
    std::fs::write(dir.join("heights.png"), &png_bytes).unwrap();
    let h = host([Some(Answer::Path(dir.join("heights.png")))]);
    let code = r#"
        local image = mg.image(ctx.ui:open_file().bytes)
        local r, g, b, a = image:pixel(1, 0)
        local outside = pcall(function() return image:pixel(2, 0) end)
        return image.width, image.height, r + g + b, a, outside
    "#;
    console(code, &h).unwrap();
    assert_eq!(h.log.borrow().as_slice(), ["2", "1", "600", "255", "false"]);
    assert!(console(r#"return mg.image("no picture")"#, &host([])).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

fn game() -> Option<Arc<GameData>> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    Some(Arc::new(GameData::open(&install).unwrap()))
}

fn applied(outcome: &Outcome) -> Module {
    let mut ws = Workspace::new(bare());
    ws.apply(Command::new(outcome.label.clone(), outcome.edits.clone())).unwrap();
    ws.flush().unwrap();
    ws.module
}

/// With the game's data: an area made as the Area Wizard makes it, and
/// its terrain painted as the brushes do.
#[test]
fn an_area_is_made_and_painted() {
    let Some(game) = game() else { return };
    let h = host([]);
    let code = r#"
        local rural
        for _, t in ctx.terrain:tilesets() do
            if t.resref == "ttr01" then rural = t.name end
        end
        local resref = ctx.terrain:new_area({ name = "High Field", tileset = "ttr01", width = 8, height = 8 })
        local area = ctx.terrain:open(resref)
        local terrain, height = area:corner(3, 6)
        local painted = area:paint(1, 1, "Water")
        local raised = area:set_height(5, 5, 2)
        local _, top = area:corner(5, 5)
        local _, beside = area:corner(4, 4)
        local tile = area:tile(4, 4)
        -- A road through three cells, a house beside it, and the Eraser on
        -- both: the road's tile and the house go.
        local road = area:cross("Road", { { 1, 5 }, { 1, 6 }, { 2, 6 } })
        local house = area:place_group(area.groups[1].name, 6, 0)
        local over = area:place_group(area.groups[1].name, 6, 0)
        local erased = area:erase(1, 6) and area:erase(6, 0)
        local crooked = pcall(function() return area:cross("Road", { { 0, 1 }, { 2, 2 } }) end)
        ctx.log:info(`{road} {house} {over} {erased} {crooked} {#area.crossers > 0}`)
        local outside = pcall(function() return area:corner(9, 0) end)
        local unknown = pcall(function() return area:paint(1, 1, "Lava") end)
        return rural, resref, area.width, area.tileset, terrain, height, painted,
            (area:corner(1, 1)), raised, top, beside, tile.height >= 1, outside, unknown,
            #area.terrains > 2, area.step
    "#;
    let outcome = run_console(code, Input { module: bare(), game: Some(game) }, h.clone()).unwrap();
    assert_eq!(h.log.borrow_mut().remove(0), "true true true true false true");
    assert_eq!(
        h.log.borrow().as_slice(),
        [
            "\"Rural\"",
            "\"highfield\"",
            "8",
            "\"ttr01\"",
            "\"Grass\"",
            "0",
            "true",
            "\"Water\"",
            "true",
            "2",
            "1",
            "true",
            "false",
            "false",
            "true",
            "5"
        ]
    );
    // What it made is a module's area: listed, with its three files, and
    // tiles that agree with what the plugin read.
    let module = applied(&outcome);
    let key = |name: &str| ResKey::from_filename(name).unwrap();
    for file in ["highfield.are", "highfield.git", "highfield.gic"] {
        assert!(module.contains(&key(file)), "{file}");
    }
    let are = module.gff(&key("highfield.are")).unwrap().unwrap();
    let tiles = are.root.list("Tile_List").unwrap();
    assert_eq!(tiles.len(), 64);
    assert!(tiles.iter().any(|t| t.integer("Tile_Height") == Some(1)));
}

/// The example that imports a tileset's folder: its files go into a hak,
/// and an area is made of a group, the tileset read from the hak that is
/// not written yet.
#[test]
fn a_tileset_folder_becomes_a_hak_and_an_area() {
    let Some(game) = game() else { return };
    // A tileset's folder: the game's rural set under another name, a
    // texture, and a file no hak takes.
    let dir = scratch("tileset");
    let folder = dir.join("export");
    std::fs::create_dir_all(&folder).unwrap();
    let set = game.resman.get(&ResKey::from_filename("ttr01.set").unwrap()).unwrap().into_owned();
    std::fs::write(folder.join("mytiles.set"), &set).unwrap();
    std::fs::write(folder.join("mytiles_001.tga"), "pixels").unwrap();
    std::fs::write(folder.join("readme.docx"), "notes").unwrap();
    let parsed = mg_set::Tileset::parse(&set, mg_core::Codepage::WINDOWS_1252).unwrap();
    let group = parsed.groups.iter().find(|g| g.rows >= 2 && g.columns >= 2).unwrap();

    let plugin = Plugin::load(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/plugins/examples/tileset-import"),
    )
    .unwrap();
    let form = BTreeMap::from([
        ("set".to_string(), serde_json::json!("mytiles.set")),
        ("hak".to_string(), serde_json::json!("mytiles")),
        ("group".to_string(), serde_json::json!(group.name)),
        ("area".to_string(), serde_json::json!("Imported")),
    ]);
    let h =
        host([Some(Answer::Path(folder.clone())), Some(Answer::Values(form)), Some(Answer::Yes)]);
    let input = Input { module: bare(), game: Some(game) };
    let outcome = run_command(&plugin, "import", input, h.clone()).unwrap();
    let log = h.log.borrow().join("\n");
    assert!(log.contains("2 files into mytiles.hak, 1 left out"), "{log}");
    assert!(log.contains("made the area imported with the group"), "{log}");
    assert_eq!(outcome.haks.len(), 1);
    assert_eq!(outcome.haks[0].files.len(), 2);

    let module = applied(&outcome);
    assert_eq!(module.haks().unwrap(), ["mytiles"]);
    let are = module.gff(&ResKey::from_filename("imported.are").unwrap()).unwrap().unwrap();
    assert_eq!(are.root.get("Tileset").and_then(|v| v.as_resref()).unwrap().to_string(), "mytiles");
    assert_eq!(are.root.integer("Width"), Some(i64::from(group.columns)));
    assert_eq!(are.root.integer("Height"), Some(i64::from(group.rows)));
    // The group's tiles are the area's, row by row from the south-west.
    let tiles = are.root.list("Tile_List").unwrap();
    for (i, tile) in group.tiles.iter().enumerate() {
        if let Some(id) = tile {
            assert_eq!(tiles[i].integer("Tile_ID"), Some(i64::from(*id)), "tile {i}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A module whose hak has an `encoding.2da` (byte 0xF0 is "ğ"): a
/// plugin's strings are the table's letters, coming out and going in.
#[test]
fn a_plugin_s_text_is_read_and_written_by_the_module_s_encoding_table() {
    use mg_resman::{LayerClass, MemContainer, ResMan, priority};
    let mut rows = String::from("2DA V2.0\n\n  Codepoint\n");
    for row in 0..240 {
        rows.push_str(&format!("{row} ****\n"));
    }
    rows.push_str("240 0x11f\n");
    let game = |table: bool| {
        let mut hak = MemContainer::new();
        if table {
            hak.insert(ResKey::from_filename("encoding.2da").unwrap(), rows.as_bytes());
        }
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:x", LayerClass::Erf, hak);
        Arc::new(GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH)))
    };
    let module = || {
        let mut m = bare();
        m.set(ResKey::from_filename("x.nss").unwrap(), &b"// da\xf0"[..]);
        m
    };
    let code = r#"
        ctx.edit:write("y.nss", "// ığ")
        return ctx.module:text("x.nss")
    "#;
    let written = |outcome: &Outcome| {
        let mut ws = Workspace::new(module());
        ws.apply(Command::new(outcome.label.clone(), outcome.edits.clone())).unwrap();
        ws.flush().unwrap();
        ws.module.get(&ResKey::from_filename("y.nss").unwrap()).map(<[u8]>::to_vec)
    };
    let h = host([]);
    let input = Input { module: module(), game: Some(game(true)) };
    // ("ı" has no byte in this table: refused, as ever.)
    let refused = run_console(code, input, h.clone()).unwrap_err().to_string();
    assert!(refused.contains("cannot hold"), "{refused}");
    let code = code.replace("ığ", "ğ");
    let input = Input { module: module(), game: Some(game(true)) };
    let outcome = run_console(&code, input, h.clone()).unwrap();
    assert_eq!(h.log.borrow_mut().remove(0), "\"// dağ\"");
    assert_eq!(written(&outcome).unwrap(), b"// \xf0");
    // Without the table: Windows-1252, where "ğ" has no byte.
    let input = Input { module: module(), game: Some(game(false)) };
    let refused = run_console(&code, input, h.clone()).unwrap_err().to_string();
    assert!(refused.contains("cannot hold"), "{refused}");
    let code = code.replace("ğ", "ð");
    let input = Input { module: module(), game: Some(game(false)) };
    let outcome = run_console(&code, input, h.clone()).unwrap();
    assert_eq!(h.log.borrow_mut().remove(0), "\"// dað\"");
    assert_eq!(written(&outcome).unwrap(), b"// \xf0");
}
