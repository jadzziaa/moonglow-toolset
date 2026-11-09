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
    /// The editor's background (sRGB); `None` follows the theme (its text
    /// fields' ground).
    pub background: Option<[u8; 3]>,
    /// Tab indents with a tab, rather than spaces (the default).
    pub indent_tabs: bool,
    /// How many spaces a step of indentation is (and the column a tab
    /// counts to): four unless chosen.
    pub indent_width: u8,
}

impl Default for ScriptStyle {
    fn default() -> ScriptStyle {
        ScriptStyle {
            font_size: 13,
            colors: [None; 8],
            background: None,
            indent_tabs: false,
            indent_width: 4,
        }
    }
}

/// A variant's name, read as a name (RON's bare identifier, JSON's
/// string), and a struct's field's.
struct Name(String);

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Name, D::Error> {
        struct Text;
        impl serde::de::Visitor<'_> for Text {
            type Value = Name;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a name")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Name, E> {
                Ok(Name(v.to_string()))
            }
        }
        d.deserialize_identifier(Text)
    }
}

/// A setting that is one of a few names (a plain enum), read so that a
/// name this version doesn't know (a newer one's settings, read by an
/// older Moonglow kept beside it) is its default, rather than every
/// setting lost with it: the derived reading fails the whole file on one.
trait Named: Copy + Default + 'static {
    /// The type's name, as it is saved under.
    const TYPE: &'static str;
    /// Each one by the name it is saved as.
    const NAMES: &'static [(&'static str, Self)];
}

/// Reads a [`Named`] setting: the one of that name, else the default.
fn named<'de, D: serde::Deserializer<'de>, T: Named>(d: D) -> Result<T, D::Error> {
    use serde::de::{EnumAccess, VariantAccess};
    struct Of<T>(std::marker::PhantomData<T>);
    impl<'de, T: Named> serde::de::Visitor<'de> for Of<T> {
        type Value = T;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a {}", T::TYPE)
        }
        fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<T, A::Error> {
            let (Name(name), variant) = data.variant()?;
            variant.unit_variant()?;
            Ok(T::NAMES.iter().find(|(n, _)| *n == name).map_or_else(T::default, |(_, v)| *v))
        }
    }
    d.deserialize_enum(T::TYPE, &[], Of(std::marker::PhantomData))
}

/// Makes a plain enum of the settings [`Named`], and read as one: every
/// variant is named here (one left out does not compile).
macro_rules! named {
    ($type:ident: $($variant:ident),+) => {
        impl Named for $type {
            const TYPE: &'static str = stringify!($type);
            const NAMES: &'static [(&'static str, $type)] =
                &[$((stringify!($variant), $type::$variant)),+];
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<$type, D::Error> {
                // (Each variant has its name above.)
                let _ = |v: $type| match v {
                    $($type::$variant => {})+
                };
                named(d)
            }
        }
    };
}

named!(ExistingArchive: Ask, Append, Overwrite);
named!(ModuleFolders: Ask, Folder, Module);
named!(TalkReuse: Ask, Use, Add, File);
named!(FilesThere: Ask, Replace, Skip);
named!(MissingFeats: Ask, Add, Refuse);
named!(ShadowQuality: Fast, High);
named!(GraphicsBackend: Automatic, Vulkan, Direct3d12, Metal);

/// What File › Export… does with an archive that is there already
/// (Options › General): asks, appends to it or overwrites it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum ExistingArchive {
    #[default]
    Ask,
    Append,
    Overwrite,
}

/// What opening a `.mod` with a folder of its name beside it opens
/// (Options › General): Aurora asks, or with its Always open module
/// directories opens the folder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum ModuleFolders {
    #[default]
    Ask,
    Folder,
    Module,
}

/// Options › Language › When the talk table has the text: what a text
/// typed into a field does where the module's talk table has a line with
/// that text already (`talk_typed`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum TalkReuse {
    /// Asks: use that line, add another, or keep the text in the file.
    #[default]
    Ask,
    /// Uses that line.
    Use,
    /// Adds another line.
    Add,
    /// Keeps the text in the file, as Aurora writes it.
    File,
}

/// What to do with files that are there already where Moonglow writes
/// (Options › General › Questions): a hak's added files, a model's Export
/// Files, conversations or minimaps exported, NUI windows imported.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum FilesThere {
    /// Asks: replace them, skip them, or cancel.
    #[default]
    Ask,
    Replace,
    Skip,
}

/// What equipping a creature with an item it lacks the feats for does
/// (Options › General › Questions; Aurora asks, Yes adding the feats).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum MissingFeats {
    #[default]
    Ask,
    /// Adds the feats and equips it (Aurora's Yes).
    Add,
    /// Leaves it unequipped (Aurora's No).
    Refuse,
}

/// Options › Area › Shadows quality (and the area view's Shadows menu):
/// how the area view draws the game's shadows. Fast, a shadow map (its
/// edges as fine as the map's texels: blocky up close), cheap in every
/// area; High, stencil shadow volumes as the game casts them (edges as
/// sharp as the picture's, anti-aliased with it), dearer in the largest
/// areas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum ShadowQuality {
    #[default]
    Fast,
    High,
}

/// Options › General › Graphics backend: what Moonglow draws with, read
/// at its start (`apps/moonglow`, before the window has a device;
/// [`mg_render::backend::pick_adapter`]). Automatic is wgpu's choice, but
/// an AMD GPU on Windows through Direct3D 12 rather than Vulkan. (A name
/// this version doesn't know, a newer one's or OpenGL's of old, is
/// Automatic.)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum GraphicsBackend {
    #[default]
    Automatic,
    Vulkan,
    Direct3d12,
    Metal,
}

impl GraphicsBackend {
    /// The backend chosen (`None`: Automatic).
    pub fn backend(self) -> Option<wgpu::Backend> {
        match self {
            GraphicsBackend::Automatic => None,
            GraphicsBackend::Vulkan => Some(wgpu::Backend::Vulkan),
            GraphicsBackend::Direct3d12 => Some(wgpu::Backend::Dx12),
            GraphicsBackend::Metal => Some(wgpu::Backend::Metal),
        }
    }

    /// The choice of `backend` (Automatic: one that can't be chosen).
    pub fn of(backend: wgpu::Backend) -> GraphicsBackend {
        match backend {
            wgpu::Backend::Vulkan => GraphicsBackend::Vulkan,
            wgpu::Backend::Dx12 => GraphicsBackend::Direct3d12,
            wgpu::Backend::Metal => GraphicsBackend::Metal,
            _ => GraphicsBackend::Automatic,
        }
    }

    /// Its name in Options.
    pub fn name(self) -> &'static str {
        self.backend().map_or("Automatic", mg_render::backend::backend_name)
    }

    /// The choices on `platform`: Automatic, then the backends wgpu
    /// offers there.
    pub fn offered(platform: mg_render::backend::Platform) -> Vec<GraphicsBackend> {
        let all = [GraphicsBackend::Vulkan, GraphicsBackend::Direct3d12, GraphicsBackend::Metal];
        let here = platform.backends();
        std::iter::once(GraphicsBackend::Automatic)
            .chain(all.into_iter().filter(|c| c.backend().is_some_and(|b| here.contains(&b))))
            .collect()
    }
}

