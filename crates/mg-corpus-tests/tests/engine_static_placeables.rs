//! What a static placeable is to scripts, settled in the engine (the wiki
//! says the server keeps static placeables as objects that scripts find;
//! a builders' guide says no script can find one). The server keeps it:
//! found by its tag and among the area's objects, its variables read, and
//! made Plot whatever its file says. (What scripts then do to it the
//! client doesn't show: it is merged into the area's tiles.) Make
//! Placeables Static keeps a placeable with variables dynamic for that.
//!
//! And of two objects with one tag, `GetObjectByTag` took the one placed
//! second: a copy pasted after its original is what a script finds.

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

use crate::common;
use common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

void Seen(string sTag)
{
    object o = GetObjectByTag(sTag);
    int nInArea = 0;
    object oIn = GetFirstObjectInArea(GetArea(GetObjectByTag("MG_DYNAMIC")));
    while (GetIsObjectValid(oIn))
    {
        if (GetTag(oIn) == sTag) nInArea++;
        oIn = GetNextObjectInArea(GetArea(GetObjectByTag("MG_DYNAMIC")));
    }
    Log("MG_SEEN " + sTag + "|" + IntToString(GetIsObjectValid(o)) + "|"
        + IntToString(GetLocalInt(o, "mg_n")) + "|" + IntToString(nInArea) + "|"
        + IntToString(GetPlotFlag(o)) + "|" + IntToString(GetUseableFlag(o)));
}

void main()
{
    Seen("MG_STATIC");
    Seen("MG_DYNAMIC");
    // Two objects with one tag: which the lookup takes.
    Log("MG_TWICE " + IntToString(GetLocalInt(GetObjectByTag("MG_TWICE"), "mg_n")) + "|"
        + IntToString(GetLocalInt(GetObjectByTag("MG_TWICE", 1), "mg_n")));
    Log("MG_DONE");
}
"#;

#[test]
fn scripts_find_a_static_placeable_and_its_variables() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_static_placeables");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Static Placeables", &mut rng).unwrap();
    let spec = AreaSpec { name: "Statics".into(), tileset: r("tic01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();

    let barrel = || -> Struct {
        Gff::read(&game.resman.get(&ResKey::new(r("plc_barrel"), ResType::UTP)).unwrap())
            .unwrap()
            .root
    };
    let var = |n: i32| {
        let mut v = Struct::new(0);
        v.set("Name", Value::String(b"mg_n".to_vec()));
        v.set("Type", Value::Dword(1));
        v.set("Value", Value::Int(n));
        Value::List(vec![v])
    };
    let items = |_: ResRef| -> Option<Struct> { None };
    let placing = Placing { game: &game, item: &items };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTP).unwrap();
    let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
    for (i, (tag, is_static, n)) in
        [("MG_STATIC", 1, 7), ("MG_DYNAMIC", 0, 8), ("MG_TWICE", 0, 1), ("MG_TWICE", 0, 2)]
            .into_iter()
            .enumerate()
    {
        let mut s = barrel();
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s.set("Static", Value::Byte(is_static));
        s.set("Useable", Value::Byte(0));
        s.set("Plot", Value::Byte(0));
        s.set("VarTable", var(n));
        let at = Placement { position: [3.0 + 3.0 * i as f32, 20.0, 0.0], rotation: 0.0 };
        objects.push(instance(&placing, ResType::UTP, &s, at, &[]).unwrap());
    }
    git.root.set(list, Value::List(objects));
    m.set_gff(git_key, &git).unwrap();

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_static.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_static", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let seen = run.values("MG_SEEN");
    let twice = run.values("MG_TWICE");
    // tag | found | its variable | in the area's objects | plot | useable:
    // the static one is Plot, its file's Plot 0 notwithstanding.
    assert_eq!(seen, ["MG_STATIC|1|7|1|1|0", "MG_DYNAMIC|1|8|1|0|0"], "{}", run.log);
    // The second placed is the first found.
    assert_eq!(twice, ["2|1"], "{}", run.log);
}
