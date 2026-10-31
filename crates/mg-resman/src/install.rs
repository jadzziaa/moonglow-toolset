//! Finding the game and building the base resource stack.

use std::env;
use std::path::{Path, PathBuf};

use crate::container::{DirContainer, ErfContainer, KeyContainer};
use crate::{LayerClass, ResError, ResMan, priority};

/// A game install, the player's user directory and the language to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameInstall {
    /// The install root (has `data/nwn_base.key`).
    pub root: PathBuf,
    /// The user directory (`override`, `hak`, `modules`, `portraits`, ...);
    /// `None` to use the install alone.
    pub user_dir: Option<PathBuf>,
    /// Two-letter language code (`en`, `de`, ...), a folder under `lang/`.
    pub language: String,
    /// The Steam Workshop items subscribed to are read as well (a Steam
    /// install's `steamapps/workshop/content/704450/*`), as the game
    /// started through Steam reads them: each one's `override/` under the
    /// user's, its `hak/` and `tlk/` after the user's. Off unless asked
    /// for: the other tools and the game started outside Steam don't.
    pub workshop: bool,
    /// The haks the user's `userpatch.ini` names are read as well, from
    /// the folder `nwn.ini` aliases as `PATCH` (`patch/` in the user
    /// directory), between the module and `override/` as the game reads
    /// them ([`GameInstall::userpatch_haks`]). Aurora doesn't read them;
    /// off unless asked for.
    pub userpatch: bool,
}

impl GameInstall {
    pub fn new(root: impl Into<PathBuf>, user_dir: Option<PathBuf>, language: &str) -> GameInstall {
        GameInstall {
            root: root.into(),
            user_dir,
            language: language.to_string(),
            workshop: false,
            userpatch: false,
        }
    }

    /// The game as a player who downloads the module has it: this
    /// install's own files, without the user directory's folders
    /// (`override/`, `development/`, portraits, `userpatch.ini`) or the
    /// Workshop's. A module's haks are still found where this install
    /// finds them ([`ResMan::add_haks`] with the install itself).
    pub fn as_player(&self) -> GameInstall {
        GameInstall { user_dir: None, workshop: false, ..self.clone() }
    }

    /// The haks `userpatch.ini` (in the user directory) names, highest
    /// first, each with its file in the `PATCH` folder if it is there:
    /// `PatchFile000=name` and on, the highest number winning, as the game
    /// reads them. The folder is `nwn.ini`'s `[Alias]` `PATCH=`, else
    /// `patch/` in the user directory. Empty without the file, or with
    /// [`GameInstall::userpatch`] off.
    pub fn userpatch_haks(&self) -> Vec<(String, Option<PathBuf>)> {
        let Some(user) = self.user_dir.as_ref().filter(|_| self.userpatch) else {
            return Vec::new();
        };
        let read = |name: &str| {
            std::fs::read(user.join(name)).map(|b| b.iter().map(|&c| c as char).collect::<String>())
        };
        let Ok(ini) = read("userpatch.ini") else { return Vec::new() };
        let folder = read("nwn.ini")
            .ok()
            .and_then(|nwn| ini_value(&nwn, "alias", "patch"))
            .map(PathBuf::from)
            .unwrap_or_else(|| user.join("patch"));
        let mut haks: Vec<(u32, String)> = ini_section(&ini, "patch")
            .into_iter()
            .filter_map(|(k, v)| {
                let n = k.to_ascii_lowercase().strip_prefix("patchfile")?.parse().ok()?;
                let name = v.strip_suffix(".hak").unwrap_or(&v).to_string();
                (!name.is_empty()).then_some((n, name))
            })
            .collect();
        haks.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
        haks.into_iter()
            .map(|(_, name)| {
                let path = Some(folder.join(format!("{name}.hak"))).filter(|p| p.is_file());
                (name, path)
            })
            .collect()
    }

    /// Whether a directory looks like a game install.
    pub fn is_install(root: &Path) -> bool {
        root.join("data").join("nwn_base.key").is_file()
    }

