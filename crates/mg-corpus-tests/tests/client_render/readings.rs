//! Readings of the game client that answer questions of the deferred list
//! (the deferred sprint's Wave 1, 2026-10-29): small scenes, a client run
//! or two each, whose pictures were looked at. Each test writes the
//! client's pictures to its scratch directory
//! (`target/test-output/client_reading_<name>/`) and prints where; what
//! was seen is in its comment. They check little themselves: they are
//! kept so that a reading can be made again (another game version, a
//! change of Moonglow's beside it).
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=:2 cargo test -p mg-corpus-tests --release --test client_render reading -- --ignored --nocapture`.

use super::*;
use mg_module::instances::{Placement, Placing, instance};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The player made invisible (a statement of an entry script's body).
const INVISIBLE: &str = "ApplyEffectToObject(DURATION_TYPE_PERMANENT, \
                         EffectVisualEffect(VFX_DUR_CUTSCENE_INVISIBILITY), pc);";

/// An entry script: `body` run as the player enters (with `pc`, `area`
/// and `p`, the start position, in scope), the camera held at `(facing,
/// distance, pitch)`, `MG_READY` written to the log four seconds after.
fn enter(body: &str, (facing, distance, pitch): (f32, f32, f32)) -> String {
    // (Given at once, the client keeps its own camera but for the
    // reference scenes' (which its view was fitted to): another is given a
    // second later, as `save_area_module_facing` does.)
    let camera = format!(
        "AssignCommand(pc, SetCameraFacing({facing:.1}, {distance:.1}, {pitch:.1}, \
         CAMERA_TRANSITION_TYPE_SNAP))"
    );
    let camera = if (facing, distance, pitch) == (90.0, 12.0, 45.0) {
        format!("{camera};")
    } else {
        format!("DelayCommand(1.0, {camera});")
    };
    format!(
        r#"
void Log(string s) {{ WriteTimestampedLogEntry(s); }}
void main()
{{
    object pc = GetEnteringObject();
    if (!GetIsPC(pc)) return;
    location start = GetStartingLocation();
    object area = GetAreaFromLocation(start);
    vector p = GetPositionFromLocation(start);
    {body}
    {camera}
    LockCameraDirection(pc, TRUE);
    LockCameraPitch(pc, TRUE);
    LockCameraDistance(pc, TRUE);
    DelayCommand(4.0, Log("MG_READY"));
}}
"#
    )
}

/// A reading's module, before it is run: the game (with the scratch user
/// directory's `override`), the module and its area's ARE and GIT.
struct Reading {
    dir: PathBuf,
    game: GameData,
    m: mg_module::Module,
    are: Gff,
    git: Gff,
}

impl Reading {
    /// A new module of one 4×4 area of `tileset` lit by `light`, the files
    /// `overrides` in the scratch user directory's `override` (the game
    /// read with them).
    fn new(name: &str, tileset: &str, light: Lighting, overrides: &[(&str, Vec<u8>)]) -> Reading {
        let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
        let dir = scratch_dir(&format!("client_reading_{name}"));
        let over = dir.join("user/override");
        std::fs::create_dir_all(&over).unwrap();
        for (file, data) in overrides {
            std::fs::write(over.join(file), data).unwrap();
        }
        let game = GameData::open(&GameInstall::new(&root, Some(dir.join("user")), "en")).unwrap();
        let (m, are) = build_module_on(&game, &dir, light, tileset, 0);
        let git_key = *m.keys_of(ResType::GIT).next().unwrap();
        let git = m.gff(&git_key).unwrap().unwrap();
        Reading { dir, game, m, are, git }
    }

    /// A placeable of `appearance` placed in the area (the armoire's
    /// blueprint, made that appearance), with `fields` set.
    fn place(&mut self, appearance: u32, at: [f32; 3], facing: f32, fields: &[(&str, Value)]) {
        let bp =
            Gff::read(&self.game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
        let none = |_: ResRef| None;
        let placing = Placing { game: &self.game, item: &none };
        let at = Placement { position: at, rotation: facing.to_radians() };
        let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
        p.set("Appearance", Value::Dword(appearance));
        for (label, v) in fields {
            p.set(label, v.clone());
        }
        let mut list = self.git.root.list("Placeable List").map(<[_]>::to_vec).unwrap_or_default();
        list.push(p);
        self.git.root.set("Placeable List", Value::List(list));
    }

    /// Saves the module with the entry script `enter` and the client's
    /// `settings`.
    fn save(&mut self, enter: &str, settings: &str) {
        let are_key = *self.m.keys_of(ResType::ARE).next().unwrap();
        let git_key = *self.m.keys_of(ResType::GIT).next().unwrap();
        self.m.set_gff(are_key, &self.are).unwrap();
        self.m.set_gff(git_key, &self.git).unwrap();
        self.m.set(
            ResKey::parse("mg_enter", ResType::NCS).unwrap(),
            compile(&self.dir, "mg_enter", enter),
        );
        std::fs::write(self.dir.join("user/settings.tml"), settings).unwrap();
        self.m
            .save_as(&ModuleLocation::Archive(self.dir.join("user/modules/MgScene.mod")))
            .unwrap();
    }

    /// [`Reading::save`], then the client's picture of the scene, kept as
    /// `client.png` in the scratch directory.
    fn shoot(mut self, enter: &str, settings: &str) -> (PathBuf, Rgba) {
        self.save(enter, settings);
        let Some(shot) = client_screenshot(&self.dir, "MgScene") else {
            panic!("the client did not reach the scene (see {})", self.dir.display());
        };
        eprintln!("the client's picture: {}", self.dir.join("client.png").display());
        (self.dir, shot)
    }

    /// [`Reading::shoot`] with the reference scenes' camera (looking north
    /// from the middle of the area: `SetCameraFacing(90, 12, 45)`, whose
    /// view was fitted), and Moonglow's area view of the same area from the
    /// same camera beside it (`moonglow.png` with its own shader,
    /// `moonglow_on.png` with the game's). `view` changes the area view's
    /// settings.
    fn shoot_beside(
        self,
        body: &str,
        settings: &str,
        view: &dyn Fn(&mut mg_area::View),
    ) -> (PathBuf, Rgba) {
        let (game, are, git) = (
            GameData::open(&GameInstall::new(
                mg_testkit::nwn_root().unwrap(),
                Some(self.dir.join("user")),
                "en",
            ))
            .unwrap(),
            self.are.clone(),
            self.git.clone(),
        );
        let (dir, client) = self.shoot(&enter(&logged(body), (90.0, 12.0, 45.0)), settings);
        moonglow_beside(&dir, &game, &are, &git, &client, view);
        (dir, client)
    }
}

/// Moonglow's area view of the area `are` and `git` beside the client's
/// picture `client` of it from the reference scenes' camera
/// ([`Reading::shoot_beside`]), the grass grown where it is seen.
fn moonglow_beside(
    dir: &Path,
    game: &GameData,
    are: &Gff,
    git: &Gff,
    client: &Rgba,
    view: &dyn Fn(&mut mg_area::View),
) {
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("Moonglow's side skipped: no GPU");
        return;
    };
    let tileset = mg_area::tileset(game, are.root.resref("Tileset").unwrap()).ok();
    let model = mg_area::AreaModel::read(game, &are.root, &git.root, tileset.as_ref());
    let area_scene = mg_area::AreaScene::new(&gpu, game, &model);
    let pc = player_stood(dir, game, &model, glam::Vec2::new(20.0, 20.0));
    let camera = fitted_camera(pc, FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
    let size = (client.width, client.height);
    let sight = mg_area::Sight::of(&camera, size.0 as f32 / size.1 as f32);
    let mut v = mg_area::View { fog: false, grass: Some(sight), ..mg_area::View::of(&model) };
    view(&mut v);
    let mut scene = area_scene.scene(&model, &v);
    // (Where nothing is drawn: magenta, told apart from black meshes.)
    if std::env::var_os("MG_MAGENTA").is_some() {
        scene.background = [1.0, 0.0, 1.0];
    }
    off_and_on(&gpu, game, &scene, &camera, size, Some(client), dir, "moonglow");
}

/// The entry script of [`Reading::shoot_beside`]: the player's place
/// written to the log, for Moonglow's camera.
fn logged(body: &str) -> String {
    format!(
        "{body}\n    DelayCommand(3.5, Log(\"MG_PC \" + FloatToString(GetPosition(pc).x, 0, 3) + \" \" \
         + FloatToString(GetPosition(pc).y, 0, 3) + \" \" + FloatToString(GetPosition(pc).z, 0, 3)));"
    )
}

/// The lines of the client's log that the entry script wrote (`MG_`).
fn print_log(dir: &Path) {
    let log = std::fs::read_to_string(dir.join("user/logs/nwclientLog1.txt")).unwrap_or_default();
    for line in log.lines().filter(|l| l.contains("MG_")) {
        eprintln!("{line}");
    }
    eprintln!("{}", dir.display());
}

/// Runs the client with `args` (no entry script to wait for) and
/// screenshots its window after `wait`, as `<dir>/client.png`; then, if
/// `then` is given, runs it and, where it says so, screenshots the window
/// again (`<dir>/client_then.png`). The first picture, and what `then`
/// said.
fn client_after(
    dir: &Path,
    args: &[&str],
    wait: Duration,
    then: Option<&dyn Fn(&Path) -> bool>,
) -> (Option<Rgba>, bool) {
    let _ = std::fs::remove_file(dir.join("user/logs/nwclientLog1.txt"));
    let Ok(mut child) = Command::new(common::repo().join("tools/nwclient/run-client.sh"))
        .arg(dir)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return (None, false);
    };
    let winshot = |name: &str| {
        let shot = dir.join(name);
        Command::new("python3")
            .arg(common::repo().join("tools/aurora/xdrive.py"))
            .args(["winshot", "Neverwinter Nights: Enhanced Edition"])
            .arg(&shot)
            .status()
            .is_ok_and(|s| s.success())
            .then(|| common::read_png(&shot))
    };
    std::thread::sleep(wait);
    let first = winshot("client.png");
    let mut said = false;
    if let Some(then) = then.filter(|_| first.is_some()) {
        said = then(dir);
        if said {
            std::thread::sleep(Duration::from_secs(3));
            winshot("client_then.png");
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    (first, said)
}

/// Test Module's Choose Character (`+LoadNewModule <module>`, as
/// `mg_ui::test_module::command` builds it with `choose`): the client
/// opens the module and shows its character selection, not the main menu
/// (seen 2026-10-29, 89.8193.37: "Select a character" over the
/// module's name, with New and the local vault's characters; the scratch
/// vault has none). The module is the reference scene's.
#[test]
#[ignore]
fn reading_choose_character() {
    let light = Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
    let mut r = Reading::new("choose_character", "tic01", light, &[]);
    r.save(&enter(INVISIBLE, (90.0, 12.0, 45.0)), SETTINGS);
    // The selection, then Play (the first character chosen): the module's
    // area, entered (its entry script's `MG_READY`).
    let play = |dir: &Path| {
        let xdrive = common::repo().join("tools/aurora/xdrive.py");
        let out = Command::new("python3").arg(&xdrive).arg("windows").output().unwrap();
        let listed = String::from_utf8_lossy(&out.stdout).into_owned();
        let line = listed.lines().find(|l| l.contains("Neverwinter Nights")).unwrap();
        let geometry = line.split('\t').nth(1).unwrap();
        let (at, size) = geometry.split_once(' ').unwrap();
        let (x, y) = at.split_once(',').unwrap();
        let (w, h) = size.split_once('x').unwrap();
        let n = |s: &str| s.parse::<i32>().unwrap();
        // (Play, at (615, 615) of a 1024×768 window.)
        let (px, py) = (n(x) + n(w) * 615 / 1024, n(y) + n(h) * 615 / 768);
        Command::new("python3")
            .arg(&xdrive)
            .args(["click", &px.to_string(), &py.to_string()])
            .status()
            .unwrap();
        let log = dir.join("user/logs/nwclientLog1.txt");
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(90) {
            if std::fs::read_to_string(&log).is_ok_and(|l| l.contains("MG_READY")) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        false
    };
    let (shot, entered) =
        client_after(&r.dir, &["+LoadNewModule", "MgScene"], Duration::from_secs(40), Some(&play));
    assert!(shot.is_some(), "no picture of the client (see {})", r.dir.display());
    assert!(entered, "Play did not enter the module (see {})", r.dir.display());
    eprintln!("the client's pictures: {}", r.dir.display());
}

/// The inventory's armor icons: a performer's outfit (`nw_aarcl004`)
/// whose own colors are all 0, beside a copy whose torso part has colors
/// of its own (44, the part's six), both in the player's bag (made in a
/// chest beside the player, whose copies made on the way fill it, then
/// taken), the inventory open (`PopUpGUIPanel`), the player a man and
/// (`female`) made a woman (`SetGender`).
fn inventory(female: bool) -> (PathBuf, Rgba) {
    let light = Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
    let name = if female { "inventory_female" } else { "inventory_male" };
    let r = Reading::new(name, "tic01", light, &[]);
    // (The player of `+TestNewModule` is the first pregenerated
    // character, a woman.)
    let gender =
        if female { "SetGender(pc, GENDER_FEMALE);" } else { "SetGender(pc, GENDER_MALE);" };
    let body = format!(
        r#"{gender}
    object chest = CreateObject(OBJECT_TYPE_PLACEABLE, "plc_chest1", Location(area, Vector(p.x + 1.0, p.y, p.z), 0.0));
    object plain = CreateItemOnObject("nw_aarcl004", chest);
    int c;
    for (c = 0; c < 6; c++) {{
        object o = plain;
        plain = CopyItemAndModify(o, ITEM_APPR_TYPE_ARMOR_COLOR, c, 0, TRUE);
        DestroyObject(o);
    }}
    object parted = CopyItem(plain, chest, TRUE);
    for (c = 0; c < 6; c++) {{
        object o = parted;
        parted = CopyItemAndModify(o, ITEM_APPR_TYPE_ARMOR_COLOR,
            ITEM_APPR_ARMOR_NUM_COLORS + ITEM_APPR_ARMOR_MODEL_TORSO * ITEM_APPR_ARMOR_NUM_COLORS + c, 44, TRUE);
        DestroyObject(o);
    }}
    SetName(plain, "MG plain");
    SetName(parted, "MG parted");
    Log("MG_ITEMS plain own leather1 " + IntToString(GetItemAppearance(plain, ITEM_APPR_TYPE_ARMOR_COLOR, 0))
        + ", parted torso leather1 " + IntToString(GetItemAppearance(parted, ITEM_APPR_TYPE_ARMOR_COLOR,
            ITEM_APPR_ARMOR_NUM_COLORS + ITEM_APPR_ARMOR_MODEL_TORSO * ITEM_APPR_ARMOR_NUM_COLORS)));
    DelayCommand(0.5, AssignCommand(pc, ActionTakeItem(plain, chest)));
    DelayCommand(0.6, AssignCommand(pc, ActionTakeItem(parted, chest)));
    DelayCommand(2.5, PopUpGUIPanel(pc, GUI_PANEL_INVENTORY));"#
    );
    r.shoot(&enter(&body, (90.0, 12.0, 45.0)), SETTINGS)
}

/// See [`inventory`]. Seen 2026-10-29 (89.8193.37): see the entry's
/// verdict in `docs/log.md`.
#[test]
#[ignore]
fn reading_inventory_icons() {
    for female in [false, true] {
        let (dir, _) = inventory(female);
        print_log(&dir);
    }
}

/// A placeable's Illuminated (`LightState`) and its flame: two braziers
/// (placeables.2da row 57, `PLC_I05`, `LightColor` 6) in a dark interior,
/// the west one Illuminated, the east one not, both in their default
/// animation state.
#[test]
#[ignore]
fn reading_illuminated_brazier() {
    let light = Lighting { ambient: 0x101010, diffuse: 0x101010, main_light: 0, only_tile: None };
    let mut r = Reading::new("illuminated", "tic01", light, &[]);
    for (x, on) in [(18.5f32, 1u8), (21.5, 0)] {
        r.place(57, [x, 23.0, 0.0], 270.0, &[(mg_rules::placeables::LIGHT_STATE, Value::Byte(on))]);
    }
    let (dir, _) = r.shoot(&enter(INVISIBLE, (90.0, 8.0, 60.0)), SETTINGS);
    eprintln!("{}", dir.display());
}

/// An 8×8 TGA of one color.
fn solid_tga(rgb: [u8; 3]) -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 32, 8];
    for _ in 0..64 {
        t.extend([rgb[2], rgb[1], rgb[0], 255]);
    }
    t
}

/// A tile's replacement texture (`Tile_ReplaceTex`, a `replacetexture.2da`
/// row) in the client: the area's tiles (tms01) made `tms01_c08_01` (the
/// one model with a `replace_tex` texture, a flag; its text renamed, in the
/// override), every tile's field `value` (`None`: left out). The game has
/// neither `replace_tex` nor `T_Flag02`; unless `stock`, `replace_tex` is
/// red here and the table's rows 0 and 1 name a blue and a green texture.
/// The client's picture, if it reached the area.
fn replacement(name: &str, value: Option<u8>, stock: bool) -> Option<Rgba> {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let table = "2DA V2.0\r\n\r\n       TEXTURENAME\r\n0      mgr_row0\r\n1      mgr_row1\r\n";
    let overrides = if stock {
        Vec::new()
    } else {
        vec![
            ("replacetexture.2da", table.as_bytes().to_vec()),
            ("replace_tex.tga", solid_tga([255, 0, 0])),
            ("mgr_row0.tga", solid_tga([0, 0, 255])),
            ("mgr_row1.tga", solid_tga([0, 255, 0])),
        ]
    };
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new(&format!("replacement_{name}"), "tms01", light, &overrides);
    let tiles = r.are.root.list_mut("Tile_List").unwrap();
    let set = mg_set::Tileset::parse(
        &base.resman.get_named("tms01", ResType::SET).unwrap(),
        base.language.codepage(),
    )
    .unwrap();
    let mut models: Vec<String> = Vec::new();
    for t in tiles.iter_mut() {
        let id = t.integer("Tile_ID").unwrap() as usize;
        models.push(set.tiles[id].model.to_ascii_lowercase());
        match value {
            Some(v) => t.set("Tile_ReplaceTex", Value::Byte(v)),
            None => {
                t.remove("Tile_ReplaceTex");
            }
        }
    }
    models.sort();
    models.dedup();
    let flag = base.resman.get_named("tms01_c08_01", ResType::MDL).unwrap();
    for model in &models {
        let text = String::from_utf8_lossy(&flag)
            .replace("TMS01_C08_01", model)
            .replace("tms01_c08_01", model);
        std::fs::write(r.dir.join(format!("user/override/{model}.mdl")), text).unwrap();
    }
    eprintln!("{name}: the area's tiles {models:?} made tms01_c08_01, Tile_ReplaceTex {value:?}");
    r.save(&enter(&logged(INVISIBLE), (90.0, 12.0, 45.0)), SETTINGS);
    let shot = client_screenshot(&r.dir, "MgScene");
    eprintln!("{name}: reached the area: {} ({})", shot.is_some(), r.dir.display());
    if let Some(client) = &shot {
        let game =
            GameData::open(&GameInstall::new(&root, Some(r.dir.join("user")), "en")).unwrap();
        moonglow_beside(&r.dir, &game, &r.are, &r.git, client, &|_| {});
    }
    shot
}

/// See [`replacement`]. Seen 2026-10-29 (89.8193.37): see `docs/log.md`.
#[test]
#[ignore]
fn reading_tile_replacement_texture() {
    let only = std::env::var("MG_REPLACE").ok();
    for (name, value, stock) in [
        ("none", None, false),
        ("zero", Some(0u8), false),
        ("one", Some(1), false),
        ("two", Some(2), false),
        ("three", Some(3), false),
        ("max", Some(255), false),
        ("stock_zero", Some(0), true),
        ("stock_one", Some(1), true),
    ] {
        if only.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        replacement(name, value, stock);
    }
}

/// Tiles far below and above the ground (`Tile_Height` −2 and 20, ttr01's
/// transition 5 m) with an armoire on each, where the server stood them
/// (`a_tile_s_own_height_stands_where_the_engine_puts_it`), as the client
/// and Moonglow's area view draw them: from the middle of the area
/// (`tile_heights_ground`), and with the player entering on the high tile
/// (`tile_heights_high`).
#[test]
#[ignore]
fn reading_tile_heights() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    // (From the middle, the server stood the player on the high tile: the
    // ground's player enters south of it.)
    for (name, entry) in [
        ("ground", [20.0f32, 12.0, 0.0]),
        ("high", [25.0, 25.0, 100.0]),
        ("low", [15.0, 25.0, -10.0]),
    ] {
        let mut r = Reading::new(&format!("tile_heights_{name}"), "ttr01", light, &[]);
        let tiles = r.are.root.list_mut("Tile_List").unwrap();
        tiles[9].set("Tile_Height", Value::Int(-2));
        tiles[10].set("Tile_Height", Value::Int(20));
        r.place(0, [15.0, 27.0, -10.0], 270.0, &[]);
        r.place(0, [25.0, 27.0, 100.0], 270.0, &[]);
        let [x, y, z] = entry;
        let mut info = r.m.info().unwrap();
        info.root.write(&ifo::MOD_ENTRY_X, x);
        info.root.write(&ifo::MOD_ENTRY_Y, y);
        info.root.write(&ifo::MOD_ENTRY_Z, z);
        r.m.set_info(&info).unwrap();
        let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
        eprintln!("{}", dir.display());
    }
}

/// The client's settings with its grass on (Grass Detail: High).
fn with_grass() -> String {
    let s = SETTINGS.replace("[graphics.grass]\n\t\tmode = 0", "[graphics.grass]\n\t\tmode = 2");
    assert!(s.contains("mode = 2"));
    s
}

/// A slab of `side` metres, 2 m high (above the tileset grass's 1.2 m),
/// its top walkable grass (its
/// walkmesh's material 3) and its texture white: the model and walkmesh.
fn slab(name: &str, side: f32) -> (String, String) {
    let h = side / 2.0;
    let top = [[-h, -h, 2.0], [h, -h, 2.0], [h, h, 2.0], [-h, h, 2.0]];
    let sides = [
        [[-h, -h, 0.0], [h, -h, 0.0], [h, -h, 2.0], [-h, -h, 2.0]],
        [[h, -h, 0.0], [h, h, 0.0], [h, h, 2.0], [h, -h, 2.0]],
        [[h, h, 0.0], [-h, h, 0.0], [-h, h, 2.0], [h, h, 2.0]],
        [[-h, h, 0.0], [-h, -h, 0.0], [-h, -h, 2.0], [-h, h, 2.0]],
    ];
    let mut verts = String::new();
    let mut faces = String::new();
    for (k, q) in std::iter::once(top).chain(sides).enumerate() {
        for [x, y, z] in q {
            verts += &format!("    {x} {y} {z}\n");
        }
        let b = k * 4;
        faces += &format!(
            "    {} {} {} 1 0 1 2 0\n    {} {} {} 1 0 2 3 0\n",
            b,
            b + 1,
            b + 2,
            b,
            b + 2,
            b + 3
        );
    }
    let tverts = "  tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n";
    let mdl = format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
         node trimesh {name}_slab\n  parent {name}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
         ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap mgwhite\n  \
         render 1\n  shadow 0\n  verts 20\n{verts}{tverts}  faces 10\n{}endnode\n\
         endmodelgeom {name}\ndonemodel {name}\n",
        faces
    );
    let pwk = format!(
        "# model: {name}_pwk\nnode trimesh {name}_wg\n  parent {name}_pwk\n  position 0 0 0\n  \
         orientation 0 0 0 0\n  bitmap NULL\n  verts 4\n    {} {} 2.0\n    {} {} 2.0\n    \
         {} {} 2.0\n    {} {} 2.0\n  faces 2\n    0 1 2 1 0 0 0 3\n    0 2 3 1 0 0 0 3\nendnode\n",
        -h, -h, h, -h, h, h, -h, h
    );
    (mdl, pwk)
}

