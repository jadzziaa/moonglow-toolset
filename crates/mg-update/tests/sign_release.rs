//! `packaging/sign-release.sh`, which the owner runs to sign a release, run
//! here with `gh` and `minisign` stood in for by scripts that write down
//! what they are asked (no GitHub, no key): the `SHA256SUMS` it writes is
//! what the app reads, and its signature carries the trusted comment the
//! app checks.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let root = match std::env::var_os("MOONGLOW_TEST_OUTPUT") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-output"),
    };
    let dir = root.join("mg-update").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn script(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn the_signing_script_writes_the_sums_the_app_reads() {
    let dir = scratch("sign-release");
    let bin = dir.join("bin");
    let uploaded = dir.join("uploaded");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&uploaded).unwrap();
    // gh: the release's packages "downloaded", the uploads kept.
    script(
        &bin.join("gh"),
        r#"#!/usr/bin/env bash
printf '%s\n' "gh $*" >> "$LOG"
if [[ $1 == release && $2 == download ]]; then
    while [[ $# -gt 0 ]]; do [[ $1 == --dir ]] && dir=$2; shift; done
    printf 'image' > "$dir/Moonglow-9.9.9-x86_64.AppImage"
    printf 'disk image' > "$dir/Moonglow-9.9.9-macos.dmg"
elif [[ $1 == release && $2 == upload ]]; then
    cp SHA256SUMS SHA256SUMS.minisig "$UPLOADED/"
fi
"#,
    );
    // minisign: its arguments one a line, and a signature "written".
    script(
        &bin.join("minisign"),
        r#"#!/usr/bin/env bash
printf 'minisign' >> "$LOG"; printf ' [%s]' "$@" >> "$LOG"; printf '\n' >> "$LOG"
if [[ $1 == -S ]]; then printf 'signature' > SHA256SUMS.minisig; fi
"#,
    );
    let key = dir.join("moonglow.key");
    let public = dir.join("minisign.pub");
    std::fs::write(&key, "a secret key").unwrap();
    std::fs::write(&public, "a public key").unwrap();
    let log = dir.join("log");
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    let out = std::process::Command::new("bash")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/sign-release.sh"))
        .arg("v9.9.9")
        .env("PATH", path)
        .env("LOG", &log)
        .env("UPLOADED", &uploaded)
        .env("MOONGLOW_MINISIGN_KEY", &key)
        .env("MOONGLOW_MINISIGN_PUB", &public)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let sums = std::fs::read_to_string(uploaded.join("SHA256SUMS")).unwrap();
    let lines = mg_update::verify::parse_sums(&sums);
    let hash = |b: &[u8]| {
        let mut h = mg_update::verify::Hasher::default();
        h.update(b);
        h.finish()
    };
    assert_eq!(
        lines,
        [
            ("Moonglow-9.9.9-macos.dmg".to_string(), hash(b"disk image")),
            ("Moonglow-9.9.9-x86_64.AppImage".to_string(), hash(b"image")),
        ]
    );
    assert!(uploaded.join("SHA256SUMS.minisig").exists());
    let log = std::fs::read_to_string(&log).unwrap();
    let comment = mg_update::verify::trusted_comment("v9.9.9");
    let expected = [
        "gh release download v9.9.9 --repo jadzziaa/moonglow-toolset --dir".to_string(),
        format!("minisign [-S] [-s] [{}] [-m] [SHA256SUMS] [-t] [{comment}]", key.display()),
        format!("minisign [-V] [-p] [{}] [-m] [SHA256SUMS]", public.display()),
        "gh release upload v9.9.9 --repo jadzziaa/moonglow-toolset --clobber SHA256SUMS \
         SHA256SUMS.minisig"
            .to_string(),
    ];
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), expected.len(), "{log}");
    for (line, want) in lines.iter().zip(&expected) {
        assert!(line.starts_with(want.as_str()), "{line:?} begins {want:?}");
    }

    // Without a public key (not made yet), nothing is downloaded or signed.
    std::fs::remove_file(dir.join("log")).unwrap();
    let out = std::process::Command::new("bash")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/sign-release.sh"))
        .arg("v9.9.9")
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("LOG", dir.join("log"))
        .env("MOONGLOW_MINISIGN_KEY", &key)
        .env("MOONGLOW_MINISIGN_PUB", dir.join("none.pub"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("make the key first"));
    assert!(!dir.join("log").exists(), "nothing asked of gh or minisign");
}
