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
//! "Enter CD-KEY". With the owner's key (the third client batch,
//! 2026-10-29) both readings ran; the Creator is driven by hand
//! (`MG_JOINED_HOLD`, `tools/aurora/xdrive.py`: a tree node opens on a
//! quick double click, not on its `+`).
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=$D MG_CLIENT_CDKEY=... cargo test -p mg-corpus-tests --release --test client_render joined -- --ignored --nocapture`.

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
    std::fs::write(joined::client_dir(&dir).join("settings.tml"), joined_settings()).unwrap();
    // The game's own DM character in the DM vault: the DM client's list
    // is empty without one, pregenerated or not (seen 2026-10-29). (A
    // copy: the client may write it.)
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let vault = joined::client_dir(&dir).join("dmvault");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::copy(root.join("data/dmv/dungeonmaster.bic"), vault.join("dm.bic")).unwrap();
    dir
}

/// The client's settings: the reference scenes', and a player name (else
/// joining stops at "Please Enter a Player Name", seen 2026-10-29).
fn joined_settings() -> String {
    format!("{SETTINGS}[client]\n\t[client.identity]\n\t\tname = \"MgTester\"\n")
}

/// How long a joined reading keeps the client: `MG_JOINED_HOLD` seconds
/// (to drive it by hand meanwhile, `tools/aurora/xdrive.py`), else
/// `seconds`.
fn hold(seconds: u64) -> u64 {
    std::env::var("MG_JOINED_HOLD").ok().and_then(|s| s.parse().ok()).unwrap_or(seconds)
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
///
/// Seen (2026-10-29, 89.8193.37, Creator › Placeable Object › Custom):
/// "Hipólito's Chests" holding "Red Chest" (Red in red), "Hipólito" and
/// "Señora Peña's Chest", and "MG Talk Table Category" holding its
/// chest. Polish (`MG_JOINED_LANG=pl`): "Skrzynie Łucji", "Zażółć gęślą
/// jaźń", "Skrzynia Świętego Łukasza", "MG Kategoria z tabeli: Źdźbło".
#[test]
#[ignore]
fn joined_dm_creator_palette() {
    let Some(_) = joined::cd_key() else {
        eprintln!("skipped: the sandboxed client has no CD key (MG_CLIENT_CDKEY)");
        return;
    };
    use mg_core::{Gender, Language, LocString};
    use mg_module::palette::{self, BlueprintKind};
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    // (`MG_JOINED_LANG=pl`: the game, the client and the names Polish.)
    let polish = std::env::var("MG_JOINED_LANG").is_ok_and(|l| l == "pl");
    let (code, language) =
        if polish { ("pl", Language::POLISH) } else { ("en", Language::ENGLISH) };
    let game = GameData::open(&GameInstall::new(&root, None, code)).unwrap();
    let light = Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
    let dir0 = scratch_dir("client_joined_dm_build");
    let (mut m, _) = build_module(&game, &dir0, light);
    // The module's own talk table, in the server's and the client's `tlk`.
    let mut t = mg_tlk::Tlk::new(language);
    let talk_name =
        if polish { "MG Kategoria z tabeli: Źdźbło" } else { "MG Talk Table Category" };
    // (In the game's bytes for the language, as a talk table is.)
    t.entries.push(mg_tlk::TlkEntry::text(game.codepage().encode(talk_name).unwrap().into_owned()));
    let tlk = t.to_bytes().unwrap();
    let mut info = m.info().unwrap();
    info.root.set("Mod_CustomTlk", Value::String(b"mgjoined".to_vec()));
    m.set_info(&info).unwrap();
    // The placeables' own skeleton: two categories of the module's.
    let cp = game.codepage();
    let kind = BlueprintKind::Placeable;
    let mut skeleton = palette::skeleton(&m, &game, kind).unwrap();
    let category = if polish { "Skrzynie Łucji" } else { "Hipólito's Chests" };
    let (_, hipolito) = palette::add_category(&mut skeleton, &[], category, cp).unwrap();
    let (path, talk) = palette::add_category(&mut skeleton, &[], "placeholder", cp).unwrap();
    palette::name_by_strref(&mut skeleton, &path, 16_777_216).unwrap();
    m.set(kind.skeleton_key(), skeleton.to_bytes().unwrap());
    let chest = Gff::read(&game.resman.get_named("plc_chest1", ResType::UTP).unwrap()).unwrap();
    let (a, b) = if polish {
        ("Zażółć gęślą jaźń", "Skrzynia Świętego Łukasza")
    } else {
        ("Hipólito", "Señora Peña's Chest")
    };
    let hip = cp.encode(a).unwrap().into_owned();
    let pena = cp.encode(b).unwrap().into_owned();
    let names: [(&str, &[u8], u8); 4] = [
        ("mg_hipolito", &hip, hipolito),
        ("mg_pena", &pena, hipolito),
        ("mg_red", b"<c\xff\x20\x20>Red</c> Chest", hipolito),
        ("mg_talk", b"Chest of the Talk Table", talk),
    ];
    for (resref, name, category) in names {
        let mut g = chest.clone();
        g.root.set("TemplateResRef", Value::ResRef(resref.as_bytes().to_vec()));
        g.root.set("Tag", Value::String(resref.to_uppercase().into_bytes()));
        g.root.set("LocName", Value::LocString(LocString::from_text(language, Gender::Male, name)));
        g.root.set("PaletteID", Value::Byte(category));
        m.set(ResKey::parse(resref, ResType::UTP).unwrap(), g.to_bytes().unwrap());
    }
    palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    let dir = scratch_with(if polish { "dm_pl" } else { "dm" }, &mut m);
    if polish {
        let settings =
            format!("{}[game]\n\t[game.language]\n\t\toverride = \"pl\"\n", joined_settings());
        std::fs::write(joined::client_dir(&dir).join("settings.tml"), settings).unwrap();
    }
    for d in [joined::server_dir(&dir), joined::client_dir(&dir)] {
        std::fs::create_dir_all(d.join("tlk")).unwrap();
        std::fs::write(d.join("tlk/mgjoined.tlk"), &tlk).unwrap();
    }
    let j = joined::start(&dir, "MgJoined", Join::Dm, None, &[]).unwrap();
    watch(&j, hold(90));
    j.stop();
}

/// An NWSync download: the module (Neverwinter Chess with a hak of the
/// game's and a talk table of its own, `mg_rtlk`, with its feminine
/// `mg_rtlkf` beside it in the server's `tlk`) published into a folder
/// the sandbox serves over HTTP, the server given its URL and hash, and
/// the player client joining: what it downloads lands in its user
/// directory's `nwsync`. The client ends itself after the download; the
/// sandbox starts it again (`MG_CLIENT_RUNS`), and it joins with what it
/// downloaded. In the game (a pregenerated character chosen by hand,
/// `MG_JOINED_HOLD`), the entry script makes the player a woman and
/// shows line 16777216 by StrRef (the client's own lookup) and the
/// server's `GetStringByStrRef` of it for each gender.
///
/// Seen (2026-10-29, 89.8193.37): "You are missing 8.2 MB", all 1,327
/// files downloaded (every request 200) into the client's `nwsync`
/// databases; then "We disconnected you from the server while NWSync is
/// running" and the client ends (exit 0). Started again, it finds the
/// server, shows the character list and ends again (exit 0): a player in
/// the game after a download isn't reached in the sandbox.
///
/// The fifth client batch (2026-10-30): the client's Downloader asks
/// first ("You do not have all needed content to play. You are missing
/// 8.2 MB"), its Download clicked by hand (`xdrive.py`). Published at
/// zstd level 22 (`MG_NWSYNC_LEVEL=22`) and at 3, all 1,327 data files
/// were taken into the client's `nwsyncdata_0.sqlite3`, each stored
/// compressed again by the client (none of level 22's, and 1,318 of
/// level 3's, the same bytes as served): it reads both levels. Started
/// again, the client joins and leaves the server within the second
/// (`Joined as Player 1`, `Left as a Player` in the server's log), its
/// own logs saying nothing of why.
///
/// The seventh client batch (2026-11-02): why. With the client's network
/// log on (`client-settings.tml`, put back before each start), its second
/// start is told less of the server (`BNXR` of 61 bytes, where the first
/// was 125) and, after the character list's request, leaves; the server
/// had reloaded its module as its last player left for the download
/// (`server.reload-when-empty`, on by default), and the module reloaded
/// is advertised without the NWSync manifest (the client's own server
/// code: `CServerExoAppInternal::PlayerListChange` loads it again with an
/// empty `NWSync::Advertisement`), so the client, its haks not in the
/// game, leaves; and any return to the main menu ends a client started
/// with `+connect` (`CClientExoAppInternal::ShutDownToMainMenu` calls
/// `CAutoConnectPanel::Bail`, which sets `g_bExitProgram`): the exit 0
/// after the download too. With the module kept loaded
/// (`mg_testkit::joined`'s server settings), the second start is told
/// the manifest, shows the character list, and Play (clicked with
/// `MG_NWSYNC_CLICK`) puts the player in the game: the entry script runs
/// (`MG_ENTERED`), the server's `GetStringByStrRef` reads `MG FEMININE
/// ZERO` for a woman, and the client's own lookup draws `MG MASCULINE
/// ZERO` for her, whether the feminine table is published or not
/// (`MG_NWSYNC_FEMININE=1`: downloaded into the client's `nwsync` beside
/// the masculine one): as in single player.
#[test]
#[ignore]
fn joined_nwsync_download() {
    let _ = mg_testkit::oracle_tool!("nwn_script_comp");
    let Some(_) = joined::cd_key() else {
        eprintln!("skipped: the sandboxed client has no CD key (MG_CLIENT_CDKEY)");
        return;
    };
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let mut m = mg_module::Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let mut ifo = m.info().unwrap();
    ifo.root.set("Mod_HakList", mg_module::attach::hak_list(&ifo.root, &["sg_resources".into()]));
    ifo.root.set("Mod_CustomTlk", Value::String(b"mg_rtlk".to_vec()));
    ifo.root.set("Mod_OnClientEntr", Value::ResRef(b"mg_enter".to_vec()));
    m.set_info(&ifo).unwrap();
    let enter = r#"
void main()
{
    object pc = GetEnteringObject();
    if (!GetIsPC(pc)) return;
    SetGender(pc, GENDER_FEMALE);
    WriteTimestampedLogEntry("MG_ENTERED");
    DelayCommand(6.0, FloatingTextStrRefOnCreature(16777216, pc, FALSE));
    DelayCommand(6.5, SendMessageToPC(pc, "MG f: " + GetStringByStrRef(16777216, GENDER_FEMALE)));
    DelayCommand(7.0, SendMessageToPC(pc, "MG m: " + GetStringByStrRef(16777216, GENDER_MALE)));
    WriteTimestampedLogEntry("MG_SERVER f: " + GetStringByStrRef(16777216, GENDER_FEMALE)
        + ", m: " + GetStringByStrRef(16777216, GENDER_MALE));
}
"#;
    let scripts = scratch_dir("client_joined_nwsync_scripts");
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(&scripts, "mg_enter", enter));
    let dir = scratch_with("nwsync", &mut m);
    // (The client's network debug log on, at every start: what it is told
    // as it joins and leaves. It writes its settings back without it.)
    let settings = joined_settings().replace(
        "[graphics]",
        "[game]\n\t[game.log]\n\t\t[game.log.debug]\n\t\t\tnetwork = true\n[graphics]",
    );
    std::fs::write(dir.join("client-settings.tml"), settings).unwrap();
    // (The server reads the hak and the talk tables from its own user
    // directory; so does the publishing.)
    let server = joined::server_dir(&dir);
    let to = server.join("hak/sg_resources.hak");
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    mg_testkit::link_or_copy(&root.join("data/hk/sg_resources.hak"), &to).unwrap();
    let table = |text: &str| {
        let mut t = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
        t.entries.push(mg_tlk::TlkEntry::text(format!("MG {text} ZERO").as_str()));
        t.to_bytes().unwrap()
    };
    std::fs::create_dir_all(server.join("tlk")).unwrap();
    std::fs::write(server.join("tlk/mg_rtlk.tlk"), table("MASCULINE")).unwrap();
    std::fs::write(server.join("tlk/mg_rtlkf.tlk"), table("FEMININE")).unwrap();
    let install = GameInstall::new(&root, Some(server.clone()), "en");
    let path = server.join("modules/MgJoined.mod");
    let m = mg_module::Module::open(&path).unwrap();
    let mut rm = mg_resman::ResMan::for_game(&install).unwrap();
    let _ = m.stack_onto(&mut rm, &install);
    let (mut contents, missing) =
        mg_module::nwsync::module_contents(&m, &install, &rm, false, None).unwrap();
    assert!(missing.is_empty(), "{missing:?}");
    // (`MG_NWSYNC_FEMININE=1`: the feminine table published beside it, which
    // `nwn_nwsync_write` and Moonglow leave out.)
    if std::env::var_os("MG_NWSYNC_FEMININE").is_some() {
        contents.push((
            ResKey::parse("mg_rtlkf", ResType::TLK).unwrap(),
            mg_module::nwsync::Source::Bytes(table("FEMININE").into()),
        ));
    }
    let web = dir.join("web");
    // (`MG_NWSYNC_LEVEL`: the data files' zstd level, 3 by default.)
    let level = std::env::var("MG_NWSYNC_LEVEL").ok().and_then(|l| l.parse().ok()).unwrap_or(0);
    let o = mg_module::nwsync::Options { latest: true, level, ..Default::default() };
    let w = mg_module::nwsync::write(&web, &contents, &o, &mut |_, _| {}).unwrap();
    let j = joined::start(&dir, "MgJoined", Join::Player, Some(&web), &["-nwsynchash", &w.sha1])
        .unwrap();
    // (`MG_NWSYNC_CLICK`: the NWSync Downloader's Download clicked, at
    // (435, 596) of the window, 25 s in; then, the client started again
    // and joined, Play on its character list (615, 615: the first
    // pregenerated character), and pictures in the game. Else it is all
    // clicked by hand.)
    if std::env::var_os("MG_NWSYNC_CLICK").is_some() {
        std::thread::sleep(Duration::from_secs(25));
        let _ = j.winshot("before_download");
        super::nui::click_in_window(435, 596);
        let start = std::time::Instant::now();
        let joined_again = || j.server.log().matches("Joined as Player").count() >= 2;
        while !joined_again() && start.elapsed() < Duration::from_secs(180) {
            std::thread::sleep(Duration::from_millis(500));
        }
        assert!(joined_again(), "the client did not join again after the download");
        std::thread::sleep(Duration::from_secs(5));
        let _ = j.winshot("character_list");
        super::nui::click_in_window(615, 615);
        let entered = j.wait_for("MG_ENTERED", Duration::from_secs(90));
        std::thread::sleep(Duration::from_secs(10));
        for i in 0..3 {
            let _ = j.winshot(&format!("in_game_{i}"));
            std::thread::sleep(Duration::from_secs(3));
        }
        for line in j.server.log().lines().filter(|l| l.contains("MG_") || l.contains("Player")) {
            eprintln!("{line}");
        }
        assert!(entered, "the player did not enter the game after the download");
    } else {
        watch(&j, hold(120));
    }
    let synced = joined::client_dir(&dir).join("nwsync");
    eprintln!("the client's nwsync folder: {}", synced.display());
    j.stop();
}

