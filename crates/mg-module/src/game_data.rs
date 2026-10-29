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
    /// What went wrong opening a hak that is there (the module is put
    /// over the haks opened before it, and over the game).
    pub wrong_haks: Option<ResError>,
    /// The custom talk table, as it was looked for: the game data has it
    /// where it is [`Custom::Found`].
    pub talk: Custom,
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
        let haks = self.missing_haks.iter().map(|h| format!("hak {h} not found"));
        let wrong = self.wrong_haks.iter().map(|e| format!("the module's haks: {e}"));
        haks.chain(wrong).chain(self.talk.problem()).collect()
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
        let (missing_haks, wrong_haks) = self.stack_onto(&mut game.resman, install);
        let found = talk::of_module(self, &game.resman, &install.tlk_dirs(), Some(game.language));
        let (talk, tlk) = found.read();
        game.set_custom_tlk(tlk);
        game.invalidate();
        Ok(ForModule { game, missing_haks, wrong_haks, talk })
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
}
