//! Visual transforms (Adjust Location's Scale, Rotation and Translation)
//! in the engine: what it reads from a module's GIT, read back by script
//! (`GetObjectVisualTransform`), and how its own save writes one a script
//! set. Placed objects are given theirs by Moonglow's own edits
//! (`mg_area::edit::visual_transform_edits`, what Adjust Location
//! applies), so the test holds what Moonglow writes to what the engine
//! reads: a builder's mirror (scaled 0.46, tilted -50.8° about X) as a
//! dynamic placeable, Useable or not, and as a static one; a creature, a
//! door and an item; and the older `VisualTransform` struct. The server
//! reads every one of them, the static placeable's too (what the client
//! draws is another matter: a static placeable without it, settled by
//! `visual_transforms_look` in `client_render.rs`).

use std::time::Duration;

use glam::Vec3;
use mg_area::{AreaObject, ObjectKind, VisualTransform};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Workspace};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, save_game, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// The builder's mirror: Scale 0.46, X Rotation -50.8.
const MIRROR: VisualTransform = VisualTransform {
    scale: Vec3::splat(0.46),
    rotate: Vec3::new(-50.8, 0.0, 0.0),
    translate: Vec3::ZERO,
};

/// Logs, for each tag, whether it was found and its scale, rotations
/// about X, Y and Z and Z translation as the engine holds them; then sets
/// one on `MG_SCRIPTED` by script, for the save to show how the engine
/// writes one.
const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

string F(float f) { return FloatToString(f, 0, 2); }

void Seen(string sTag)
{
    object o = GetObjectByTag(sTag);
    Log("MG_VT " + sTag + "|" + IntToString(GetIsObjectValid(o)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_SCALE)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_ROTATE_X)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_ROTATE_Y)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_ROTATE_Z)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_TRANSLATE_Z)));
}

void main()
{
    Seen("MG_DYNAMIC");
    Seen("MG_USEABLE");
    Seen("MG_STATIC");
    Seen("MG_LEGACY");
    Seen("MG_PLAIN");
    Seen("MG_CREATURE");
    Seen("MG_DOOR");
    Seen("MG_ITEM");
    object o = GetObjectByTag("MG_SCRIPTED");
    SetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_SCALE, 0.46);
    SetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_ROTATE_X, -50.8);
    Log("MG_DONE");
    Log("MG_READY");
}
"#;

