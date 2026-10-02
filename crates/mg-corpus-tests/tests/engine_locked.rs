//! Locked objects (Moonglow's `MG_Locked` byte on a placed object) are the
//! same objects to the engine: modules with every placed object locked
//! present the same world as the originals.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::Value;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::{PROBE, compile, world};

#[test]
fn locked_objects_are_the_same_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_locked");
    let probe = compile(&dir, "mg_probe", PROBE);
    let probe_key = ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS);
    let mut locked_objects = 0;
    for (i, name) in ["The Winds of Eremor.mod", "To Heir is Human.mod"].iter().enumerate() {
        let mut worlds = Vec::new();
        for lock in [false, true] {
            let mut m = Module::open(&root.join("data/mod").join(name)).unwrap();
            let mut info = m.info().unwrap();
            info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
            m.set_info(&info).unwrap();
            m.set(probe_key, probe.clone());
            if lock {
                let gits: Vec<ResKey> = m.keys_of(ResType::GIT).copied().collect();
                for git in gits {
                    let mut g = m.gff(&git).unwrap().unwrap();
                    for f in &mut g.root.fields {
                        if let Value::List(items) = &mut f.value {
                            for item in items {
                                item.set(mg_area::LOCKED, Value::Byte(1));
                                locked_objects += 1;
                            }
                        }
                    }
                    m.set_gff(git, &g).unwrap();
                }
            }
            let module = format!("l{i}{}", u8::from(lock));
            let user = dir.join(format!("user-{module}"));
            m.save_as(&ModuleLocation::Archive(user.join(format!("modules/{module}.mod"))))
                .unwrap();
            let run =
                run_server(&root, &user, &module, "MG_DONE", Duration::from_secs(300)).unwrap();
            assert!(run.finished, "{name}: the server didn't finish");
            worlds.push(world(&run));
        }
        assert!(worlds[0].len() > 10, "{name}: the probe saw little");
        assert_eq!(worlds[0], worlds[1], "{name}");
    }
    eprintln!("{locked_objects} objects locked");
}
