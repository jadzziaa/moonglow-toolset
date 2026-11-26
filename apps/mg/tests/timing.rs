//! A timing view of the review of November 2026's fix M17 (the deferred
//! list's "performance fixes without a timing view"): `mg haks --usage`
//! on a module of many part-based creatures and of scripts that name
//! things, beside the same command on a small module (the process's own
//! start and the game's opening are a cost of both). Run by hand, in
//! release; it prints `TIMING name = ms` lines and the machine's load:
//! `cargo test --release -p mg --test cli timing -- --ignored --nocapture`.
//! `MG_TIMING_BIN` names the `mg` to run (a copy of one built at another
//! commit), `MG_TIMING_DIR` the folder the two modules are made in (kept:
//! made once, so that two builds read the same). The game's install is
//! read; the modules are made here from nothing of its own.

use std::path::{Path, PathBuf};
use std::time::Instant;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::{Module, ModuleLocation};
use mg_preview::CreatureLook;
use mg_resman::ResKey;
use mg_schema::{ExoString, StructExt, ifo};

fn load() -> String {
    std::fs::read_to_string("/proc/loadavg").unwrap_or_default().trim().to_string()
}

/// `areas` areas of `creatures` part-based creatures each (a look of
/// their own from the heads, bodies and races), and `scripts` scripts of
/// four strings each, in a folder module with one hak.
fn make(dir: &Path, areas: usize, creatures: usize, scripts: usize) {
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    let mut h = ifo::MOD_HAK_LIST.new_item();
    h.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from("timing_hak"));
    info.root.items_mut(&ifo::MOD_HAK_LIST).push(h);
    for a in 0..areas {
        let mut item = ifo::MOD_AREA_LIST.new_item();
        item.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str(&format!("a{a}")).unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(item);
    }
    m.set_info(&info).unwrap();
    for a in 0..areas {
        let mut are = Gff::new(*b"ARE ");
        are.root.set("Tileset", Value::resref(ResRef::from_str("tcn01").unwrap()));
        m.set_gff(ResKey::parse(&format!("a{a}"), ResType::ARE).unwrap(), &are).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut list = Vec::with_capacity(creatures);
        for c in 0..creatures {
            let n = a * creatures + c;
            let look = CreatureLook {
                appearance: [6, 6, 1, 0, 2, 3, 4, 5][n % 8],
                gender: (n % 2) as u8,
                head: 1 + (n % 12) as u8,
                body: 1 + (n / 12 % 8) as u8,
                ..CreatureLook::new(6)
            };
            let mut s = look.to_utc();
            s.set("Tag", Value::String(format!("C{n}").into_bytes()));
            s.set("TemplateResRef", Value::resref(ResRef::from_str("nw_bandit").unwrap()));
            list.push(s);
        }
        git.root.set("Creature List", Value::List(list));
        m.set_gff(ResKey::parse(&format!("a{a}"), ResType::GIT).unwrap(), &git).unwrap();
    }
    for s in 0..scripts {
        let body = format!(
            "void main() {{\n  object o = CreateObject(OBJECT_TYPE_CREATURE, \"nw_bandit\", GetLocation(OBJECT_SELF));\n  \
             object p = GetObjectByTag(\"nw_chest{}\");\n  AssignCommand(o, PlayAnimation(ANIMATION_LOOPING_PAUSE));\n  \
             SetLocalString(o, \"plc_chest1\", \"nw_wblcl001\");\n  SendMessageToPC(o, \"plc_barrel1 s{s}\");\n}}\n",
            s % 40
        );
        m.set(ResKey::parse(&format!("s{s}"), ResType::NSS).unwrap(), body.into_bytes());
    }
    m.save_as(&ModuleLocation::Folder(dir.to_path_buf())).unwrap();
}

fn hak(user: &Path) {
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    w.add(ResRef::from_str("timing_note").unwrap(), ResType::TXT, b"x".to_vec()).unwrap();
    std::fs::create_dir_all(user.join("hak")).unwrap();
    std::fs::write(user.join("hak/timing_hak.hak"), w.to_bytes().unwrap()).unwrap();
}

fn run(root: &Path, user: &Path, module: &Path) -> (std::time::Duration, usize) {
    let bin = std::env::var_os("MG_TIMING_BIN")
        .map_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_mg")), PathBuf::from);
    let home = user.join("home");
    let t = Instant::now();
    let out = std::process::Command::new(bin)
        .env("MOONGLOW_DATA_DIR", home.join("data"))
        .env("MOONGLOW_CACHE_DIR", home.join("cache"))
        .arg("--root")
        .arg(root)
        .arg("--user-dir")
        .arg(user)
        .arg("--json")
        .args(["haks", "--usage"])
        .arg(module)
        .output()
        .unwrap();
    let took = t.elapsed();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    (took, out.stdout.len())
}

#[test]
#[ignore]
fn m17_haks_usage_of_many_creatures() {
    let root = mg_testkit::corpus!();
    let dir = std::env::var_os("MG_TIMING_DIR")
        .map_or_else(|| mg_testkit::scratch_dir("mg-timing-usage"), PathBuf::from);
    let user = dir.join("user");
    hak(&user);
    for (label, areas, creatures, scripts) in
        [("small", 1, 20, 200), ("20,000 creatures, 10,000 scripts", 20, 1_000, 10_000)]
    {
        let module = dir.join(format!("module-{areas}-{creatures}"));
        if !module.exists() {
            make(&module, areas, creatures, scripts);
        }
        let mut took = Vec::new();
        for _ in 0..3 {
            let (t, bytes) = run(&root, &user, &module);
            assert!(bytes > 0);
            took.push(format!("{:.0}", t.as_secs_f64() * 1000.0));
        }
        println!("TIMING M17 {label}: mg haks --usage = {} ms (load {})", took.join(" "), load());
    }
}
