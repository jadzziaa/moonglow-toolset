//! A `.mod` with a folder of its name beside it (`modules/name.mod` and
//! `modules/name/`: a module directory, as Aurora keeps one): opening the
//! `.mod` asks which to open, as Aurora asks, unless Options › General
//! says (Aurora's Always open module directories opens the folder). The
//! folder opened is saved into, and written as the `.mod` beside it too
//! (`write_mod_beside_folder`): the game and its server read the `.mod`.

use std::path::{Path, PathBuf};

use crate::settings::ModuleFolders;
use crate::{Action, Moonglow};

/// The folder of a `.mod`'s name beside it, if there is one that holds a
/// module (its `module.ifo`), its name in any case
/// ([`mg_module::folder::beside_mod`]).
pub fn folder_beside(path: &Path) -> Option<PathBuf> {
    mg_module::folder::beside_mod(path)
}

/// Opens a module the user chose (File › Open Module…, Recent Modules,
/// the command line): a `.mod` with its folder beside it opens the folder
/// or the `.mod` as Options › General says, or asks which.
pub(crate) fn open(app: &mut Moonglow, path: &Path) {
    let Some(folder) = folder_beside(path) else { return app.open_module(path) };
    match app.settings.module_folders {
        ModuleFolders::Ask => app.module_folder = Some(path.to_path_buf()),
        ModuleFolders::Folder => app.open_module(&folder),
        ModuleFolders::Module => app.open_module(path),
    }
}

/// The question, in Aurora's words, while one is asked.
pub(crate) fn ui(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(path) = app.module_folder.clone() else { return };
    let folder = folder_beside(&path).unwrap_or_else(|| path.with_extension(""));
    let id = egui::Id::new("module-folder-always");
    let (on, tip) = crate::widgets::always_now(
        ctx,
        id,
        "Tools › Options › General › Questions › A .mod with its folder beside it",
    );
    let footed = crate::widgets::modal(ctx, "Open Directory as a Module").show_foot(ctx, |ui| {
        ui.label("A directory with the same name has been found.");
        ui.label("Do you want to open this directory as a module?");
        ui.weak(folder.display().to_string());
        let foot = crate::widgets::Foot::new()
            .check("Always do this", on, Some(tip))
            .main("Open Directory", true)
            .tip("Saved into the folder")
            .button("Open .mod File", true)
            .cancel();
        ((), foot)
    });
    let Some(f) = footed else { return };
    let always = crate::widgets::always_kept(ctx, id, f.pressed.checked);
    let chosen = if f.pressed.main() {
        Some((folder, ModuleFolders::Folder))
    } else if f.pressed.clicked("Open .mod File") {
        Some((path, ModuleFolders::Module))
    } else {
        None
    };
    if let Some((p, choice)) = chosen {
        // (Always so: Options' choice, from now on.)
        if always {
            app.settings.module_folders = choice;
        }
        app.actions.push(Action::OpenModuleAsIs(p));
        app.module_folder = None;
    } else if f.pressed.cancelled() {
        app.module_folder = None;
    }
}
