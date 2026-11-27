//! The game's data for a module, as the game reads it: the install's
//! resources and `dialog.tlk`, the module's haks over them, the module
//! on top, and its custom talk table ([`Module::game_data`]). Every tool
//! that reads a module with the game's tables puts it together so, and
//! says what it lacks ([`ForModule::warnings`]) in the same words.

use mg_resman::{GameInstall, ResError};
use mg_rules::{GameData, RulesError};
use thiserror::Error;

use crate::talk::{self, Custom};
use crate::{Module, ModuleError};

/// What stops the game's data for a module being put together.
#[derive(Debug, Error)]
pub enum GameDataError {
    /// The game itself: its keys, its `dialog.tlk`.
    #[error("{0}")]
    Game(#[from] RulesError),
    /// The module's hak list can't be read.
    #[error("{0}")]
    Module(#[from] ModuleError),
}

/// The game's data for a module, and what of it wasn't found.
pub struct ForModule {
    pub game: GameData,
    /// The haks the module lists that are found nowhere the game looks.
    pub missing_haks: Vec<String>,
    /// What is said of each of them ([`GameInstall::hak_not_found`]: a
    /// file of its name in another case, where there is one).
    pub missing_said: Vec<String>,
    /// What went wrong opening a hak that is there (the module is put
    /// over the haks opened before it, and over the game).
    pub wrong_haks: Option<ResError>,
    /// The custom talk table, as it was looked for: the game data has it
    /// where it is [`Custom::Found`].
    pub talk: Custom,
    /// The module's NWSync content, where its project names a server.
    pub nwsync: Option<crate::nwsync_content::Mounted>,
}

impl std::fmt::Debug for ForModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (f.debug_struct("ForModule"))
            .field("missing_haks", &self.missing_haks)
            .field("wrong_haks", &self.wrong_haks)
            .field("talk", &self.talk.name())
            .finish_non_exhaustive()
    }
}

impl ForModule {
    /// What the game data lacks, each as every tool says it (`mg` after
    /// `warning: `): a hak not found, a hak that can't be opened, a custom
    /// talk table not found or that can't be read.
    pub fn warnings(&self) -> Vec<String> {
        let haks = self.missing_said.iter().cloned();
        let wrong = self.wrong_haks.iter().map(|e| format!("the module's haks: {e}"));
        let nwsync = self.nwsync.iter().flat_map(|n| n.warnings());
        haks.chain(wrong).chain(nwsync).chain(self.talk.problem()).collect()
    }
}

impl Module {
    /// The game's data for the module ([`ForModule`]): the install as
    /// `install` reads it (its language's `dialog.tlk`; the user's
    /// `override/`, userpatch haks and the Workshop as it is set), the
    /// module's haks (the user's, else the install's), the module, and its
    /// custom talk table found as the game finds it
    /// ([`talk::of_module`], in the language of the `dialog.tlk` read).
    /// An error where the game can't be read or the module's `module.ifo`
    /// can't be.
    pub fn game_data(&self, install: &GameInstall) -> Result<ForModule, GameDataError> {
        self.game_data_over(GameData::open(install)?, install)
    }

    /// [`game_data`](Self::game_data) as a player who downloads the module
    /// has the game ([`GameInstall::as_player`]): no user `override/` or
    /// development folder, no userpatch haks, no Workshop; the haks found
    /// where `install` finds them. What a release of the module reaches.
    pub fn player_game_data(&self, install: &GameInstall) -> Result<ForModule, GameDataError> {
        self.game_data_over(GameData::open(&install.as_player())?, install)
    }

