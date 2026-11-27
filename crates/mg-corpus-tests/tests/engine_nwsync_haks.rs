//! The haks and talk table `mg serve`, `mg test` and Test Module write
//! from a module's NWSync content (`[nwsync] source`;
//! `mg_module::nwsync_content::write_haks`, through
//! `mg_erf::write_streamed`) are read by the game's server: a 2DA from a
//! written hak resolves (`Get2DAString`) and a string of the written custom
//! talk table reads back (`GetStringByStrRef`). The content comes from a
//! made-up game NWSync storage in a folder of the test's own (the
//! client's schema, `mg_nwsync::fixtures`), never the player's.

use std::time::Duration;

use mg_core::sha1::sha1;
use mg_core::{Language, ResType};
use mg_erf::{Erf, ErfWriter};
use mg_gff::{Gff, Value};
use mg_resman::{GameInstall, ResKey};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, scratch_dir};

use crate::common::probe_module;

const PROBE: &str = r#"void main()
{
    WriteTimestampedLogEntry("MG_2DA [" + Get2DAString("mg_sync", "Label", 1) + "]");
    WriteTimestampedLogEntry("MG_TLK [" + GetStringByStrRef(16777217) + "]");
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

#[test]
fn the_server_reads_haks_and_a_talk_table_written_from_nwsync() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let dir = scratch_dir("engine_nwsync_haks");
    let (player, server) = (dir.join("player"), dir.join("server"));
    for sub in ["hak", "tlk"] {
        std::fs::create_dir_all(player.join(sub)).unwrap();
        std::fs::create_dir_all(server.join(sub)).unwrap();
    }

    // The server's content: a 2DA (in hak mg_synced, the only hak the
    // module lists: no origins file, so the manifest goes into it) and
    // the module's custom talk table.
    let table = b"2DA V2.0\n\n   Label\n0  zero\n1  synced\n".to_vec();
    let mut tlk = mg_tlk::Tlk::new(Language::ENGLISH);
    tlk.entries.push(mg_tlk::TlkEntry::text("row zero"));
    tlk.entries.push(mg_tlk::TlkEntry::text("From NWSync"));
    let tlk = tlk.to_bytes().unwrap();
    let entries = [
        (ResKey::parse("mg_sync", ResType::TWODA).unwrap(), sha1(&table), table.len() as u32),
        (ResKey::parse("mg_sync", ResType::TLK).unwrap(), sha1(&tlk), tlk.len() as u32),
    ];
    let manifest = mg_nwsync::manifest::write(&entries);
    let url = "https://nwsync.example.org";
    mg_nwsync::fixtures::write_storage(
        &player.join("nwsync"),
        &[mg_nwsync::fixtures::Registered {
            sha1: sha1(&manifest),
            origin_url: url.into(),
            created_at: 1,
            last_used_at: 1,
            resources: entries.iter().map(|(k, s, _)| (*k, *s)).collect(),
            cached: Some(manifest.clone()),
        }],
        &[vec![(sha1(&table), table.clone()), (sha1(&tlk), tlk.clone())]],
    );

    // The module: the probe on load, hak mg_synced, talk table mg_sync.
    probe_module(&root, &server, "mgsync", PROBE, &["mg_synced"], &[]);
    let path = server.join("modules/mgsync.mod");
    let data = std::fs::read(&path).unwrap();
    let erf = Erf::read(&data).unwrap();
    let mut w = ErfWriter::new(*b"MOD ");
    for e in &erf.entries {
        let mut bytes = erf.data(e).unwrap().into_owned();
        if e.restype == ResType::IFO {
            let mut ifo = Gff::read(&bytes).unwrap();
            ifo.root.set("Mod_CustomTlk", Value::String(b"mg_sync".to_vec()));
            bytes = ifo.to_bytes().unwrap();
        }
        w.add(e.resref, e.restype, bytes).unwrap();
    }
    std::fs::write(&path, w.to_bytes().unwrap()).unwrap();

    // Written from the content as `mg serve` writes them (into a cache of
    // the test's own), and placed where its decision says: each hak it
    // wrote, and the talk table.
    let gi = GameInstall::new(&root, Some(player.clone()), "en");
    let mut m = mg_module::Module::open(&path).unwrap();
    m.project_file.nwsync_source = Some(url.into());
    m.project_file.nwsync_cache = Some(dir.join("cache"));
    let o = mg_module::nwsync_content::WriteOptions::default();
    let files = m.write_nwsync_haks(&gi, &o).unwrap().unwrap();
    let hak = files.hak("mg_synced").expect("written from the manifest");
    std::fs::copy(&hak.path, server.join("hak/mg_synced.hak")).unwrap();
    let (name, written) = files.tlk.as_ref().expect("the manifest's talk table");
    std::fs::copy(written, server.join("tlk").join(name)).unwrap();

    let run = run_server(&root, &server, "mgsync", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "{:?}\n{}", run.why, run.log);
    assert!(run.log.contains("MG_2DA [synced]"), "{}", run.log);
    assert!(run.log.contains("MG_TLK [From NWSync]"), "{}", run.log);
}
