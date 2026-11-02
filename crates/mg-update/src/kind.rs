//! What installed this copy of Moonglow, and so which package of a
//! release updates it.

use std::path::{Path, PathBuf};

use crate::Version;

/// A package manager that installed this copy, and updates it: Moonglow
/// doesn't check by itself there, and never writes its files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installer {
    Flatpak,
    Scoop,
    Nix,
}

impl Installer {
    pub fn name(self) -> &'static str {
        match self {
            Installer::Flatpak => "Flatpak",
            Installer::Scoop => "Scoop",
            Installer::Nix => "Nix",
        }
    }

    /// How to update through it.
    pub fn how(self) -> &'static str {
        match self {
            Installer::Flatpak => "flatpak update io.github.moonglow_toolset.Moonglow",
            Installer::Scoop => "scoop update moonglow",
            Installer::Nix => "update the flake or channel it comes from",
        }
    }

    /// The installer of a program at `exe`, with `FLATPAK_ID` set to
    /// `flatpak_id` and `/.flatpak-info` there or not.
    pub fn detect(flatpak_id: Option<&str>, flatpak_info: bool, exe: &Path) -> Option<Installer> {
        if flatpak_id.is_some_and(|id| !id.is_empty()) || flatpak_info {
            return Some(Installer::Flatpak);
        }
        // (Either separator, any case: Scoop is on Windows, the tests not.)
        let path = exe.to_string_lossy().replace('\\', "/").to_lowercase();
        if path.contains("/scoop/apps/") {
            Some(Installer::Scoop)
        } else if path.starts_with("/nix/store/") {
            Some(Installer::Nix)
        } else {
            None
        }
    }

    /// This program's installer.
    pub fn here() -> Option<Installer> {
        let exe = std::env::current_exe().unwrap_or_default();
        let id = std::env::var("FLATPAK_ID").ok();
        Installer::detect(id.as_deref(), Path::new("/.flatpak-info").exists(), &exe)
    }
}

/// What is known of this program where it runs, from which [`Kind`] is
/// told (read once at start by [`Facts::here`]; made up by tests).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// The program (`current_exe`).
    pub exe: PathBuf,
    /// `$APPIMAGE`: the image the AppImage's runtime started.
    pub appimage: Option<PathBuf>,
    /// `$FLATPAK_ID`, and `/.flatpak-info` there.
    pub flatpak_id: Option<String>,
    pub flatpak_info: bool,
    /// `std::env::consts::OS` and `ARCH`.
    pub os: String,
    pub arch: String,
    /// The installer's uninstaller (`unins000.exe`) is beside the program.
    pub uninstaller: bool,
    /// The Program Files folders (`%ProgramFiles%`, `%ProgramW6432%`):
    /// an install there is for all users.
    pub program_files: Vec<PathBuf>,
}

impl Facts {
    pub fn here() -> Facts {
        let exe = std::env::current_exe().unwrap_or_default();
        let uninstaller = exe.parent().is_some_and(|d| d.join("unins000.exe").is_file());
        let program_files = ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"]
            .iter()
            .filter_map(std::env::var_os)
            .map(PathBuf::from)
            .collect();
        Facts {
            exe,
            appimage: std::env::var_os("APPIMAGE")
                .filter(|a| !a.is_empty())
                .map(PathBuf::from)
                .filter(|a| a.is_file()),
            flatpak_id: std::env::var("FLATPAK_ID").ok(),
            flatpak_info: Path::new("/.flatpak-info").exists(),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            uninstaller,
            program_files,
        }
    }
}

/// How this copy was installed, and so how it is updated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// An AppImage: the image at `image` is replaced.
    AppImage { image: PathBuf, arch: String },
    /// Installed on Windows by the installer into `dir`: the new installer
    /// is run, for this user or (`all_users`, under Program Files) all.
    WindowsInstaller { dir: PathBuf, all_users: bool },
    /// Windows' zip, unpacked into `dir`: its files are swapped.
    WindowsZip { dir: PathBuf },
    /// The macOS app at `app` (`… .app`): swapped for the disk image's.
    MacApp { app: PathBuf },
    /// A package manager's: it updates Moonglow.
    Managed(Installer),
    /// Anything else (a build from source, a distribution's package):
    /// only the release's page is offered.
    Other,
}

