//! Installing an update made ready ([`crate::Staged`]): as steps, in
//! order, so that what is run on Windows and macOS is a list the tests
//! read here.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::stage::{PREFIX, Ready, Staged};
use crate::{Kind, verify};

/// The last install's outcome, in the updates' cache folder: the window
/// is gone as Moonglow closes, so the next start logs it ([`take_report`]).
pub const REPORT: &str = "last-install.txt";

/// A step of an install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// A program run, waited for (and failing unless it succeeds), or
    /// started and left.
    Run {
        program: PathBuf,
        args: Vec<OsString>,
        wait: bool,
    },
    /// A rename in one volume: atomic, and undone if a later step fails.
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    CreateDir(PathBuf),
    /// Removed, whatever it holds; a failure is no failure (what is left is
    /// removed at the next start).
    RemoveAll(PathBuf),
    /// Fails unless the path is there (saying what is missing).
    Exists(PathBuf, String),
}

/// How Moonglow is started again once it is installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restart {
    /// That program (the AppImage itself, not the program inside its
    /// mount, which goes when it exits).
    Exe(PathBuf),
    /// The macOS app, through `open -n`.
    Open(PathBuf),
    /// The installer starts it when it is done.
    ByInstaller,
}

fn step_error(step: &Step, e: impl std::fmt::Display) -> String {
    match step {
        Step::Run { program, .. } => format!("{} failed: {e}", program.display()),
        Step::Rename { from, to } => {
            format!("can't move {} to {}: {e}", from.display(), to.display())
        }
        Step::CreateDir(p) => format!("can't make {}: {e}", p.display()),
        Step::RemoveAll(p) => format!("can't remove {}: {e}", p.display()),
        Step::Exists(_, what) => format!("{what} is missing: {e}"),
    }
}

fn one(step: &Step) -> Result<(), String> {
    match step {
        Step::Run { program, args, wait } => {
            let mut command = std::process::Command::new(program);
            command.args(args);
            if *wait {
                let status = command.status().map_err(|e| step_error(step, e))?;
                if !status.success() {
                    return Err(step_error(step, status));
                }
            } else {
                command.spawn().map_err(|e| step_error(step, e))?;
            }
            Ok(())
        }
        Step::Rename { from, to } => std::fs::rename(from, to).map_err(|e| step_error(step, e)),
        Step::CreateDir(p) => std::fs::create_dir_all(p).map_err(|e| step_error(step, e)),
        Step::RemoveAll(p) => {
            let _ = if p.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) };
            Ok(())
        }
        Step::Exists(p, _) => {
            if p.exists() {
                Ok(())
            } else {
                Err(step_error(step, "not there"))
            }
        }
    }
}

/// Runs `steps` in order; at a failure, the renames done are undone, last
/// first, and the failure said.
pub fn run(steps: &[Step]) -> Result<(), String> {
    for (i, step) in steps.iter().enumerate() {
        if let Err(e) = one(step) {
            for done in steps[..i].iter().rev() {
                if let Step::Rename { from, to } = done {
                    let _ = std::fs::rename(to, from);
                }
            }
            return Err(e);
        }
    }
    Ok(())
}

/// Runs every one of `steps`, whatever fails, and says the first failure.
pub fn run_all(steps: &[Step]) -> Result<(), String> {
    let mut first = None;
    for step in steps {
        if let Err(e) = one(step) {
            first.get_or_insert(e);
        }
    }
    first.map_or(Ok(()), Err)
}

/// The Windows installer's command line: silent, into the same folder, for
/// the same users, closing what holds its files, and (`restart`) starting
/// Moonglow when done (its `[Run]` entry, `packaging/windows/moonglow.iss`).
pub fn installer_args(dir: &Path, all_users: bool, log: &Path, restart: bool) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "/SILENT",
        "/SUPPRESSMSGBOXES",
        "/NORESTART",
        "/SP-",
        "/CLOSEAPPLICATIONS",
        if all_users { "/ALLUSERS" } else { "/CURRENTUSER" },
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    let mut d = OsString::from("/DIR=");
    d.push(dir);
    args.push(d);
    let mut l = OsString::from("/LOG=");
    l.push(log);
    args.push(l);
    if restart {
        args.push("/restart=1".into());
    }
    args
}