    /// Why `root` is not a game install, for a builder who chose it
    /// (`None`: it is one). The mistakes builders make are told apart: the
    /// original game's folder (1.69, the Diamond Edition's: its
    /// `chitin.key` beside `nwmain.exe`, no `data/nwn_base.key`; GitHub
    /// issue 39), a folder inside the game's (`bin/win32`, `data`), and
    /// one above it (Steam's `common`).
    pub fn why_not(root: &Path) -> Option<String> {
        if Self::is_install(root) {
            return None;
        }
        if root.join("chitin.key").is_file() {
            return Some(
                "this is the original game (1.69, as the Diamond Edition is): Moonglow needs \
                 Enhanced Edition, whose folder has data/nwn_base.key"
                    .into(),
            );
        }
        if let Some(game) = root.ancestors().skip(1).take(3).find(|a| Self::is_install(a)) {
            return Some(format!("this is a folder inside the game's: choose {}", game.display()));
        }
        let children = std::fs::read_dir(root).into_iter().flatten().flatten();
        if let Some(game) = children.map(|e| e.path()).find(|c| Self::is_install(c)) {
            return Some(format!("the game is in {}: choose that folder", game.display()));
        }
        Some("no data/nwn_base.key in it".into())
    }

    /// The game's folder: `NWN_ROOT`, else the first install in the
    /// usual places (each Steam library, then GOG's). The tests find the
    /// game here too, so that they run where the toolset finds it.
    pub fn find_root() -> Option<PathBuf> {
        match env::var_os("NWN_ROOT") {
            Some(r) => Some(PathBuf::from(r)).filter(|p| Self::is_install(p)),
            None => candidate_roots().into_iter().find(|p| Self::is_install(p)),
        }
    }

    /// Finds the install (`NWN_ROOT`, then the usual Steam locations) and the
    /// user directory (`NWN_HOME`, `NWN_USER_DIRECTORY`, then the platform
    /// default), in English.
    pub fn detect() -> Option<GameInstall> {
        let root = Self::find_root()?;
        let user_dir = Self::detect_user_dir();
        Some(GameInstall {
            root,
            user_dir,
            language: "en".into(),
            workshop: false,
            userpatch: false,
        })
    }

    /// The user directory as [`GameInstall::detect`] finds it, with or
    /// without the game: `NWN_HOME`, `NWN_USER_DIRECTORY`, then the
    /// platform default; a folder that is there.
    pub fn detect_user_dir() -> Option<PathBuf> {
        env::var_os("NWN_HOME")
            .or_else(|| env::var_os("NWN_USER_DIRECTORY"))
            .map(PathBuf::from)
            .or_else(default_user_dir)
            .filter(|p| p.is_dir())
    }

    /// `lang/<language>/data`.
    pub fn lang_data(&self) -> PathBuf {
        self.root.join("lang").join(&self.language).join("data")
    }

    /// The talk table for the language (`dialog.tlk`).
    pub fn talk_table(&self) -> PathBuf {
        self.lang_data().join("dialog.tlk")
    }

    fn user(&self, sub: &str) -> Option<PathBuf> {
        self.user_dir.as_ref().map(|u| u.join(sub))
    }

    /// The Steam Workshop items read ([`GameInstall::workshop`]): their
    /// folders, by number. None for an install that isn't in a Steam
    /// library.
    pub fn workshop_items(&self) -> Vec<PathBuf> {
        if !self.workshop {
            return Vec::new();
        }
        // `<library>/steamapps/common/<game>`.
        let steamapps = self.root.parent().and_then(Path::parent);
        let Some(content) = steamapps
            .filter(|s| s.file_name().is_some_and(|n| n.eq_ignore_ascii_case("steamapps")))
            .map(|s| s.join("workshop").join("content").join(STEAM_APP))
        else {
            return Vec::new();
        };
        let mut items: Vec<PathBuf> = std::fs::read_dir(content)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        items.sort();
        items
    }

    /// A folder of each Workshop item that has it (`override`, `hak`).
    fn workshop(&self, sub: &str) -> Vec<PathBuf> {
        self.workshop_items().into_iter().map(|i| i.join(sub)).filter(|p| p.is_dir()).collect()
    }

    /// Where hak paks are, in the order the game searches: the user's
    /// `hak/`, the Workshop items' (when read), then the install's
    /// `data/hk/`.
    pub fn hak_dirs(&self) -> Vec<PathBuf> {
        self.user("hak")
            .into_iter()
            .chain(self.workshop("hak"))
            .chain([self.root.join("data").join("hk")])
            .collect()
    }

    /// The user's own hak folder (`hak/` in the user folder), where haks
    /// made or written go; `None` without a user folder.
    pub fn user_hak_dir(&self) -> Option<PathBuf> {
        self.user("hak")
    }

    /// The file of the hak `name` (without `.hak`, as
    /// [`crate::file_name_text`] reads a module's) the game opens: the
    /// first of [`GameInstall::hak_dirs`] that has [`crate::named_file`].
    pub fn find_hak(&self, name: &str) -> Option<PathBuf> {
        let file = crate::named_file(name, "hak");
        self.hak_dirs().into_iter().map(|d| d.join(&file)).find(|p| p.is_file())
    }