/// A module with the objects the probe asks about, given their visual
/// transforms as Adjust Location writes them, saved where the server
/// finds it.
fn module(game: &GameData, dir: &std::path::Path) -> std::path::PathBuf {
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(game, "Visual Transforms", &mut rng).unwrap();
    let spec = AreaSpec { name: "Transforms".into(), tileset: r("ttr01"), width: 4, height: 4 };
    let area = add_area(&mut m, game, &spec, &mut rng).unwrap();
    let get = |name: &str, t: ResType| -> Struct {
        Gff::read(&game.resman.get(&ResKey::new(r(name), t)).unwrap()).unwrap().root
    };
    let items = |_: ResRef| -> Option<Struct> { None };
    let placing = Placing { game, item: &items };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    // (tag, blueprint, Static, Useable)
    let placeables = [
        ("MG_DYNAMIC", 0, 0),
        ("MG_USEABLE", 0, 1),
        ("MG_STATIC", 1, 0),
        ("MG_LEGACY", 0, 0),
        ("MG_PLAIN", 0, 0),
        ("MG_SCRIPTED", 0, 0),
    ];
    let mut add = |t: ResType, s: Struct| {
        let (list, _) = git_list(t).unwrap();
        let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
        objects.push(s);
        git.root.set(list, Value::List(objects));
    };
    for (i, (tag, is_static, useable)) in placeables.into_iter().enumerate() {
        let mut s = get("plc_barrel", ResType::UTP);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        // placeables.2da 447: Mirror.
        s.set("Appearance", Value::Dword(447));
        s.set("Static", Value::Byte(is_static));
        s.set("Useable", Value::Byte(useable));
        let at = Placement { position: [4.0 + 4.0 * i as f32, 20.0, 0.0], rotation: 0.0 };
        add(ResType::UTP, instance(&placing, ResType::UTP, &s, at, &[]).unwrap());
    }
    for (name, t, tag, x) in [
        ("nw_badger", ResType::UTC, "MG_CREATURE", 8.0),
        ("nw_door_ttr_01", ResType::UTD, "MG_DOOR", 16.0),
        ("nw_wswss001", ResType::UTI, "MG_ITEM", 24.0),
    ] {
        let mut s = get(name, t);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
        add(t, instance(&placing, t, &s, at, &[]).unwrap());
    }
    m.set_gff(git_key, &git).unwrap();
    let probe = compile(dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);

    // Adjust Location's edits, as the window applies them.
    let mut ws = Workspace::new(m);
    let wanted = ["MG_DYNAMIC", "MG_USEABLE", "MG_CREATURE", "MG_DOOR", "MG_ITEM"];
    for kind in [ObjectKind::Placeable, ObjectKind::Creature, ObjectKind::Door, ObjectKind::Item] {
        let root = ws.doc(&git_key).unwrap().root.clone();
        for (index, s) in root.list(kind.list()).unwrap_or(&[]).iter().enumerate() {
            let o = AreaObject::placed(kind, index, s);
            if !wanted.contains(&o.tag.as_str()) {
                continue;
            }
            assert!(o.takes_visual_transform(), "{} takes one", o.tag);
            let edits = mg_area::edit::visual_transform_edits(git_key, &o, s, MIRROR);
            assert!(!edits.is_empty(), "{}: an edit", o.tag);
            ws.apply(Command::new("Adjust location", edits)).unwrap();
        }
    }
    // A static placeable's file may still have one (set before it was
    // made static elsewhere, or by hand); and the older struct.
    let root = ws.doc(&git_key).unwrap().root.clone();
    let mut edits = Vec::new();
    for (index, s) in root.list("Placeable List").unwrap().iter().enumerate() {
        let (label, value) = match s.string("Tag").unwrap_or_default() {
            b"MG_STATIC" => ("VisTransformList", mg_area::edit::visual_transform_list(MIRROR)),
            b"MG_LEGACY" => {
                let mut v = Struct::new(6);
                for a in ["X", "Y", "Z"] {
                    v.set(&format!("Scale{a}"), Value::Float(0.46));
                }
                v.set("RotateX", Value::Float(-50.8));
                ("VisualTransform", Value::Struct(v))
            }
            _ => continue,
        };
        edits.push(mg_edit::Edit::SetField {
            key: git_key,
            path: mg_edit::GffPath::root().item("Placeable List", index),
            label: label.into(),
            value: Some(value),
        });
    }
    ws.apply(Command::new("By hand", edits)).unwrap();
    let user = dir.join("user");
    ws.save_as(&ModuleLocation::Archive(user.join("modules/mg_vt.mod"))).unwrap();
    // And as a folder: the same GIT, field for field.
    let folder = dir.join("folder");
    ws.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let saved = mg_module::Module::open(&user.join("modules/mg_vt.mod")).unwrap();
    let in_folder = mg_module::Module::open(&folder).unwrap();
    assert_eq!(
        saved.gff(&git_key).unwrap().unwrap().root,
        in_folder.gff(&git_key).unwrap().unwrap().root
    );
    user
}

