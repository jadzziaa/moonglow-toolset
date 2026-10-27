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
pub mod places;

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// The game install to test against, if one can be found: where the
/// toolset finds it ([`mg_resman::GameInstall::find_root`]: `NWN_ROOT`,
/// else each Steam library, then GOG's). This kept a shorter list of its
/// own, and tests skipped on an install the toolset found.
pub fn nwn_root() -> Option<PathBuf> {
    mg_resman::GameInstall::find_root()
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

/// Whether the corpus tests sweep the whole install (`MOONGLOW_FULL_CORPUS`
/// set and not `0`: before a release) or a sample of it (every day).
pub fn full_corpus() -> bool {
    env::var_os("MOONGLOW_FULL_CORPUS").is_some_and(|v| v != "0")
}

/// What a sweep covers of `items`: all of them in a full run
/// ([`full_corpus`]), else every `one_in`-th, from the first (the same ones
/// each run, so that a failure stays where it was).
pub fn sweep<T>(items: Vec<T>, one_in: usize) -> Vec<T> {
    if full_corpus() { items } else { items.into_iter().step_by(one_in.max(1)).collect() }
}

/// A count that a full sweep reaches (`full`), or what a sweep of every
/// `one_in`-th item should still reach (somewhat under its share).
pub fn at_least(full: usize, one_in: usize) -> usize {
    if full_corpus() { full } else { full / (one_in + one_in / 4 + 1) }
}

/// A value made once per key in a test binary and shared by the tests that
/// ask for it (a module unpacked, a server run): the first to ask makes it,
/// the others wait for it.
#[derive(Debug)]
pub struct Shared<K, V>(Mutex<BTreeMap<K, Arc<OnceLock<V>>>>);

impl<K: Ord + Clone, V: Clone> Shared<K, V> {
    /// An empty one (for a `static`).
    pub const fn new() -> Self {
        Self(Mutex::new(BTreeMap::new()))
    }

    /// The value for `key`, made by `make` if this is the first call for it.
    pub fn get(&self, key: &K, make: impl FnOnce() -> V) -> V {
        let slot = {
            let mut all = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            all.entry(key.clone()).or_default().clone()
        };
        slot.get_or_init(make).clone()
    }
}

impl<K: Ord + Clone, V: Clone> Default for Shared<K, V> {
    fn default() -> Self {
        Self::new()
    }
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

/// Folders of a builder's haks that the corpus tests take as installed
/// (`MOONGLOW_HAKS`, separated as `PATH` is), beside the user's `hak/`
/// and the game's `data/hk/`: a module's own build folder, say.
pub fn builder_hak_dirs() -> Vec<PathBuf> {
    env::var_os("MOONGLOW_HAKS").map(|v| env::split_paths(&v).collect()).unwrap_or_default()
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

/// The test files of a crate that no test binary holds: the `*.rs` files of
/// `<manifest_dir>/tests` that are neither a `[[test]]` of the crate's
/// `Cargo.toml` nor a module of a group (`#[path = "../<file>.rs"]` in
/// `tests/groups/*.rs`). For crates that turn `autotests` off: a file left
/// out would not be compiled, and its tests would pass by not running.
pub fn ungrouped_tests(manifest_dir: &str) -> Vec<String> {
    let dir = Path::new(manifest_dir);
    let mut named = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    for group in files_with_ext(&dir.join("tests/groups"), "rs") {
        named.push_str(&std::fs::read_to_string(group).unwrap_or_default());
    }
    files_with_ext(&dir.join("tests"), "rs")
        .iter()
        .filter_map(|p| Some(p.file_name()?.to_str()?.to_string()))
        .filter(|file| {
            !named.contains(&format!("\"tests/{file}\""))
                && !named.contains(&format!("\"../{file}\""))
        })
        .collect()
}

/// A fresh, empty scratch directory for a test under `target/test-output/`.
pub fn scratch_dir(name: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-output");
    let dir = base.join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Puts `from` at `to` for a test's scratch user folder (a builder's hak
/// in its `hak`): a symbolic link where the platform makes one without
/// asking (Unix), else a copy.
pub fn link_or_copy(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(from, to)
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(from, to).map(|_| ())
    }
}

/// A directory under `target/test-output/` that is kept as it is: for
/// output that several tests add to (the pictures of `screens.rs`), which
/// [`scratch_dir`] would empty under them.
pub fn output_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-output").join(name);
    std::fs::create_dir_all(&dir).expect("create output dir");
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
        assert!(mg_resman::GameInstall::is_install(&root));
        assert!(!bundled_modules(&root).is_empty());
    }

    #[test]
    fn scratch_dirs_are_empty() {
        let d = scratch_dir("mg-testkit-self");
        std::fs::write(d.join("x"), b"1").unwrap();
        let d = scratch_dir("mg-testkit-self");
        assert_eq!(std::fs::read_dir(d).unwrap().count(), 0);
    }

    /// A scratch folder is emptied by each call: two tests (or two calls of
    /// one) naming the same one empty it under each other as they run side
    /// by side. Pictures several tests add to go in an [`output_dir`].
    #[test]
    fn every_scratch_dir_is_named_once() {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|e| e == "rs") {
                    out.push(p);
                }
            }
        }
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (mut files, mut named) = (Vec::new(), BTreeMap::<String, Vec<String>>::new());
        walk(&repo.join("crates"), &mut files);
        walk(&repo.join("apps"), &mut files);
        let call = concat!("scratch_dir", "(\"");
        for file in files.iter().filter(|f| !f.ends_with("mg-testkit/src/lib.rs")) {
            let text = std::fs::read_to_string(file).unwrap_or_default();
            for (i, line) in text.lines().enumerate() {
                for part in line.split(call).skip(1) {
                    let name = part.split('"').next().unwrap_or_default().to_string();
                    let at = format!("{}:{}", file.display(), i + 1);
                    named.entry(name).or_default().push(at);
                }
            }
        }
        assert!(named.len() > 100, "{} found", named.len());
        let twice: Vec<_> = named.iter().filter(|(_, at)| at.len() > 1).collect();
        assert!(twice.is_empty(), "scratch folders named more than once: {twice:#?}");
    }
}
