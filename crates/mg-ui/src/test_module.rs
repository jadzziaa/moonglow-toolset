//! Build › Test Module (F9), as Aurora's: saves the module, then starts the
//! game on it (`nwmain -userdirectory <dir> +TestNewModule <module>`): the
//! first local-vault character, at the start location. The game finds the
//! module by name in the user directory's `modules` folder, so the module
//! must be saved there.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The game client of the install at `root`, for this platform.
pub fn client_binary(root: &Path) -> Option<PathBuf> {
    let rel = if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "bin/linux-arm64/nwmain-linux"
        } else {
            "bin/linux-x86/nwmain-linux"
        }
    } else if cfg!(target_os = "macos") {
        "bin/macos/nwmain.app/Contents/MacOS/nwmain"
    } else {
        "bin/win32/nwmain.exe"
    };
    Some(root.join(rel)).filter(|p| p.is_file())
}

/// The module's name for `+TestNewModule`, if `module` (a `.mod` file or a
/// module folder) is in `user_dir`'s `modules` folder.
pub fn module_name(user_dir: &Path, module: &Path) -> Option<String> {
    let folder = module.parent()?;
    let modules = user_dir.join("modules");
    let same = |a: &Path, b: &Path| {
        a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a, b)| a == b)
    };
    if !same(folder, &modules) {
        return None;
    }
    let is_mod = module.extension().is_some_and(|e| e.eq_ignore_ascii_case("mod"));
    let name = if is_mod { module.file_stem()? } else { module.file_name()? };
    Some(name.to_string_lossy().into_owned())
}

/// The command Test Module runs: the client, from its own folder. The game
/// starts the module with the first character of the list
/// (`+TestNewModule`, as Aurora's F9 does), or, with `choose`, shows its
/// character selection for the module (`+LoadNewModule`).
pub fn command(client: &Path, user_dir: &Path, module: &str, choose: bool) -> Command {
    let mut c = Command::new(client);
    if let Some(dir) = client.parent() {
        c.current_dir(dir);
    }
    let how = if choose { "+LoadNewModule" } else { "+TestNewModule" };
    c.arg("-userdirectory").arg(user_dir).arg(how).arg(module);
    c
}

/// The module Test From Here writes into the modules folder (and replaces
/// each time).
pub const FROM_HERE: &str = "moonglow-test";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_is_named_from_the_modules_folder() {
        let dir = std::env::temp_dir().join(format!("mg-test-module-{}", std::process::id()));
        let modules = dir.join("modules");
        std::fs::create_dir_all(modules.join("Folder Module")).unwrap();
        std::fs::write(modules.join("My Module.mod"), b"").unwrap();
        assert_eq!(module_name(&dir, &modules.join("My Module.mod")).as_deref(), Some("My Module"));
        assert_eq!(
            module_name(&dir, &modules.join("Folder Module")).as_deref(),
            Some("Folder Module")
        );
        assert_eq!(module_name(&dir, &dir.join("elsewhere.mod")), None);
        let c = command(Path::new("/game/bin/linux-x86/nwmain-linux"), &dir, "My Module", false);
        let args: Vec<String> = c.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(
            args,
            [
                "-userdirectory".to_string(),
                dir.display().to_string(),
                "+TestNewModule".into(),
                "My Module".into()
            ]
        );
        assert_eq!(c.get_current_dir(), Some(Path::new("/game/bin/linux-x86")));
        let c = command(Path::new("/game/bin/linux-x86/nwmain-linux"), &dir, "My Module", true);
        assert_eq!(c.get_args().nth(2).unwrap(), "+LoadNewModule");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
