//! What Moonglow remembers between runs (saved by the desktop app).

use std::path::{Path, PathBuf};

use mg_resman::GameInstall;
use serde::{Deserialize, Serialize};

/// How many recent modules File > Recent Modules keeps.
pub const RECENT_MAX: usize = 10;

/// The script editor's syntax elements, in the order of
/// [`ScriptStyle::colors`] (Aurora's Options > Script Editor list).
pub const SCRIPT_ELEMENTS: [&str; 8] =
    ["Text", "Comment", "Directive", "Keyword", "Number", "String", "Constant", "Error"];

/// The script editor's look (Options > Script Editor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScriptStyle {
    /// Font size in points.
    pub font_size: u8,
    /// A colour (sRGB) for each of [`SCRIPT_ELEMENTS`]; `None` follows the
    /// light or dark theme.
    pub colors: [Option<[u8; 3]>; 8],
}

impl Default for ScriptStyle {
    fn default() -> ScriptStyle {
        ScriptStyle { font_size: 13, colors: [None; 8] }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Recently opened or saved modules, most recent first.
    pub recent: Vec<PathBuf>,
    /// The plugins enabled, by id (one installed is off until it is here).
    pub plugins_enabled: Vec<String>,
    /// The game install, when not the detected one.
    pub game_root: Option<PathBuf>,
    /// The NWN user folder (haks, override, modules), when not the detected
    /// one.
    pub user_dir: Option<PathBuf>,
    pub script_style: ScriptStyle,
    /// Options > General: Build module on save (Aurora's `Verify On Save`).
    pub build_on_save: bool,
    /// Options > General: Show areas by name: the module tree lists areas
    /// (and their tabs are titled) by their names rather than their
    /// ResRefs.
    pub area_names: bool,
    /// Options > General: Minimize Toolset on test module.
    pub minimize_on_test: bool,
    /// Options > General: Create backups of modules off (Aurora's default:
    /// on; the module as it was kept as `<name>.BackupMod` at each save).
    pub no_backups: bool,
    /// Options > General: Keep recovery copies of unsaved work, off (on by
    /// default; Moonglow's, Aurora has none).
    pub no_autosave: bool,
    /// Every so many minutes (`None`: 5).
    pub autosave_minutes: Option<u32>,
    /// Options > General: Reload haks, override and development when they
    /// change, off (on by default; Moonglow's).
    #[serde(default)]
    pub no_auto_reload: bool,
    /// Options > General: Show reserved Blueprint ResRef namespace warning,
    /// off.
    pub no_namespace_warning: bool,
    /// Options > General: Show invalid creature spell assignment warning,
    /// off (Aurora's on closing a creature's properties; Moonglow's on its
    /// Spells page).
    pub no_spell_warning: bool,
    /// Options > General: Show creature inventory warning, off (Aurora's
    /// notice on opening a creature's inventory; Moonglow's on its
    /// Inventory page).
    pub no_inventory_warning: bool,
    /// Options > General: Show resource in Hak Pak warning, off.
    pub no_hak_warning: bool,
    /// Options > General: Show standard resource overwrite warning, off.
    pub no_standard_warning: bool,
    /// Options > Script Editor: Automatically Compile Scripts on Save.
    pub auto_compile: bool,
    /// Options > Script Editor: Generate Debug Information When Compiling
    /// Scripts (`.ndb` next to the `.ncs`).
    pub debug_info: bool,
    /// Options > Script Editor: Code Templates Directory, listed with the
    /// game's (`data/scr`) and the user's `scripttemplates`.
    pub script_templates: Option<PathBuf>,
    /// Options > Script Editor: External Script Editor, a program given the
    /// script's file.
    pub external_editor: Option<PathBuf>,
    /// Options > Area: the area view's background colour (sRGB); `None`:
    /// the area's fog colour, as the game shows it (Aurora: silver grey,
    /// 0xC0C0C0).
    pub area_background: Option<[u8; 3]>,
    /// Options > Area: Show Encounter Spawnpoint Markers off (Aurora's
    /// default: on, height 12 and width 4).
    /// Options > Area: the spawn point markers' Height and Width, in
    /// tenths of a metre (`None`: Aurora's 12 and 4).
    pub spawn_marker_size: Option<(u8, u8)>,
    pub no_spawn_markers: bool,
    /// Options > Area: Show Door Orientation Arrows off (Aurora's default:
    /// on).
    pub no_door_arrows: bool,
    /// Options > Language: the language text is shown and edited in
    /// (language.2da row); `None`: the default, English.
    pub edit_language: Option<u32>,
    /// Options > Conversation Editor: Show speaker name before text
    /// (Aurora's default: shown).
    pub dialog_hide_names: bool,
    /// The conversation editor's Scripts toggle off: lines don't show the
    /// names of their conditions, actions, journal updates and sounds.
    pub dialog_hide_scripts: bool,
    /// The palettes' favorites (`utp:plc_chest1`), in the order added.
    pub palette_favorites: Vec<String>,
    /// The blueprints placed most recently, the last first.
    pub palette_recent: Vec<String>,
    /// Options › Keyboard: the keys chosen for commands, by command id
    /// ([`crate::keys::Cmd::id`]), where they aren't Moonglow's.
    pub key_bindings: std::collections::BTreeMap<String, Vec<String>>,
    /// Build › Publish to NWSync: the repository folder last written.
    pub nwsync_repository: Option<PathBuf>,
    /// Options > Conversation Editor: NPC and player text colours (sRGB);
    /// `None`: Moonglow's red and blue.
    pub dialog_npc_color: Option<[u8; 3]>,
    pub dialog_pc_color: Option<[u8; 3]>,
    /// Options > Sounds: Play placed sound objects in area, off (Aurora's
    /// default: on).
    pub no_placed_sounds: bool,
    /// Options > Sounds: Play ambient sound in area (Aurora's default: off).
    pub ambient_sound: bool,
    /// Options > Sounds: Play ambient music in area (Aurora's default: off).
    pub ambient_music: bool,
    /// The area view's snapping: moved and placed objects to this grid
    /// (centimeters), turned objects to this angle (degrees).
    #[serde(default)]
    pub snap_grid: Option<u16>,
    #[serde(default)]
    pub snap_angle: Option<u16>,
    /// Options > Sounds: Ambient music volume, of 127 (`None`: Aurora's 92).
    pub music_volume: Option<u8>,
    /// Options > Conversation Editor: Show popup when creating a new text
    /// entry, off (Aurora's default: on; Add asks for the new line's text).
    pub dialog_no_text_popup: bool,
    /// Options > Conversation Editor › Paste Link Options: Link Source To
    /// Destination (the copied line gets a link to the selected one); off:
    /// Aurora's default, Link Destination To Source (the selected line gets
    /// a link to the copied one).
    pub dialog_paste_source_to_dest: bool,
    /// Options > Conversation Editor › Drag Link Options: Link Destination
    /// To Source (the line dropped on gets a link to the dragged one); off:
    /// Aurora's default, Link Source To Destination.
    pub dialog_drag_dest_to_source: bool,
    /// Options > Conversation Editor: Automatically backup the conversation
    /// files, off (Aurora's default: on).
    pub dialog_no_backup: bool,
    /// The backup interval in minutes (`None`: Aurora's 5).
    pub dialog_backup_minutes: Option<u32>,
}

impl Settings {
    /// Puts a module at the top of the recent list.
    pub fn remember(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(RECENT_MAX);
    }