    /// The haks in the hak folders, by the names a module names them by
    /// ([`crate::name_of_file`]), sorted and without duplicates.
    pub fn hak_names(&self) -> Vec<String> {
        GameInstall::names_in(&self.hak_dirs(), "hak")
    }

    /// The talk tables in the talk table folders, by the names a module
    /// names them by, sorted and without duplicates.
    pub fn tlk_names(&self) -> Vec<String> {
        GameInstall::names_in(&self.tlk_dirs(), "tlk")
    }

    /// The names a module names the files of an extension in some folders
    /// by ([`crate::name_of_file`]: as they are, not lower case), sorted
    /// and without duplicates. The extension in its case where the file
    /// system has one ([`crate::CASE_IGNORED`]): the game opens `.hak`.
    pub fn names_in(dirs: &[PathBuf], ext: &str) -> Vec<String> {
        let ours = |e: &std::ffi::OsStr| match crate::CASE_IGNORED {
            true => e.eq_ignore_ascii_case(ext),
            false => e == ext,
        };
        let mut out: Vec<String> = (dirs.iter())
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(ours))
            .filter_map(|p| p.file_stem().map(crate::name_of_file))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Where the game finds no hak `name`: a file in the hak folders whose
    /// name differs from it in case alone ([`crate::Listed::OtherCase`]),
    /// which the game doesn't open on Linux. `None` where it finds one.
    pub fn hak_in_other_case(&self, name: &str) -> Option<PathBuf> {
        if self.find_hak(name).is_some() {
            return None;
        }
        let dirs = self.hak_dirs();
        let crate::Listed::OtherCase(other) =
            crate::listed(&GameInstall::names_in(&dirs, "hak"), name)
        else {
            return None;
        };
        let file = crate::named_file(&other, "hak");
        dirs.iter().map(|d| d.join(&file)).find(|p| p.is_file())
    }

    /// What every tool says of the hak `name` the game doesn't find (`mg`
    /// after `warning: `, the window's log with a capital): that it isn't
    /// found, and where a file of its name in another case is there, that
    /// file and what to do.
    pub fn hak_not_found(&self, name: &str) -> String {
        match self.hak_in_other_case(name) {
            Some(file) => format!(
                "hak {name} not found: the game opens a hak only by its exact name, and {} has \
                 its name in another case (rename the file or the module's hak to match)",
                file.display()
            ),
            None => format!("hak {name} not found"),
        }
    }

    /// Where custom talk tables are: the user's `tlk/`, then the install's
    /// `data/tlk/` (the premium campaigns' tables, beside their haks in
    /// `data/hk/`).
    pub fn tlk_dirs(&self) -> Vec<PathBuf> {
        self.user("tlk")
            .into_iter()
            .chain(self.workshop("tlk"))
            .chain([self.root.join("data").join("tlk")])
            .collect()
    }

    /// Where movies are: the user's `movies/`, then the install's.
    pub fn movie_dirs(&self) -> Vec<PathBuf> {
        self.user("movies").into_iter().chain([self.root.join("movies")]).collect()
    }