/// placeables.2da with rows for `models` after the game's (each the
/// armoire's row with its model and `Static`), and the first new row's
/// number.
fn placeables_with(game: &GameData, models: &[(&str, u8)]) -> (String, u32) {
    let table = game.resman.get_named("placeables", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).into_owned();
    let header: Vec<String> =
        table.lines().nth(2).unwrap().split_whitespace().map(str::to_owned).collect();
    let column = |name: &str| header.iter().position(|c| c == name).unwrap() + 1;
    let first = table.lines().count() - 3;
    let armoire: Vec<String> =
        table.lines().nth(3).unwrap().split_whitespace().map(str::to_owned).collect();
    if !table.ends_with('\n') {
        table.push('\n');
    }
    for (i, (model, is_static)) in models.iter().enumerate() {
        let mut row = armoire.clone();
        row[0] = (first + i).to_string();
        row[column("ModelName")] = (*model).to_owned();
        row[column("Static")] = is_static.to_string();
        table += &(row.join(" ") + "\n");
    }
    (table, first as u32)
}

/// Grass on a placeable's walkmesh: two slabs of 4 m ([`slab`]) on a
/// rural field (ttr01) with the client's grass on, north of the player,
/// the west one static (merged into the tiles) and the east one not, as
/// the client and Moonglow's area view draw them.
#[test]
#[ignore]
fn reading_grass_on_placeables() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let (mdl, pwk) = slab("mg_slab", 4.0);
    let (table, first) = placeables_with(&base, &[("mg_slab", 1), ("mg_slab", 0)]);
    let overrides = [
        ("mg_slab.mdl", mdl.into_bytes()),
        ("mg_slab.pwk", pwk.into_bytes()),
        ("mgwhite.tga", white_tga()),
        ("placeables.2da", table.into_bytes()),
    ];
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("grass_on_placeables", "ttr01", light, &overrides);
    for (row, x, is_static) in [(first, 16.5f32, 1u8), (first + 1, 23.5, 0)] {
        r.place(row, [x, 25.0, 0.0], 0.0, &[("Static", Value::Byte(is_static))]);
    }
    let (dir, _) = r.shoot_beside(INVISIBLE, &with_grass(), &|_| {});
    eprintln!("{}", dir.display());
    // The static slab's walkmesh is the area's: the player entering over
    // it stands on its top (2 m up), seen from close.
    let mut r = Reading::new("grass_on_placeables_top", "ttr01", light, &overrides);
    r.place(first, [16.5, 25.0, 0.0], 0.0, &[("Static", Value::Byte(1))]);
    let mut info = r.m.info().unwrap();
    info.root.write(&ifo::MOD_ENTRY_X, 16.5);
    info.root.write(&ifo::MOD_ENTRY_Y, 24.0);
    info.root.write(&ifo::MOD_ENTRY_Z, 2.0);
    r.m.set_info(&info).unwrap();
    let (dir, _) = r.shoot(&enter(&logged(INVISIBLE), (90.0, 5.0, 70.0)), &with_grass());
    print_log(&dir);
}