/// A backend that drew wrong on a GPU and driver
/// ([`Settings::graphics_failures`]): the GPU's PCI vendor and device IDs,
/// its driver's name and version, the backend (by its name, as
/// [`mg_render::backend::backend_name`] says it), when (Unix seconds) and
/// what found it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsFailure {
    pub vendor: u32,
    pub device: u32,
    /// The GPU's name, to show.
    pub adapter: String,
    pub driver: String,
    pub backend: String,
    pub when: u64,
    pub what: String,
}

impl GraphicsFailure {
    /// The failure of `info`'s adapter, found by `what`, now.
    pub fn of(info: &wgpu::AdapterInfo, what: &str) -> GraphicsFailure {
        GraphicsFailure {
            vendor: info.vendor,
            device: info.device,
            adapter: info.name.clone(),
            driver: mg_render::backend::driver_of(info),
            backend: mg_render::backend::backend_name(info.backend).to_string(),
            when: mg_core::time::unix_now(),
            what: what.to_string(),
        }
    }

    /// As [`mg_render::backend::pick_adapter`] passes it over (`None`: a
    /// backend this version doesn't know).
    pub fn failed(&self) -> Option<mg_render::backend::Failed> {
        use wgpu::Backend::*;
        let backend = [Vulkan, Dx12, Metal, Gl]
            .into_iter()
            .find(|b| mg_render::backend::backend_name(*b) == self.backend)?;
        Some(mg_render::backend::Failed {
            vendor: self.vendor,
            device: self.device,
            driver: self.driver.clone(),
            backend,
        })
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
    /// The circle takes the chosen blueprints' placeables out of the area
    /// (Erase), rather than painting them.
    pub erase: bool,
    /// What it puts down is locked (the area editor's Lock): later clicks
    /// and boxes pass through it.
    pub lock: bool,
    /// Density counts every placeable in the circle, not only the chosen
    /// blueprints'.
    pub count_all: bool,
    /// Spacing is kept between the placeables' sides (each one's size
    /// by its model's box), rather than between their middles.
    pub by_size: bool,
}

impl PaintSettings {
    /// Whether a drag lays the pieces (Row, Fill), rather than a brush
    /// dabbing them (Scatter) or taking them out (Erase).
    pub fn by_drag(&self) -> bool {
        !self.erase && (self.row || self.fill)
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
            erase: false,
            lock: false,
            count_all: false,
            by_size: false,
        }
    }
}

/// The fields of settings saved as RON text, each by its name with its
/// value's text as written; `None` where the text is no struct.
fn fields_of(text: &str) -> Option<Vec<(String, String)>> {
    use ron::value::RawValue;
    struct Fields(Vec<(String, String)>);
    impl<'de> Deserialize<'de> for Fields {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Fields, D::Error> {
            struct Each;
            impl<'de> serde::de::Visitor<'de> for Each {
                type Value = Fields;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("the settings")
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> Result<Fields, A::Error> {
                    let mut fields = Vec::new();
                    while let Some(Name(name)) = map.next_key()? {
                        let value: Box<RawValue> = map.next_value()?;
                        fields.push((name, value.get_ron().trim().to_string()));
                    }
                    Ok(Fields(fields))
                }
            }
            d.deserialize_struct("Settings", &[], Each)
        }
    }
    ron::from_str::<Fields>(text).ok().map(|f| f.0)
}

/// What reading the saved settings came to ([`Settings::read_eframe`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsRead {
    pub settings: Settings,
    /// The settings this version could not read, now at their defaults:
    /// each one's name, and why.
    pub unread: Vec<String>,
    /// Why none of the file could be read (the settings are the defaults).
    pub unreadable: Option<String>,
    /// The file as it was, kept beside it, where anything did not read.
    pub kept: Option<PathBuf>,
}

impl SettingsRead {
    /// What to say in the log, where anything did not read.
    pub fn problem(&self) -> Option<String> {
        let what = match (&self.unreadable, self.unread.as_slice()) {
            (Some(why), _) => format!(
                "The saved settings could not be read ({why}): every setting is at its default"
            ),
            (None, []) => return None,
            (None, unread) => format!(
                "{} of the saved settings could not be read by this version of Moonglow and \
                 {} at the default: {}",
                unread.len(),
                if unread.len() == 1 { "is" } else { "are" },
                unread.join(", ")
            ),
        };
        Some(match &self.kept {
            Some(kept) => format!("{what}. The file as it was is kept as {}", kept.display()),
            None => format!("{what}. A copy of the file as it was could not be kept"),
        })
    }
}

