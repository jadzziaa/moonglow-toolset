//! The scripts the script tests sweep (the base game's, then each shipped
//! module's over its haks), and every core put to a list of them.

use std::path::Path;

use mg_core::ResType;
use mg_module::Module;
use mg_resman::{GameInstall, ResMan};
use mg_testkit::{bundled_modules, sweep};

/// One set of scripts and the resources they compile against.
pub(crate) struct Set<'a> {
    /// "base", or the module's file name.
    pub label: String,
    pub rm: &'a ResMan,
    /// The scripts' names, lowercase.
    pub names: &'a [String],
    /// The module, its haks (first: highest priority) and its number among
    /// the shipped modules; `None` for the base game.
    pub module: Option<(&'a Module, &'a [String], usize)>,
}

/// Calls `each` with the base game's scripts, then with those of each
/// shipped module that has any. A full run takes them all; else an eighth
/// of the base game's and every sixth module.
pub(crate) fn each_script_set(root: &Path, mut each: impl FnMut(&Set<'_>)) {
    let install = GameInstall::new(root, None, "en");
    let base = ResMan::for_game(&install).unwrap();
    let names: Vec<String> =
        base.list(ResType::NSS).iter().map(|r| r.to_lowercase().to_string()).collect();
    each(&Set { label: "base".into(), rm: &base, names: &sweep(names, 8), module: None });
    let modules: Vec<_> = bundled_modules(root).into_iter().enumerate().collect();
    for (i, path) in sweep(modules, 6) {
        let m = Module::open(&path).unwrap();
        let names: Vec<String> =
            m.keys_of(ResType::NSS).map(|k| k.resref.to_lowercase().to_string()).collect();
        if names.is_empty() {
            continue;
        }
        let mut rm = ResMan::for_game(&install).unwrap();
        let (_, wrong) = m.stack_onto(&mut rm, &install);
        assert!(wrong.is_none(), "{}: {wrong:?}", path.display());
        let haks = m.haks().unwrap();
        let label = path.file_name().unwrap().to_string_lossy().into_owned();
        each(&Set { label, rm: &rm, names: &names, module: Some((&m, &haks, i)) });
    }
}

/// Runs `work` on a part of `names` on each core (a compiler is made per
/// thread); the parts' results, in order.
pub(crate) fn on_every_core<T: Send>(
    names: &[String],
    work: impl Fn(&[String]) -> T + Sync,
) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = names.len().div_ceil(threads).max(1);
    let work = &work;
    std::thread::scope(|s| {
        let handles: Vec<_> = names.chunks(chunk).map(|part| s.spawn(move || work(part))).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}