/// A grass tuft's shape: a rural field's grass (ttr01) from low and
/// close (`SetCameraFacing(90, 3, 85)`), the player invisible.
#[test]
#[ignore]
fn reading_grass_tufts() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    // From the side and from above (a quad seen edge on is a line, crossed
    // ones a cross).
    for (name, pitch) in [("side", 85.0), ("above", 15.0)] {
        let r = Reading::new(&format!("grass_tufts_{name}"), "ttr01", light, &[]);
        let (dir, _) = r.shoot(&enter(INVISIBLE, (90.0, 3.0, pitch)), &with_grass());
        eprintln!("{}", dir.display());
    }
    // A sparse grass of white opaque tufts (an area override: 0.3 a
    // square metre, its texture white), each tuft's shape plain, beside
    // Moonglow's from the reference camera.
    let overrides = [("mgwhite.tga", white_tga())];
    let mut r = Reading::new("grass_tufts_white", "ttr01", light, &overrides);
    let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else { panic!() };
    let mut ovr = mg_gff::Struct::new(101);
    let white = grass_override(3, "mgwhite", 0.3, 1.2, [1.0, 1.0, 1.0]);
    ovr.set("GrassOvrList", Value::List(vec![white]));
    props.set("GrassOverrides", Value::Struct(ovr));
    let (dir, _) = r.shoot_beside(INVISIBLE, &with_grass(), &|_| {});
    eprintln!("{}", dir.display());
}