/// Keeps a copy of the file at `path` beside it, as
/// `<name>.unreadable-<time>` (Unix seconds): where it is. One that is
/// there with the same bytes already is that copy (a Moonglow that can't
/// read the file and is never closed in order makes one copy, not one a
/// start).
fn keep_aside(path: &Path) -> Option<PathBuf> {
    let bytes = std::fs::read(path).ok()?;
    let name = format!("{}.unreadable-", path.file_name()?.to_string_lossy());
    let dir = path.parent()?;
    let kept = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
        let is_copy = p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&name));
        is_copy && std::fs::read(p).is_ok_and(|had| had == bytes)
    });
    if kept.is_some() {
        return kept;
    }
    // (Never over another copy: a second one within the second is
    // numbered.)
    let now = mg_core::time::unix_now();
    for n in 1..100 {
        let to = match n {
            1 => dir.join(format!("{name}{now}")),
            n => dir.join(format!("{name}{now}-{n}")),
        };
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&to) {
            Ok(mut file) => {
                use std::io::Write;
                return file.write_all(&bytes).ok().map(|()| to);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        }
    }
    None
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
    /// The area view's Shadows switch is off (no sun or moon shadows), in
    /// views opened from now on.
    pub no_shadows: bool,
    /// The area view's Creature Shadows (in the Shadows button's menu) are
    /// off, in views opened from now on.
    pub no_creature_shadows: bool,
    /// The area view's Light Shadows (in the Shadows button's menu) are
    /// off, in views opened from now on.
    pub no_light_shadows: bool,
    /// Options › Area › Shadows quality (also in the Shadows button's
    /// menu): every area view's.
    pub shadow_quality: ShadowQuality,
    /// Options › Area › Largest textures: the most pixels a side the 3D
    /// views upload a texture at (its larger levels left out:
    /// [`mg_render::Gpu::set_texture_limit`]); `None`, as it is.
    pub texture_limit: Option<u16>,
    /// Options › Area › Area view resolution: the share of the screen's
    /// pixels the area views draw their pictures at, percent (shown
    /// stretched to the view); `None`, all of them.
    pub area_resolution: Option<u8>,
    /// Options › Area › Keep animating behind other windows: the 3D views'
    /// animations go on while another program's window has the focus
    /// (else they hold still there, sparing the GPU).
    pub animate_unfocused: bool,
    /// Options › Area › Middle drag moves the view: a middle drag moves
    /// the area view's camera, as Ctrl + drag does, and Alt + middle drag
    /// turns it (the two swapped). Off, as in Blender: a middle drag
    /// turns it.
    pub middle_drag_moves: bool,
    /// The area view's Sound Ranges switch is on, in views opened from
    /// now on.
    pub sound_ranges: bool,
    /// The area view's Minimap switch is on, in views opened from now on.
    pub area_minimap: bool,
    /// The area view's Animations switch is off: creatures and other
    /// placed objects hold still.
    pub still_objects: bool,
    /// The area view's Grass switch is off: no grass is drawn (in every
    /// area view).
    pub no_grass: bool,
    /// The area view's Tile Border switch is on: past the area's edge, the
    /// tileset's edge tiles five deep, darkened, as the game client draws
    /// them (in every area view). Off by default, as Aurora draws nothing
    /// there and the area's edge shows plainest so.
    pub tile_border: bool,
    /// The 3D views' Shaders switch (the area view's toolbar, the model
    /// view's): the game's own shaders draw what they can, a material's
    /// custom ones among them (`docs/shaders-proposal.md`). `None`: as
    /// [`GAME_SHADERS_BY_DEFAULT`] has it, until the switch is used.
    pub game_shaders: Option<bool>,
    /// The model viewer's Shadow switch: the model casts the sun's shadow
    /// on a ground under it that takes only shadows. `None`: on, until
    /// the switch is used.
    pub model_shadows: Option<bool>,
    /// The kinds of objects the area view's toolbar hides (`Triggers`,
    /// `Encounters`…, as [`mg_area::ObjectKind::label`] names them, and
    /// `Start` for the start location's marker; `Merchants`, as 1.35 and
    /// earlier wrote it, is read as `Stores`): every area view, the open
    /// ones and those opened from now on, hides them, as in Aurora. For
    /// this run only, as Aurora's (it shows every kind again when it
    /// starts: captured 2026-10-29), so an area never opens looking empty.
    #[serde(skip)]
    pub hidden_in_areas: Vec<String>,
    /// Options › Verify: what Verify Module leaves out of its report, by
    /// the words [`mg_module::doctor::Skip`] reads (groups, checks,
    /// categories of missing references, haks, folders, resources).
    pub verify_skip: Vec<String>,
    /// The folder each hak was built from (the hak's path, the folder's),
    /// most recent first: Update from Folder reads it again.
    pub hak_folders: Vec<(PathBuf, PathBuf)>,
    /// Options > General: Open a module on the area opened last is off.
    pub no_last_area: bool,
    /// Options > General: Open a module with the tabs it had when it was
    /// closed is off (`open_tabs`).
    pub no_reopen_tabs: bool,
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
    /// The projects' commands (moonglow.toml's `[commands]`) allowed to
    /// run: the project's folder (its canonical path), the command's name
    /// and its text as allowed (another text asks again). Kept here, not
    /// in the project's files, which a repository could carry.
    pub allowed_commands: Vec<(String, String, String)>,
    /// The values of plugins' settings of scope `user` (theirs for every
    /// module), by plugin id and setting: text, a whole number, true or
    /// false. Those of a project's scopes are in its files.
    pub plugin_settings:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, serde_json::Value>>,
    /// How long, in seconds, the handlers of an event that runs at once
    /// (a save, a paste, a wizard's Finish) may take, each plugin's
    /// together; 0: [`mg_plugin::EVENT_BUDGET`].
    pub plugin_event_seconds: u32,
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
    /// Options › General: a `.mod` opened with a folder of its name
    /// beside it (as Aurora keeps a module directory): which is opened.
    pub module_folders: ModuleFolders,
    /// Options › General: what Export does with an archive that is there
    /// already (its question's "Always do this" sets it too).
    pub export_existing: ExistingArchive,
    /// Options › General › Questions: files there already where Moonglow
    /// writes (the question's "Always do this" sets it too).
    pub files_there: FilesThere,
    /// Options › General › Questions: equipping an item a creature lacks
    /// the feats for.
    pub missing_feats: MissingFeats,
    /// Options › General › Questions: Ask before deleting from the
    /// module, unticked: what nothing else names is deleted at once (what
    /// something names is still asked about, the window saying what).
    pub delete_without_asking: bool,
    /// View: the module tree, the log and the palettes folded away.
    pub hide_tree: bool,
    pub hide_log: bool,
    pub hide_palettes: bool,
    /// The area view's Fade Geometry (0 never, 1 in object mode only,
    /// 2 always), in views opened from now on.
    pub fade_geometry: u8,
    /// The palettes show pictures (their Gallery switch), as last left.
    pub palette_gallery: bool,
    /// The tiles palette's Exact tile switch, as last left (off: a tile
    /// chosen in the Gallery is painted with the tiles around fitted to
    /// it; on, it goes in alone, nothing around it changed: #29; a group
    /// chosen goes in as it is too).
    pub exact_tiles: bool,
    /// The palette shown last: `tiles`, `prefabs`, or a blueprint type's
    /// extension (`utc`); and whether it showed the module's blueprints.
    /// `None`: none chosen yet (a module opens on its tiles).
    pub palette_type: Option<String>,
    pub palette_custom: bool,
    /// What was typed in the palettes' search, as last left.
    pub palette_search: String,
    /// The palettes' categories left open: each a blueprint type's
    /// extension, `custom` or `standard`, and the category's place in its
    /// palette (`utc/standard/2.0`). They open so at the next start.
    pub palette_open: Vec<String>,
    /// Each palette type's Group by, by its extension (`utp`): the id of
    /// one of the project's groupings (`[palette.<id>]` in moonglow.toml).
    /// None, or one the project doesn't offer there: by Category.
    pub palette_group_by: std::collections::BTreeMap<String, String>,
    /// The palette types (extensions) whose groups list their blueprints
    /// in one list, Then by Category off (on unless said).
    pub palette_flat: Vec<String>,
    /// How large the galleries' pictures are, points (their slider's;
    /// `None`: the usual size).
    pub gallery_tile: Option<u16>,
    /// How large Tile Properties' variants' pictures are, points (their
    /// slider's; `None`: the usual size).
    pub tile_variant_side: Option<u16>,
    /// The talk table editor's split: each table's (by its name, and
    /// `+f` with a feminine table; the last changed first) share of the
    /// tab given to the chosen line's text, thousandths.
    pub talk_splits: Vec<(String, u16)>,
    /// Options > General: Show ResRefs beside names: `Name (resref)` in
    /// the module tree (where it lists by name) and the palettes.
    pub name_resrefs: bool,
    /// Options > General: Show challenge ratings in the creature palette,
    /// off (the default: shown).
    pub palette_no_cr: bool,
    /// Options > General: Show armor and cloaks on a woman (the item's
    /// Appearance page's Shown on chooses it too): their models, and the
    /// inventory icons of armor in inventories, the palette and the item's
    /// page, are a woman's rather than a man's.
    pub armor_on_woman: bool,
    /// Options > General: Light theme (the default: dark).
    pub light_theme: bool,
    /// Options > General: Graphics backend (from the next start).
    pub graphics_backend: GraphicsBackend,
    /// The backends that drew wrong on this machine's GPU and driver (the
    /// startup check, or the area view's watch and the builder's restart):
    /// Automatic passes over them until the driver changes
    /// ([`mg_render::backend::pick_adapter`]). Options › General forgets
    /// them.
    pub graphics_failures: Vec<GraphicsFailure>,
    /// The lit check's notice that the game's shaders draw wrong, hidden
    /// (Don't Show Again) on a GPU, driver and backend: Moonglow draws with
    /// its own there without saying so under the toolbar, until the driver
    /// changes ([`crate::renderer_check::lit_found`]). Options › General
    /// shows it again.
    pub graphics_notices_hidden: Vec<GraphicsFailure>,
    /// Options > General: Toolbar buttons, Icons only: the main toolbar's
    /// and the area view's buttons show their glyphs alone, their names
    /// in their tips (the default: Icons and text).
    pub toolbar_icons_only: bool,
    /// Options › Area › Toolbar: the area view's toolbar's buttons left
    /// out, by name ([`crate::icons::AREA_TOOLS`]; an objects' filter as
    /// "Show Creatures"). A builder's ask: a toolbar of what is used.
    pub hidden_area_tools: Vec<String>,
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
    /// Every so many minutes (`None`: [`AUTOSAVE_MINUTES`]).
    pub autosave_minutes: Option<u32>,
    /// Options > Folders: Reload haks, override and development when they
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
    /// Options > General: Show duplicate tags warning (after a paste or an
    /// area's Copy…), on. Off by default: Aurora has none, and builders
    /// share tags on purpose (a spawn system's waypoints). A project
    /// whose moonglow.toml sets `[tags] paste_warning` follows that.
    pub warn_duplicate_tags: bool,
    /// Options > Script Editor: Automatically Compile Scripts on Save.
    pub auto_compile: bool,
    /// Options › Script Editor: Check for errors as you type, off (on by
    /// default).
    pub no_live_check: bool,
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
    /// Options > General: the module tree stays where it is when a tab is
    /// chosen, the area's row alone marked as the one in front (on, as
    /// Aurora: its tree follows nothing else); off, it goes to the tab's
    /// resource, whatever kind.
    pub tree_stays: bool,
    /// Options > General: the module tree leaves out its groups with
    /// nothing in them (Scripts (0)) and an area's kinds with nothing
    /// placed (Doors (0)); off, as Aurora lists them all.
    pub tree_hides_empty: bool,
    /// Options > General: the palettes leave out the categories with no
    /// blueprint in them or under them; off, as Aurora lists them all.
    pub palette_hides_empty: bool,
    /// Options > General: each Properties window (a blueprint's, a placed
    /// object's) opens in a window of its own (off: as a tab of the
    /// window one is open in already).
    pub properties_own_windows: bool,
    /// Options > General: Closing a window cancels its changes. A window
    /// with OK and Cancel closed other than by them (its tab's ×, Close,
    /// Close All, Ctrl+W) sets back what it changed, as its Cancel and
    /// Aurora's close button do (`blueprint::closing`). Off by default:
    /// closing keeps the changes, as OK does.
    pub close_cancels: bool,
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
    /// Options > Folders: Read Steam Workshop content, as the game
    /// started through Steam reads it. Off by default, as Aurora: an item
    /// subscribed to for play is not the module's (builders' asks; the
    /// setting was `no_workshop`, on by default, until 1.38, and is read
    /// anew, so it is off for everyone until turned on).
    pub read_workshop: bool,
    /// Options > Folders: Read userpatch.ini's haks, as the game reads
    /// them. Off by default, as Aurora (the same reason).
    pub read_userpatch: bool,
    /// Options > Folders: Follow nwn.ini's folder aliases, unticked (a
    /// builder whose user folder's nwn.ini names another install's
    /// folders). Followed by default, as the game follows them.
    pub no_nwn_aliases: bool,
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
    /// Options > Area: Allow editing tiles of groups and features (off: a
    /// group or feature is one thing, its tiles changed only with it, and
    /// no group goes over another; Jasperre's ask).
    pub edit_group_tiles: bool,
    /// Options > Area: Show the turning and tilt rings around selected
    /// objects is off.
    pub no_turn_ring: bool,
    /// Options > Area: Lights of any color and reach, as content written
    /// into a hak ([`mg_module::lights`]); off by default (the project
    /// file's `[lights] generated` wins).
    pub generated_lights: bool,
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
    /// File › Export… and Import…: the folder of the archive last written
    /// or read, where their file dialogs open next (before one: the user
    /// folder's `erf`, as Aurora's open).
    pub erf_folder: Option<PathBuf>,
    /// Options › Folders: the command that sends the NWSync repository to
    /// the web server, once for each part (`{folder}`, `{part}`), where
    /// the project names none (`[nwsync] upload`).
    pub nwsync_upload: Option<String>,
    /// It reads what the builder types (as a project command's `input`).
    pub nwsync_upload_input: bool,
    /// Publish to NWSync: upload after publishing and after Remove Unused
    /// Data.
    pub nwsync_upload_after: bool,
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
    /// The area view: what is placed goes onto the top of a placeable or
    /// door under the pointer (the toolbar's Onto Objects), as Drop onto
    /// Object sets it after; off, on the ground.
    #[serde(default)]
    pub place_onto_objects: bool,
    /// The area view's placeable brush (Paint): its strip's settings.
    #[serde(default)]
    pub paint: PaintSettings,
    /// Options > Sounds: Ambient music volume, of 127 (`None`: Aurora's 92).
    pub music_volume: Option<u8>,
    /// Options > Conversation Editor: Show popup when creating a new text
    /// entry, off (Aurora's default: on; Add asks for the new line's text).
    pub dialog_no_text_popup: bool,
    /// Options › Language › Put text typed into the talk table: a name, a
    /// description or a conversation line typed becomes a line of the
    /// module's custom talk table (`talk_typed`); off, Aurora's way, the
    /// text is the file's.
    pub talk_typed: bool,
    /// Options › Language › When the talk table has the text.
    pub talk_reuse: TalkReuse,
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
    /// The backup interval in minutes (`None`: [`DIALOG_BACKUP_MINUTES`], Aurora's).
    pub dialog_backup_minutes: Option<u32>,
    /// Options › Updates: Check for updates automatically, off (on by
    /// default: GitHub is asked for the latest release at each start and once a day).
    pub no_update_check: bool,
    /// When the automatic check last asked GitHub, UNIX seconds.
    pub update_checked: Option<u64>,
    /// Options › Updates: Install updates automatically (off): a newer
    /// release the automatic check finds is downloaded, verified and
    /// installed as Moonglow closes.
    pub update_auto_install: bool,
    /// Options › Updates: Include pre-releases (off): the check offers a
    /// newer pre-release too.
    pub update_prereleases: bool,
    /// The version the update notice is not to tell of again (Don't remind
    /// me about this version); a later one is told.
    pub update_skipped: Option<String>,
}