/// Test Module's quick start with NWNX, end to end with the real game: a
/// server with the local NWNX build (`MG_NWNX_HOME`, by default
/// `~/.local/opt/nwnx/current`; its core alone, `NWNX_CORE_SKIP_ALL`,
/// through Moonglow's own `resolve_with` and `server_env`), a pregenerated
/// character put alone in the server's vault for the client's CD key
/// (`mg_serve::place_in_vault`, the key's public part read by
/// `mg_serve::cdkey` from the same `cdkey.ini` the client is given) with
/// `-servervault 1`, and the client joining as `join_command` has it
/// (`+connect 127.0.0.1:PORT +password …`). Read: NWNX started in the
/// server and its build is the server's; the client connected; and the
/// character list it is shown (`joined_nwnx_list.png`, looked at).
/// `MG_NWNX_CLICK`: Play clicked on the list's first character (the
/// vault's one), and a picture of what follows (`joined_nwnx_in_game.png`).
/// Seen 2026-11-27 (89.8193.37-17, NWNX 8193.37-17 [2d8465a]): NWNX
/// started, its build the server's; "MgTester (<key>) Joined as Player 1"
/// with the vault folder's key, 6 s after the module loaded; and the
/// client's list held the one character, "Gar Mamm (Barbarian) (Server
/// Vault)", selected, with Play.
#[test]
#[ignore]
fn joined_nwnx_quick_start() {
    let Some(key_file) = joined::cd_key() else {
        eprintln!("skipped: the sandboxed client has no CD key (MG_CLIENT_CDKEY)");
        return;
    };
    let home = std::env::var_os("MG_NWNX_HOME").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/opt/nwnx/current")
    });
    if !home.join("Binaries/NWNX_Core.so").is_file() {
        eprintln!("skipped: no NWNX in {}", home.display());
        return;
    }
    // (TDoD's builds write its plugin into this NWNX's Binaries holding
    // the lock exclusive: held shared meanwhile, as its servers do.)
    let lock = std::fs::File::open(home.join("tdod-plugin.lock")).ok();
    if let Some(l) = &lock {
        l.lock_shared().unwrap();
    }
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let mut m = mg_module::Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let dir = scratch_with("nwnx", &mut m);
    let server = joined::server_dir(&dir);
    std::fs::write(joined::client_dir(&dir).join("settings.tml"), joined_settings()).unwrap();
    // NWNX as Test Module resolves it for a home given.
    let o = mg_module::nwnx::Options {
        choice: mg_module::nwnx::Choice::Home(home.clone()),
        linux: true,
        ..Default::default()
    };
    let more = [("NWNX_CORE_SKIP_ALL".to_string(), "y".to_string())];
    let r = mg_module::nwnx::resolve_with(&Default::default(), &o, &more);
    let nwnx = r.nwnx.unwrap_or_else(|| panic!("{:?}", r.notes));
    // The quick start's vault: the character alone in the client's key's
    // folder. (The key never in a message.)
    let public = mg_serve::cdkey::public_key_of_ini(&key_file).expect("the CD key's public part");
    let bic = root.join("data/lcv/barbarian.bic");
    let placed = mg_serve::place_in_vault(&server, &public, &bic).unwrap();
    assert!(placed.is_file());
    let j = joined::start_with_server_env(
        &dir,
        "MgJoined",
        Join::Player,
        None,
        &["-servervault", "1"],
        &nwnx.server_env(),
    )
    .unwrap();
    // (The server logs a player as it connects, by the public part of its
    // key: "Name (PUBLIC) Joined as Player 1", at the character list.)
    let connected = j.wait_for("Joined as Player", Duration::from_secs(150));
    std::thread::sleep(Duration::from_secs(hold(20)));
    let list = j.winshot("joined_nwnx_list");
    let out = j.server_out();
    let mut builds = mg_module::nwnx::Builds::default();
    let said: Vec<_> = out.lines().filter_map(|l| builds.line(l)).collect();
    let refused: Vec<_> =
        out.lines().filter_map(|l| mg_module::nwnx::console_note(l, true)).collect();
    if std::env::var_os("MG_NWNX_CLICK").is_some() {
        super::nui::click_in_window(615, 615);
        std::thread::sleep(Duration::from_secs(15));
        let _ = j.winshot("joined_nwnx_in_game");
    }
    let log = j.server.log();
    let client_out = std::fs::read_to_string(dir.join("client.out")).unwrap_or_default();
    j.stop();
    drop(lock);
    // (The key's public part kept out of what is printed too.)
    let joined_with_key = log.contains(&format!("({public}) Joined as Player"));
    let log = log.replace(&public, "<public key>");
    eprintln!("server log:\n{log}");
    eprintln!("NWNX: {said:?}; its errors: {refused:?}");
    eprintln!("client: {}", client_out.lines().rev().take(5).collect::<Vec<_>>().join(" | "));
    eprintln!("picture: {list:?}");
    assert!(out.contains("Starting NWNX"), "NWNX did not start in the server:\n{out}");
    assert!(
        said.iter().any(|n| n.level == mg_module::nwnx::Level::Info),
        "NWNX's build is the server's: {said:?}"
    );
    assert!(refused.iter().all(|n| n.level != mg_module::nwnx::Level::Error), "{refused:?}");
    assert!(connected, "the client never connected:\n{log}");
    assert!(joined_with_key, "joined with another key than the vault's folder's:\n{log}");
}
