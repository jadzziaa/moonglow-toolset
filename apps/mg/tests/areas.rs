//! `mg areas`: a module's areas listed and narrowed by its filters, and
//! fields, flags and variables set on those chosen; a dry run and a change
//! that can't be made change nothing.

use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::new(ResRef::from_str(name).unwrap(), t)
}

/// Two caves (underground) and an inn.
fn module(path: &std::path::Path) {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for (name, title, tileset, flags) in [
        ("cave1", "Wolf Cave", "tdc01", 0x7u32),
        ("cave2", "Bear Cave", "tdc01", 0x3 | 0x100),
        ("inn", "Apple Inn", "tin01", 0x1),
    ] {
        let mut are = Gff::new(*b"ARE ");
        let title = LocString::from_text(Language::ENGLISH, Gender::Male, title);
        are.root.set("Name", Value::LocString(title));
        are.root.set("Tileset", Value::resref(ResRef::from_str(tileset).unwrap()));
        are.root.set("Flags", Value::Dword(flags));
        are.root.set("SunFogAmount", Value::Byte(2));
        m.set_gff(key(name, ResType::ARE), &are).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut props = Struct::new(100);
        props.set("MusicDay", Value::Int(3));
        git.root.set("AreaProperties", Value::Struct(props));
        m.set_gff(key(name, ResType::GIT), &git).unwrap();
    }
    m.save_as(&ModuleLocation::Archive(path.into())).unwrap();
}