    /// The names (without extension, lower case) of the files with an
    /// extension in some directories, sorted and without duplicates.
    pub fn file_names(dirs: &[PathBuf], ext: &str) -> Vec<String> {
        let mut out: Vec<String> = dirs
            .iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// The game's number on Steam.
const STEAM_APP: &str = "704450";

/// Where the game may be: in each Steam library (Steam's own folder, and
/// those it lists on other drives), then where GOG puts it.
fn candidate_roots() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let game = Path::new("steamapps").join("common").join("Neverwinter Nights");
    let gog = "Neverwinter Nights Enhanced Edition";
    // Steam's own folders.
    let mut steam: Vec<PathBuf> = Vec::new();
    let mut other: Vec<PathBuf> = Vec::new();
    if cfg!(target_os = "linux") {
        if let Some(h) = &home {
            steam.push(h.join(".local/share/Steam"));
            steam.push(h.join(".steam/steam"));
            steam.push(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
            steam.push(h.join("snap/steam/common/.local/share/Steam"));
            other.push(h.join("GOG Games").join(gog));
        }
    } else if cfg!(target_os = "macos") {
        if let Some(h) = &home {
            steam.push(h.join("Library/Application Support/Steam"));
        }
    } else if cfg!(windows) {
        for pf in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = env::var_os(pf) {
                steam.push(PathBuf::from(&p).join("Steam"));
                other.push(PathBuf::from(p).join("GOG Galaxy").join("Games").join(gog));
            }
        }
        // Steam where its installer put it, as the registry says (a
        // folder of the user's choosing).
        steam.extend(steam_from_registry());
        // Steam, a library of its own or GOG's folder on any drive.
        for drive in 'C'..='Z' {
            let drive = PathBuf::from(format!("{drive}:\\"));
            steam.push(drive.join("Steam"));
            steam.push(drive.join("SteamLibrary"));
            other.push(drive.join("GOG Games").join(gog));
        }
    }
    // The libraries each lists (a game put on a second drive is in one).
    let mut libraries = steam.clone();
    for s in &steam {
        for list in [s.join("steamapps"), s.join("config")] {
            if let Ok(text) = std::fs::read_to_string(list.join("libraryfolders.vdf")) {
                libraries.extend(steam_libraries(&text));
            }
        }
    }
    // Beamdog's client: its library folders (its settings, else its
    // usual one), the stable release before the development one.
    let beamdog_settings = if cfg!(windows) {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| env::var_os("USERPROFILE").map(|h| Path::new(&h).join("AppData/Roaming")))
    } else if cfg!(target_os = "macos") {
        home.as_ref().map(|h| h.join("Library/Application Support"))
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join(".config")))
    }
    .map(|d| d.join("Beamdog Client").join("settings.json"));
    let beamdog_home =
        if cfg!(windows) { env::var_os("USERPROFILE").map(PathBuf::from) } else { home };
    let settings = beamdog_settings.and_then(|p| std::fs::read_to_string(p).ok());
    other.extend(beamdog_roots(settings.as_deref(), beamdog_home.as_deref()));
    let mut v: Vec<PathBuf> = Vec::new();
    for root in libraries.into_iter().map(|l| l.join(&game)).chain(other) {
        if !v.contains(&root) {
            v.push(root);
        }
    }
    v
}

/// The library folders a Steam `libraryfolders.vdf` lists: the value of
/// each `"path"` key (and, in the files of older Steam versions, of each
/// numbered key).
fn steam_libraries(vdf: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for line in vdf.lines() {
        // `"key"  "value"`, backslashes doubled.
        let mut quoted = line.split('"').skip(1).step_by(2);
        let (Some(key), Some(value)) = (quoted.next(), quoted.next()) else { continue };
        // (A numbered key is also an app's, with its size: a folder has a
        // slash in it.)
        let numbered = !key.is_empty()
            && key.chars().all(|c| c.is_ascii_digit())
            && value.contains(['/', '\\']);
        if (key.eq_ignore_ascii_case("path") || numbered) && !value.is_empty() {
            out.push(PathBuf::from(value.replace("\\\\", "\\")));
        }
    }
    out
}

/// Beamdog's numbers for the game: the stable release, then the
/// development one (neverwinter.nim's `findNwnRoot` looks for both).
const BEAMDOG_RELEASES: [&str; 2] = ["00785", "00829"];

/// Where Beamdog's client may have put the game: in each library folder
/// its `settings.json` lists (`"folders"`), else in its usual one
/// (`Beamdog Library` in the home folder), each release's folder.
fn beamdog_roots(settings: Option<&str>, home: Option<&Path>) -> Vec<PathBuf> {
    let listed: Vec<PathBuf> = settings
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|j| {
            j.get("folders")?
                .as_array()
                .map(|a| a.iter().filter_map(|f| f.as_str()).map(PathBuf::from).collect::<Vec<_>>())
        })
        .unwrap_or_default();
    let folders = if listed.is_empty() {
        home.map(|h| vec![h.join("Beamdog Library")]).unwrap_or_default()
    } else {
        listed
    };
    BEAMDOG_RELEASES.iter().flat_map(|r| folders.iter().map(move |f| f.join(r))).collect()
}

/// Steam's own folder as Windows' registry has it: the user's
/// `SteamPath`, then the machine's `InstallPath` (Steam installed where
/// its installer was told to). Elsewhere, none.
fn steam_from_registry() -> Vec<PathBuf> {
    if !cfg!(windows) {
        return Vec::new();
    }
    [
        (r"HKCU\Software\Valve\Steam", "SteamPath"),
        (r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
        (r"HKLM\SOFTWARE\Valve\Steam", "InstallPath"),
    ]
    .into_iter()
    .filter_map(|(key, name)| {
        let mut reg = std::process::Command::new("reg");
        reg.args(["query", key, "/v", name]);
        #[cfg(windows)]
        {
            // (No console window flashing up from the window's program.)
            use std::os::windows::process::CommandExt;
            reg.creation_flags(0x0800_0000);
        }
        let out = reg.output().ok().filter(|o| o.status.success())?;
        reg_value(&String::from_utf8_lossy(&out.stdout), name)
    })
    .map(PathBuf::from)
    .collect()
}

/// A value of `reg query KEY /v NAME`'s output (`    NAME    REG_SZ
/// VALUE`), its slashes as they are.
fn reg_value(output: &str, name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim_start();
        let rest = line.strip_prefix(name)?;
        let rest = rest.trim_start().strip_prefix("REG_")?;
        // (The type's name, then the value, which may hold spaces.)
        let (_, value) = rest.split_once(char::is_whitespace)?;
        Some(value.trim().to_string()).filter(|v| !v.is_empty())
    })
}

