//! `mg watch`: a module kept as files is followed as it is edited; changed
//! scripts go compiled into a development folder, with changed
//! blueprints, and other changes build the module again.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::Value;

fn mg(root: &Path, args: &[&str]) -> Command {
    let mut c = crate::mg();
    c.args(["--root", root.to_str().unwrap(), "--no-user-dir", "--json"]).args(args);
    c
}

/// The next event of `kind`, within a while.
fn next(events: &Receiver<Value>, kind: &str) -> Value {
    loop {
        let e = events
            .recv_timeout(Duration::from_secs(60))
            .unwrap_or_else(|_| panic!("no {kind} event"));
        if e["event"] == kind {
            return e;
        }
    }
}

#[test]
fn changes_go_to_the_development_folder_or_build_the_module() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-watch");
    let (project, dev, built) = (dir.join("chess"), dir.join("development"), dir.join("out.mod"));
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let unpack = [chess.to_str().unwrap(), project.to_str().unwrap()];
    assert!(mg(&root, &[&["unpack"][..], &unpack[..]].concat()).status().unwrap().success());
    // Something of someone else's in the folder, which is not ours to
    // take away.
    std::fs::create_dir_all(&dev).unwrap();
    std::fs::write(dev.join("theirs.2da"), "2DA V2.0\n").unwrap();

    let p = project.to_str().unwrap();
    let mut child = mg(
        &root,
        &[
            "watch",
            p,
            "--dev-dir",
            dev.to_str().unwrap(),
            "--mod",
            built.to_str().unwrap(),
            "--ndb",
        ],
    )
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();
    let out = child.stdout.take().unwrap();
    let (send, events) = channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                let _ = send.send(v);
            }
        }
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let first = next(&events, "watching");
        assert!(first["compiled"].as_array().unwrap().len() > 10, "{first}");
        assert!(built.is_file(), "the module built at the start");
        assert_eq!(std::fs::read_dir(&dev).unwrap().count(), 1, "nothing put out yet");

        // A script changes: it goes out compiled.
        let m = mg_module::Module::open(&project).unwrap();
        let script = (m.keys_of(mg_core::ResType::NSS))
            .find(|k| m.get(k).is_some_and(mg_script::outline::has_entry_point))
            .unwrap()
            .resref
            .to_string();
        let file = project.join(format!("{script}.nss"));
        let mut source = std::fs::read(&file).unwrap();
        source.extend_from_slice(b"\n// changed\n");
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(&file, source).unwrap();
        let synced = next(&events, "synced");
        let written: Vec<&str> =
            synced["written"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
        assert!(written.contains(&format!("{script}.ncs").as_str()), "{synced}");
        assert!(dev.join(format!("{script}.ncs")).is_file());
        // (With --ndb: its debug file beside it.)
        assert!(written.contains(&format!("{script}.ndb").as_str()), "{synced}");
        let ndb = std::fs::read(dev.join(format!("{script}.ndb"))).unwrap();
        assert!(ndb.starts_with(b"NDB V1.0"));

        // A script that does not compile: said, and nothing put out.
        let before = std::fs::read(dev.join(format!("{script}.ncs"))).unwrap();
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(&file, "void main() { nothing here }\n").unwrap();
        let broken = next(&events, "synced");
        assert_eq!(broken["errors"].as_array().unwrap().len(), 1, "{broken}");
        assert!(broken["written"].as_array().unwrap().is_empty());
        assert_eq!(std::fs::read(dev.join(format!("{script}.ncs"))).unwrap(), before);
        std::fs::write(&file, "void main() { }\n").unwrap();
        next(&events, "synced");

        // A blueprint changes: it goes out as it is.
        std::thread::sleep(Duration::from_millis(1100));
        let set = mg(&root, &["set", p, "pawn_w.utc", "Tag=WATCHED"]).output().unwrap();
        assert!(set.status.success(), "{}", String::from_utf8_lossy(&set.stdout));
        let synced = next(&events, "synced");
        assert_eq!(synced["written"], serde_json::json!(["pawn_w.utc"]), "{synced}");

        // The module's own file changes: built again, and what was put
        // out is taken away (the module has it); theirs stays.
        let was = std::fs::metadata(&built).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(1100));
        let set = mg(&root, &["set", p, "module.ifo", "Mod_Tag=WATCHED"]).output().unwrap();
        assert!(set.status.success(), "{}", String::from_utf8_lossy(&set.stdout));
        let rebuilt = next(&events, "rebuilt");
        assert_eq!(rebuilt["changed"], serde_json::json!(["module.ifo"]), "{rebuilt}");
        assert!(std::fs::metadata(&built).unwrap().modified().unwrap() > was);
        let left: Vec<String> = std::fs::read_dir(&dev)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, ["theirs.2da"]);
        let m = mg_module::Module::open(&built).unwrap();
        let key = mg_resman::ResKey::from_filename(&format!("{script}.ncs")).unwrap();
        assert!(m.contains(&key), "the built module has the scripts compiled");
    }));
    let _ = child.kill();
    let _ = child.wait();
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// The server's options are for --serve: without it, refused before
/// anything is read or written.
#[test]
fn the_server_s_options_want_serve() {
    let dir = mg_testkit::scratch_dir("mg-watch-serve-options");
    let (project, dev) = (dir.join("none"), dir.join("development"));
    let (p, d) = (project.to_str().unwrap(), dev.to_str().unwrap());
    for given in [
        &["--dm-password", "x"][..],
        &["--player-password", "x"],
        &["--admin-password", "x"],
        &["--dir", d],
        &["--port", "5121"],
        &["--env", d],
        &["--preload", d],
        &["--", "-maxclients", "4"],
    ] {
        let out = crate::mg().args(["watch", p, "--dev-dir", d]).args(given).output().unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{given:?}: {said}");
        assert!(said.contains("--serve"), "{given:?}: {said}");
    }
    assert!(!dev.exists(), "nothing made");
}