/// Whether the client's grass sways: [`reading_grass_tufts`]'s white
/// tufts in a strong wind (`WindPower` 2), three pictures a second and a
/// half apart, how much of the tufts' part of the picture changed between
/// each two printed (the ground and the sky stand still).
#[test]
#[ignore]
fn reading_grass_sway() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let overrides = [("mgwhite.tga", white_tga())];
    let mut r = Reading::new("grass_sway", "ttr01", light, &overrides);
    r.are.root.set("WindPower", Value::Int(2));
    let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else { panic!() };
    let mut ovr = mg_gff::Struct::new(101);
    let white = grass_override(3, "mgwhite", 0.3, 1.2, [1.0, 1.0, 1.0]);
    ovr.set("GrassOvrList", Value::List(vec![white]));
    props.set("GrassOverrides", Value::Struct(ovr));
    r.save(&enter(INVISIBLE, (90.0, 12.0, 45.0)), &with_grass());
    let shots = common::client_screenshots(&r.dir, "MgScene", 3, Duration::from_millis(1500))
        .expect("the client's pictures");
    for (i, pair) in shots.windows(2).enumerate() {
        let (a, b) = (&pair[0].1, &pair[1].1);
        let (w, h) = (a.width as usize, a.height as usize);
        let mut changed = 0usize;
        let mut n = 0usize;
        // (The tufts' rows: below the horizon, above the GUI.)
        for y in h / 4..h * 3 / 4 {
            for x in w / 10..w * 8 / 10 {
                let k = (y * w + x) * 4;
                let d: u32 = (0..3).map(|c| u32::from(a.data[k + c].abs_diff(b.data[k + c]))).sum();
                changed += usize::from(d > 30);
                n += 1;
            }
        }
        save_png(a, &r.dir.join(format!("sway_{i}.png")));
        eprintln!(
            "shots {i} and {}: {:.2}% of the pixels changed",
            i + 1,
            100.0 * changed as f64 / n as f64
        );
    }
    eprintln!("{}", r.dir.display());
}