impl Kind {
    pub fn detect(facts: &Facts) -> Kind {
        if let Some(i) =
            Installer::detect(facts.flatpak_id.as_deref(), facts.flatpak_info, &facts.exe)
        {
            return Kind::Managed(i);
        }
        if let Some(image) = &facts.appimage {
            return Kind::AppImage { image: image.clone(), arch: facts.arch.clone() };
        }
        // (A Windows path read on any system, as the tests do.)
        let dir = if facts.os == "windows" {
            let text = facts.exe.to_string_lossy();
            match text.rfind(['\\', '/']) {
                Some(i) => PathBuf::from(&text[..i]),
                None => return Kind::Other,
            }
        } else {
            match facts.exe.parent() {
                Some(d) => d.to_path_buf(),
                None => return Kind::Other,
            }
        };
        let dir = dir.as_path();
        match facts.os.as_str() {
            "windows" if facts.uninstaller => {
                let lower = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
                let all_users = facts
                    .program_files
                    .iter()
                    .any(|pf| lower(dir).starts_with(&format!("{}\\", lower(pf))));
                Kind::WindowsInstaller { dir: dir.to_path_buf(), all_users }
            }
            "windows" => Kind::WindowsZip { dir: dir.to_path_buf() },
            "macos" => {
                // `… .app/Contents/MacOS/moonglow`.
                let app = dir.parent().and_then(Path::parent).filter(|_| {
                    dir.file_name().is_some_and(|n| n == "MacOS")
                        && dir.parent().and_then(Path::file_name).is_some_and(|n| n == "Contents")
                });
                match app {
                    Some(app) if app.extension().is_some_and(|e| e == "app") => {
                        Kind::MacApp { app: app.to_path_buf() }
                    }
                    _ => Kind::Other,
                }
            }
            _ => Kind::Other,
        }
    }

    /// The release's file this copy is updated from (as
    /// `packaging/*/build-*` name them), if Moonglow updates it itself.
    pub fn package(&self, version: &Version) -> Option<String> {
        let v = version.file_part();
        match self {
            Kind::AppImage { arch, .. } => Some(format!("Moonglow-{v}-{arch}.AppImage")),
            Kind::WindowsInstaller { .. } => Some(format!("Moonglow-{v}-windows-x64-setup.exe")),
            Kind::WindowsZip { .. } => Some(format!("Moonglow-{v}-windows-x64.zip")),
            Kind::MacApp { .. } => Some(format!("Moonglow-{v}-macos.dmg")),
            Kind::Managed(_) | Kind::Other => None,
        }
    }

    /// What is replaced, said in the window.
    pub fn what(&self) -> String {
        match self {
            Kind::AppImage { image, .. } => format!("the AppImage {}", image.display()),
            Kind::WindowsInstaller { dir, .. } => {
                format!("the installed copy in {}", dir.display())
            }
            Kind::WindowsZip { dir } => format!("the files in {}", dir.display()),
            Kind::MacApp { app } => format!("the app {}", app.display()),
            Kind::Managed(i) => format!("{}'s copy", i.name()),
            Kind::Other => "this copy".into(),
        }
    }