#[test]
fn a_project_is_served_and_started_again_when_it_is_built_again() {
    use std::io::Write;
    let root = mg_testkit::corpus!();
    if mg_serve::server_binary(&root).is_none() {
        return;
    }
    let _alone = mg_testkit::engine::timed_turn();
    let dir = mg_testkit::scratch_dir("mg-watch-serve");
    let (project, served) = (dir.join("chess"), dir.join("served"));
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let unpack = [chess.to_str().unwrap(), project.to_str().unwrap()];
    assert!(mg(&root, &[&["unpack"][..], &unpack[..]].concat()).status().unwrap().success());
    let p = project.to_str().unwrap();
    let dm = "dm-of-the-watch";
    let s = served.to_str().unwrap();
    let mut child = mg(&root, &["watch", p, "--serve", "--dir", s, "--dm-password", dm])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let out = child.stdout.take().unwrap();
    let mut typed = child.stdin.take().unwrap();
    let (send, events) = channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                let _ = send.send(v);
            }
        }
    });
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        next(&events, "watching");
        let started = next(&events, "started");
        // The DM password given, the others random ones: printed, and
        // what the server was started with.
        let passwords = next(&events, "passwords");
        assert_eq!(passwords["dm"], dm, "{passwords}");
        let (player, admin) = (passwords["player"].as_str().unwrap(), &passwords["admin"]);
        assert!(player.len() > 8 && player != dm, "{passwords}");
        assert!(admin.as_str().unwrap().len() > 8, "{passwords}");
        let carried = |pid: &Value| {
            let Ok(args) = std::fs::read(format!("/proc/{pid}/cmdline")) else { return };
            let args: Vec<&[u8]> = args.split(|b| *b == 0).collect();
            let after = |flag: &str| {
                let at = args.iter().position(|a| *a == flag.as_bytes()).unwrap();
                String::from_utf8_lossy(args[at + 1]).into_owned()
            };
            assert_eq!(after("-dmpassword"), dm);
            assert_eq!(after("-playerpassword"), player);
        };
        carried(&started["pid"]);
        next(&events, "ready");
        // The server's own development folder is where changes go.
        let dev = served.join("development");
        let m = mg_module::Module::open(&project).unwrap();
        let script = (m.keys_of(mg_core::ResType::NSS))
            .find(|k| m.get(k).is_some_and(mg_script::outline::has_entry_point))
            .unwrap()
            .resref
            .to_string();
        let file = project.join(format!("{script}.nss"));
        let mut source = std::fs::read(&file).unwrap();
        source.extend_from_slice(b"\n// changed\n");
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(&file, source).unwrap();
        let synced = next(&events, "synced");
        assert!(dev.join(format!("{script}.ncs")).is_file(), "{synced}");

        // The module's own file changes: built again, and the server
        // started again on it, at its port.
        std::thread::sleep(Duration::from_millis(1100));
        let set = mg(&root, &["set", p, "module.ifo", "Mod_Tag=WATCHED"]).output().unwrap();
        assert!(set.status.success(), "{}", String::from_utf8_lossy(&set.stdout));
        next(&events, "rebuilt");
        let again = next(&events, "started");
        assert_eq!(again["port"], started["port"]);
        assert_ne!(again["pid"], started["pid"]);
        // (And its passwords: a DM keeps the one they had.)
        assert_eq!(next(&events, "passwords"), passwords);
        carried(&again["pid"]);
        next(&events, "ready");
        assert!(!dev.join(format!("{script}.ncs")).exists(), "the module has it now");

        // Told to stop, it stops the server and ends well.
        typed.write_all(b"/quit\n").unwrap();
        typed.flush().unwrap();
        let ended = next(&events, "ended");
        assert_eq!((&ended["how"], &ended["failed"]), (&"stopped".into(), &false.into()));
    }));
    if result.is_ok() {
        assert!(child.wait().unwrap().success());
    } else {
        let _ = child.kill();
        let _ = child.wait();
    }
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}