/// Whether the game's shaders draw the 3D views where the Shaders switch
/// was never used: on, by the owner's decision once the shaders
/// proposal's M3 kept a large area at the frame budget (its values in
/// arrays drawn as instances, its programs compiled off the UI thread and
/// kept on disk). A builder's own choice is kept whatever this says.
pub const GAME_SHADERS_BY_DEFAULT: bool = true;

/// Recovery copies of unsaved work every so many minutes, where
/// `autosave_minutes` is unset.
pub const AUTOSAVE_MINUTES: u32 = 5;

/// A conversation's backup every so many minutes, where
/// `dialog_backup_minutes` is unset (Aurora's 5).
pub const DIALOG_BACKUP_MINUTES: u32 = 5;

impl Settings {
    /// The minutes between recovery copies ([`AUTOSAVE_MINUTES`] unset).
    pub fn autosave_every(&self) -> u32 {
        self.autosave_minutes.unwrap_or(AUTOSAVE_MINUTES)
    }

    /// The minutes between a conversation's backups
    /// ([`DIALOG_BACKUP_MINUTES`] unset).
    pub fn dialog_backup_every(&self) -> u32 {
        self.dialog_backup_minutes.unwrap_or(DIALOG_BACKUP_MINUTES)
    }

    /// The settings kept under `key` in eframe's settings file (`app.ron`:
    /// a RON map of keys to RON text), read before eframe opens it, or
    /// `None` without the file or the key, or where they can't be read.
    /// ([`Settings::read_eframe`] is what the desktop app starts with:
    /// this is its strict reading, all or nothing.)
    pub fn read_eframe_file(path: &Path, key: &str) -> Option<Settings> {
        let text = std::fs::read_to_string(path).ok()?;
        let kv: std::collections::HashMap<String, String> = ron::from_str(&text).ok()?;
        ron::from_str(kv.get(key)?).ok()
    }

