//! What Moonglow remembers between runs (saved by the desktop app).

use std::path::{Path, PathBuf};

use mg_resman::GameInstall;
use serde::{Deserialize, Serialize};

/// How many recent modules File > Recent Modules keeps.
pub const RECENT_MAX: usize = 10;

/// How many modules' last areas are kept.
const LAST_AREAS_MAX: usize = 50;
/// How many areas a module keeps as opened lately.
const RECENT_AREAS_MAX: usize = 8;

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

/// The area view's placeable brush, as its strip of settings has it
/// (`paint_mode`): whole centimeters, percents and degrees, as the snapping
/// settings are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaintSettings {
    /// Lays a row (one piece end to end along a drag), rather than
    /// scattering in a circle.
    pub row: bool,
    /// Fills the rectangle dragged with pieces in rows and columns (a
    /// floor), rather than scattering or laying one row.
    pub fill: bool,
    /// The circle's radius, in centimeters.
    pub radius_cm: u16,
    /// How many placeables a tile's ground (10 by 10 meters) holds once
    /// painted: the brush adds until the circle has its share.
    pub density: u16,
    /// The least distance between two placeables it puts down, and
    /// between them and the placeables already there, in centimeters.
    pub spacing_cm: u16,
    /// How much of what the circle lacks each dab adds, in percent.
    pub flow: u8,
    /// How far each placeable's facing strays from the brush's (Q and E
    /// turn that), in degrees: 360 is any way at all.
    pub turn_spread: u16,
    /// Each placeable's scale is picked between these (percent); static
    /// placeables take none.
    pub scale_min: u16,
    pub scale_max: u16,
    /// How far above the ground they stand (below, if less than zero), in
    /// centimeters.
    pub lift_cm: i16,
    /// The room between pieces of a row or a fill (overlap, if less than
    /// zero), in centimeters.
    pub gap_cm: i16,
}

impl PaintSettings {
    /// Whether a drag lays the pieces (Row, Fill), rather than a brush
    /// dabbing them (Scatter).
    pub fn by_drag(&self) -> bool {
        self.row || self.fill
    }
}

