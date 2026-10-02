//! Test support shared by Moonglow's crates.
//!
//! Corpus tests read the user's installed game (never shipped with Moonglow).
//! [`nwn_root`] finds it through `NWN_ROOT` or the usual install locations;
//! tests call [`corpus!`] and skip, with a message, when there is none.
//! Set `MOONGLOW_REQUIRE_CORPUS=1` (as CI runners with the game do) to turn a
//! missing corpus into a failure instead.

pub mod engine;
pub mod erf;
pub mod gpu;

use std::env;
use std::path::{Path, PathBuf};

/// The game install to test against, if one can be found.
///
/// `NWN_ROOT` wins; otherwise the usual Steam locations for the platform are
/// tried. A directory counts as an install if it has `data/nwn_base.key`.
pub fn nwn_root() -> Option<PathBuf> {
    if let Some(root) = env::var_os("NWN_ROOT") {
        let root = PathBuf::from(root);
        return is_install(&root).then_some(root);
    }
    candidate_roots().into_iter().find(|p| is_install(p))
}

fn is_install(p: &Path) -> bool {
    p.join("data").join("nwn_base.key").is_file()
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    let home = env::var_os("HOME").map(PathBuf::from);
    let game = Path::new("steamapps").join("common").join("Neverwinter Nights");
    if cfg!(target_os = "linux") {
        if let Some(h) = &home {
            v.push(h.join(".local/share/Steam").join(&game));
            v.push(h.join(".steam/steam").join(&game));
            v.push(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam").join(&game));
        }
    } else if cfg!(target_os = "macos") {
        if let Some(h) = &home {
            v.push(h.join("Library/Application Support/Steam").join(&game));
        }
    } else if cfg!(windows) {
        for pf in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = env::var_os(pf) {
                v.push(PathBuf::from(p).join("Steam").join(&game));
            }
        }
    }
    v
}

/// Whether a missing corpus should fail tests instead of skipping them.
pub fn corpus_required() -> bool {
    env::var_os("MOONGLOW_REQUIRE_CORPUS").is_some_and(|v| v != "0")
}

/// Returns the game install, or skips the calling test (with a message) if
/// there is none. Panics instead when `MOONGLOW_REQUIRE_CORPUS` is set.
#[macro_export]
macro_rules! corpus {
    () => {
        match $crate::nwn_root() {
            Some(root) => root,
            None if $crate::corpus_required() => {
                panic!(
                    "MOONGLOW_REQUIRE_CORPUS is set but no game install was found (set NWN_ROOT)"
                )
            }
            None => {
                eprintln!("skipped: no game install found (set NWN_ROOT to run corpus tests)");
                return;
            }
        }
    };
}

/// The modules that ship with the game (`data/mod/*.mod`, `data/nwm/*.nwm`),
/// sorted by path.
pub fn bundled_modules(root: &Path) -> Vec<PathBuf> {
    let mut v = files_with_ext(&root.join("data/mod"), "mod");
    v.extend(files_with_ext(&root.join("data/nwm"), "nwm"));
    v.sort();
    v
}

/// The haks that ship with the game (`data/hk/*.hak`), sorted by path.
pub fn bundled_haks(root: &Path) -> Vec<PathBuf> {
    files_with_ext(&root.join("data/hk"), "hak")
}

/// Files in `dir` (not recursive) with extension `ext` (any case), sorted.
pub fn files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .collect();
    v.sort();
    v
}

/// A fresh, empty scratch directory for a test under `target/test-output/`.
pub fn scratch_dir(name: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-output");
    let dir = base.join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// A command-line tool to use as a test oracle: a neverwinter.nim tool (e.g.
/// `"nwn_gff"`) or nwnmdlcomp (as built by Neverblender's tools): from
/// `$NWN_TOOLS_BIN`, then `~/.local/opt/neverwinter/bin`, then
/// `~/Projects/neverblender/tools/bin`, then `PATH`.
pub fn nwn_tool(name: &str) -> Option<PathBuf> {
    let exe = format!("{name}{}", env::consts::EXE_SUFFIX);
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = env::var_os("NWN_TOOLS_BIN") {
        dirs.push(d.into());
    }
    if let Some(h) = env::var_os("HOME") {
        dirs.push(PathBuf::from(&h).join(".local/opt/neverwinter/bin"));
        dirs.push(PathBuf::from(h).join("Projects/neverblender/tools/bin"));
    }
    if let Some(path) = env::var_os("PATH") {
        dirs.extend(env::split_paths(&path));
    }
    dirs.into_iter().map(|d| d.join(&exe)).find(|p| p.is_file())
}

/// Returns an oracle tool's path, or skips the calling test if it is missing.
#[macro_export]
macro_rules! oracle_tool {
    ($name:expr) => {
        match $crate::nwn_tool($name) {
            Some(p) => p,
            None => {
                eprintln!("skipped: {} not found (set NWN_TOOLS_BIN)", $name);
                return;
            }
        }
    };
}

/// A file captured from Aurora under Wine (`tools/aurora/`), kept in the
/// oracle's state directory (`$MOONGLOW_ORACLE`, default
/// `~/.local/share/moonglow-oracle`) under `captures/`.
pub fn aurora_capture(name: &str) -> Option<PathBuf> {
    let base = env::var_os("MOONGLOW_ORACLE").map(PathBuf::from).or_else(|| {
        env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/moonglow-oracle"))
    })?;
    Some(base.join("captures").join(name)).filter(|p| p.is_file())
}

/// An Aurora capture ([`aurora_capture`]), or skip the test (fail with
/// `MOONGLOW_REQUIRE_CORPUS=1`).
#[macro_export]
macro_rules! aurora_capture {
    ($name:expr) => {
        match $crate::aurora_capture($name) {
            Some(p) => p,
            None if $crate::corpus_required() => panic!("Aurora capture {} not found", $name),
            None => {
                eprintln!("skipped: Aurora capture {} not found (tools/aurora/)", $name);
                return;
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_macro_skips_or_returns_an_install() {
        let root = corpus!();
        assert!(is_install(&root));
        assert!(!bundled_modules(&root).is_empty());
    }

    #[test]
    fn scratch_dirs_are_empty() {
        let d = scratch_dir("mg-testkit-self");
        std::fs::write(d.join("x"), b"1").unwrap();
        let d = scratch_dir("mg-testkit-self");
        assert_eq!(std::fs::read_dir(d).unwrap().count(), 0);
    }
}