/// Every file under `dir`, as paths in it.
fn files_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        let entries = std::fs::read_dir(dir.join(&rel))
            .map_err(|e| format!("can't read {}: {e}", dir.display()))?;
        for e in entries.flatten() {
            let path = rel.join(e.file_name());
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The steps that install `staged` as Moonglow closes (`cache`: the
/// updates' cache folder, for the installer's log), and how Moonglow is
/// started again after.
pub fn plan(staged: &Staged, restart: bool, cache: &Path) -> Result<(Vec<Step>, Restart), String> {
    Ok(match (&staged.kind, &staged.ready) {
        (Kind::AppImage { image, .. }, Ready::Image { file, .. }) => (
            vec![Step::Rename { from: file.clone(), to: image.clone() }],
            Restart::Exe(image.clone()),
        ),
        (Kind::AppImage { image, .. }, Ready::Installed) => {
            (Vec::new(), Restart::Exe(image.clone()))
        }
        (Kind::WindowsInstaller { dir, all_users }, Ready::Installer { file, .. }) => (
            vec![Step::Run {
                program: file.clone(),
                args: installer_args(dir, *all_users, &cache.join("install.log"), restart),
                wait: false,
            }],
            Restart::ByInstaller,
        ),
        (Kind::WindowsZip { dir }, Ready::Files { dir: new }) => {
            // Each file the zip holds: the old one moved aside (a running
            // program can be moved on Windows, not overwritten), the new
            // one into its place. Nothing else of the folder is touched.
            let old = dir.join(format!("{PREFIX}old"));
            let mut steps = vec![Step::RemoveAll(old.clone())];
            for rel in files_in(new)? {
                let (from, to) = (new.join(&rel), dir.join(&rel));
                if let Some(parent) = to.parent() {
                    steps.push(Step::CreateDir(parent.to_path_buf()));
                }
                if to.exists() {
                    let aside = old.join(&rel);
                    steps.push(Step::CreateDir(aside.parent().unwrap_or(&old).to_path_buf()));
                    steps.push(Step::Rename { from: to.clone(), to: aside });
                }
                steps.push(Step::Rename { from, to });
            }
            steps.push(Step::RemoveAll(new.clone()));
            (steps, Restart::Exe(dir.join("moonglow.exe")))
        }
        (Kind::MacApp { app }, Ready::App { app: new }) => {
            // The app moved aside, the new one into its place (undone if
            // that fails), the old one removed.
            let old = app.with_file_name(format!("{PREFIX}old.app"));
            (
                vec![
                    Step::RemoveAll(old.clone()),
                    Step::Rename { from: app.clone(), to: old.clone() },
                    Step::Rename { from: new.clone(), to: app.clone() },
                    Step::RemoveAll(old),
                ],
                Restart::Open(app.clone()),
            )
        }
        (kind, ready) => return Err(format!("{ready:?} doesn't install {}", kind.what())),
    })
}

/// Installs `staged` (its files checked again first) and says how to start
/// Moonglow again.
pub fn install(staged: &Staged, restart: bool, cache: &Path) -> Result<Restart, String> {
    if let Ready::Image { file, hash } | Ready::Installer { file, hash } = &staged.ready {
        verify::still(file, hash)?;
    }
    let (steps, again) = plan(staged, restart, cache)?;
    run(&steps)?;
    Ok(again)
}

/// The outcome of an install as Moonglow closed, kept for the next start.
pub fn write_report(cache: &Path, text: &str) {
    let _ = std::fs::create_dir_all(cache);
    let _ = std::fs::write(cache.join(REPORT), text);
}

/// The last install's outcome, if one is waiting to be said (and then
/// forgotten).
pub fn take_report(cache: &Path) -> Option<String> {
    let path = cache.join(REPORT);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Version;

    fn staged(kind: Kind, ready: Ready) -> Staged {
        Staged { version: Version::parse("3.3.0").unwrap(), kind, ready }
    }

    #[test]
    fn the_windows_installer_runs_silently_into_the_same_folder() {
        let dir = PathBuf::from(r"C:\Users\me\AppData\Local\Programs\Moonglow Toolset");
        let cache = PathBuf::from(r"C:\Users\me\AppData\Local\moonglow\cache\updates");
        let file = cache.join("Moonglow-3.3.0-windows-x64-setup.exe");
        let s = staged(
            Kind::WindowsInstaller { dir: dir.clone(), all_users: false },
            Ready::Installer { file: file.clone(), hash: [0; 32] },
        );
        let (steps, again) = plan(&s, true, &cache).unwrap();
        assert_eq!(again, Restart::ByInstaller);
        let [Step::Run { program, args, wait: false }] = steps.as_slice() else {
            panic!("{steps:?}")
        };
        assert_eq!(program, &file);
        let args: Vec<_> = args.iter().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(
            args,
            [
                "/SILENT".to_string(),
                "/SUPPRESSMSGBOXES".into(),
                "/NORESTART".into(),
                "/SP-".into(),
                "/CLOSEAPPLICATIONS".into(),
                "/CURRENTUSER".into(),
                format!("/DIR={}", dir.display()),
                format!("/LOG={}", cache.join("install.log").display()),
                "/restart=1".into(),
            ]
        );
        // For all users (under Program Files), and installed at the next
        // start without a restart.
        let all = staged(
            Kind::WindowsInstaller {
                dir: r"C:\Program Files\Moonglow Toolset".into(),
                all_users: true,
            },
            Ready::Installer { file, hash: [0; 32] },
        );
        let (steps, _) = plan(&all, false, &cache).unwrap();
        let Step::Run { args, .. } = &steps[0] else { panic!() };
        assert!(args.contains(&"/ALLUSERS".into()));
        assert!(!args.contains(&"/restart=1".into()));
    }

    #[test]
    fn the_macos_app_is_moved_aside_then_the_new_one_into_its_place() {
        let s = staged(
            Kind::MacApp { app: "/Applications/Moonglow Toolset.app".into() },
            Ready::App { app: "/Applications/.moonglow-update-3.3.0.app".into() },
        );
        let (steps, again) = plan(&s, true, Path::new("/c")).unwrap();
        assert_eq!(again, Restart::Open("/Applications/Moonglow Toolset.app".into()));
        assert_eq!(
            steps,
            [
                Step::RemoveAll("/Applications/.moonglow-update-old.app".into()),
                Step::Rename {
                    from: "/Applications/Moonglow Toolset.app".into(),
                    to: "/Applications/.moonglow-update-old.app".into()
                },
                Step::Rename {
                    from: "/Applications/.moonglow-update-3.3.0.app".into(),
                    to: "/Applications/Moonglow Toolset.app".into()
                },
                Step::RemoveAll("/Applications/.moonglow-update-old.app".into()),
            ]
        );
    }

    /// The swap run on folders here, as the Mac's would be: done, and
    /// undone when its second rename fails.
    #[test]
    fn the_app_swap_is_undone_when_it_fails() {
        let dir = crate::tests::scratch("install-swap");
        let app = dir.join("Moonglow Toolset.app");
        let new = dir.join(".moonglow-update-3.3.0.app");
        for (d, v) in [(&app, "old"), (&new, "new")] {
            std::fs::create_dir_all(d.join("Contents/MacOS")).unwrap();
            std::fs::write(d.join("Contents/MacOS/moonglow"), v).unwrap();
        }
        let s = staged(Kind::MacApp { app: app.clone() }, Ready::App { app: new.clone() });
        let read = || std::fs::read_to_string(app.join("Contents/MacOS/moonglow")).unwrap();
        // The new one gone before it is swapped in: the old one is put back.
        std::fs::rename(&new, dir.join("elsewhere")).unwrap();
        let e = install(&s, true, &dir).unwrap_err();
        assert!(e.contains("can't move"), "{e}");
        assert_eq!(read(), "old");
        assert!(!dir.join(".moonglow-update-old.app").exists());
        std::fs::rename(dir.join("elsewhere"), &new).unwrap();
        assert_eq!(install(&s, true, &dir).unwrap(), Restart::Open(app.clone()));
        assert_eq!(read(), "new");
        assert!(!new.exists() && !dir.join(".moonglow-update-old.app").exists());
    }

    #[test]
    fn a_staged_file_changed_since_it_was_checked_isn_t_installed() {
        let dir = crate::tests::scratch("install-changed");
        let image = dir.join("Moonglow.AppImage");
        let file = dir.join(".moonglow-update-3.3.0.AppImage");
        std::fs::write(&image, b"old").unwrap();
        std::fs::write(&file, b"new").unwrap();
        let hash = verify::hash_file(&file).unwrap();
        std::fs::write(&file, b"evil").unwrap();
        let s = staged(
            Kind::AppImage { image: image.clone(), arch: "x86_64".into() },
            Ready::Image { file, hash },
        );
        assert!(install(&s, true, &dir).unwrap_err().contains("changed after it was checked"));
        assert_eq!(std::fs::read(&image).unwrap(), b"old");
    }

    #[test]
    fn the_report_is_said_once() {
        let dir = crate::tests::scratch("install-report");
        assert_eq!(take_report(&dir), None);
        write_report(&dir, "Moonglow 3.3.0 was installed.");
        assert_eq!(take_report(&dir).as_deref(), Some("Moonglow 3.3.0 was installed."));
        assert_eq!(take_report(&dir), None);
    }
}