impl Default for PaintSettings {
    fn default() -> PaintSettings {
        PaintSettings {
            row: false,
            fill: false,
            radius_cm: 500,
            density: 6,
            spacing_cm: 150,
            flow: 25,
            turn_spread: 360,
            scale_min: 100,
            scale_max: 100,
            lift_cm: 0,
            gap_cm: 0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Recently opened or saved modules, most recent first.
    pub recent: Vec<PathBuf>,
    /// The area opened last in each module (its path and the area's
    /// ResRef), most recent first: opened again with the module.
    pub last_areas: Vec<(PathBuf, String)>,
    /// The areas opened lately in each module (its path and their
    /// ResRefs, most recent first): offered while no area is open.
    pub recent_areas: Vec<(PathBuf, Vec<String>)>,
    /// The area view's Lighting switch is off (an even working light), in
    /// views opened from now on.
    pub unlit_areas: bool,
    /// The area view's Sound Ranges switch is on, in views opened from
    /// now on.
    pub sound_ranges: bool,
    /// The area view's Animations switch is off: creatures and other
    /// placed objects hold still.
    pub still_objects: bool,
    /// The area view's Grass switch is off: no grass is drawn (in every
    /// area view).
    pub no_grass: bool,
    /// The 3D views' Shaders switch (the area view's toolbar, the model
    /// view's): the game's own shaders draw what they can, a material's
    /// custom ones among them (`docs/shaders-proposal.md`). `None`: as
    /// [`GAME_SHADERS_BY_DEFAULT`] has it, until the switch is used.
    pub game_shaders: Option<bool>,
    /// The kinds of objects the area view's toolbar hides (`Triggers`,
    /// `Encounters`…, as [`mg_area::ObjectKind::label`] names them, and
    /// `Start` for the start location's marker; `Merchants`, as 1.35 and
    /// earlier wrote it, is read as `Stores`): every area view, the open
    /// ones and those opened from now on, hides them, as in Aurora.
    pub hidden_in_areas: Vec<String>,
    /// Options › Verify: the groups Verify Module leaves out of its report
    /// (by [`mg_module::doctor::Group::id`]).
    pub verify_skip: Vec<String>,
    /// The folder each hak was built from (the hak's path, the folder's),
    /// most recent first: Update from Folder reads it again.
    pub hak_folders: Vec<(PathBuf, PathBuf)>,
    /// Options > General: Open a module on the area opened last is off.
    pub no_last_area: bool,
    /// Options > General: Ask before the window is closed is off.
    pub close_without_asking: bool,
    /// Options > General: Next Tab and Previous Tab (Ctrl+Tab) go through
    /// a pane's tabs in the order they were last used, not as they lie.
    pub tabs_by_recent: bool,
    /// Options > General: Write a debug log (`trace`) is off. (It is
    /// written unless switched off: what was done before a crash is there
    /// to send without having asked for it beforehand.)
    pub no_debug_log: bool,
    /// The size each kind of editor window was last left at (its
    /// `Tab::kind`, width and height in points): the next one opens so.
    pub window_sizes: Vec<(String, [u32; 2])>,
    /// Where each kind of window was last dragged to, when not a window
    /// of its own: `middle` (a tab of the main pane) or `palettes` (a tab
    /// of the palettes' pane). The next of its kind opens there.
    pub window_places: Vec<(String, String)>,
    /// The 2DA columns given a width of their own by dragging the edge of
    /// their name: each table's (by name, lowercase; the last changed
    /// first) columns and widths in points. The 2DA editor and the
    /// resource browser's read-only view show the table so.
    pub table_widths: Vec<(String, Vec<(String, u32)>)>,
    /// The plugins enabled, by id (one installed is off until it is here).
    pub plugins_enabled: Vec<String>,
    /// What each plugin's events were allowed, kept between runs: the
    /// plugin (its id and version), the question as it was asked, and the
    /// answer (`yes`, or `path:` and the folder chosen). Forgotten when
    /// the plugin is disabled, removed or installed anew.
    pub plugin_grants: Vec<(String, String, String)>,
    /// The game install, when not the detected one.
    pub game_root: Option<PathBuf>,
    /// The NWN user folder (haks, override, modules), when not the detected
    /// one.
    pub user_dir: Option<PathBuf>,
    /// The user folder given on the command line (`moonglow --user-dir
    /// DIR`): for this run only, chosen over [`Settings::user_dir`] and
    /// never saved.
    #[serde(skip)]
    pub user_dir_this_run: Option<PathBuf>,
    pub script_style: ScriptStyle,
    /// Options > General: Build module on save (Aurora's `Verify On Save`).
    pub build_on_save: bool,
    /// Options › General: saving a module that is a folder does not write
    /// it as a `.mod` beside the folder too (it does unless switched off,
    /// as Aurora does for a module directory).
    pub no_mod_beside_folder: bool,
    /// View: the module tree, the log and the palettes folded away.
    pub hide_tree: bool,
    pub hide_log: bool,
    pub hide_palettes: bool,
    /// The area view's Fade Geometry (0 never, 1 in object mode only,
    /// 2 always), in views opened from now on.
    pub fade_geometry: u8,
    /// The palettes show pictures (their Gallery switch), as last left.
    pub palette_gallery: bool,
    /// The palette shown last: `tiles`, `prefabs`, or a blueprint type's
    /// extension (`utc`); and whether it showed the module's blueprints.
    /// `None`: none chosen yet (a module opens on its tiles).
    pub palette_type: Option<String>,
    pub palette_custom: bool,
    /// How large the galleries' pictures are, points (their slider's;
    /// `None`: the usual size).
    pub gallery_tile: Option<u16>,
    /// Options > General: Show ResRefs beside names: `Name (resref)` in
    /// the module tree (where it lists by name) and the palettes.
    pub name_resrefs: bool,
    /// Options > General: Show challenge ratings in the creature palette,
    /// off (the default: shown).
    pub palette_no_cr: bool,
    /// Options > General: Light theme (the default: dark).
    pub light_theme: bool,
    /// Options > General: Interface size, per cent of the usual (`None`:
    /// 100): the whole interface, text and all, larger.
    pub ui_scale: Option<u16>,
    /// Options > General: Show areas by name: the module tree lists areas
    /// (and their tabs are titled) by their names rather than their
    /// ResRefs.
    pub area_names: bool,
    /// Options > General: a blueprint's Category list is in alphabetical
    /// order, rather than in the palette's (Palette › Categories…).
    pub categories_by_name: bool,
    /// Options > General: a window of tabs (Properties, conversations,
    /// scripts… dragged or opened out of the main window) moves when
    /// dragged anywhere in it; off, only by its tab bar.
    pub drag_windows_anywhere: bool,
    /// Options > General: Minimize Toolset on test module.
    pub minimize_on_test: bool,
    /// Options > General: Start a Steam copy through Steam for a test is
    /// off (the game is started bare: no overlay, no Workshop content).
    pub test_without_steam: bool,
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
    /// Options > Script Editor: a script opened from the module tree opens
    /// in the external script editor as well.
    pub scripts_external: bool,
    /// Options > General: an area left closed in the module tree stays
    /// closed when an object in it is selected (off, as Aurora: the
    /// selection opens it out to show the object).
    pub tree_keeps_areas_closed: bool,
    /// Options > General: each Properties window (a blueprint's, a placed
    /// object's) opens in a window of its own (off: as a tab of the
    /// window one is open in already).
    pub properties_own_windows: bool,
    /// Options > Conversation Editor: each conversation opens in a window
    /// of its own (off: as a tab of the window one is open in already).
    pub dialogs_own_windows: bool,
    /// Options > Conversation Editor: its text's size in points (`None`:
    /// the window's own).
    pub dialog_font_size: Option<u8>,
    /// Options > Script Editor: Open each script in a window of its own
    /// (off: a script opens as a tab of the window scripts are open in).
    pub scripts_own_windows: bool,
    /// Options > Script Editor: and not in Moonglow's own editor, where
    /// the script has a file of its own (a project's, a module folder's).
    pub scripts_external_only: bool,
    /// Options > Script Editor: Code Templates Directory, listed with the
    /// game's (`data/scr`) and the user's `scripttemplates`.
    pub script_templates: Option<PathBuf>,
    /// Options > Folders: the scratch folder, where To Scratch copies a
    /// script (with its compiled script) or an area as loose files (`None`:
    /// asked for the first time).
    pub scratch_dir: Option<PathBuf>,
    /// Options > Script Editor: External Script Editor, a program given the
    /// script's file.
    pub external_editor: Option<PathBuf>,
    /// Options > General: Read Steam Workshop content, off (the default:
    /// read, as the game started through Steam reads it).
    pub no_workshop: bool,
    /// Options > Script Editor: External Script Compiler, a program that
    /// compiles scripts in place of the built-in compiler (`None`: the
    /// built-in one), and its line of arguments (empty: the program's own).
    pub external_compiler: Option<PathBuf>,
    pub external_compiler_args: String,
    /// Options > Area: the area view's background colour (sRGB); `None`:
    /// the area's fog colour, as the game shows it (Aurora: silver grey,
    /// 0xC0C0C0).
    pub area_background: Option<[u8; 3]>,
    /// Options > Area: the spawn point markers' Height and Width, in
    /// tenths of a metre (`None`: Aurora's 12 and 4).
    pub spawn_marker_size: Option<(u8, u8)>,
    /// Options > Area: Show Encounter Spawnpoint Markers off (Aurora's
    /// default: on, height 12 and width 4).
    pub no_spawn_markers: bool,
    /// Options > Area: Show Door Orientation Arrows off (Aurora's default:
    /// on).
    pub no_door_arrows: bool,
    /// Options > Area: Show stores as the game's $ sign (its marker
    /// model, as Aurora shows them) rather than as Moonglow's arrow.
    /// (Named for Aurora's merchants: kept, so saved settings load.)
    pub merchant_signs: bool,
    /// Options > Area: Show the turning and tilt rings around selected
    /// objects is off.
    pub no_turn_ring: bool,
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
    /// The area view: several objects selected turn about their middle
    /// as one, keeping their places among themselves, not each about
    /// itself.
    #[serde(default)]
    pub turn_together: bool,
    /// The area view's placeable brush (Paint): its strip's settings.
    #[serde(default)]
    pub paint: PaintSettings,
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

/// Whether the game's shaders draw the 3D views where the Shaders switch
/// was never used. Off until the per-draw blocks and the compiles off the
/// UI thread (the shaders proposal's M3) keep a large area at the frame
/// budget; the owner's decision is on by default then.
pub const GAME_SHADERS_BY_DEFAULT: bool = false;

impl Settings {
    /// The Shaders switch: the game's own shaders draw the 3D views.
    pub fn game_shaders(&self) -> bool {
        self.game_shaders.unwrap_or(GAME_SHADERS_BY_DEFAULT)
    }

    /// Puts a module at the top of the recent list.
    pub fn remember(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(RECENT_MAX);
    }

    /// Notes `area` as the one opened last in the module at `module`.
    pub fn remember_area(&mut self, module: &Path, area: &str) {
        if self.last_area(module) == Some(area) {
            return;
        }
        let at = self.recent_areas.iter().position(|(m, _)| m == module);
        let mut areas = at.map(|at| self.recent_areas.remove(at).1).unwrap_or_default();
        areas.retain(|a| a != area);
        areas.insert(0, area.to_owned());
        areas.truncate(RECENT_AREAS_MAX);
        self.recent_areas.insert(0, (module.to_path_buf(), areas));
        self.recent_areas.truncate(LAST_AREAS_MAX);
        self.last_areas.retain(|(m, _)| m != module);
        self.last_areas.insert(0, (module.to_path_buf(), area.to_owned()));
        self.last_areas.truncate(LAST_AREAS_MAX);
    }

    /// Notes `folder` as what the hak at `hak` was built from.
    pub fn remember_hak_folder(&mut self, hak: &Path, folder: &Path) {
        self.hak_folders.retain(|(h, _)| h != hak);
        self.hak_folders.insert(0, (hak.to_path_buf(), folder.to_path_buf()));
        self.hak_folders.truncate(LAST_AREAS_MAX);
    }

    /// The folder the hak at `hak` was built from.
    pub fn hak_folder(&self, hak: &Path) -> Option<&Path> {
        self.hak_folders.iter().find(|(h, _)| h == hak).map(|(_, f)| f.as_path())
    }

    /// The areas opened lately in the module at `module`, most recent
    /// first.
    pub fn recent_areas(&self, module: &Path) -> &[String] {
        self.recent_areas.iter().find(|(m, _)| m == module).map_or(&[], |(_, a)| a.as_slice())
    }

    /// The area opened last in the module at `module`.
    pub fn last_area(&self, module: &Path) -> Option<&str> {
        self.last_areas.iter().find(|(m, _)| m == module).map(|(_, a)| a.as_str())
    }

    /// The game install these settings choose: the configured folders, or
    /// what is detected.
    pub fn install(&self) -> Option<GameInstall> {
        let detected = GameInstall::detect();
        let root = match &self.game_root {
            Some(r) => r.clone(),
            None => detected.as_ref()?.root.clone(),
        };
        let user_dir = (self.user_dir_this_run.clone())
            .or_else(|| self.user_dir.clone())
            .or_else(|| detected.and_then(|d| d.user_dir));
        // The game's own text (names and descriptions by StrRef, the
        // rules' names) in the language edited, where the install has it:
        // its talk table and the rest of its `lang/` folder. Else English.
        let code = mg_core::Language(self.edit_language.unwrap_or(0))
            .short_code()
            .filter(|c| root.join("lang").join(c).join("data/dialog.tlk").is_file())
            .unwrap_or("en");
        let mut install = GameInstall::new(root, user_dir, code);
        install.workshop = !self.no_workshop;
        Some(install)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's text is read in the language edited, where the install
    /// has that language.
    #[test]
    fn the_install_s_language_is_the_one_edited() {
        let Some(root) = mg_testkit::nwn_root() else {
            eprintln!("skipped: no game install");
            return;
        };
        let language = |edit: Option<u32>| {
            let s = Settings {
                game_root: Some(root.clone()),
                edit_language: edit,
                ..Default::default()
            };
            s.install().unwrap().language
        };
        assert_eq!(language(None), "en");
        // (A language the game has no folder for.)
        assert_eq!(language(Some(131)), "en");
        if root.join("lang/pl/data/dialog.tlk").is_file() {
            assert_eq!(language(Some(5)), "pl");
            // A description named by StrRef reads in Polish then (the
            // drow's, string 64101).
            let text = |edit: Option<u32>| {
                let s = Settings {
                    game_root: Some(root.clone()),
                    edit_language: edit,
                    ..Default::default()
                };
                let game = mg_rules::GameData::open(&s.install().unwrap()).unwrap();
                game.string(mg_core::StrRef(64101)).unwrap()
            };
            let (english, polish) = (text(None), text(Some(5)));
            assert!(english.contains("drow"), "{english}");
            assert_ne!(english, polish, "another language's text");
        }
    }

    /// `moonglow --user-dir DIR` chooses this run's user folder, over the
    /// one Options names, and is not saved with the settings.
    #[test]
    fn the_run_s_user_folder_is_chosen_and_not_saved() {
        let Some(root) = mg_testkit::nwn_root() else {
            eprintln!("skipped: no game install");
            return;
        };
        let dir = std::env::temp_dir();
        let s = Settings {
            game_root: Some(root),
            user_dir: Some(dir.join("options-folder")),
            user_dir_this_run: Some(dir.clone()),
            ..Default::default()
        };
        assert_eq!(s.install().unwrap().user_dir, Some(dir));
        let saved = serde_json::to_string(&s).unwrap();
        assert!(!saved.contains("user_dir_this_run"), "{saved}");
        let back: Settings = serde_json::from_str(&saved).unwrap();
        assert_eq!(back.user_dir_this_run, None);
        assert_eq!(back.user_dir, s.user_dir);
    }

    #[test]
    fn settings_saved_before_script_styles_still_load() {
        let s: Settings = serde_json::from_str(r#"{"recent":["a.mod"]}"#).unwrap();
        assert_eq!(s.script_style, ScriptStyle::default());
        let mut t = s;
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

    #[test]
    fn each_module_s_last_area_is_kept() {
        let mut s = Settings::default();
        s.remember_area(Path::new("a.mod"), "town");
        s.remember_area(Path::new("b.mod"), "cave");
        s.remember_area(Path::new("a.mod"), "inn");
        assert_eq!(s.last_area(Path::new("a.mod")), Some("inn"));
        assert_eq!(s.last_area(Path::new("b.mod")), Some("cave"));
        assert_eq!(s.last_area(Path::new("c.mod")), None);
        assert_eq!(s.last_areas.len(), 2);
        assert_eq!(s.recent_areas(Path::new("a.mod")), ["inn", "town"]);
        s.remember_area(Path::new("a.mod"), "town");
        assert_eq!(s.recent_areas(Path::new("a.mod")), ["town", "inn"]);
        assert!(s.recent_areas(Path::new("c.mod")).is_empty());
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }
}