    /// The folder the update is written into, beside what it replaces.
    pub fn folder(&self) -> Option<&Path> {
        match self {
            Kind::AppImage { image, .. } => image.parent(),
            Kind::WindowsInstaller { dir, .. } | Kind::WindowsZip { dir } => Some(dir),
            Kind::MacApp { app } => app.parent(),
            Kind::Managed(_) | Kind::Other => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_managed_installs_are_known_by_path_and_environment() {
        let detect = Installer::detect;
        let plain = Path::new("/home/me/Applications/Moonglow.AppImage");
        assert_eq!(detect(None, false, plain), None);
        assert_eq!(
            detect(Some("io.github.moonglow_toolset.Moonglow"), false, plain),
            Some(Installer::Flatpak)
        );
        assert_eq!(detect(None, true, Path::new("/app/bin/moonglow")), Some(Installer::Flatpak));
        assert_eq!(detect(Some(""), false, plain), None);
        let scoop = Path::new(r"C:\Users\me\scoop\apps\moonglow\2.4.0\moonglow.exe");
        assert_eq!(detect(None, false, scoop), Some(Installer::Scoop));
        let global = Path::new(r"C:\ProgramData\Scoop\Apps\moonglow\current\moonglow.exe");
        assert_eq!(detect(None, false, global), Some(Installer::Scoop));
        let nix = Path::new("/nix/store/abc123-moonglow-2.4.0/bin/moonglow");
        assert_eq!(detect(None, false, nix), Some(Installer::Nix));
        // The installer and the zip, the macOS app: none.
        let windows = Path::new(r"C:\Users\me\AppData\Local\Programs\Moonglow\moonglow.exe");
        assert_eq!(detect(None, false, windows), None);
        let mac = Path::new("/Applications/Moonglow Toolset.app/Contents/MacOS/moonglow");
        assert_eq!(detect(None, false, mac), None);
        assert_eq!(detect(None, false, Path::new("/home/me/nix/store/moonglow")), None);
    }

    fn facts(os: &str, exe: &str) -> Facts {
        Facts { exe: exe.into(), os: os.into(), arch: "x86_64".into(), ..Facts::default() }
    }

    #[test]
    fn each_install_is_known_and_takes_its_package() {
        let v = Version::parse("3.3.0").unwrap();
        let rc = Version::parse("3.3.0-rc.1").unwrap();
        // An AppImage, wherever its runtime mounted the program.
        let mut f = facts("linux", "/tmp/.mount_MoonglXYZ/usr/bin/moonglow");
        f.appimage = Some("/home/me/.local/opt/Moonglow.AppImage".into());
        let kind = Kind::detect(&f);
        assert_eq!(
            kind,
            Kind::AppImage {
                image: "/home/me/.local/opt/Moonglow.AppImage".into(),
                arch: "x86_64".into()
            }
        );
        assert_eq!(kind.package(&v).unwrap(), "Moonglow-3.3.0-x86_64.AppImage");
        assert_eq!(kind.package(&rc).unwrap(), "Moonglow-3.3.0-rc.1-x86_64.AppImage");
        assert_eq!(kind.folder(), Some(Path::new("/home/me/.local/opt")));
        // A Flatpak's is never an AppImage, whatever the environment says.
        f.flatpak_id = Some("io.github.moonglow_toolset.Moonglow".into());
        assert_eq!(Kind::detect(&f), Kind::Managed(Installer::Flatpak));
        assert_eq!(Kind::Managed(Installer::Flatpak).package(&v), None);
        // A build of one's own, a distribution's package: the page only.
        let own = Kind::detect(&facts("linux", "/home/me/moonglow/target/release/moonglow"));
        assert_eq!(own, Kind::Other);
        assert_eq!(own.package(&v), None);

        // Windows: the installer's (its uninstaller beside), for this user
        // or under Program Files for all; the zip's.
        let mut f =
            facts("windows", r"C:\Users\me\AppData\Local\Programs\Moonglow Toolset\moonglow.exe");
        f.uninstaller = true;
        f.program_files = vec![r"C:\Program Files".into()];
        let kind = Kind::detect(&f);
        assert!(matches!(kind, Kind::WindowsInstaller { all_users: false, .. }), "{kind:?}");
        assert_eq!(kind.package(&v).unwrap(), "Moonglow-3.3.0-windows-x64-setup.exe");
        f.exe = r"C:\Program Files\Moonglow Toolset\moonglow.exe".into();
        assert!(matches!(Kind::detect(&f), Kind::WindowsInstaller { all_users: true, .. }));
        // (A folder whose name only begins as Program Files' does.)
        f.exe = r"C:\Program Files Extra\Moonglow\moonglow.exe".into();
        assert!(matches!(Kind::detect(&f), Kind::WindowsInstaller { all_users: false, .. }));
        f.uninstaller = false;
        let zip = Kind::detect(&f);
        assert!(matches!(zip, Kind::WindowsZip { .. }));
        assert_eq!(zip.package(&v).unwrap(), "Moonglow-3.3.0-windows-x64.zip");
        // Scoop installs the zip, and updates it.
        let scoop = facts("windows", r"C:\Users\me\scoop\apps\moonglow\current\moonglow.exe");
        assert_eq!(Kind::detect(&scoop), Kind::Managed(Installer::Scoop));

        // macOS: the app the program is in.
        let mac = Kind::detect(&facts(
            "macos",
            "/Applications/Moonglow Toolset.app/Contents/MacOS/moonglow",
        ));
        assert_eq!(mac, Kind::MacApp { app: "/Applications/Moonglow Toolset.app".into() });
        assert_eq!(mac.package(&v).unwrap(), "Moonglow-3.3.0-macos.dmg");
        assert_eq!(mac.folder(), Some(Path::new("/Applications")));
        assert_eq!(Kind::detect(&facts("macos", "/usr/local/bin/moonglow")), Kind::Other);
    }
}
