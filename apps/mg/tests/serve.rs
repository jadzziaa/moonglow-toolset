//! `mg serve`: a module in the game's headless server, privately and on a
//! user directory that is not the player's.

use std::path::Path;

use serde_json::Value;

fn serve(root: &Path, user: &Path, args: &[&str]) -> (bool, Vec<Value>, String) {
    let out =
        crate::mg_on(root, Some(user)).arg("--json").arg("serve").args(args).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let events = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    (out.status.success(), events, text)
}

#[test]
fn a_module_is_served_until_a_line_of_its_log() {
    let root = mg_testkit::corpus!();
    if mg_serve::server_binary(&root).is_none() {
        return;
    }
    let _alone = mg_testkit::engine::timed_turn();
    let dir = mg_testkit::scratch_dir("mg-serve");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let chess = dir.join("Chess Game.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let served = dir.join("served");
    let (c, s) = (chess.to_str().unwrap(), served.to_str().unwrap());

    let (ok, events, text) = serve(
        &root,
        &user,
        &[c, "--dir", s, "--until", "Loading Module", "--marker", "Loading", "--timeout", "90"],
    );
    assert!(ok, "{text}");
    let of = |kind: &str| events.iter().filter(|e| e["event"] == kind).collect::<Vec<_>>();
    let started = of("started")[0];
    assert!(started["port"].as_u64().unwrap() > 0);
    assert!(started["log"].as_str().unwrap().ends_with("nwserverLog1.txt"));
    assert_eq!(started["user_dir"], s);
    assert!(of("passwords")[0]["player"].as_str().unwrap().len() > 8);
    assert_eq!(of("marker")[0]["marker"], "Loading");
    assert!(of("marker")[0]["line"].as_str().unwrap().contains("Loading Module: chess_game"));
    assert_eq!(events.last().unwrap()["failed"], false);
    // The module as an archive of the server's own; the player's folder
    // was given nothing.
    assert!(served.join("modules/chess_game.mod").is_file());
    assert_eq!(std::fs::read_dir(&user).unwrap().count(), 0);

    // A line that never comes: a failure at the timeout.
    let (ok, events, _) =
        serve(&root, &user, &[c, "--dir", s, "--until", "never said", "--timeout", "4"]);
    assert!(!ok && events.last().unwrap()["failed"] == true, "{events:?}");
    // (Meanwhile the server said its module was loaded.)
    assert_eq!(events.iter().filter(|e| e["event"] == "ready").count(), 1, "{events:?}");

    // A folder named from here (the server runs in its own).
    let here = std::env::current_dir().unwrap();
    let from_here = pathdiff(&served, &here);
    let (ok, _, text) = serve(
        &root,
        &user,
        &[c, "--dir", &from_here, "--until", "Loading Module", "--timeout", "90"],
    );
    assert!(ok, "{from_here}: {text}");

    // Not on the player's own folder, unless told.
    let (ok, _, text) = serve(&root, &user, &[c, "--dir", user.to_str().unwrap()]);
    assert!(!ok && text.contains("--real-user-dir"), "{text}");
    assert_eq!(std::fs::read_dir(&user).unwrap().count(), 0);
}

/// A module kept as files (a folder, a nasher project) is served with its
/// scripts compiled: its load script, there as source only, runs.
#[test]
fn a_module_of_sources_is_served_compiled() {
    use mg_core::ResType;
    use mg_resman::ResKey;
    use mg_schema::StructExt;
    let root = mg_testkit::corpus!();
    if mg_serve::server_binary(&root).is_none() {
        return;
    }
    let _alone = mg_testkit::engine::timed_turn();
    let dir = mg_testkit::scratch_dir("mg-serve-sources");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let mut m = mg_module::Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    m.set(
        ResKey::parse("mg_hello", ResType::NSS).unwrap(),
        b"void main() { WriteTimestampedLogEntry(\"MG_SERVED_COMPILED\"); }\n".to_vec(),
    );
    m.set(ResKey::parse("mg_broken", ResType::NSS).unwrap(), b"void main() { no }\n".to_vec());
    let mut info = m.info().unwrap();
    let load: mg_core::ResRef = "mg_hello".parse().unwrap();
    info.root.write(&mg_schema::ifo::MOD_ON_MOD_LOAD, load);
    m.set_info(&info).unwrap();
    let folder = dir.join("sources");
    m.save_as(&mg_module::ModuleLocation::Folder(folder.clone())).unwrap();
    assert!(!folder.join("mg_hello.ncs").exists());
    let served = dir.join("served");
    let (f, s) = (folder.to_str().unwrap(), served.to_str().unwrap());
    let (ok, events, text) =
        serve(&root, &user, &[f, "--dir", s, "--until", "MG_SERVED_COMPILED", "--timeout", "90"]);
    assert!(ok, "{text}");
    // What was compiled is said, and what did not compile; the sources
    // are as they were.
    let notes: Vec<&str> =
        events.iter().filter(|e| e["event"] == "note").filter_map(|e| e["note"].as_str()).collect();
    assert!(notes.iter().any(|n| n.starts_with("compiled ")), "{events:?}");
    assert!(notes.iter().any(|n| n.contains("mg_broken.nss did not compile")), "{events:?}");
    assert!(!folder.join("mg_hello.ncs").exists());

    // Compiled in the folder, then changed: served as it is now, not as
    // its older compiled script.
    let _ = crate::mg_on(&root, Some(user.as_ref())).args(["compile", f]).output().unwrap();
    assert!(folder.join("mg_hello.ncs").is_file());
    let changed = "void main() { WriteTimestampedLogEntry(\"MG_SERVED_CHANGED\"); }\n";
    std::fs::write(folder.join("mg_hello.nss"), changed).unwrap();
    let (ok, _, text) =
        serve(&root, &user, &[f, "--dir", s, "--until", "MG_SERVED_CHANGED", "--timeout", "90"]);
    assert!(ok, "{text}");
}

/// Ctrl+C stops the server as `exit` on its console does (it closes its
/// database and logs in order), not by its death with `mg`: the run ends
/// well, and the server has gone. (Ctrl+C in a terminal goes to every
/// process of its group, as here.)
#[cfg(unix)]
#[test]
fn ctrl_c_stops_the_server_with_exit() {
    use std::io::{BufRead, BufReader};
    use std::os::unix::process::CommandExt;
    let root = mg_testkit::corpus!();
    if mg_serve::server_binary(&root).is_none() {
        return;
    }
    let _alone = mg_testkit::engine::timed_turn();
    let dir = mg_testkit::scratch_dir("mg-serve-ctrl-c");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let served = dir.join("served");
    let mut child = crate::mg_on(&root, Some(user.as_ref()))
        .arg("--json")
        .args(["serve", chess.to_str().unwrap(), "--dir", served.to_str().unwrap()])
        .args(["--timeout", "90"])
        .process_group(0)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let lines = BufReader::new(child.stdout.take().unwrap()).lines().map_while(Result::ok);
    let mut events = lines.filter_map(|l| serde_json::from_str::<Value>(&l).ok());
    let mut seen = Vec::new();
    let mut until = |kind: &str, seen: &mut Vec<Value>| loop {
        let e = events.next().unwrap_or_else(|| panic!("no {kind}: {seen:?}"));
        seen.push(e.clone());
        if e["event"] == kind {
            break e;
        }
    };
    let server = until("started", &mut seen)["pid"].as_u64().unwrap().to_string();
    until("ready", &mut seen);
    let sent = std::process::Command::new("kill")
        .args(["-INT", "--", &format!("-{}", child.id())])
        .status()
        .unwrap();
    assert!(sent.success());
    let ended = until("ended", &mut seen);
    assert_eq!((&ended["how"], &ended["failed"]), (&"stopped".into(), &false.into()), "{seen:?}");
    let said = |what: &str| seen.iter().any(|e| e.to_string().contains(what));
    assert!(said("Ctrl+C: stopping the server"), "{seen:?}");
    assert!(child.wait().unwrap().success());
    // (Gone: not even a process to signal.)
    let alive = std::process::Command::new("kill")
        .args(["-0", &server])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(!alive.success(), "the server {server} is still running");
    let exiting = seen.iter().any(|e| e["event"] == "console" && e["line"] == "Server: Exiting...");
    assert!(exiting, "the server was not told to exit: {seen:?}");
}

/// `path` as seen from `from` (both absolute).
fn pathdiff(path: &Path, from: &Path) -> String {
    let (p, f): (Vec<_>, Vec<_>) = (path.components().collect(), from.components().collect());
    let shared = p.iter().zip(&f).take_while(|(a, b)| a == b).count();
    let mut out = std::path::PathBuf::new();
    for _ in shared..f.len() {
        out.push("..");
    }
    for part in &p[shared..] {
        out.push(part);
    }
    out.to_string_lossy().into_owned()
}