/// The `key=value` lines of an INI file's `[section]` (names compared
/// without case), in order; comments (`;`, `#`) and blank lines left out.
fn ini_section(text: &str, section: &str) -> Vec<(String, String)> {
    let mut inside = false;
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            inside = name.trim().eq_ignore_ascii_case(section);
        } else if inside
            && !line.starts_with(';')
            && !line.starts_with('#')
            && let Some((k, v)) = line.split_once('=')
        {
            out.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}

/// One value of an INI file's section, by name (without case).
fn ini_value(text: &str, section: &str, key: &str) -> Option<String> {
    ini_section(text, section)
        .into_iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
        .filter(|v| !v.is_empty())
}

/// The player's NWN folder where the game puts it on this system.
pub fn default_user_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var_os("USERPROFILE")
            .map(|h| PathBuf::from(h).join("Documents").join("Neverwinter Nights"))
    } else if cfg!(target_os = "macos") {
        env::var_os("HOME").map(|h| PathBuf::from(h).join("Documents/Neverwinter Nights"))
    } else {
        env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/Neverwinter Nights"))
    }
}

impl ResMan {
    /// The stack the toolset sees with no module open (see [`priority`]):
    /// portraits, `development/`, `override/`, ambient and music folders
    /// (with Steam Workshop items' own under the user's, when read) and
    /// the keys (`nwn_retail*` over `nwn_base*`, localized keys over their
    /// base). The install's `ovr/` is not searched: EE keeps `nwscript.nss`
    /// there for external tools only, and the engine resolves it from the keys.
    /// `data/txpk/*.erf` are empty stubs in EE; textures ship in the keys.
    ///
    /// Open a module with [`ResMan::add_haks`] and a layer at
    /// [`priority::MODULE`].
    pub fn for_game(install: &GameInstall) -> Result<ResMan, ResError> {
        let mut rm = ResMan::new();
        let root = &install.root;
        let data = root.join("data");
        // A Workshop item's folder of a kind, as a layer at `p` (each item
        // labelled by its number; of two items, the first by number first,
        // the game's order not being known), added before the install's
        // own of that priority: under the user's own, over the install's.
        let workshop = |rm: &mut ResMan, p: u32, sub: &str| {
            for item in install.workshop(sub) {
                let id = item.parent().and_then(Path::file_name).unwrap_or_default();
                let label = format!("workshop:{}", id.to_string_lossy());
                rm.add(p, label, LayerClass::Directory, DirContainer::open(&item));
            }
        };
        let dir = |rm: &mut ResMan, p: u32, label: &str, path: Option<PathBuf>| {
            if let Some(path) = path {
                rm.add(p, label, LayerClass::Directory, DirContainer::open(&path));
            }
        };
        dir(&mut rm, priority::PORTRAITS_USER, "user portraits", install.user("portraits"));
        workshop(&mut rm, priority::PORTRAITS, "portraits");
        dir(&mut rm, priority::PORTRAITS, "portraits", Some(data.join("prt")));
        dir(&mut rm, priority::DEVELOPMENT_USER, "development", install.user("development"));
        dir(&mut rm, priority::OVERRIDE, "override", install.user("override"));
        // The Workshop's overrides, under the user's own (the game started
        // through Steam reads them).
        workshop(&mut rm, priority::WORKSHOP, "override");
        dir(&mut rm, priority::AMBIENT_USER, "user ambient", install.user("ambient"));
        dir(&mut rm, priority::MUSIC_USER, "user music", install.user("music"));
        workshop(&mut rm, priority::AMBIENT, "ambient");
        dir(&mut rm, priority::AMBIENT, "ambient", Some(data.join("amb")));
        workshop(&mut rm, priority::MUSIC, "music");
        dir(&mut rm, priority::MUSIC, "music", Some(data.join("mus")));
        // userpatch.ini's haks, under the module and its haks, over
        // `override/`; the highest first (an archive that can't be read is
        // left out, as a missing one is).
        for (name, path) in install.userpatch_haks() {
            if let Some(c) = path.and_then(|p| ErfContainer::open(&p).ok()) {
                rm.add(priority::USERPATCH, format!("userpatch:{name}"), LayerClass::Erf, c);
            }
        }

        let lang_data = install.lang_data();
        for key in ["nwn_retail_loc", "nwn_retail", "nwn_base_loc", "nwn_base"] {
            let file = format!("{key}.key");
            let label = format!("key:{key}");
            if let Some(p) = Some(lang_data.join(&file)).filter(|p| p.is_file()) {
                rm.add(
                    priority::KEY,
                    label,
                    LayerClass::Key,
                    KeyContainer::open_localized(&p, root, &lang_data)?,
                );
            } else if let Some(p) = Some(data.join(&file)).filter(|p| p.is_file()) {
                rm.add(priority::KEY, label, LayerClass::Key, KeyContainer::open(&p, root)?);
            }
        }
        Ok(rm)
    }

