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
}

impl GameInstall {
    pub fn new(root: impl Into<PathBuf>, user_dir: Option<PathBuf>, language: &str) -> GameInstall {
        GameInstall { root: root.into(), user_dir, language: language.to_string(), workshop: false }
    }

    /// Whether a directory looks like a game install.
    pub fn is_install(root: &Path) -> bool {
        root.join("data").join("nwn_base.key").is_file()
    }

    /// Finds the install (`NWN_ROOT`, then the usual Steam locations) and the
    /// user directory (`NWN_HOME`, `NWN_USER_DIRECTORY`, then the platform
    /// default), in English.
    pub fn detect() -> Option<GameInstall> {
        let root = match env::var_os("NWN_ROOT") {
            Some(r) => Some(PathBuf::from(r)).filter(|p| Self::is_install(p)),
            None => candidate_roots().into_iter().find(|p| Self::is_install(p)),
        }?;
        let user_dir = env::var_os("NWN_HOME")
            .or_else(|| env::var_os("NWN_USER_DIRECTORY"))
            .map(PathBuf::from)
            .or_else(default_user_dir)
            .filter(|p| p.is_dir());
        Some(GameInstall { root, user_dir, language: "en".into(), workshop: false })
    }

    /// `lang/<language>/data`.
    pub fn lang_data(&self) -> PathBuf {
        self.root.join("lang").join(&self.language).join("data")
    }

    /// The talk table for the language (`dialog.tlk`, or `dialogf.tlk`).
    pub fn talk_table(&self, feminine: bool) -> PathBuf {
        self.lang_data().join(if feminine { "dialogf.tlk" } else { "dialog.tlk" })
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

/// Where the game may be: in each Steam library (Steam's own folder, and
/// those it lists on other drives), then where GOG puts it.
/// The game's number on Steam.
const STEAM_APP: &str = "704450";

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
        // Steam, a library of its own or GOG's folder on any drive (Steam
        // put elsewhere isn't found by its usual place).
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

fn default_user_dir() -> Option<PathBuf> {
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
    /// portraits, `development/`, `override/`, ambient and music folders and
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
        let mut dir = |p: u32, label: &str, path: Option<PathBuf>| {
            if let Some(path) = path {
                rm.add(p, label, LayerClass::Directory, DirContainer::open(&path));
            }
        };
        dir(priority::PORTRAITS_USER, "user portraits", install.user("portraits"));
        dir(priority::PORTRAITS, "portraits", Some(data.join("prt")));
        dir(priority::DEVELOPMENT_USER, "development", install.user("development"));
        dir(priority::OVERRIDE, "override", install.user("override"));
        // The Workshop's overrides, under the user's own (the game started
        // through Steam reads them; which of two items' wins isn't known:
        // the first by number here).
        for item in install.workshop("override").into_iter().rev() {
            let id = item.parent().and_then(Path::file_name).unwrap_or_default();
            let label = format!("workshop:{}", id.to_string_lossy());
            dir(priority::WORKSHOP, &label, Some(item));
        }
        dir(priority::AMBIENT_USER, "user ambient", install.user("ambient"));
        dir(priority::MUSIC_USER, "user music", install.user("music"));
        dir(priority::AMBIENT, "ambient", Some(data.join("amb")));
        dir(priority::MUSIC, "music", Some(data.join("mus")));

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
        names: &[&str],
    ) -> Result<Vec<String>, ResError> {
        let mut missing = Vec::new();
        for name in names {
            let file = format!("{name}.hak");
            // (A Workshop item's hak counts as the user's.)
            let user = install
                .user("hak")
                .into_iter()
                .chain(install.workshop("hak"))
                .map(|d| d.join(&file))
                .find(|p| p.is_file());
            let (p, path) = match user {
                Some(path) => (priority::HAK_USER, path),
                None => {
                    match Some(install.root.join("data/hk").join(&file)).filter(|p| p.is_file()) {
                        Some(path) => (priority::HAK, path),
                        None => {
                            missing.push(name.to_string());
                            continue;
                        }
                    }
                }
            };
            self.add(p, format!("hak:{name}"), LayerClass::Erf, ErfContainer::open(&path)?);
        }
        Ok(missing)
    }
}

#[cfg(test)]
mod install_tests {
    use super::*;

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
        let key = |n: &str| crate::ResKey::parse(n, mg_core::ResType::TWODA).unwrap();
        let mut install = GameInstall::new(&root, Some(user), "en");
        let without = ResMan::for_game(&install).unwrap();
        assert!(!without.contains(&key("mg_ws")));
        assert!(install.workshop_items().is_empty());
        install.workshop = true;
        let with = ResMan::for_game(&install).unwrap();
        assert_eq!(&*with.get(&key("mg_ws")).unwrap(), b"workshop");
        assert_eq!(&*with.get(&key("mg_both")).unwrap(), b"user", "the user's own comes first");
        assert!(install.hak_dirs().contains(&item.join("hak")));
        // Not for a game outside a Steam library.
        let elsewhere = GameInstall { root: lib.join("game"), ..install };
        assert!(elsewhere.workshop_items().is_empty());
        let _ = std::fs::remove_dir_all(&lib);
    }
}
