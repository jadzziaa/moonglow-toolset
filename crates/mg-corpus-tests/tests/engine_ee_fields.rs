//! The EE data Aurora has no fields for, as the engine reads it from a
//! module (`docs/research/notes_ee_fields.md`):
//! - texture, animation and shader replacements and the other object
//!   visuals (`TextureReplace`, `AnimationReplace`, `Material`,
//!   `MiscVisuals`), on each kind of placed object and on a blueprint
//!   created by script;
//! - area flags past the first three, and a tile's replacement texture;
//! - a creature's familiar and animal companion (read only when one of its
//!   classes has one), cleric domains and the wizard's school;
//! - a placeable with an inventory that isn't usable;
//! - an item stack past its base item's limit, and 255 charges.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};
use serde_json::Value as Json;

mod common;
use common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.as_bytes().to_vec())
}

/// The visuals every placed object gets: a texture and an animation
/// replaced, an int and a vec4 shader parameter, and of `MiscVisuals` only
/// the visible distance (the rest left to the engine's defaults).
fn add_visuals(s: &mut Struct) {
    let pair = |old: &str, new: &str, labels: (&str, &str)| {
        let mut e = Struct::new(0);
        e.set(labels.0, Value::resref(r(old)));
        e.set(labels.1, Value::resref(r(new)));
        e
    };
    let mut t = Struct::new(9);
    let e = pair("mg_old", "mg_new", ("OldTexture", "NewTexture"));
    t.set("TextureReplaceLi", Value::List(vec![e]));
    s.set("TextureReplace", Value::Struct(t));
    let mut a = Struct::new(11);
    let e = pair("walk", "mg_walk", ("OldAnimation", "NewAnimation"));
    a.set("AnimationReplace", Value::List(vec![e]));
    s.set("AnimationReplace", Value::Struct(a));
    let mut int = Struct::new(0);
    int.set("Material", text("mg_mat"));
    int.set("Param", text("mg_int"));
    int.set("Type", Value::Byte(1));
    int.set("Int", Value::Int(7));
    let mut vec4 = Struct::new(0);
    vec4.set("Material", text("mg_mat"));
    vec4.set("Param", text("mg_vec"));
    vec4.set("Type", Value::Byte(2));
    for (i, v) in [1.0, 2.0, 3.0, 4.0].into_iter().enumerate() {
        vec4.set(&format!("Float{}", i + 1), Value::Float(v));
    }
    let mut m = Struct::new(7);
    m.set("ShaderParams", Value::List(vec![int, vec4]));
    s.set("Material", Value::Struct(m));
    let mut misc = Struct::new(8);
    misc.set("VisibleDistance", Value::Float(12.5));
    s.set("MiscVisuals", Value::Struct(misc));
}