/// The GIT through a nasher project's JSON and back as nasher packs it
/// (neverwinter.nim's `nwn_gff`): the visual transforms are as Moonglow
/// wrote them.
fn through_nasher(dir: &std::path::Path, user: &std::path::Path) {
    let m = mg_module::Module::open(&user.join("modules/mg_vt.mod")).unwrap();
    let key = *m.keys_of(ResType::GIT).next().unwrap();
    let git = m.gff(&key).unwrap().unwrap();
    let conv = mg_module::nasher::Conversion::default();
    let json = conv.to_source(ResType::GIT, &git).unwrap();
    let source = dir.join(format!("{}.git.json", key.resref));
    std::fs::write(&source, &json).unwrap();
    let packed = dir.join(format!("{}.git", key.resref));
    let nwn_gff = mg_testkit::nwn_tool("nwn_gff").expect("nwn_gff");
    let out = std::process::Command::new(nwn_gff)
        .args(["-i", source.to_str().unwrap(), "-o", packed.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "nwn_gff: {}", String::from_utf8_lossy(&out.stderr));
    let back = Gff::read(&std::fs::read(&packed).unwrap()).unwrap();
    // (`nwn_gff` keeps fields in hash order, a known divergence: compared
    // with each struct's fields sorted.)
    fn sorted(v: &Value) -> Value {
        let in_order = |s: &Struct| {
            let mut s = s.clone();
            for f in &mut s.fields {
                f.value = sorted(&f.value);
            }
            s.fields.sort_by(|a, b| a.label.as_bytes().cmp(b.label.as_bytes()));
            s
        };
        match v {
            Value::Struct(s) => Value::Struct(in_order(s)),
            Value::List(items) => Value::List(items.iter().map(in_order).collect()),
            v => v.clone(),
        }
    }
    let transforms = |g: &Gff| -> Vec<(Option<Value>, Option<Value>)> {
        let field = |p: &Struct, label: &str| p.get(label).map(sorted);
        g.root
            .list("Placeable List")
            .unwrap()
            .iter()
            .map(|p| (field(p, "VisTransformList"), field(p, "VisualTransform")))
            .collect()
    };
    assert_eq!(transforms(&back), transforms(&git));
    // (Moonglow reads its own JSON back the same, too.)
    let ours = conv.from_source(&format!("{}.git.json", key.resref), json.as_bytes()).unwrap();
    assert_eq!(transforms(&ours), transforms(&git));
}

#[test]
fn the_engine_reads_visual_transforms_as_adjust_location_writes_them() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_visual_transforms");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let user = module(&game, &dir);
    let _ = oracle_tool!("nwn_gff");
    through_nasher(&dir, &user);
    let run = run_server(&root, &user, "mg_vt", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    // tag | found | scale | rotate X, Y, Z | translate Z
    let seen = run.values("MG_VT");
    let mirror = |tag: &str| format!("{tag}|1|0.46|-50.80|0.00|0.00|0.00");
    let plain = |tag: &str| format!("{tag}|1|1.00|0.00|0.00|0.00|0.00");
    assert_eq!(
        seen,
        [
            mirror("MG_DYNAMIC"),
            mirror("MG_USEABLE"),
            mirror("MG_STATIC"),
            mirror("MG_LEGACY"),
            plain("MG_PLAIN"),
            mirror("MG_CREATURE"),
            mirror("MG_DOOR"),
            mirror("MG_ITEM"),
        ],
        "{}",
        run.log
    );

    // The engine's own save of one a script set: the fields Moonglow
    // writes (scope 0 and each axis's `ValueTo`), read back the same.
    let sav = save_game(&root, &user, "mg_vt", "MG_READY", Duration::from_secs(120))
        .unwrap_or_else(|e| panic!("the server did not save the game: {e}"));
    let data = std::fs::read(&sav).unwrap();
    let erf = mg_erf::Erf::read(&data).unwrap();
    let area = erf
        .entries
        .iter()
        .find(|e| e.restype == ResType::GIT)
        .map(|e| e.resref)
        .expect("the save has the area's GIT");
    let entry = erf.find(&area, ResType::GIT).unwrap();
    let saved = Gff::read(&erf.data(entry).unwrap()).unwrap().root;
    let placeables = saved.list("Placeable List").unwrap();
    let tagged = |tag: &[u8]| placeables.iter().find(|p| p.string("Tag") == Some(tag));
    let scripted = tagged(b"MG_SCRIPTED").expect("the scripted placeable is saved");
    let list = scripted.list("VisTransformList").expect("saved as a VisTransformList");
    // The script's one scale is `ScaleX`: the engine writes Y and Z as 1
    // (and the client scales by X alone: `VisualTransform::matrix`).
    let read = VisualTransform::read(scripted).unwrap();
    assert_eq!(read.scale, Vec3::new(0.46, 1.0, 1.0), "{list:#?}");
    assert_eq!(read.rotate, MIRROR.rotate);
    assert_eq!(read.matrix(), MIRROR.matrix());
    let entry = list.iter().find(|e| e.integer("Scope") == Some(0)).expect("scope 0");
    for label in ["ScaleX", "ScaleY", "ScaleZ", "RotateX"] {
        let axis = entry.child(label).unwrap_or_else(|| panic!("{label}"));
        assert!(axis.float("ValueTo").is_some(), "{label}: ValueTo");
    }
    // What Adjust Location gave the dynamic mirror, the engine kept.
    let dynamic = tagged(b"MG_DYNAMIC").unwrap();
    assert_eq!(VisualTransform::read(dynamic), Some(MIRROR));
}