    /// The settings from their saved text (RON): as they are, or, where
    /// the whole does not read (a field of another type, a value this
    /// version can't hold: a newer Moonglow's, or a file changed by
    /// hand), each field that reads by itself, with the names of those
    /// that do not (and why), which are at their defaults. An error where
    /// the text is no settings at all (cut short, not RON).
    pub fn read_fields(text: &str) -> Result<(Settings, Vec<String>), String> {
        let whole = match ron::from_str::<Settings>(text) {
            Ok(settings) => return Ok((settings, Vec::new())),
            Err(e) => e,
        };
        // (Why the whole did not read says more than why its fields can't
        // be told apart.)
        let Some(fields) = fields_of(text) else { return Err(whole.to_string()) };
        let (mut read, mut unread) = (Vec::new(), Vec::new());
        for (name, value) in fields {
            let one = format!("{name}:{value}");
            match ron::from_str::<Settings>(&format!("({one})")) {
                Ok(_) => read.push(one),
                Err(e) => unread.push(format!("{name} ({})", e.code)),
            }
        }
        match ron::from_str::<Settings>(&format!("({})", read.join(","))) {
            Ok(settings) => Ok((settings, unread)),
            Err(e) => Err(e.to_string()),
        }
    }

    /// The settings as they are saved: RON text.
    pub fn saved_text(&self) -> String {
        ron::to_string(self).unwrap_or_default()
    }

    /// The settings' text as it is kept under `key` in eframe's settings
    /// file now, if it is there.
    pub fn text_in_eframe_file(path: &Path, key: &str) -> Option<String> {
        let text = std::fs::read_to_string(path).ok()?;
        ron::from_str::<std::collections::HashMap<String, String>>(&text).ok()?.remove(key)
    }

    /// These settings as this window saves them over what is in the file
    /// now (`saved`, another Moonglow's since this one last read or wrote
    /// it): each setting this window changed since `was` (its own, when
    /// it last read or saved them) is this window's, every other is as it
    /// was saved. Two Moonglows share one settings file, each writing it
    /// whole every half minute: without this, an option changed in one
    /// window is put back by the other's next save. Where `saved` does not
    /// read, these settings as they are.
    pub fn saved_over(&self, was: &Settings, saved: &str) -> String {
        let mine = self.saved_text();
        let by_name = |text: &str| -> Option<std::collections::HashMap<String, String>> {
            Some(fields_of(text)?.into_iter().collect())
        };
        let (Some(now), Some(was), Some(theirs)) =
            (fields_of(&mine), by_name(&was.saved_text()), by_name(saved))
        else {
            return mine;
        };
        let merged: Vec<String> = now
            .into_iter()
            .map(|(name, value)| {
                let changed = was.get(&name) != Some(&value);
                let kept = match theirs.get(&name) {
                    Some(theirs) if !changed => theirs,
                    _ => &value,
                };
                format!("{name}:{kept}")
            })
            .collect();
        // (What this version reads of it: a setting of theirs it can't
        // read is this window's own.)
        match Settings::read_fields(&format!("({})", merged.join(","))) {
            Ok((settings, unread)) if unread.is_empty() => settings.saved_text(),
            _ => mine,
        }
    }

    /// The settings the desktop app starts with: those kept under `key`
    /// in eframe's settings file (`app.ron`: a RON map of keys to RON
    /// text), read before eframe opens it (the graphics backend is needed
    /// before the window has a device). Without the file or the key, the
    /// defaults. Where the file is there and does not read, or some of
    /// its settings do not ([`Settings::read_fields`]), the file as it is
    /// is kept beside it first (`app.ron.unreadable-<time>`): eframe saves
    /// over it half a minute on, and what could not be read would be gone.
    pub fn read_eframe(path: &Path, key: &str) -> SettingsRead {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SettingsRead::default(),
            Err(e) => {
                let (unreadable, kept) = (Some(e.to_string()), keep_aside(path));
                return SettingsRead { unreadable, kept, ..Default::default() };
            }
        };
        let saved = match ron::from_str::<std::collections::HashMap<String, String>>(&text) {
            Ok(mut kv) => kv.remove(key),
            Err(e) => {
                let (unreadable, kept) = (Some(e.to_string()), keep_aside(path));
                return SettingsRead { unreadable, kept, ..Default::default() };
            }
        };
        let Some(saved) = saved else { return SettingsRead::default() };
        match Settings::read_fields(&saved) {
            Ok((settings, unread)) if unread.is_empty() => {
                SettingsRead { settings, ..Default::default() }
            }
            Ok((settings, unread)) => {
                SettingsRead { settings, unread, unreadable: None, kept: keep_aside(path) }
            }
            Err(e) => {
                SettingsRead { unreadable: Some(e), kept: keep_aside(path), ..Default::default() }
            }
        }
    }

    /// The backends recorded as failing ([`Settings::graphics_failures`]),
    /// as Automatic passes them over.
    pub fn failed_backends(&self) -> Vec<mg_render::backend::Failed> {
        self.graphics_failures.iter().filter_map(GraphicsFailure::failed).collect()
    }

    /// Records a backend's failure, in place of one of the same GPU,
    /// driver and backend.
    pub fn record_graphics_failure(&mut self, failure: GraphicsFailure) {
        let key = failure.failed();
        self.graphics_failures.retain(|f| f.failed() != key);
        self.graphics_failures.push(failure);
    }

    /// Hides the lit check's notice on a GPU, driver and backend
    /// ([`Settings::graphics_notices_hidden`]), in place of one of the same.
    pub fn hide_graphics_notice(&mut self, hidden: GraphicsFailure) {
        let key = hidden.failed();
        self.graphics_notices_hidden.retain(|f| f.failed() != key);
        self.graphics_notices_hidden.push(hidden);
    }

    /// Whether the lit check's notice is hidden on `info`'s GPU, driver and
    /// backend.
    pub fn graphics_notice_hidden(&self, info: &wgpu::AdapterInfo) -> bool {
        let here = mg_render::backend::Failed::of(info);
        self.graphics_notices_hidden.iter().any(|f| f.failed().as_ref() == Some(&here))
    }

    /// The Shaders switch: the game's own shaders draw the 3D views.
    pub fn game_shaders(&self) -> bool {
        self.game_shaders.unwrap_or(GAME_SHADERS_BY_DEFAULT)
    }

    /// The model viewer's Shadow switch ([`Settings::model_shadows`]).
    pub fn model_shadows(&self) -> bool {
        self.model_shadows.unwrap_or(true)
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
    /// what is detected. A folder kept with a leading `~` (typed so before
    /// Options expanded it) is the home folder's.
    pub fn install(&self) -> Option<GameInstall> {
        let detected = GameInstall::detect();
        let root = match &self.game_root {
            Some(r) => expand_home(r),
            None => detected.as_ref()?.root.clone(),
        };
        let user_dir = (self.user_dir_this_run.clone())
            .or_else(|| self.user_dir.as_deref().map(expand_home))
            .or_else(|| detected.and_then(|d| d.user_dir));
        // The game's own text (names and descriptions by StrRef, the
        // rules' names) in the language edited, where the install has it:
        // its talk table and the rest of its `lang/` folder. Else English.
        let code = mg_core::Language(self.edit_language.unwrap_or(0))
            .short_code()
            .filter(|c| root.join("lang").join(c).join("data/dialog.tlk").is_file())
            .unwrap_or("en");
        let mut install = GameInstall::new(root, user_dir, code);
        install.workshop = self.read_workshop;
        install.userpatch = self.read_userpatch;
        install.aliases = !self.no_nwn_aliases;
        Some(install)
    }
}

