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
    run_console(code, Input::new(bare(), None), host.clone())
}

fn scratch(name: &str) -> PathBuf {
    let dir = mg_testkit::scratch_dir(&format!("mg-plugin-{name}"));
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

/// A hak a job wrote is kept as it was and as it was written, so that the
/// command's Undo (or its edits failing) takes it back and Redo writes it
/// again: a hak that was there comes back as it was, one the job made
/// goes; one changed since is left as it is. The copies go with the
/// record.
/// A file put into a hak with `folder:to_hak` is not read to say the
/// game would load it, and `ctx.game:bytes` holds it to what a plugin
/// reads of a file: one of 65 MB (nothing of it on the disk) is there,
/// and too large to read.
#[test]
fn a_file_put_into_a_hak_is_not_read_to_say_it_is_there() {
    let dir = scratch("hak-large");
    let big = std::fs::File::create(dir.join("big.tga")).unwrap();
    big.set_len(65 << 20).unwrap();
    drop(big);
    std::fs::write(dir.join("small.tga"), "pixels").unwrap();
    let code = r#"
        local folder = ctx.ui:open_folder()
        folder:to_hak("my_tiles", "big.tga")
        folder:to_hak("my_tiles", "small.tga")
        local read, why = pcall(function() return ctx.game:bytes("big.tga") end)
        return ctx.game:has("big.tga"), read, tostring(why), ctx.game:bytes("small.tga")
    "#;
    let h = host([Some(Answer::Path(dir.clone())), Some(Answer::Yes)]);
    let started = std::time::Instant::now();
    console(code, &h).unwrap();
    let log = h.log.borrow();
    assert_eq!(
        (log[0].as_str(), log[1].as_str(), log[3].as_str()),
        ("true", "false", "\"pixels\"")
    );
    assert!(log[2].contains("big.tga has 65 MB; a plugin reads files of up to 64"), "{}", log[2]);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_hak_written_is_taken_back_and_written_again() {
    let dir = scratch("hak-undo");
    let haks = dir.join("hak");
    std::fs::create_dir_all(&haks).unwrap();
    let job = |code: &str| {
        let h = host([Some(Answer::Yes)]);
        console(code, &h).unwrap().haks.remove(0)
    };
    let read = |name: &str| {
        let hak = mg_module::hak_edit::Hak::open(&haks.join(format!("{name}.hak"))).unwrap();
        let mut keys: Vec<String> = hak.items().iter().map(|i| i.key.to_string()).collect();
        keys.sort();
        keys
    };
    // There before: added to, then taken back and written again.
    job(r#"ctx.hak:write("there", "one.txt", "1")"#).write(&haks).unwrap();
    let mut done = job(r#"ctx.hak:write("there", "two.txt", "2")"#).write_kept(&haks).unwrap();
    assert!(done.was_there());
    assert_eq!(read("there"), ["one.txt", "two.txt"]);
    done.undo().unwrap();
    assert_eq!(read("there"), ["one.txt"]);
    done.redo().unwrap();
    assert_eq!(read("there"), ["one.txt", "two.txt"]);
    // Made: taken back, it is gone.
    let mut made = job(r#"ctx.hak:write("made", "a.txt", "a")"#).write_kept(&haks).unwrap();
    assert!(!made.was_there());
    made.undo().unwrap();
    assert!(!haks.join("made.hak").exists());
    made.redo().unwrap();
    assert_eq!(read("made"), ["a.txt"]);
    // Changed since (another write): left as it is, and said.
    job(r#"ctx.hak:write("made", "b.txt", "b")"#).write(&haks).unwrap();
    let e = made.undo().unwrap_err();
    assert!(e.contains("was changed since"), "{e}");
    assert_eq!(read("made"), ["a.txt", "b.txt"]);
    // The copies, in the hak folder's hidden folder, go with the records.
    let copies = haks.join(format!("{}{}", mg_plugin::UNDO_FOLDER, std::process::id()));
    assert!(copies.is_dir());
    drop((done, made));
    assert!(!copies.exists(), "the copies go");
    let _ = std::fs::remove_dir_all(&dir);
}

/// What a job puts into haks past what it holds in memory waits in
/// scratch files, read back as the rest (`ctx.game` reads it), written
/// into the hak, and taken away with the job's haks.
#[test]
fn a_job_s_haks_past_its_memory_wait_in_scratch_files() {
    let dir = scratch("hak-held");
    let mb = 1 << 20;
    let n = mg_plugin::HELD_IN_MEMORY / mb + 3;
    let code = format!(
        r#"local chunk = string.rep("x", {mb})
        for i = 1, {n} do ctx.hak:write("big", "part" .. i .. ".txt", chunk .. i) end
        assert(ctx.game:bytes("part{n}.txt") == chunk .. {n})"#
    );
    let h = host([Some(Answer::Yes)]);
    let outcome = console(&code, &h).unwrap();
    let hak = &outcome.haks[0];
    let files = |f: &dyn Fn(&HakData) -> bool| hak.files.iter().filter(|(_, d)| f(d)).count();
    let held = files(&|d| matches!(d, HakData::Bytes(_)));
    assert_eq!(held, mg_plugin::HELD_IN_MEMORY / (mb + 2), "held in memory up to the limit");
    assert_eq!(files(&|d| matches!(d, HakData::File(_))), n - held);
    let scratch_dir = hak.held.as_ref().expect("the job's scratch files").dir().to_path_buf();
    assert!(scratch_dir.is_dir());
    assert_eq!(hak.write(&dir).unwrap(), (n, 0));
    let written = mg_module::hak_edit::Hak::open(&dir.join("big.hak")).unwrap();
    let last = written.data(ResKey::from_filename(&format!("part{n}.txt")).unwrap()).unwrap();
    assert_eq!(last.len(), mb + format!("{n}").len());
    drop(outcome);
    assert!(!scratch_dir.exists(), "the scratch files go with the job's haks");
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
    let outcome = run_console(code, Input::new(bare(), Some(game)), h.clone()).unwrap();
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

/// Every corner of a 32 × 32 area painted in a batch (`area:batch`): the
/// area's tiles read once, what the function reads back is what its
/// strokes left, and the tiles go in as a handful of edits, where one
/// stroke at a time makes thousands; a batch that fails puts in none of
/// its strokes.
#[test]
fn a_batch_of_strokes_goes_in_at_once() {
    let Some(game) = game() else { return };
    let paint = |batched: bool| {
        let code = format!(
            r#"
            local resref = ctx.terrain:new_area({{ name = "Big", tileset = "ttr01", width = 32, height = 32 }})
            local area = ctx.terrain:open(resref)
            local function all()
                local painted = 0
                for y = 0, 32 do
                    for x = 0, 32 do
                        local t = ((x + y) % 7 == 0) and "Water" or "Grass"
                        if area:paint(x, y, t) then painted += 1 end
                    end
                end
                -- What it reads back is what the strokes left.
                assert((area:corner(0, 0)) == "Water")
                return painted
            end
            local painted = 0
            if {batched} then
                painted = area:batch(all)
                -- A batch that fails puts nothing in.
                local ok = pcall(area.batch, area, function()
                    area:paint(16, 16, "Water")
                    error("no")
                end)
                assert(not ok)
            else
                painted = all()
            end
            return painted
            "#
        );
        let h = host([]);
        let outcome = run_console(&code, Input::new(bare(), Some(game.clone())), h.clone())
            .unwrap_or_else(|e| panic!("{e}"));
        let painted: usize = h.log.borrow()[0].parse().unwrap();
        (outcome, painted)
    };
    let (one_by_one, painted) = paint(false);
    assert!(painted > 100, "{painted}");
    let (batched, painted_too) = paint(true);
    assert!(painted_too > 100, "{painted_too}");
    // (The area's own files, then the tiles: past a few, as one list.)
    let made =
        |o: &Outcome| o.edits.iter().filter(|e| e.key().restype == mg_core::ResType::ARE).count();
    assert!(made(&one_by_one) > 1000, "{} edits one by one", made(&one_by_one));
    assert!(made(&batched) <= 3, "{} edits in a batch", made(&batched));
    // What went in is what the plugin read: water on corner (0, 0)'s tile.
    let module = applied(&batched);
    let are = module.gff(&ResKey::from_filename("big.are").unwrap()).unwrap().unwrap();
    assert_eq!(are.root.list("Tile_List").unwrap().len(), 32 * 32);
    // (A tile set as it is given, in a batch: the strokes before it go in
    // first, and it is read back.)
    let check = r#"
        local area = ctx.terrain:open("big")
        area:batch(function()
            area:paint(24, 24, "Water")
            area:set_tile(30, 30, 0)
            assert(area:tile(30, 30).id == 0 and (area:corner(24, 24)) == "Water")
        end)
        return (area:corner(0, 0)), (area:corner(16, 16))
    "#;
    let h = host([]);
    run_console(check, Input::new(module, Some(game)), h.clone()).unwrap();
    assert_eq!(
        h.log.borrow().as_slice(),
        ["\"Water\"", "\"Grass\""],
        "the failed batch left nothing"
    );
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
    let h = host([Some(Answer::Path(folder)), Some(Answer::Values(form)), Some(Answer::Yes)]);
    let input = Input::new(bare(), Some(game));
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
    let input = Input::new(module(), Some(game(true)));
    // ("ı" has no byte in this table: refused, as ever.)
    let refused = run_console(code, input, h.clone()).unwrap_err().to_string();
    assert!(refused.contains("cannot hold"), "{refused}");
    let code = code.replace("ığ", "ğ");
    let input = Input::new(module(), Some(game(true)));
    let outcome = run_console(&code, input, h.clone()).unwrap();
    assert_eq!(h.log.borrow_mut().remove(0), "\"// dağ\"");
    assert_eq!(written(&outcome).unwrap(), b"// \xf0");
    // Without the table: Windows-1252, where "ğ" has no byte.
    let input = Input::new(module(), Some(game(false)));
    let refused = run_console(&code, input, h.clone()).unwrap_err().to_string();
    assert!(refused.contains("cannot hold"), "{refused}");
    let code = code.replace("ğ", "ð");
    let input = Input::new(module(), Some(game(false)));
    let outcome = run_console(&code, input, h.clone()).unwrap();
    assert_eq!(h.log.borrow_mut().remove(0), "\"// dað\"");
    assert_eq!(written(&outcome).unwrap(), b"// \xf0");
}

/// A plugin in a scratch folder, with `sections` after its `[plugin]`.
fn written(name: &str, sections: &str, code: &str) -> Plugin {
    let dir = scratch(name);
    let cfg = format!(
        "[plugin]\nid = \"test.{name}\"\nname = \"Writer\"\nversion = \"1\"\napi = \"0.3\"\n\
         license = \"GPL-3.0-or-later\"\n{sections}"
    );
    std::fs::write(dir.join("plugin.cfg"), cfg).unwrap();
    std::fs::write(dir.join("main.luau"), code).unwrap();
    Plugin::load(&dir).unwrap()
}

/// Files go into a folder the user chose, with their leave, when the
/// handler has returned; nothing that leads out of the folder.
#[test]
fn a_folder_the_user_chose_is_written_into_with_their_leave() {
    let dir = scratch("folder-write");
    std::fs::create_dir_all(dir.join("out/sub")).unwrap();
    std::fs::write(dir.join("out/old.2da"), "old").unwrap();
    std::fs::write(dir.join("secret.txt"), "secret").unwrap();
    let out = dir.join("out");
    let code = r#"
        local folder = ctx.ui:open_folder({ title = "Generated" })
        folder:write("old.2da", "new")
        folder:write("feat.2da", "first")
        folder:write("feat.2da", "2DA V2.0")
        return #folder:files()
    "#;
    // Refused: the job fails, and nothing is handed back.
    let h = host([Some(Answer::Path(out.clone())), Some(Answer::No)]);
    let refused = console(code, &h).unwrap_err().to_string();
    assert!(refused.contains("was not allowed"), "{refused}");
    // Allowed (asked once for the folder): written when the host says.
    let h = host([Some(Answer::Path(out.clone())), Some(Answer::Yes)]);
    let outcome = console(code, &h).unwrap();
    assert_eq!(h.asked.borrow().len(), 2);
    let Question::Confirm(text) = &h.asked.borrow()[1] else { panic!() };
    assert!(text.contains("write files into the folder") && text.contains("out"), "{text}");
    assert_eq!(std::fs::read(out.join("old.2da")).unwrap(), b"old", "not while it runs");
    assert_eq!(outcome.folders.len(), 1);
    assert_eq!(outcome.folders[0].write().unwrap(), (1, 1));
    assert_eq!(std::fs::read(out.join("old.2da")).unwrap(), b"new");
    assert_eq!(std::fs::read(out.join("feat.2da")).unwrap(), b"2DA V2.0");

    // A name is a name: no way out of the folder, into one, or hidden.
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.join("secret.txt"), out.join("link.txt")).unwrap();
    for name in ["../secret.txt", "sub/x.txt", "/tmp/x", ".hidden", "", "sub", "a\\b", "link.txt"] {
        if name == "link.txt" && cfg!(not(unix)) {
            continue;
        }
        let code = format!("ctx.ui:open_folder():write({name:?}, 'x')");
        let h = host([Some(Answer::Path(out.clone())), Some(Answer::Yes)]);
        let e = console(&code, &h).unwrap_err().to_string();
        assert!(e.contains("not a file"), "{name}: {e}");
    }
    assert_eq!(std::fs::read(dir.join("secret.txt")).unwrap(), b"secret");
    // (Nor past the check when it is written: a link put there meanwhile.)
    let sneaky =
        mg_plugin::FolderWrite { dir: out.clone(), files: vec![("link.txt".into(), vec![1])] };
    if cfg!(unix) {
        assert!(sneaky.write().is_err());
        assert_eq!(std::fs::read(dir.join("secret.txt")).unwrap(), b"secret");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// What a plugin does on an event is declared, registered with `mg.on`,
/// and run as a command is, with what happened; a check writes nothing.
#[test]
fn an_event_s_handler_gets_what_happened() {
    use mg_plugin::{Event, EventKind, run_check, run_event};
    let dir = scratch("event-out");
    let plugin = written(
        "events",
        "[event]\nid = \"emit\"\non = \"module-saved\"\n[check]\nid = \"writes\"\ntitle = \"Writes\"\n",
        r#"
        local mg = require("@moonglow")
        mg.on("emit", function(ctx)
            ctx.log:info(`{ctx.event.name}: {table.concat(ctx.event.resources, " ")}`)
            local folder = ctx.ui:open_folder({ title = "Development" })
            folder:write("saved.txt", table.concat(ctx.event.resources, "\n"))
            ctx.edit:write("note.nss", "// saved\n")
            return "Emit"
        end)
        mg.check("writes", function(ctx)
            ctx.ui:open_folder():write("x.txt", "x")
        end)
        "#,
    );
    assert_eq!(plugin.manifest.events[0].on, EventKind::ModuleSaved);
    let happened =
        Event::new(EventKind::ModuleSaved, vec![ResKey::from_filename("guard.utc").unwrap()]);
    let input = || Input::new(bare(), None);
    let h = host([Some(Answer::Path(dir.clone())), Some(Answer::Yes)]);
    let outcome = run_event(&plugin, "emit", &happened, input(), h.clone()).unwrap();
    assert_eq!(h.log.borrow().as_slice(), ["module-saved: guard.utc"]);
    assert_eq!((outcome.label.as_str(), outcome.edits.len()), ("Emit", 1));
    outcome.folders[0].write().unwrap();
    assert_eq!(std::fs::read(dir.join("saved.txt")).unwrap(), b"guard.utc");
    assert!(run_event(&plugin, "other", &happened, input(), h).is_err());
    // A check only reads.
    let h = host([Some(Answer::Path(dir.clone())), Some(Answer::Yes)]);
    let e = run_check(&plugin, "writes", input(), h).unwrap_err().to_string();
    assert!(e.contains("a check only reads"), "{e}");
    // A manifest names an event that there is.
    let bad = std::panic::catch_unwind(|| {
        written("bad-event", "[event]\nid = \"x\"\non = \"midnight\"\n", "")
    });
    assert!(bad.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A folder's files are written whole, each in the place of the one of
/// its name: nothing is left beside them, and a link of that name (to a
/// file of the folder) is replaced, not written through.
#[test]
fn a_folder_s_files_are_written_whole() {
    let dir = scratch("folder-whole");
    std::fs::write(dir.join("a.txt"), "old").unwrap();
    let files = vec![("a.txt".to_string(), b"new".to_vec()), ("b.txt".to_string(), b"b".to_vec())];
    let write = mg_plugin::FolderWrite { dir: dir.clone(), files };
    assert_eq!(write.write().unwrap(), (1, 1));
    assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"new");
    let names = || {
        let mut names: Vec<String> = (std::fs::read_dir(&dir).unwrap().flatten())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    assert_eq!(names(), ["a.txt", "b.txt"]);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.join("a.txt"), dir.join("link.txt")).unwrap();
        let files = vec![("link.txt".to_string(), b"mine".to_vec())];
        let write = mg_plugin::FolderWrite { dir: dir.clone(), files };
        assert_eq!(write.write().unwrap(), (0, 1));
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"new", "written through the link");
        assert!(!dir.join("link.txt").symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read(dir.join("link.txt")).unwrap(), b"mine");
        assert_eq!(names(), ["a.txt", "b.txt", "link.txt"]);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
