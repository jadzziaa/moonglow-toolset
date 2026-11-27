//! The quiet lock across its takers: `mg_testkit::quiet` (Rust),
//! `tools/locks.sh` and the shell examples `docs/locks.md` gives an outside
//! program take the same files the same way (`docs/locks.md`).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use crate::quiet::{hold_in, share_in};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch state folder (`XDG_STATE_HOME`) and oracle folder.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mg-quiet-sh-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn locks(state: &Path) -> PathBuf {
    state.join("moonglow/locks")
}

/// `tools/locks.sh ARGS -- true`, waiting `wait` seconds.
fn sh(state: &Path, args: &[&str], wait: &str, held: Option<&str>) -> Output {
    let mut c = Command::new(root().join("tools/locks.sh"));
    c.args(args)
        .args(["--", "true"])
        .env("XDG_STATE_HOME", state)
        .env("MOONGLOW_ORACLE", state.join("oracle"))
        .env("MOONGLOW_LOCK_WAIT", wait)
        .env_remove("MOONGLOW_LOCKS_HELD");
    if let Some(h) = held {
        c.env("MOONGLOW_LOCKS_HELD", h);
    }
    c.output().unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

/// The script and the module share one quiet lock: a quiet run in Rust
/// keeps the shell's ordinary runs out (the display's too, which shares
/// it first), and an ordinary run in Rust keeps the shell's quiet run out;
/// under an ancestor's hold the shell takes nothing.
#[test]
fn the_script_and_the_module_share_one_quiet_lock() {
    let state = scratch("one");
    let q = hold_in(&locks(&state), Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
    for args in [&["share", "quiet"][..], &["hold", "display"], &["hold", "quiet"]] {
        let out = sh(&state, args, "0", None);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let want = match args {
            ["hold", "quiet"] => "the machine is still busy after 0 min: nothing was run",
            _ => "the machine is still kept quiet after 0 min: nothing was run",
        };
        assert_eq!(stderr(&out).trim(), want, "{args:?}");
    }
    let out = sh(&state, &["share", "quiet"], "1", None);
    let said = stderr(&out);
    assert!(
        said.starts_with(&format!(
            "waiting for a quiet run to end: held by PID {}, ",
            std::process::id()
        )),
        "{said}"
    );
    assert!(said.contains("; up to 1 s\n"), "{said}");
    assert!(sh(&state, &["share", "quiet"], "0", Some("quiet")).status.success());
    drop(q);
    assert!(sh(&state, &["share", "quiet"], "0", None).status.success());
    assert!(sh(&state, &["hold", "display"], "0", None).status.success());
    let shared =
        share_in(&locks(&state), Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
    let out = sh(&state, &["hold", "quiet"], "1", None);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let said = stderr(&out);
    assert!(
        said.starts_with(&format!(
            "waiting for the machine to be quiet (1 run holds it: PID {}); up to 1 s\n",
            std::process::id()
        )),
        "{said}"
    );
    assert!(said.ends_with("the machine is still busy after 1 s: nothing was run\n"), "{said}");
    // (Ordinary runs share it with each other.)
    assert!(sh(&state, &["share", "quiet"], "0", None).status.success());
    drop(shared);
    let out = sh(&state, &["hold", "quiet"], "0", None);
    assert!(out.status.success(), "{out:?}");
    let line = std::fs::read_to_string(locks(&state).join("quiet.lock")).unwrap();
    assert!(line.starts_with("PID ") && line.contains(", true, in /"), "{line:?}");
}

/// The shell example of `docs/locks.md` named `name` (the ```sh block after
/// `<!-- example: NAME -->`), written to a file in `dir`.
fn example(dir: &Path, name: &str) -> PathBuf {
    let doc = std::fs::read_to_string(root().join("docs/locks.md")).unwrap();
    let marker = format!("<!-- example: {name} -->");
    let after = &doc[doc.find(&marker).unwrap_or_else(|| panic!("no {marker} in docs/locks.md"))..];
    let start = after.find("```sh\n").expect("its sh block") + 6;
    let body = &after[start..];
    let body = &body[..body.find("```").expect("the block's end")];
    let path = dir.join(format!("{name}.sh"));
    std::fs::write(&path, body).unwrap();
    path
}

/// The example run with `bash`, its run being `sleep SECONDS`.
fn run_example(script: &Path, state: &Path, wait: &str, sleep: &str) -> Child {
    Command::new("bash")
        .arg(script)
        .args(["sleep", sleep])
        .env("XDG_STATE_HOME", state)
        .env("MOONGLOW_LOCK_WAIT", wait)
        .env_remove("MOONGLOW_LOCKS_HELD")
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

/// Until `f` holds (a child has taken its lock), or ten seconds.
fn until(what: &str, f: impl Fn() -> bool) {
    let start = Instant::now();
    while !f() {
        assert!(start.elapsed() < Duration::from_secs(10), "never: {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// An outside program honors the quiet lock from the doc alone: its
/// example for a quiet run keeps Moonglow's ordinary runs out and waits for
/// them; its example for an ordinary run keeps a quiet run out and waits
/// for one.
#[test]
fn the_docs_shell_examples_honor_the_quiet_lock() {
    let state = scratch("doc");
    let dir = locks(&state);
    let alone = example(&state, "quiet-alone");
    let shared = example(&state, "quiet-shared");
    // The quiet run: Moonglow's ordinary runs wait for it.
    let mut c = run_example(&alone, &state, "0", "2");
    until("the example holds the machine", || {
        share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).is_err()
    });
    assert!(sh(&state, &["share", "quiet"], "0", None).status.code() == Some(2));
    assert!(c.wait().unwrap().success());
    assert!(share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).is_ok());
    // Refused, with status 2, while an ordinary run holds it.
    let s = share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
    let out = run_example(&alone, &state, "0", "0").wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    drop(s);
    // The ordinary run: a quiet run waits for it, and it for a quiet run.
    let mut c = run_example(&shared, &state, "0", "2");
    until("the example shares the lock", || {
        hold_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).is_err()
    });
    assert!(share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).is_ok());
    assert!(c.wait().unwrap().success());
    let q = hold_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
    let out = run_example(&shared, &state, "0", "0").wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    drop(q);
    let out = run_example(&shared, &state, "0", "0").wait_with_output().unwrap();
    assert!(out.status.success(), "{out:?}");
}