/// A custom talk table kept in the module alone (`mg_rtlk.tlk` in the
/// .mod, named by `Mod_CustomTlk`; none in the user directory's `tlk`),
/// its row 0 shown by the client: floated over the player by StrRef
/// (`FloatingTextStrRefOnCreature`, the client's own lookup) and by the
/// its row 1 as the server looked it up, sent to the chat as text.
#[test]
#[ignore]
fn reading_custom_talk_table_in_the_module() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("custom_tlk", "tic01", light, &[]);
    let mut t = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
    t.entries.push(mg_tlk::TlkEntry::text("MG CUSTOM ROW ZERO"));
    t.entries.push(mg_tlk::TlkEntry::text("MG CUSTOM ROW ONE"));
    r.m.set(ResKey::parse("mg_rtlk", ResType::TLK).unwrap(), t.to_bytes().unwrap());
    let mut info = r.m.info().unwrap();
    info.root.set("Mod_CustomTlk", Value::String(b"mg_rtlk".to_vec()));
    r.m.set_info(&info).unwrap();
    let body = format!(
        "{INVISIBLE}
    DelayCommand(1.0, FloatingTextStrRefOnCreature(16777216, pc, FALSE));
    DelayCommand(2.0, SendMessageToPC(pc, \"MG server: \" + GetStringByStrRef(16777217)));
    Log(\"MG_SERVER \" + GetStringByStrRef(16777216));"
    );
    let (dir, _) = r.shoot(&enter(&body, (90.0, 8.0, 60.0)), SETTINGS);
    print_log(&dir);
}

/// A cloak of a custom base item lying in an area: baseitems.2da with a
/// row after the game's, the cloak's (80) made to name a model of its own
/// (`DefaultModel` `it_torch_000`, a torch, where the game's names the
/// bag), and another the cloak's row as it is, and three cloaks
/// (`nw_maarcl055`) lying north of the player, west to east the game's
/// and one of each new base item, as the client and Moonglow's area view
/// draw them.
#[test]
#[ignore]
fn reading_custom_cloak_base_item() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let table = base.resman.get_named("baseitems", ResType::TWODA).unwrap();
    let mut table = String::from_utf8_lossy(&table).into_owned();
    let row: Vec<String> = table
        .lines()
        .find(|l| l.split_whitespace().next() == Some("80"))
        .unwrap()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let header: Vec<&str> = table.lines().nth(2).unwrap().split_whitespace().collect();
    let column = header.iter().position(|c| *c == "DefaultModel").unwrap() + 1;
    let next = table.lines().count() - 3;
    if !table.ends_with('\n') {
        table.push('\n');
    }
    // (And a second, the cloak's row as it is, naming the bag.)
    for (i, model) in ["it_torch_000", "it_bag"].into_iter().enumerate() {
        let mut new = row.clone();
        new[0] = (next + i).to_string();
        new[column] = model.into();
        table += &(new.join(" ") + "\n");
    }
    let overrides = [("baseitems.2da", table.into_bytes())];
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("custom_cloak", "tic01", light, &overrides);
    let bp = Gff::read(&r.game.resman.get_named("nw_maarcl055", ResType::UTI).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &r.game, item: &none };
    let mut placed = Vec::new();
    for (x, base_item) in [(18.5f32, 80i32), (20.0, next as i32), (21.5, next as i32 + 1)] {
        let at = Placement { position: [x, 23.0, 0.0], rotation: 0.0 };
        let mut item = instance(&placing, ResType::UTI, &bp.root, at, &[]).unwrap();
        item.set("BaseItem", Value::Int(base_item));
        placed.push(item);
    }
    r.git.root.set(mg_area::ObjectKind::Item.list(), Value::List(placed));
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("base item {next}: {}", dir.display());
}

/// A placeable whose model lacks the animation of its state, as CEP's
/// corpses on a creature's supermodel (`plc_orc2`) do: the goblin's model
/// (`c_goblina`, supermodel `a_ba`, neither with a `default`) renamed
/// `mg_corpse` and made a placeable appearance, standing north of the
/// player in its default state, as the client and Moonglow's area view
/// (its rest pose) draw it.
#[test]
#[ignore]
fn reading_placeable_without_its_state_s_animation() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let goblin = Model::read(&base.resman.get_named("c_goblina", ResType::MDL).unwrap()).unwrap();
    let text = mg_mdl::ascii_write::to_ascii(&goblin, &Default::default())
        .replace("c_goblinA", "mg_corpse")
        .replace("c_goblina", "mg_corpse");
    let (table, first) = placeables_with(&base, &[("mg_corpse", 0)]);
    let overrides = [("mg_corpse.mdl", text.into_bytes()), ("placeables.2da", table.into_bytes())];
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("placeable_no_state_animation", "tic01", light, &overrides);
    r.place(first, [20.0, 23.0, 0.0], 270.0, &[]);
    // And goblins (`nw_goblina`): west as it is, east with its idle
    // renamed (`AnimationReplace`, as `ReplaceObjectAnimation` keeps it)
    // to one its model lacks; a third made by the entry script and
    // renamed so by script, beside the east one (the client's alone).
    let bp = Gff::read(&r.game.resman.get_named("nw_goblina", ResType::UTC).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &r.game, item: &none };
    let mut placed = Vec::new();
    for (x, replaced) in [(17.5f32, false), (22.5, true)] {
        let at = Placement { position: [x, 23.0, 0.0], rotation: -std::f32::consts::FRAC_PI_2 };
        let mut c = instance(&placing, ResType::UTC, &bp.root, at, &[]).unwrap();
        let scripts: Vec<String> = c
            .fields
            .iter()
            .map(|f| f.label.to_string_lossy())
            .filter(|l| l.starts_with("Script"))
            .collect();
        for label in scripts {
            c.set(&label, Value::resref(ResRef::EMPTY));
        }
        if replaced {
            let mut pair = mg_gff::Struct::new(0);
            pair.set("OldAnimation", Value::String(b"pause1".to_vec()));
            pair.set("NewAnimation", Value::String(b"mg_none".to_vec()));
            let mut a = mg_gff::Struct::new(11);
            a.set("AnimationReplace", Value::List(vec![pair]));
            c.set("AnimationReplace", Value::Struct(a));
        }
        placed.push(c);
    }
    r.git.root.set("Creature List", Value::List(placed));
    let body = format!(
        "{INVISIBLE}
    object g = CreateObject(OBJECT_TYPE_CREATURE, \"nw_goblina\", Location(area, Vector(24.5, 23.0, 0.0), 270.0));
    ReplaceObjectAnimation(g, \"pause1\", \"mg_none\");"
    );
    let (dir, _) = r.shoot_beside(&body, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// The sun's direction in an area's GIT (`AreaProperties`
/// `SunDirectionX/Y/Z`, which the server reads: `engine_git_runtime.rs`)
/// and the client's lighting: a city's street (tcn01) at noon with the
/// client's environment shadows on, without the field (`git_sun_none`)
/// and with a sun low in the west (`git_sun_west`: (−1, 0, 0.25)), the
/// moon's the same.
#[test]
#[ignore]
fn reading_git_sun_direction() {
    let light = Lighting { ambient: 0x303030, diffuse: 0xE0E0E0, main_light: 0, only_tile: None };
    let settings = SETTINGS.replace(
        "[graphics.shadows.environment]\n\t\t\tenabled = false",
        "[graphics.shadows.environment]\n\t\t\tenabled = true",
    );
    assert!(settings.contains("enabled = true"));
    for (name, sun) in
        [("none", None), ("west", Some([-1.0f32, 0.0, 0.25])), ("north", Some([0.0, 1.0, 0.5]))]
    {
        let mut r = Reading::new(&format!("git_sun_{name}"), "tcn01", light, &[]);
        r.are.root.set("SunShadows", Value::Byte(1));
        r.are.root.set("ShadowOpacity", Value::Byte(60));
        r.place(0, [20.0, 24.0, 0.0], 270.0, &[]);
        if let Some([x, y, z]) = sun {
            let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else {
                panic!("no AreaProperties")
            };
            for (axis, v) in [("X", x), ("Y", y), ("Z", z)] {
                props.set(&format!("SunDirection{axis}"), Value::Float(v));
                props.set(&format!("MoonDirection{axis}"), Value::Float(v));
            }
        }
        let (dir, _) = r.shoot_beside(INVISIBLE, &settings, &|v| v.shadows = true);
        eprintln!("{}", dir.display());
    }
}

/// The shine of a dropped cloak's colors: the game's cloak
/// (`nw_maarcl055`, its environment map cloakmodel.2da's `default`)
/// lying north of the player in colors 0, 56 to 59 and 61 (every channel),
/// west to east, as the client and Moonglow's area view draw them (Aurora's
/// from above: `area_dropped_cloaks`).
#[test]
#[ignore]
fn reading_dropped_cloak_shine() {
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("dropped_cloak_shine", "tic01", light, &[]);
    let bp = Gff::read(&r.game.resman.get_named("nw_maarcl055", ResType::UTI).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &r.game, item: &none };
    let mut placed = Vec::new();
    for (i, color) in [0u8, 56, 57, 58, 59, 61].into_iter().enumerate() {
        let at = Placement { position: [17.5 + i as f32, 22.5, 0.0], rotation: 0.0 };
        let mut item = instance(&placing, ResType::UTI, &bp.root, at, &[]).unwrap();
        for channel in mg_rules::items::ArmorChannel::ALL {
            item.set(channel.field(), Value::Byte(color));
        }
        placed.push(item);
    }
    r.git.root.set(mg_area::ObjectKind::Item.list(), Value::List(placed));
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// The direction the client lights an area from, read back
/// (`lightAreaDiffuseDirection`, view space, through the debug
/// `inc_standard`, turned back to the world by the fitted camera) for an
/// area's GIT without a sun direction and with two, beside Moonglow's.
#[test]
#[ignore]
fn reading_git_sun_uniform() {
    let root = mg_testkit::nwn_root().expect("the game");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let overrides = [("inc_standard.shd", debug_shader(&base).into_bytes())];
    let light = Lighting { ambient: 0x303030, diffuse: 0xE0E0E0, main_light: 0, only_tile: None };
    for (name, sun) in
        [("none", None), ("west", Some([-1.0f32, 0.0, 0.25])), ("north", Some([0.0, 1.0, 1.0]))]
    {
        let mut r = Reading::new(&format!("git_sun_uniform_{name}"), "tcn01", light, &overrides);
        if let Some([x, y, z]) = sun {
            let Some(Value::Struct(props)) = r.git.root.get_mut("AreaProperties") else {
                panic!("no AreaProperties")
            };
            for (axis, v) in [("X", x), ("Y", y), ("Z", z)] {
                props.set(&format!("SunDirection{axis}"), Value::Float(v));
                props.set(&format!("MoonDirection{axis}"), Value::Float(v));
            }
        }
        let model = mg_area::AreaModel::read(&r.game, &r.are.root, &r.git.root, None);
        let ours = model.lighting.direction(false);
        let (dir, client) = r.shoot(&enter(&logged(INVISIBLE), (90.0, 12.0, 45.0)), SETTINGS);
        let v = |k| debug_value(&client, k, 1.0) * 2.0 - 1.0;
        let seen = Vec3::new(v(96), v(97), v(98));
        let pc = player_stood(&dir, &base, &model, glam::Vec2::new(20.0, 20.0));
        let camera = fitted_camera(pc, FIT_FOV, FIT_FOCUS, FIT_DISTANCE, FIT_PITCH);
        let world = camera.view().inverse().transform_vector3(seen);
        eprintln!(
            "{name}: GIT {sun:?}: the client's (view) {seen:.3}, in the world {world:.3}; \
             Moonglow's {:.3}",
            ours.unwrap_or_else(|| Vec3::new(4000.0, 4500.0, 7000.0).normalize())
        );
    }
}

/// Forest - Facelift's cliffs (ttf02's tiles 0 and 1, compiled meshes
/// whose index lists have more triangles than their face lists, which
/// Moonglow draws: `Mesh::drawn`) north of the player, as the client and
/// Moonglow's area view draw them: holes where only the faces were drawn.
#[test]
#[ignore]
fn reading_facelift_cliffs() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("facelift_cliffs", "ttf02", light, &[]);
    let tiles = r.are.root.list_mut("Tile_List").unwrap();
    for (i, id) in [(9usize, 0i32), (10, 1), (13, 2), (14, 3)] {
        tiles[i].set("Tile_ID", Value::Int(id));
    }
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// A rural shore at a slant (ttr01's tile 7, water to its north, with the
/// all-water tile 20 beyond), its plants' see-through edges over the
/// ground and the water, as the client and Moonglow's area view draw it
/// (a builder saw blue-white patches round plants and along shores).
#[test]
#[ignore]
fn reading_shore() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("shore", "ttr01", light, &[]);
    let tiles = r.are.root.list_mut("Tile_List").unwrap();
    for (i, id) in [(9usize, 7i32), (10, 7), (13, 20), (14, 20)] {
        tiles[i].set("Tile_ID", Value::Int(id));
    }
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}

/// An exact tile beside neighbors it disagrees with: ttr01's tile 20 (all
/// four corners Water) put where the area is grass, north-west of the
/// player, as the client and Moonglow's area view draw the seams.
#[test]
#[ignore]
fn reading_exact_tile_seams() {
    let light = Lighting { ambient: 0x606060, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("exact_tile_seams", "ttr01", light, &[]);
    let tiles = r.are.root.list_mut("Tile_List").unwrap();
    tiles[9].set("Tile_ID", Value::Int(20));
    let (dir, _) = r.shoot_beside(INVISIBLE, SETTINGS, &|_| {});
    eprintln!("{}", dir.display());
}