    /// The module's haks, the module and its custom talk table over a
    /// game's data already read (its own `dialog.tlk`: a test's).
    pub fn game_data_over(
        &self,
        mut game: GameData,
        install: &GameInstall,
    ) -> Result<ForModule, GameDataError> {
        // (A module with no module.ifo, as `mg model-files` has without
        // one, has no haks; one that can't be read stops it.)
        if self.contains(&Module::info_key()) {
            self.haks()?;
        }
        let crate::Stacked { missing: missing_haks, wrong: wrong_haks, nwsync } =
            self.stack(&mut game.resman, install);
        let found = talk::of_module(self, &game.resman, &install.tlk_dirs(), Some(game.language));
        let (talk, tlk) = found.read();
        game.set_custom_tlk(tlk);
        game.invalidate();
        let missing_said = missing_haks.iter().map(|h| install.hak_not_found(h)).collect();
        Ok(ForModule { game, missing_haks, missing_said, wrong_haks, talk, nwsync })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::{Language, ResType};
    use mg_gff::{Gff, Value};
    use mg_resman::ResKey;
    use mg_schema::{ExoString, StructExt, ifo};

    /// An install of the bare minimum (a key with no files, a
    /// `dialog.tlk`), a user directory with `hak/` and `tlk/`.
    fn install(dir: &std::path::Path) -> GameInstall {
        let root = dir.join("game");
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(root.join("data/nwn_base.key"), mg_key_bytes()).unwrap();
        let lang = root.join("lang/en/data");
        std::fs::create_dir_all(&lang).unwrap();
        let mut tlk = mg_tlk::Tlk::new(Language::ENGLISH);
        tlk.entries.push(mg_tlk::TlkEntry::text("Bad Strref"));
        std::fs::write(lang.join("dialog.tlk"), tlk.to_bytes().unwrap()).unwrap();
        let user = dir.join("user");
        std::fs::create_dir_all(user.join("hak")).unwrap();
        std::fs::create_dir_all(user.join("tlk")).unwrap();
        GameInstall::new(&root, Some(user), "en")
    }

    /// A KEY file with no BIFs and no resources.
    fn mg_key_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"KEY V1  ");
        for v in [0u32, 0, 64, 64] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&[0; 40]);
        b
    }

    fn module(haks: &[&str], tlk: &str) -> Module {
        let mut m = Module::new();
        let mut info = Gff::new(*b"IFO ");
        info.root.write(&ifo::MOD_CUSTOM_TLK, ExoString(tlk.into()));
        let list = haks
            .iter()
            .map(|h| {
                let mut item = ifo::MOD_HAK_LIST.new_item();
                item.write(&ifo::mod_hak_list::MOD_HAK, ExoString(h.as_bytes().to_vec()));
                item
            })
            .collect();
        info.root.set("Mod_HakList", Value::List(list));
        m.set_info(&info).unwrap();
        m
    }

    #[test]
    fn a_module_s_haks_and_talk_table_over_the_game() {
        let dir = mg_testkit::scratch_dir("game-data-for-a-module");
        let gi = install(&dir);
        let mut custom = mg_tlk::Tlk::new(Language::ENGLISH);
        custom.entries.push(mg_tlk::TlkEntry::text("Mine"));
        std::fs::write(dir.join("user/tlk/mine.tlk"), custom.to_bytes().unwrap()).unwrap();
        let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
        let x = ResKey::parse("x", ResType::TWODA).unwrap();
        hak.add(x.resref, x.restype, b"2DA V2.0\n\n A\n0 1\n".to_vec()).unwrap();
        std::fs::write(dir.join("user/hak/there.hak"), hak.to_bytes().unwrap()).unwrap();

        let m = module(&["there", "gone"], "mine");
        let got = m.game_data(&gi).unwrap();
        assert_eq!(got.missing_haks, ["gone"]);
        assert!(got.wrong_haks.is_none());
        assert!(matches!(got.talk, Custom::Found { .. }));
        assert_eq!(got.game.custom_tlk().map(|t| t.entries.len()), Some(1));
        assert!(got.game.resman.contains(&x));
        assert_eq!(got.warnings(), ["hak gone not found"]);

        // A table found nowhere is said as every tool says it.
        let got = module(&[], "nowhere").game_data(&gi).unwrap();
        assert!(got.game.custom_tlk().is_none());
        let said = got.warnings();
        assert_eq!(said.len(), 1);
        assert!(said[0].starts_with("custom talk table nowhere not found"), "{said:?}");

        // And one that is no talk table.
        std::fs::write(dir.join("user/tlk/broken.tlk"), b"not a table").unwrap();
        let got = module(&[], "broken").game_data(&gi).unwrap();
        assert!(matches!(got.talk, Custom::Unreadable { .. }));
        assert!(got.warnings()[0].starts_with("custom talk table broken: "));

        // A player's game has no user override; the haks are still found.
        let got = module(&["there"], "").player_game_data(&gi).unwrap();
        assert!(got.missing_haks.is_empty());
        assert!(got.game.resman.layers().iter().all(|l| l.label != "override"));
        assert!(matches!(got.talk, Custom::None));
    }

    /// A manifest the server built with the module's own resources
    /// (`includes_module_contents`): those are left out of the layer, so
    /// the module being edited is read, not the server's copy; where the
    /// game's metadata doesn't say, they stay.
    #[test]
    fn a_manifest_built_with_the_module_leaves_its_resources_out() {
        use mg_core::sha1::sha1;
        use mg_nwsync::fixtures::{Registered, write_metadata, write_storage};
        for (says, read) in [(Some(true), &b"module"[..]), (None, &b"server"[..])] {
            let dir = mg_testkit::scratch_dir("game-data-nwsync-with-module");
            let gi = install(&dir);
            let own = ResKey::parse("mg_own", ResType::NSS).unwrap();
            let theirs = b"server".to_vec();
            let entries = [(own, sha1(&theirs), theirs.len() as u32)];
            let manifest = mg_nwsync::manifest::write(&entries);
            let url = "https://nwsync.example.org";
            let storage = dir.join("user/nwsync");
            write_storage(
                &storage,
                &[Registered {
                    sha1: sha1(&manifest),
                    origin_url: url.into(),
                    created_at: 1,
                    last_used_at: 1,
                    resources: vec![(own, sha1(&theirs))],
                    cached: None,
                }],
                &[vec![(sha1(&theirs), theirs.clone())]],
            );
            write_metadata(&storage, &sha1(&manifest), says);
            let mut m = module(&["gone"], "");
            m.set(own, b"module".to_vec());
            m.project_file.nwsync_source = Some(url.into());
            m.project_file.nwsync_cache = Some(dir.join("cache"));
            let got = m.game_data(&gi).unwrap();
            assert!(got.nwsync.as_ref().unwrap().layer.is_ok());
            assert_eq!(&*got.game.resman.get(&own).unwrap(), read, "{says:?}");
        }
    }

    /// A module whose project names an NWSync server, with the game's
    /// storage holding the server's manifest (a 2DA a hak on disk has
    /// too, and the custom talk table): its content over the haks, the
    /// talk table read from it, and the haks not on disk not missing (all
    /// of them without an origins file; with one, those it names). With
    /// no manifest of the server here, the haks are missing and that is
    /// said. A hak or talk table on disk newer than the manifest is read
    /// over it, and said (Moonglow's rule).
    #[test]
    fn a_module_s_nwsync_content_over_its_haks() {
        use mg_core::sha1::sha1;
        use mg_nwsync::fixtures::{Registered, write_storage};
        use std::time::{Duration, SystemTime, UNIX_EPOCH};
        let dir = mg_testkit::scratch_dir("game-data-nwsync");
        // (The manifest came a day from now: the files written now are
        // older than it.)
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let came = now + 86_400;
        let set_time = |path: &std::path::Path, secs: u64| {
            let f = std::fs::File::options().write(true).open(path).unwrap();
            f.set_modified(UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
        };
        let gi = install(&dir);
        let x = ResKey::parse("x", ResType::TWODA).unwrap();
        let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
        hak.add(x.resref, x.restype, b"2DA V2.0\n\n A\n0 hak\n".to_vec()).unwrap();
        std::fs::write(dir.join("user/hak/there.hak"), hak.to_bytes().unwrap()).unwrap();
        let synced = b"2DA V2.0\n\n A\n0 nwsync\n".to_vec();
        let mut tlk = mg_tlk::Tlk::new(Language::ENGLISH);
        tlk.entries.push(mg_tlk::TlkEntry::text("From NWSync"));
        let tlk = tlk.to_bytes().unwrap();
        let table = ResKey::parse("mine", ResType::TLK).unwrap();
        let entries = [(x, sha1(&synced), synced.len() as u32), (table, sha1(&tlk), 0)];
        let manifest = mg_nwsync::manifest::write(&entries);
        let url = "https://nwsync.example.org";
        let packed = mg_erf::compressedbuf::compress(&synced, *b"NSYC").unwrap();
        write_storage(
            &dir.join("user/nwsync"),
            &[Registered {
                sha1: sha1(&manifest),
                origin_url: url.into(),
                created_at: came as i64,
                last_used_at: came as i64,
                resources: entries.iter().map(|(k, s, _)| (*k, *s)).collect(),
                cached: None,
            }],
            &[vec![(sha1(&synced), packed), (sha1(&tlk), tlk.clone())]],
        );
        let mut m = module(&["gone1", "there", "gone2"], "mine");
        m.project_file.nwsync_source = Some(url.into());
        m.project_file.nwsync_cache = Some(dir.join("cache"));

        let got = m.game_data(&gi).unwrap();
        assert!(got.missing_haks.is_empty(), "{:?}", got.missing_haks);
        assert!(got.warnings().is_empty(), "{:?}", got.warnings());
        let n = got.nwsync.as_ref().unwrap();
        let layer = n.layer.as_ref().unwrap();
        assert_eq!(layer.from, mg_nwsync::ManifestFrom::GameStorage);
        assert_eq!(
            (layer.haks.clone(), layer.by_origins),
            (vec!["gone1".into(), "gone2".into()], false)
        );
        assert!(n.summary().unwrap().contains("the 2 haks not on disk taken to be in it"));
        // (Over the hak on disk, as the game has it.)
        assert_eq!(&*got.game.resman.get(&x).unwrap(), &synced[..]);
        assert_eq!(got.game.resman.origin(&x), Some(layer.label.as_str()));
        assert!(matches!(&got.talk, Custom::Found { .. }));
        let custom = got.game.custom_tlk().unwrap();
        assert_eq!(custom.entries[0].text, b"From NWSync");

        // The hak on disk changed after the manifest came: over it, and
        // said; the talk table on disk too.
        set_time(&dir.join("user/hak/there.hak"), came + 60);
        let mut mine = mg_tlk::Tlk::new(Language::ENGLISH);
        mine.entries.push(mg_tlk::TlkEntry::text("On disk"));
        std::fs::write(dir.join("user/tlk/mine.tlk"), mine.to_bytes().unwrap()).unwrap();
        set_time(&dir.join("user/tlk/mine.tlk"), came + 60);
        let got = m.game_data(&gi).unwrap();
        let layer = got.nwsync.as_ref().unwrap().layer.as_ref().unwrap();
        assert_eq!(layer.newer_haks, ["there"]);
        assert_eq!(layer.newer_tlk.as_deref(), Some("mine"));
        assert_eq!(&*got.game.resman.get(&x).unwrap(), b"2DA V2.0\n\n A\n0 hak\n");
        assert_eq!(got.game.resman.origin(&x), Some("hak:there"));
        assert_eq!(got.game.custom_tlk().unwrap().entries[0].text, b"On disk");
        assert_eq!(
            got.warnings(),
            [
                "hak there on disk is newer than the NWSync manifest: used over it",
                "talk table mine on disk is newer than the NWSync manifest: used over it",
            ]
        );
        // Older than the manifest: under it again, silently.
        set_time(&dir.join("user/hak/there.hak"), came - 60);
        set_time(&dir.join("user/tlk/mine.tlk"), came - 60);
        let got = m.game_data(&gi).unwrap();
        assert!(got.warnings().is_empty(), "{:?}", got.warnings());
        assert_eq!(&*got.game.resman.get(&x).unwrap(), &synced[..]);
        assert_eq!(got.game.custom_tlk().unwrap().entries[0].text, b"From NWSync");
        std::fs::remove_file(dir.join("user/tlk/mine.tlk")).unwrap();

        // The server's origins file names one of the two.
        let cache = mg_nwsync::Cache::new(dir.join("cache"));
        let origins = "Erf:/srv/nwsync/hak/gone1.hak\n\tx.2da\n";
        cache.put_manifest(&sha1(&manifest), "origin", origins.as_bytes()).unwrap();
        let got = m.game_data(&gi).unwrap();
        assert_eq!(got.missing_haks, ["gone2"]);
        assert_eq!(got.warnings(), ["hak gone2 not found"]);
        let layer = got.nwsync.as_ref().unwrap().layer.as_ref().unwrap();
        assert_eq!((layer.haks.clone(), layer.by_origins), (vec!["gone1".to_string()], true));

        // A module kept in one file, its server named beside it
        // (`NAME.moonglow.toml`): opened, its content there too.
        let path = dir.join("mod/krezk.mod");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut one = module(&["gone1", "there", "gone2"], "mine");
        one.save_as(&crate::ModuleLocation::Archive(path.clone())).unwrap();
        let beside = format!("[nwsync]\nsource = \"{url}\"\n");
        std::fs::write(dir.join("mod/krezk.moonglow.toml"), beside).unwrap();
        let one = Module::open(&path).unwrap();
        assert!(one.project_file.warnings.is_empty(), "{:?}", one.project_file.warnings);
        let got = one.game_data(&gi).unwrap();
        let layer = got.nwsync.as_ref().unwrap().layer.as_ref().unwrap();
        assert_eq!(layer.from, mg_nwsync::ManifestFrom::GameStorage);
        assert!(got.missing_haks.is_empty(), "{:?}", got.missing_haks);
        assert_eq!(&*got.game.resman.get(&x).unwrap(), &synced[..]);

        // No manifest of this server here.
        m.project_file.nwsync_source = Some("https://other.example.org".into());
        let got = m.game_data(&gi).unwrap();
        assert_eq!(got.missing_haks, ["gone1", "gone2"]);
        assert!(got.nwsync.as_ref().unwrap().layer.is_err());
        let said = got.warnings();
        assert!(said.iter().any(|w| w.contains("mg nwsync-pull")), "{said:?}");
        assert!(got.game.resman.layers().iter().all(|l| l.class != mg_resman::LayerClass::NwSync));
        // (The hak's copy, without the layer.)
        assert_ne!(&*got.game.resman.get(&x).unwrap(), &synced[..]);
    }
}