/// A class: its number, level and extra fields.
type Class<'a> = (i32, i16, &'a [(&'a str, Value)]);

/// A creature's classes.
fn classes(s: &mut Struct, list: &[Class<'_>]) {
    let items = list
        .iter()
        .map(|(class, level, extra)| {
            let mut c = Struct::new(2);
            c.set("Class", Value::Int(*class));
            c.set("ClassLevel", Value::Short(*level));
            for (label, v) in *extra {
                c.set(label, v.clone());
            }
            c
        })
        .collect();
    s.set("ClassList", Value::List(items));
}

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

// An object's visuals as the engine holds them, in pieces short enough for
// the log.
void Visuals(string sTag, object o)
{
    json j = ObjectToJson(o);
    json v = JsonObject();
    v = JsonObjectSet(v, "TextureReplace", JsonObjectGet(j, "TextureReplace"));
    v = JsonObjectSet(v, "AnimationReplace", JsonObjectGet(j, "AnimationReplace"));
    v = JsonObjectSet(v, "Material", JsonObjectGet(j, "Material"));
    v = JsonObjectSet(v, "MiscVisuals", JsonObjectGet(j, "MiscVisuals"));
    string s = JsonDump(v);
    int i;
    for (i = 0; i < GetStringLength(s); i += 600)
        Log("MG_J_" + sTag + " " + GetSubString(s, i, 600));
    Log("MG_OBJ " + sTag + "|" + IntToString(GetIsObjectValid(o)) + "|"
        + FloatToString(GetObjectVisibleDistance(o), 0, 1) + "|"
        + IntToString(GetObjectUiDiscoveryMask(o)));
}

void Creature(string sTag)
{
    object o = GetObjectByTag(sTag);
    Log("MG_CLASSES " + sTag + "|" + IntToString(GetFamiliarCreatureType(o)) + "|" + GetFamiliarName(o)
        + "|" + IntToString(GetAnimalCompanionCreatureType(o)) + "|" + GetAnimalCompanionName(o)
        + "|" + IntToString(GetSpecialization(o, CLASS_TYPE_WIZARD))
        + "|" + IntToString(GetDomain(o, 1, CLASS_TYPE_CLERIC))
        + "|" + IntToString(GetDomain(o, 2, CLASS_TYPE_CLERIC)));
}

void main()
{
    string sTags = "MG_PLC MG_DOOR MG_CRE MG_ITEM MG_WP MG_TRIG MG_SOUND MG_STORE MG_ENC";
    int n = 0;
    string sTag = "";
    while (n < GetStringLength(sTags))
    {
        int nEnd = FindSubString(sTags, " ", n);
        if (nEnd < 0) nEnd = GetStringLength(sTags);
        sTag = GetSubString(sTags, n, nEnd - n);
        Visuals(sTag, GetObjectByTag(sTag));
        n = nEnd + 1;
    }
    // A blueprint with visuals, created by script.
    object oArea = GetArea(GetObjectByTag("MG_WP"));
    object oMade = CreateObject(OBJECT_TYPE_PLACEABLE, "mg_vis", Location(oArea, Vector(5.0, 5.0, 0.0), 0.0));
    Visuals("MG_MADE", oMade);

    json jArea = ObjectToJson(oArea);
    json jAre = JsonObjectGet(JsonObjectGet(jArea, "ARE"), "value");
    Log("MG_AREA " + IntToString(GetIsAreaInterior(oArea)) + "|" + JsonDump(JsonObjectGet(jAre, "Flags")));
    json jTile = JsonArrayGet(JsonObjectGet(JsonObjectGet(jAre, "Tile_List"), "value"), 0);
    Log("MG_TILE " + JsonDump(JsonObjectGet(jTile, "Tile_ReplaceTex")));

    Creature("MG_WIZ");
    Creature("MG_CLR");
    Creature("MG_DRU");
    Creature("MG_RGR3");
    Creature("MG_RGR6");

    object oChest = GetObjectByTag("MG_CHEST");
    int nItems = 0;
    object oItem = GetFirstItemInInventory(oChest);
    while (GetIsObjectValid(oItem)) { nItems++; oItem = GetNextItemInInventory(oChest); }
    Log("MG_CHEST " + IntToString(GetUseableFlag(oChest)) + "|" + IntToString(GetHasInventory(oChest)) + "|" + IntToString(nItems));
    Log("MG_STACK " + IntToString(GetItemStackSize(GetObjectByTag("MG_ARROWS"))) + "|" + IntToString(GetItemCharges(GetObjectByTag("MG_WAND"))));
    Log("MG_DONE");
}
"#;

#[test]
fn the_engine_reads_the_ee_fields_aurora_has_none_for() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_ee_fields");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(&game, "EE Fields", &mut rng).unwrap();
    let spec = AreaSpec { name: "Fields".into(), tileset: r("ttr01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();

    // Area flags past the three, and the first tile's replacement texture.
    let are_key = ResKey::new(area, ResType::ARE);
    let mut are = m.gff(&are_key).unwrap().unwrap();
    are.root.set("Flags", Value::Dword(0x1 | 0x8 | 0x100));
    are.root.list_mut("Tile_List").unwrap()[0].set("Tile_ReplaceTex", Value::Byte(1));
    m.set_gff(are_key, &are).unwrap();

    let blueprint = |name: &str, t: ResType| -> Struct {
        Gff::read(&game.resman.get(&ResKey::new(r(name), t)).unwrap()).unwrap().root
    };
    let first = |t: ResType| -> Struct {
        let name = game.resman.list(t)[0];
        blueprint(&name.to_string(), t)
    };
    let items = |res: ResRef| {
        let data = game.resman.get(&ResKey::new(res, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let placing = Placing { game: &game, item: &items };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let mut place = |t: ResType, mut s: Struct, tag: &str, x: f32, outline: bool| {
        s.set("Tag", text(tag));
        let at = Placement { position: [x, 20.0, 0.0], rotation: 0.0 };
        let shape: Vec<[f32; 3]> = if outline {
            vec![[x, 18.0, 0.0], [x + 1.0, 18.0, 0.0], [x, 19.0, 0.0]]
        } else {
            vec![]
        };
        let placed = instance(&placing, t, &s, at, &shape).unwrap();
        let (list, _) = git_list(t).unwrap();
        let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
        objects.push(placed);
        git.root.set(list, Value::List(objects));
    };

    // One of each kind, with visuals.
    let kinds = [
        (ResType::UTP, blueprint("plc_chest1", ResType::UTP), "MG_PLC", false),
        (ResType::UTD, first(ResType::UTD), "MG_DOOR", false),
        (ResType::UTC, blueprint("nw_bandit001", ResType::UTC), "MG_CRE", false),
        (ResType::UTI, blueprint("nw_wamar001", ResType::UTI), "MG_ITEM", false),
        (ResType::UTW, blueprint("nw_waypoint001", ResType::UTW), "MG_WP", false),
        (ResType::UTT, first(ResType::UTT), "MG_TRIG", true),
        (ResType::UTS, first(ResType::UTS), "MG_SOUND", false),
        (ResType::UTM, first(ResType::UTM), "MG_STORE", false),
        (ResType::UTE, first(ResType::UTE), "MG_ENC", true),
    ];
    for (i, (t, mut s, tag, outline)) in kinds.into_iter().enumerate() {
        add_visuals(&mut s);
        place(t, s, tag, 3.0 + 3.0 * i as f32, outline);
    }

    // Creatures: the familiar and companion fields on each, read only where
    // a class has one.
    let creature = |list: &[Class<'_>]| {
        let mut s = blueprint("nw_bandit001", ResType::UTC);
        classes(&mut s, list);
        s.set("FamiliarType", Value::Int(3));
        s.set("FamiliarName", text("Fluffy"));
        s.set("CompanionType", Value::Int(2));
        s.set("CompanionName", text("Rex"));
        s
    };
    let school: &[(&str, Value)] = &[("School", Value::Byte(2))];
    let domains: &[(&str, Value)] = &[("Domain1", Value::Byte(5)), ("Domain2", Value::Byte(7))];
    for (tag, list, x) in [
        ("MG_WIZ", vec![(10, 3, school)], 3.0),
        ("MG_CLR", vec![(2, 3, domains)], 6.0),
        ("MG_DRU", vec![(3, 1, &[][..])], 9.0),
        ("MG_RGR3", vec![(7, 3, &[][..])], 12.0),
        ("MG_RGR6", vec![(7, 6, &[][..])], 15.0),
    ] {
        let s = creature(&list);
        let mut s = s;
        s.set("Tag", text(tag));
        let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
        let placed = instance(&placing, ResType::UTC, &s, at, &[]).unwrap();
        let mut objects = git.root.list("Creature List").unwrap_or(&[]).to_vec();
        objects.push(placed);
        git.root.set("Creature List", Value::List(objects));
    }

    // A chest with an inventory that isn't usable.
    let mut chest = blueprint("plc_chest1", ResType::UTP);
    chest.set("Useable", Value::Byte(0));
    chest.set("HasInventory", Value::Byte(1));
    let mut entry = Struct::new(0);
    entry.set("InventoryRes", Value::resref(r("nw_wamar001")));
    entry.set("Repos_PosX", Value::Word(0));
    entry.set("Repos_Posy", Value::Word(0));
    chest.set("ItemList", Value::List(vec![entry]));
    // Items: 5,000 arrows (the base item stacks to 99), a wand with 255
    // charges.
    let mut arrows = blueprint("nw_wamar001", ResType::UTI);
    arrows.set("StackSize", Value::Word(5000));
    let wand_name = game
        .resman
        .list(ResType::UTI)
        .into_iter()
        .find(|n| {
            items(*n).is_some_and(|s| s.integer("Charges").unwrap_or(0) > 0)
                && n.to_string().contains("wmgwn")
        })
        .expect("a wand");
    let mut wand = items(wand_name).unwrap();
    wand.set("Charges", Value::Byte(255));
    let mut place = |t: ResType, mut s: Struct, tag: &str, x: f32| {
        s.set("Tag", text(tag));
        let at = Placement { position: [x, 35.0, 0.0], rotation: 0.0 };
        let placed = instance(&placing, t, &s, at, &[]).unwrap();
        let (list, _) = git_list(t).unwrap();
        let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
        objects.push(placed);
        git.root.set(list, Value::List(objects));
    };
    place(ResType::UTP, chest, "MG_CHEST", 3.0);
    place(ResType::UTI, arrows, "MG_ARROWS", 6.0);
    place(ResType::UTI, wand, "MG_WAND", 9.0);
    m.set_gff(git_key, &git).unwrap();

    // A placeable blueprint with visuals, for the script to create.
    let mut made = blueprint("plc_chest1", ResType::UTP);
    add_visuals(&mut made);
    made.set("TemplateResRef", Value::resref(r("mg_vis")));
    let mut g = Gff::new(*b"UTP ");
    g.root = made;
    m.set_gff(ResKey::new(r("mg_vis"), ResType::UTP), &g).unwrap();

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_fields.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_fields", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);

    // Visuals, per kind: all four on placeables, doors, creatures and
    // items (placed or made from a blueprint), the visible distance alone
    // on triggers, nothing on the rest.
    for (tag, all, distance) in [
        ("MG_PLC", true, true),
        ("MG_DOOR", true, true),
        ("MG_CRE", true, true),
        ("MG_ITEM", true, true),
        ("MG_MADE", true, true),
        ("MG_TRIG", false, true),
        ("MG_WP", false, false),
        ("MG_SOUND", false, false),
        ("MG_STORE", false, false),
        ("MG_ENC", false, false),
    ] {
        let obj = run.values("MG_OBJ").into_iter().find(|v| v.starts_with(&format!("{tag}|")));
        let want_distance = if distance { "12.5" } else { "45.0" };
        // The rest of MiscVisuals keeps its defaults (UI discovery 15).
        assert_eq!(obj, Some(format!("{tag}|1|{want_distance}|15")), "{tag}");
        let j: Json = serde_json::from_str(&run.values(&format!("MG_J_{tag}")).concat()).unwrap();
        let tex = &j["TextureReplace"]["value"]["TextureReplaceLi"]["value"][0];
        let anim = &j["AnimationReplace"]["value"]["AnimationReplace"]["value"][0];
        let params = j["Material"]["value"]["ShaderParams"]["value"].as_array();
        if all {
            assert_eq!(tex["OldTexture"]["value"], "mg_old", "{tag}");
            assert_eq!(tex["NewTexture"]["value"], "mg_new", "{tag}");
            assert_eq!(anim["OldAnimation"]["value"], "walk", "{tag}");
            assert_eq!(anim["NewAnimation"]["value"], "mg_walk", "{tag}");
            let params = params.unwrap();
            assert_eq!(params[0]["Int"]["value"], 7, "{tag}");
            assert_eq!(params[1]["Float4"]["value"], 4.0, "{tag}");
        } else {
            assert!(tex.is_null() && anim.is_null() && params.is_none(), "{tag}: {j}");
        }
    }

    // Area flags and the tile's texture are kept.
    assert_eq!(run.values("MG_AREA"), [r#"1|{"type":"dword","value":265}"#]);
    assert_eq!(run.values("MG_TILE"), [r#"{"type":"byte","value":1}"#]);

    // Familiars only with a wizard; companions with a druid or ranger of any
    // level; the school and domains.
    let classes = run.values("MG_CLASSES");
    for want in [
        "MG_WIZ|3|Fluffy|0||2|-1|-1",
        "MG_CLR|0||0||-1|5|7",
        "MG_DRU|0||2|Rex|-1|-1|-1",
        "MG_RGR3|0||2|Rex|-1|-1|-1",
        "MG_RGR6|0||2|Rex|-1|-1|-1",
    ] {
        assert!(classes.contains(&want.to_string()), "{want}: {classes:?}");
    }

    // The chest isn't usable and holds its item; the stack is kept whole;
    // charges stop at 250.
    assert_eq!(run.values("MG_CHEST"), ["0|1|1"]);
    assert_eq!(run.values("MG_STACK"), ["5000|250"]);
}