/// A folder typed with a leading `~` (`~/.local/share/Neverwinter
/// Nights`, as a shell takes it): the home folder's. Others as they are.
/// (A builder's report: kept as typed, every hak was "not found".)
pub fn expand_home(path: &Path) -> PathBuf {
    let home =
        || std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    match path.strip_prefix("~") {
        Ok(rest) => home().map_or_else(|| path.to_path_buf(), |h| h.join(rest)),
        Err(_) => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A leading `~` is the home folder; `~name` and a `~` further in are
    /// left as they are.
    #[test]
    fn a_folder_typed_with_a_tilde_is_the_home_folder_s() {
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
        let Some(home) = home.map(PathBuf::from) else { return };
        let nwn = Path::new("~").join(".local").join("share").join("Neverwinter Nights");
        assert_eq!(expand_home(&nwn), home.join(".local/share/Neverwinter Nights"));
        assert_eq!(expand_home(Path::new("~")), home);
        for kept in ["~other/nwn", "/games/~nwn", "relative"] {
            assert_eq!(expand_home(Path::new(kept)), PathBuf::from(kept));
        }
        let s = Settings { user_dir: Some(nwn), ..Default::default() };
        let user = s.install().and_then(|i| i.user_dir);
        assert!(user.as_ref().is_none_or(|u| u.starts_with(&home)), "{user:?}");
    }

    /// Workshop items and userpatch.ini's haks are not read unless asked
    /// for, as in Aurora (builders' asks); settings saved when the
    /// Workshop was read by default (`no_workshop: false`) don't read it.
    /// The area view's shadows by default (and for settings saved before
    /// they were): all three kinds on; each kept as chosen.
    #[test]
    fn the_shadow_kinds_default_and_are_kept() {
        let old: Settings = serde_json::from_str(r#"{"no_shadows":false}"#).unwrap();
        assert!(!old.no_shadows && !old.no_creature_shadows && !old.no_light_shadows);
        // (Fast, the shadow map, unless High is chosen.)
        assert_eq!(old.shadow_quality, ShadowQuality::Fast);
        let chosen = Settings {
            no_shadows: true,
            no_creature_shadows: true,
            no_light_shadows: true,
            shadow_quality: ShadowQuality::High,
            ..Settings::default()
        };
        let back: Settings =
            serde_json::from_str(&serde_json::to_string(&chosen).unwrap()).unwrap();
        assert!(back.no_shadows && back.no_creature_shadows && back.no_light_shadows);
        assert_eq!(back.shadow_quality, ShadowQuality::High);
    }

    /// The area view's Tile Border: off by default and for settings saved
    /// before it was; on, kept.
    #[test]
    fn the_tile_border_is_off_by_default_and_kept() {
        assert!(!Settings::default().tile_border);
        let old: Settings = serde_json::from_str(r#"{"no_grass":true}"#).unwrap();
        assert!(!old.tile_border);
        let on = Settings { tile_border: true, ..Settings::default() };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&on).unwrap()).unwrap();
        assert!(back.tile_border);
    }

    /// The largest textures are as they are unless a limit is chosen, and
    /// a limit chosen is kept.
    #[test]
    fn the_texture_limit_is_none_until_chosen_and_kept() {
        let old: Settings = serde_json::from_str(r#"{"no_shadows":false}"#).unwrap();
        assert_eq!(old.texture_limit, None);
        assert_eq!(old.area_resolution, None, "every pixel unless chosen");
        let chosen =
            Settings { texture_limit: Some(512), area_resolution: Some(50), ..Settings::default() };
        let back: Settings =
            serde_json::from_str(&serde_json::to_string(&chosen).unwrap()).unwrap();
        assert_eq!((back.texture_limit, back.area_resolution), (Some(512), Some(50)));
        // Animations hold still behind other windows unless kept going.
        assert!(!old.animate_unfocused);
        // A middle drag turns the area view unless asked to move it.
        assert!(!old.middle_drag_moves);
        let kept = Settings { animate_unfocused: true, ..Settings::default() };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&kept).unwrap()).unwrap();
        assert!(back.animate_unfocused);
    }

    /// Closing a window cancels its changes: off by default and for
    /// settings saved before it was; on, kept.
    #[test]
    fn closing_a_window_cancels_is_off_by_default_and_kept() {
        assert!(!Settings::default().close_cancels);
        let old: Settings = serde_json::from_str(r#"{"properties_own_windows":true}"#).unwrap();
        assert!(!old.close_cancels);
        let on = Settings { close_cancels: true, ..Settings::default() };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&on).unwrap()).unwrap();
        assert!(back.close_cancels);
    }

    #[test]
    fn workshop_and_userpatch_are_off_until_asked_for() {
        let s = Settings::default();
        assert!(!s.read_workshop && !s.read_userpatch);
        let old: Settings = serde_json::from_str(r#"{"no_workshop":false}"#).unwrap();
        assert!(!old.read_workshop);
    }

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
        // A description named by StrRef reads in the language then (the
        // drow's, string 64101), and the language's own archives
        // (`lang/<code>/data/nwn_base_loc.key`: its voices and
        // conversations) are read: each of the EE's languages installed.
        let game = |edit: Option<u32>| {
            let s = Settings {
                game_root: Some(root.clone()),
                edit_language: edit,
                ..Default::default()
            };
            mg_rules::GameData::open(&s.install().unwrap()).unwrap()
        };
        let english = game(None).string(mg_core::StrRef(64101)).unwrap();
        assert!(english.contains("drow"), "{english}");
        let mut seen = 0;
        for id in 1..=5 {
            let code = mg_core::Language(id).short_code().unwrap();
            if !root.join("lang").join(code).join("data/dialog.tlk").is_file() {
                continue;
            }
            seen += 1;
            assert_eq!(language(Some(id)), code);
            let game = game(Some(id));
            let text = game.string(mg_core::StrRef(64101)).unwrap();
            assert_ne!(english, text, "{code}: another language's text");
            let localized = game.resman.layers().iter().any(|l| l.label == "key:nwn_base_loc");
            let lang_key = root.join("lang").join(code).join("data/nwn_base_loc.key");
            assert_eq!(localized, lang_key.is_file(), "{code}: its own archives");
        }
        eprintln!("{seen} languages besides English");
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

    /// The graphics backend is read from eframe's file before the window
    /// opens: kept as eframe keeps the settings (RON text under a key of
    /// a RON map), Automatic where it isn't there or can't be read, and a
    /// name this version doesn't know is Automatic, the rest kept.
    #[test]
    fn the_graphics_backend_is_read_from_eframes_file() {
        let dir = mg_testkit::scratch_dir("settings-eframe-file");
        let path = dir.join("app.ron");
        let key = "moonglow-settings";
        let write = |settings: &str| {
            let kv = std::collections::HashMap::from([
                ("egui".to_string(), "(options:())".to_string()),
                (key.to_string(), settings.to_string()),
            ]);
            let pretty = ron::ser::PrettyConfig::default();
            std::fs::write(&path, ron::ser::to_string_pretty(&kv, pretty).unwrap()).unwrap();
        };
        let s = Settings {
            graphics_backend: GraphicsBackend::Direct3d12,
            light_theme: true,
            ..Default::default()
        };
        write(&ron::to_string(&s).unwrap());
        assert_eq!(Settings::read_eframe_file(&path, key), Some(s));
        assert_eq!(Settings::read_eframe_file(&path, "other"), None);
        assert_eq!(Settings::read_eframe_file(&dir.join("none.ron"), key), None);
        write("(graphics_backend:WebGpu,light_theme:true)");
        let newer = Settings::read_eframe_file(&path, key).unwrap();
        assert_eq!((newer.graphics_backend, newer.light_theme), (GraphicsBackend::Automatic, true));
        let json: Settings = serde_json::from_str(r#"{"graphics_backend":"WebGpu"}"#).unwrap();
        assert_eq!(json.graphics_backend, GraphicsBackend::Automatic);
        let json: Settings = serde_json::from_str(r#"{"graphics_backend":"Metal"}"#).unwrap();
        assert_eq!(json.graphics_backend, GraphicsBackend::Metal);
        // (OpenGL, no longer offered, reads as Automatic.)
        let gl: Settings = serde_json::from_str(r#"{"graphics_backend":"OpenGl"}"#).unwrap();
        assert_eq!(gl.graphics_backend, GraphicsBackend::Automatic);
        write("(graphics_backend:");
        assert_eq!(Settings::read_eframe_file(&path, key), None);
        std::fs::write(&path, "not ron").unwrap();
        assert_eq!(Settings::read_eframe_file(&path, key), None);
    }

    /// A plain enum's name this version doesn't know (a newer Moonglow's
    /// settings read by an older one) is that setting's default, and every
    /// other setting is kept: the derived reading lost them all.
    #[test]
    fn a_name_this_version_does_not_know_is_the_default() {
        let read = |text: &str| ron::from_str::<Settings>(text).unwrap();
        let s = read("(shadow_quality:Ultra,light_theme:true)");
        assert_eq!((s.shadow_quality, s.light_theme), (ShadowQuality::Fast, true));
        let newer = "(export_existing:Merge,module_folders:Newest,talk_reuse:Guess,\
                     files_there:Rename,missing_feats:Wait,light_theme:true)";
        assert_eq!(read(newer), Settings { light_theme: true, ..Default::default() });
        // Each name it does know reads as before, and as it is written.
        let all = Settings {
            export_existing: ExistingArchive::Overwrite,
            module_folders: ModuleFolders::Module,
            talk_reuse: TalkReuse::File,
            files_there: FilesThere::Skip,
            missing_feats: MissingFeats::Refuse,
            shadow_quality: ShadowQuality::High,
            graphics_backend: GraphicsBackend::Metal,
            ..Default::default()
        };
        let text = ron::to_string(&all).unwrap();
        assert!(text.contains("shadow_quality:High") && text.contains("talk_reuse:File"), "{text}");
        assert_eq!(read(&text), all);
        let json = serde_json::to_string(&all).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), all);
        for (name, _) in ShadowQuality::NAMES.iter().chain(&[("Ultra", ShadowQuality::Fast)]) {
            let read: ShadowQuality = ron::from_str(name).unwrap();
            let known = ShadowQuality::NAMES.iter().find(|(n, _)| n == name);
            assert_eq!(read, known.map_or(ShadowQuality::Fast, |(_, v)| *v), "{name}");
        }
    }

    /// Settings of which one does not read (a number where this version
    /// keeps a switch, a list of another shape) are read field by field:
    /// the others are as saved, and the one is named. Text that is no
    /// settings at all is an error.
    #[test]
    fn settings_are_read_field_by_field_where_the_whole_does_not_read() {
        let saved = Settings {
            light_theme: true,
            recent: vec![PathBuf::from("/modules/a (1).mod"), PathBuf::from("b,c\"d.mod")],
            shadow_quality: ShadowQuality::High,
            script_style: ScriptStyle { font_size: 17, ..Default::default() },
            ..Default::default()
        };
        let text = ron::to_string(&saved).unwrap();
        assert_eq!(Settings::read_fields(&text), Ok((saved.clone(), Vec::new())));
        // One field of another type, one of another shape, one unknown.
        let odd = text
            .replacen("no_shadows:false", "no_shadows:3", 1)
            .replacen("last_areas:[]", "last_areas:[(1,2,3)]", 1)
            .replacen("(recent:", "(a_newer_setting:Some((1,\"x\")),recent:", 1);
        assert_ne!(odd, text);
        assert!(ron::from_str::<Settings>(&odd).is_err());
        let (read, unread) = Settings::read_fields(&odd).unwrap();
        assert_eq!(read, saved);
        assert_eq!(unread.len(), 2, "{unread:?}");
        assert!(unread[0].starts_with("last_areas (") || unread[1].starts_with("last_areas ("));
        assert!(unread.iter().any(|u| u.starts_with("no_shadows (")), "{unread:?}");
        // Cut short, or no RON.
        assert!(Settings::read_fields(&text[..text.len() / 2]).is_err());
        assert!(Settings::read_fields("not ron").is_err());
    }

    /// Where the saved settings do not read, the file as it was is kept
    /// beside it before eframe saves over it, once, and the log is told;
    /// settings that read (and no file at all) keep nothing aside.
    #[test]
    fn a_settings_file_that_does_not_read_is_kept_aside() {
        let dir = mg_testkit::scratch_dir("settings-kept-aside");
        let path = dir.join("app.ron");
        let key = "moonglow-settings";
        let write = |settings: &str| {
            let kv = std::collections::HashMap::from([
                ("egui".to_string(), "(options:())".to_string()),
                (key.to_string(), settings.to_string()),
            ]);
            std::fs::write(&path, ron::to_string(&kv).unwrap()).unwrap();
        };
        let copies = || {
            let mut all: Vec<PathBuf> = (std::fs::read_dir(&dir).unwrap().flatten())
                .map(|e| e.path())
                .filter(|p| p != &path)
                .collect();
            all.sort();
            all
        };
        // No file, and a file without the settings: the defaults, quietly.
        assert_eq!(Settings::read_eframe(&path, key), SettingsRead::default());
        write("(light_theme:true)");
        assert_eq!(Settings::read_eframe(&path, "another-key"), SettingsRead::default());
        // Settings that read; a newer version's name among them too.
        write("(light_theme:true,shadow_quality:Ultra)");
        let read = Settings::read_eframe(&path, key);
        assert!(read.settings.light_theme && read.problem().is_none(), "{read:?}");
        assert!(copies().is_empty());
        // One that does not read: the rest, a copy, and a line for the log.
        write("(light_theme:true,no_shadows:3)");
        let as_it_was = std::fs::read(&path).unwrap();
        let read = Settings::read_eframe(&path, key);
        assert!(read.settings.light_theme && !read.settings.no_shadows);
        let kept = read.kept.clone().expect("kept aside");
        assert_eq!(copies(), std::slice::from_ref(&kept));
        assert!(kept.file_name().unwrap().to_string_lossy().starts_with("app.ron.unreadable-"));
        assert_eq!(std::fs::read(&kept).unwrap(), as_it_was);
        let said = read.problem().unwrap();
        assert!(said.starts_with("1 of the saved settings could not be read"), "{said}");
        assert!(said.contains("no_shadows (") && said.contains("app.ron.unreadable-"), "{said}");
        // Started again before anything was saved: the same copy.
        assert_eq!(Settings::read_eframe(&path, key).kept, Some(kept.clone()));
        assert_eq!(copies().len(), 1);
        // A file cut short: the defaults, and the file kept.
        std::fs::write(&path, &as_it_was[..as_it_was.len() / 2]).unwrap();
        let read = Settings::read_eframe(&path, key);
        assert_eq!(read.settings, Settings::default());
        let said = read.problem().unwrap();
        assert!(said.starts_with("The saved settings could not be read ("), "{said}");
        assert_eq!(copies().len(), 2);
        assert_eq!(std::fs::read(read.kept.unwrap()).unwrap(), as_it_was[..as_it_was.len() / 2]);
    }

    /// Two Moonglows share one settings file: a window saves the settings
    /// it changed since it last read or saved them, and leaves the others
    /// as the other window saved them (it wrote its own, older ones back
    /// over them all).
    #[test]
    fn a_window_saves_its_own_changes_over_another_window_s() {
        let started = Settings { light_theme: false, no_grass: false, ..Default::default() };
        // The other window: the theme, and a module opened.
        let mut theirs = started.clone();
        theirs.light_theme = true;
        theirs.recent = vec![PathBuf::from("/modules/theirs.mod")];
        let saved = theirs.saved_text();
        // This one: the grass, since it started.
        let mut mine = started.clone();
        mine.no_grass = true;
        let merged: Settings = ron::from_str(&mine.saved_over(&started, &saved)).unwrap();
        assert!(merged.light_theme && merged.no_grass, "both windows' changes");
        assert_eq!(merged.recent, theirs.recent);
        // The same setting changed in both: this window's, as it saves.
        mine.light_theme = false;
        let mut was = started.clone();
        was.light_theme = true;
        let merged: Settings = ron::from_str(&mine.saved_over(&was, &saved)).unwrap();
        assert!(!merged.light_theme);
        // Nothing changed here: the file as the other window left it. And
        // nothing changed there: this window's own.
        assert_eq!(started.saved_over(&started, &saved), saved);
        assert_eq!(mine.saved_over(&started, &started.saved_text()), mine.saved_text());
        // What is there does not read: this window's own.
        assert_eq!(mine.saved_over(&started, "(light_theme:tr"), mine.saved_text());
        // And the file's text is read from where eframe keeps it.
        let dir = mg_testkit::scratch_dir("settings-saved-over");
        let path = dir.join("app.ron");
        let kv = std::collections::HashMap::from([("k".to_string(), saved.clone())]);
        std::fs::write(&path, ron::to_string(&kv).unwrap()).unwrap();
        assert_eq!(Settings::text_in_eframe_file(&path, "k"), Some(saved));
        assert_eq!(Settings::text_in_eframe_file(&path, "other"), None);
    }

    /// A backend's failure is kept by GPU, driver and backend (one in place
    /// of another of the same), and settings saved before there were any
    /// read as none; a backend this version doesn't know is passed over by
    /// nothing.
    #[test]
    fn graphics_failures_are_kept_by_gpu_driver_and_backend() {
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, wgpu::Backend::Dx12);
        (info.vendor, info.device, info.name) = (0x1002, 0x67df, "Radeon RX 580".into());
        info.driver_info = "25.9.1".into();
        let mut s = Settings::default();
        s.record_graphics_failure(GraphicsFailure::of(&info, "the startup check"));
        s.record_graphics_failure(GraphicsFailure::of(&info, "the area view's watch"));
        assert_eq!(s.graphics_failures.len(), 1, "one of each");
        assert_eq!(s.graphics_failures[0].what, "the area view's watch");
        assert_eq!(s.graphics_failures[0].backend, "Direct3D 12");
        assert_eq!(s.failed_backends(), [mg_render::backend::Failed::of(&info)]);
        info.backend = wgpu::Backend::Vulkan;
        s.record_graphics_failure(GraphicsFailure::of(&info, "the startup check"));
        assert_eq!(s.failed_backends().len(), 2);
        let back: Settings = ron::from_str(&ron::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.graphics_failures, s.graphics_failures);
        let older: Settings = ron::from_str("(light_theme:true)").unwrap();
        assert!(older.graphics_failures.is_empty());
        let unknown = GraphicsFailure { backend: "WebGPU 2".into(), ..Default::default() };
        assert_eq!(unknown.failed(), None);
    }

    /// The lit check's notice is hidden on one GPU, driver and backend
    /// alone (once, however often it is hidden): another backend, or a
    /// driver update, shows it again; settings saved before read as none
    /// hidden.
    #[test]
    fn a_hidden_graphics_notice_is_kept_by_gpu_driver_and_backend() {
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, wgpu::Backend::Vulkan);
        (info.vendor, info.device, info.name) = (0x1002, 0x67df, "Radeon RX 480 Graphics".into());
        info.driver_info = "25.9.1".into();
        let mut s = Settings::default();
        assert!(!s.graphics_notice_hidden(&info));
        s.hide_graphics_notice(GraphicsFailure::of(&info, "the lit check, the game's shaders"));
        s.hide_graphics_notice(GraphicsFailure::of(&info, "the lit check, the game's shaders"));
        assert_eq!(s.graphics_notices_hidden.len(), 1);
        assert!(s.graphics_notice_hidden(&info));
        assert!(s.graphics_failures.is_empty(), "Automatic passes over nothing");
        let back: Settings = ron::from_str(&ron::to_string(&s).unwrap()).unwrap();
        assert!(back.graphics_notice_hidden(&info));
        let mut dx12 = info.clone();
        dx12.backend = wgpu::Backend::Dx12;
        assert!(!s.graphics_notice_hidden(&dx12));
        info.driver_info = "26.1.1".into();
        assert!(!s.graphics_notice_hidden(&info), "a driver update shows it again");
        let older: Settings = ron::from_str("(light_theme:true)").unwrap();
        assert!(older.graphics_notices_hidden.is_empty());
    }

    /// Automatic and the backends the platform has.
    #[test]
    fn each_platform_offers_its_graphics_backends() {
        use GraphicsBackend::*;
        use mg_render::backend::Platform;
        assert_eq!(GraphicsBackend::offered(Platform::Windows), [Automatic, Vulkan, Direct3d12]);
        assert_eq!(GraphicsBackend::offered(Platform::Linux), [Automatic, Vulkan]);
        assert_eq!(GraphicsBackend::offered(Platform::MacOs), [Automatic, Metal]);
        let names: Vec<_> = [Automatic, Vulkan, Direct3d12, Metal].map(|b| b.name()).into();
        assert_eq!(names, ["Automatic", "Vulkan", "Direct3D 12", "Metal"]);
    }

    #[test]
    fn settings_saved_before_script_styles_still_load() {
        let s: Settings = serde_json::from_str(r#"{"recent":["a.mod"]}"#).unwrap();
        assert_eq!(s.script_style, ScriptStyle::default());
        let mut t = s;
        t.script_style.colors[3] = Some([1, 2, 3]);
        let back: Settings = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        assert_eq!(back, t);
        // A style saved before the background, the indentation and the
        // live check were: theirs are the defaults (four spaces, the
        // theme's ground, checked as you type).
        let old = r#"{"script_style":{"font_size":15,"colors":[null,null,null,null,null,null,null,null]}}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.script_style.font_size, 15);
        assert_eq!((s.script_style.indent_tabs, s.script_style.indent_width), (false, 4));
        assert_eq!(s.script_style.background, None);
        assert!(!s.no_live_check);
        let mut t = s;
        (t.script_style.background, t.script_style.indent_tabs) = (Some([9, 9, 9]), true);
        t.no_live_check = true;
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
