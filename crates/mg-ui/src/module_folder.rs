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
/// module (its `module.ifo`, in any case).
pub fn folder_beside(path: &Path) -> Option<PathBuf> {
    let is_mod = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mod"));
    if !is_mod || !path.is_file() {
        return None;
    }
    let dir = path.with_extension("");
    let has_info = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("module.ifo"));
    has_info.then_some(dir)
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
    let folder = path.with_extension("");
    crate::widgets::window("Open Directory as a Module").show(ctx, |ui| {
        ui.label("A directory with the same name has been found.");
        ui.label("Do you want to open this directory as a module?");
        ui.weak(folder.display().to_string());
        ui.horizontal(|ui| {
            let mut chosen = None;
            if ui.button("Open Directory").on_hover_text("Saved into the folder").clicked() {
                chosen = Some(folder.clone());
            }
            if ui.button("Open .mod File").clicked() {
                chosen = Some(path.clone());
            }
            if let Some(p) = chosen {
                app.actions.push(Action::OpenModuleAsIs(p));
                app.module_folder = None;
            }
            if crate::widgets::cancel(ui) {
                app.module_folder = None;
            }
        });
        ui.weak("Options › General: A .mod with its folder beside it");
    });
}