#[test]
fn areas_are_listed_and_changed_together() {
    let dir = mg_testkit::scratch_dir("mg-areas");
    let path = dir.join("caves.mod");
    module(&path);
    let run = |args: &[&str]| {
        let out = crate::mg().arg("areas").arg(&path).args(args).output();
        let out = out.unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        (out.status.success(), text(&out.stdout), text(&out.stderr))
    };
    let are =
        |name: &str| Module::open(&path).unwrap().gff(&key(name, ResType::ARE)).unwrap().unwrap();
    let music = |name: &str| {
        let git = Module::open(&path).unwrap().gff(&key(name, ResType::GIT)).unwrap().unwrap();
        git.root.child("AreaProperties").unwrap().integer("MusicDay")
    };

    // Listed: all, then the underground ones.
    let (ok, all, _) = run(&[]);
    assert!(ok && all.lines().count() == 3, "{all}");
    let (ok, under, notes) = run(&["--underground"]);
    assert!(ok, "{notes}");
    let listed: Vec<&str> = under.lines().map(|l| l.split('\t').next().unwrap()).collect();
    assert_eq!(listed, ["cave1", "cave2"]);
    assert!(under.contains("Wolf Cave") && under.contains("underground"));

    // A dry run lists the changes and changes nothing.
    let bytes = std::fs::read(&path).unwrap();
    let change = ["--underground", "--set", "MusicDay=57", "--set", "SunFogAmount=9"];
    let (ok, would, notes) = run(&[&change[..], &["--var", "nMusic=57", "--dry-run"]].concat());
    assert!(ok, "{notes}");
    assert!(would.contains("cave1\tMusicDay\t3 -> 57") && would.contains("cave2\tvar nMusic"));
    assert!(notes.contains("would change 2 of 2"), "{notes}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    // Changed: the caves, each keeping its other flags; not the inn.
    let (ok, _, notes) =
        run(&[&change[..], &["--var", "nMusic=57", "--set", "Natural=no"]].concat());
    assert!(ok, "{notes}");
    assert_eq!((music("cave1"), music("cave2"), music("inn")), (Some(57), Some(57), Some(3)));
    assert_eq!(are("cave1").root.integer("Flags"), Some(0x3));
    assert_eq!(are("cave2").root.integer("Flags"), Some(0x3 | 0x100));
    assert_eq!(are("cave2").root.get("SunFogAmount"), Some(&Value::Byte(9)));
    assert_eq!(are("inn").root.get("SunFogAmount"), Some(&Value::Byte(2)));
    // (A variable is the area's GIT's, where the game reads it.)
    let git =
        |name: &str| Module::open(&path).unwrap().gff(&key(name, ResType::GIT)).unwrap().unwrap();
    assert_eq!(git("cave1").root.list("VarTable").unwrap()[0].integer("Value"), Some(57));

    // Every area only when asked; a value that doesn't fit changes nothing.
    let bytes = std::fs::read(&path).unwrap();
    let (ok, _, notes) = run(&["--set", "SunFogAmount=1"]);
    assert!(!ok && notes.contains("--all"), "{notes}");
    let (ok, _, notes) = run(&["--all", "--set", "SunFogAmount=1", "--set", "SunFogAmount=999"]);
    assert!(!ok && notes.contains("SunFogAmount"), "{notes}");
    let (ok, _, notes) = run(&["nowhere", "--set", "SunFogAmount=1"]);
    assert!(!ok && notes.contains("nowhere"), "{notes}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    // JSON: the areas chosen and what changed.
    let out = crate::mg()
        .args(["--json", "areas"])
        .arg(&path)
        .args(["inn", "--remove-var", "nMusic", "--set", "Underground=yes"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["areas"][0]["resref"], "inn");
    assert_eq!(json["areas"][0]["name"], "Apple Inn");
    assert_eq!(json["changes"].as_array().unwrap().len(), 1, "the inn has no nMusic to delete");
    assert_eq!(json["changes"][0]["field"], "Underground");
    assert_eq!(are("inn").root.integer("Flags"), Some(0x3));
}

/// `--static-placeables`: the placeables that lose nothing by it are made
/// static; a dry run changes nothing.
#[test]
fn placeables_are_made_static_in_the_areas_chosen() {
    let dir = mg_testkit::scratch_dir("mg-areas-static");
    let path = dir.join("caves.mod");
    module(&path);
    let git_key = key("inn", ResType::GIT);
    let mut m = Module::open(&path).unwrap();
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let placeable = |tag: &str, useable: u8| {
        let mut p = Struct::new(9);
        p.set("Tag", Value::String(tag.as_bytes().to_vec()));
        p.set("Static", Value::Byte(0));
        p.set("Useable", Value::Byte(useable));
        p
    };
    git.root.set("Placeable List", Value::List(vec![placeable("ROCK", 0), placeable("CHEST", 1)]));
    m.set_gff(git_key, &git).unwrap();
    m.save().unwrap();
    let run = |args: &[&str]| {
        let out = crate::mg().arg("areas").arg(&path).args(args).output();
        let out = out.unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        (out.status.success(), text(&out.stdout), text(&out.stderr))
    };
    let fixed = || -> Vec<i64> {
        let git = Module::open(&path).unwrap().gff(&git_key).unwrap().unwrap();
        let list = git.root.list("Placeable List").unwrap();
        list.iter().map(|p| p.integer("Static").unwrap()).collect()
    };

    let (ok, out, err) = run(&["--all", "--static-placeables", "--dry-run"]);
    assert!(ok, "{err}");
    assert!(out.contains("inn\tplaceable 0 ROCK Static\t0 -> 1"), "{out}");
    assert!(!out.contains("CHEST"), "{out}");
    assert_eq!(fixed(), [0, 0], "a dry run changes nothing");

    let (ok, _, err) = run(&["--all", "--static-placeables"]);
    assert!(ok, "{err}");
    assert_eq!(fixed(), [1, 0], "the Useable chest stays dynamic");
}

/// `--static-placeables` in a module folder with a project file: a
/// placeable with variables stays dynamic, and the project's rules keep
/// others; each is named with why. What the file doesn't understand is
/// said too.
#[test]
fn make_static_follows_the_project_file() {
    let dir = mg_testkit::scratch_dir("mg-areas-static-project");
    let archive = dir.join("caves.mod");
    module(&archive);
    let folder = dir.join("caves");
    let mut m = Module::open(&archive).unwrap();
    let git_key = key("inn", ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let placeable = |tag: &str, var: Option<&str>| {
        let mut p = Struct::new(9);
        p.set("Tag", Value::String(tag.as_bytes().to_vec()));
        p.set("Static", Value::Byte(0));
        p.set("Useable", Value::Byte(0));
        if let Some(name) = var {
            let mut v = Struct::new(0);
            v.set("Name", Value::String(name.as_bytes().to_vec()));
            v.set("Type", Value::Dword(1));
            v.set("Value", Value::Int(1));
            p.set("VarTable", Value::List(vec![v]));
        }
        p
    };
    git.root.set(
        "Placeable List",
        Value::List(vec![
            placeable("ROCK", None),
            placeable("STONE", Some("tdod_lore")),
            placeable("CRATE", Some("x_note")),
            placeable("LAMP_1", None),
        ]),
    );
    m.set_gff(git_key, &git).unwrap();
    m.save().unwrap();
    m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    std::fs::write(
        folder.join("moonglow.toml"),
        "[make_static]\nkeep_variables = [\"tdod_*\"]\nkeep_tags = [\"LAMP_*\"]\nshout = 1\n",
    )
    .unwrap();
    let out = crate::mg()
        .args(["--json", "areas"])
        .arg(&folder)
        .args(["--all", "--static-placeables"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let left: Vec<(String, String)> = json["left_dynamic"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (l["tag"].as_str().unwrap().into(), l["why"].as_str().unwrap().into()))
        .collect();
    assert_eq!(
        left,
        [
            ("STONE".into(), "it has the variable tdod_lore".into()),
            ("LAMP_1".into(), "the project keeps tags LAMP_* dynamic".into()),
        ]
    );
    let notes: Vec<&str> =
        json["notes"].as_array().unwrap().iter().filter_map(|n| n.as_str()).collect();
    let said = "moonglow.toml:4: \"shout\" is not a key of [make_static]";
    assert!(notes.contains(&said), "{notes:?}");
    let git = Module::open(&folder).unwrap().gff(&git_key).unwrap().unwrap();
    let fixed: Vec<i64> = git
        .root
        .list("Placeable List")
        .unwrap()
        .iter()
        .map(|p| p.integer("Static").unwrap())
        .collect();
    assert_eq!(fixed, [1, 0, 1, 0], "x_note isn't the project's");
    // Without the file, any variable keeps a placeable dynamic.
    let out = crate::mg()
        .arg("areas")
        .arg(&archive)
        .args(["--all", "--static-placeables"])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("inn: placeable 2 CRATE left dynamic: it has the variable x_note"),
        "{err}"
    );
}

/// `--where FIELD=VALUE`, `--where-var NAME=VALUE` and `--music ROW`
/// choose the areas by what they hold (their ARE's fields, their GIT's
/// music and variables), with the other filters, to list or to change.
#[test]
fn areas_are_chosen_by_what_they_hold() {
    let dir = mg_testkit::scratch_dir("mg-areas-where");
    let path = dir.join("caves.mod");
    module(&path);
    let run = |args: &[&str]| {
        let out = crate::mg().arg("areas").arg(&path).args(args).output().unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        let listed: Vec<String> = (text(&out.stdout).lines())
            .map(|l| l.split('\t').next().unwrap().to_string())
            .collect();
        (out.status.success(), listed, text(&out.stderr))
    };
    let (ok, _, notes) = run(&["cave2", "--set", "MusicDay=57", "--var", "kz_bears=4"]);
    assert!(ok, "{notes}");
    assert_eq!(run(&["--music", "57"]).1, ["cave2"]);
    assert_eq!(run(&["--music", "3"]).1, ["cave1", "inn"]);
    assert_eq!(run(&["--where", "musicday=57"]).1, ["cave2"], "a label in any case");
    assert_eq!(run(&["--where", "SunFog*=2", "--underground"]).1, ["cave1", "cave2"]);
    assert_eq!(run(&["--where-var", "kz_*"]).1, ["cave2"]);
    assert!(run(&["--where-var", "kz_bears=5"]).1.is_empty());
    // A filter of its own: what is chosen changes.
    let (ok, changed, notes) = run(&["--where-var", "kz_bears=4", "--set", "SunFogAmount=7"]);
    assert!(ok, "{notes}");
    assert_eq!(changed, ["cave2"]);
    let (ok, _, notes) = run(&["--where-var", "=4"]);
    assert!(!ok && notes.contains("name a variable"), "{notes}");
}

/// `--grass`, `--remove-grass` and `--tileset-grass`: the areas' own grass
/// set in their GITs' AreaProperties, where the game reads it.
#[test]
fn areas_grow_grass_of_their_own() {
    use mg_module::grass::AreaGrass;
    let dir = mg_testkit::scratch_dir("mg-areas-grass");
    let path = dir.join("caves.mod");
    module(&path);
    let run = |args: &[&str]| {
        let out = crate::mg().arg("areas").arg(&path).args(args).output().unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        (out.status.success(), text(&out.stdout), text(&out.stderr))
    };
    let grass = |name: &str| {
        let git = Module::open(&path).unwrap().gff(&key(name, ResType::GIT)).unwrap().unwrap();
        AreaGrass::read(&git.root)
    };
    let (ok, out, notes) = run(&[
        "--underground",
        "--grass",
        "1:mgdirt:3:0.5:1,0,0",
        "--grass",
        "5:grass:2:1",
        "--tileset-grass",
        "no",
    ]);
    assert!(ok, "{notes}");
    assert!(out.contains("cave1\tgrass on material 1\t(none) -> mgdirt, density 3"), "{out}");
    assert!(out.contains("cave2\ttileset grass\tyes -> no"), "{out}");
    let cave = grass("cave1");
    assert!(cave.default_disabled);
    let set: Vec<(u32, &str, f32)> = cave
        .overrides
        .iter()
        .map(|o| (o.material, o.look.texture.as_str(), o.look.height))
        .collect();
    assert_eq!(set, [(1, "mgdirt", 0.5), (5, "grass", 1.0)]);
    assert_eq!(cave.overrides[0].look.diffuse, [1.0, 0.0, 0.0]);
    assert_eq!(cave.overrides[1].look.ambient, mg_module::grass::GAME_GRASS_COLOR);
    assert_eq!(grass("inn"), AreaGrass::default());
    // Changed in place, then taken off; set again, nothing changes.
    let (ok, out, notes) =
        run(&["cave1", "--grass", "1:mgdirt:6:0.5:1,0,0", "--remove-grass", "5"]);
    assert!(ok, "{notes}");
    assert!(out.contains("density 3") && out.contains("-> mgdirt, density 6"), "{out}");
    let cave = grass("cave1");
    assert_eq!(cave.overrides.len(), 1);
    assert_eq!(cave.overrides[0].look.density, 6.0);
    let (ok, out, notes) = run(&["cave1", "--grass", "1:mgdirt:6:0.5:1,0,0"]);
    assert!(ok && out.is_empty() && notes.contains("changed 0 of 1"), "{out}{notes}");
    // What can't be read changes nothing.
    let bytes = std::fs::read(&path).unwrap();
    for bad in [&["--all", "--grass", "1:mgdirt:3"][..], &["--all", "--tileset-grass", "maybe"]] {
        let (ok, _, notes) = run(bad);
        assert!(!ok, "{bad:?}: {notes}");
    }
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

/// `--lighting-scheme`: an environment.2da row of the game's, by label or
/// name, gives the areas its lighting and each tile its lights, as Area
/// Properties' Lighting Scheme does; `--set` is made after it. A scheme
/// the table lacks changes nothing.
#[test]
fn a_lighting_scheme_is_given_to_the_areas() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-areas-scheme");
    let path = dir.join("caves.mod");
    module(&path);
    let are_key = key("cave1", ResType::ARE);
    let mut m = Module::open(&path).unwrap();
    let mut are = m.gff(&are_key).unwrap().unwrap();
    let tile = || {
        let mut t = Struct::new(1);
        t.set("Tile_ID", Value::Int(0));
        for l in ["Tile_MainLight1", "Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2"] {
            t.set(l, Value::Byte(0));
        }
        t
    };
    are.root.set("Tile_List", Value::List(vec![tile(), tile(), tile(), tile()]));
    m.set_gff(are_key, &are).unwrap();
    m.save().unwrap();
    let run = |args: &[&str]| {
        let mut cmd = crate::mg_on(&root, None);
        cmd.arg("areas");
        let out = cmd.arg(&path).args(args).output().unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        (out.status.success(), text(&out.stdout), text(&out.stderr))
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let env = game.table("environment").unwrap();
    let row = (0..env.len()).find(|&r| env.get(r, "LABEL") == Some("InteriorDark")).unwrap();
    let cell = |c: &str| i64::from(env.get_int(row, c).unwrap());

    // (A seed, so that the tiles' lights picked are not by chance their own.)
    let scheme =
        ["cave1", "--lighting-scheme", "interiordark", "--seed", "1", "--set", "SunFogAmount=5"];
    let (ok, out, err) = run(&scheme);
    assert!(ok, "{err}");
    assert!(out.contains(&format!("cave1\tLightingScheme\t(none) -> {row}")), "{out}");
    assert!(out.contains("cave1\tlights of 4 tiles\ttheir own -> "), "{out}");
    let are = Module::open(&path).unwrap().gff(&are_key).unwrap().unwrap().root;
    assert_eq!(are.integer("LightingScheme"), Some(row as i64));
    assert_eq!(are.integer("IsNight"), Some(1), "an interior's scheme is night");
    assert_eq!(are.integer("MoonFogAmount"), Some(cell("DARK_FOG")));
    assert_eq!(are.integer("SunFogAmount"), Some(5), "--set after the scheme");
    let colors = |prefix: &str| (1..=4).map(|n| cell(&format!("{prefix}{n}"))).collect::<Vec<_>>();
    for t in are.list("Tile_List").unwrap() {
        let light = |l: &str| t.integer(l).unwrap();
        assert!(colors("MAIN1_COLOR").contains(&light("Tile_MainLight1")), "{t:?}");
        assert!(colors("MAIN2_COLOR").contains(&light("Tile_MainLight2")), "{t:?}");
        assert!(colors("SECONDARY_COLOR").contains(&light("Tile_SrcLight1")), "{t:?}");
        assert_eq!(light("Tile_SrcLight2"), light("Tile_SrcLight1"));
    }
    let inn = Module::open(&path).unwrap().gff(&key("inn", ResType::ARE)).unwrap().unwrap();
    assert!(inn.root.get("LightingScheme").is_none(), "only the area named");

    // By the name players read, and to all; one the table lacks: nothing.
    let bytes = std::fs::read(&path).unwrap();
    let (ok, out, err) = run(&["--all", "--lighting-scheme", "Exterior Overcast", "--dry-run"]);
    assert!(ok, "{err}");
    assert!(out.contains("inn\tLightingScheme\t(none) -> 1"), "{out}");
    for bad in ["NoSuchScheme", "999"] {
        let (ok, _, err) = run(&["--all", "--lighting-scheme", bad]);
        assert!(!ok && err.contains("environment"), "{bad}: {err}");
    }
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "a dry run and a refusal change nothing");

    // The tiles' lights are picked with a number a run prints, a dry run
    // too: with it, a run makes the dry run's picks, and the same picks
    // on another copy of the module.
    let (a, b) = (dir.join("a.mod"), dir.join("b.mod"));
    std::fs::copy(&path, &a).unwrap();
    std::fs::copy(&path, &b).unwrap();
    let run_on = |module: &std::path::Path, args: &[&str]| {
        let mut cmd = crate::mg_on(&root, None);
        cmd.arg("areas");
        let out = cmd.arg(module).args(args).output().unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        assert!(out.status.success(), "{}", text(&out.stderr));
        (text(&out.stdout), text(&out.stderr))
    };
    let overcast = ["--all", "--lighting-scheme", "Exterior Overcast"];
    let (listed, said) = run_on(&a, &[&overcast[..], &["--dry-run"]].concat());
    let seed = said.split("--seed ").nth(1).and_then(|s| s.split_whitespace().next());
    let seed = seed.unwrap_or_else(|| panic!("no seed said: {said}"));
    let with = [&overcast[..], &["--seed", seed]].concat();
    assert_eq!(run_on(&a, &with).0, listed, "the dry run's picks");
    run_on(&b, &with);
    let tiles = |p: &std::path::Path| {
        let are = Module::open(p).unwrap().gff(&are_key).unwrap().unwrap();
        are.root.list("Tile_List").unwrap().to_vec()
    };
    assert_eq!(tiles(&a), tiles(&b), "the same number, the same picks");
}

/// A `.mod` named to `mg` with its module folder beside it (Aurora's
/// module directory, `Caves/` beside `caves.mod`, in any case): the
/// command reads and changes the `.mod`, as the game's server reads it,
/// and says that the folder is left as it is, in the JSON's notes too.
#[test]
fn a_mod_s_folder_beside_it_is_said() {
    let dir = mg_testkit::scratch_dir("mg-areas-folder-beside");
    let path = dir.join("caves.mod");
    module(&path);
    let run = |args: &[&str]| {
        let out = crate::mg().args(args).output().unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        (out.status.success(), text(&out.stdout), text(&out.stderr))
    };
    let p = path.to_str().unwrap();
    let (ok, _, notes) = run(&["areas", p]);
    assert!(ok && !notes.contains("module folder"), "{notes}");
    let folder = dir.join("Caves");
    Module::open(&path).unwrap().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let (ok, _, notes) = run(&["areas", p, "--all", "--set", "SunFogAmount=5"]);
    assert!(ok, "{notes}");
    assert_eq!(notes.matches("has its module folder beside it").count(), 1, "{notes}");
    assert!(notes.contains(&folder.display().to_string()), "{notes}");
    let fog = |m: &std::path::Path| {
        let are = Module::open(m).unwrap().gff(&key("cave1", ResType::ARE)).unwrap().unwrap();
        are.root.integer("SunFogAmount")
    };
    assert_eq!((fog(&path), fog(&folder)), (Some(5), Some(2)), "the .mod changed, not the folder");
    let (ok, json, _) = run(&["--json", "areas", p]);
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(ok && json["notes"].to_string().contains("has its module folder beside it"), "{json}");
}
