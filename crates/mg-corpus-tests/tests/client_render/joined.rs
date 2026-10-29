//! Readings of a client joined to a server (`mg_testkit::joined`: the
//! headless server and the client in one sandbox, on a loopback of its
//! own). Each takes pictures of the client's window into its scratch
//! directory and prints where; what is read is in them.
//!
//! The sandboxed client has no CD key (no Steam): these skip, saying so,
//! unless `MG_CLIENT_CDKEY` names a `cdkey.ini` (`mg_testkit::joined`).
//! Seen without one (2026-10-29, 89.8193.37): the server and the client
//! start in the one sandbox, the server loads its module, and the
//! client's `+connect 127.0.0.1:PORT` is answered "Could not translate
//! address", then "Your CD Key is unauthorized"; its Multiplayer asks
//! "Enter CD-KEY". Not run past that here.
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=:2 MG_CLIENT_CDKEY=... cargo test -p mg-corpus-tests --release --test client_render joined -- --ignored --nocapture`.

use super::*;
use mg_testkit::joined::{self, Join};

/// The scratch directory with `module` (a module saved elsewhere) as the
/// server's `modules/<name>.mod`.
fn scratch_with(name: &str, module: &mut mg_module::Module) -> std::path::PathBuf {
    let dir = scratch_dir(&format!("client_joined_{name}"));
    let modules = joined::server_dir(&dir).join("modules");
    std::fs::create_dir_all(&modules).unwrap();
    module.save_as(&ModuleLocation::Archive(modules.join("MgJoined.mod"))).unwrap();
    std::fs::create_dir_all(joined::client_dir(&dir)).unwrap();
    std::fs::write(joined::client_dir(&dir).join("settings.tml"), SETTINGS).unwrap();
    dir
}

/// Pictures of the client every ten seconds for `seconds`, as
/// `joined_<i>.png`.
fn watch(j: &joined::Joined, seconds: u64) {
    for i in 0..seconds / 10 {
        std::thread::sleep(Duration::from_secs(10));
        if let Some(p) = j.winshot(&format!("joined_{i}")) {
            eprintln!("{}", p.display());
        }
    }
}

/// The DM client's Creator with a palette category named from the
/// module's own talk table (Palette › Categories…, Name by StrRef: a
/// line 16777216 and up), and a custom palette's names with letters
/// outside ASCII (the palettes entries of `docs/deferred.md`): the DM
/// joins with the server's DM password, and the pictures show what the
/// client shows; the Creator is opened by hand in them.
#[test]
#[ignore]
fn joined_dm_creator_palette() {
    let Some(_) = joined::cd_key() else {
        eprintln!("skipped: the sandboxed client has no CD key (MG_CLIENT_CDKEY)");
        return;
    };
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let light = Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
    let dir0 = scratch_dir("client_joined_dm_build");
    let (mut m, _) = build_module(&game, &dir0, light);
    let dir = scratch_with("dm", &mut m);
    let j = joined::start(&dir, "MgJoined", Join::Dm, None, &[]).unwrap();
    watch(&j, 90);
    j.stop();
}

/// An NWSync download: the module (Neverwinter Chess with a hak and a
/// talk table of the game's) published into a folder the sandbox serves
/// over HTTP, the server given its URL and hash, and the player client
/// joining: what it downloads lands in its user directory's `nwsync`.
#[test]
#[ignore]
fn joined_nwsync_download() {
    let Some(_) = joined::cd_key() else {
        eprintln!("skipped: the sandboxed client has no CD key (MG_CLIENT_CDKEY)");
        return;
    };
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let install = GameInstall::new(&root, None, "en");
    let mut m = mg_module::Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let mut ifo = m.info().unwrap();
    ifo.root.set("Mod_HakList", mg_module::attach::hak_list(&ifo.root, &["sg_resources".into()]));
    ifo.root.set("Mod_CustomTlk", Value::String(b"tyrants".to_vec()));
    m.set_info(&ifo).unwrap();
    let dir = scratch_with("nwsync", &mut m);
    let path = joined::server_dir(&dir).join("modules/MgJoined.mod");
    let m = mg_module::Module::open(&path).unwrap();
    let mut rm = mg_resman::ResMan::for_game(&install).unwrap();
    let _ = m.stack_onto(&mut rm, &install);
    let (contents, missing) =
        mg_module::nwsync::module_contents(&m, &install, &rm, false, None).unwrap();
    assert!(missing.is_empty(), "{missing:?}");
    let web = dir.join("web");
    let o = mg_module::nwsync::Options { latest: true, ..Default::default() };
    let w = mg_module::nwsync::write(&web, &contents, &o, &mut |_, _| {}).unwrap();
    // (The server reads the hak and the talk table it names from its own
    // user directory.)
    for (from, to) in [
        ("data/hk/sg_resources.hak", "hak/sg_resources.hak"),
        ("data/tlk/tyrants.tlk", "tlk/tyrants.tlk"),
    ] {
        let to = joined::server_dir(&dir).join(to);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        mg_testkit::link_or_copy(&root.join(from), &to).unwrap();
    }
    let j = joined::start(&dir, "MgJoined", Join::Player, Some(&web), &["-nwsynchash", &w.sha1])
        .unwrap();
    watch(&j, 120);
    let synced = joined::client_dir(&dir).join("nwsync");
    eprintln!("the client's nwsync folder: {}", synced.display());
    j.stop();
}
