//! The shipped modules as the area tests walk them: each opened over its
//! haks and the game.

use std::path::{Path, PathBuf};

use mg_module::Module;
use mg_resman::GameInstall;
use mg_rules::GameData;
use mg_testkit::{bundled_modules, sweep};

/// One shipped module and the game's data with it and its haks on top.
pub(crate) struct Opened {
    /// The module's file name without its extension.
    pub name: String,
    pub path: PathBuf,
    pub module: Module,
    pub game: GameData,
    /// The module's haks, as it names them.
    pub haks: Vec<String>,
    /// Those of them the install doesn't have.
    pub missing_haks: Vec<String>,
}

/// Calls `each` with every shipped module (every sixth unless the run is
/// full), opened.
pub(crate) fn each_module(root: &Path, mut each: impl FnMut(&Opened)) {
    let install = GameInstall::new(root, None, "en");
    for path in sweep(bundled_modules(root), 6) {
        let module = Module::open(&path).unwrap();
        let mut game = GameData::open(&install).unwrap();
        let haks = module.haks().unwrap();
        let (missing_haks, wrong) = module.stack_onto(&mut game.resman, &install);
        assert!(wrong.is_none(), "{}: {wrong:?}", path.display());
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        each(&Opened { name, path, module, game, haks, missing_haks });
    }
}