    /// Adds a module's haks (`Mod_HakList` order, first highest), each from
    /// the user's `hak/` directory if it is there (priority
    /// [`priority::HAK_USER`]), else from the install's `data/hk/`
    /// ([`priority::HAK`]). Returns the names that were found nowhere.
    pub fn add_haks(
        &mut self,
        install: &GameInstall,
        names: &[impl AsRef<str>],
    ) -> Result<Vec<String>, ResError> {
        let mut missing = Vec::new();
        for name in names {
            let name = name.as_ref();
            let Some(path) = install.find_hak(name) else {
                missing.push(name.to_string());
                continue;
            };
            // (The install's own, `data/hk/`, under the user's; a
            // Workshop item's hak counts as the user's.)
            let stock = path.parent() == Some(install.root.join("data").join("hk").as_path());
            let p = if stock { priority::HAK } else { priority::HAK_USER };
            self.add(p, format!("hak:{name}"), LayerClass::Erf, ErfContainer::open(&path)?);
        }
        Ok(missing)
    }
}

#[cfg(test)]
mod install_tests {
    use super::*;

    /// Why a folder isn't the game, as a builder who chose it is told: the
    /// original game's (GitHub issue 39, the Diamond Edition), a folder
    /// inside the game's or above it, or none of those.
    #[test]
    fn a_folder_that_isn_t_the_game_says_why() {
        let base = env::temp_dir().join(format!("moonglow-why-not-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let ee = base.join("common").join("Neverwinter Nights");
        std::fs::create_dir_all(ee.join("data")).unwrap();
        std::fs::create_dir_all(ee.join("bin").join("win32")).unwrap();
        std::fs::write(ee.join("data").join("nwn_base.key"), b"KEY V1  ").unwrap();
        let diamond = base.join("NWN Diamond");
        std::fs::create_dir_all(&diamond).unwrap();
        std::fs::write(diamond.join("chitin.key"), b"KEY V1  ").unwrap();
        let why = |p: &Path| GameInstall::why_not(p).unwrap_or_default();
        assert_eq!(GameInstall::why_not(&ee), None);
        assert!(why(&diamond).contains("original game (1.69"), "{}", why(&diamond));
        let inside = why(&ee.join("bin").join("win32"));
        assert_eq!(inside, format!("this is a folder inside the game's: choose {}", ee.display()));
        let above = why(&base.join("common"));
        assert_eq!(above, format!("the game is in {}: choose that folder", ee.display()));
        assert_eq!(why(&base.join("nowhere")), "no data/nwn_base.key in it");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn steam_s_library_list_is_read() {
        // Steam's own, as it writes it now.
        let now = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/me/.local/share/Steam\"\n\
                   \t\t\"label\"\t\t\"\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"704450\"\t\t\"123\"\n\t\t}\n\t}\n\
                   \t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}\n";
        assert_eq!(
            steam_libraries(now),
            [PathBuf::from("/home/me/.local/share/Steam"), PathBuf::from("D:\\SteamLibrary")]
        );
        // And as older versions did: a path under each number.
        let old = "\"LibraryFolders\"\n{\n\t\"TimeNextStatsReport\"\t\"1\"\n\t\"1\"\t\t\"/mnt/games/Steam\"\n}\n";
        assert_eq!(steam_libraries(old), [PathBuf::from("/mnt/games/Steam")]);
        assert!(steam_libraries("not a list at all").is_empty());
    }

    /// Beamdog's client: the game in each library its settings list, the
    /// stable release before the development one; without its settings,
    /// in its usual library.
    #[test]
    fn beamdog_s_libraries_are_looked_in() {
        let settings = r#"{"folders": ["D:\\Games\\Beamdog Library", "/mnt/bd"], "other": 1}"#;
        let home = Path::new("/home/me");
        assert_eq!(
            beamdog_roots(Some(settings), Some(home)),
            [
                PathBuf::from("D:\\Games\\Beamdog Library").join("00785"),
                PathBuf::from("/mnt/bd/00785"),
                PathBuf::from("D:\\Games\\Beamdog Library").join("00829"),
                PathBuf::from("/mnt/bd/00829"),
            ]
        );
        let usual = [home.join("Beamdog Library/00785"), home.join("Beamdog Library/00829")];
        assert_eq!(beamdog_roots(None, Some(home)), usual);
        assert_eq!(beamdog_roots(Some("not json"), Some(home)), usual);
        assert_eq!(beamdog_roots(Some(r#"{"folders": []}"#), Some(home)), usual);
        assert!(beamdog_roots(None, None).is_empty());
    }

    /// Steam's folder from `reg query`'s output, spaces in it kept.
    #[test]
    fn the_registry_s_steam_folder_is_read() {
        let out = "\r\nHKEY_CURRENT_USER\\Software\\Valve\\Steam\r\n    SteamPath    REG_SZ    \
                   e:/my games/steam\r\n\r\n";
        assert_eq!(reg_value(out, "SteamPath").as_deref(), Some("e:/my games/steam"));
        let machine = "HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\Valve\\Steam\n    InstallPath    \
                       REG_SZ    E:\\Steam\n";
        assert_eq!(reg_value(machine, "InstallPath").as_deref(), Some("E:\\Steam"));
        assert_eq!(reg_value(machine, "SteamPath"), None);
        assert_eq!(reg_value("ERROR: The system was unable to find the key", "SteamPath"), None);
        assert!(steam_from_registry().is_empty() || cfg!(windows));
    }

    /// This machine's registry asked (run by hand on Windows, or under
    /// Wine with `wine reg add HKCU\Software\Valve\Steam /v SteamPath /t
    /// REG_SZ /d "c:/my games/steam"`): `MG_STEAM_EXPECTED` is what it
    /// should find first.
    #[test]
    #[ignore]
    fn steam_folder_from_this_machine_s_registry() {
        let found = steam_from_registry();
        eprintln!("{found:?}");
        if let Some(expected) = env::var_os("MG_STEAM_EXPECTED") {
            assert_eq!(found.first(), Some(&PathBuf::from(expected)));
        }
    }

    /// A Steam install's Workshop items are read when asked for: their
    /// overrides under the user's own, their haks found by name.
    #[test]
    fn workshop_items_are_read_when_asked_for() {
        let lib = env::temp_dir().join(format!("moonglow-workshop-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&lib);
        let root = lib.join("steamapps/common/Neverwinter Nights");
        let item = lib.join("steamapps/workshop/content/704450/123");
        let user = lib.join("user");
        for d in [root.join("data"), item.join("override"), item.join("hak"), user.join("override")]
        {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(item.join("override/mg_ws.2da"), "workshop").unwrap();
        std::fs::write(item.join("override/mg_both.2da"), "workshop").unwrap();
        std::fs::write(user.join("override/mg_both.2da"), "user").unwrap();
        std::fs::write(item.join("hak/mg_ws_hak.hak"), "").unwrap();
        // Its portraits, ambient sounds and music: under the user's, over
        // the install's.
        for (sub, name, ext) in [
            ("portraits", "po_mgws_m", mg_core::ResType::TGA),
            ("ambient", "al_mg_ws", mg_core::ResType::WAV),
            ("music", "mus_mg_ws", mg_core::ResType::BMU),
        ] {
            std::fs::create_dir_all(item.join(sub)).unwrap();
            let file = format!("{name}.{}", ext.extension().unwrap());
            std::fs::write(item.join(sub).join(file), sub).unwrap();
        }
        std::fs::create_dir_all(user.join("portraits")).unwrap();
        std::fs::write(user.join("portraits/po_mgws_m.tga"), "user").unwrap();
        let key = |n: &str| crate::ResKey::parse(n, mg_core::ResType::TWODA).unwrap();
        let mut install = GameInstall::new(&root, Some(user.clone()), "en");
        let without = ResMan::for_game(&install).unwrap();
        assert!(!without.contains(&key("mg_ws")));
        assert!(install.workshop_items().is_empty());
        install.workshop = true;
        let with = ResMan::for_game(&install).unwrap();
        assert_eq!(&*with.get(&key("mg_ws")).unwrap(), b"workshop");
        assert_eq!(&*with.get(&key("mg_both")).unwrap(), b"user", "the user's own comes first");
        assert!(install.hak_dirs().contains(&item.join("hak")));
        let res = |n: &str, t| crate::ResKey::parse(n, t).unwrap();
        let (tga, wav, bmu) = (mg_core::ResType::TGA, mg_core::ResType::WAV, mg_core::ResType::BMU);
        assert_eq!(&*with.get(&res("po_mgws_m", tga)).unwrap(), b"user", "the user's portrait");
        assert_eq!(&*with.get(&res("al_mg_ws", wav)).unwrap(), b"ambient");
        assert_eq!(&*with.get(&res("mus_mg_ws", bmu)).unwrap(), b"music");
        assert_eq!(with.origin(&res("mus_mg_ws", bmu)), Some("workshop:123"));
        std::fs::remove_file(user.join("portraits/po_mgws_m.tga")).unwrap();
        let with = ResMan::for_game(&install).unwrap();
        assert_eq!(&*with.get(&res("po_mgws_m", tga)).unwrap(), b"portraits");
        assert!(!without.contains(&res("mus_mg_ws", bmu)));
        // Not for a game outside a Steam library.
        let elsewhere = GameInstall { root: lib.join("game"), ..install };
        assert!(elsewhere.workshop_items().is_empty());
        let _ = std::fs::remove_dir_all(&lib);
    }

    /// userpatch.ini's haks are read when asked for, the highest number
    /// winning, from `nwn.ini`'s PATCH folder, over `override/`.
    #[test]
    fn userpatch_haks_are_read_when_asked_for() {
        let dir = env::temp_dir().join(format!("moonglow-userpatch-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("game");
        let user = dir.join("user");
        let patches = dir.join("elsewhere");
        for d in [root.join("data"), user.join("override"), user.join("patch"), patches.clone()] {
            std::fs::create_dir_all(d).unwrap();
        }
        let hak = |at: &Path, text: &str| {
            let mut w = mg_erf::ErfWriter::new(*b"HAK ");
            for n in ["mg_both", &format!("mg_{text}")] {
                w.add(
                    mg_core::ResRef::from_str(n).unwrap(),
                    mg_core::ResType::TWODA,
                    text.as_bytes(),
                )
                .unwrap();
            }
            std::fs::write(at, w.to_bytes().unwrap()).unwrap();
        };
        hak(&user.join("patch/low.hak"), "low");
        hak(&user.join("patch/high.hak"), "high");
        std::fs::write(user.join("override/mg_low.2da"), "override").unwrap();
        // Comments, blank lines, any case, the extension or none, a hak
        // that isn't there and another section's keys.
        std::fs::write(
            user.join("userpatch.ini"),
            "; mine\r\n[Other]\r\nPatchFile009=nope\r\n\r\n[PATCH]\r\nPatchFile000=low\r\n\
             patchfile002=high.hak\r\nPatchFile001=gone\r\n",
        )
        .unwrap();
        let key = |n: &str| crate::ResKey::parse(n, mg_core::ResType::TWODA).unwrap();
        let mut install = GameInstall::new(&root, Some(user.clone()), "en");
        assert!(install.userpatch_haks().is_empty(), "off unless asked for");
        assert!(!ResMan::for_game(&install).unwrap().contains(&key("mg_high")));
        install.userpatch = true;
        let names: Vec<_> =
            install.userpatch_haks().into_iter().map(|(n, p)| (n, p.is_some())).collect();
        assert_eq!(names, [("high".into(), true), ("gone".into(), false), ("low".into(), true)]);
        let rm = ResMan::for_game(&install).unwrap();
        assert_eq!(&*rm.get(&key("mg_both")).unwrap(), b"high", "the highest number wins");
        assert_eq!(&*rm.get(&key("mg_low")).unwrap(), b"low", "over override/");
        assert_eq!(rm.origin(&key("mg_high")), Some("userpatch:high"));
        // nwn.ini's PATCH alias names the folder.
        std::fs::rename(user.join("patch/high.hak"), patches.join("high.hak")).unwrap();
        std::fs::write(
            user.join("nwn.ini"),
            format!("[Alias]\nTLK=x\nPATCH={}\n[Game Options]\nPATCH=no\n", patches.display()),
        )
        .unwrap();
        let found: Vec<_> =
            install.userpatch_haks().into_iter().filter(|(_, p)| p.is_some()).collect();
        assert_eq!(found, [("high".into(), Some(patches.join("high.hak")))]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