    /// The game install these settings choose: the configured folders, or
    /// what is detected.
    pub fn install(&self) -> Option<GameInstall> {
        let detected = GameInstall::detect();
        let root = match &self.game_root {
            Some(r) => r.clone(),
            None => detected.as_ref()?.root.clone(),
        };
        let user_dir = self.user_dir.clone().or_else(|| detected.and_then(|d| d.user_dir));
        Some(GameInstall::new(root, user_dir, "en"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_saved_before_script_styles_still_load() {
        let s: Settings = serde_json::from_str(r#"{"recent":["a.mod"]}"#).unwrap();
        assert_eq!(s.script_style, ScriptStyle::default());
        let mut t = s.clone();
        t.script_style.colors[3] = Some([1, 2, 3]);
        let back: Settings = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn recent_modules_are_most_recent_first_without_duplicates() {
        let mut s = Settings::default();
        for p in ["a.mod", "b.mod", "a.mod"] {
            s.remember(Path::new(p));
        }
        assert_eq!(s.recent, [PathBuf::from("a.mod"), PathBuf::from("b.mod")]);
        for i in 0..20 {
            s.remember(Path::new(&format!("{i}.mod")));
        }
        assert_eq!(s.recent.len(), RECENT_MAX);
        assert_eq!(s.recent[0], PathBuf::from("19.mod"));
    }
}
