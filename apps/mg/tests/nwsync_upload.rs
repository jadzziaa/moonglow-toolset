//! `mg nwsync-prune --upload` with rsync to a folder of this machine (as a
//! web server's folder would be, over SSH): what Remove Unused Data and an
//! upload whose command takes away what the repository no longer has
//! leave the server. Nothing goes anywhere but the test's own folder.
//! (Where rsync isn't installed, there is nothing to run; `json.rs` runs
//! `mg nwsync --upload` with a stand-in command on every system.)

#![cfg(unix)]

use mg_module::nwsync::{Options, Source, shell_quoted, write};

fn data_file(root: &std::path::Path, bytes: &[u8]) -> std::path::PathBuf {
    let h = mg_core::sha1::hex(&mg_core::sha1::sha1(bytes));
    root.join("data/sha1").join(&h[..2]).join(&h[2..4]).join(&h)
}

#[test]
fn a_pruned_repository_is_mirrored_by_rsync() {
    if std::process::Command::new("rsync").arg("--version").output().is_err() {
        eprintln!("skipped: no rsync");
        return;
    }
    let dir = mg_testkit::scratch_dir("mg-nwsync-rsync");
    let (repo, served) = (dir.join("repo"), dir.join("served dir"));
    std::fs::create_dir_all(&served).unwrap();
    let key = |n: &str| mg_resman::ResKey::parse(n, mg_core::ResType::TXT).unwrap();
    let bytes = |b: &'static [u8]| Source::Bytes(b.into());
    let o = Options { latest: true, ..Default::default() };
    let old =
        write(&repo, &[(key("a"), bytes(b"old")), (key("b"), bytes(b"kept"))], &o, &mut |_, _| {})
            .unwrap();
    let new = write(&repo, &[(key("b"), bytes(b"kept"))], &o, &mut |_, _| {}).unwrap();
    let r = repo.to_str().unwrap();
    let command = format!("rsync -a --delete {{part}} {}/", shell_quoted(&served));
    let (ok, j) = crate::mg_json(&["nwsync-prune", r, "--upload", &command]);
    assert!(ok, "{j}");
    assert_eq!(j["uploaded"], serde_json::json!(["manifests", "data"]));
    for sha in [&old.sha1, &new.sha1] {
        assert!(served.join("manifests").join(sha).is_file());
    }
    assert!(data_file(&served, b"old").is_file() && data_file(&served, b"kept").is_file());
    assert!(!served.join("latest").exists(), "a prune leaves latest as it was: not sent");

    // The old manifest taken away and its data pruned: the server's go
    // too, the manifest before its data.
    std::fs::remove_file(repo.join("manifests").join(&old.sha1)).unwrap();
    let month_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 86_400);
    let file = std::fs::File::options().write(true).open(data_file(&repo, b"old")).unwrap();
    file.set_modified(month_ago).unwrap();
    drop(file);
    let (ok, j) = crate::mg_json(&["nwsync-prune", r, "--upload", &command]);
    assert!(ok, "{j}");
    assert_eq!(j["removed"].as_array().map(Vec::len), Some(1));
    assert!(!served.join("manifests").join(&old.sha1).exists());
    assert!(!served.join("manifests").join(format!("{}.json", old.sha1)).exists());
    assert!(!data_file(&served, b"old").exists());
    assert!(
        data_file(&served, b"kept").is_file() && served.join("manifests").join(&new.sha1).is_file()
    );

    // A dry run sends nothing.
    let (ok, j) = crate::mg_json(&["nwsync-prune", r, "--dry-run", "--upload", "exit 3"]);
    assert!(ok && j["uploaded"] == serde_json::json!([]), "{j}");
}
