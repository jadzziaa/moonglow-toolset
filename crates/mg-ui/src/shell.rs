//! The main window's frame (the UI redesign's shell, proposal section 5
//! and its milestone M3), around the editor area that stays the one
//! egui_dock `DockArea` of [`Moonglow::ui`]:
//!
//! - the **Top Bar** ([`top_bar`]): the eight menus, the quick actions
//!   (New, Open, Save; Undo, Redo) as icons, and at the right end the
//!   build actions (Compile, Verify, Test Module with Choose Character
//!   under its arrow), from [`commands::TOP_BAR`]; in the middle Quick
//!   Open's field (M3's T8, `quick_open`), an icon button where the
//!   bar's middle is narrow;
//! - the **Status Bar** ([`status_bar`]): the Bottom Dock's switches at
//!   the left (the Log's), the log's last line and the active editor's
//!   hints in the middle ([`shown`], T12), its counts, a running job, the
//!   module's name and the game folder at the right;
//! - the **Left Dock** ([`left_dock`]): its tab bar (the tabs of its
//!   panels, Module and Resources, and the shown panel's header buttons),
//!   the panel under it: the Module panel (the Filter over the module
//!   tree, whose Areas group scrolls in a well of its own: M3's T3) or the
//!   Resources panel (the resource browser: T4); the one button that
//!   folds the dock at its tab bar's end toward the editor area (M3 T14's
//!   review); folded, a strip with the chevron that brings it back at its
//!   top and a button for each panel under it. Which panels a dock holds
//!   is the registry's, [`Panel`];
//! - the **Right Dock** ([`right_dock`]): the sections the active docked
//!   editor brings ([`editor_brings`], M4's U2), each a tab bar of its
//!   panels: an area's one section, the Palette panel (the blueprint and
//!   tile palettes, `palette_view`: M3's T5) and the Preview panel (the
//!   blueprint chosen in the palette: `area_tools::preview_body`, T6);
//!   drawn only where the active editor brings them (an area's tab, or the
//!   placeholder in its place) or the Palette was asked for by name; the
//!   sections stand as one bar under 1400, stacked between, in two columns
//!   in the wide layout ([`arrange`]); its fold button at its top bar's
//!   end by the window's edge (in two columns, the last column's bar);
//!   folded, a strip with the chevron that brings it back at its top and a
//!   button for each panel; a panel's body is the same in the dock and in
//!   a floating editor's own window ([`panel_body`], [`window_panes`]);
//! - the **Bottom Dock** ([`bottom_dock`]): its tab bar (a tab for each
//!   panel: the editor's own, then the global ones, the Log's; the panel
//!   in front's tools, the log's Copy, Save… and Clear), the panel in
//!   front under it; opened and closed by its panels' switches in the
//!   Status Bar (issue 10's one control per dock: the switches are the
//!   Bottom Dock's).
//!
//! The order [`Moonglow::ui`] draws them in makes the layout: the Top Bar,
//! then the notices under it, the Status Bar, the Left Dock and the Right
//! Dock (each the whole height between the bars), the Bottom Dock (under
//! the editor area only), and the editor area in what is left. The panels
//! keep the ids the window's panes had (`"tree"`, `"log"`), so the widths
//! and heights eframe kept for them carry over, taken once as the sizes
//! chosen; the Right Dock's panel has an id of its own ([`RIGHT_DOCK_ID`]:
//! the palettes' pane was a share of the editor area, kept for no run
//! after). The size each dock was given (its edge dragged) is kept apart
//! from the size it is drawn at (`Settings::left_dock_width`,
//! `right_dock_width`, `bottom_dock_height`; [`chosen_size`]): drawn held
//! to what the window allows, a dock is as large again when the window
//! is. `Settings::hide_tree`
//! is "the Left Dock folded" as the builder left it, `Settings::hide_palettes`
//! "the Right Dock folded" and `Settings::hide_log` "the Bottom Dock
//! closed".
//!
//! How the shell yields (T14, proposal sections 5.2 and 5.3): one
//! `Yields` a frame ([`frame_yields`]) for the Top Bar, Quick Open, the
//! docks and the Status Bar. A narrow window folds a side dock without
//! writing `hide_tree` or `hide_palettes` (the Left Dock under 1500, the
//! Right Dock under 796, and where both do not fit beside the editor
//! area's least the one unfolded first): every read of a side dock's fold
//! is [`left_folded`] or [`right_folded`], every write [`set_dock_open`].
//! The Right Dock is 280 under 1400; from 2560 a dock not chosen is wider
//! (320, 741) and the Right Dock stands in two columns: the Palette and
//! Preview tabs, then Properties. Hints (T12) and the Outliner (T13) are in their own places.

use egui::{Margin, Ui};
use mg_uikit::shell::{self as kit, DockTab, DockTabs, FoldStrip, Side, StripClick, Yields};
use mg_uikit::stack::{DockDrop, DropSection, StackDrop, StackZone};
use mg_uikit::status::{
    self, Hint, HintPart, Hints, StatusItem, StatusJob, StatusLog, StatusSwitch,
};
use mg_uikit::{Icon, IconButton, Join, metrics};

use crate::commands::{self, CommandRow, Id, TOP_BAR, TOP_BAR_TEST_MENU, TopBarButton};
use crate::icons::Tools;
use crate::{Moonglow, tree};

/// A line between the Top Bar's groups, as wide as it takes
/// (`kit::vsep`).
const VSEP: f32 = 9.0;

/// The least size of the main window (its inside, in points): the Top
/// Bar's eight menus fit (Help ends about 395 points from the left), and
/// the bars and a line of the editor area beside the docks. In points, so
/// at every interface size: the menus are as many points wide at 150% as
/// at 100%, and a point is then 1.5 pixels ([`keep_least_size`]). A
/// window kept smaller opens at this size (`apps/moonglow`).
pub const LEAST_WINDOW_SIZE: egui::Vec2 = egui::Vec2::new(420.0, 320.0);

/// Keeps the window at least [`LEAST_WINDOW_SIZE`] points inside, at any
/// interface size: each time the scale from points to pixels changes
/// (Options › General › Interface size, Ctrl with + and -, a screen of
/// another scale; the first frame too), the least size is given again
/// (egui-winit turns it into pixels at the scale of that frame), and a
/// window smaller than it in points is made as large. The scale read is
/// the frame's input's, which a zoom set this frame reaches the next: it
/// waits for the interface size to be in effect (on the first frame the
/// setting's zoom is not yet), so a window is never grown for a size it
/// is not drawn at.
pub(crate) fn keep_least_size(app: &mut Moonglow, ctx: &egui::Context) {
    let scale = ctx.input(|i| i.pixels_per_point);
    // (Not while the interface size is on its way: its zoom set, or set
    // this frame, and the frame's scale still the old one.)
    // (The size in use: the setting's, clamped and held to the screen.)
    let wanted = app
        .settings
        .ui_scale
        .map_or(ctx.zoom_factor(), |_| f32::from(crate::commands::ui_scale(app)) / 100.0);
    let settled = (ctx.zoom_factor() - wanted).abs() < 0.001
        && (ctx.pixels_per_point() - scale).abs() < 0.001;
    if !settled || app.least_size_for == Some(scale) {
        return;
    }
    app.least_size_for = Some(scale);
    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(LEAST_WINDOW_SIZE));
    let inside = ctx.input(|i| i.viewport().inner_rect).unwrap_or_else(|| ctx.content_rect());
    let size = inside.size();
    if size.x < LEAST_WINDOW_SIZE.x || size.y < LEAST_WINDOW_SIZE.y {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size.max(LEAST_WINDOW_SIZE)));
    }
}

/// A panel of the docks (the plan's decision 2: a registry of the panels
/// and the dock each lives in, fixed there by the owner's ruling R5;
/// generalized in M4's U2). The Left Dock holds Module and Resources, in
/// that order ([`Panel::LEFT`]), and under them, split from them by a
/// seam, the Outliner (T13), not a tab: a panel of its own under the one
/// shown. The Right Dock holds the **sections** the active docked editor
/// brings ([`editor_brings`]: an area's Palette and Preview, one section
/// of two tabs), each a tab bar of its panels; the Bottom Dock the
/// editor's own panels (a script's Compiler and Bookmarks) and the global
/// ones ([`Panel::BOTTOM_TABS`]: Search Results and the Log), one tab bar,
/// each with its switch in the Status Bar. A dock
/// whose panels none is available is not drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Panel {
    /// The module tree, its Filter and its header buttons (T3).
    #[default]
    Module,
    /// The resource browser: every resource of the load order (T4).
    Resources,
    /// The blueprint and tile palettes, prefabs, lights and the start
    /// location (T5).
    Palette,
    /// The blueprint chosen in the palette: its fields, and its model in
    /// 3D (Aurora's Preview window, Moonglow's until T6), the Palette's
    /// neighbor on the Right Dock's tab bar.
    Preview,
    /// What the active editor holds, under the Left Dock's tabs (T13): an
    /// area's placed objects by kind, each kind with an eye; a
    /// conversation's bookmarks (M4's U15), in its window where it floats
    /// (`outliner.rs`).
    Outliner,
    /// The log: the Bottom Dock's global panel, whatever the editor in
    /// front, with its switch in the Status Bar.
    Log,
    /// The tiles selected in an area's Select Tiles: their lights and
    /// loops, height, replacement texture, textures and the tiles to put
    /// in (`tile_select::panel`, M4's U5), the area's Right Dock's lower
    /// section while Select Tiles is on (the AreaTiles board).
    TileProperties,
    /// What is selected in an area's view with any tool but Select Tiles:
    /// one object's properties, or several of one type's edited together
    /// (`properties_panel::panel`, M4's U6: the Properties window's pages
    /// without OK and Cancel, as a rail at its left), the area's Right
    /// Dock's lower section (the Main, Narrow, WideArea and Floating
    /// boards).
    Properties,
    /// A script's Reference (M4's U12, the Script board's Right Dock): the
    /// functions, variables and constants the script can use and the
    /// templates, chosen by a segmented control over a filter and the
    /// list; a click shows one in Help, a double click puts it in at the
    /// cursor (`script_reference::reference`).
    Reference,
    /// A script's Help (M4's U12), under Reference: the declaration of
    /// the function or constant chosen or under the cursor, and its
    /// documentation as prose with a row for each parameter
    /// (`script_reference::help`).
    Help,
    /// A script's Compiler (M4's U13, the Script board's Bottom Dock): what
    /// its latest compile said, a click on a message going to its line
    /// (`script_panels::compiler`).
    Compiler,
    /// A script's Bookmarks (M4's U13): its bookmarked lines and numbered
    /// bookmarks, a click going to the line (`script_panels::bookmarks`).
    Bookmarks,
    /// Search Results (M4's U13): the Bottom Dock's global panel beside
    /// the Log, whatever the editor in front: the last search's hits, a
    /// click opening one (`search_results::body`). Fed by a script's
    /// searches (Find In Files, Find In Currently Open Scripts,
    /// References), Find Instance and Find and Replace Text (U28: their
    /// forms its header).
    SearchResults,
    /// Hak Pak Conflict Analysis (M4's U28, a window before): a Bottom
    /// Dock tab while its report is held, opened by Check for Conflicts…,
    /// closed by its Close (`dock_reports::hak_body`).
    HakConflicts,
    /// Resources Used (M4's U28, Build › Area Statistics; a window before):
    /// a Bottom Dock tab while its area's view is open
    /// (`dock_reports::used_body`).
    ResourcesUsed,
    /// The Claims Table (M4's U28, Edit › Claims Table…; a window before):
    /// a Bottom Dock tab while it is open and the project keeps claims
    /// (`claims::table_body`).
    ClaimsTable,
    /// A conversation's line chosen (M4's U14, the Conversation board's
    /// Right Dock): its text, condition, action, other actions and
    /// comments, a section each on one scroll, under the Node section's
    /// tab "Data" (`dialog_view::node_data`).
    NodeData,
    /// A conversation's Find and Replace (M4's U14): the Node section's
    /// tab "Search", beside Data (`dialog_view::node_search`). Its
    /// bookmarks are the Outliner's (U15).
    NodeSearch,
    /// A conversation's own settings (M4's U14), under the Node section:
    /// Normal end, Aborted and Stop camera zoom in, the word count in its
    /// header (`dialog_view::conversation_panel`).
    Conversation,
    /// A table's cell chosen (M4's U16, the Table board's Right Dock): its
    /// value, typed in, its list's or flags' choice and the choosers its
    /// column's kind has, its kind's words and what the table is described
    /// by (`table_view::panels::cell`).
    Cell,
    /// What names a table's row (M4's U16), under Cell: Find References to
    /// Row N and what it found, the game's tables' cells and the module's
    /// fields and scripts (`table_view::panels::references`).
    References,
    /// A talk table's line chosen (M4's U17, the Talk board's Right Dock):
    /// its text, feminine text, sound and length, typed in or, for a table
    /// only looked at, read-only (`talk_view::panels::string`). A talk
    /// table's References are [`Panel::References`], under it.
    String,
    /// A talk table's facts (M4's U17), under References: its lines,
    /// language, feminine table and file, and Module's Table for a module
    /// with none (`talk_view::panels::file`).
    TalkFile,
    /// A NUI designer's palette of controls (M4's U18, the NUI board's
    /// Right Dock, its top section): Find a control… and the components by
    /// group, a click adding one to the control chosen, a drag putting one
    /// where it is dropped (`nui_view::panels::components`). A designer's
    /// properties are [`Panel::Properties`], under it.
    Components,
    /// A NUI designer's binds (M4's U18), under Properties: Create bind,
    /// each bind's value, watch and uses, Client delivery and Window
    /// identity (`nui_view::panels::bindings`).
    Bindings,
    /// A NUI designer's Checks (M4's U19, the NUI board's Bottom Dock): what
    /// the checks of its window found, each a row of its kind, its control
    /// (a click chooses it in the design, Layers and the source), its
    /// message and its fix (Bind It, Turn It Off, Add Close Handler)
    /// (`nui_view::checks::panel`); its Status Bar switch counts them.
    Checks,
    /// Problems (M4's U20): the Bottom Dock's global panel of findings,
    /// whatever the editor in front, each source's last (the tileset
    /// editor's Check; U28 adds Build Module and Verify), a click going to
    /// where one is (`problems::body`); its Status Bar switch counts them.
    Problems,
    /// A tileset's Textures (M4's U20, the Tileset board's Right Dock,
    /// under its Preview): the textures the chosen tile's model draws,
    /// where each is found and what draws it (`tileset_view::textures`).
    Textures,
    /// A hak's Details (M4's U21, the Hak board's Right Dock, under its
    /// Preview): its file, what it holds, the folder it was built from with
    /// Update from Folder, and its description (`hak_view::details`).
    Details,
    /// A journal's entry chosen (M4's U22, the Journal board's Right Dock):
    /// its ID, Finish category and Text in the editing language, its
    /// detail "[0030] of M1Q5_Bertrand" (`journal_view::entry_panel`).
    Entry,
    /// A journal's category (M4's U22), under Entry: the one chosen, or the
    /// chosen entry's (its parent): Name, Tag, Priority, XP and Comments
    /// (`journal_view::category_panel`).
    Category,
    /// The faction editor's chosen faction (M4's U22, the Factions board's
    /// Right Dock): its name and parent, read-only, and Global effect
    /// (`faction_view::faction_panel`).
    Faction,
    /// The faction editor's chosen reaction (M4's U22), under Faction: the
    /// sentence, the slider and the Hostile, Neutral, Friendly scale
    /// (`faction_view::reaction_panel`).
    Reaction,
    /// How the chosen faction regards the others (M4's U22), under
    /// Reaction, in Advanced: a row a faction, Full detail at its foot
    /// (`faction_view::reactions_panel`).
    Reactions,
    /// The Appearance Gallery's chosen appearance (M4's U26, the
    /// AppearanceGallery board's Right Dock, under its Preview): its row,
    /// name, label and model, and its kind's facts: a creature's model
    /// type, size, movement rate, personal space, height and blood color
    /// (`appearance_gallery::facts`).
    Appearance,
    /// Edit Areas Together (M4's U29; a window before): the areas chosen by
    /// what they hold (`area_props::AreaChooser`) and their shared pages,
    /// Area Properties' Visual, Grass, Audio, Events, Advanced and Variables,
    /// on a rail as Properties' are, what is changed set on each at once: a
    /// section of the Right Dock while it is open, whatever the editor in
    /// front (`area_props::together_panel`).
    AreasTogether,
    /// Custom Light and Change Light (M4's U29, the Light Maker; a window
    /// before): a section of its area's Right Dock while it is open, its
    /// main action and Cancel in the row at its top (`light_maker::panel`).
    LightMaker,
    /// Bake Walkmesh to Transform (M4's U29; a window before): a section of
    /// its area's Right Dock while it is open, the view showing the
    /// walkmeshes as baked, Bake and Cancel in the row at its top
    /// (`bake_view::panel`).
    Bake,
    /// A panel of the unit tests' own (`shell::tests`): numbers under
    /// [`SAMPLE_BOTTOM`] are the Right Dock's, the others the Bottom
    /// Dock's, the count their badge. Built only into the library's own
    /// tests: neither the application nor `tests/` has it.
    #[cfg(test)]
    Sample(u8),
}

/// A dock that holds panels ([`Panel::dock`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dock {
    Left,
    Right,
    Bottom,
}

/// The first of the test panels ([`Panel::Sample`]) that is the Bottom
/// Dock's.
#[cfg(test)]
pub(crate) const SAMPLE_BOTTOM: u8 = 8;

/// The test panel that never has anything to show (its dock is drawn
/// without it).
#[cfg(test)]
pub(crate) const SAMPLE_EMPTY: u8 = 7;

/// The test panels' names, by number.
#[cfg(test)]
const SAMPLE_NAMES: [&str; 16] = [
    "Sample 0",
    "Sample 1",
    "Sample 2",
    "Sample 3",
    "Sample 4",
    "Sample 5",
    "Sample 6",
    "Sample 7",
    "Sample 8",
    "Sample 9",
    "Sample 10",
    "Sample 11",
    "Sample 12",
    "Sample 13",
    "Sample 14",
    "Sample 15",
];

impl Panel {
    /// The Left Dock's panels, in the order of its tabs (the Outliner is
    /// under them, no tab: [`Panel::OUTLINER`]).
    pub(crate) const LEFT: [Panel; 2] = [Panel::Module, Panel::Resources];
    /// The Left Dock's panel under its tabs, split from them.
    pub(crate) const OUTLINER: Panel = Panel::Outliner;
    /// Every panel of the docks that takes the arrow keys, each with its
    /// own (`Panel::keys`): where they are while it is not drawn, they
    /// leave it ([`keys_leave_panels_not_drawn`]). A new panel with keys
    /// of its own is named here.
    pub(crate) const KEYED: [Panel; 38] = [
        Panel::Module,
        Panel::Resources,
        Panel::Outliner,
        Panel::Palette,
        Panel::Preview,
        Panel::Log,
        Panel::TileProperties,
        Panel::Properties,
        Panel::Reference,
        Panel::Help,
        Panel::Compiler,
        Panel::Bookmarks,
        Panel::SearchResults,
        Panel::NodeData,
        Panel::NodeSearch,
        Panel::Conversation,
        Panel::Cell,
        Panel::References,
        Panel::String,
        Panel::TalkFile,
        Panel::Components,
        Panel::Bindings,
        Panel::Checks,
        Panel::Problems,
        Panel::Textures,
        Panel::Details,
        Panel::Entry,
        Panel::Category,
        Panel::Faction,
        Panel::Reaction,
        Panel::Reactions,
        Panel::Appearance,
        Panel::HakConflicts,
        Panel::ResourcesUsed,
        Panel::ClaimsTable,
        Panel::AreasTogether,
        Panel::LightMaker,
        Panel::Bake,
    ];
    /// The Bottom Dock's global panels (shown whatever the editor in
    /// front), in their tabs' order after the editor's own: the Script
    /// board's Compiler, Bookmarks, then Search Results, Log; Problems before
    /// Search Results, as the GalleryWide board's tabs have them (M4's U20).
    /// U28's report tabs after them, each while open.
    pub(crate) const BOTTOM_TABS: &'static [Panel] = &[
        Panel::Problems,
        Panel::SearchResults,
        Panel::Log,
        Panel::HakConflicts,
        Panel::ResourcesUsed,
        Panel::ClaimsTable,
    ];
    /// The global panels' switches before the editor's own in the Status
    /// Bar (every board's Log, Problems: M4's U20), and after them (its
    /// Search Results).
    pub(crate) const SWITCHES_BEFORE: &'static [Panel] = &[Panel::Log, Panel::Problems];
    /// U28's report tabs' switches after Search Results', each while open.
    pub(crate) const SWITCHES_AFTER: &'static [Panel] =
        &[Panel::SearchResults, Panel::HakConflicts, Panel::ResourcesUsed, Panel::ClaimsTable];

    /// The dock it lives in.
    pub(crate) fn dock(self) -> Dock {
        match self {
            Panel::Module | Panel::Resources | Panel::Outliner => Dock::Left,
            Panel::Palette
            | Panel::Preview
            | Panel::TileProperties
            | Panel::Properties
            | Panel::Reference
            | Panel::NodeData
            | Panel::NodeSearch
            | Panel::Conversation
            | Panel::Cell
            | Panel::References
            | Panel::String
            | Panel::TalkFile
            | Panel::Components
            | Panel::Bindings
            | Panel::Textures
            | Panel::Details
            | Panel::Entry
            | Panel::Category
            | Panel::Faction
            | Panel::Reaction
            | Panel::Reactions
            | Panel::Appearance
            | Panel::AreasTogether
            | Panel::LightMaker
            | Panel::Bake => Dock::Right,
            // (A script's Help under its text, as 4.2.1 had it: 5.2.0, the
            // owner's ruling on Jasperre's report.)
            Panel::Log
            | Panel::Compiler
            | Panel::Help
            | Panel::Bookmarks
            | Panel::SearchResults
            | Panel::Checks
            | Panel::Problems
            | Panel::HakConflicts
            | Panel::ResourcesUsed
            | Panel::ClaimsTable => Dock::Bottom,
            #[cfg(test)]
            Panel::Sample(n) if n < SAMPLE_BOTTOM => Dock::Right,
            #[cfg(test)]
            Panel::Sample(_) => Dock::Bottom,
        }
    }

    /// Its name: its tab's words, its strip button's name, its switch's.
    pub fn name(self) -> &'static str {
        match self {
            Panel::Module => "Module",
            Panel::Resources => "Resources",
            Panel::Palette => "Palette",
            Panel::Preview => "Preview",
            Panel::Outliner => "Outliner",
            Panel::Log => "Log",
            Panel::TileProperties => "Tile Properties",
            Panel::Properties => "Properties",
            Panel::Reference => "Reference",
            Panel::Help => "Help",
            Panel::Compiler => "Compiler",
            Panel::Bookmarks => "Bookmarks",
            Panel::SearchResults => "Search Results",
            Panel::NodeData => "Data",
            Panel::NodeSearch => "Search",
            Panel::Conversation => "Conversation",
            Panel::Cell => "Cell",
            Panel::References => "References",
            Panel::String => "String",
            Panel::TalkFile => "Talk Table",
            Panel::Components => "Components",
            Panel::Bindings => "Bindings",
            Panel::Checks => "Checks",
            Panel::Problems => "Problems",
            Panel::Textures => "Textures",
            Panel::Details => "Details",
            Panel::Entry => "Entry",
            Panel::Category => "Category",
            Panel::Faction => "Faction",
            Panel::Reaction => "Reaction",
            Panel::Reactions => "Reactions",
            Panel::Appearance => "Appearance",
            Panel::HakConflicts => "Hak Pak Conflict Analysis",
            Panel::ResourcesUsed => "Resources Used",
            Panel::ClaimsTable => "Claims Table",
            Panel::AreasTogether => "Edit Areas Together",
            Panel::LightMaker => "Custom Light",
            Panel::Bake => "Bake Walkmesh to Transform",
            #[cfg(test)]
            Panel::Sample(n) => SAMPLE_NAMES[usize::from(n) % SAMPLE_NAMES.len()],
        }
    }

    /// The panel of the name `name` ([`Panel::name`]), as the settings
    /// keep it (`Settings::dock_layouts`): `None` for a name this version
    /// does not know (a newer one's panel).
    pub(crate) fn named(name: &str) -> Option<Panel> {
        let known = [
            Panel::Module,
            Panel::Resources,
            Panel::Palette,
            Panel::Preview,
            Panel::Outliner,
            Panel::Log,
            Panel::TileProperties,
            Panel::Properties,
            Panel::Reference,
            Panel::Help,
            Panel::Compiler,
            Panel::Bookmarks,
            Panel::SearchResults,
            Panel::NodeData,
            Panel::NodeSearch,
            Panel::Conversation,
            Panel::Cell,
            Panel::References,
            Panel::String,
            Panel::TalkFile,
            Panel::Components,
            Panel::Bindings,
            Panel::Checks,
            Panel::Problems,
            Panel::Textures,
            Panel::Details,
            Panel::Entry,
            Panel::Category,
            Panel::Faction,
            Panel::Reaction,
            Panel::Reactions,
            Panel::Appearance,
            Panel::HakConflicts,
            Panel::ResourcesUsed,
            Panel::ClaimsTable,
            Panel::AreasTogether,
            Panel::LightMaker,
            Panel::Bake,
        ];
        #[cfg(test)]
        if let Some(n) = SAMPLE_NAMES.iter().position(|s| *s == name) {
            return Some(Panel::Sample(n as u8));
        }
        known.into_iter().find(|p| p.name() == name)
    }

    /// Its icon on its dock's strip, folded (`Narrow.html`'s rail), on
    /// its tab where the dock is too narrow for the tabs' words, and on
    /// its switch.
    fn icon(self) -> Icon {
        match self {
            Panel::Module => Icon::Module,
            Panel::Resources => Icon::Folder,
            Panel::Palette => Icon::Palette,
            Panel::Preview => Icon::Eye,
            Panel::Outliner => Icon::Outliner,
            Panel::Log => Icon::Log,
            Panel::TileProperties => Icon::Tile,
            Panel::Properties => Icon::Properties,
            Panel::Reference => Icon::Book,
            Panel::Help => Icon::Help,
            Panel::Compiler => Icon::Compile,
            Panel::Bookmarks => Icon::Bookmark,
            Panel::SearchResults => Icon::Search,
            Panel::NodeData => Icon::Properties,
            Panel::NodeSearch => Icon::Find,
            Panel::Conversation => Icon::Conversation,
            Panel::Cell => Icon::Column,
            Panel::References => Icon::References,
            Panel::String => Icon::Description,
            Panel::TalkFile => Icon::TalkTable,
            Panel::Components => Icon::Nui,
            Panel::Bindings => Icon::Link,
            Panel::Checks => Icon::CheckCircle,
            Panel::Problems => Icon::Problems,
            Panel::Textures => Icon::Texture,
            Panel::Details => Icon::Info,
            Panel::Entry => Icon::Description,
            Panel::Category => Icon::Journal,
            Panel::Faction => Icon::Faction,
            Panel::Reaction => Icon::Edit,
            Panel::Reactions => Icon::List,
            Panel::Appearance => Icon::Info,
            Panel::HakConflicts => Icon::Hak,
            Panel::ResourcesUsed => Icon::Statistics,
            Panel::ClaimsTable => Icon::Lock,
            Panel::AreasTogether => Icon::Together,
            Panel::LightMaker => Icon::CustomLight,
            Panel::Bake => Icon::Bake,
            #[cfg(test)]
            Panel::Sample(_) => Icon::Eye,
        }
    }

    /// How many header buttons it has on its dock's tab bar: the Module
    /// panel's Module Properties, New Area, Expand All and Collapse All;
    /// the Resources panel's Export and Refresh; the Log's Copy, Save…
    /// and Clear; none of the Palette panel's and the Preview panel's
    /// (their tools are their own rows, as the boards draw them).
    fn tools(self) -> usize {
        match self {
            // (The Left Dock's draw their own: `module_tools`,
            // `browser::tools`.)
            Panel::Module => 4,
            Panel::Resources => 2,
            // (The Node section's Bookmark and Test, its own: drawn by
            // `dialog_view::node_tools`.)
            Panel::NodeData | Panel::NodeSearch => 2,
            // (A talk table's String: Copy the StrRef, M4's U17.)
            Panel::String => 1,
            // (A NUI designer's Properties: Element Actions, M4's U18; the
            // area's: Open in Tab, drawn by its own code.)
            // (The others' from their list; the Outliner's Expand All and
            // Collapse All are on its own header.)
            _ => self.header_tools().len(),
        }
    }

    /// Its strip button's tip, without the keys.
    fn strip_tip(self) -> &'static str {
        match self {
            Panel::Module => "Module: the module tree",
            Panel::Resources => "Resources: the resource browser",
            Panel::Palette => "Palette: the blueprint and tile palettes",
            Panel::Preview => "Preview: the model of the blueprint open or chosen in the palette",
            Panel::Outliner => "Outliner: what the editor in front holds",
            Panel::Log => "Log: what Moonglow reports",
            Panel::TileProperties => "Tile Properties: the tiles selected",
            Panel::Properties => "Properties: what is selected",
            Panel::Reference => {
                "Reference: the functions, variables, constants and templates a script can use"
            }
            Panel::Help => "Help: the function or constant chosen or under the cursor",
            Panel::Compiler => "Compiler: what the script's latest compile said",
            Panel::Bookmarks => "Bookmarks: the script's bookmarked lines",
            Panel::SearchResults => "Search Results: what the last search found",
            Panel::NodeData => "Data: the conversation's line chosen",
            Panel::NodeSearch => "Search: find and replace in the conversation",
            Panel::Conversation => "Conversation: the conversation's own settings",
            Panel::Cell => "Cell: the table's cell chosen",
            Panel::References => "References: what names the table's row or line chosen",
            Panel::String => "String: the talk table's line chosen",
            Panel::TalkFile => "Talk Table: the talk table's lines, language and file",
            Panel::Components => "Components: the controls to add to the NUI window",
            Panel::Bindings => "Bindings: the NUI window's binds",
            Panel::Checks => "Checks: what the checks of the NUI window found",
            Panel::Problems => "Problems: what the checks found",
            Panel::Textures => "Textures: the textures the chosen tile's model draws",
            Panel::Details => "Details: the hak's file, contents, folder and description",
            Panel::Entry => "Entry: the journal's entry chosen",
            Panel::Category => "Category: the journal's category chosen, or the entry's",
            Panel::Faction => "Faction: the faction chosen, its parent and Global effect",
            Panel::Reaction => "Reaction: how the chosen faction regards the one chosen",
            Panel::Reactions => "Reactions: how the chosen faction regards the others",
            Panel::Appearance => "Appearance: the facts of the appearance chosen in the gallery",
            Panel::HakConflicts => {
                "Hak Pak Conflict Analysis: the resources the module's haks share and override"
            }
            Panel::ResourcesUsed => "Resources Used: what the area's view has loaded",
            Panel::ClaimsTable => "Claims Table: whose each area is",
            Panel::AreasTogether => "Edit Areas Together: several areas' shared pages",
            Panel::LightMaker => "Custom Light: a light of your color and reach, made into a hak",
            Panel::Bake => "Bake Walkmesh to Transform: what the selected placeables block",
            #[cfg(test)]
            Panel::Sample(_) => "A test's panel",
        }
    }

    /// The command that shows it (Tools › Palettes, Tools › Preview,
    /// Tools › Resource Browser, View › Log), whose keys its strip button's
    /// tip says.
    fn command(self) -> Option<Id> {
        match self {
            Panel::Palette => Some(Id::Palettes),
            Panel::Preview => Some(Id::PreviewWindow),
            Panel::Resources => Some(Id::Resources),
            Panel::Log => Some(Id::ViewLog),
            _ => None,
        }
    }

    /// Its tools at its tab bar's end where its dock draws them from this
    /// list (the Bottom Dock): command, icon, name, tip. The Log's Copy,
    /// Save… and Clear.
    fn header_tools(self) -> &'static [TopBarButton] {
        match self {
            Panel::Log => &LOG_TOOLS,
            Panel::HakConflicts | Panel::ResourcesUsed | Panel::ClaimsTable => &REPORT_TOOLS,
            _ => &[],
        }
    }

    /// [`Panel::badge`] for the active docked editor (the tests').
    pub(crate) fn badge_for_tests(self, app: &Moonglow) -> Option<mg_uikit::Badge> {
        self.badge(app, app.active_editor())
    }

    /// Its switch's count badge in the Status Bar, where it has one (M4:
    /// Problems' errors, Search Results' hits, Checks'); the Log has none.
    /// `editor` is the editor it counts for (the active docked one).
    fn badge(self, app: &Moonglow, editor: Option<&crate::Tab>) -> Option<mg_uikit::Badge> {
        match self {
            // (Its hits, where the last search found any.)
            Panel::SearchResults => crate::search_results::badge(app),
            // (A NUI designer's errors and warnings, the NUI board's "1":
            // in `error` where any is an error, else in `warn`.)
            Panel::Checks => crate::nui_view::checks::badge(app, editor),
            // (Every source's errors and warnings, the board's "3" in
            // `error`: M4's U20.)
            Panel::Problems => crate::problems::badge(app),
            #[cfg(test)]
            Panel::Sample(n) if n >= SAMPLE_BOTTOM => {
                Some(mg_uikit::Badge::count(usize::from(n - SAMPLE_BOTTOM)))
            }
            _ => None,
        }
    }

    /// What its header says after its title where its section holds it
    /// alone (the kit's `PanelHeader`, `.docktab.title` and its `.t3`
    /// detail): Tile Properties' "6 tiles" or "Tile (4, 4)" (the AreaTiles
    /// and GalleryTiles boards). `None`: its section shows a tab bar, as
    /// every other panel's does. `editor` is the editor it draws for.
    fn detail(self, app: &Moonglow, editor: Option<&crate::Tab>) -> Option<String> {
        match self {
            Panel::TileProperties => Some(crate::tile_select::detail(app, editor)),
            Panel::Properties if crate::nui_view::panels::nui_of(editor).is_some() => {
                crate::nui_view::panels::detail(app, editor, self)
            }
            Panel::Properties => Some(crate::properties_panel::detail(app, editor)),
            Panel::Preview if crate::tileset_view::tileset_of(editor).is_some() => {
                crate::tileset_view::panel_detail(app, editor, self)
            }
            // (The Appearance Gallery's: the model of the row chosen,
            // "c_aribeth", and its table, "appearance.2da"; M4's U26.)
            Panel::Preview | Panel::Appearance if crate::appearance_gallery::is_gallery(editor) => {
                crate::appearance_gallery::panel_detail(app, editor, self)
            }
            // (A hak's: the resource chosen, "c_masterius.mdl", and the
            // hak's file, the Hak board's; M4's U21.)
            Panel::Preview | Panel::Details if crate::hak_view::hak_of(editor).is_some() => {
                crate::hak_view::panel_detail(app, editor, self)
            }
            // (From the first frame, before the view is made: its bar keeps
            // its form.)
            Panel::Preview => own_preview(app, editor).map(|source| {
                crate::model_view::info(app, &source).unwrap_or_else(|| source.to_string())
            }),
            // (The Script board's: where the lists are from, and the name
            // Help shows.)
            Panel::Reference => Some(crate::script_reference::REFERENCE_DETAIL.to_string()),
            Panel::Help => Some(crate::script_reference::help_detail(app)),
            // (The Conversation board's "3834 words in the file".)
            Panel::Conversation => crate::dialog_view::words_detail(app, editor),
            // (The Table board's "Row 9, TargetType" and "Row 9,
            // Bulls_Strength".)
            Panel::Cell => crate::table_view::panels::detail(app, editor, false),
            // (A talk table's "Talk-table line 18608", the Talk board's.)
            Panel::References if talk_of(app, editor).is_some() => {
                crate::talk_view::panels::detail(app, editor, self)
            }
            Panel::References => crate::table_view::panels::detail(app, editor, true),
            Panel::String | Panel::TalkFile => crate::talk_view::panels::detail(app, editor, self),
            // (A NUI designer's: the control chosen, "Button · Send"; the
            // binds' count, "3 binds". The board's.)
            Panel::Bindings if crate::nui_view::panels::nui_of(editor).is_some() => {
                crate::nui_view::panels::detail(app, editor, self)
            }
            // (A tileset's: "Tile 16, tcn01_c02_01" and "of tcn01_c02_01
            // (21)", the Tileset board's; M4's U20.)
            Panel::Textures => crate::tileset_view::panel_detail(app, editor, self),
            // (The Journal board's "[0030] of M1Q5_Bertrand" and the
            // category's tag; the Factions board's "Victim (Danger) toward
            // Hostile" and "How Victim (Danger) regards the others": M4's
            // U22.)
            Panel::Entry | Panel::Category => crate::journal_view::panel_detail(app, self),
            Panel::Reaction | Panel::Reactions => crate::faction_view::panel_detail(app, self),
            // (M4's U29: the areas ticked, the light's words, the
            // placeables to bake.)
            Panel::AreasTogether | Panel::LightMaker | Panel::Bake => {
                crate::tool_sections::detail(app, self)
            }
            _ => None,
        }
    }

    /// The name its section shows before its tabs (the kit's
    /// `DockTabs::title`, `.docktab.title`): the Conversation board's
    /// "Node" before Data and Search, where they stand as a section of
    /// their own (not in the one tab bar of a narrow window).
    fn section_title(self) -> Option<&'static str> {
        match self {
            Panel::NodeData | Panel::NodeSearch => Some(crate::dialog_view::NODE),
            _ => None,
        }
    }

    /// Whether it has something to show now: the module tree a module
    /// open; the resource browser a module open, or (without one) once it
    /// was asked for (Tools › Resource Browser), for the run; the palettes
    /// where the active editor brings them ([`editor_brings`]: an area's
    /// tab or the placeholder in front, the game's data read), or where
    /// they were asked for by name ([`Moonglow::palettes_asked`]: the
    /// game's standard palettes without a module, or with no game data
    /// read the panel saying so, beside any tab), unless a test turned
    /// them off (`Moonglow::palette_dock`); the Preview with the palettes,
    /// on the tab beside theirs, or alone where the active editor brings
    /// its own (a blueprint's, M4's U11; not while a test turned the
    /// palettes off); the Outliner where the active editor
    /// brings it (an area's tab in front); the Log always; any other
    /// panel where the active editor brings it.
    pub(crate) fn available(self, app: &Moonglow) -> bool {
        self.available_with(app, &editor_brings(app))
    }

    /// [`Panel::available`] where `brings` is what the editor brings (the
    /// active docked editor's, or a floating editor's in its window).
    pub(crate) fn available_with(self, app: &Moonglow, brings: &Brings) -> bool {
        match self {
            Panel::Module => app.ws.is_some(),
            Panel::Resources => app.ws.is_some() || app.resources_alone,
            Panel::Palette => app.palette_dock && (app.palettes_asked || brings.has(self)),
            // (With the palettes, theirs or asked for by name; or a kind's
            // own, brought without them: a blueprint's, U11.)
            // (A tileset's own, its tile's diagram: M4's U20, whatever the
            // palettes.)
            Panel::Preview if brings.diagram => true,
            Panel::Preview => match brings.has(self) && !brings.has(Panel::Palette) {
                true => app.palette_dock,
                false => Panel::Palette.available_with(app, brings),
            },
            Panel::Outliner => brings.outliner,
            Panel::Log => true,
            Panel::TileProperties | Panel::Properties => {
                brings.has(self) || brings.window_has(self)
            }
            Panel::Reference
            | Panel::Help
            | Panel::Compiler
            | Panel::Bookmarks
            | Panel::NodeData
            | Panel::NodeSearch
            | Panel::Conversation
            | Panel::Cell
            | Panel::References
            | Panel::String
            | Panel::TalkFile
            | Panel::Components
            | Panel::Bindings
            | Panel::Checks
            | Panel::Textures
            | Panel::Details
            | Panel::Entry
            | Panel::Category
            | Panel::Faction
            | Panel::Reaction
            | Panel::Reactions
            | Panel::Appearance => brings.has(self),
            // (Whatever the editor, with nothing open too, as every board
            // draws its switch: with no search yet, it says what fills it.)
            // (Problems too, M4's U20: with nothing found, the kit's empty
            // state.)
            Panel::SearchResults | Panel::Problems => true,
            // (U28's report tabs: while their reports are held.)
            Panel::HakConflicts => app.hak_report.is_some(),
            Panel::ResourcesUsed => app.area_stats.is_some(),
            Panel::ClaimsTable => app.claims.table.is_some(),
            // (M4's U29: while open, Custom Light and Bake beside their
            // area alone.)
            Panel::AreasTogether | Panel::LightMaker | Panel::Bake => {
                crate::tool_sections::shown_beside(app, self, brings.area)
            }
            #[cfg(test)]
            Panel::Sample(n) => n != SAMPLE_EMPTY && brings.has(self),
        }
    }

    /// The arrow keys' owner while it is shown (F6 gives them to it).
    pub(crate) fn keys(self) -> crate::palette_view::Keys {
        use crate::palette_view::Keys;
        match self {
            Panel::Module => Keys::Tree,
            Panel::Resources => Keys::Browser,
            Panel::Palette => Keys::Palette,
            Panel::Preview => Keys::Preview,
            Panel::Outliner => Keys::Outliner,
            Panel::Log => Keys::Log,
            // (No arrows of its own: F6 there gives its first control the
            // focus, `Moonglow::step_pane`.)
            Panel::TileProperties => Keys::TileProperties,
            Panel::Properties => Keys::Properties,
            // (Reference's: F6 gives its filter the focus; Help's: nothing
            // to take them, its text to read.)
            Panel::Reference => Keys::Reference,
            Panel::Help => Keys::Help,
            Panel::Compiler => Keys::Compiler,
            Panel::Bookmarks => Keys::Bookmarks,
            Panel::SearchResults => Keys::SearchResults,
            // (No arrows of their own: F6 gives the panel's first field the
            // keyboard's focus, `dialog_view::begin_panel`.)
            Panel::NodeData => Keys::NodeData,
            Panel::NodeSearch => Keys::NodeSearch,
            Panel::Conversation => Keys::Conversation,
            // (No arrows of their own: F6 gives the panel's first field
            // the keyboard's focus.)
            Panel::Cell => Keys::Cell,
            Panel::References => Keys::TableReferences,
            // (A talk table's, M4's U17: F6 gives String's Text and the
            // Talk Table panel's first control the focus.)
            Panel::String => Keys::TalkString,
            Panel::TalkFile => Keys::TalkFile,
            // (A NUI designer's, M4's U18: F6 gives Find a control… and
            // the Bindings panel's first field the focus.)
            Panel::Components => Keys::NuiComponents,
            Panel::Bindings => Keys::NuiBindings,
            // (F6 gives its first row's control the focus, M4's U19.)
            Panel::Checks => Keys::NuiChecks,
            // (F6 gives its first row's link the focus; Textures' its first
            // row's Copy, M4's U20.)
            Panel::Problems => Keys::Problems,
            Panel::Textures => Keys::Textures,
            // (A hak's Details: F6 gives its Description the focus, M4's U21.)
            Panel::Details => Keys::HakDetails,
            // (A journal's and the faction editor's, M4's U22: F6 gives the
            // panel's first field the focus; Reactions' its first row.)
            Panel::Entry => Keys::JournalEntry,
            Panel::Category => Keys::JournalCategory,
            Panel::Faction => Keys::FactionPanel,
            Panel::Reaction => Keys::Reaction,
            Panel::Reactions => Keys::Reactions,
            // (The Appearance Gallery's, M4's U26: facts to read, nothing to
            // take the keys.)
            Panel::Appearance => Keys::AppearanceFacts,
            // (U28's report tabs: their tables' arrows.)
            Panel::HakConflicts => Keys::HakReport,
            Panel::ResourcesUsed => Keys::ResourcesUsed,
            Panel::ClaimsTable => Keys::ClaimsTable,
            // (M4's U29: Enter their main action, Escape their Cancel.)
            Panel::AreasTogether => Keys::AreasTogether,
            Panel::LightMaker => Keys::LightMaker,
            Panel::Bake => Keys::Bake,
            #[cfg(test)]
            Panel::Sample(_) => Keys::List,
        }
    }
}

/// A section of the Right Dock: one tab bar of its panels, in their tabs'
/// order (the Main board's Palette and Preview; Properties; the Script
/// board's Reference; Help).
pub(crate) type Section = &'static [Panel];

/// What an editor brings to the docks ([`brings_of`]), by its kind: the
/// Left Dock's Outliner, the Right Dock's sections from the top, the
/// order of their panels where they fold into one tab bar (under 1400:
/// Narrow's Palette, Properties, Preview; none, the sections' order), the
/// Bottom Dock's own panels in their tabs' order (a script's Compiler and
/// Bookmarks, the NUI designer's Checks), and whether they go with the
/// editor into a window of its own (`floats`: Floating.png's script, its
/// Reference and Help at its right and its Compiler and Bookmarks under
/// it, [`window_panes`]) or stay in the main window's docks where it
/// floats (the area's palettes: the main window's, beside any area).
/// Static tables: nothing is made each time it is asked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Brings {
    pub(crate) outliner: bool,
    pub(crate) right: &'static [Section],
    pub(crate) folded: &'static [Panel],
    pub(crate) bottom: &'static [Panel],
    pub(crate) floats: bool,
    /// Its Preview is its own drawing, not a model the palettes choose (a
    /// tileset's tile diagram, M4's U20): shown whatever the palettes.
    pub(crate) diagram: bool,
    /// The global Bottom Dock panels that follow the editor into its
    /// window, after its own on the window's bottom pane (Floating.png's
    /// script: Search Results, U13), where it `floats`.
    pub(crate) window_bottom: &'static [Panel],
    /// The sections of `right` that go with the editor into a window of
    /// its own where it does not `floats` (the rest stay the main
    /// window's): a dragged out area's Tile Properties, its Palette and
    /// Preview staying in the main window's Right Dock (U5).
    pub(crate) window_right: &'static [Section],
    /// The sections' shares of the Right Dock's height until the builder
    /// drags a seam, in thousandths of an equal share (none: equal): the
    /// Main board's palettes over Properties, 58 to 42.
    pub(crate) shares: &'static [u16],
    /// The sections' shares in the editor's own window, where they differ
    /// from the dock's (none: `shares`): the Floating board's script,
    /// Reference 44 and Help 56, where the Script board's dock has 56
    /// and 44.
    pub(crate) window_shares: &'static [u16],
    /// The area it is, where it is an area's editor: the one its tool
    /// sections (M4's U29: Custom Light, Bake Walkmesh to Transform) are
    /// shown beside.
    pub(crate) area: Option<mg_core::ResRef>,
}

impl Brings {
    /// Whether it brings `panel` to a side or the Bottom Dock.
    pub(crate) fn has(&self, panel: Panel) -> bool {
        self.right.iter().any(|s| s.contains(&panel)) || self.bottom.contains(&panel)
    }

    /// Whether `panel` goes with the editor into its window
    /// ([`Brings::window_right`]).
    pub(crate) fn window_has(&self, panel: Panel) -> bool {
        self.window_right.iter().any(|s| s.contains(&panel))
    }
}

/// The palettes' section: the Palette and the Preview, one tab bar.
const PALETTES: Section = &[Panel::Palette, Panel::Preview];
/// A blueprint's or placed object's own Preview, alone (M4's U11, the
/// Creature board's Right Dock): its model, in the docks or in the right
/// pane of its window where it floats.
const PREVIEW_SECTION: Section = &[Panel::Preview];
const PREVIEW_ALONE: &[Section] = &[PREVIEW_SECTION];
/// The placeholder's Right Dock (no area: nothing to select), and an
/// area's in Select Tiles with Tile Properties hidden by its toggle: the
/// palettes alone, one section.
const AREA_RIGHT: &[Section] = &[PALETTES];
const AREA_FOLDED: &[Panel] = &[Panel::Palette, Panel::Preview];
/// The area's Right Dock with any tool but Select Tiles (the Main,
/// WideArea, Floating and AreaPaint boards): the Palette and the Preview,
/// and under them Properties; folded into one tab bar under 1400 as
/// Narrow draws it, Palette, Properties, Preview.
const AREA_PROPS_RIGHT: &[Section] = &[PALETTES, PROPS_SECTION];
const AREA_PROPS_FOLDED: &[Panel] = &[Panel::Palette, Panel::Properties, Panel::Preview];
/// The Main board's shares: the palettes' section 58, Properties 42.
const AREA_PROPS_SHARES: &[u16] = &[1160, 840];
/// Properties' section, alone (where the palettes are not shown, and in a
/// dragged out area's window).
const PROPS_SECTION: Section = &[Panel::Properties];
const PROPS_ALONE: &[Section] = &[PROPS_SECTION];
/// The area's Right Dock while Select Tiles is on (the AreaTiles board):
/// the Palette and the Preview, and under them Tile Properties; folded
/// as the sections stand (no board draws them folded).
const AREA_TILES_RIGHT: &[Section] = &[PALETTES, TILES_SECTION];
const AREA_TILES_FOLDED: &[Panel] = &[Panel::Palette, Panel::Preview, Panel::TileProperties];
/// Tile Properties' section, alone (where the palettes are not shown, and
/// in a dragged out area's window).
const TILES_SECTION: Section = &[Panel::TileProperties];
const TILES_ALONE: &[Section] = &[TILES_SECTION];
/// A script's Right Dock (M4's U12, the Script board's): Reference, with
/// the script into its window where it floats (Floating.png). Help, under
/// Reference on the boards, went back under the text in 5.2.0 (the owner's
/// ruling on Jasperre's report: a narrow box at the right was worse than
/// 4.2.1's pane as wide as the text).
const SCRIPT_RIGHT: &[Section] = &[&[Panel::Reference]];
const SCRIPT_FOLDED: &[Panel] = &[Panel::Reference];
/// A script's own Bottom Dock panels (M4's U13, the Script board's tabs
/// before Search Results and the Log; Help among them since 5.2.0, after
/// Compiler as 4.2.1's pane under the text had it), and the global one that
/// follows it into its window's bottom pane (Floating.png: Compiler,
/// Bookmarks, Search Results).
const SCRIPT_BOTTOM: &[Panel] = &[Panel::Compiler, Panel::Help, Panel::Bookmarks];
const SCRIPT_WINDOW_BOTTOM: &[Panel] = &[Panel::SearchResults];
/// A conversation's Right Dock (M4's U14, the Conversation board's): the
/// Node section, its title before Data and Search, and the Conversation
/// panel under it with its header; one tab bar of the three under 1400;
/// with the conversation into its window where it floats. The board's
/// shares: the Node section takes what the Conversation panel's 120
/// points leave in the board's dock, 690 of 811 (the dock's height less
/// the line between them).
const DIALOG_RIGHT: &[Section] = &[&[Panel::NodeData, Panel::NodeSearch], &[Panel::Conversation]];
const DIALOG_FOLDED: &[Panel] = &[Panel::NodeData, Panel::NodeSearch, Panel::Conversation];
const DIALOG_SHARES: &[u16] = &[1702, 298];
/// In its own window, shorter, the Conversation panel keeps room for its
/// three rows (Normal end, Aborted, Stop camera zoom in).
const DIALOG_WINDOW_SHARES: &[u16] = &[1600, 400];
/// A table's Right Dock (M4's U16, the Table board's): Cell over
/// References, each a section with its header; one tab bar of the two
/// under 1400; with the table into its window where it floats. The
/// board's shares: Cell's 372 points of the dock's 811.
const TABLE_RIGHT: &[Section] = &[&[Panel::Cell], &[Panel::References]];
const TABLE_FOLDED: &[Panel] = &[Panel::Cell, Panel::References];
const TABLE_SHARES: &[u16] = &[918, 1082];
/// A talk table's Right Dock (M4's U17, the Talk board's): String over
/// References over Talk Table, each a section with its header; one tab bar
/// of the three under 1400; with the table into its window where it
/// floats. The board's shares: String's 284 points of the dock's 811,
/// References' 124, Talk Table the rest.
const TALK_RIGHT: &[Section] = &[&[Panel::String], &[Panel::References], &[Panel::TalkFile]];
const TALK_FOLDED: &[Panel] = &[Panel::String, Panel::References, Panel::TalkFile];
const TALK_SHARES: &[u16] = &[700, 306, 994];
/// In its own window, shorter, String keeps the room for every field at
/// the window's first size (Text, Sound and Length whole in a window 600
/// high: M4's U17 review).
const TALK_WINDOW_SHARES: &[u16] = &[1100, 300, 600];

/// A NUI designer's Right Dock (M4's U18, the NUI board's): Components over
/// Properties over Bindings, each a section with its header; one tab bar of
/// the three under 1400; with the designer into its window where it floats,
/// its Layers the Outliner. The board's heights: Components 166 and
/// Bindings 222 of the dock's 811, Properties the rest.
const NUI_RIGHT: &[Section] = &[&[Panel::Components], &[Panel::Properties], &[Panel::Bindings]];
const NUI_FOLDED: &[Panel] = &[Panel::Components, Panel::Properties, Panel::Bindings];
const NUI_SHARES: &[u16] = &[410, 1042, 548];
/// In its own window, shorter, each keeps something usable at the window's
/// first size in a 1280 by 720 screen (M4's U18 review): Components its
/// search and a row of its components, Properties the chosen control's
/// binding and its Content whole, Bindings its first words and Create Bind.
const NUI_WINDOW_SHARES: &[u16] = &[440, 1040, 520];
/// A NUI designer's own Bottom Dock panel (M4's U19, the NUI board's tab
/// before Search Results and the Log): its Checks, with the designer into
/// its window's bottom pane where it floats.
const NUI_BOTTOM: &[Panel] = &[Panel::Checks];

/// A tileset's Right Dock (M4's U20, the Tileset board's): Preview over
/// Textures, each a section with its header; one tab bar of the two under
/// 1400; with the tileset into its window where it floats. The board's
/// heights: Preview 419 of the dock's 831, Textures the rest.
const TILESET_RIGHT: &[Section] = &[&[Panel::Preview], &[Panel::Textures]];
const TILESET_FOLDED: &[Panel] = &[Panel::Preview, Panel::Textures];
const TILESET_SHARES: &[u16] = &[1008, 992];
/// A hak's Right Dock (M4's U21, the Hak board's): Preview over Details,
/// each a section with its header; one tab bar of the two under 1400; with
/// the hak into its window where it floats. The board's heights: Details
/// 300 of the dock's 831, Preview the rest.
const HAK_RIGHT: &[Section] = &[&[Panel::Preview], &[Panel::Details]];
const HAK_FOLDED: &[Panel] = &[Panel::Preview, Panel::Details];
const HAK_SHARES: &[u16] = &[1278, 722];
/// The Appearance Gallery's Right Dock (M4's U26, the AppearanceGallery
/// board's): Preview over Appearance, each a section with its header; one
/// tab bar of the two under 1400; with the gallery into its window where
/// it floats. The board's heights: Preview 60% of the dock, Appearance
/// the rest.
const GALLERY_RIGHT: &[Section] = &[&[Panel::Preview], &[Panel::Appearance]];
const GALLERY_FOLDED: &[Panel] = &[Panel::Preview, Panel::Appearance];
const GALLERY_SHARES: &[u16] = &[1200, 800];
/// A journal's Right Dock (M4's U22, the Journal board's): Entry over
/// Category, each a section with its header; one tab bar of the two under
/// 1400; with the journal into its window where it floats. The board's
/// heights: Entry 454 of the dock's 844, Category the rest.
const JOURNAL_RIGHT: &[Section] = &[&[Panel::Entry], &[Panel::Category]];
const JOURNAL_FOLDED: &[Panel] = &[Panel::Entry, Panel::Category];
const JOURNAL_SHARES: &[u16] = &[1076, 924];
/// The faction editor's Right Dock in Advanced (M4's U22, the Factions
/// board's): Faction over Reaction over Reactions, each a section with its
/// header; one tab bar of the three under 1400; with the editor into its
/// window where it floats. The board's heights: Faction 132 and Reaction
/// 148 of the dock's 844, Reactions the rest.
const FACTIONS_RIGHT: &[Section] = &[&[Panel::Faction], &[Panel::Reaction], &[Panel::Reactions]];
const FACTIONS_FOLDED: &[Panel] = &[Panel::Faction, Panel::Reaction, Panel::Reactions];
const FACTIONS_SHARES: &[u16] = &[469, 526, 2005];
/// In Basic the editor is the chart of how the chosen faction regards the
/// others (the moves' "Not Drawn" Basic chart view): Reactions is not
/// drawn beside it, Faction and Reaction are.
const FACTIONS_BASIC_RIGHT: &[Section] = &[&[Panel::Faction], &[Panel::Reaction]];
const FACTIONS_BASIC_FOLDED: &[Panel] = &[Panel::Faction, Panel::Reaction];
const FACTIONS_BASIC_SHARES: &[u16] = &[313, 1687];

/// The talk table an editor's tab is, if it is one's.
pub(crate) fn talk_of(
    app: &Moonglow,
    editor: Option<&crate::Tab>,
) -> Option<crate::talk_view::Which> {
    crate::talk_view::Which::of(app, editor?)
}

/// What the editor `tab` brings to the docks (the plan's decision 2,
/// `editor_brings` by kind): an area's tab the Outliner (with a module
/// open: what is placed is read from the module) and, with the game's
/// data read and a module open (the palettes are the game's), the
/// Palette and the Preview; under them, with a module open, the lower
/// section by the tool in hand (M4's U6): Tile Properties in Select
/// Tiles (none there while its header toggle hides it), Properties with
/// every other tool (Select Objects, Paint, Terrain), whatever is
/// selected; each also the one section that goes into the area's window
/// where it is dragged out. The placeholder that stands for no area
/// brings the palettes alone (it holds nothing to list or select, and no
/// panel is shown empty). A script's tab brings its Reference over its
/// Help (M4's U12), and its Compiler and Bookmarks to the Bottom Dock
/// (U13), Search Results after them in its window where it floats. Any other tab brings none until M4 gives it its
/// own: the Left Dock is then its tabs' panel alone, the whole height,
/// and the Right Dock is not drawn, folded or not, its sizes kept.
pub(crate) fn brings_of(app: &Moonglow, tab: &crate::Tab) -> Brings {
    let owner = owner_of(app, tab);
    let tab = &*owner;
    #[cfg(test)]
    if let Some((kind, brings)) = app.sample_brings
        && kind == tab.kind()
    {
        return brings;
    }
    let palettes = app.game.is_some() && app.ws.is_some();
    let right = if palettes { AREA_RIGHT } else { &[] };
    match tab {
        // (While Select Tiles is on, and Tile Properties not hidden by its
        // header toggle: under the palettes, the AreaTiles board's lower
        // section; in the area's own window where it is dragged out.)
        crate::Tab::Area(area) if app.ws.is_some() && tile_properties_shown(app, *area) => {
            let right = if palettes { AREA_TILES_RIGHT } else { TILES_ALONE };
            let folded = AREA_TILES_FOLDED;
            let area = Some(*area);
            Brings {
                outliner: true,
                right,
                folded,
                window_right: TILES_ALONE,
                area,
                ..Brings::default()
            }
        }
        // (Select Tiles with Tile Properties hidden: the palettes alone,
        // not the objects' Properties, which Select Tiles does not
        // select.)
        crate::Tab::Area(area) if app.ws.is_some() && tiles_in_hand(app, *area) => {
            Brings { outliner: true, right, area: Some(*area), ..Brings::default() }
        }
        // (Any other tool: Properties under the palettes, the Main board's
        // lower section; in the area's own window where it is dragged
        // out.)
        crate::Tab::Area(area) if app.ws.is_some() => {
            let props = area_properties_shown(app, *area);
            let right = match (palettes, props) {
                (true, true) => AREA_PROPS_RIGHT,
                (true, false) => AREA_RIGHT,
                (false, true) => PROPS_ALONE,
                (false, false) => &[],
            };
            let folded = if props { AREA_PROPS_FOLDED } else { AREA_FOLDED };
            let shares = AREA_PROPS_SHARES;
            let window_props =
                !app.settings.hide_properties && !app.props_popped_out.contains(area);
            let window_right = if window_props { PROPS_ALONE } else { &[] };
            let area = Some(*area);
            Brings {
                outliner: true,
                right,
                folded,
                window_right,
                shares,
                area,
                ..Brings::default()
            }
        }
        crate::Tab::Area(_) => Brings { right, ..Brings::default() },
        crate::Tab::NoArea => Brings { right, ..Brings::default() },
        // (A script's Reference and Help, whatever is open: its own
        // symbols are listed without the game's; with it into its window
        // where it floats. The Right Dock stays while the script's group
        // has the focus, the owner's word at T9.)
        crate::Tab::Script(_) => Brings {
            right: SCRIPT_RIGHT,
            folded: SCRIPT_FOLDED,
            bottom: SCRIPT_BOTTOM,
            window_bottom: SCRIPT_WINDOW_BOTTOM,
            floats: true,
            ..Brings::default()
        },
        // (A conversation's Node and Conversation panels, whatever is
        // open; with it into its window where it floats. The Right Dock
        // stays while the conversation's group has the focus, as a
        // script's does.)
        crate::Tab::Dialog(_) => Brings {
            outliner: true,
            right: DIALOG_RIGHT,
            folded: DIALOG_FOLDED,
            shares: DIALOG_SHARES,
            window_shares: DIALOG_WINDOW_SHARES,
            floats: true,
            ..Brings::default()
        },
        // (A table's Cell and References and its columns in the
        // Outliner, whatever is open (a table is no module's alone); with
        // it into its window where it floats.)
        // (A talk table's String, References and Talk Table, the game's
        // dialog.tlk's too, whatever is open; with it into its window
        // where it floats.)
        crate::Tab::TalkTable => Brings {
            right: TALK_RIGHT,
            folded: TALK_FOLDED,
            shares: TALK_SHARES,
            window_shares: TALK_WINDOW_SHARES,
            floats: true,
            ..Brings::default()
        },
        // (A NUI designer's Components, Properties and Bindings and its
        // Layers in the Outliner, M4's U18, with a module open, on its
        // Design page in the Editor view as its three panes were; with it
        // into its window where it floats.)
        crate::Tab::Nui(_) if app.ws.is_some() => Brings {
            outliner: true,
            right: NUI_RIGHT,
            folded: NUI_FOLDED,
            shares: NUI_SHARES,
            window_shares: NUI_WINDOW_SHARES,
            bottom: NUI_BOTTOM,
            floats: true,
            ..Brings::default()
        },
        // (A tileset's Preview over its Textures and its parts in the
        // Outliner, M4's U20, whatever is open (a tileset is a file of its
        // own); with it into its window where it floats.)
        crate::Tab::Tileset(_) => Brings {
            outliner: true,
            right: TILESET_RIGHT,
            folded: TILESET_FOLDED,
            shares: TILESET_SHARES,
            floats: true,
            diagram: true,
            ..Brings::default()
        },
        // (A hak's Preview over its Details and its types in the Outliner,
        // M4's U21, whatever is open (a hak is a file of its own); with it
        // into its window where it floats. Its Preview its own, whatever
        // the palettes.)
        crate::Tab::Hak(_) => Brings {
            outliner: true,
            right: HAK_RIGHT,
            folded: HAK_FOLDED,
            shares: HAK_SHARES,
            floats: true,
            diagram: true,
            ..Brings::default()
        },
        // (A journal's Entry and Category, M4's U22, with a module open;
        // with it into its window where it floats. No Outliner: the board's
        // Left Dock is the module tree alone.)
        crate::Tab::Journal if app.ws.is_some() => Brings {
            right: JOURNAL_RIGHT,
            folded: JOURNAL_FOLDED,
            shares: JOURNAL_SHARES,
            floats: true,
            ..Brings::default()
        },
        // (The faction editor's Faction, Reaction and Reactions and its
        // factions in the Outliner, M4's U22, with a module open; Reactions
        // in Advanced alone. With it into its window where it floats.)
        crate::Tab::Factions if app.ws.is_some() => {
            let (right, folded, shares) = match app.faction_view.advanced {
                true => (FACTIONS_RIGHT, FACTIONS_FOLDED, FACTIONS_SHARES),
                false => (FACTIONS_BASIC_RIGHT, FACTIONS_BASIC_FOLDED, FACTIONS_BASIC_SHARES),
            };
            Brings { outliner: true, right, folded, shares, floats: true, ..Brings::default() }
        }
        // (The Appearance Gallery's Preview over Appearance, M4's U26, with
        // the game's data read, whatever is open (Welcome opens it with no
        // module); with it into its window where it floats. Its Preview its
        // own, whatever the palettes.)
        crate::Tab::PlaceableGallery if app.game.is_some() => Brings {
            right: GALLERY_RIGHT,
            folded: GALLERY_FOLDED,
            shares: GALLERY_SHARES,
            floats: true,
            diagram: true,
            ..Brings::default()
        },
        crate::Tab::Table(_) => Brings {
            outliner: true,
            right: TABLE_RIGHT,
            folded: TABLE_FOLDED,
            shares: TABLE_SHARES,
            floats: true,
            ..Brings::default()
        },
        // (A blueprint's or a placed object's Properties form with a model
        // of its own, the game's data read: its own Preview, alone, the
        // Creature board's; with it into its window where it floats.)
        tab if app.game.is_some()
            && crate::model_view::Source::of_editor(tab).is_some_and(|s| s.has_model()) =>
        {
            Brings { right: PREVIEW_ALONE, floats: true, ..Brings::default() }
        }
        _ => Brings::default(),
    }
}

/// Whether `area`'s editor shows Tile Properties: Select Tiles is on in
/// its view (as the view was last drawn, where it is being drawn now) and
/// the header's Tile Properties toggle has not hidden it
/// (`Settings::hide_tile_properties`).
pub(crate) fn tile_properties_shown(app: &Moonglow, area: mg_core::ResRef) -> bool {
    let tiles = match app.area_views.get(&area) {
        Some(view) => view.tile_mode && view.read_only.is_none(),
        None => app.tile_mode_areas.contains(&area),
    };
    tiles && !app.settings.hide_tile_properties
}

/// Whether Select Tiles is the tool in hand in `area`'s view (as the view
/// was last drawn, where it is being drawn now), whatever the header's
/// Tile Properties toggle says.
fn tiles_in_hand(app: &Moonglow, area: mg_core::ResRef) -> bool {
    match app.area_views.get(&area) {
        Some(view) => view.tile_mode && view.read_only.is_none(),
        None => app.tile_mode_areas.contains(&area),
    }
}

/// The editor a tab belongs to, whose panels it brings and whose layout
/// of the docks the settings keep: the tab itself, but for a tab that is
/// part of another editor: the NUI designer's source (M4's U19, a second
/// editor group of the designer's) is the designer's tab, so that focusing
/// it keeps the designer's Right Dock, Outliner and Checks. A script's
/// group brings its own (the owner's ruling at T9's review).
pub(crate) fn owner_of<'a>(
    app: &Moonglow,
    tab: &'a crate::Tab,
) -> std::borrow::Cow<'a, crate::Tab> {
    let _ = app;
    match tab {
        crate::Tab::NuiSource(key) => std::borrow::Cow::Owned(crate::Tab::Nui(*key)),
        tab => std::borrow::Cow::Borrowed(tab),
    }
}

/// The kind a tab's layout of the docks is kept under
/// (`Settings::dock_layouts`): its owner's `Tab::kind`, the placeholder
/// that stands for no area the area's (they bring the same section).
pub(crate) fn layout_kind(app: &Moonglow, tab: &crate::Tab) -> &'static str {
    match &*owner_of(app, tab) {
        crate::Tab::NoArea => crate::Tab::Area(mg_core::ResRef::default()).kind(),
        owner => owner.kind(),
    }
}

/// What the active docked editor ([`Moonglow::active_editor`], in either
/// mode of the Open editors switch) brings to the docks ([`brings_of`]):
/// a floating window's editor never fills the main window's docks.
pub(crate) fn editor_brings(app: &Moonglow) -> Brings {
    app.active_editor().map_or_else(Brings::default, |tab| brings_of(app, tab))
}

/// The kind of the active docked editor ([`layout_kind`]), whose layout
/// of the docks the settings keep (`Settings::dock_layouts`): every kind
/// keeps its fronts, the area's too (the owner's ruling of 2026-11-15).
fn editor_kind(app: &Moonglow) -> Option<&'static str> {
    app.active_editor().map(|tab| layout_kind(app, tab))
}

/// The docks follow the active editor: the Palette asked for by name
/// (Tools › Palettes beside a tab that brings none, or with no module) is
/// held until the active editor brings it itself; from then on the plain
/// rule has it ([`editor_brings`]), and a tab that brings none takes the
/// Right Dock away again.
pub(crate) fn follow_editor(app: &mut Moonglow) {
    if app.palettes_asked && editor_brings(app).has(Panel::Palette) {
        app.palettes_asked = false;
    }
}

/// The Right Dock's sections now, from the top, each its available
/// panels in their tabs' order (none: the dock is not drawn): those the
/// active editor brings, and before them the palettes' where they were
/// asked for by name beside an editor that brings none.
pub(crate) fn right_sections(app: &Moonglow) -> Vec<Vec<Panel>> {
    let brings = editor_brings(app);
    let held =
        |s: Section| -> Vec<Panel> { s.iter().copied().filter(|p| p.available(app)).collect() };
    let mut sections: Vec<Vec<Panel>> =
        brings.right.iter().map(|s| held(s)).filter(|s| !s.is_empty()).collect();
    // (Their Preview only where the kind brings none of its own.)
    if Panel::Palette.available(app) && !brings.has(Panel::Palette) {
        let asked = PALETTES.iter().copied();
        sections.insert(0, asked.filter(|p| *p == Panel::Palette || !brings.has(*p)).collect());
    }
    // (M4's U29: the tools open, a section of their own under the rest,
    // whatever the editor brings; first in the one tab bar under 1400, as
    // no editor's `folded` names them.)
    let tools = crate::tool_sections::section(app, brings.area);
    if !tools.is_empty() {
        sections.push(tools);
    }
    sections
}

/// The panels `dock` holds now, in their tabs' order (none: the dock is
/// not drawn): the Right Dock's sections as they fold into one tab bar
/// (the editor's `folded` order, else the sections' own), the Bottom
/// Dock's the editor's own and then the global ones.
pub(crate) fn panels(app: &Moonglow, dock: Dock) -> Vec<Panel> {
    match dock {
        Dock::Left => Panel::LEFT.into_iter().filter(|p| p.available(app)).collect(),
        Dock::Right => folded_order(&right_sections(app), editor_brings(app).folded),
        Dock::Bottom => {
            let own = editor_brings(app).bottom.iter();
            own.chain(Panel::BOTTOM_TABS).copied().filter(|p| p.available(app)).collect()
        }
    }
}

/// The sections' panels in one tab bar: by their places in `folded` (the
/// editor's order), those it leaves out first in the sections' order.
pub(crate) fn folded_order(sections: &[Vec<Panel>], folded: &[Panel]) -> Vec<Panel> {
    let mut all = sections.concat();
    if !folded.is_empty() {
        all.sort_by_key(|p| folded.iter().position(|q| q == p).map_or(0, |i| i + 1));
    }
    all
}

/// The Bottom Dock's switches in the Status Bar, in their order: the
/// global ones before the editor's own (the Log's), the editor's own, the
/// global ones after (the Script board's Search Results), of those it
/// holds.
pub(crate) fn switches(app: &Moonglow) -> Vec<Panel> {
    let own = editor_brings(app).bottom;
    (Panel::SWITCHES_BEFORE.iter().chain(own).chain(Panel::SWITCHES_AFTER))
        .copied()
        .filter(|p| p.available(app))
        .collect()
}

/// How the Right Dock stands, its sections (`sections`, from the top) laid
/// out as columns, each of sections, each a tab bar of panels: under 1400
/// (`narrow`, section 5.2) one tab bar of them all in the `folded` order;
/// in the wide layout (`columns`, a dock wider than two narrow ones) the
/// first section in the first column and the rest in the second
/// (WideArea), a lone section of more than one panel standing as its
/// first panel and the rest (the Palette | the Preview, as T14 left it
/// until U6 brings Properties); else the sections stacked in one column.
pub(crate) fn arrange(
    sections: &[Vec<Panel>],
    folded: &[Panel],
    narrow: bool,
    columns: bool,
) -> Vec<Vec<Vec<Panel>>> {
    if sections.is_empty() {
        return Vec::new();
    }
    if narrow {
        return vec![vec![folded_order(sections, folded)]];
    }
    if columns {
        return match sections {
            [one] if one.len() > 1 => vec![vec![one[..1].to_vec()], vec![one[1..].to_vec()]],
            [one] => vec![vec![one.clone()]],
            [first, rest @ ..] => vec![vec![first.clone()], rest.to_vec()],
            [] => Vec::new(),
        };
    }
    vec![sections.to_vec()]
}

fn to_area_panel(p: Panel) -> Option<crate::settings::AreaRightPanel> {
    match p {
        Panel::Palette => Some(crate::settings::AreaRightPanel::Palette),
        Panel::Preview => Some(crate::settings::AreaRightPanel::Preview),
        Panel::Properties => Some(crate::settings::AreaRightPanel::Properties),
        _ => None,
    }
}

fn from_area_panel(p: crate::settings::AreaRightPanel) -> Panel {
    match p {
        crate::settings::AreaRightPanel::Palette => Panel::Palette,
        crate::settings::AreaRightPanel::Preview => Panel::Preview,
        crate::settings::AreaRightPanel::Properties => Panel::Properties,
    }
}

/// Whether `Panel::Properties` is currently shown in the main Right Dock
/// for `area`: hidden while Select Tiles is in hand, while manually hidden,
/// while popped out into an Instance/Instances tab or window, or in the
/// ultrawide layout while nothing editable is selected.
pub(crate) fn area_properties_shown(app: &Moonglow, area: mg_core::ResRef) -> bool {
    !tiles_in_hand(app, area)
        && !app.settings.hide_properties
        && !app.props_popped_out.contains(&area)
        && (!app.yields.wide || wide_editable(app, area))
}

/// Whether something editable is selected in `area`, for the wide
/// layout's auto-hide: as [`hold_wide_properties`] last let it change for
/// the active area (live for another).
fn wide_editable(app: &Moonglow, area: mg_core::ResRef) -> bool {
    match app.wide_props {
        Some((a, editable)) if a == area => editable,
        _ => crate::properties_panel::editable_selected(app, Some(&crate::Tab::Area(area))),
    }
}

/// The wide layout's auto-hide of Properties (its column given back while
/// nothing editable is selected) changes the Right Dock's width, and so
/// moves the area view's scene. It is let change only while no pointer
/// button is down and, after a press in an area view, once egui's
/// double-click time has passed since it was let go: a click, a double
/// click and a drag all end on the scene as it stood when they began (a
/// double click on a chest had its second click land 200 points away).
pub(crate) fn hold_wide_properties(app: &mut Moonglow, ctx: &egui::Context) {
    app.wide_props_waiting = false;
    let area = match app.active_editor() {
        Some(crate::Tab::Area(area)) if app.yields.wide => *area,
        _ => {
            app.wide_props = None;
            return;
        }
    };
    let (down, pressed, released, at, now) = ctx.input(|i| {
        let p = &i.pointer;
        (p.any_down(), p.any_pressed(), p.any_released(), p.interact_pos(), i.time)
    });
    let in_area = at.is_some_and(|p| app.area_views.values().any(|v| v.rect.contains(p)));
    if pressed {
        app.area_press.1 = in_area;
    }
    if released && std::mem::take(&mut app.area_press.1) {
        app.area_press.0 = Some(now);
    }
    let delay = ctx.options(|o| o.input_options.max_double_click_delay);
    let wait = app.area_press.0.map_or(0.0, |t| t + delay - now);
    let editable = crate::properties_panel::editable_selected(app, Some(&crate::Tab::Area(area)));
    match app.wide_props {
        Some((a, was)) if a == area && was != editable && (down || wait > 0.0) => {
            app.wide_props_waiting = true;
            if !down {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
            }
        }
        _ => app.wide_props = Some((area, editable)),
    }
}

fn active_area_right_base(app: &Moonglow) -> crate::settings::AreaRightLayout {
    app.settings
        .area_right_layout
        .as_ref()
        .map(crate::settings::AreaRightLayout::normalized)
        .unwrap_or_else(|| {
            if app.yields.wide {
                crate::settings::AreaRightLayout::two_columns()
            } else {
                crate::settings::AreaRightLayout::stacked()
            }
        })
}

fn customizable_area(app: &Moonglow, editor: Option<&crate::Tab>) -> Option<mg_core::ResRef> {
    let Some(crate::Tab::Area(area)) = editor else { return None };
    #[cfg(test)]
    if app.sample_brings.is_some() {
        return None;
    }
    if app.ws.is_none() || app.game.is_none() || tiles_in_hand(app, *area) {
        return None;
    }
    Some(*area)
}

fn area_visible_columns(app: &Moonglow, area: mg_core::ResRef) -> Vec<Vec<Vec<Panel>>> {
    let base = active_area_right_base(app);
    let raw = match area_properties_shown(app, area) {
        true => base.columns,
        false => base.without_properties(),
    };
    raw.into_iter()
        .filter_map(|col| {
            let kept: Vec<Vec<Panel>> = col
                .into_iter()
                .filter_map(|sec| {
                    let s: Vec<Panel> =
                        sec.into_iter().map(from_area_panel).filter(|p| p.available(app)).collect();
                    (!s.is_empty()).then_some(s)
                })
                .collect();
            (!kept.is_empty()).then_some(kept)
        })
        .collect()
}

fn area_wants_one_column(app: &Moonglow) -> bool {
    let Some(area) = customizable_area(app, app.active_editor()) else { return false };
    area_visible_columns(app, area).len() <= 1
}

/// Arranges an Area editor's Right Dock columns according to the builder's
/// custom layout (or default), window width tier, Properties visibility,
/// and any plugin tool sections.
fn arrange_area(
    app: &Moonglow,
    area: mg_core::ResRef,
    sections: &[Vec<Panel>],
    folded_as: &[Panel],
    narrow: bool,
    columns_fit: bool,
) -> Vec<Vec<Vec<Panel>>> {
    let tools = crate::tool_sections::section(app, Some(area));
    if narrow {
        if app.settings.area_right_layout.is_none() {
            return arrange(sections, folded_as, true, false);
        }
        let mut flat: Vec<Panel> = if !tools.is_empty() { tools } else { Vec::new() };
        for p in area_visible_columns(app, area).into_iter().flatten().flatten() {
            if !flat.contains(&p) {
                flat.push(p);
            }
        }
        return if flat.is_empty() { Vec::new() } else { vec![vec![flat]] };
    }
    let mut cols = area_visible_columns(app, area);
    if (!app.yields.wide || !columns_fit) && cols.len() > 1 {
        let single: Vec<Vec<Panel>> = cols.into_iter().flatten().collect();
        cols = vec![single];
    }
    if !tools.is_empty() {
        if let Some(last) = cols.last_mut() {
            last.push(tools);
        } else {
            cols.push(vec![tools]);
        }
    }
    cols
}

#[derive(Clone)]
struct AreaRightMenuState {
    wide: bool,
    base: crate::settings::AreaRightLayout,
    props_shown: bool,
    can_show_props: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AreaRightCommand {
    SetLayout(Option<crate::settings::AreaRightLayout>),
    HideProperties,
    ShowProperties(mg_core::ResRef),
    ResetLayout,
}

type AreaSectionBox = DropSection<Panel>;

/// `dragged` dropped at `zone` in the area's Right Dock as `layout`
/// arranges it: the arrangement it makes (the kit's rules, `StackLayout::apply`).
fn apply_area_drop(
    layout: &crate::settings::AreaRightLayout,
    dragged: crate::settings::AreaRightPanel,
    zone: StackZone,
) -> crate::settings::AreaRightLayout {
    layout.stack().apply(&crate::settings::AREA_RIGHT_RULES, dragged, zone).into()
}

/// A Palette, Preview or Properties tab or header dragged in an area's
/// Right Dock (`can`: the dock takes a custom arrangement here): where it
/// would land outlined, and the arrangement changed there once the button
/// is let go.
fn area_drop(
    app: &mut Moonglow,
    ui: &egui::Ui,
    pane: egui::Rect,
    can: bool,
    section_boxes: &[AreaSectionBox],
) {
    let Some(dragged) = app.right_dock_drag.filter(|_| can) else {
        app.right_dock_drag = None;
        return;
    };
    let pointer = ui.ctx().pointer_interact_pos().or_else(|| ui.ctx().pointer_latest_pos());
    let drop = pointer.and_then(|pos| area_drop_target(app, pane, dragged, pos, section_boxes));
    if let Some((_, preview)) = drop {
        mg_uikit::stack::paint_drop_preview(ui.ctx(), preview);
    }
    if !ui.input(|i| i.pointer.any_down()) {
        if let Some((zone, _)) = drop
            && let Some(dragged_ap) = to_area_panel(dragged)
        {
            let base = active_area_right_base(app);
            app.settings.area_right_layout = Some(apply_area_drop(&base, dragged_ap, zone));
            app.right_panel = Some(dragged);
        }
        app.right_dock_drag = None;
    }
}

/// Where `dragged` lands in the area's Right Dock with the pointer at
/// `pos` (the kit's [`StackDrop`]): the Right Dock alone takes it, so a
/// panel let go over the Left or Bottom Dock lands nowhere.
fn area_drop_target(
    app: &Moonglow,
    pane: egui::Rect,
    dragged: Panel,
    pos: egui::Pos2,
    boxes: &[AreaSectionBox],
) -> Option<(StackZone, egui::Rect)> {
    let dragged_ap = to_area_panel(dragged)?;
    let rules = &crate::settings::AREA_RIGHT_RULES;
    let right = StackDrop {
        pane,
        sections: boxes,
        bar_height: mg_uikit::metrics::DOCK_TABS + 4.0,
        new_columns: app.yields.wide
            && active_area_right_base(app).stack().room_for_column(rules, dragged_ap),
    };
    let target = DockDrop::new().dock(Dock::Right, right).target(dragged, pos)?;
    Some((target.zone, target.preview))
}

fn show_area_context_menu(
    ui: &mut egui::Ui,
    area: mg_core::ResRef,
    menu: &AreaRightMenuState,
    panel: Panel,
    cmd: &mut Option<AreaRightCommand>,
) {
    use crate::settings::AreaRightLayout;
    mg_uikit::menu_rows(ui);
    if menu.wide {
        let active = menu.base == AreaRightLayout::two_columns();
        if mg_uikit::menu_radio(ui, active, "Two Columns").clicked() {
            *cmd = Some(AreaRightCommand::SetLayout(Some(AreaRightLayout::two_columns())));
            ui.close();
        }
    }
    if mg_uikit::menu_radio(ui, menu.base == AreaRightLayout::stacked(), "Stacked (Half-Height)")
        .clicked()
    {
        *cmd = Some(AreaRightCommand::SetLayout(Some(AreaRightLayout::stacked())));
        ui.close();
    }
    if mg_uikit::menu_radio(ui, menu.base == AreaRightLayout::one_tab_bar(), "One Tab Bar")
        .clicked()
    {
        *cmd = Some(AreaRightCommand::SetLayout(Some(AreaRightLayout::one_tab_bar())));
        ui.close();
    }
    if let Some(ap) = to_area_panel(panel)
        && let Some((c, s, i)) = menu.base.stack().find(ap)
    {
        mg_uikit::menu_separator(ui);
        let sec = &menu.base.columns[c][s];
        if sec.len() > 1 {
            if i > 0 && ui.button("Move Tab Left").clicked() {
                *cmd = Some(AreaRightCommand::SetLayout(Some(apply_area_drop(
                    &menu.base,
                    ap,
                    StackZone::Tab { col: c, sec: s, index: i - 1 },
                ))));
                ui.close();
            }
            if i + 1 < sec.len() && ui.button("Move Tab Right").clicked() {
                *cmd = Some(AreaRightCommand::SetLayout(Some(apply_area_drop(
                    &menu.base,
                    ap,
                    StackZone::Tab { col: c, sec: s, index: i + 1 },
                ))));
                ui.close();
            }
            if ui.button("Split into Own Section").clicked() {
                *cmd = Some(AreaRightCommand::SetLayout(Some(apply_area_drop(
                    &menu.base,
                    ap,
                    StackZone::Below { col: c, sec: s },
                ))));
                ui.close();
            }
        }
        if s > 0 && ui.button("Move Section Up").clicked() {
            *cmd = Some(AreaRightCommand::SetLayout(Some(apply_area_drop(
                &menu.base,
                ap,
                StackZone::Above { col: c, sec: s - 1 },
            ))));
            ui.close();
        }
        if s + 1 < menu.base.columns[c].len() && ui.button("Move Section Down").clicked() {
            *cmd = Some(AreaRightCommand::SetLayout(Some(apply_area_drop(
                &menu.base,
                ap,
                StackZone::Below { col: c, sec: s + 1 },
            ))));
            ui.close();
        }
        for (oc, ocol) in menu.base.columns.iter().enumerate() {
            for (os, osec) in ocol.iter().enumerate() {
                if oc == c && os == s {
                    continue;
                }
                let names: Vec<_> = osec.iter().map(|&p| from_area_panel(p).name()).collect();
                let label = format!("Dock with {}", names.join(", "));
                if ui.button(label).clicked() {
                    *cmd = Some(AreaRightCommand::SetLayout(Some(apply_area_drop(
                        &menu.base,
                        ap,
                        StackZone::Into { col: oc, sec: os },
                    ))));
                    ui.close();
                }
            }
        }
        if menu.wide {
            let can_left = c == 1;
            let can_right = c == 0
                && (menu.base.columns.len() == 2
                    || sec.len() > 1
                    || menu.base.columns[0].len() > 1);
            if can_left && ui.button("Move to Left Column").clicked() {
                let next = if menu.base.columns.len() == 2
                    && sec.len() == 1
                    && menu.base.columns[c].len() == 1
                {
                    apply_area_drop(&menu.base, ap, StackZone::Column { last: false })
                } else {
                    apply_area_drop(
                        &menu.base,
                        ap,
                        StackZone::Below { col: 0, sec: menu.base.columns[0].len() - 1 },
                    )
                };
                *cmd = Some(AreaRightCommand::SetLayout(Some(next)));
                ui.close();
            }
            if can_right && ui.button("Move to Right Column").clicked() {
                let next = apply_area_drop(&menu.base, ap, StackZone::Column { last: true });
                *cmd = Some(AreaRightCommand::SetLayout(Some(next)));
                ui.close();
            }
        }
    }
    mg_uikit::menu_separator(ui);
    if menu.props_shown {
        if ui.button("Hide Properties").clicked() {
            *cmd = Some(AreaRightCommand::HideProperties);
            ui.close();
        }
    } else if menu.can_show_props && ui.button("Show Properties").clicked() {
        *cmd = Some(AreaRightCommand::ShowProperties(area));
        ui.close();
    }
    if ui.button("Reset Right Dock Layout").clicked() {
        *cmd = Some(AreaRightCommand::ResetLayout);
        ui.close();
    }
}

fn attach_area_interactions(
    area: Option<mg_core::ResRef>,
    menu: Option<&AreaRightMenuState>,
    panels: &[Panel],
    front: Panel,
    bar_resp: &egui::Response,
    tab_resps: &[egui::Response],
    (dragged, menu_cmd): (&mut Option<Panel>, &mut Option<AreaRightCommand>),
) {
    let (Some(area), Some(menu)) = (area, menu) else { return };
    if tab_resps.is_empty() {
        if bar_resp.drag_started() {
            *dragged = to_area_panel(front).map(from_area_panel);
        }
        mg_uikit::context_menu(bar_resp, |ui| {
            show_area_context_menu(ui, area, menu, front, menu_cmd);
        });
        return;
    }
    mg_uikit::context_menu(bar_resp, |ui| {
        show_area_context_menu(ui, area, menu, front, menu_cmd);
    });
    for (p, r) in panels.iter().zip(tab_resps) {
        if r.drag_started() {
            *dragged = to_area_panel(*p).map(from_area_panel);
        }
        mg_uikit::context_menu(r, |ui| {
            show_area_context_menu(ui, area, menu, *p, menu_cmd);
        });
    }
}

fn apply_area_right_command(app: &mut Moonglow, cmd: AreaRightCommand) {
    match cmd {
        AreaRightCommand::SetLayout(layout) => {
            app.settings.area_right_layout = layout;
        }
        AreaRightCommand::HideProperties => {
            app.settings.hide_properties = true;
        }
        AreaRightCommand::ShowProperties(area) => {
            app.settings.hide_properties = false;
            app.props_popped_out.remove(&area);
            app.show_dock_panel(Panel::Properties);
        }
        AreaRightCommand::ResetLayout => {
            app.settings.area_right_layout = None;
            app.settings.hide_properties = false;
            if let Some(layout) = app.settings.dock_layouts.get_mut("area") {
                layout.shares.clear();
                layout.column = None;
            }
        }
    }
}

/// The panel in front of a section (its tab selected): the Right Dock's
/// panel chosen last where the section holds it, else the one chosen
/// last of its panels in the run, else the one the settings keep for the
/// editor's kind, else its first.
pub(crate) fn section_front(app: &Moonglow, section: &[Panel]) -> Panel {
    if let Some(chosen) = app.right_panel.filter(|p| section.contains(p)) {
        return chosen;
    }
    run_or_kept_front(app, editor_kind(app), &editor_brings(app), section)
}

/// The place among the kind's own sections (`brings.right`) of the one
/// holding `section`'s panels: what the settings keep its front and share
/// by. `None` for a section the kind does not bring (the palettes asked
/// for by name beside it), of which nothing is kept for the kind.
fn own_index(brings: &Brings, section: &[Panel]) -> Option<usize> {
    brings.right.iter().position(|s| section.iter().any(|p| s.contains(p)))
}

/// A section's front by the run's choices (`Moonglow::right_fronts`),
/// else as the settings keep it for `kind` (where the kind keeps them, of
/// its own sections), else its first.
fn run_or_kept_front(
    app: &Moonglow,
    kind: Option<&str>,
    brings: &Brings,
    section: &[Panel],
) -> Panel {
    let run = app.right_fronts.iter().rev().find(|p| section.contains(p)).copied();
    let kept = || {
        let layout = app.settings.dock_layouts.get(kind?)?;
        let name = layout.fronts.get(own_index(brings, section)?)?;
        Panel::named(name).filter(|p| section.contains(p))
    };
    run.or_else(kept).unwrap_or(section[0])
}

/// `panel` chosen in front of its section of `kind`'s Right Dock (or its
/// window's right pane): first among the run's choices, and kept in the
/// settings where the section is one the kind brings (`brings`): never
/// the palettes' asked for by name beside another kind.
fn keep_front(app: &mut Moonglow, kind: Option<&'static str>, brings: &Brings, panel: Panel) {
    app.right_fronts.retain(|p| *p != panel);
    app.right_fronts.push(panel);
    let Some(kind) = kind else { return };
    let Some(index) = own_index(brings, &[panel]) else { return };
    let narrow = app.yields.right_dock_narrow;
    let layout = app.settings.dock_layouts.entry(kind.to_string()).or_default();
    // (Chosen in the one tab bar of a narrow window: that bar's front too.)
    if narrow {
        layout.folded = Some(panel.name().to_string());
    }
    mg_uikit::stack::keep_front(&mut layout.fronts, index, panel.name());
}

/// A tab of the Right Dock chosen (its tab clicked, its strip button,
/// asked for by name): the dock's panel from then on, its section's for
/// the run, and kept in the settings as the section's front for the
/// editor's kind (the area's too: the owner's ruling of 2026-11-15).
pub(crate) fn choose_right(app: &mut Moonglow, panel: Panel) {
    app.right_panel = Some(panel);
    keep_front(app, editor_kind(app), &editor_brings(app), panel);
}

/// The Right Dock's panel moved to without choosing a tab (a press in a
/// section that is not on its tab, F6 to a section): the dock's panel the
/// strip and the keys come back to, nothing written (the section's front
/// is already that panel).
pub(crate) fn focus_right(app: &mut Moonglow, panel: Panel) {
    app.right_panel = Some(panel);
}

/// The panels an editor brings into its own window where it floats
/// ([`Brings::floats`]: Floating.png's script, its Reference over its
/// Help at its right, the whole height, and its Compiler, Bookmarks and
/// Search Results under the editor): its sections stacked in a right pane
/// by the kind's shares, with seams, and its own Bottom Dock panels and
/// the global ones that follow it (`Brings::window_bottom`) on the tabs
/// of a bottom pane under the editor, carved from `ui` (the window's tab)
/// before the editor draws in what is left; each body the one the dock
/// draws ([`panel_body`], its host [`Host::Window`], its editor `tab`). A
/// press in a pane makes the editor's pane the plugins' pane last clicked
/// in ([`plugin_pane`]); each pane's panel in front is a stop of F6
/// (`Moonglow::window_bars`). An editor whose panels stay in the main
/// window's docks (an area's) brings none here.
pub(crate) fn window_panes(app: &mut Moonglow, ui: &mut Ui, tab: &crate::Tab) {
    // (A NUI designer's source group, a part of the designer: its panes are
    // the designer's, drawn beside the design. M4's U19.)
    if matches!(tab, crate::Tab::NuiSource(_)) {
        return;
    }
    let brings = brings_of(app, tab);
    if !brings.floats && brings.window_right.is_empty() {
        return;
    }
    let kind = Some(layout_kind(app, tab));
    let held = |s: Section| -> Vec<Panel> {
        s.iter().copied().filter(|p| p.available_with(app, &brings)).collect()
    };
    // (A kind that does not float brings only its `window_right` here:
    // the rest stay the main window's.)
    let sections = if brings.floats { brings.right } else { brings.window_right };
    let right: Vec<Vec<Panel>> =
        sections.iter().map(|s| held(s)).filter(|s| !s.is_empty()).collect();
    let mut bottom = if brings.floats { held(brings.bottom) } else { Vec::new() };
    for p in held(brings.window_bottom).into_iter().filter(|_| brings.floats) {
        if !bottom.contains(&p) {
            bottom.push(p);
        }
    }
    let t = mg_uikit::tokens(ui.ctx());
    let pane = ui.unique_id();
    let pressed = |ui: &Ui, rect: egui::Rect| {
        ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(rect)
    };
    // (The right pane first, the whole height; the bottom pane under the
    // editor alone, as Floating.png draws them. Not in a window too narrow
    // for it and the editor's least beside it: the editor keeps its room,
    // and its panels come back as the window is widened.)
    let room = ui.available_width();
    // (A script's pane never goes: it narrows to half the window, as the
    // script's side lists did, so that its Reference and Help stay in
    // reach in a small window. M4's U12.)
    let halves = pane_halves(tab);
    let fits = halves || room - RIGHT_DOCK_LEAST - 1.0 >= window_editor_least(tab) - 0.5;
    let owner = owner_of(app, tab).into_owned();
    match fits || right.is_empty() {
        true => app.panes_dropped.remove(&owner),
        false => app.panes_dropped.insert(owner.clone()),
    };
    if !right.is_empty() && fits {
        let (least, most) = match halves && room < 2.0 * RIGHT_DOCK_LEAST {
            true => {
                let half = (room / 2.0).floor();
                (half, half)
            }
            false => (RIGHT_DOCK_LEAST, (room * 0.6).max(RIGHT_DOCK_LEAST)),
        };
        // (As wide as the kind's last was dragged to, in a window of its
        // own: 5.2.0, Jasperre's report.)
        let kept = window_kept(app, kind, false);
        seed_window_pane(ui, pane.with("window-right"), kept);
        let shown = egui::Panel::right(pane.with("window-right"))
            .resizable(true)
            .default_size(kept.unwrap_or(RIGHT_DOCK_LEAST))
            .size_range(least.min(most)..=most)
            .frame(egui::Frame::new().fill(t.panel))
            .show(ui, |ui| {
                let whole = ui.max_rect();
                // (Its window's own shares where it has them.)
                let shares = match brings.window_shares {
                    [] => brings.shares,
                    own => own,
                };
                let window = Brings { shares, ..brings };
                let stack = Stack { kind, brings: &window };
                let places = section_rects(app, whole, &right, stack);
                let mut chosen = None;
                for (i, (place, section)) in places.iter().copied().zip(&right).enumerate() {
                    let name = match i {
                        0 => WINDOW_RIGHT.to_string(),
                        _ => format!("{WINDOW_RIGHT}, Section {}", i + 1),
                    };
                    // (Brought to the front in this window, else the run's
                    // or the kind's.)
                    let asked = app.window_rights.get(&owner).copied();
                    let front = match asked.filter(|p| section.contains(p)) {
                        Some(asked) => asked,
                        None => run_or_kept_front(app, kind, &brings, section),
                    };
                    let mut part =
                        ui.new_child(egui::UiBuilder::new().id_salt(&name).max_rect(place));
                    part.set_clip_rect(place.intersect(ui.clip_rect()));
                    if pressed(&part, place) {
                        app.plugins.pane = plugin_pane(app, front, Some(tab)).or(app.plugins.pane);
                    }
                    let on = (section.as_slice(), front, Some(tab));
                    let bar = section_bar(app, &mut part, &name, on, None, None, None, |_| ());
                    if let Some(at) = bar.chosen {
                        chosen = Some(section[at]);
                    }
                    if bar.opened {
                        opened_in_tab(app, Some(tab));
                    }
                    if bar.hidden {
                        app.settings.hide_properties = true;
                    }
                    if let Some(tool) = bar.node {
                        crate::dialog_view::node_tool(app, Some(tab), tool);
                    }
                    app.window_bars_drawn.push(front);
                    if front == Panel::Preview {
                        app.window_previews_drawn.push(tab.clone());
                    }
                    let on = (front, section.len() == 1);
                    panel_body(app, &mut part, on, Host::Window, pane, Some(tab));
                }
                let salt = format!("window-right-{}", pane.value());
                section_seams(app, ui, &places, &right, stack, &salt);
                #[cfg(test)]
                app.sample_panes.push((WINDOW_RIGHT, whole));
                ui.advance_cursor_after_rect(whole);
                chosen
            });
        // (Its edge, for the seam over the editor drawn after it, where it
        // may be dragged: not a script's held to half the window.)
        if least < most {
            let edge = RightEdge { dock: shown.response.rect, least, most };
            ui.ctx().data_mut(|d| d.insert_temp(window_edge_id(pane), edge));
            let edges =
                [pane.with("window-right").with("__resize"), window_edge_id(pane).with("seam")];
            let now = shown.response.rect.width();
            keep_window_size(app, ui, (kind, false), &edges, kept, now);
        }
        if let Some(panel) = shown.inner {
            // (A tab chosen in the window: the window's own front no more.)
            app.window_rights.remove(&owner);
            keep_front(app, kind, &brings, panel);
        }
    }
    // (Its Outliner, where it brings one and floats: a floating
    // conversation's bookmarks, M4's U15. At its left, the whole height,
    // after the right pane, so that what the Node section's bar did this
    // frame is listed; drawn only where the tree keeps the room it had
    // before (`WINDOW_TREE_AT_FIRST`: the window opens that much wider
    // where the screen allows, `Moonglow::open_window`), as wide as the
    // Left Dock or narrower, down to `WINDOW_LEFT_LEAST`. Elsewhere the
    // pane gives way before the header does (the coordinator's ruling at
    // U15's review): folded to a strip at the left of the tree, under the
    // header, its Outliner's icon unfolding it over the tree
    // (`outliner::strip`), so the bookmarks stay in reach. A floating
    // table's columns the same, M4's U16, its grid keeping the room the
    // window gave it before its panes came into it, `WINDOW_GRID_AT_FIRST`.)
    let room = ui.available_width();
    let keeps = match tab {
        crate::Tab::Table(_) => WINDOW_GRID_AT_FIRST,
        // (A NUI designer's canvas and header, M4's U18.)
        crate::Tab::Nui(_) => WINDOW_CANVAS_AT_FIRST,
        // (A tileset's list and form side by side and its header, M4's U20.)
        crate::Tab::Tileset(_) => WINDOW_TILES_AT_FIRST,
        // (A hak's table and header, M4's U21: the room its window had.)
        crate::Tab::Hak(_) => WINDOW_TILES_AT_FIRST,
        // (The faction editor's matrix and header, M4's U22.)
        crate::Tab::Factions => WINDOW_FACTIONS_AT_FIRST,
        _ => WINDOW_TREE_AT_FIRST,
    };
    let most = (room - keeps - 1.0).min(room * 0.4).floor();
    if let Some(key) = crate::outliner::Owner::of(tab)
        && brings.outliner
        && brings.floats
    {
        if most >= WINDOW_LEFT_LEAST {
            egui::Panel::left(pane.with("window-left"))
                .resizable(true)
                .default_size(WINDOW_LEFT_WIDE.min(most))
                .size_range(WINDOW_LEFT_LEAST..=most)
                .frame(egui::Frame::new().fill(t.panel))
                .show(ui, |ui| {
                    if pressed(ui, ui.max_rect()) {
                        app.plugins.pane =
                            plugin_pane(app, Panel::OUTLINER, Some(tab)).or(app.plugins.pane);
                    }
                    app.window_bars_drawn.push(Panel::OUTLINER);
                    crate::outliner::window_ui(app, ui, key);
                });
        } else {
            app.outliner.strips_now.insert(key);
        }
    }
    if !bottom.is_empty() {
        // (Brought to the front in the run, else as kept for the kind,
        // else its first.)
        let owner = owner_of(app, tab).into_owned();
        let run = app.window_bottoms.get(&owner).copied().filter(|p| bottom.contains(p));
        let kept = (kind.and_then(|k| app.settings.dock_layouts.get(k)))
            .and_then(|l| l.bottom.as_deref())
            .and_then(Panel::named)
            .filter(|p| bottom.contains(p));
        let front = run.or(kept).unwrap_or(bottom[0]);
        // (A script's live error, for the bar's right end.)
        let note = match tab {
            crate::Tab::Script(key) => script_note(app, *key),
            _ => None,
        };
        // (At most a third of the window's room under its tab bar, the
        // editor keeping two thirds, and at least its tab bar and a line:
        // in a small window the pane no longer crowds the code, M4's U13.)
        let (least, most) = window_bottom_range(ui, ui.available_height());
        // (As high as the kind's last was dragged to: 5.2.0.)
        let kept = window_kept(app, kind, true);
        // (Cut at the room the right pane leaves: its edge to drag is not
        // over the right pane, nor is what it holds.)
        let room = ui.available_rect_before_wrap();
        ui.set_clip_rect(ui.clip_rect().intersect(room));
        let bottom_id = egui::Id::new(("window-bottom", &owner));
        seed_window_pane(ui, bottom_id, kept);
        let shown = egui::Panel::bottom(bottom_id)
            .resizable(true)
            .default_size(kept.unwrap_or(WINDOW_BOTTOM_HIGH).clamp(least, most))
            .size_range(least..=most)
            .frame(egui::Frame::new().fill(t.panel))
            .show(ui, |ui| {
                if pressed(ui, ui.max_rect()) {
                    app.plugins.pane = plugin_pane(app, front, Some(tab)).or(app.plugins.pane);
                }
                let mut at = bottom.iter().position(|p| *p == front).unwrap_or(0);
                let mut tabs = DockTabs::new(WINDOW_BOTTOM);
                for p in &bottom {
                    tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
                }
                #[cfg(test)]
                app.sample_panes.push((WINDOW_BOTTOM, ui.max_rect()));
                // (A floating script's live error at the bar's right end, in
                // `error`, as the docked one's is in the Status Bar: the
                // window has none of its own. Cut short to the room the
                // tabs leave, its error icon alone where that is little.)
                let words = tabs.tabs_width(ui, false);
                let room = (ui.available_width() - words - 16.0).min(WINDOW_NOTE_MOST);
                if note.is_some() && room >= mg_uikit::metrics::ICON + 8.0 {
                    tabs = tabs.tools_width(room);
                }
                let error = mg_uikit::tokens(ui.ctx()).error;
                let bar = tabs.show(ui, &mut at, |ui| {
                    let note = note.as_ref().filter(|_| room >= mg_uikit::metrics::ICON + 8.0)?;
                    let said = format!("Found as you type: {}", note.text);
                    let r = if room >= 80.0 {
                        let text = egui::RichText::new(&note.text).color(error).small();
                        let label = egui::Label::new(text).truncate().sense(egui::Sense::click());
                        ui.set_max_width(room - 4.0);
                        let r = ui.add(label);
                        r.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &said)
                        });
                        r
                    } else {
                        let icon = IconButton::new(Icon::Error, &said).icon_only(true);
                        ui.add(icon)
                    };
                    r.on_hover_text(&note.tip).clicked().then_some(note.at).flatten()
                });
                app.window_bars_drawn.push(front);
                match front {
                    Panel::Log => {
                        let margin = Margin { left: 8, right: 8, top: 4, bottom: 4 };
                        egui::Frame::new().inner_margin(margin).show(ui, |ui| app.log_ui(ui));
                    }
                    _ => panel_body(app, ui, (front, false), Host::Window, pane, Some(tab)),
                }
                (bar.changed.then(|| bottom[at]), bar.inner)
            });
        if least < most {
            let edges = [bottom_id.with("__resize")];
            let now = shown.response.rect.height();
            keep_window_size(app, ui, (kind, true), &edges, kept, now);
        }
        let (picked, to_line) = shown.inner;
        if let Some((key, line)) = to_line {
            let text = app.script_source(&key).unwrap_or_default();
            app.script_tools.jump = Some((key, crate::script_tools::line_start(&text, line)));
            app.script_tools.keys_back = Some(key);
        }
        if let Some(panel) = picked {
            app.window_bottoms.insert(owner, panel);
            if let Some(kind) = kind {
                app.settings.dock_layouts.entry(kind.to_string()).or_default().bottom =
                    Some(panel.name().to_string());
            }
        }
    }
}

/// The width of a kind's window's right pane (`bottom`: the height of its
/// bottom pane) as the builder last dragged it, in any window of the kind
/// (`DockLayout::window_width`, `window_bottom`): a new window opens with it.
fn window_kept(app: &Moonglow, kind: Option<&str>, bottom: bool) -> Option<f32> {
    let kind = kind?;
    let layout = app.settings.dock_layouts.get(kind)?;
    let kept = if bottom { layout.window_bottom } else { layout.window_width };
    kept.map(f32::from)
}

/// Lets the kind's kept size ([`window_kept`]) apply to a window's pane
/// `panel` that was not drawn in the pass before (its window opened, its
/// tab brought to the front, the first pass of a run): egui keeps a
/// panel's size by its id, which goes by the editor's tab, so a script
/// opened again would take the size its pane had before, from this run or
/// (egui's memory kept) the last, not the kept one. Drawn pass after pass,
/// the pane keeps its own size.
fn seed_window_pane(ui: &Ui, panel: egui::Id, kept: Option<f32>) {
    let slot = panel.with("window-drawn");
    let pass = ui.ctx().cumulative_pass_nr();
    let last = ui.ctx().data(|d| d.get_temp::<u64>(slot));
    if last == Some(pass) {
        return;
    }
    let anew = last != Some(pass.wrapping_sub(1));
    ui.ctx().data_mut(|d| {
        if anew && kept.is_some() {
            d.remove::<egui::containers::panel::PanelState>(panel);
        }
        d.insert_temp(slot, pass);
    });
}

/// Keeps a window's pane's size `now` for its kind ([`window_kept`],
/// the bottom pane's where `bottom`) while the builder drags one of its
/// `edges` (their ids) to another, and the passes after its release (the
/// pane follows the pointer a pass late: a drag let go as it moves would
/// keep the size from before): not where the window's own size moved it
/// (the pane held to its range), which is no choice of the builder's.
fn keep_window_size(
    app: &mut Moonglow,
    ui: &Ui,
    (kind, bottom): (Option<&str>, bool),
    edges: &[egui::Id],
    kept: Option<f32>,
    now: f32,
) {
    let Some(kind) = kind else { return };
    let pass = ui.ctx().cumulative_pass_nr();
    // (The window's own, by its edges: not the kind's, or another window
    // of the kind drawn after it would keep its own untouched size.)
    let slot = egui::Id::new(("window-size-dragged", edges));
    if edges.iter().any(|id| ui.ctx().is_being_dragged(*id)) {
        ui.ctx().data_mut(|d| d.insert_temp(slot, pass));
    }
    let last = ui.ctx().data(|d| d.get_temp::<u64>(slot));
    let dragged = last.is_some_and(|last| pass.wrapping_sub(last) <= 3);
    let now = now.round();
    if !dragged || kept.is_some_and(|k| (k - now).abs() < 0.5) {
        return;
    }
    let layout = app.settings.dock_layouts.entry(kind.to_string()).or_default();
    let size = Some(now.clamp(0.0, f32::from(u16::MAX)) as u16);
    match bottom {
        true => layout.window_bottom = size,
        false => layout.window_width = size,
    }
}

/// The least and the most height of a floating editor's window's bottom
/// pane in a window `room` high under its tab bar ([`window_panes`]): its
/// tab bar and a line of its panel at least; at most a third of the room
/// ([`WINDOW_BOTTOM_SHARE`]), the editor keeping the rest, or the least
/// where that is more (a window too short for both: the code keeps what
/// the pane's tab bar and a line leave).
pub(crate) fn window_bottom_range(ui: &Ui, room: f32) -> (f32, f32) {
    let line = ui.text_style_height(&egui::TextStyle::Monospace);
    let least = metrics::DOCK_TABS + line + 12.0;
    (least, (room * WINDOW_BOTTOM_SHARE).floor().max(least))
}

/// The most a floating script's window's bottom pane's bar gives its live
/// error's message, at its right end.
const WINDOW_NOTE_MOST: f32 = 360.0;

/// The most of a floating editor's window's height under its tab bar its
/// bottom pane takes (M4's U13: the known difference "In small script
/// windows the Compiler and Help strip crowds the code" settled; Help went
/// to the right pane in U12).
pub(crate) const WINDOW_BOTTOM_SHARE: f32 = 1.0 / 3.0;
/// A floating editor's window's bottom pane's height at first, as the
/// script's own pane under its text was.
pub(crate) const WINDOW_BOTTOM_HIGH: f32 = 140.0;

/// Whether `tab`'s window's right pane stays at every width, narrowed to
/// half the window under twice its least (a script's Reference and Help:
/// its side lists did so), rather than going where the editor's least
/// beside it does not fit.
pub(crate) fn pane_halves(tab: &crate::Tab) -> bool {
    // (And a conversation's Node and Conversation panels, M4's U14: every
    // field of its lines is there, as its bottom pane had them.)
    // (And a table's Cell and References, M4's U16: its cell's choosers
    // were under its grid.)
    // (And a talk table's String, M4's U17: its line's fields were under
    // its list.)
    matches!(
        tab,
        crate::Tab::Script(_)
            | crate::Tab::Dialog(_)
            | crate::Tab::Table(_)
            | crate::Tab::TalkTable
            // (And a NUI designer's Components, Properties and Bindings,
            // M4's U18: a control's properties are nowhere else.)
            | crate::Tab::Nui(_)
    )
}

/// The least room an editor keeps in a window of its own beside its
/// window's right pane ([`window_panes`]): under it the pane is not drawn.
/// An area's view of 400 points (its header compact there, its tools in
/// More) and its tool shelf, 36, with their lines.
pub(crate) const WINDOW_EDITOR_LEAST: f32 = 440.0;

/// The least room a Properties form keeps beside its window's Preview
/// pane (M4's U11, the coordinator's ruling R1): what U9's first width of
/// 980 gives it, its page list and pages as U9's review restored them.
/// Under it the pane is not drawn, and the Preview button opens the
/// model's own tab.
pub(crate) const WINDOW_FORM_LEAST: f32 = 972.0;

/// The least room `tab`'s editor keeps beside its window's right pane: a
/// Properties form's [`WINDOW_FORM_LEAST`], every other editor's
/// [`WINDOW_EDITOR_LEAST`].
pub(crate) fn window_editor_least(tab: &crate::Tab) -> f32 {
    match crate::model_view::Source::of_editor(tab) {
        Some(_) => WINDOW_FORM_LEAST,
        None => WINDOW_EDITOR_LEAST,
    }
}

/// The name of an editor's window's right pane's tab bar (its first
/// section's; [`window_panes`]).
pub(crate) const WINDOW_RIGHT: &str = "Window's Right Pane";
/// A floating conversation's window's Outliner's width at first, where the
/// window has the room (the Conversation board's Left Dock's 260).
pub(crate) const WINDOW_LEFT_WIDE: f32 = 260.0;
/// The room a floating conversation's tree keeps beside its window's
/// Outliner at first: what its window's first size gave it beside the
/// right pane before the Outliner came into it (920, less the pane's 280
/// and the lines), so that its header yields no sooner.
pub(crate) const WINDOW_TREE_AT_FIRST: f32 = 638.0;
/// The least of it: a bookmark's kind, number and a few words.
pub(crate) const WINDOW_LEFT_LEAST: f32 = 150.0;
/// The room a floating table's grid keeps beside its window's Outliner
/// (M4's U16): what its window gave the editor before its panes came into
/// it (1040, its lines less), so that its header yields no sooner. The
/// window opens wider by its right pane and the Outliner's where the
/// screen allows (`Moonglow::open_window`).
pub(crate) const WINDOW_GRID_AT_FIRST: f32 = 1038.0;
/// The room a floating tileset's page keeps beside its window's Outliner
/// (M4's U20): what its window gave the editor before its panes came into
/// it (920, its lines less), so that its list and form stand side by side
/// and its header yields no sooner.
pub(crate) const WINDOW_TILES_AT_FIRST: f32 = 918.0;
/// The room a floating NUI designer's canvas keeps beside its window's
/// Outliner (M4's U18): what its window gives the editor before its panes
/// come into it (760, its lines less), so that its header yields no sooner.
pub(crate) const WINDOW_CANVAS_AT_FIRST: f32 = 758.0;
/// The room a floating faction editor's matrix keeps beside its window's
/// Outliner (M4's U22): what its window gives the editor before its panes
/// come into it (780, its lines less), so that its header yields no sooner.
pub(crate) const WINDOW_FACTIONS_AT_FIRST: f32 = 778.0;
/// The name of an editor's window's bottom pane's tab bar.
pub(crate) const WINDOW_BOTTOM: &str = "Window's Bottom Pane";

/// The panel the Right Dock shows (its tab in front), of those it holds:
/// the one chosen last (`Moonglow::right_panel`), else in a window under
/// 1400 the one bar's front as chosen there for the kind
/// (`DockLayout::folded`), else its first section's front. `None` where it
/// holds none (and is not drawn).
pub(crate) fn right_shown(app: &Moonglow) -> Option<Panel> {
    let sections = right_sections(app);
    let held = |p: &Panel| sections.iter().any(|s| s.contains(p));
    if let Some(chosen) = app.right_panel.filter(held) {
        return Some(chosen);
    }
    // (Under 1400, one tab bar: its front as chosen there for the kind.)
    if app.yields.right_dock_narrow
        && let Some(kept) = (editor_kind(app).and_then(|k| app.settings.dock_layouts.get(k)))
            .and_then(|l| l.folded.as_deref())
            .and_then(Panel::named)
            .filter(held)
    {
        return Some(kept);
    }
    sections.first().map(|s| section_front(app, s))
}

/// The panel the Left Dock shows (its tab selected), of those it holds:
/// the one chosen last, else its first. `None` where it holds none.
pub(crate) fn left_shown(app: &Moonglow) -> Option<Panel> {
    let held = panels(app, Dock::Left);
    held.contains(&app.left_panel).then_some(app.left_panel).or(held.first().copied())
}

/// Whether the Left Dock holds the Outliner alone: no tab panel (no module
/// open, the Resources panel not asked for), the editor in front listing
/// in the Outliner (a table's columns). The dock is drawn with the
/// Outliner the whole height, its header ending with the fold button, and
/// folds to a strip of its icon (the coordinator's ruling at U16's
/// review; before it, the dock was drawn only once a tab panel was).
pub(crate) fn outliner_alone(app: &Moonglow) -> bool {
    left_shown(app).is_none()
        && Panel::OUTLINER.available(app)
        && crate::outliner::contents(app).is_some()
}

/// Whether the Left Dock is drawn at all (unfolded or as its strip): it
/// holds a tab panel, or the Outliner alone ([`outliner_alone`]).
pub(crate) fn left_holds(app: &Moonglow) -> bool {
    left_shown(app).is_some() || outliner_alone(app)
}

/// The panel the Bottom Dock shows (its tab in front), of those it holds:
/// the one chosen last (`Moonglow::bottom_panel`), else the one the
/// settings keep for the editor's kind, else the editor's own first (a
/// script's Compiler, the Script board's), else the Log (not Search
/// Results, the first global tab: a dock opened beside an editor of no
/// panels of its own shows the log, as before it). `None` where it holds
/// none.
pub(crate) fn bottom_shown(app: &Moonglow) -> Option<Panel> {
    let held = panels(app, Dock::Bottom);
    if let Some(chosen) = app.bottom_panel.filter(|p| held.contains(p)) {
        return Some(chosen);
    }
    let kept = editor_kind(app)
        .and_then(|k| app.settings.dock_layouts.get(k))
        .and_then(|l| l.bottom.as_deref())
        .and_then(Panel::named)
        .filter(|p| held.contains(p));
    let own = || editor_brings(app).bottom.iter().copied().find(|p| held.contains(p));
    let log = || held.contains(&Panel::Log).then_some(Panel::Log);
    kept.or_else(own).or_else(log).or(held.first().copied())
}

/// A tab of the Bottom Dock chosen (its tab, its switch): the dock's panel
/// from then on, and kept for the editor's kind where it is one of the
/// kind's own.
pub(crate) fn choose_bottom(app: &mut Moonglow, panel: Panel) {
    app.bottom_panel = Some(panel);
    if editor_brings(app).bottom.contains(&panel)
        && let Some(kind) = editor_kind(app)
    {
        let layout = app.settings.dock_layouts.entry(kind.to_string()).or_default();
        layout.bottom = Some(panel.name().to_string());
    }
}

/// The Bottom Dock's panel brought to the front without choosing its tab
/// (a script's compile, bookmark or search: `ScriptTools::ask_forward`):
/// the dock's panel from then on, nothing kept in the settings.
pub(crate) fn focus_bottom(app: &mut Moonglow, panel: Panel) {
    app.bottom_panel = Some(panel);
}

/// Whether the Bottom Dock shows `panel` now: open, with it in front.
pub(crate) fn bottom_shows(app: &Moonglow, panel: Panel) -> bool {
    !app.settings.hide_log && bottom_shown(app) == Some(panel)
}

// ------------------------------------------------------------------ yields

/// The width under which both side docks, open, do not fit beside the
/// editor area at its least: the Left Dock's least, the Right Dock's narrow
/// width, each dock's line and the editor's least (962). Only a dock
/// unfolded by hand in a narrow window is open there; where both are, the
/// one unfolded first folds to its strip ([`left_folded`]).
pub(crate) const BOTH_DOCKS_UNDER: f32 =
    LEFT_DOCK_LEAST + 1.0 + kit::RIGHT_DOCK_NARROW + 1.0 + kit::EDITOR_LEAST;

/// What the shell yields at this frame, decided once as the frame begins
/// from the window's width (`Yields::of`; the editor's width as it was
/// last drawn, for M4's header), and read by the Top Bar, Quick Open, the
/// docks and the Status Bar ([`Moonglow::yields`]). A dock unfolded by
/// hand in a window that folds it ([`set_dock_open`]) stays so until the
/// window is wide enough not to fold it: from there the builder's own
/// fold (`Settings::hide_tree`, `hide_palettes`) has it again, and a
/// window narrowed again folds it again.
pub(crate) fn frame_yields(app: &mut Moonglow, ctx: &egui::Context) {
    let window = ctx.content_rect().width();
    app.yields = Yields::of(window, app.dock_width.unwrap_or(window));
    // (The windows' panes drawn at the last frame, for F6; and the lists
    // of bookmarks, docked and in windows.)
    app.window_bars = std::mem::take(&mut app.window_bars_drawn);
    app.outliner.marks_last = std::mem::take(&mut app.outliner.marks_now);
    app.outliner.strips_now.clear();
    if !app.yields.left_dock_folded {
        app.left_unfolded_here = false;
    }
    if !app.yields.right_dock_folded {
        app.right_unfolded_here = false;
    }
    hold_wide_properties(app, ctx);
}

/// Whether the Left Dock is folded to its strip at this frame: in a
/// window under 1500 (`Yields::left_dock_folded`) unless unfolded there by
/// hand, else as the builder left it (`Settings::hide_tree`, never written
/// by a width); and where both side docks are open in a window too narrow
/// for both beside the editor area's least ([`BOTH_DOCKS_UNDER`]), the one
/// unfolded first folds (the one the builder unfolded last stays).
pub(crate) fn left_folded(app: &Moonglow) -> bool {
    let (left, right) = folds_by_choice(app);
    left || (!right && squeezed(app) == Some(Side::Left))
}

/// Whether the Right Dock is folded to its strip at this frame: in a
/// window under [`kit::RIGHT_DOCK_STRIP_UNDER`] unless unfolded there by
/// hand, else as the builder left it (`Settings::hide_palettes`); and
/// beside a Left Dock unfolded by hand later where both do not fit
/// ([`left_folded`]).
pub(crate) fn right_folded(app: &Moonglow) -> bool {
    let (left, right) = folds_by_choice(app);
    right || (!left && squeezed(app) == Some(Side::Right))
}

/// The arrow keys never stay with a panel that is not drawn: where the
/// dock panel holding them is not drawn this frame (its dock folded by
/// the window's width or by the builder, the Bottom Dock closed, its dock
/// holding no panel, as beside a script's tab, or the Outliner no longer
/// brought by the editor in front), they go to the editor area's active
/// editor ([`Moonglow::editor_keys`]), and stay there when it is drawn
/// again (the orchestrator's decision of T14's review, reversible). Where
/// its dock is drawn on another of its tabs, they go with the tab, to the
/// panel in front: the Left Dock's tab bar hands them over as it draws
/// ([`left_dock`]), the Right Dock's and the Bottom Dock's here. Run once
/// a frame before the docks. (Whether the Right Dock stands in more than
/// one tab bar, each drawing its panel in front, is the last frame's
/// (`Moonglow::right_bars`): there the keys' bar is the dock's panel,
/// [`Moonglow::right_panel`], so the frame it changes moves no keys.)
pub(crate) fn keys_leave_panels_not_drawn(app: &mut Moonglow, ctx: &egui::Context) {
    use crate::palette_view::{give_keys, keys_are};
    // (A conversation's bookmarks: drawn in the Outliner while the Left
    // Dock is unfolded with that conversation in front, or in its window,
    // as at the last frame.)
    if keys_are(ctx, crate::palette_view::Keys::Marks) {
        let key = app.outliner.marks_keys;
        let docked = left_holds(app)
            && !left_folded(app)
            && key.is_some_and(|k| crate::outliner::contents(app) == Some(k.contents()));
        let window = app.outliner.marks_last.iter().any(|(k, w)| *w && Some(*k) == key);
        if !docked && !window {
            app.outliner.marks_keys = None;
            let (keys, table) = app.editor_keys();
            give_keys(ctx, keys);
            if let Some(id) = table {
                crate::table_view::take_keys(ctx, id);
            }
        }
        return;
    }
    let Some(owner) = Panel::KEYED.into_iter().find(|p| keys_are(ctx, p.keys())) else { return };
    let held = owner.keys();
    let (front, drawn) = match owner.dock() {
        // (Drawn on its other tab, the Left Dock's tab bar hands them to
        // that tab's panel, the tree's to the browser's list and back.)
        Dock::Left => {
            let left = left_shown(app).filter(|_| !left_folded(app));
            // (The Outliner's rows of an area: not where it lists a
            // conversation's bookmarks, whose keys are their own.)
            let outliner = owner.available(app)
                && crate::outliner::keys_of(app) == crate::palette_view::Keys::Outliner;
            let drawn = left.is_some() && (owner != Panel::OUTLINER || outliner);
            (None, drawn)
        }
        Dock::Right => {
            let right = right_shown(app).filter(|_| !right_folded(app));
            let apart = app.right_columns && app.right_bars.iter().any(|q| q.keys() == held);
            // (Or in a floating editor's window: a dragged out area's Tile
            // Properties, the last frame's.)
            let in_window = app.window_bars.iter().any(|q| q.keys() == held);
            (right, right.is_some_and(|p| p.keys() == held) || apart || in_window)
        }
        Dock::Bottom => {
            let bottom = bottom_shown(app).filter(|_| !app.settings.hide_log);
            (bottom, bottom.is_some_and(|p| p.keys() == held))
        }
    };
    if drawn {
        return;
    }
    // (Its dock drawn on another tab: to that tab's panel.)
    if let Some(front) = front {
        give_keys(ctx, front.keys());
        app.palette.to_first |= front == Panel::Palette;
        return;
    }
    let (keys, table) = app.editor_keys();
    give_keys(ctx, keys);
    if let Some(id) = table {
        crate::table_view::take_keys(ctx, id);
    }
}

/// Each side dock's fold by the window's width and the builder's hand,
/// before the two are weighed against each other ([`squeezed`]).
fn folds_by_choice(app: &Moonglow) -> (bool, bool) {
    let y = &app.yields;
    let left = match y.left_dock_folded {
        true => !app.left_unfolded_here,
        false => app.settings.hide_tree,
    };
    let right = match y.right_dock_folded {
        true => !app.right_unfolded_here,
        false => app.settings.hide_palettes,
    };
    (left, right)
}

/// Which side dock folds where both are open (and each holds panels) in a
/// window narrower than [`BOTH_DOCKS_UNDER`]: the one not unfolded last.
fn squeezed(app: &Moonglow) -> Option<Side> {
    let window = app.screen.map_or(f32::INFINITY, |s| s.width());
    let both = left_holds(app) && right_shown(app).is_some();
    let first = match app.unfolded_last {
        Some(Side::Right) => Side::Left,
        _ => Side::Right,
    };
    (both && window < BOTH_DOCKS_UNDER).then_some(first)
}

/// Unfolds (`open`) or folds a side dock, as the builder asks (its fold
/// button, a strip's button, View › Left Dock or Right Dock and their
/// keys, a panel asked for by name): in a window whose width folds it,
/// for as long as the window does ([`frame_yields`]), the builder's own
/// setting left as it was; else the setting itself (`hide_tree`,
/// `hide_palettes`).
pub(crate) fn set_dock_open(app: &mut Moonglow, side: Side, open: bool) {
    if open {
        app.unfolded_last = Some(side);
    }
    match side {
        Side::Left if app.yields.left_dock_folded => app.left_unfolded_here = open,
        Side::Left => app.settings.hide_tree = !open,
        Side::Right if app.yields.right_dock_folded => app.right_unfolded_here = open,
        Side::Right => app.settings.hide_palettes = !open,
    }
}

/// What View's row of a side dock (Left Dock, Right Dock) and the dock's
/// edge button do where it is not the builder's own fold that has it:
/// `None` where it is (the row's usual tip). In a window that folds the
/// dock by its width, unfolding it there holds for as long as the window
/// is that narrow; where both side docks do not fit, unfolding one folds
/// the other.
pub(crate) fn dock_tip(app: &Moonglow, side: Side) -> Option<String> {
    let (by_width, here, folded, other) = match side {
        Side::Left => {
            let y = app.yields.left_dock_folded;
            (y, app.left_unfolded_here, left_folded(app), RIGHT_DOCK)
        }
        Side::Right => {
            let y = app.yields.right_dock_folded;
            (y, app.right_unfolded_here, right_folded(app), LEFT_DOCK)
        }
    };
    let dock = match side {
        Side::Left => LEFT_DOCK,
        Side::Right => RIGHT_DOCK,
    };
    let squeezed = squeezed(app) == Some(side);
    match (folded, by_width && !here) {
        (true, _) if squeezed => Some(format!("Show the {dock} in place of the {other}")),
        (true, true) => Some(format!("Show the {dock} in this narrow window")),
        (false, false) if by_width => Some(format!("Fold the {dock} back to its strip")),
        _ => None,
    }
}

/// The Left Dock's name: its tab bar's and, folded, its strip's group.
pub(crate) const LEFT_DOCK: &str = "Left Dock";
/// The Right Dock's name: its tab bar's and, folded, its strip's group.
pub const RIGHT_DOCK: &str = "Right Dock";
/// The Right Dock's panel's id, under which egui keeps its width (from
/// run to run, as eframe keeps egui's memory). Its own: no other panel or
/// area of the window is named so.
pub(crate) const RIGHT_DOCK_ID: &str = "right-dock";
/// The Right Dock's least width (`Narrow.png`'s 280): the palette's type
/// strip in two rows, its list's toolbar on one.
pub const RIGHT_DOCK_LEAST: f32 = kit::RIGHT_DOCK_NARROW;
/// The late slot: where a panel's body that draws a 3D view an editor's
/// tab may also show is drawn in this frame, after the editor area
/// ([`draw_late`]; a model is drawn in one place a frame, `model_view`, and
/// the tab keeps its view): the panel, its rectangle in its host (the
/// Right Dock, or an editor's window), the host's layer, and the host's
/// pane in the accessibility tree that its widgets are under, the host,
/// and the editor it draws for (the active docked editor's in a dock, the
/// floating editor's in its window). Generalized from the Preview's slot
/// of M3's T6 (U2).
#[derive(Clone, Debug)]
pub(crate) struct LateSlot {
    pub(crate) panel: Panel,
    pub(crate) rect: egui::Rect,
    pub(crate) layer: egui::LayerId,
    pub(crate) pane: egui::Id,
    pub(crate) host: Host,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the Preview reads it once a blueprint brings its own (U11)")
    )]
    pub(crate) editor: Option<crate::Tab>,
}

impl LateSlot {
    /// `builder` given the id salt of a body drawn in it, or in its
    /// place: the dock's `salt` as it always was, a window's with its
    /// pane, so that two of one panel in one frame (the dock's and a
    /// floating editor's) keep apart.
    pub(crate) fn salted(
        builder: egui::UiBuilder,
        host: Host,
        salt: &'static str,
        pane: egui::Id,
    ) -> egui::UiBuilder {
        match host {
            Host::Dock => builder.id_salt(salt),
            Host::Window => builder.id_salt((salt, pane)),
        }
    }
}

/// The panels' bodies kept for the late slot this frame
/// ([`Moonglow::late_slots`]), drawn after the editor area, each in its
/// room: the Preview's (`area_tools::preview_late`).
pub(crate) fn draw_late(app: &mut Moonglow, ui: &mut Ui) {
    for slot in std::mem::take(&mut app.late_slots) {
        match slot.panel {
            Panel::Preview => crate::area_tools::preview_late(app, ui, slot),
            #[cfg(test)]
            Panel::Sample(_) => {
                app.sample_late_drawn.push((slot.panel, slot.editor));
            }
            _ => {}
        }
    }
}

/// Where a panel's body is drawn ([`panel_body`]): in a dock of the main
/// window, or in the editor's own window, its right or bottom pane, where
/// the editor floats and brings it there ([`Brings::floats`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Host {
    Dock,
    Window,
}

/// The Log's tools on the Bottom Dock's tab bar ([`Panel::header_tools`]).
const LOG_TOOLS: [TopBarButton; 3] = [
    (Id::CopyLog, Icon::Copy, "Copy Log", "Copy the whole log to the clipboard"),
    (Id::SaveLog, Icon::Save, "Save Log", "Save the whole log into a text file"),
    (Id::ClearLog, Icon::Eraser, "Clear Log", "Empty the log"),
];
/// A report tab's tool (M4's U28): its Close, at the tab bar's end.
const REPORT_TOOLS: [TopBarButton; 1] =
    [(Id::CloseReport, Icon::Close, "Close Report", "Close this report")];

/// The temporary datum a panel in the Bottom Dock leaves where its first
/// row ends ([`first_row_ends`]).
const FIRST_ROW: &str = "bottom-dock-first-row";

/// A panel's body (Search Results, the reports' tabs) says where its first
/// row ends, under its form or header: the Bottom Dock is never lower than
/// that and the room under it, so that its least height shows the form and
/// a whole row (M4's U28). In a script's window's pane, nothing reads it.
pub(crate) fn first_row_ends(ui: &Ui, y: f32) {
    // (The lowest said this frame: a form says where it ends whole, its
    // rows where they are under it as drawn.)
    let id = egui::Id::new(FIRST_ROW);
    ui.ctx().data_mut(|d| {
        let y = d.get_temp::<f32>(id).map_or(y, |was| was.max(y));
        d.insert_temp(id, y);
    });
}

/// A body with no rows in sight (nothing found yet, an empty state) says
/// where what it drew ends, unless a row said where it ends: the dock's
/// least height shows its form whole all the same.
pub(crate) fn drawn_ends(ui: &Ui) {
    let id = egui::Id::new(FIRST_ROW);
    if ui.ctx().data(|d| d.get_temp::<f32>(id)).is_none() {
        first_row_ends(ui, ui.min_rect().bottom());
    }
}

/// The Bottom Dock's name: its tab bar's group.
pub(crate) const BOTTOM_DOCK: &str = "Bottom Dock";
/// The Test Module arrow's name (it opens the menu of the other ways to
/// test: Choose Character).
pub(crate) const TEST_MENU: &str = "More Ways to Test";

/// Every icon the main window's bars show: the Top Bar's, Test Module's
/// arrow, the Module panel's header buttons and the Resources panel's.
#[cfg(test)]
pub(crate) fn icons() -> Vec<Icon> {
    let mut all: Vec<Icon> = TOP_BAR.icons().collect();
    all.push(Icon::ChevronDown);
    all.extend(MODULE_TOOLS.iter().map(|t| t.1));
    all.extend([Icon::ExpandAll, Icon::CollapseAll]);
    all.extend([Icon::Export, Icon::Update]);
    all
}

/// The Module panel's header buttons that run commands (the old
/// toolbar's Properties and New Area), as the Top Bar's: command, icon,
/// name, tip. (Expand All and Collapse All after them are the tree's:
/// [`module_tools`].) The tree's row of the same name is apart from the
/// button: in the group [`tree::MODULE_TREE`], a selectable row.
const MODULE_TOOLS: [TopBarButton; 2] = [
    (Id::ModuleProperties, Icon::Module, "Module Properties", "Open the module's properties"),
    (Id::AreaWizard, Icon::Plus, "New Area", "Make a new area with the Area Wizard"),
];

/// The id of the Left Dock's upper panel over the Outliner (its tabs and
/// the panel shown), under which egui keeps the height it is drawn at;
/// the height chosen is `Settings::module_panel_height`.
pub(crate) const MODULE_PANEL_ID: &str = "module-panel";
/// The least height of the Left Dock's upper panel over the Outliner: its
/// tab bar, the Module panel's Filter and five rows.
pub(crate) const MODULE_PANEL_LEAST: f32 = 180.0;
/// The Outliner's height beside a conversation where the builder has not
/// dragged its seam (the Conversation board's `.outliner-panel`, 166).
pub(crate) const OUTLINER_BOOKMARKS_HIGH: f32 = 166.0;
/// The Left Dock's panel over the Outliner beside a table until its seam
/// is dragged (M4's U16, the Table board's `.res-panel`: 254 high), the
/// Outliner's columns the rest.
pub(crate) const OVER_COLUMNS_HIGH: f32 = 254.0;
/// The Left Dock's panel over the Outliner beside a NUI designer until its
/// seam is dragged (M4's U18, the NUI board's `.module-panel`: 416 high),
/// its Layers the rest.
pub(crate) const OVER_LAYERS_HIGH: f32 = 416.0;
/// The Left Dock's panel over the Outliner beside a tileset until its seam
/// is dragged (M4's U20, the Tileset board's `.module-panel`: 394 high), its
/// parts the rest.
pub(crate) const OVER_PARTS_HIGH: f32 = 394.0;

/// The Left Dock's panel over the Outliner beside a hak until its seam is
/// dragged (the Hak board's `.module-panel`, 412; M4's U21).
pub(crate) const OVER_TYPES_HIGH: f32 = 412.0;
/// The Left Dock's panel over the Outliner beside the faction editor until
/// its seam is dragged (the Factions board's `.module-panel`, 452; M4's
/// U22).
pub(crate) const OVER_FACTIONS_HIGH: f32 = 452.0;
/// The Outliner's least height: its header and four rows.
pub(crate) const OUTLINER_LEAST: f32 = 116.0;

/// The Left Dock's least width: its tab bar's tabs as icons and the
/// Module panel's four header buttons, side by side (the tabs' words
/// come back where they fit beside the buttons).
pub(crate) const LEFT_DOCK_LEAST: f32 = 190.0;

/// What the Module panel's header buttons asked: a command to run, and
/// Expand All (`Some(true)`) or Collapse All (`Some(false)`).
type ModuleAsked = (Option<Id>, Option<bool>);

/// The Module panel's header buttons (`.panel-tabs .tools`): Module
/// Properties and New Area (`commands`), then Expand All and Collapse All,
/// their tips with their keys (Options › Keyboard).
fn module_tools(ui: &mut Ui, commands: &[Prepared], keymap: &crate::keys::Keymap) -> ModuleAsked {
    let mut asked = (None, None);
    for b in commands {
        if b.show(ui, Tools::icons_alone(true)) {
            asked.0 = Some(b.id);
        }
    }
    asked.1 = crate::palette_view::fold_icons(ui, "group", keymap);
    asked
}

/// The thickness of the line a panel draws along its inner edge, which
/// egui counts in a panel's size.
fn panel_line(ui: &Ui) -> f32 {
    ui.visuals().widgets.noninteractive.bg_stroke.width.round()
}

/// A command's key as its tip says it: " (Ctrl+N)", or nothing.
fn key_note(app: &Moonglow, ctx: &egui::Context, id: Id) -> String {
    match app.keymap.label_of(&id.id(), ctx) {
        k if k.is_empty() => String::new(),
        k => format!(" ({k})"),
    }
}

/// A button of a bar, read from the window before the bar is drawn (the
/// bar's parts are laid out by closures that can't all hold the window).
struct Prepared {
    id: Id,
    icon: Icon,
    name: &'static str,
    tip: String,
    enabled: bool,
}

impl Prepared {
    fn of(app: &Moonglow, ctx: &egui::Context, (id, icon, name, tip): TopBarButton) -> Prepared {
        // (Undo and Redo say the step they take, as the Edit menu does.)
        let tip = if matches!(id, Id::Undo | Id::Redo) { id.label(app) } else { tip.to_string() };
        let tip = format!("{tip}{}", key_note(app, ctx, id));
        Prepared { id, icon, name, tip, enabled: id.enabled(app) }
    }

    /// Drawn as `tools` labels it (its icon alone, or with its name), its
    /// tip shown enabled or not; whether it was clicked.
    fn show(&self, ui: &mut Ui, tools: Tools) -> bool {
        let r = ui.add_enabled(self.enabled, tools.widget(self.icon, self.name));
        tools.named(r, self.name, &self.tip).clicked()
    }

    /// A build action: [`Prepared::show`] framed as a word button is
    /// (where it is its icon alone too: the mock's `.btn`), joined to the
    /// button beside it where `join` says (Test Module and its arrow).
    fn show_framed(&self, ui: &mut Ui, tools: Tools, join: Option<Join>) -> bool {
        let mut button = tools.widget(self.icon, self.name).framed(true);
        if let Some(join) = join {
            button = button.joined(join);
        }
        let r = ui.add_enabled(self.enabled, button);
        tools.named(r, self.name, &self.tip).clicked()
    }
}

/// The Top Bar (`Panel::top("menu")`), drawn first; returns its rectangle
/// (Undo and Redo on it are the editor's the pointer came from).
pub(crate) fn top_bar(app: &mut Moonglow, ui: &mut Ui) -> egui::Rect {
    let t = mg_uikit::tokens(ui.ctx());
    // (The panel's line under it counted in: the row itself 30 high.)
    let size = metrics::TOP_BAR + panel_line(ui);
    egui::Panel::top("menu")
        .exact_size(size)
        .resizable(false)
        .frame(kit::top_bar_frame(&t))
        .show(ui, |ui| top_bar_row(app, ui))
        .response
        .rect
}

fn top_bar_row(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    // (Decided once as the frame began: `frame_yields`.)
    let yields = app.yields;
    let quick: Vec<Option<Prepared>> =
        TOP_BAR.quick.iter().map(|b| b.map(|b| Prepared::of(app, &ctx, b))).collect();
    let build: Vec<Prepared> = TOP_BAR.build.iter().map(|b| Prepared::of(app, &ctx, *b)).collect();
    let test_menu: Vec<CommandRow> =
        TOP_BAR_TEST_MENU.iter().map(|id| CommandRow::of(app, &ctx, *id)).collect();
    let words = Tools::icons_alone(!yields.build_words);
    // In a window too narrow for them all, the build actions go, then the
    // quick actions (each group whole, in that order as the window
    // narrows), rather than overlap or leave the window: every one of
    // them is in the menus too (File, Edit, Build).
    let side = IconButton::new(Icon::New, "").icon_only(true).width(ui);
    let quick_width: f32 = (quick.iter())
        .map(|b| if b.is_some() { side } else { VSEP })
        .map(|w| w + kit::TOP_BAR_GAP)
        .sum::<f32>()
        + VSEP;
    let arrow_width = IconButton::new(Icon::ChevronDown, "").icon_only(true);
    let arrow_width = arrow_width.joined(Join::Previous).width(ui);
    let build_width = (build.iter())
        .map(|b| words.widget(b.icon, b.name).width(ui) + kit::TOP_BAR_RIGHT_GAP)
        .sum::<f32>()
        + mg_uikit::joined_spacing(ui)
        + arrow_width;
    // Quick Open in the middle: the field, at most 440 (300 under 1300),
    // as wide as the room left; an icon button where that is under 240.
    // Its icon needs this much room, beside the groups' gaps.
    let quick_open_least = mg_uikit::metrics::ICON_BUTTON + 2.0 * kit::TOP_BAR_MID_PADDING;
    let key = app.keymap.label(crate::keys::Cmd::QuickOpen, &ctx);
    let busy = app.busy();
    let mut qo = std::mem::take(&mut app.quick_open);
    // (Decided once the menus are laid out, for both groups: the quick
    // actions where they fit after the menus, the build actions only where
    // both fit and Quick Open's icon too. As the window narrows: Quick
    // Open's field narrows to 240 and becomes its icon (it is the one way
    // to it by the pointer but Help's Command Palette, which opens it
    // among the commands); then the build actions go (each is in the Build
    // menu); then the icon (Ctrl+P and Help › Command Palette… still open
    // it, over the bar); last the quick actions, which New, Open, Save,
    // Undo and Redo are used more often than the rest.)
    let show_build = std::cell::Cell::new(false);
    let quick_shown = std::cell::Cell::new(false);
    let (quick_clicked, (), build_clicked) = kit::top_bar(
        ui,
        yields.quick_open_cap,
        |ui| {
            commands::menu_bar(app, ui);
            let mut clicked = None;
            let menus_end = ui.cursor().min.x + kit::TOP_BAR_GAP;
            let right = ui.max_rect().right();
            let show_quick = menus_end + quick_width <= right;
            quick_shown.set(show_quick);
            let with_icon = menus_end + quick_width + quick_open_least;
            show_build.set(show_quick && with_icon + build_width <= right);
            if show_quick {
                kit::vsep(ui);
                for b in &quick {
                    match b {
                        None => {
                            kit::vsep(ui);
                        }
                        Some(b) => {
                            if b.show(ui, Tools::icons_alone(true)) {
                                clicked = Some(b.id);
                            }
                        }
                    }
                }
            }
            clicked
        },
        // (Nothing where the quick actions went: the icon goes first.)
        |ui, width| {
            let width = if quick_shown.get() { width } else { 0.0 };
            crate::quick_open::middle(ui, width, &mut qo, &key, !busy);
        },
        |ui| {
            let mut clicked = None;
            if !show_build.get() {
                return None;
            }
            for (i, b) in build.iter().enumerate() {
                let last = i + 1 == build.len();
                if last {
                    // (Test Module and its arrow, one control, 1 px apart:
                    // the mock's `.join`. egui puts the spacing after a
                    // widget as it adds it.)
                    ui.spacing_mut().item_spacing.x = mg_uikit::joined_spacing(ui);
                }
                if b.show_framed(ui, words, last.then_some(Join::Next)) {
                    clicked = Some(b.id);
                }
            }
            let arrow = IconButton::new(Icon::ChevronDown, TEST_MENU)
                .icon_only(true)
                .joined(Join::Previous);
            // (Enabled while one of its rows is, as Test Module is.)
            let any = test_menu.iter().any(CommandRow::enabled);
            let menu = ui.add_enabled_ui(any, |ui| {
                arrow.menu(ui, |ui| {
                    let mut chosen = None;
                    for row in &test_menu {
                        if row.show(ui) {
                            chosen = Some(row.id);
                        }
                    }
                    chosen
                })
            });
            let tip = "More ways to test the module";
            menu.inner.response.on_hover_text(tip).on_disabled_hover_text(tip);
            if let Some(Some(id)) = menu.inner.inner {
                clicked = Some(id);
            }
            clicked
        },
    );
    // (A menu's row run while the state was out: Help › Command
    // Palette… opened the one left in its place.)
    let asked = std::mem::replace(&mut app.quick_open, qo);
    if asked.open {
        let prefix =
            if crate::quick_open::parse(&asked.text).0 { crate::quick_open::COMMANDS } else { "" };
        app.quick_open.open_with(prefix);
    }
    if let Some(id) = quick_clicked {
        id.run(app, &ctx);
    }
    if let Some(id) = build_clicked {
        id.run(app, &ctx);
    }
}

/// The Status Bar (`Panel::bottom("status")`): the Log's switch at the
/// left; in the middle, while the Bottom Dock is closed, the log's last
/// line ([`last_line`]; pressed, it opens the log), held to
/// [`last_line_cap`], and after it the active editor's hints ([`shown`]:
/// key caps and a word, dropping whole from the end); at the right the
/// editor's counts, a running job (its name and bar), the module's name
/// (with "(modified)", its path in the tip) and the game folder.
///
/// As the window narrows they yield in one order (SPEC's section 12:
/// the switches become icons, then the folder drops, before what is in
/// the middle; the boards' `.sb-log` and `.sb-hints`: the hints take what
/// the line leaves, `flex: 1 1 0`):
///
/// 0. the Bottom Dock's switches show their icons alone, their words in
///    their tips (under 1400, `Yields::status_switch_icons`, or where
///    everything does not fit beside their words);
/// 1. the game folder goes (under 1400, `Yields::game_folder`; above, it
///    is shown only where every hint and the line at its cap fit beside
///    it);
/// 2. the hints drop whole from the end (a notice at their head is cut
///    short with an ellipsis before it drops);
/// 3. the last line is cut short down to its least room
///    ([`LAST_LINE_LEAST`]; a shorter line is shown whole while it
///    fits), then goes;
/// 4. the counts go whole from the end;
/// 5. the job's name is cut short (its bar kept);
/// 6. last the module's name is cut short ("(modified)" kept whole),
///    shared with the note of a look under way as before ([`share`]).
pub(crate) fn status_bar(app: &mut Moonglow, ui: &mut Ui) {
    let t = mg_uikit::tokens(ui.ctx());
    let size = metrics::STATUS_BAR + panel_line(ui);
    let ctx = ui.ctx().clone();
    // (Made after the editor area, last frame: `update_shown`.)
    let shown = std::mem::take(&mut app.bar_shown);
    let cap = last_line_cap(app);
    // (The Bottom Dock's switches, in their order, each on while its
    // panel is shown: the Log's at least.)
    // (Each with its badge, made from the app here.)
    let switched: Vec<(Panel, bool, String, Option<mg_uikit::Badge>)> = (switches(app).into_iter())
        .map(|p| {
            let tip = match p {
                Panel::Log => {
                    format!("Show the log, or close it{}", key_note(app, &ctx, Id::ViewLog))
                }
                Panel::Compiler => "Show what the script's latest compile said, or close it".into(),
                Panel::Help => {
                    "Show the comment and declaration of the name under the cursor, or close it"
                        .into()
                }
                Panel::Bookmarks => "Show the script's bookmarks, or close them".into(),
                Panel::SearchResults => "Show what the last search found, or close it".into(),
                Panel::Checks => {
                    "Show what the checks of the NUI window found, or close them".into()
                }
                Panel::Problems => "Show what the checks found, or close it".into(),
                _ => format!("Show the {} panel, or close it", p.name()),
            };
            (p, bottom_shows(app, p), tip, p.badge(app, app.active_editor()))
        })
        .collect();
    // (The log's last line shows while the log is not.)
    let log_on = bottom_shows(app, Panel::Log);
    let looking = app.looking_note();
    let modified = app.ws.as_ref().is_some_and(|ws| ws.is_modified());
    let (module, module_tip) = match (app.module_path(), &app.ws) {
        (Some(p), _) => (module_name(&p), Some(p.display().to_string())),
        (None, Some(_)) => ("New module, not saved yet".to_string(), None),
        (None, None) => ("No module open".to_string(), None),
    };
    let (folder, folder_tip) = match &app.install {
        Some(gi) => {
            let name = gi.root.file_name().map(|n| n.to_string_lossy().into_owned());
            let shown = name.map_or_else(
                || gi.root.display().to_string(),
                |n| format!("…{}{n}", std::path::MAIN_SEPARATOR),
            );
            (shown, Some(gi.root.display().to_string()))
        }
        None => ("No game install".to_string(), None),
    };
    // The log's last line, while the dock that shows the log is closed:
    // its message's first line (the board's, with no time), in the log's
    // color for a warning or an error; its time and its first lines in
    // the tip, made only while the line is hovered or has the focus.
    let log = &app.log;
    // (The editor's message in its place, whether the log shows or not:
    // a script's live error, M4's U13.)
    // (A warning or an error logged since it was found is shown instead,
    // while the log is closed: it is newer, and as urgent.)
    let alarm = |i: usize| {
        i >= shown.note.as_ref().map_or(0, |n| n.since)
            && !matches!(log.entries[i].0, crate::Level::Info)
    };
    let newer = (!log_on).then(|| last_line(log)).flatten().filter(|(i, _)| alarm(*i));
    let message = shown.note.clone().filter(|_| newer.is_none());
    // (A script's live error is shown whole wherever the bar has room for
    // it, 5.2.0: the owner's rulings on Jasperre's report and at review.
    // No cap; the others give way only as far as its width needs: the
    // folder, then the hints from the last, then the switches' words, then
    // the module's name is cut short down to `ERROR_LEAVES`, then the
    // counts (the cursor's line and column last: the builder reads it
    // against the error's line); only then is the message cut, whole in
    // its tip.)
    let error_line = message.is_some();
    let cap = if error_line { f32::INFINITY } else { cap };
    let last = match &message {
        Some(n) => Some((None, n.text.clone(), Some(t.error))),
        None => (!log_on).then(|| last_line(log)).flatten().map(|(i, line)| {
            let color = match log.entries[i].0 {
                crate::Level::Info => None,
                crate::Level::Warning => Some(t.warn),
                crate::Level::Error => Some(t.error),
            };
            (Some(i), line, color)
        }),
    };
    let tips_made = std::cell::Cell::new(0);
    let job = app.job_shown().map(|crate::jobs::JobShown { title, steps, note }| {
        let (name, mut tip) = match steps {
            Some((done, total)) => {
                (format!("{title}, {done} of {total}"), format!("{title}: {done} of {total}"))
            }
            None => (format!("{title}, under way"), format!("{title}: under way")),
        };
        if !note.is_empty() {
            tip = format!("{tip}\n{note}");
        }
        let job = match steps {
            Some((d, n)) => StatusJob::new(title, Some(d as f32 / n as f32)),
            None => StatusJob::unknown(title),
        };
        job.name(name).tooltip(tip)
    });
    let hints = &shown.hints;
    let yields = app.yields;
    let (clicked, last_clicked) = egui::Panel::bottom("status")
        .exact_size(size)
        .resizable(false)
        .frame(status::frame(&t))
        .show(ui, |ui| {
            let switches: Vec<StatusSwitch> = (switched.iter())
                .map(|(p, on, tip, badge)| {
                    StatusSwitch::new(p.icon(), p.name(), *on).badge_if(badge.clone()).tooltip(tip)
                })
                .collect();
            // ("(modified)" is kept whole where the name is cut short: it
            // is the part that matters.)
            let mut item = StatusItem::new(&module).modified(modified);
            if modified {
                item = item.kept(" (modified)");
            }
            if let Some(tip) = &module_tip {
                item = item.tooltip(tip);
            }
            let mut folder_item = StatusItem::new(&folder).weak();
            if let Some(tip) = &folder_tip {
                folder_item = folder_item.tooltip(tip);
            }
            let mut counts: Vec<StatusItem> = (shown.counts.iter())
                .map(|c| {
                    let item = if c.warn {
                        StatusItem::new(&c.text).warn()
                    } else {
                        StatusItem::new(&c.text)
                    };
                    match &c.tip {
                        Some(tip) => item.tooltip(tip),
                        None => item,
                    }
                })
                .collect();
            // (Its whole words in its tip, where it is cut short.)
            let mut note = looking.map(|n| StatusItem::new(n).tooltip(n));
            let mut job = job;
            // The middle at its fullest: a separator after the switch,
            // the last line at its cap, the hints whole.
            let between = 2.0 * status::GAP + status::SEPARATOR_WIDTH;
            let line_full =
                last.as_ref().map(|(_, text, _)| StatusLog::new(text).width(ui).min(cap));
            // (Each hint measured once, for its width, its fit and its
            // drawing.)
            let measured = Hints::new(hints).measure(ui);
            let hints_full = measured.width();
            let middle_full = match (line_full, hints.is_empty()) {
                (None, true) => 0.0,
                (None, false) => status::SEPARATOR_WIDTH + status::GAP + hints_full,
                (Some(l), true) => status::SEPARATOR_WIDTH + status::GAP + l,
                (Some(l), false) => status::SEPARATOR_WIDTH + 2.0 * status::GAP + l + hints_full,
            };
            // The right-hand items by the order they yield in: the folder
            // first (where it does not fit beside the middle at its
            // fullest), the middle's own (the hints, then the line, in
            // `middle` below); then the counts whole from the end; then
            // the job's name is cut short, down to its bar; then the note
            // and the module's name are cut short with an ellipsis
            // (`share`), each to half the room where both are wider, or
            // to what the other leaves (the name's path in its tip,
            // "(modified)" kept whole).
            let note_gap = if note.is_some() { between } else { 0.0 };
            let job_gap = if job.is_some() { between } else { 0.0 };
            let note_width = note.as_ref().map_or(0.0, |n| n.width(ui));
            let job_width = job.as_ref().map_or(0.0, |j| j.width(ui));
            let count_widths: Vec<f32> = counts.iter().map(|c| c.width(ui) + between).collect();
            let core = item.width(ui) + note_width + note_gap + job_width + job_gap;
            let all_counts: f32 = count_widths.iter().sum();
            let with_folder = core + all_counts + between + folder_item.width(ui);
            // Under 1400 the switches show their icons alone first
            // (`Yields::status_switch_icons`, the Narrow board's), then the
            // folder goes (`Yields::game_folder`), then the hints drop.
            // From 1400 up (SPEC 12, proposal 5.2, Script.png at 1600: the
            // coordinator's ruling at U13's review) the folder goes where it
            // does not fit beside the middle at its fullest, then the hints
            // drop whole, and the switches lose their words only where the
            // bar does not fit beside them with no hint and the line at its
            // cap; then the line is cut.
            let gaps = 2.0 * status::GAP;
            let full = with_folder + middle_full;
            let bare = core
                + all_counts
                + line_full.map_or(0.0, |l| status::SEPARATOR_WIDTH + status::GAP + l);
            let icons = yields.status_switch_icons
                || switches_width(ui, &switches) + gaps + bare > ui.available_width();
            let switches: Vec<StatusSwitch> =
                switches.into_iter().map(|s| s.icon_only(icons)).collect();
            let left = switches_width(ui, &switches) + gaps;
            let room = ui.available_width() - left;
            // (The folder only where it fits beside the middle at its
            // fullest, a live error's too: it gives way first, before the
            // hints (the coordinator's ruling at W5's review: a wider
            // window never shows fewer hints). Beside a live error the
            // hints then give way in the middle as far as it needs, and
            // the right-hand items have what the whole message leaves.)
            let error_need = line_full.map_or(0.0, |l| status::SEPARATOR_WIDTH + gaps + l);
            let show_folder = yields.game_folder && full <= room;
            let room = if error_line { room - error_need } else { room };
            // (Where a count goes, the middle has gone before it: what the
            // count leaves is no room for the line.)
            let mut middle_on = true;
            if error_line && !show_folder {
                // (The name and the others cut first, the name to
                // `ERROR_LEAVES` at the least; then the counts go from the
                // first, the editor's own last (the Resources panel's goes
                // before it), a script's line and column the last of all;
                // what is left of the room is the message's, cut where it
                // is still wider.)
                let least = core.min(ERROR_LEAVES + note_width + note_gap + job_width + job_gap);
                let n = count_widths.len();
                let mut kept = n;
                while kept > 0 && least + count_widths[n - kept..].iter().sum::<f32>() > room {
                    kept -= 1;
                }
                counts.drain(..n - kept);
                let core_room = (room - count_widths[n - kept..].iter().sum::<f32>()).max(least);
                if core > core_room {
                    let others = item.width(ui) + note_width + note_gap + job_gap;
                    job = job.map(|j| j.max_width(core_room - others));
                    let job_width = job.as_ref().map_or(0.0, |j| j.width(ui));
                    let rest = core_room - note_gap - job_width - job_gap;
                    let (name_max, note_max) = share(rest, item.width(ui), note_width);
                    item = item.max_width(name_max);
                    note = note.map(|n| n.max_width(note_max));
                }
            } else if !show_folder {
                // (The middle yields before the counts: they go only where
                // the right-hand items alone are wider than the bar. Not a
                // live error, before which the counts go.)
                let mut kept = count_widths.len();
                while kept > 0 && core + count_widths[..kept].iter().sum::<f32>() > room {
                    kept -= 1;
                }
                middle_on = kept == counts.len();
                counts.truncate(kept);
                if core > room {
                    let others = item.width(ui) + note_width + note_gap + job_gap;
                    job = job.map(|j| j.max_width(room - others));
                    let job_width = job.as_ref().map_or(0.0, |j| j.width(ui));
                    let rest = room - note_gap - job_width - job_gap;
                    let (name_max, note_max) = share(rest, item.width(ui), note_width);
                    item = item.max_width(name_max);
                    note = note.map(|n| n.max_width(note_max));
                }
            }
            let mut right = item.width(ui)
                + note.as_ref().map_or(0.0, |n| n.width(ui))
                + note_gap
                + job.as_ref().map_or(0.0, |j| j.width(ui))
                + job_gap
                + counts.iter().map(|c| c.width(ui) + between).sum::<f32>();
            if show_folder {
                right += between + folder_item.width(ui);
            }
            let (clicked, last_clicked, ()) = status::bar_sized(
                ui,
                right,
                |ui| {
                    let mut clicked = None;
                    for (switch, (panel, ..)) in switches.into_iter().zip(&switched) {
                        if ui.add(switch).clicked() {
                            clicked = Some(*panel);
                        }
                    }
                    clicked
                },
                |ui, room| {
                    if !middle_on {
                        return None;
                    }
                    // (After a separator, as the boards have it.)
                    let inner = room - status::SEPARATOR_WIDTH - status::GAP;
                    // The line takes its room first, up to its cap, and is
                    // shown in its least or more; the hints have what it
                    // leaves (the boards' `flex: 0 1 auto` and `1 1 0`).
                    let line = last.as_ref().zip(line_full).and_then(|((i, text, color), full)| {
                        let fits = full <= inner || inner >= LAST_LINE_LEAST;
                        fits.then_some((*i, text, *color, full.min(inner)))
                    });
                    let hints_room = match &line {
                        Some((.., width)) => inner - width - status::GAP,
                        None => inner,
                    };
                    let any_hint = measured.fitting(hints_room) > 0;
                    if line.is_none() && !any_hint {
                        return None;
                    }
                    status::separator(ui);
                    let mut pressed = None;
                    if let Some((i, text, color, width)) = line {
                        let name = match i {
                            Some(_) => format!("Last log line: {text}"),
                            None => format!("Found as you type: {text}"),
                        };
                        let mut line = StatusLog::new(text.as_str()).name(name).max_width(width);
                        if let Some(color) = color {
                            line = line.color(color);
                        }
                        let mut shown = ui.add(line);
                        if shown.hovered() || shown.has_focus() {
                            tips_made.set(tips_made.get() + 1);
                            let tip = match (i, &message) {
                                (Some(i), _) => last_line_tip(log, i),
                                (None, Some(n)) => n.tip.clone(),
                                (None, None) => String::new(),
                            };
                            shown = shown.on_hover_text(tip);
                        }
                        pressed = Some(shown.clicked());
                    }
                    if any_hint {
                        measured.show(ui, hints_room);
                    }
                    pressed
                },
                |ui| {
                    for count in counts {
                        ui.add(count);
                        status::separator(ui);
                    }
                    if let Some(note) = note {
                        ui.add(note);
                        status::separator(ui);
                    }
                    if let Some(job) = job {
                        ui.add(job);
                        status::separator(ui);
                    }
                    ui.add(item);
                    if show_folder {
                        status::separator(ui);
                        ui.add(folder_item);
                    }
                },
            );
            (clicked, last_clicked == Some(true))
        })
        .inner;
    app.last_line_tips += tips_made.get();
    app.bar_shown = shown;
    // A switch on closes the dock; one off opens it on its panel.
    if let Some(panel) = clicked {
        let on = switched.iter().any(|(p, on, ..)| *p == panel && *on);
        if !on {
            choose_bottom(app, panel);
        }
        app.settings.hide_log = on;
    } else if last_clicked {
        match message {
            // (The live error's line, the keys in its script's text.)
            Some(Note { at: Some((key, line)), .. }) => {
                let text = app.script_source(&key).unwrap_or_default();
                app.script_tools.jump = Some((key, crate::script_tools::line_start(&text, line)));
                app.script_tools.keys_back = Some(key);
            }
            Some(_) => {}
            None => app.show_panel(Id::ViewLog, true),
        }
    }
}

/// The width of the Status Bar's switches side by side, [`status::GAP`]
/// apart (the bar's `gap: 8px`), as its left group lays them out: each
/// switch's own width ([`StatusSwitch::width`]: its icon, its words unless
/// it shows its icon alone, its badge) and the gaps between them.
pub(crate) fn switches_width(ui: &Ui, switches: &[StatusSwitch]) -> f32 {
    let gaps = status::GAP * switches.len().saturating_sub(1) as f32;
    switches.iter().map(|s| s.width(ui)).sum::<f32>() + gaps
}

/// How many of the log's entries, from its end, the Status Bar looks in
/// for its last line: the last of them whose first line is not blank (a
/// project command's blank output lines, `"  "`, are passed over). Where
/// all of them are blank, no line is shown; a log of thousands of blank
/// lines is not walked each frame.
pub(crate) const LAST_LINE_LOOK_BACK: usize = 64;
/// The most characters of the last line the Status Bar shows and names
/// its button by (a longer one is cut with an ellipsis): a message of
/// megabytes is not laid out, or said, whole each frame.
pub(crate) const LAST_LINE_CHARS: usize = 300;
/// The bytes of an entry read for its first line.
const LAST_LINE_BYTES: usize = 4096;
/// The lines of an entry its tip shows (after its time), each cut to
/// [`LAST_LINE_CHARS`]; more, and the tip says so.
const LAST_LINE_TIP_LINES: usize = 8;

/// The log's last line as the Status Bar shows it: the entry's place in
/// the log and its first line, trimmed and cut to [`LAST_LINE_CHARS`]
/// with an ellipsis, from the last of the last [`LAST_LINE_LOOK_BACK`]
/// entries whose first line is not blank.
pub(crate) fn last_line(log: &crate::Log) -> Option<(usize, String)> {
    let n = log.entries.len();
    let from = n.saturating_sub(LAST_LINE_LOOK_BACK);
    (from..n).rev().find_map(|i| first_line(&log.entries[i].1).map(|l| (i, l)))
}

/// `text` up to `bytes` (at a character's boundary).
fn head(text: &str, bytes: usize) -> &str {
    let mut end = text.len().min(bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// `line` cut to [`LAST_LINE_CHARS`] characters, with an ellipsis where it
/// was cut (or where `more` says it goes on).
fn cut_line(line: &str, more: bool) -> String {
    let mut chars = line.chars();
    let mut out: String = chars.by_ref().take(LAST_LINE_CHARS).collect();
    if more || chars.next().is_some() {
        out.push('…');
    }
    out
}

/// The first line of an entry, trimmed and cut ([`cut_line`]), read from
/// its first [`LAST_LINE_BYTES`] alone: `None` where it is blank.
fn first_line(text: &str) -> Option<String> {
    let head = head(text, LAST_LINE_BYTES);
    let (line, more) = match head.find('\n') {
        Some(at) => (&head[..at], false),
        None => (head, head.len() < text.len()),
    };
    let line = line.trim();
    (!line.is_empty()).then(|| cut_line(line, more))
}

/// The tip of the last line, entry `i`: its time and its first
/// [`LAST_LINE_TIP_LINES`] lines, each cut ([`cut_line`]), and "… more in
/// the log" where it has more.
fn last_line_tip(log: &crate::Log, i: usize) -> String {
    let text = &log.entries[i].1;
    let at = log.times.get(i).map_or("", String::as_str);
    let head = head(text, LAST_LINE_BYTES * LAST_LINE_TIP_LINES);
    let mut lines = head.lines();
    let shown: Vec<String> =
        lines.by_ref().take(LAST_LINE_TIP_LINES).map(|l| cut_line(l, false)).collect();
    let mut tip = format!("{at} {}", shown.join("\n"));
    if lines.next().is_some() || head.len() < text.len() {
        tip.push_str("\n… more in the log");
    }
    tip
}

/// How short a script's live error cuts the module's name on the Status
/// Bar (5.2.0) before the counts give way and the error itself is cut:
/// the name is given this much (its text, cut at a letter with an
/// ellipsis, can be a few points less).
pub(crate) const ERROR_LEAVES: f32 = 120.0;

/// The least room the log's last line is shown in (a short word and its
/// ellipsis); in less, it is not shown.
pub(crate) const LAST_LINE_LEAST: f32 = 48.0;

/// The most of the bar the log's last line takes beside an area's
/// hints (`.sb-log`'s `max-width` on the Main, AreaPaint and AreaTiles
/// boards).
pub(crate) const LAST_LINE_CAP_AREA: f32 = 96.0;
/// The most it takes beside another editor's (150 to 260 on the other
/// boards: the Creature board's 160).
pub(crate) const LAST_LINE_CAP: f32 = 160.0;
/// The most it takes beside a script's (the Script board's `.sb-log`,
/// 170). Not the live error's message (M4's U13), which has no cap since
/// 5.2.0 ([`ERROR_LEAVES`]).
pub(crate) const LAST_LINE_CAP_SCRIPT: f32 = 170.0;

/// The most of the bar the log's last line (or the Status Bar's message)
/// takes now: less beside an area, whose hints are many.
pub(crate) fn last_line_cap(app: &Moonglow) -> f32 {
    match app.active_editor() {
        Some(crate::Tab::Area(_) | crate::Tab::NoArea) => LAST_LINE_CAP_AREA,
        Some(crate::Tab::Script(_)) => LAST_LINE_CAP_SCRIPT,
        _ => LAST_LINE_CAP,
    }
}

/// What the Status Bar shows for the editor in front: its hints, the most
/// useful first (a notice of what is in hand before them), and its counts
/// at the bar's right, before the job.
#[derive(Clone, Debug, Default)]
pub(crate) struct Shown {
    pub(crate) hints: Vec<Hint>,
    pub(crate) counts: Vec<Count>,
    /// The editor's message, in the place of the log's last line (the
    /// Script board's `.sb-log`), shown whether the log is or not: a
    /// script's live error (M4's U13).
    pub(crate) note: Option<Note>,
}

impl Shown {
    /// What it says, a line a hint (as a screen reader says it), a line a
    /// count and its message: two frames that say the same show the same.
    pub(crate) fn said(&self) -> Vec<String> {
        let hints = self.hints.iter().map(Hint::spoken);
        let counts = self.counts.iter().map(|c| c.text.clone());
        hints.chain(counts).chain(self.note.iter().map(|n| n.text.clone())).collect()
    }
}

/// `key`'s live error as the Status Bar's message ([`Note`]), where its
/// check found one: the compiler's words, the tip one short sentence
/// after them, a click going to the line its underline is under (its own,
/// the nearest with text above an end of file's, an include's `#include`
/// line: `script_view::underlined_line`).
pub(crate) fn script_note(app: &Moonglow, key: mg_resman::ResKey) -> Option<Note> {
    let (line, text) = app.live_error(key)?;
    let text = text.replace(['\r', '\n'], " ");
    let tip = format!("{text}\nClick to go to its line.");
    let at = app.error_underline_line(key).or(*line).map(|l| (key, l));
    let since = app.script_nav.live.get(&key).map_or(0, |l| l.logged);
    Some(Note { text, tip, at, since })
}

/// The Status Bar's message of the editor in front ([`Shown::note`]): a
/// script's live error, in the compiler's words (`hello.nss(3): ERROR:
/// …`), in the theme's `error`,
/// its whole words and where it comes from in its tip; a click goes to
/// its line where it says one (the script, its line from 0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Note {
    pub(crate) text: String,
    pub(crate) tip: String,
    pub(crate) at: Option<(mg_resman::ResKey, usize)>,
    /// How many entries the log had when it was found: a warning or an
    /// error logged since takes its place in the bar (as the log's last
    /// line would), an information line does not.
    pub(crate) since: usize,
}

/// One of the Status Bar's counts ("11 by 11 tiles", "1 selected"), with
/// its tip where it has more to say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Count {
    pub(crate) text: String,
    pub(crate) tip: Option<String>,
    /// In the theme's `warn`: a NUI designer's "Not built" and "Build
    /// failed" (M4's U19, the NUI board's `.sb-item.nb`).
    pub(crate) warn: bool,
}

impl Count {
    pub(crate) fn new(text: impl Into<String>) -> Count {
        Count { text: text.into(), tip: None, warn: false }
    }
}

/// The keys a hint shows: a command's by the user's own key bindings
/// (`keys.rs`; a command with no key has no hint), the modifiers of the
/// mouse's gestures by this system's names (`Cmd` and `Option` on macOS,
/// where the area view reads egui's `command` and `alt`).
pub(crate) struct HintKeys<'a> {
    keymap: &'a crate::keys::Keymap,
    mac: bool,
}

impl<'a> HintKeys<'a> {
    pub(crate) fn new(keymap: &'a crate::keys::Keymap, ctx: &egui::Context) -> HintKeys<'a> {
        HintKeys { keymap, mac: ctx.os() == egui::os::OperatingSystem::Mac }
    }

    /// Ctrl's name (Cmd on macOS).
    pub(crate) fn ctrl(&self) -> &'static str {
        if self.mac { "Cmd" } else { "Ctrl" }
    }

    /// Alt's name (Option on macOS).
    pub(crate) fn alt(&self) -> &'static str {
        if self.mac { "Option" } else { "Alt" }
    }

    /// The key caps of `cmd`'s first key, where it has one.
    fn caps(&self, cmd: crate::keys::Cmd) -> Option<Vec<String>> {
        self.keymap.keys(cmd).first().map(|k| crate::keys::caps(k, self.mac))
    }

    /// `cmd`'s first key and `word` (`Ctrl` + `N` New module); none where
    /// it has no key.
    pub(crate) fn hint(&self, cmd: crate::keys::Cmd, word: &str) -> Option<Hint> {
        self.caps(cmd).map(|caps| Hint::keys(caps, word))
    }

    /// The keys of `a` and `b`, which do the same each its way, joined by
    /// `/` (`Q` / `E` Turn), and `word`; the one that has a key alone
    /// where the other has none, and none where neither has.
    pub(crate) fn either(
        &self,
        a: crate::keys::Cmd,
        b: crate::keys::Cmd,
        word: &str,
    ) -> Option<Hint> {
        self.joined([a, b], Some("/"), word)
    }

    /// [`HintKeys::either`] with nothing between the two keys (the Main
    /// board's `Q` `E` Turn by a step).
    pub(crate) fn both(
        &self,
        a: crate::keys::Cmd,
        b: crate::keys::Cmd,
        word: &str,
    ) -> Option<Hint> {
        self.joined([a, b], None, word)
    }

    fn joined(
        &self,
        cmds: [crate::keys::Cmd; 2],
        join: Option<&'static str>,
        word: &str,
    ) -> Option<Hint> {
        let mut hint = Hint::new(word);
        let mut any = false;
        for caps in cmds.into_iter().filter_map(|c| self.caps(c)) {
            if any && let Some(j) = join {
                hint = hint.part(HintPart::Join(j));
            }
            for (i, cap) in caps.into_iter().enumerate() {
                hint = if i == 0 { hint.key(cap) } else { hint.plus(cap) };
            }
            any = true;
        }
        any.then_some(hint)
    }
}

/// What the Status Bar shows from the next frame on ([`shown`]), made
/// once a frame after the editor area is drawn, from the state it left
/// (the Escape a docked tab would take, none where the pointer is over a
/// floating model's window, which an Escape closes; the pointer over
/// Options); where it differs from what the bar showed, that frame is
/// asked for.
pub(crate) fn update_shown(app: &mut Moonglow, ctx: &egui::Context) {
    let mut escape = app.escape_hint_next.clone().filter(|_| !app.model_under_pointer_next);
    let hovered = app.options_hovered_next;
    // (A floating window's editor in place of the docked one: under the
    // pointer, or with the last press while the pointer is over no docked
    // editor.)
    let over_docked = |p: egui::Pos2| {
        app.editor_area_rect().is_some_and(|r| r.contains(p))
            && ctx.layer_id_at(p).is_none_or(|l| l.order != egui::Order::Middle)
    };
    let pointer = ctx.pointer_hover_pos();
    // (The top window under the pointer, by egui's order of the layers.)
    let order = |l: &egui::LayerId| ctx.memory(|m| m.layer_ids().position(|x| x == *l));
    let under =
        (app.window_tabs_next.iter()).filter(|(_, _, over)| *over).max_by_key(|(_, l, _)| order(l));
    let pressed = (app.window_tabs_next.iter())
        .find(|(_, l, _)| app.last_press.is_some_and(|(_, p)| p == Some(*l)))
        .filter(|_| pointer.is_none_or(|p| !over_docked(p)));
    let window = under.or(pressed).map(|(tab, _, _)| tab.clone());
    let front_dock_layer = app.window_layers_drawn.iter().copied().max_by_key(|l| order(l));
    if escape.is_none()
        && let Some(w) = &window
        && let Some(front_layer) = front_dock_layer
        && app.window_escape_candidates.iter().any(|(t, l)| t == w && *l == front_layer)
        && !app.model_under_pointer_next
        && !crate::widgets::dialog_drawn(ctx)
        && ctx.memory(|m| m.top_modal_layer()).is_none()
        && !egui::Popup::is_any_open(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
    {
        escape = Some(w.clone());
    }
    let named = escape.as_ref().map(|t| (t.clone(), crate::tabs::name(app, t)));
    let next =
        shown(app, ctx, named.as_ref().map(|(t, n)| (t, n.as_str())), hovered, window.as_ref());
    if next.said() != app.bar_shown.said() {
        ctx.request_repaint();
    }
    app.bar_shown = next;
}

/// A model view's ways with the pointer, as the Status Bar gives them
/// beside a blueprint's own Preview (the Creature board's).
const MODEL_VIEW_HINTS: [(&[&str], &str); 5] = [
    (&["Drag"], "Turn view"),
    (&["Right Drag"], "Move view"),
    (&["Middle Drag"], "Move view"),
    (&["Wheel"], "Zoom"),
    (&["Shift", "Wheel"], "Zoom finer"),
];

/// Whether Preview is drawn for the active editor (`tab`), floating or docked.
fn preview_shown(app: &Moonglow, tab: &crate::Tab, floating: bool) -> bool {
    if floating {
        app.window_previews_drawn.contains(tab)
    } else {
        !right_folded(app) && Panel::Preview.available(app)
    }
}

/// Whether something in front takes the keys and presses the views'
/// would: an `egui::Modal` (a job's window, a plugin's question), a
/// dialog, a menu or popup. Not a kit popover (an editor header's): it is
/// not modal, and the view's keys go on acting while one is open (the
/// coordinator's decision in U3's review), so its hints stay.
pub(crate) fn keys_blocked(ctx: &egui::Context) -> bool {
    ctx.memory(|m| m.top_modal_layer()).is_some()
        || crate::widgets::dialog_open(ctx)
        || egui::Popup::is_any_open(ctx)
}

/// What the Status Bar shows for the editor in use: `window`, the tab in
/// front of a floating window the pointer is over or that had the last
/// press (`update_shown` chooses it, M4's U23 fix), else the active docked
/// editor ([`Moonglow::active_editor`], as the Right Dock follows it); or for the
/// Resources panel while it is in use (the pointer over it, or its list
/// with the keys: Up and Down and Enter where the list has them, Double
/// Click, Right Click, Find's key with the pointer over it; the editor's
/// counts kept): an area's
/// state's hints and counts (`area_view::bar`); the Welcome page's New
/// module, Open module and Quick Open by their keys; a script's keys (the
/// Script board's); a tab with OK and Cancel (`Tab::escape_cancels`):
/// Enter for Options' OK where it is (the pointer over it, nothing with
/// the keyboard), Esc for its Cancel where an Escape would be its own
/// (`escape`, the tab whose Cancel it would be and its name: ruling R1,
/// T10's rule, as `tabs::Viewer::body` foresees it), and Tab's fields.
/// Where an Escape would be another docked tab's than the active
/// editor's, its Esc comes first, named by it.
pub(crate) fn shown(
    app: &Moonglow,
    ctx: &egui::Context,
    escape: Option<(&crate::Tab, &str)>,
    options_hovered: bool,
    window: Option<&crate::Tab>,
) -> Shown {
    use crate::Tab;
    if let Some(shown) = &app.status_for_tests {
        return shown.clone();
    }
    use crate::keys::Cmd;
    let k = HintKeys::new(&app.keymap, ctx);
    let active = window.or(app.active_editor());
    let esc_mine = escape.is_some_and(|(t, _)| Some(t) == active);
    let mut shown = match active {
        Some(Tab::Area(area)) => {
            let mut shown = (app.area_views.get(area))
                .map(|v| crate::area_view::status(app, v, ctx))
                .unwrap_or_default();
            // (A modal, a dialog or a menu in front: the view's keys and
            // presses do nothing, `response.hovered()` false. The counts
            // stay. A header's popover is not modal: the keys act and the
            // hints stay.)
            if keys_blocked(ctx) {
                shown.hints.clear();
            }
            shown
        }
        // (Not while a modal, a dialog or a menu takes the keys: M4's U24,
        // the area's rule.)
        Some(Tab::Welcome) if keys_blocked(ctx) => Shown::default(),
        Some(Tab::Welcome) => {
            let hints = [
                k.hint(Cmd::NewModule, "New module"),
                k.hint(Cmd::OpenModule, "Open module"),
                k.hint(Cmd::QuickOpen, "Go to resource or command"),
            ];
            Shown { hints: hints.into_iter().flatten().collect(), ..Shown::default() }
        }
        Some(Tab::Script(key)) => {
            let hints = [
                k.hint(Cmd::Find, "Find"),
                k.hint(Cmd::FindNext, "Find next"),
                k.hint(Cmd::Definition, "Definition"),
                k.hint(Cmd::References, "References"),
                k.hint(Cmd::Complete, "Complete"),
                k.hint(Cmd::Bookmark, "Bookmark"),
                k.hint(Cmd::CompileAll, "Compile all"),
                k.hint(Cmd::Replace, "Replace"),
            ];
            // (The Script board's count: the cursor's line and column, as
            // its editor last found them.)
            let (line, column) = app.scripts.get(key).and_then(|b| b.place).map_or((1, 1), |p| p.2);
            let counts = vec![Count::new(format!("Line {line}, column {column}"))];
            // (Its live error, as the Status Bar's message: M4's U13.)
            // (In the compiler's words, which say the script and the line,
            // `hello.nss(5): ERROR: …`, or an include's.)
            let note = script_note(app, *key);
            let hints = hints.into_iter().flatten().collect();
            Shown { hints, counts, note }
        }
        Some(Tab::Dialog(key)) => dialog_shown(app, &k, ctx, *key),
        // (A NUI designer's, M4's U19: the NUI board's; its source group's
        // too, its counts and build state without the canvas's hints.)
        Some(tab @ (Tab::Nui(key) | Tab::NuiSource(key))) => {
            crate::nui_view::shown(app, &k, ctx, *key, matches!(tab, Tab::Nui(_)))
        }
        // (A table's, M4's U16: the Table board's.)
        Some(Tab::Table(id)) => crate::table_view::panels::shown(app, &k, ctx, *id),
        // (A tileset's, M4's U20: the Tileset board's.)
        Some(Tab::Tileset(id)) => crate::tileset_view::shown(app, &k, ctx, *id),
        // (A hak's, M4's U21: the Hak board's.)
        Some(Tab::Hak(id)) => crate::hak_view::shown(app, &k, ctx, *id),
        // (The Appearance Gallery's, M4's U26: the AppearanceGallery board's.)
        Some(Tab::PlaceableGallery) => crate::appearance_gallery::shown(app, ctx),
        // (The journal's and the faction editor's, M4's U22: their boards'.)
        Some(Tab::Journal) => crate::journal_view::shown(app, &k, ctx),
        Some(Tab::Factions) => crate::faction_view::shown(app, &k, ctx),
        Some(Tab::TalkTable) => {
            crate::talk_view::panels::shown(app, ctx, crate::talk_view::Which::shown(app))
        }
        // (A table only looked at, the game's or a hak's copy in the
        // resource view: its counts, and none of the keys that change it.)
        Some(Tab::Resource(key)) if key.restype == mg_core::ResType::TWODA => {
            crate::browser::table_shown(app, &k, ctx, *key, None).unwrap_or_default()
        }
        Some(Tab::ResourceCopy { key, layer }) if key.restype == mg_core::ResType::TWODA => {
            crate::browser::table_shown(app, &k, ctx, *key, Some(layer)).unwrap_or_default()
        }
        // (Any other resource only looked at, M4's U23: which it is, where
        // it is read from and its size, the Resources board's; no keys, as
        // nothing in it changes.)
        // (A text's: its Find's keys, v5.2's W3.)
        Some(Tab::Resource(key)) => Shown {
            hints: crate::browser::shows_text(app, *key, None)
                .then(|| [k.hint(Cmd::Find, "Find"), k.hint(Cmd::FindNext, "Find next")])
                .into_iter()
                .flatten()
                .flatten()
                .collect(),
            counts: crate::browser::resource_count(app, *key, None)
                .map(Count::new)
                .into_iter()
                .collect(),
            ..Shown::default()
        },
        Some(Tab::ResourceCopy { key, layer }) => Shown {
            hints: crate::browser::shows_text(app, *key, Some(layer))
                .then(|| [k.hint(Cmd::Find, "Find"), k.hint(Cmd::FindNext, "Find next")])
                .into_iter()
                .flatten()
                .flatten()
                .collect(),
            counts: crate::browser::resource_count(app, *key, Some(layer))
                .map(Count::new)
                .into_iter()
                .collect(),
            ..Shown::default()
        },
        // (A blueprint's or a placed object's form with its own Preview
        // shown, M4's U11: the 3D view's ways, the Creature board's, after
        // Esc where an Escape would be its own; not Tab's, which the board
        // leaves out.)
        Some(tab)
            if own_preview(app, Some(tab)).is_some()
                && preview_shown(app, tab, window.is_some()) =>
        {
            let mut hints = Vec::new();
            if esc_mine {
                hints.push(Hint::keys(["Esc"], "Cancel"));
            }
            hints.extend(
                MODEL_VIEW_HINTS
                    .iter()
                    .map(|(keys, words)| Hint::keys(keys.iter().copied(), *words)),
            );
            Shown { hints, ..Shown::default() }
        }
        Some(tab) if tab.escape_cancels() => {
            let mut hints = Vec::new();
            if *tab == Tab::Options && options_hovered && enter_is_options_ok(app, ctx) {
                hints.push(Hint::keys(["Enter"], "OK"));
            }
            if esc_mine {
                hints.push(Hint::keys(["Esc"], "Cancel"));
            }
            hints.push(Hint::keys(["Tab"], "Next field"));
            hints.push(Hint::keys(["Shift", "Tab"], "Previous field"));
            Shown { hints, ..Shown::default() }
        }
        _ => Shown::default(),
    };
    if let Some((_, name)) = escape.filter(|_| !esc_mine) {
        shown.hints.insert(0, Hint::keys(["Esc"], format!("Cancel {name}")));
    }
    // Options in a window of its own over another editor (M4's U23): its
    // Enter OK first where the pointer is over it, as docked.
    if active != Some(&Tab::Options) && options_hovered && enter_is_options_ok(app, ctx) {
        shown.hints.insert(0, Hint::keys(["Enter"], "OK"));
    }
    // A list of the Bottom Dock with the keys (M4's U28 review): its hints in
    // the editor's place (the area view's keys are not its own meanwhile),
    // the editor's counts kept.
    if let Some(hints) = crate::dock_reports::hints(app, ctx, &k) {
        shown.hints = hints;
    }
    // (The area editor's tools with the keys, M4's U29: theirs.)
    if let Some(hints) = crate::tool_sections::hints(app, ctx) {
        shown.hints = hints;
    }
    // The Resources panel in use: its hints (the Resources board's), the
    // editor's counts kept.
    if let Some(over) = app.resources_here {
        let keys = crate::palette_view::keys_are(ctx, crate::palette_view::Keys::Browser);
        let mut hints = Vec::new();
        if keys {
            hints.push(Hint::either(["Up", "Down"], "Move in the list"));
            hints.push(Hint::keys(["Enter"], "Open"));
        }
        hints.push(Hint::keys(["Double Click"], "Open"));
        hints.push(Hint::keys(["Right Click"], "More"));
        if over {
            hints.extend(k.hint(Cmd::Find, "Find"));
        }
        shown.hints = hints;
        // (Its count first, the editor's after it: the board's.)
        shown.counts.insert(0, Count::new(app.browser.count_text()));
    }
    shown
}

/// Whether an Enter would be Options' OK (with the pointer over it): not
/// while a field is typed in or a dropdown is open (they have the
/// keyboard), Options › Keyboard waits for a key (it takes the keys
/// itself), or a list drawn before the editor area has the keys (the
/// module tree takes Enter first). The one rule for the hint and for OK
/// itself (`options::ui`, M4's U23 fix): `recording`, Options › Keyboard
/// waits for a key.
pub(crate) fn enter_is_ok(recording: bool, ctx: &egui::Context) -> bool {
    let typing = ctx.memory(|m| m.focused().is_some());
    !typing && !recording && !crate::palette_view::a_list_has_the_keys(ctx)
}

fn enter_is_options_ok(app: &Moonglow, ctx: &egui::Context) -> bool {
    enter_is_ok(app.options.as_ref().is_some_and(|o| o.recording.is_some()), ctx)
}

/// What the Status Bar shows beside a conversation (M4's U15, the
/// Conversation board's): the tree's ways with the pointer, Drag Move
/// line, Ctrl + Drag Link (Cmd on macOS) and Double Click Go to linked;
/// the keys its commands take where they act (the pointer over the editor
/// or its tree with the keys, nothing typed in, as the editor was drawn
/// last): Add's and Find's as bound (none where unbound), Alt + Up Move
/// up (Option on macOS: a fixed key, no command of its own), and `*` Open
/// branch while the tree has the keys, Find's key, and last Toggle
/// Bookmark's (M5's W4) where Find's shows and a line is chosen; and its
/// words as the file counts
/// them. Where it is only looked at, nothing that would change it. None
/// while a modal, a dialog or a menu is in front, as beside an area.
fn dialog_shown(
    app: &Moonglow,
    k: &HintKeys<'_>,
    ctx: &egui::Context,
    key: mg_resman::ResKey,
) -> Shown {
    use crate::keys::Cmd;
    let (here, tree) = app.dialog_views.get(&key).map_or((false, false), |v| v.keys_took);
    let edits = !crate::dialog_view::looked_at(app, &key);
    let mut hints = Vec::new();
    if !keys_blocked(ctx) {
        if edits {
            hints.push(Hint::keys(["Drag"], "Move line"));
            hints.push(Hint::keys([k.ctrl(), "Drag"], "Link"));
        }
        hints.push(Hint::keys(["Double Click"], "Go to linked"));
        if here && edits {
            hints.extend(k.hint(Cmd::AddLine, "Add"));
            hints.push(Hint::keys([k.alt(), "Up"], "Move up"));
        }
        if tree {
            hints.push(Hint::keys(["*"], "Open branch"));
        }
        if here {
            hints.extend(k.hint(Cmd::DialogFind, "Find"));
        }
        // (Last: F5 Bookmark, where it works: as Find's, and with a line
        // chosen. M5's W4.)
        let chosen = app.dialog_views.get(&key).is_some_and(|v| v.selected.is_some());
        if here && chosen {
            hints.extend(k.hint(Cmd::DialogBookmark, "Bookmark"));
        }
    }
    let counts = crate::dialog_view::file_words(app, key)
        .map(|n| Count::new(if n == 1 { "1 word".to_string() } else { format!("{n} words") }))
        .into_iter()
        .collect();
    Shown { hints, counts, note: None }
}

/// The widths at most of the Status Bar's module name and its note (the
/// game folder left out), `name` and `note` wide as they are, in `room`:
/// both whole where they fit; else the one narrower than half the room
/// whole and the other cut to what it leaves; else each cut to half.
fn share(room: f32, name: f32, note: f32) -> (f32, f32) {
    let room = room.max(0.0);
    let half = room / 2.0;
    if name + note <= room {
        (name, note)
    } else if note <= half {
        (room - note, note)
    } else if name <= half {
        (name, room - name)
    } else {
        (half, half)
    }
}

/// A module's name as the Status Bar says it: its file's name without the
/// extension (`Chapter1`), a folder's name as it is.
fn module_name(path: &std::path::Path) -> String {
    let is_file = path.extension().is_some_and(|e| {
        let e = e.to_string_lossy().to_ascii_lowercase();
        matches!(e.as_str(), "mod" | "nwm" | "erf" | "sav")
    });
    let name = if is_file { path.file_stem() } else { path.file_name() };
    name.map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// The size of a dock as the builder chose it (`kept`, one of
/// `Settings::left_dock_width`, `right_dock_width` and
/// `bottom_dock_height`), else `default`, the size the window's layout
/// gives it (`Yields`): nothing is stored until an edge is dragged, so
/// that a dock not chosen takes the wide layout's size in a wide window
/// and the usual one in another.
pub(crate) fn chosen_size(kept: Option<u32>, default: f32) -> f32 {
    kept.map_or(default, |s| s as f32)
}

/// Each dock's legacy default sizes, `(tree, Right Dock, log)`, that
/// older versions stored as if user-chosen before edge drags were tracked:
/// the module tree's `260` or `240` (legacy 4.2.0 default), the Right
/// Dock's `320`, and the log's `180` or `120`. Because each dock wrote its
/// size on first draw, an untouched settings file may hold any mix of
/// these values, or only a subset if another dock stayed folded or closed.
const SIZES_AT_FIRST: [&[u32]; 3] = [&[260, 240], &[320], &[180, 120]];

/// Once for good (`Settings::dock_sizes_taken`), on the first frame: the
/// size an older version's egui panel kept for each dock taken as chosen
/// where none is (4.2.0 and older kept the module tree's width and the
/// log's height so); then, where every size kept is one of its dock's
/// sizes at first ([`SIZES_AT_FIRST`], in any mix, or one alone), none of
/// it is taken as chosen. A file with any size other than those keeps
/// them all, as dragged to (a builder who dragged one may have dragged
/// another to a size at first); what can still be lost is a builder's drag
/// of every dock kept to exactly a size at first. Then a size is chosen
/// only as an edge is dragged; what a narrow window draws is never taken.
pub(crate) fn take_dock_sizes(app: &mut Moonglow, ctx: &egui::Context) {
    if app.settings.dock_sizes_taken {
        return;
    }
    app.settings.dock_sizes_taken = true;
    let s = &mut app.settings;
    for (kept, id, axis) in [
        (&mut s.left_dock_width, "tree", 0),
        (&mut s.right_dock_width, RIGHT_DOCK_ID, 0),
        (&mut s.bottom_dock_height, "log", 1),
    ] {
        if kept.is_none() {
            let was = egui::containers::panel::PanelState::load(ctx, egui::Id::new(id));
            *kept = was.map(|s| s.size()[axis].round().max(0.0) as u32);
        }
    }
    let kept = [s.left_dock_width, s.right_dock_width, s.bottom_dock_height];
    let at_first = kept.iter().any(Option::is_some)
        && kept.iter().zip(SIZES_AT_FIRST).all(|(k, at)| k.is_none_or(|v| at.contains(&v)));
    if at_first {
        (s.left_dock_width, s.right_dock_width, s.bottom_dock_height) = (None, None, None);
    }
}

/// Has egui draw the panel `id` `size` points along `axis` at this frame
/// (the chosen size held to what the window allows): egui draws a panel
/// at the size it kept for it, and keeps the size it drew, so a window
/// made smaller would otherwise leave the dock as small as it was drawn
/// there, for good.
fn draw_at(ctx: &egui::Context, id: &str, axis: usize, size: f32) {
    size_panel(ctx, egui::Id::new(id), axis, size);
}

/// [`draw_at`] for the panel of id `id`.
fn size_panel(ctx: &egui::Context, id: egui::Id, axis: usize, size: f32) {
    use egui::containers::panel::PanelState;
    let mut state = PanelState::load(ctx, id).unwrap_or(PanelState {
        outer_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(size)),
    });
    if (state.size()[axis] - size).abs() > 0.01 {
        // (Where it was: only its size is read.)
        let r = &mut state.outer_rect;
        if axis == 0 {
            r.max.x = r.min.x + size
        } else {
            r.max.y = r.min.y + size
        }
        ctx.data_mut(|d| d.insert_persisted(id, state));
    }
}

/// Says of a dock's panel (`ui`, the one its contents are laid out in)
/// that it is a pane, for a screen reader: everything the dock draws is
/// under it (tests find a dock's widgets so, wherever they are drawn).
fn dock_pane(ui: &Ui) {
    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
        node.set_role(egui::accesskit::Role::Pane);
    });
}

/// Whether the edge of the panel `id` is being dragged, or was let go of,
/// at this frame (egui then draws the panel as large as the pointer asks,
/// within its range): read before the panel is drawn, as egui reads it.
/// (egui's resize handle is the panel's id with `"__resize"`.)
fn edge_dragged(ctx: &egui::Context, id: &str) -> bool {
    let handle = egui::Id::new(id).with("__resize");
    ctx.read_response(handle).is_some_and(|r| r.dragged() || r.drag_stopped())
}

/// The Left Dock (`Panel::left("tree")`), with a module open: its tab bar
/// (the Module tab; the Module panel's Module Properties, New Area, Expand
/// All and Collapse All, and the fold button at its right), the Module
/// panel under it (`tree::module_tree`); where the active editor brings it
/// ([`editor_brings`]), the Outliner under them, split by a seam
/// the builder drags (`Settings::module_panel_height`, the upper panel's
/// height as chosen, drawn held to what the dock allows). Folded
/// (`Settings::hide_tree`), the strip of its panels' icons (the
/// Outliner's after its tabs') under the chevron that brings it back.
pub(crate) fn left_dock(app: &mut Moonglow, ui: &mut Ui) {
    app.left_rect = None;
    app.strips[0] = None;
    let held = panels(app, Dock::Left);
    let front = left_shown(app);
    if front.is_none() && !outliner_alone(app) {
        return;
    }
    let outliner = Panel::OUTLINER.available(app);
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let line = panel_line(ui);
    let fold_key = key_note(app, &ctx, Id::ViewTree);
    if left_folded(app) {
        // (The strip with its chevron at its top, `Narrow.png`'s: the one
        // control that brings the dock back as it was.)
        let unfold_tip = fold_tip(app, &ctx, Side::Left, false);
        let shown = egui::Panel::left("tree-strip")
            .exact_size(kit::STRIP + line)
            .resizable(false)
            .frame(egui::Frame::new().fill(t.bar))
            .show(ui, |ui| {
                let mut strip = FoldStrip::new(LEFT_DOCK, Side::Left).unfold_tip(unfold_tip);
                for p in &held {
                    // (The key unfolds the dock as it was, on the panel shown.)
                    let key = if Some(*p) == front { fold_key.as_str() } else { "" };
                    strip = strip.panel(p.icon(), p.name(), format!("{}{key}", p.strip_tip()));
                }
                // (And the Outliner's, under its tabs' as the dock has it:
                // `Narrow.png`'s third icon.)
                if outliner {
                    let p = Panel::OUTLINER;
                    strip = strip.panel(p.icon(), p.name(), p.strip_tip());
                }
                strip.show(ui)
            });
        // (Where a floating editor first opens clear of it, `Moonglow::strips`.)
        app.strips[0] = Some(shown.response.rect);
        match shown.inner {
            Some(StripClick::Panel(i)) if i < held.len() => app.show_dock_panel(held[i]),
            // (The Outliner's: the dock unfolded, the keys the Outliner's,
            // the tab shown as it was.)
            Some(StripClick::Panel(_)) => {
                app.show_panel(Id::ViewTree, true);
                crate::outliner::take_keys(app, &ctx);
            }
            Some(StripClick::Unfold) => app.show_panel(Id::ViewTree, true),
            None => {}
        }
        return;
    }
    let Some(front) = front else {
        outliner_alone_ui(app, ui);
        return;
    };
    // (The arrow keys go with the tab: from the tree to the browser's list
    // and back, where they were the panel's that went.)
    {
        use crate::palette_view::{give_keys, keys_are};
        let keys = Panel::keys;
        if let Some(was) = app.left_drawn.replace(front)
            && was != front
            && keys_are(&ctx, keys(was))
        {
            give_keys(&ctx, keys(front));
            // (At the row they were at, else the first: as F6 gives them.)
            match front {
                Panel::Module if app.tree_cursor.is_none() => {
                    app.tree_cursor = Some(tree::first_row());
                    app.tree_cursor_moved = true;
                }
                Panel::Resources if app.browser.cursor.is_none() => app.browser.go_to(0),
                _ => {}
            }
        }
    }
    let tools: Vec<Prepared> = MODULE_TOOLS.iter().map(|b| Prepared::of(app, &ctx, *b)).collect();
    // (The Resources panel's header reads the browser's list first: how
    // many Export as Files would write.)
    let browser = (front == Panel::Resources).then(|| crate::browser::header(app));
    // The tools' width the tabs are laid out beside: the widest of the
    // panels' the dock holds (the Module panel's four buttons), whichever
    // is shown, so that the tabs keep their form (their words; their
    // words closer to their sides where only so they fit beside those
    // tools; their icons alone where not even so) and their place when
    // another tab is chosen: the dock's width alone decides it.
    // (And the fold button after them, at the bar's end toward the
    // editor area.)
    let button = mg_uikit::IconButton::new(Icon::Plus, "").icon_only(true).width(ui);
    let most = held.iter().map(|p| p.tools()).max().unwrap_or(0);
    let tools_width = button * (most + 1) as f32;
    let fold = fold_tip(app, &ctx, Side::Left, true);
    let (range, dragged) = left_width(app, &ctx);
    let upper =
        Upper { held: &held, front, tools: &tools, tools_width, browser: &browser, fold: &fold };
    let shown = egui::Panel::left("tree")
        .resizable(true)
        .default_size(app.yields.left_dock_width)
        .size_range(range)
        .frame(egui::Frame::new().fill(t.panel))
        .show(ui, |ui| {
            dock_pane(ui);
            // Split: the tabs' panel over the Outliner, at the seam as
            // chosen (the board's at first), held so that each keeps its
            // least where the dock is short; the seam chosen changes only
            // as it is dragged, and is kept for a taller dock. Without
            // the Outliner, the same panel the whole height: its widgets
            // keep their ids (the tree's open groups, where it is
            // scrolled to) as the Outliner comes and goes.
            let whole = ui.max_rect().height();
            // (A dock too short for both at their least: the Outliner
            // folds to its header first, the panel over it keeping at
            // least its own least, and the seam rests.)
            let folded = outliner && whole - MODULE_PANEL_LEAST < OUTLINER_LEAST;
            let most = (whole - OUTLINER_LEAST).max(MODULE_PANEL_LEAST);
            let range = MODULE_PANEL_LEAST..=most;
            // (The board's seam where none is chosen: 496, 760 in a wide
            // window, GalleryWide's.)
            // (Beside a conversation, the Conversation board's: the
            // Outliner 166 high under it, its bookmarks a short list.)
            // (Each kept apart, the areas' and a conversation's bookmarks':
            // the coordinator's ruling at U15's review.)
            // (Beside a table, the Table board's: the panel over the
            // Outliner 254 high, its columns the rest; kept apart too,
            // M4's U16.)
            let listed = crate::outliner::contents(app);
            let (at_first, kept) = match listed {
                Some(crate::outliner::Contents::Bookmarks(_)) => {
                    (whole - OUTLINER_BOOKMARKS_HIGH, app.settings.bookmarks_panel_height)
                }
                Some(crate::outliner::Contents::Columns(_)) => {
                    (OVER_COLUMNS_HIGH, app.settings.columns_panel_height)
                }
                // (Beside a NUI designer, the NUI board's: the panel over
                // its Layers 416 high; kept apart, M4's U18.)
                Some(crate::outliner::Contents::Layers(_)) => {
                    (OVER_LAYERS_HIGH, app.settings.layers_panel_height)
                }
                // (Beside a tileset, the Tileset board's: the panel over its
                // parts 394 high; kept apart, M4's U20.)
                Some(crate::outliner::Contents::Tileset(_)) => {
                    (OVER_PARTS_HIGH, app.settings.parts_panel_height)
                }
                // (Beside a hak, the Hak board's: the panel over its types
                // 412 high; kept apart, M4's U21.)
                Some(crate::outliner::Contents::Hak(_)) => {
                    (OVER_TYPES_HIGH, app.settings.types_panel_height)
                }
                // (Beside the faction editor, the Factions board's: the
                // panel over its factions 452 high; kept apart, M4's U22.)
                Some(crate::outliner::Contents::Factions) => {
                    (OVER_FACTIONS_HIGH, app.settings.factions_panel_height)
                }
                _ => (app.yields.module_panel_height, app.settings.module_panel_height),
            };
            let chosen = kept.map_or(at_first, |h| h as f32);
            let tall = match (outliner, folded) {
                (true, false) => chosen.clamp(*range.start(), *range.end()),
                (true, true) => (whole - metrics::DOCK_TABS).max(0.0),
                (false, _) => whole,
            };
            draw_at(&ctx, MODULE_PANEL_ID, 1, tall);
            let resizable = outliner && !folded;
            let seam_dragged = resizable && edge_dragged(&ctx, MODULE_PANEL_ID);
            let mut top = egui::Panel::top(MODULE_PANEL_ID)
                .resizable(resizable)
                .show_separator_line(false)
                .frame(egui::Frame::new());
            top = match resizable {
                true => top.default_size(at_first).size_range(range),
                false => top.exact_size(tall),
            };
            let top = top.show(ui, |ui| {
                let pane = ui.max_rect();
                upper_panel(app, ui, pane, &upper)
            });
            if seam_dragged {
                let high = Some(top.response.rect.height().round() as u32);
                match listed {
                    Some(crate::outliner::Contents::Bookmarks(_)) => {
                        app.settings.bookmarks_panel_height = high;
                    }
                    Some(crate::outliner::Contents::Columns(_)) => {
                        app.settings.columns_panel_height = high;
                    }
                    Some(crate::outliner::Contents::Layers(_)) => {
                        app.settings.layers_panel_height = high;
                    }
                    Some(crate::outliner::Contents::Tileset(_)) => {
                        app.settings.parts_panel_height = high;
                    }
                    Some(crate::outliner::Contents::Hak(_)) => {
                        app.settings.types_panel_height = high;
                    }
                    Some(crate::outliner::Contents::Factions) => {
                        app.settings.factions_panel_height = high;
                    }
                    _ => app.settings.module_panel_height = high,
                }
            }
            if outliner {
                crate::outliner::ui(app, ui);
            }
            top.inner
        });
    app.tree_width = Some(shown.response.rect.width());
    app.left_rect = Some(shown.response.rect);
    if dragged {
        app.settings.left_dock_width = Some(shown.response.rect.width().round() as u32);
    }
    let ((((clicked, _), browser_asked), folded), chosen) = shown.inner;
    if folded {
        app.show_panel(Id::ViewTree, false);
    }
    if let Some(id) = clicked {
        id.run(app, &ctx);
    }
    if let Some(asked) = browser_asked {
        crate::browser::run_tools(app, asked);
    }
    if let Some(panel) = chosen {
        app.show_dock_panel(panel);
    }
}

/// The Left Dock's widths: drawn at the width chosen (else the layout's:
/// 260, 320 in a wide window), held to what the window allows: at most two
/// fifths of it, and the editor area left its least beside the Right Dock
/// (where the dock is not unfolded by hand in a window too narrow for
/// that); the width chosen changes only as the edge is dragged. The range
/// and whether its edge is dragged.
fn left_width(app: &Moonglow, ctx: &egui::Context) -> (std::ops::RangeInclusive<f32>, bool) {
    let window = ctx.content_rect().size();
    let beside = right_reserved(app, window.x) + kit::EDITOR_LEAST;
    let most = (window.x * 0.4).min(window.x - beside).max(LEFT_DOCK_LEAST);
    let range = LEFT_DOCK_LEAST..=most;
    let wide = chosen_size(app.settings.left_dock_width, app.yields.left_dock_width);
    draw_at(ctx, "tree", 0, wide.clamp(*range.start(), *range.end()));
    (range, edge_dragged(ctx, "tree"))
}

/// The Left Dock holding the Outliner alone ([`outliner_alone`]), unfolded:
/// the Outliner the whole height, at the dock's width (the same panel, so
/// the width is kept as a module opens and closes), the fold button at the
/// end of its header (`outliner::alone`).
fn outliner_alone_ui(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let (range, dragged) = left_width(app, &ctx);
    let fold = fold_tip(app, &ctx, Side::Left, true);
    let shown = egui::Panel::left("tree")
        .resizable(true)
        .default_size(app.yields.left_dock_width)
        .size_range(range)
        .frame(egui::Frame::new().fill(t.panel))
        .show(ui, |ui| {
            dock_pane(ui);
            crate::outliner::alone(app, ui, &fold)
        });
    app.tree_width = Some(shown.response.rect.width());
    app.left_rect = Some(shown.response.rect);
    if dragged {
        app.settings.left_dock_width = Some(shown.response.rect.width().round() as u32);
    }
    if shown.inner {
        app.show_panel(Id::ViewTree, false);
    }
}

/// What the Left Dock's upper panel is drawn with ([`upper_panel`]): the
/// tabs' panels, the one shown, the Module panel's header buttons, the
/// tools' width the tabs are laid out beside, and the Resources panel's
/// header where it is shown.
struct Upper<'a> {
    held: &'a [Panel],
    front: Panel,
    tools: &'a [Prepared],
    tools_width: f32,
    browser: &'a Option<crate::browser::Header>,
    /// The fold button's tip (its name and keys, what it does here).
    fold: &'a str,
}

/// What the Left Dock's tab bar asked: the Module panel's header buttons'
/// (a command, Expand All or Collapse All), the Resources panel's, and a
/// tab chosen.
type UpperAsked = (((ModuleAsked, Option<crate::browser::Asked>), bool), Option<Panel>);

/// The Left Dock's upper panel (the whole dock without an Outliner): its
/// tab bar and the panel shown under it, in `pane` (the panel's whole
/// rectangle, its header included).
fn upper_panel(app: &mut Moonglow, ui: &mut Ui, pane: egui::Rect, u: &Upper<'_>) -> UpperAsked {
    let mut tab = u.held.iter().position(|p| *p == u.front).unwrap_or(0);
    let keymap = &app.keymap;
    let mut tabs = DockTabs::new(LEFT_DOCK).tools_width(u.tools_width);
    for p in u.held {
        tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
    }
    // (The tools are the panel's shown: the Module panel's four, or the
    // Resources panel's Export and Refresh; then the dock's fold button,
    // at the bar's end toward the editor area, inside the dock: no board
    // draws one for the open dock, and beside the edge it was over the
    // editor area.)
    let bar = tabs.show(ui, &mut tab, |ui| {
        let tools = match u.browser {
            None => (module_tools(ui, u.tools, keymap), None),
            Some(header) => ((None, None), Some(crate::browser::tools(ui, header))),
        };
        (tools, kit::fold_button_tip(ui, Side::Left, LEFT_DOCK, u.fold).clicked())
    });
    // (A tab chosen shows its panel from the next frame: this one draws the
    // panel the tools were made for.)
    let chosen = bar.changed.then(|| u.held[tab]);
    let (((_, folded), _), _) = bar.inner;
    match u.front {
        Panel::Module => {
            // (The tree's rows reach the panel's sides, as the mock's; its
            // Filter has a margin of its own.)
            let margin = Margin { left: 0, right: 0, top: 2, bottom: 2 };
            let body = egui::Frame::new().inner_margin(margin);
            body.show(ui, |ui| tree::module_tree(app, ui, folded));
        }
        Panel::Resources => crate::browser::ui(app, ui, pane),
        // (Not the Left Dock's tabs'.)
        _ => {}
    }
    (bar.inner, chosen)
}

/// The tip of a side dock's fold control (open: the button at its tab
/// bar's end; folded: the chevron at its strip's top): what it does
/// (`dock_tip` where a narrow window has a say), with View's key for it.
fn fold_tip(app: &Moonglow, ctx: &egui::Context, side: Side, open: bool) -> String {
    let (dock, view) = match side {
        Side::Left => (LEFT_DOCK, Id::ViewTree),
        Side::Right => (RIGHT_DOCK, Id::ViewPalettes),
    };
    let key = key_note(app, ctx, view);
    match dock_tip(app, side) {
        Some(tip) => format!("{tip}{key}"),
        None if open => format!("Fold the {dock}{key}"),
        None => format!("Show the {dock}{key}"),
    }
}

/// The Right Dock (`Panel::right(RIGHT_DOCK_ID)`), where the active
/// docked editor brings sections of panels (an area's tab or the
/// placeholder in front: the Palette and the Preview, one section) or the
/// Palette was asked for by name ([`right_sections`]); not drawn at all
/// where it holds none (no strip, no fold button: its fold and its width
/// kept for the next area). Its sections as [`arrange`] stands them: one
/// tab bar (the kit's `DockTabs`: Palette and Preview, as the Main board
/// draws them; their icons alone where the dock is too narrow for their
/// words, by its width alone, as the Left Dock's) and under it the panel
/// in front, filling the dock; or, stacked, each section its own bar over
/// its share of the height, split by seams the builder drags
/// (`Settings::dock_layouts`, by the editor's kind); or in the wide
/// layout's two columns. A panel's body is [`panel_body`]: the Palette
/// (`palette_view::ui`) or the Preview (drawn after the editor area where
/// a tab may show its model, in the room kept for it here: [`LateSlot`]).
/// 320 px at first, at least [`RIGHT_DOCK_LEAST`], at most two fifths of
/// the window (as the Left Dock), resized by its inner edge; its fold
/// button at its top bar's end by the window's edge (in two columns, the
/// last column's). Folded (`Settings::hide_palettes`, or by the window's
/// width), the strip: the chevron that unfolds it at its top, then its
/// panels' icons, each unfolding it on its panel. A press in the dock
/// makes the panel's editor the plugins' "pane last clicked in": the
/// palette's (`plugins::Pane::Palette`) for the Palette and the Preview,
/// as a press in the palettes' tab did ([`plugin_pane`]).
pub(crate) fn right_dock(app: &mut Moonglow, ui: &mut Ui) {
    follow_editor(app);
    crate::dialog_view::search_forward(app);
    // (Where it and the panels' late bodies are drawn, this frame's alone:
    // a dock that folds or goes leaves none behind.)
    app.right_rect = None;
    ui.ctx().data_mut(|d| d.remove::<RightEdge>(egui::Id::new(RIGHT_EDGE)));
    app.strips[1] = None;
    app.preview_rect = None;
    app.late_slots.clear();
    app.right_columns = false;
    app.props_full_height = false;
    app.right_bars.clear();
    app.right_places.clear();
    let sections = right_sections(app);
    let folded_as = editor_brings(app).folded;
    let held = folded_order(&sections, folded_as);
    let Some(front) = right_shown(app) else {
        app.right_dock_drag = None;
        return;
    };
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let line = panel_line(ui);
    if right_folded(app) {
        app.right_dock_drag = None;
        // (Each panel's button with the key that shows it: View › Right
        // Dock's for the one in front, which unfolds the dock as it was;
        // Tools › Palettes' and Tools › Preview's.)
        let keys: Vec<String> = (held.iter())
            .map(|p| match *p {
                p if p == front => key_note(app, &ctx, Id::ViewPalettes),
                p => p.command().map_or_else(String::new, |id| key_note(app, &ctx, id)),
            })
            .collect();
        // (Its chevron at the strip's top, as the Left Dock's.)
        let unfold_tip = fold_tip(app, &ctx, Side::Right, false);
        let shown = egui::Panel::right("right-dock-strip")
            .exact_size(kit::STRIP + line)
            .resizable(false)
            .frame(egui::Frame::new().fill(t.bar))
            .show(ui, |ui| {
                let mut strip = FoldStrip::new(RIGHT_DOCK, Side::Right).unfold_tip(unfold_tip);
                for (p, key) in held.iter().zip(&keys) {
                    strip = strip.panel(p.icon(), p.name(), format!("{}{key}", p.strip_tip()));
                }
                strip.show(ui)
            });
        app.strips[1] = Some(shown.response.rect);
        match shown.inner {
            Some(StripClick::Panel(i)) => app.show_dock_panel(held[i]),
            Some(StripClick::Unfold) => app.show_panel(Id::ViewPalettes, true),
            None => {}
        }
        return;
    }
    let window = ctx.content_rect().size();
    // (The tools' width the tabs are laid out beside: the widest of the
    // panels' it holds (none has any yet) and the dock's fold button after
    // them, at the bar's end, inside the dock: no board draws a fold
    // control on the open dock, and beside its edge it was over the
    // editor area.)
    let tools_width = {
        let button = IconButton::new(Icon::Plus, "").icon_only(true).width(ui);
        button * (held.iter().map(|p| p.tools()).max().unwrap_or(0) + 1) as f32
    };
    let fold = fold_tip(app, &ctx, Side::Right, true);
    // (Never past the window's edge, and the editor area left its least
    // where the window has it: a dock unfolded by hand in a window too
    // narrow for that leaves the editor area less, never nothing.)
    let room = ui.available_rect_before_wrap().width().max(0.0);
    let least = RIGHT_DOCK_LEAST.min(room);
    let most = (window.x * 0.4).min(room - kit::EDITOR_LEAST).max(least);
    let range = least..=most.min(room.max(least));
    // (Drawn at the width chosen, else the layout's (320, 741 in a wide
    // window), held to what the window allows, as the Left Dock; in a
    // window under 1400 at its narrow 280, whatever was chosen, which is
    // kept for a wider window: its edge is not dragged there.)
    let narrow = app.yields.right_dock_narrow;
    let wide = chosen_size(kept_right_width(app), right_layout_width(app));
    let drawn = if narrow { least } else { wide.clamp(*range.start(), *range.end()) };
    draw_at(&ctx, RIGHT_DOCK_ID, 0, drawn);
    let resizable = !narrow;
    let edge_range = (*range.start(), *range.end());
    let dragged = resizable && edge_dragged(&ctx, RIGHT_DOCK_ID);
    let mut panel = egui::Panel::right(RIGHT_DOCK_ID)
        .resizable(resizable)
        .frame(egui::Frame::new().fill(t.panel));
    panel = match narrow {
        true => panel.exact_size(drawn),
        false => panel.default_size(drawn).size_range(range),
    };
    let yields = app.yields;
    // (The editor its panels draw for: the active docked one.)
    let editor = app.active_editor().cloned();
    let custom_area = customizable_area(app, editor.as_ref());
    let area_menu = custom_area.map(|area| {
        let props_shown = area_properties_shown(app, area);
        let editable = wide_editable(app, area);
        let can_show_props = (app.settings.hide_properties || app.props_popped_out.contains(&area))
            && (!app.yields.wide || editable);
        AreaRightMenuState {
            wide: app.yields.wide,
            base: active_area_right_base(app),
            props_shown,
            can_show_props,
        }
    });
    let shown = panel.show(ui, |ui| {
        // (Where it is drawn, from now on: the Gallery's enlargement stands
        // beside it.)
        app.right_rect = Some(ui.max_rect());
        dock_pane(ui);
        let pane = ui.max_rect();
        let pane_id = ui.unique_id();
        let columns = yields.right_dock_columns(pane.width());
        let layout = match custom_area {
            Some(area) => arrange_area(app, area, &sections, folded_as, narrow, columns),
            None => arrange(&sections, folded_as, narrow, columns),
        };
        app.props_full_height = app.yields.wide
            && layout.len() == 2
            && layout.iter().any(|col| col.len() == 1 && col[0] == [Panel::Properties]);
        let mut drag_start: Option<Panel> = None;
        let mut menu_cmd: Option<AreaRightCommand> = None;
        let mut hidden_props = false;
        let mut section_boxes: Vec<AreaSectionBox> = Vec::new();
        // (The pane last clicked in, whose selection a plugin's command
        // from the Plugins menu or a key runs on: the panel's editor's.)
        if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(pane) {
            app.plugins.pane = plugin_pane(app, front, app.active_editor()).or(app.plugins.pane);
        }
        // One tab bar: the panel in front under it.
        if let [one] = layout.as_slice()
            && let [bar_panels] = one.as_slice()
        {
            let front =
                if bar_panels.contains(&front) { front } else { section_front(app, bar_panels) };
            // (A form's own Preview alone, M4's U11: its header, its title
            // and the model's info line, as a stacked section of one has
            // it. So does a script's Reference, alone since Help went under
            // the text in 5.2.0: its header and "nwscript and includes", as
            // the Script board has it. Every other bar keeps its tabs and
            // the dock's name.)
            let titled = matches!(bar_panels.as_slice(), [Panel::Preview] | [Panel::Reference])
                && front.detail(app, editor.as_ref()).is_some();
            let (chosen, inner) = if titled {
                let on = (bar_panels.as_slice(), front, editor.as_ref());
                let bar = section_bar(
                    app,
                    ui,
                    RIGHT_DOCK,
                    on,
                    Some(tools_width),
                    custom_area,
                    area_menu.as_ref(),
                    |ui| kit::fold_button_tip(ui, Side::Right, RIGHT_DOCK, &fold).clicked(),
                );
                if bar.dragged.is_some() {
                    drag_start = bar.dragged;
                }
                if bar.menu_cmd.is_some() {
                    menu_cmd = bar.menu_cmd;
                }
                hidden_props |= bar.hidden;
                section_boxes.push(AreaSectionBox {
                    col: 0,
                    sec: 0,
                    rect: pane,
                    panels: bar_panels.clone(),
                    tabs: bar.tab_rects,
                });
                (None, bar.inner)
            } else {
                let mut tab = bar_panels.iter().position(|p| *p == front).unwrap_or(0);
                let mut tabs = DockTabs::new(RIGHT_DOCK)
                    .tools_width(tools_width)
                    .draggable(custom_area.is_some());
                for p in bar_panels {
                    tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
                }
                // (A conversation's Bookmark and Test, the Node section's,
                // before the fold button where its panels share the bar.)
                let node = bar_panels.contains(&Panel::NodeData);
                let pass = ui.ctx().cumulative_pass_nr();
                let enabled =
                    node && crate::dialog_view::node_tools_enabled(app, editor.as_ref(), pass);
                let string = bar_panels.contains(&Panel::String);
                let tip = crate::dialog_view::bookmark_tip(app, ui.ctx());
                let bar = tabs.show(ui, &mut tab, |ui| {
                    let clicked =
                        node.then(|| crate::dialog_view::node_tools(ui, enabled, &tip)).flatten();
                    // (A talk table's Copy the StrRef, String's, M4's U17.)
                    if string {
                        crate::talk_view::panels::copy_tool(app, ui, editor.as_ref());
                    }
                    (clicked, kit::fold_button_tip(ui, Side::Right, RIGHT_DOCK, &fold).clicked())
                });
                let tab_rects: Vec<(Panel, egui::Rect)> =
                    bar_panels.iter().copied().zip(bar.tabs.iter().map(|r| r.rect)).collect();
                attach_area_interactions(
                    custom_area,
                    area_menu.as_ref(),
                    bar_panels,
                    front,
                    &bar.response,
                    &bar.tabs,
                    (&mut drag_start, &mut menu_cmd),
                );
                section_boxes.push(AreaSectionBox {
                    col: 0,
                    sec: 0,
                    rect: pane,
                    panels: bar_panels.clone(),
                    tabs: tab_rects,
                });
                let (clicked, folded) = bar.inner;
                if let Some(tool) = clicked {
                    crate::dialog_view::node_tool(app, editor.as_ref(), tool);
                }
                // (A tab chosen shows its panel from the next frame: this
                // one draws the panel in front as the frame began.)
                (bar.changed.then(|| bar_panels[tab]), folded)
            };
            app.right_bars.push(front);
            panel_body(app, ui, (front, titled), Host::Dock, pane_id, editor.as_ref());
            if let Some(p) = drag_start {
                app.right_dock_drag = Some(p);
            }
            area_drop(app, ui, pane, custom_area.is_some() && !narrow, &section_boxes);
            if let Some(cmd) = menu_cmd {
                apply_area_right_command(app, cmd);
            }
            if hidden_props {
                app.settings.hide_properties = true;
            }
            return (chosen, None, inner);
        }
        // In the wide layout, a dock wider than two narrow ones stands in
        // two columns (WideArea's `.dock.cols`), a line between them: the
        // first (the board's 340, half the dock where that is less) and
        // the second, each its sections stacked, each section its own tab
        // bar. Else one column of stacked sections.
        let rects = match layout.len() {
            2 => {
                let first = first_column(app, pane.width());
                let (one, rest) = pane.split_left_right_at_x(pane.left() + first);
                let two = rest.with_min_x(rest.left() + 1.0);
                let line = egui::Rect::from_x_y_ranges(one.right()..=two.left(), pane.y_range());
                ui.painter().rect_filled(line, 0.0, t.line);
                vec![one, two]
            }
            _ => vec![pane],
        };
        // (A press in a section makes its panel in front the dock's, the
        // one View › Right Dock's strip and the keys come back to.)
        let (mut pressed, mut chosen, mut folded) = (None, None, false);
        let mut index = 0;
        let brings = editor_brings(app);
        let stack = Stack { kind: editor_kind(app), brings: &brings };
        for (c, (rect, column)) in rects.iter().zip(&layout).enumerate() {
            let last_column = c + 1 == layout.len();
            let places = section_rects(app, *rect, column, stack);
            for (s, (place, section)) in places.iter().copied().zip(column).enumerate() {
                let name = match (c, s) {
                    (0, 0) => RIGHT_DOCK.to_string(),
                    (1, 0) => RIGHT_COLUMN_TWO.to_string(),
                    _ => format!("{RIGHT_DOCK}, Section {}", index + 1),
                };
                let front = section_front(app, section);
                app.right_places.push((front, place));
                if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(place) {
                    pressed = Some(front);
                    app.plugins.pane =
                        plugin_pane(app, front, app.active_editor()).or(app.plugins.pane);
                }
                let builder = egui::UiBuilder::new().id_salt(&name).max_rect(place);
                let mut part = ui.new_child(builder);
                part.set_clip_rect(place.intersect(ui.clip_rect()));
                // (The dock's fold button at the outer end of the last
                // column's top bar, by the window's edge: where it is in
                // one column, and not mid-dock, where WideArea draws none.)
                let fold_here = last_column && s == 0;
                let tools = fold_here.then_some(tools_width);
                let on = (section.as_slice(), front, editor.as_ref());
                let bar = section_bar(
                    app,
                    &mut part,
                    &name,
                    on,
                    tools,
                    custom_area,
                    area_menu.as_ref(),
                    |ui| {
                        fold_here
                            && kit::fold_button_tip(ui, Side::Right, RIGHT_DOCK, &fold).clicked()
                    },
                );
                folded |= bar.inner;
                if let Some(at) = bar.chosen {
                    chosen = Some(section[at]);
                }
                if bar.opened {
                    opened_in_tab(app, editor.as_ref());
                }
                hidden_props |= bar.hidden;
                if bar.dragged.is_some() {
                    drag_start = bar.dragged;
                }
                if bar.menu_cmd.is_some() {
                    menu_cmd = bar.menu_cmd;
                }
                if let Some(tool) = bar.node {
                    crate::dialog_view::node_tool(app, editor.as_ref(), tool);
                }
                section_boxes.push(AreaSectionBox {
                    col: c,
                    sec: s,
                    rect: place,
                    panels: section.clone(),
                    tabs: bar.tab_rects,
                });
                app.right_bars.push(front);
                // (A section of one panel has its header over it.)
                let titled = section.len() == 1;
                let on = (front, titled);
                panel_body(app, &mut part, on, Host::Dock, pane_id, editor.as_ref());
                index += 1;
            }
            section_seams(app, ui, &places, column, stack, &format!("right-dock-column-{c}"));
        }
        // (The seam between the two columns, made after their widgets so
        // that it is over them: dragged, the first column's width kept for
        // the kind, each column at least the narrow dock's 280.)
        if let [one, _] = rects.as_slice() {
            column_seam(app, ui, pane, one.right());
        }
        if let Some(p) = drag_start {
            app.right_dock_drag = Some(p);
        }
        area_drop(app, ui, pane, custom_area.is_some() && !narrow, &section_boxes);
        if let Some(cmd) = menu_cmd {
            apply_area_right_command(app, cmd);
        }
        if hidden_props {
            app.settings.hide_properties = true;
        }
        ui.advance_cursor_after_rect(pane);
        app.right_columns = true;
        (chosen, pressed, folded)
    });
    let dock = shown.response.rect;
    if dragged {
        keep_right_width(app, dock.width().round() as u32);
    }
    if resizable {
        let edge = RightEdge { dock, least: edge_range.0, most: edge_range.1 };
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(RIGHT_EDGE), edge));
    }
    app.right_rect = Some(dock);
    let (chosen, pressed, folded) = shown.inner;
    if folded {
        app.show_panel(Id::ViewPalettes, false);
    }
    // (A tab clicked is a choice, kept; a press elsewhere in a section only
    // moves the dock's panel there.)
    match (chosen, pressed) {
        (Some(panel), _) => choose_right(app, panel),
        (None, Some(panel)) => focus_right(app, panel),
        (None, None) => {}
    }
}

/// Help asked to the front (`ScriptTools::help_forward`: a double click
/// on a name in a script, Go to Definition, a click in Reference's list):
/// Help is under the script's text since 5.2.0, a tab of the Bottom Dock
/// beside Compiler and Bookmarks (the owner's ruling on Jasperre's
/// report). Where the script floats, its window's bottom pane brings Help
/// to the front, for each of the window's tabs (the pane is drawn for the
/// one in front); docked, the Bottom Dock does, opened where it was closed
/// (a name asked for by hand is to be read), nothing kept in the settings
/// beyond the dock opened ([`focus_bottom`]). Asked while no script is in
/// front (Go to Definition opening one), it waits for the script's tab,
/// unless nothing is left to bring one.
fn help_forward(app: &mut Moonglow) {
    let Some(from) = app.script_tools.help_forward else { return };
    // (A script closed since: nothing.)
    let Some(at) = app.dock.find_tab(&crate::Tab::Script(from)) else {
        app.script_tools.help_forward = None;
        return;
    };
    if !at.surface.is_main() {
        app.script_tools.help_forward = None;
        let tabs: Vec<crate::Tab> = (app.dock.iter_leaves())
            .filter(|(p, _)| p.surface == at.surface)
            .flat_map(|(_, leaf)| leaf.tabs.iter().cloned())
            .collect();
        for tab in tabs {
            app.window_bottoms.insert(tab, Panel::Help);
        }
        return;
    }
    // (Asked while another editor came to the front, Go to Definition of
    // the game's opening `nwscript.nss`: kept until a script is in front
    // again.)
    if !matches!(app.active_editor(), Some(crate::Tab::Script(_))) {
        return;
    }
    app.script_tools.help_forward = None;
    if panels(app, Dock::Bottom).contains(&Panel::Help) && !bottom_shows(app, Panel::Help) {
        focus_bottom(app, Panel::Help);
        app.settings.hide_log = false;
    }
}

/// A Bottom Dock panel a script asked to the front
/// (`ScriptTools::forward`: a compile, a bookmark, a search's hits), acted
/// on as the frame begins, before the docks are drawn (the editor that
/// asked, drawn inside the dock area, cannot see the active editor): where
/// the script floats, its window's bottom pane shows the panel (whichever
/// of the window's tabs is in front); else the
/// main window's Bottom Dock does, a script's own panel only where the
/// script is the docked editor in front (a Compile All's scripts behind
/// another editor leave the dock as it is), a search's hits whatever is in
/// front. The dock open, the panel comes to the front; closed, it opens on
/// it only where asked to (`Forward::open`: a search's hits, a failed
/// compile). Nothing is kept in the settings beyond the dock opened
/// ([`focus_bottom`]).
pub(crate) fn bottom_forward(app: &mut Moonglow) {
    use crate::script_tools::InfoTab;
    // (Windows closed since keep no front.)
    if !app.window_bottoms.is_empty() || !app.window_rights.is_empty() {
        let dock = &app.dock;
        app.window_bottoms.retain(|tab, _| dock.find_tab(tab).is_some());
        app.window_rights.retain(|tab, _| dock.find_tab(tab).is_some());
    }
    nui_forward(app);
    problems_forward(app);
    // (A script's Help, a Bottom Dock panel since 5.2.0.)
    help_forward(app);
    let asks = std::mem::take(&mut app.script_tools.forward);
    if asks.is_empty() {
        return;
    }
    let panel_of = |tab: InfoTab| match tab {
        InfoTab::Compiler => Panel::Compiler,
        InfoTab::Bookmarks => Panel::Bookmarks,
        InfoTab::SearchResults => Panel::SearchResults,
    };
    // (The main window's Bottom Dock's: the last ask it takes, opening it
    // where any of the asks it takes for that panel opens it, so that a
    // script in front that failed among several saved opens Compiler.)
    let mut docked: Option<(Panel, bool)> = None;
    for asked in asks {
        let panel = panel_of(asked.tab);
        let script = asked.from.map(crate::Tab::Script);
        let at = script.as_ref().and_then(|t| app.dock.find_tab(t));
        // (Its window's pane is drawn for the tab in front there, whichever:
        // each of the window's tabs has it in front.)
        if let Some(at) = at
            && !at.surface.is_main()
        {
            let tabs: Vec<crate::Tab> = (app.dock.iter_leaves())
                .filter(|(p, _)| p.surface == at.surface)
                .flat_map(|(_, leaf)| leaf.tabs.iter().cloned())
                .collect();
            for tab in tabs {
                app.window_bottoms.insert(tab, panel);
            }
            continue;
        }
        let own = panel != Panel::SearchResults;
        if own && (script.is_none() || app.active_editor() != script.as_ref()) {
            continue;
        }
        let open = asked.open || docked.is_some_and(|(p, o)| p == panel && o);
        docked = Some((panel, open));
    }
    let Some((panel, open)) = docked else { return };
    if !panels(app, Dock::Bottom).contains(&panel) {
        return;
    }
    // (Open on the log, the dock stays on it unless asked to open: a
    // bookmark or a compile that passed do not take the log's place.)
    let on_log = bottom_shows(app, Panel::Log);
    if open || (!app.settings.hide_log && !on_log) {
        focus_bottom(app, panel);
        app.settings.hide_log = false;
    }
}

/// A source of Problems found something (M4's U20: a tileset's Check):
/// Problems to the front of the main window's Bottom Dock (Problems is the
/// main window's, a floating tileset's too), opening it where asked to
/// (`open`: anything found, as a search's hits open Search Results, U13's
/// rule for what was asked for); nothing found leaves a closed dock closed,
/// and a dock open on the log on it, as a search that found nothing does.
fn problems_forward(app: &mut Moonglow) {
    let Some(open) = app.problems.forward.take() else { return };
    if !panels(app, Dock::Bottom).contains(&Panel::Problems) {
        return;
    }
    let on_log = bottom_shows(app, Panel::Log);
    if open || (!app.settings.hide_log && !on_log) {
        focus_bottom(app, Panel::Problems);
        app.settings.hide_log = false;
    }
}

/// A NUI designer's checks found more errors (`nui_view::checks`, M4's
/// U19): its Checks to the front, opening the Bottom Dock where the
/// designer is the active docked editor (or its source group is), or its
/// window's bottom pane where it floats; a designer behind another tab or
/// window opens nothing.
fn nui_forward(app: &mut Moonglow) {
    for key in std::mem::take(&mut app.nui_checks_forward) {
        let Some(at) = app.dock.find_tab(&crate::Tab::Nui(key)) else { continue };
        if !at.surface.is_main() {
            let tabs: Vec<crate::Tab> = (app.dock.iter_leaves())
                .filter(|(p, _)| p.surface == at.surface)
                .flat_map(|(_, leaf)| leaf.tabs.iter().cloned())
                .collect();
            for tab in tabs {
                app.window_bottoms.insert(tab, Panel::Checks);
            }
            continue;
        }
        let ours = app.active_editor().is_some_and(|t| *owner_of(app, t) == crate::Tab::Nui(key));
        if ours && panels(app, Dock::Bottom).contains(&Panel::Checks) {
            focus_bottom(app, Panel::Checks);
            app.settings.hide_log = false;
        }
    }
}

/// After the frame is drawn: where a Bottom Dock panel drawn in this pass
/// showed what has changed since (`ScriptTools::made`: the script's
/// editor, drawn after it, compiled or set a bookmark; a search's window
/// found its hits), or a panel was asked to the front, the next frame is
/// asked for, once; nothing else is compared, and a panel not drawn asks
/// for nothing (the endless redraw M4's U12 review found must not come
/// back through them).
pub(crate) fn follow_bottom(app: &Moonglow, ctx: &egui::Context) {
    let t = &app.script_tools;
    let pass = ctx.cumulative_pass_nr();
    let stale = t.bottom_drawn.is_some_and(|(p, least)| p == pass && least != t.made);
    // (Problems drawn this pass before its findings changed: once more.)
    let problems = app.problems.forward.is_some()
        || app.problems.drawn.is_some_and(|(p, r)| p == pass && r != app.problems.revision());
    if stale || !t.forward.is_empty() || !app.nui_checks_forward.is_empty() || problems {
        ctx.request_repaint();
    }
}

/// What a stacked section's bar did ([`section_bar`]): the tab chosen in
/// it, if one was, and what its tools gave.
struct SectionBar<R> {
    chosen: Option<usize>,
    inner: R,
    /// The panel's own Open in Tab was clicked (Properties').
    opened: bool,
    /// The Hide Properties button was clicked (Properties').
    hidden: bool,
    /// The Node section's own tool clicked (Bookmark, Test).
    node: Option<crate::dialog_view::NodeTool>,
    /// Drag started on a tab or header (`AreaRightPanel`).
    dragged: Option<Panel>,
    /// Context action chosen from the Area Right Dock menu.
    menu_cmd: Option<AreaRightCommand>,
    /// Tab rects for drop hit-testing.
    tab_rects: Vec<(Panel, egui::Rect)>,
}

/// A stacked section's bar, named `name`, of `section` with `front` in
/// front, drawn for `editor` (the active docked one in the dock, the
/// floating one in its window): its
/// tab bar (the kit's `DockTabs`, `tools_width` the tools' width where it
/// has tools), or where it holds one panel with a detail of its own
/// ([`Panel::detail`]: Tile Properties' "6 tiles") the kit's
/// `PanelHeader`, its title and detail, named by both ("Tile Properties,
/// 6 tiles"), and before `tools` the panel's own (Properties' Open in Tab
/// and Hide Properties, at the bar's end as the Main and WideArea boards
/// draw it).
#[allow(clippy::too_many_arguments)]
fn section_bar<R>(
    app: &Moonglow,
    ui: &mut Ui,
    name: &str,
    (section, front, editor): (&[Panel], Panel, Option<&crate::Tab>),
    tools_width: Option<f32>,
    area: Option<mg_core::ResRef>,
    area_menu: Option<&AreaRightMenuState>,
    tools: impl FnOnce(&mut Ui) -> R,
) -> SectionBar<R> {
    let mut dragged = None;
    let mut menu_cmd = None;
    if let [one] = section
        && let Some(detail) = one.detail(app, editor)
    {
        // (Named with its detail, "Tile Properties, 6 tiles", as a screen
        // reader says it.)
        let said = format!("{}, {detail}", one.name());
        let header = mg_uikit::shell::PanelHeader::new(one.name())
            .detail(detail)
            .name(said)
            .draggable(area_menu.is_some());
        let mut opened = false;
        let mut hidden = false;
        let bar_resp = header.show(ui, |ui| {
            // (A NUI designer's Element Actions there, M4's U18.)
            if *one == Panel::Properties && crate::nui_view::panels::nui_of(editor).is_some() {
                crate::nui_view::panels::actions(app, ui, editor);
            } else if *one == Panel::Bindings {
                // (A NUI designer's Create Bind, the board's plus, M4's
                // U19.)
                crate::nui_view::panels::bindings_plus(ui, editor);
            } else if *one == Panel::Properties {
                opened = crate::properties_panel::open_in_tab_button(app, ui, editor);
                hidden = crate::properties_panel::hide_button(ui);
            } else if *one == Panel::Preview && crate::tileset_view::tileset_of(editor).is_some() {
                // (A tileset's Open the Model in a Tab, the Tileset
                // board's icon at its Preview's header's end: M4's U20.)
                opened = crate::tileset_view::open_model_button(app, ui, editor);
            } else if *one == Panel::Preview && crate::hak_view::hak_of(editor).is_some() {
                // (A hak's Open the Model in a Tab and Close, the Hak
                // board's icons: M4's U21.)
                opened = crate::hak_view::open_model_button(app, ui, editor);
            }
            // (A talk table's Copy the StrRef at String's header's end,
            // the Talk board's.)
            if *one == Panel::String {
                crate::talk_view::panels::copy_tool(app, ui, editor);
            }
            tools(ui)
        });
        attach_area_interactions(
            area,
            area_menu,
            section,
            *one,
            &bar_resp.response,
            &[],
            (&mut dragged, &mut menu_cmd),
        );
        return SectionBar {
            chosen: None,
            inner: bar_resp.inner,
            opened,
            hidden,
            node: None,
            dragged,
            menu_cmd,
            tab_rects: Vec::new(),
        };
    }
    let mut at = section.iter().position(|p| *p == front).unwrap_or(0);
    let mut tabs = DockTabs::new(name).draggable(area_menu.is_some());
    // (The Conversation board's "Node" before Data and Search.)
    if let Some(title) = section.first().and_then(|p| p.section_title()) {
        tabs = tabs.title(title);
    }
    for p in section {
        tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
    }
    // (The Node section's Bookmark and Test, before the dock's own tools,
    // as the Conversation board draws them at its bar's end.)
    let node = section.contains(&Panel::NodeData);
    let own = match node {
        true => crate::dialog_view::node_tools_width(ui),
        false => 0.0,
    };
    match tools_width {
        Some(width) => tabs = tabs.tools_width(width.max(own)),
        None if node => tabs = tabs.tools_width(own),
        None => {}
    }
    let pass = ui.ctx().cumulative_pass_nr();
    let enabled = node && crate::dialog_view::node_tools_enabled(app, editor, pass);
    let tip = crate::dialog_view::bookmark_tip(app, ui.ctx());
    let bar = tabs.show(ui, &mut at, |ui| {
        let clicked = node.then(|| crate::dialog_view::node_tools(ui, enabled, &tip)).flatten();
        (clicked, tools(ui))
    });
    let tab_rects: Vec<(Panel, egui::Rect)> =
        section.iter().copied().zip(bar.tabs.iter().map(|r| r.rect)).collect();
    attach_area_interactions(
        area,
        area_menu,
        section,
        front,
        &bar.response,
        &bar.tabs,
        (&mut dragged, &mut menu_cmd),
    );
    let (node, inner) = bar.inner;
    SectionBar {
        chosen: bar.changed.then_some(at),
        inner,
        opened: false,
        hidden: false,
        node,
        dragged,
        menu_cmd,
        tab_rects,
    }
}

/// A section header's open in a tab clicked ([`SectionBar::opened`]):
/// Properties' Open in Tab, or a tileset Preview's model in a tab of its own
/// (M4's U20).
fn opened_in_tab(app: &mut Moonglow, editor: Option<&crate::Tab>) {
    if crate::hak_view::hak_of(editor).is_some() {
        return crate::hak_view::open_model(app, editor);
    }
    match crate::tileset_view::tileset_of(editor) {
        Some(_) => crate::tileset_view::open_model(app, editor),
        None => crate::properties_panel::open_in_tab(app, editor),
    }
}

/// The plugins' "pane last clicked in" for a press in `panel`: a panel an
/// editor brings counts as that editor's pane (U2, `17-plugin-api.md`):
/// the Palette's and the Preview's the palette (the Preview shows the
/// palette's blueprint), another an area's (`editor`, the editor it
/// draws for: the active docked one, or the floating one in its window)
/// the area's; an editor with no pane of its own for plugins leaves it as
/// it was (`None`).
fn plugin_pane(
    app: &Moonglow,
    panel: Panel,
    editor: Option<&crate::Tab>,
) -> Option<crate::plugins::Pane> {
    // (A form's own Preview, M4's U11, is its form's: no pane of its own
    // for plugins. A Preview asked for by name beside a form without a
    // model, a store's, is the palette's.)
    // (A tileset's Preview too, its tile's diagram: M4's U20.)
    let own = panel == Panel::Preview
        && (own_preview(app, editor).is_some()
            || crate::tileset_view::tileset_of(editor).is_some()
            || crate::hak_view::hak_of(editor).is_some());
    match panel {
        Panel::Preview if own => None,
        Panel::Palette | Panel::Preview => Some(crate::plugins::Pane::Palette),
        _ => match editor {
            Some(crate::Tab::Area(area)) => Some(crate::plugins::Pane::Area(*area)),
            _ => None,
        },
    }
}

/// What the Preview shows where `editor` (the editor it draws for) brings
/// its own (a blueprint's or a placed object's form, M4's U11): the
/// form's object, the first of several. `None` where it shows the
/// palette's blueprint (beside an area, or asked for by name).
pub(crate) fn own_preview(
    app: &Moonglow,
    editor: Option<&crate::Tab>,
) -> Option<crate::model_view::Source> {
    let editor = owner_of(app, editor?);
    let brings = brings_of(app, &editor);
    let own = brings.has(Panel::Preview) && !brings.has(Panel::Palette);
    own.then(|| crate::model_view::Source::of_editor(&editor)).flatten()
}

/// The least height of a stacked section of the Right Dock: its tab bar
/// and a few rows of its panel.
pub(crate) const SECTION_LEAST: f32 = mg_uikit::stack::SECTION_LEAST;

/// Whose stacked sections they are ([`section_rects`], [`section_seams`]):
/// the editor's kind and what it brings, by which their shares are kept.
#[derive(Clone, Copy)]
struct Stack<'a> {
    kind: Option<&'static str>,
    brings: &'a Brings,
}

fn section_share_index(brings: &Brings, section: &[Panel]) -> Option<usize> {
    if brings.area.is_some() && section == [Panel::Preview] {
        return Some(2);
    }
    if brings.area.is_some()
        && section.contains(&Panel::Properties)
        && !section.contains(&Panel::Palette)
    {
        return Some(1);
    }
    own_index(brings, section)
}

impl Stack<'_> {
    /// A section's share as kept (`Settings::dock_layouts`, by its place
    /// among the kind's own: [`own_index`]), else the kind's own at first
    /// (`Brings::shares`), 1000 where it has none.
    fn share(&self, app: &Moonglow, section: &[Panel]) -> f32 {
        let layout = self.kind.and_then(|k| app.settings.dock_layouts.get(k));
        let index = section_share_index(self.brings, section);
        let kept = layout.zip(index).and_then(|(l, i)| l.shares.get(i));
        let first = index.and_then(|i| self.brings.shares.get(i));
        kept.or(first).map_or(1000.0, |s| f32::from((*s).max(1)))
    }
}

/// The places of `sections` stacked in `column`, from the top, a 1 px
/// line between each two (drawn by [`section_seams`]), by their shares of
/// the height as kept for the kind (equal where none is kept), each at
/// least [`SECTION_LEAST`] where the column has room.
fn section_rects(
    app: &Moonglow,
    column: egui::Rect,
    sections: &[Vec<Panel>],
    stack: Stack<'_>,
) -> Vec<egui::Rect> {
    let count = sections.len();
    if count <= 1 {
        return vec![column];
    }
    let shares: Vec<f32> = sections.iter().map(|s| stack.share(app, s)).collect();
    let leasts: Vec<f32> = sections.iter().map(|s| section_least(app, s)).collect();
    let room = column.height() - (count - 1) as f32;
    let heights = tools_heights(app, room, sections, &shares, &leasts)
        .unwrap_or_else(|| section_heights_with(room, &shares, &leasts));
    mg_uikit::stack::stack_rects(column, &heights)
}

/// The heights of stacked `sections` in `room` where one is the tools'
/// (M4's U29 review): the tools' section its share of the room
/// (`tool_sections::share`, two thirds at first), the others the rest by
/// their own shares, at least their least where the rest has it; Properties
/// with nothing selected only its header ([`shut`]), what its share was going
/// to the tools'. Judged by the selection alone, never by what was drawn:
/// nothing that flips from frame to frame. None: no tools' section.
fn tools_heights(
    app: &Moonglow,
    room: f32,
    sections: &[Vec<Panel>],
    shares: &[f32],
    leasts: &[f32],
) -> Option<Vec<f32>> {
    let tools = sections.iter().position(|s| crate::tool_sections::is_tools(s))?;
    let header = mg_uikit::metrics::DOCK_TABS;
    let others: Vec<usize> = (0..sections.len()).filter(|i| *i != tools).collect();
    let all: f32 = others.iter().map(|i| shares[*i]).sum::<f32>().max(f32::EPSILON);
    let rest = room * (1.0 - crate::tool_sections::share(app));
    let open: Vec<usize> = others.iter().copied().filter(|i| !shut(app, &sections[*i])).collect();
    let open_share: f32 = open.iter().map(|i| shares[*i]).sum();
    let open_room = (rest * open_share / all).max(0.0);
    let open_heights = section_heights_with(
        open_room,
        &open.iter().map(|i| shares[*i]).collect::<Vec<_>>(),
        &open.iter().map(|i| leasts[*i]).collect::<Vec<_>>(),
    );
    let mut heights = vec![0.0; sections.len()];
    for (i, h) in open.iter().zip(open_heights) {
        heights[*i] = h.round();
    }
    for i in others.iter().filter(|i| !open.contains(i)) {
        heights[*i] = header;
    }
    let used: f32 = heights.iter().sum();
    heights[tools] = (room - used).max(0.0);
    Some(heights)
}

/// Whether the section `section` is Properties with nothing selected,
/// beside an open tool: only its header is shown ([`tools_heights`]).
fn shut(app: &Moonglow, section: &[Panel]) -> bool {
    section == [Panel::Properties]
        && crate::properties_panel::nothing_selected(app, app.active_editor())
}

/// The lines between stacked sections at `rects` (from [`section_rects`]),
/// each a seam the builder drags (3 points either side of it; made after
/// the sections' own widgets, so that it is over them): a drag keeps the
/// two sections' shares for the kind, where they are its own.
fn section_seams(
    app: &mut Moonglow,
    ui: &Ui,
    rects: &[egui::Rect],
    sections: &[Vec<Panel>],
    stack: Stack<'_>,
    salt: &str,
) {
    // (Not under a Properties shut beside a tool: its header alone is
    // shown while nothing is selected.)
    let fixed =
        |i: usize| crate::tool_sections::is_tools(&sections[i + 1]) && shut(app, &sections[i]);
    let Some(drag) = mg_uikit::stack::stack_seams(ui, rects, salt, fixed) else { return };
    let (i, now) = (drag.index, drag.heights);
    // (The seam over a tool's section: its share of the column, kept for
    // the run, as the Properties sections' seams keep theirs.)
    if crate::tool_sections::is_tools(&sections[i + 1]) {
        let room: f32 = rects.iter().map(egui::Rect::height).sum();
        app.tools.share = Some(((now[i + 1] + 1.0) / room.max(1.0)).clamp(0.1, 0.95));
        return;
    }
    let Some(kind) = stack.kind else { return };
    let own: Vec<Option<usize>> =
        sections.iter().map(|s| section_share_index(stack.brings, s)).collect();
    let layout = app.settings.dock_layouts.entry(kind.to_string()).or_default();
    mg_uikit::stack::keep_shares(&mut layout.shares, stack.brings.right.len(), &now, &own);
}

/// The heights of stacked sections in `room` (the lines between them left
/// out) by their `shares`: each its share of the room, at least
/// [`SECTION_LEAST`] where the room has that for each, the last taking
/// what is left so that they fill it in whole points.
#[cfg(test)]
pub(crate) fn section_heights(room: f32, shares: &[f32]) -> Vec<f32> {
    section_heights_with(room, shares, &vec![SECTION_LEAST; shares.len()])
}

/// The least height of the stacked section holding `section`'s panels:
/// [`SECTION_LEAST`], or what its panel needs to show whole at rest (a
/// conversation's Conversation panel, its three rows at the editor's
/// text size: M4's U14 review).
/// The Faction panel's least height: its header and its three rows (the
/// Factions board's `.faction-panel`, 132).
const FACTION_LEAST: f32 = 132.0;
/// The Reaction panel's: its header, sentence, slider and scale (148).
const REACTION_LEAST: f32 = 148.0;

fn section_least(app: &Moonglow, section: &[Panel]) -> f32 {
    match section {
        [Panel::Conversation] => crate::dialog_view::conversation_least(app),
        // (The faction editor's Faction and Reaction, their rows whole: the
        // board's 132 and 148, M4's U22.)
        [Panel::Faction] => FACTION_LEAST,
        [Panel::Reaction] => REACTION_LEAST,
        _ => SECTION_LEAST,
    }
}

/// [`section_heights`] with each section's own least height (`leasts`):
/// each its share of the room, at least its least where the room has
/// that for it and those after it, the last taking what is left.
pub(crate) fn section_heights_with(room: f32, shares: &[f32], leasts: &[f32]) -> Vec<f32> {
    mg_uikit::stack::stack_heights(room, shares, leasts)
}

/// The Right Dock's width in the wide layout where it holds one panel
/// alone (a blueprint's Preview, M4's U11), until the builder drags its
/// edge: WideCreature's `--right-w` is 560, less the 11 points the
/// application's page list and margins take more than the board's, so
/// that a creature's pages beside it stand in the board's five columns of
/// 480 at 3440.
pub(crate) const RIGHT_DOCK_WIDE_ONE: f32 = 549.0;

/// Whether the Right Dock holds one panel alone (a form's Preview): a
/// lone section of two, the palettes', stands in two columns in the wide
/// layout, as WideArea's.
fn right_alone(app: &Moonglow) -> bool {
    let sections = right_sections(app);
    matches!(sections.as_slice(), [] | [_]) && sections.iter().all(|s| s.len() <= 1)
}

/// The Right Dock's width by the layout until the builder drags its edge
/// (`Yields::right_dock_width`): 320, 280 in a narrow window, 741 in the
/// wide layout for two columns, or [`RIGHT_DOCK_WIDE_ONE`] there where
/// the active editor brings one panel.
fn right_layout_width(app: &Moonglow) -> f32 {
    if app.yields.wide && area_wants_one_column(app) {
        return match app.settings.palette_gallery {
            true => mg_uikit::shell::RIGHT_COLUMN_GALLERY,
            false => mg_uikit::shell::RIGHT_DOCK_WIDE_SINGLE,
        };
    }
    match app.yields.wide && right_alone(app) {
        true => RIGHT_DOCK_WIDE_ONE,
        false if gallery_wide(app) => mg_uikit::shell::RIGHT_DOCK_GALLERY,
        false => app.yields.right_dock_width,
    }
}

/// Whether the Right Dock stands in the wide layout's two columns with the
/// palettes showing their Gallery (Options keep the switch: toggling it
/// writes no width; a width or column dragged to still wins).
fn gallery_wide(app: &Moonglow) -> bool {
    app.yields.wide
        && app.settings.palette_gallery
        && !right_alone(app)
        && !area_wants_one_column(app)
}

/// The Right Dock's width as chosen for the layout of this frame: the one
/// dragged to in the wide layout (from 2560, where it stands in two
/// columns wider than 560) apart from the one dragged to below it, so that
/// neither layout's choice is the other's (`Settings::right_dock_width`,
/// `right_dock_width_wide`, `right_dock_width_wide_one`); and in the wide
/// layout, where the active editor's dock holds one panel alone (a form's
/// Preview, M4's U11), the one dragged to for its kind
/// (`DockLayout::wide_width`), apart from the area's two columns.
fn kept_right_width(app: &Moonglow) -> Option<u32> {
    match (app.yields.wide, area_wants_one_column(app), right_alone(app)) {
        (true, true, _) => app.settings.right_dock_width_wide_one,
        (true, false, true) => (editor_kind(app).and_then(|k| app.settings.dock_layouts.get(k)))
            .and_then(|l| l.wide_width)
            .map(u32::from),
        (true, false, false) => app.settings.right_dock_width_wide,
        (false, _, _) => app.settings.right_dock_width,
    }
}

/// Keeps the Right Dock's width dragged to, `width`, where
/// [`kept_right_width`] reads it.
fn keep_right_width(app: &mut Moonglow, width: u32) {
    match (app.yields.wide, area_wants_one_column(app), right_alone(app), editor_kind(app)) {
        (true, true, _, _) => app.settings.right_dock_width_wide_one = Some(width),
        (true, false, true, Some(kind)) => {
            let layout = app.settings.dock_layouts.entry(kind.to_string()).or_default();
            layout.wide_width = Some(width.min(u32::from(u16::MAX)) as u16);
        }
        (true, false, _, _) => app.settings.right_dock_width_wide = Some(width),
        (false, _, _, _) => app.settings.right_dock_width = Some(width),
    }
}

/// The Right Dock's first column's width where it stands in two in a
/// pane `width` wide: the one dragged to for the active editor's kind
/// (`DockLayout::column`), else the layout's (the board's 340, half the
/// dock where that is less); each column at least [`RIGHT_DOCK_LEAST`]
/// (held again as the dock narrows), the line between them 1.
pub(crate) fn first_column(app: &Moonglow, width: f32) -> f32 {
    let kept = (editor_kind(app).and_then(|k| app.settings.dock_layouts.get(k)))
        .and_then(|l| l.column)
        .map(f32::from);
    let layout = match gallery_wide(app) {
        // (Held to leave the second column its least by the clamp below.)
        true => mg_uikit::shell::RIGHT_COLUMN_GALLERY,
        false => Yields::right_first_column(width),
    };
    let first = kept.unwrap_or(layout);
    let most = (width - 1.0 - RIGHT_DOCK_LEAST).max(RIGHT_DOCK_LEAST);
    first.clamp(RIGHT_DOCK_LEAST.min(most), most).round()
}

/// Where the Right Dock's edge may be dragged this frame (`right_dock`
/// sets it where the dock is drawn and resizable; `window_panes` a
/// floating editor's right pane's): the panel's rectangle and the widths
/// it may be dragged to.
#[derive(Clone, Copy)]
pub(crate) struct RightEdge {
    dock: egui::Rect,
    least: f32,
    most: f32,
}

/// The id [`RightEdge`] is kept under, for the frame.
const RIGHT_EDGE: &str = "right-dock-edge";

/// The Right Dock's edge as a seam over the editor area (M5's W8): egui's
/// panel takes a drag 3 points either side of its edge, but the editor
/// area, drawn after the dock, took the 3 points outside it wherever it
/// has a widget there (a form's scroll area beside the Preview), which
/// left 2.5 points to find. Made after the editor area, 3 points either
/// side of the edge, as the dock's other seams are: the resize cursor, the
/// edge lit in the kit's accent, and a drag keeps the width dragged
/// to (`keep_right_width`), held to the dock's range, the point taken
/// staying under the pointer.
pub(crate) fn right_edge_seam(app: &mut Moonglow, ui: &Ui) {
    let id = egui::Id::new(RIGHT_EDGE);
    let Some(edge) = ui.ctx().data(|d| d.get_temp::<RightEdge>(id)) else { return };
    if let Some(width) = edge_seam(ui, id, edge) {
        keep_right_width(app, width as u32);
    }
}

/// A floating editor's right pane's edge as a seam over the editor
/// (`window_panes` keeps where it is under `pane`, the window's tab's id,
/// [`window_edge_id`]), as [`right_edge_seam`] is the Right Dock's: drawn
/// after the editor, whose widgets beside the pane (a form's) took the 3
/// points outside egui's edge. A drag sizes the pane for the run, as its
/// own edge does.
pub(crate) fn window_edge_seam(ui: &Ui, pane: egui::Id) {
    let id = window_edge_id(pane);
    let Some(edge) = ui.ctx().data(|d| d.get_temp::<RightEdge>(id)) else { return };
    if let Some(width) = edge_seam(ui, id, edge) {
        size_panel(ui.ctx(), pane.with("window-right"), 0, width);
    }
}

/// Where a floating editor's right pane's edge is kept for the frame
/// ([`window_edge_seam`]), the window's tab's id `pane`.
pub(crate) fn window_edge_id(pane: egui::Id) -> egui::Id {
    pane.with("window-right-edge")
}

/// A panel's left edge `edge` as a seam of id `id`, 3 points either side
/// of it (the resize cursor, the edge lit in the kit's accent):
/// dragged, the width it is dragged to, held to its range, the point
/// taken staying under the pointer.
fn edge_seam(ui: &Ui, id: egui::Id, edge: RightEdge) -> Option<f32> {
    let x = edge.dock.left();
    let grip = egui::Rect::from_x_y_ranges(x - 3.0..=x + 3.0, edge.dock.y_range());
    // (A drag alone: `Sense::drag()` is focusable too, and the seam, last
    // in the window, took the keyboard's Tab and Module Properties' scroll.)
    let seam = ui.interact(grip, id.with("seam"), egui::Sense::DRAG);
    if !(seam.hovered() || seam.dragged()) {
        return None;
    }
    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    // (In the kit's accent, as the editor groups' seams light (egui_dock's
    // separators, `mg_uikit::dock`), 2 points across the edge: the hovered
    // stroke's colour was dark in Light and all but gone beside the
    // Preview's dark picture, review D.)
    let stroke = egui::Stroke::new(2.0, mg_uikit::tokens(ui.ctx()).accent);
    let painter =
        (ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Middle, id))).with_clip_rect(grip);
    painter.vline(x, edge.dock.y_range(), stroke);
    let at = seam.interact_pointer_pos()?;
    // (Where on the seam it was taken: the edge follows that point.)
    let taken = id.with("taken");
    if seam.drag_started() {
        ui.ctx().data_mut(|d| d.insert_temp(taken, at.x - x));
    }
    if !seam.dragged() {
        return None;
    }
    let offset = ui.ctx().data(|d| d.get_temp::<f32>(taken)).unwrap_or(0.0);
    ui.ctx().request_repaint();
    Some((edge.dock.right() - (at.x - offset)).clamp(edge.least, edge.most).round())
}

/// The seam between the Right Dock's two columns at `x` in `pane` (3
/// points either side of the line): a drag keeps the first column's
/// width for the active editor's kind (`DockLayout::column`).
fn column_seam(app: &mut Moonglow, ui: &Ui, pane: egui::Rect, x: f32) {
    // (A drag alone, never the keyboard's Tab: M5's W8.)
    let id = egui::Id::new("right-dock-column-seam");
    let Some(first) = mg_uikit::stack::column_seam(ui, id, pane, x, RIGHT_DOCK_LEAST) else {
        return;
    };
    let Some(kind) = editor_kind(app) else { return };
    app.settings.dock_layouts.entry(kind.to_string()).or_default().column = Some(first as u16);
}

/// The name of the Right Dock's second column's tab bar where the dock
/// stands in two (the Preview's, in the wide layout; the first keeps the
/// dock's name, [`RIGHT_DOCK`]).
pub const RIGHT_COLUMN_TWO: &str = "Right Dock, Second Column";

/// A panel's body under its tab bar (`.panel-body`: 8 px round it),
/// filling what is left of `ui`, the one function each panel's body is
/// whichever its host ([`Host`]: a dock, or the editor's own window): the
/// Palette (`palette_view::ui`) or the Preview (its body drawn after the
/// editor area where an editor's tab may show the same model, in the room
/// kept for it here: [`LateSlot`]). `pane` is the host's panel (its
/// widgets are under it for a screen reader); `editor` the editor it
/// draws for (decision 2: a body takes the editor's tab): the active
/// docked editor in a dock, the floating editor in its window. `titled`:
/// its section's header ([`section_bar`]) is over it, with its title,
/// detail and tools (where it shares a tab bar, under 1400, Properties
/// draws its detail and Open in Tab at its own top).
pub(crate) fn panel_body(
    app: &mut Moonglow,
    ui: &mut Ui,
    (panel, titled): (Panel, bool),
    host: Host,
    pane: egui::Id,
    editor: Option<&crate::Tab>,
) {
    // (Properties' rail stands flush with the panel's edges, as the boards
    // draw it; its page keeps its own room.)
    // (A blueprint's own Preview too: its header row and its view stand
    // flush with the panel's edges, the Creature board's.)
    let own = (panel == Panel::Preview).then(|| own_preview(app, editor)).flatten();
    let margin = match panel {
        // (Compiler's inset ground fills the panel, its own margin in it:
        // the Script board's `.compiler-out`.)
        // (Help's ground too, the editor's, 4.2.1's.)
        Panel::Properties | Panel::Compiler | Panel::Help => Margin::ZERO,
        // (A conversation's: their scroll's own room, its bar at the
        // panel's edge.)
        Panel::NodeData | Panel::NodeSearch | Panel::Conversation => Margin::ZERO,
        Panel::Cell | Panel::References | Panel::String | Panel::TalkFile => Margin::ZERO,
        Panel::Components | Panel::Bindings | Panel::Checks | Panel::Problems => Margin::ZERO,
        Panel::Textures | Panel::Details => Margin::ZERO,
        Panel::Entry | Panel::Category => Margin::ZERO,
        Panel::Faction | Panel::Reaction | Panel::Reactions => Margin::ZERO,
        Panel::Appearance => Margin::ZERO,
        Panel::AreasTogether | Panel::LightMaker | Panel::Bake => Margin::ZERO,
        Panel::Preview if crate::appearance_gallery::is_gallery(editor) => Margin::ZERO,
        Panel::Preview if crate::tileset_view::tileset_of(editor).is_some() => Margin::ZERO,
        Panel::Preview if crate::hak_view::hak_of(editor).is_some() => Margin::ZERO,
        Panel::Preview if own.is_some() => Margin::ZERO,
        _ => Margin { left: 8, right: 8, top: 6, bottom: 4 },
    };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        // (Laid out in the host's room and cut at its edges: a widget
        // wider than the dock never widens it, which would push its edge
        // past the window's and draw the editor area over it.)
        let room = ui.available_rect_before_wrap();
        let child = |ui: &mut Ui, salt: &'static str| -> egui::Ui {
            let builder = LateSlot::salted(egui::UiBuilder::new().max_rect(room), host, salt, pane);
            let mut body = ui.new_child(builder);
            body.set_clip_rect(room.intersect(ui.clip_rect()));
            body
        };
        match panel {
            Panel::Palette => {
                let mut body = ui.new_child(egui::UiBuilder::new().max_rect(room));
                body.set_clip_rect(room.intersect(ui.clip_rect()));
                crate::palette_view::ui(app, &mut body);
            }
            // Drawn in Tab order after the tab bar; where an editor tab may show
            // the same model, after the editor area so that the tab keeps its view.
            Panel::Preview
                if room.is_positive() && crate::tileset_view::tileset_of(editor).is_some() =>
            {
                let mut body = child(ui, "tileset-preview");
                crate::tileset_view::preview(app, &mut body, editor);
            }
            Panel::Preview
                if room.is_positive() && crate::appearance_gallery::is_gallery(editor) =>
            {
                let mut body = child(ui, "gallery-preview");
                crate::appearance_gallery::preview(app, &mut body, editor);
            }
            Panel::Appearance => {
                let mut body = child(ui, panel.name());
                crate::appearance_gallery::facts(app, &mut body, editor);
            }
            Panel::Preview if room.is_positive() && crate::hak_view::hak_of(editor).is_some() => {
                let mut body = child(ui, "hak-preview");
                crate::hak_view::preview(app, &mut body, editor);
            }
            Panel::Details => {
                let mut body = child(ui, panel.name());
                crate::hak_view::details(app, &mut body, editor);
            }
            Panel::Entry | Panel::Category => {
                let mut body = child(ui, panel.name());
                match panel {
                    Panel::Entry => crate::journal_view::entry_panel(app, &mut body, editor),
                    _ => crate::journal_view::category_panel(app, &mut body, editor),
                }
            }
            Panel::Faction | Panel::Reaction | Panel::Reactions => {
                let mut body = child(ui, panel.name());
                match panel {
                    Panel::Faction => crate::faction_view::faction_panel(app, &mut body, editor),
                    Panel::Reaction => crate::faction_view::reaction_panel(app, &mut body, editor),
                    _ => crate::faction_view::reactions_panel(app, &mut body, editor),
                }
            }
            Panel::Textures => {
                let mut body = child(ui, panel.name());
                crate::tileset_view::textures(app, &mut body, editor);
            }
            Panel::Preview if room.is_positive() => {
                if host == Host::Dock {
                    app.preview_rect = Some(room);
                }
                let chosen = app.palette.selected.map(crate::model_view::Source::Resource);
                if let Some(source) = own {
                    // (Drawn in its place: its form draws no model, and
                    // where its model has a tab of its own, it says so.)
                    let mut body = child(ui, "preview-panel");
                    crate::area_tools::own_preview_body(app, &mut body, room, source, titled);
                } else if chosen.is_some_and(|s| crate::model_view::a_tab_may_show(app, &s)) {
                    let layer = ui.layer_id();
                    let editor = editor.cloned();
                    app.late_slots.push(LateSlot { panel, rect: room, layer, pane, host, editor });
                } else {
                    let mut body = child(ui, "preview-panel");
                    crate::area_tools::preview_body(app, &mut body, room);
                }
            }
            Panel::TileProperties => {
                let mut body = child(ui, "tile-properties");
                crate::tile_select::panel(app, &mut body, editor);
            }
            Panel::Properties => {
                let mut body = child(ui, "properties");
                match crate::nui_view::panels::nui_of(editor) {
                    Some(_) => crate::nui_view::panels::properties(app, &mut body, editor, titled),
                    None => {
                        let one = crate::properties_panel::one_scroll(app, host);
                        crate::properties_panel::panel(app, &mut body, editor, (titled, one));
                    }
                }
            }
            Panel::AreasTogether | Panel::LightMaker | Panel::Bake => {
                let mut body = child(ui, panel.name());
                crate::tool_sections::body(app, &mut body, panel, titled);
            }
            Panel::Components | Panel::Bindings => {
                let mut body = child(ui, panel.name());
                match panel {
                    Panel::Components => {
                        crate::nui_view::panels::components(app, &mut body, editor)
                    }
                    _ => crate::nui_view::panels::bindings(app, &mut body, editor, titled),
                }
            }
            Panel::Reference => {
                let mut body = child(ui, panel.name());
                crate::script_reference::reference(app, &mut body, editor);
            }
            Panel::NodeData | Panel::NodeSearch | Panel::Conversation => {
                let mut body = child(ui, panel.name());
                match panel {
                    Panel::NodeData => crate::dialog_view::node_data(app, &mut body, editor),
                    Panel::NodeSearch => crate::dialog_view::node_search(app, &mut body, editor),
                    _ => crate::dialog_view::conversation_panel(app, &mut body, editor, titled),
                }
            }
            Panel::Cell | Panel::References | Panel::String | Panel::TalkFile => {
                let talk = talk_of(app, editor).is_some();
                let mut body = child(ui, panel.name());
                match panel {
                    Panel::Cell => crate::table_view::panels::cell(app, &mut body, editor),
                    Panel::References if talk => {
                        crate::talk_view::panels::references(app, &mut body, editor)
                    }
                    Panel::String => crate::talk_view::panels::string(app, &mut body, editor),
                    Panel::TalkFile => crate::talk_view::panels::file(app, &mut body, editor),
                    _ => crate::table_view::panels::references(app, &mut body, editor),
                }
            }
            // Panels hosted in the Bottom Dock or a script window's bottom pane.
            Panel::Compiler
            | Panel::Help
            | Panel::Bookmarks
            | Panel::SearchResults
            | Panel::Checks
            | Panel::Problems
            | Panel::HakConflicts
            | Panel::ResourcesUsed
            | Panel::ClaimsTable => {
                let mut body = child(ui, panel.name());
                match panel {
                    Panel::Compiler => crate::script_panels::compiler(app, &mut body, editor),
                    Panel::Help => crate::script_reference::help(app, &mut body, editor),
                    Panel::Bookmarks => crate::script_panels::bookmarks(app, &mut body, editor),
                    Panel::Checks => crate::nui_view::checks::panel(app, &mut body, editor),
                    Panel::Problems => crate::problems::body(app, &mut body),
                    Panel::HakConflicts => crate::dock_reports::hak_body(app, &mut body),
                    Panel::ResourcesUsed => crate::dock_reports::used_body(app, &mut body),
                    Panel::ClaimsTable => crate::dock_reports::claims_body(app, &mut body),
                    _ => crate::search_results::body(app, &mut body),
                }
                // Report panels' forms and headers draw over their rows: when
                // empty, keep drawn-height tight rather than filling room like
                // Problems or script panels.
                let formed = matches!(
                    panel,
                    Panel::SearchResults
                        | Panel::HakConflicts
                        | Panel::ResourcesUsed
                        | Panel::ClaimsTable
                );
                if host == Host::Dock && formed {
                    drawn_ends(&body);
                }
            }
            #[cfg(test)]
            Panel::Sample(_) => {
                ui.label(format!("{} body", panel.name()));
                let layer = ui.layer_id();
                let editor = editor.cloned();
                app.sample_hosts.push((panel, host, editor.clone()));
                app.late_slots.push(LateSlot { panel, rect: room, layer, pane, host, editor });
            }
            // (The Left Dock's and the log, drawn by their docks.)
            _ => {}
        }
    });
}

/// The width the Right Dock takes at the window's side in a window
/// `window` wide, its line included, as [`right_dock`] draws it (for the
/// Left Dock, drawn before it, to leave the editor area its least): none
/// where it holds no panel, its strip where folded, else its width (280 in
/// a narrow window; the width chosen or the layout's, at least 280 and at
/// most two fifths of the window).
fn right_reserved(app: &Moonglow, window: f32) -> f32 {
    if right_shown(app).is_none() {
        return 0.0;
    }
    if right_folded(app) {
        return kit::STRIP + 1.0;
    }
    if app.yields.right_dock_narrow {
        return RIGHT_DOCK_LEAST + 1.0;
    }
    let wide = chosen_size(kept_right_width(app), right_layout_width(app));
    wide.clamp(RIGHT_DOCK_LEAST, (window * 0.4).max(RIGHT_DOCK_LEAST)) + 1.0
}

/// The Bottom Dock (`Panel::bottom("log")`), unless closed
/// (`Settings::hide_log`) or holding no panel: its tab bar (a tab for
/// each panel it holds, [`panels`]: the editor's own, then the global
/// ones, the Log's; the panel in front's tools at its right, the Log's
/// Copy, Save… and Clear, which yield whole from the end where the dock
/// is too narrow for them beside the tabs, the tabs their icons alone),
/// the panel in front under it, as low as the tab bar and a line and at
/// most half the window. Each panel's switch in the Status Bar opens the
/// dock on it, or closes it ([`status_bar`]).
pub(crate) fn bottom_dock(app: &mut Moonglow, ui: &mut Ui) {
    app.bottom_rect = None;
    if app.settings.hide_log {
        return;
    }
    let held = panels(app, Dock::Bottom);
    let Some(front) = bottom_shown(app) else {
        return;
    };
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let window = ctx.content_rect().size();
    let line = ui.text_style_height(&egui::TextStyle::Monospace);
    // (The tab bar, a line of the log and the room round it; for a panel
    // with a form or tools over its rows, down to its first row's end as
    // last drawn, and the room under it: M4's U28.)
    let base = metrics::DOCK_TABS + line + 8.0 + panel_line(ui);
    // (Never more than half the window: a form taller than that scrolls
    // over its rows, as at the least window.)
    let least = base.max(app.dock_need.min(window.y * 0.5));
    // (The panel in front's tools: the Log's.)
    let buttons = front.header_tools();
    let tips: Vec<String> =
        buttons.iter().map(|b| format!("{}{}", b.3, key_note(app, &ctx, b.0))).collect();
    // (Each tool enabled as its command is: the log's while it has lines.)
    let enabled: Vec<bool> = buttons.iter().map(|b| b.0.enabled(app)).collect();
    // (Drawn at the height chosen, held to what the window allows: as the
    // side docks' widths.)
    let range = least..=(window.y * 0.5).max(least);
    let high = chosen_size(app.settings.bottom_dock_height, metrics::BOTTOM_DOCK);
    draw_at(&ctx, "log", 1, high.clamp(*range.start(), *range.end()));
    let dragged = edge_dragged(&ctx, "log");
    let editor = app.active_editor().cloned();
    let shown = egui::Panel::bottom("log")
        .resizable(true)
        .default_size(metrics::BOTTOM_DOCK)
        .size_range(range)
        .frame(egui::Frame::new().fill(t.panel))
        .show(ui, |ui| {
            dock_pane(ui);
            let pane_id = ui.unique_id();
            let mut tab = held.iter().position(|p| *p == front).unwrap_or(0);
            // The tabs turn to their icons where their words and the
            // tools do not fit side by side; narrower still, the tools
            // yield whole from the end (Clear Log first), each still in
            // View's menu: the tabs and the tools never overlap. (The
            // tools' width the tabs are laid out beside is the widest of
            // the panels' it holds, whichever is in front, so that the
            // tabs keep their form as another is chosen.)
            let button = IconButton::new(Icon::Copy, "").icon_only(true).width(ui);
            let most = held.iter().map(|p| p.tools()).max().unwrap_or(0);
            // (A NUI designer's Checks says what they count at the bar's
            // end, the NUI board's "crier: 0 errors · 1 warning".)
            let detail = match front {
                Panel::Checks => crate::nui_view::checks::detail(app, editor.as_ref()),
                // (Problems' counts, "2 errors · 1 warning": M4's U20.)
                Panel::Problems => crate::problems::detail(app),
                _ => None,
            };
            let detail_width = detail.as_ref().map_or(0.0, |d| {
                let font = egui::TextStyle::Small.resolve(ui.style());
                let galley = ui.fonts_mut(|f| f.layout_no_wrap(d.clone(), font, t.text_3));
                galley.size().x + 12.0
            });
            let mut tabs =
                DockTabs::new(BOTTOM_DOCK).tools_width(button * most as f32 + detail_width);
            for p in &held {
                tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
            }
            let bar_width = ui.available_width();
            // (Narrower than its tabs' icons, in a window too narrow for
            // the docks beside the editor area, the bar is drawn empty,
            // its tools gone first: the panel under it, the Status Bar's
            // switches closing it.)
            let tab_fits = tabs.tabs_width(ui, true) <= bar_width;
            if !tab_fits {
                tabs = DockTabs::new(BOTTOM_DOCK);
            }
            let room = bar_width
                - tabs.tabs_width_in(ui, bar_width)
                - kit::TOOLS_PADDING.0
                - kit::TOOLS_PADDING.1;
            // (The tools go before the tabs do: none without them.)
            let fitting = if tab_fits {
                ((room / button).floor().max(0.0) as usize).min(buttons.len())
            } else {
                0
            };
            let bar = tabs.show(ui, &mut tab, |ui| {
                let mut chosen = None;
                let all = buttons.iter().zip(&tips).zip(&enabled).take(fitting);
                for (((id, icon, name, _), tip), enabled) in all {
                    let button = IconButton::new(*icon, *name).icon_only(true);
                    let r = ui.add_enabled(*enabled, button);
                    if r.on_hover_text(tip).on_disabled_hover_text(tip).clicked() {
                        chosen = Some(*id);
                    }
                }
                if let Some(d) = detail.as_ref().filter(|_| tab_fits) {
                    ui.label(egui::RichText::new(d).small().color(t.text_3));
                }
                chosen
            });
            // (A tab chosen shows its panel from the next frame.)
            let picked = (bar.changed && tab_fits).then(|| held[tab]);
            app.bottom_rect = Some(ui.max_rect());
            let margin = Margin { left: 8, right: 8, top: 4, bottom: 4 };
            let dock_top = ui.max_rect().top();
            ui.ctx().data_mut(|d| d.remove_temp::<f32>(egui::Id::new(FIRST_ROW)));
            match front {
                Panel::Log => {
                    egui::Frame::new().inner_margin(margin).show(ui, |ui| app.log_ui(ui));
                }
                _ => panel_body(app, ui, (front, false), Host::Dock, pane_id, editor.as_ref()),
            }
            // (Its first row's end as drawn: the least height from the
            // next frame, drawn again once where it changed.)
            let first = ui.ctx().data_mut(|d| d.remove_temp::<f32>(egui::Id::new(FIRST_ROW)));
            let need = first.map_or(0.0, |y| (y - dock_top + 8.0).ceil());
            if (need - app.dock_need).abs() > 0.5 {
                app.dock_need = need;
                ui.ctx().request_repaint();
            }
            (bar.inner, picked)
        });
    if dragged {
        app.settings.bottom_dock_height = Some(shown.response.rect.height().round() as u32);
    }
    let (asked, picked) = shown.inner;
    if let Some(panel) = picked {
        choose_bottom(app, panel);
    }
    if let Some(id) = asked {
        id.run(app, &ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::share;

    /// Each tool section's tab icon is no other panel's, and Custom
    /// Light's and Bake's are their own (M4's U29 review: under 1400 the
    /// Right Dock's tabs may be icons alone, and Custom Light's was the
    /// header's Lighting, Bake's the walkmesh overlay's), named in the
    /// window's code by their panels alone. (Edit Areas Together's is also
    /// the header's Together: the review named the other two, whose tabs
    /// were the icons alone at 1280.)
    #[test]
    fn the_tools_tabs_have_icons_of_their_own() {
        use super::Panel;
        let tools = crate::tool_sections::TOOLS;
        for (i, t) in tools.iter().enumerate() {
            for u in &tools[i + 1..] {
                assert_ne!(t.icon(), u.icon(), "{t:?} and {u:?}");
            }
            for p in Panel::KEYED.iter().filter(|p| !tools.contains(p)) {
                assert_ne!(t.icon(), p.icon(), "{t:?}'s icon is {p:?}'s");
            }
        }
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = vec![src];
        let mut uses = std::collections::HashMap::<String, usize>::new();
        while let Some(f) = files.pop() {
            if f.is_dir() {
                files.extend(std::fs::read_dir(&f).unwrap().map(|e| e.unwrap().path()));
            } else if f.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&f).unwrap();
                for t in [Panel::LightMaker, Panel::Bake] {
                    // (The panel's arm of `Panel::icon`, or a name in the window's
                    // own table of icons.)
                    let name = format!("Icon::{:?}", t.icon());
                    let table = format!("= {:?};", t.icon());
                    let n = text.matches(&name).count() + text.matches(&table).count();
                    *uses.entry(name).or_default() += n;
                }
            }
        }
        for (name, n) in uses {
            assert_eq!(n, 1, "{name} named {n} times in mg-ui's code");
        }
    }

    /// The Status Bar's name and note share the room: whole where both
    /// fit; the narrower whole and the other cut where it is under half;
    /// each cut to half where both are wider; never more than the room.
    #[test]
    fn the_name_and_the_note_share_the_room() {
        assert_eq!(share(300.0, 100.0, 150.0), (100.0, 150.0));
        assert_eq!(share(300.0, 400.0, 100.0), (200.0, 100.0));
        assert_eq!(share(300.0, 100.0, 400.0), (100.0, 200.0));
        assert_eq!(share(300.0, 400.0, 400.0), (150.0, 150.0));
        assert_eq!(share(300.0, 400.0, 0.0), (300.0, 0.0), "no note: the name has it all");
        assert_eq!(share(-20.0, 400.0, 400.0), (0.0, 0.0));
    }
}

/// The docks' registry generalized (M4's U2): how sections stand, the
/// Status Bar's switches' widths, and, with a kind of editor of the
/// tests' own bringing test panels ([`Panel::Sample`], built into these
/// tests alone), that what an editor brings fills the docks and its
/// window whatever the kind.
#[cfg(test)]
mod dock_tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};

    const S0: Panel = Panel::Sample(0);
    const S1: Panel = Panel::Sample(1);
    const S2: Panel = Panel::Sample(2);
    const B8: Panel = Panel::Sample(SAMPLE_BOTTOM);
    const B9: Panel = Panel::Sample(SAMPLE_BOTTOM + 1);

    /// Under 1400 the sections fold into one tab bar in the editor's
    /// order (Narrow's Palette, Properties, Preview); in the wide layout
    /// the first section takes the first column and the rest the second
    /// (WideArea), a lone section standing as its first panel and the
    /// rest (today's Palette | Preview); else stacked in one column.
    #[test]
    fn sections_fold_into_one_bar_and_stand_in_columns() {
        let area = vec![vec![Panel::Palette, Panel::Preview]];
        let main = vec![vec![S0, S2], vec![S1]];
        let narrow_order = [S0, S1, S2];
        // Under 1400: one bar, in the editor's order, or the sections'.
        assert_eq!(arrange(&main, &narrow_order, true, false), vec![vec![vec![S0, S1, S2]]]);
        assert_eq!(arrange(&main, &[], true, false), vec![vec![vec![S0, S2, S1]]]);
        assert_eq!(arrange(&main, &narrow_order, true, true), vec![vec![vec![S0, S1, S2]]]);
        // Wide: the first section, then the rest.
        assert_eq!(
            arrange(&main, &narrow_order, false, true),
            vec![vec![vec![S0, S2]], vec![vec![S1]]]
        );
        let three = vec![vec![S0], vec![S1], vec![S2]];
        assert_eq!(
            arrange(&three, &[], false, true),
            vec![vec![vec![S0]], vec![vec![S1], vec![S2]]]
        );
        // A lone section of two: the Palette | the Preview, as today.
        assert_eq!(
            arrange(&area, &[], false, true),
            vec![vec![vec![Panel::Palette]], vec![vec![Panel::Preview]]]
        );
        assert_eq!(arrange(&[vec![S0]], &[], false, true), vec![vec![vec![S0]]]);
        // Between: stacked in one column; the area's one bar.
        assert_eq!(arrange(&main, &narrow_order, false, false), vec![main.clone()]);
        assert_eq!(arrange(&area, &[], false, false), vec![area.clone()]);
        assert!(arrange(&[], &[], false, true).is_empty());
        // (The order folded leaves what it does not name first.)
        assert_eq!(folded_order(&main, &[S1]), vec![S0, S2, S1]);
    }

    /// Stacked sections fill their room in whole points by their shares,
    /// each at least its least where the room has it.
    #[test]
    fn stacked_sections_share_the_room() {
        assert_eq!(section_heights(600.0, &[1000.0, 1000.0]), vec![300.0, 300.0]);
        assert_eq!(section_heights(601.0, &[1000.0, 1000.0, 1000.0]), vec![200.0, 200.0, 201.0]);
        assert_eq!(section_heights(600.0, &[3000.0, 1000.0]), vec![450.0, 150.0]);
        let least = section_heights(600.0, &[10.0, 1000.0, 10.0]);
        assert_eq!(least, vec![SECTION_LEAST, 600.0 - 2.0 * SECTION_LEAST, SECTION_LEAST]);
        assert_eq!(section_heights(100.0, &[1.0, 1.0]), vec![50.0, 50.0], "short: halves");
        assert!(section_heights(100.0, &[]).is_empty());
    }

    /// The switches' width as the Status Bar reckons it is the width its
    /// left group lays them out in, for one to five switches, with their
    /// words, as icons alone, with a badge (`status_bar.rs`'s sweep does
    /// it for the Log's one).
    #[test]
    fn the_switches_widths_for_one_to_five() {
        let ctx = egui::Context::default();
        mg_uikit::install_fonts(&ctx);
        mg_uikit::apply(&ctx, mg_uikit::Theme::Dark);
        let names = ["Log", "Problems", "Compiler", "Bookmarks", "Search Results"];
        let mut out = ctx.run_ui(Default::default(), |ui| {
            for n in 1..=5 {
                for icons in [false, true] {
                    for badged in [false, true] {
                        let make = || -> Vec<StatusSwitch> {
                            (names.iter().take(n).enumerate())
                                .map(|(i, name)| {
                                    let s = StatusSwitch::new(Icon::Log, *name, i == 0);
                                    let badge = (badged && i % 2 == 1)
                                        .then(|| mg_uikit::Badge::count(12 * i));
                                    s.badge_if(badge).icon_only(icons)
                                })
                                .collect()
                        };
                        let reckoned = switches_width(ui, &make());
                        let laid = ui
                            .horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = status::GAP;
                                for s in make() {
                                    ui.add(s);
                                }
                            })
                            .response
                            .rect
                            .width();
                        let what = format!("{n} switches, icons {icons}, badges {badged}");
                        assert!((reckoned - laid).abs() < 0.01, "{what}: {reckoned} {laid}");
                        let one = make()[0].width(ui);
                        if n == 1 {
                            assert_eq!(reckoned, one, "{what}");
                        } else {
                            assert!(reckoned > one * n as f32 * 0.5, "{what}");
                        }
                    }
                }
            }
        });
        out.textures_delta.clear();
    }

    /// A window with no module (the Welcome page in front) whose kind of
    /// editor brings test panels, `width` points wide, the Bottom Dock
    /// open.
    fn sample_window(brings: Brings, kind: &'static str, width: f32) -> Harness<'static, Moonglow> {
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.settings.game_root = Some(mg_testkit::scratch_dir("ui-dock-samples"));
        app.settings.hide_log = false;
        app.sample_brings = Some((kind, brings));
        let mut h = Harness::builder()
            .with_size(egui::vec2(width, 800.0))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        settle(&mut h);
        h
    }

    /// Runs frames until the window rests and its own work is done
    /// (`Moonglow::under_way`), as `tests/app`'s `settle`: never a count of
    /// frames.
    fn settle(h: &mut Harness<'_, Moonglow>) {
        let started = std::time::Instant::now();
        loop {
            let ran = h.try_run();
            if ran.is_ok() && h.state().under_way().is_none() {
                return;
            }
            assert!(started.elapsed() < std::time::Duration::from_secs(30), "never at rest");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// A drag with the pointer from `from` to `to`, in steps.
    fn drag(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2) {
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        h.event(egui::Event::PointerMoved(from));
        h.run();
        h.event(button(from, true));
        h.run();
        for k in 1..=4 {
            h.event(egui::Event::PointerMoved(from + (to - from) * (k as f32 / 4.0)));
            h.run();
        }
        h.event(button(to, false));
        settle(h);
    }

    /// The tabs of the tab bar named `bar`, by name, left to right.
    fn tabs_of(h: &Harness<'_, Moonglow>, bar: &str) -> Vec<String> {
        let bar = h.get_by_label(bar);
        let tab =
            |n: &egui_kittest::kittest::AccessKitNode<'_>| n.role() == egui::accesskit::Role::Tab;
        let mut tabs: Vec<(f32, String)> = (bar
            .query_all(egui_kittest::kittest::by().predicate(tab)))
        .map(|n| (n.rect().left(), n.accesskit_node().label().unwrap_or_default()))
        .collect();
        tabs.sort_by(|a, b| a.0.total_cmp(&b.0));
        tabs.into_iter().map(|(_, name)| name).collect()
    }

    /// The Status Bar's switch named `name`.
    fn switch<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
        h.get_by_label(status::NAME).get_by_label(name)
    }

    /// The Status Bar's switches (its buttons that say whether they are
    /// on), by name, left to right.
    fn switches_shown(h: &Harness<'_, Moonglow>) -> Vec<String> {
        let bar = h.get_by_label(status::NAME);
        let switch = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.toggled().is_some();
        let mut all: Vec<(f32, String)> = (bar
            .query_all(egui_kittest::kittest::by().predicate(switch)))
        .map(|n| (n.rect().left(), n.accesskit_node().label().unwrap_or_default()))
        .collect();
        all.sort_by(|a, b| a.0.total_cmp(&b.0));
        all.into_iter().map(|(_, name)| name).collect()
    }

    const TWO_SECTIONS: &[Section] = &[&[S0, S2], &[S1]];
    const NARROW_ORDER: &[Panel] = &[S0, S1, S2];
    const OWN_BOTTOM: &[Panel] = &[B8, B9];

    fn second_kind() -> Brings {
        Brings {
            right: TWO_SECTIONS,
            folded: NARROW_ORDER,
            bottom: OWN_BOTTOM,
            ..Brings::default()
        }
    }

    /// A second kind of editor (the Welcome page, in these tests) brings
    /// its own panels as the area brings the Palette and the Preview: two
    /// sections stacked in the Right Dock, one bar of the three in its
    /// order under 1400, two columns in the wide layout; its Bottom Dock
    /// panels before the Log, their switches after the Log's, with their
    /// badges, each opening the dock on its panel and closing it; each
    /// body drawn in the dock, and its late body after the editor area.
    #[test]
    fn a_second_kind_of_editor_brings_its_panels() {
        let mut h = sample_window(second_kind(), "welcome", 1600.0);
        assert_eq!(tabs_of(&h, RIGHT_DOCK), ["Sample 0", "Sample 2"]);
        assert_eq!(tabs_of(&h, "Right Dock, Section 2"), ["Sample 1"]);
        assert_eq!(h.state().right_bars, [S0, S1], "each section's front drawn");
        let drawn = |h: &Harness<'_, Moonglow>| -> Vec<(Panel, Host)> {
            h.state().sample_hosts.iter().map(|(p, host, _)| (*p, *host)).collect()
        };
        let welcome = Some(crate::Tab::Welcome);
        assert!(
            h.state().sample_hosts.contains(&(S0, Host::Dock, welcome.clone())),
            "for its editor"
        );
        assert!(drawn(&h).contains(&(S0, Host::Dock)) && drawn(&h).contains(&(S1, Host::Dock)));
        assert!(h.state().sample_late_drawn.contains(&(S0, welcome)), "the late slot drawn");
        // (The global ones after the editor's own: Problems, Search
        // Results and the Log, M4's U13 and U20; the Log's and Problems'
        // switches before the editor's, Search Results' after.)
        assert_eq!(
            tabs_of(&h, BOTTOM_DOCK),
            ["Sample 8", "Sample 9", "Problems", "Search Results", "Log"]
        );
        assert_eq!(
            switches_shown(&h),
            ["Log", "Problems", "Sample 8", "Sample 9", "Search Results"]
        );
        let badge = switch(&h, "Sample 9").accesskit_node().value();
        assert_eq!(badge.as_deref(), Some("1"), "its count");
        // The Bottom Dock in front: the editor's first own panel.
        assert_eq!(bottom_shown(h.state()), Some(B8));
        assert_eq!(
            switch(&h, "Sample 8").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        // A switch off opens the dock on its panel; on, closes it.
        switch(&h, "Sample 9").click();
        settle(&mut h);
        assert!(bottom_shows(h.state(), B9), "on Sample 9");
        switch(&h, "Sample 9").click();
        settle(&mut h);
        assert!(h.state().settings.hide_log, "closed");
        switch(&h, "Log").click();
        settle(&mut h);
        assert!(bottom_shows(h.state(), Panel::Log), "on the Log");
        // A tab of the upper section chosen: its section's front, kept for
        // the kind.
        h.get_by_label("Sample 2").click();
        settle(&mut h);
        assert_eq!(h.state().right_bars, [S2, S1]);
        let kept = h.state().settings.dock_layouts.get("welcome").cloned().unwrap_or_default();
        assert_eq!(kept.fronts, ["Sample 2"]);
        assert_eq!(kept.bottom.as_deref(), Some("Sample 9"), "the kind's own chosen last");
        // Under 1400: one bar, in the kind's order.
        h.set_size(egui::vec2(1280.0, 800.0));
        settle(&mut h);
        assert_eq!(tabs_of(&h, RIGHT_DOCK), ["Sample 0", "Sample 1", "Sample 2"]);
        assert!(h.query_by_label("Right Dock, Section 2").is_none());
        // Wide: the first section's column and the second's.
        h.set_size(egui::vec2(3440.0, 1440.0));
        settle(&mut h);
        assert_eq!(tabs_of(&h, RIGHT_DOCK), ["Sample 0", "Sample 2"]);
        assert_eq!(tabs_of(&h, RIGHT_COLUMN_TWO), ["Sample 1"]);
    }

    /// What the settings keep for a kind is its layout when it comes back
    /// (a new run): the section's front, and the Bottom Dock's of the
    /// kind's own panels; a name this version does not know is passed by.
    #[test]
    fn a_kind_s_layout_is_kept_in_the_settings() {
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.settings.game_root = Some(mg_testkit::scratch_dir("ui-dock-kept"));
        app.settings.hide_log = false;
        let layout = crate::settings::DockLayout {
            fronts: vec!["Sample 2".into(), "A Newer Panel".into()],
            shares: vec![1500, 500],
            bottom: Some("Sample 9".into()),
            column: None,
            folded: None,
            wide_width: None,
            window_width: None,
            window_bottom: None,
        };
        app.settings.dock_layouts.insert("welcome".into(), layout);
        app.sample_brings = Some(("welcome", second_kind()));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1600.0, 800.0))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        settle(&mut h);
        assert_eq!(h.state().right_bars, [S2, S1]);
        assert_eq!(bottom_shown(h.state()), Some(B9));
        // (The shares: three quarters over a quarter.)
        let upper = h.get_by_label("Sample 2").rect().top();
        let lower = h.get_by_label("Sample 1").rect().top();
        let dock = h.state().right_rect.unwrap();
        let share = (lower - upper) / dock.height();
        assert!((share - 0.75).abs() < 0.02, "{share}");
    }

    /// A dock whose panels none is available is not drawn: no tab bar, no
    /// strip, no room kept.
    #[test]
    fn a_dock_with_no_available_panel_is_not_drawn() {
        const EMPTY: &[Section] = &[&[Panel::Sample(SAMPLE_EMPTY)]];
        let brings = Brings { right: EMPTY, ..Brings::default() };
        let h = sample_window(brings, "welcome", 1600.0);
        assert!(h.state().right_rect.is_none(), "no Right Dock");
        assert!(h.query_by_label(RIGHT_DOCK).is_none());
        assert!(h.query_by_label("Show the Right Dock").is_none(), "no strip");
        assert_eq!(right_shown(h.state()), None);
        // (With one that has: drawn.)
        const ONE: &[Section] = &[&[Panel::Sample(SAMPLE_EMPTY)], &[S0]];
        let h = sample_window(Brings { right: ONE, ..Brings::default() }, "welcome", 1600.0);
        assert_eq!(tabs_of(&h, RIGHT_DOCK), ["Sample 0"]);
        assert!(h.query_by_label("Right Dock, Section 2").is_none());
    }

    /// Where the editor floats, a kind that brings its panels into its
    /// window draws each body there (the same body as in the dock, its
    /// host the window), its own bottom panels under it; the main window's
    /// docks are the docked editor's.
    #[test]
    fn a_floating_editor_s_panels_are_in_its_window() {
        let brings = Brings { floats: true, ..second_kind() };
        let mut h = sample_window(brings, "manual", 1600.0);
        let surface = h.state_mut().dock.add_window(vec![crate::Tab::Manual]);
        let window = h.state_mut().dock.get_window_state_mut(surface).unwrap();
        window.set_position(egui::pos2(300.0, 100.0)).set_size(egui::vec2(900.0, 600.0));
        settle(&mut h);
        let state = h.state();
        // (Each drawn for the floating editor, not the docked Welcome page.)
        let manual = Some(crate::Tab::Manual);
        let hosts = &state.sample_hosts;
        assert!(hosts.contains(&(S0, Host::Window, manual.clone())), "{hosts:?}");
        assert!(hosts.contains(&(B8, Host::Window, manual.clone())));
        assert!(!hosts.iter().any(|(_, host, _)| *host == Host::Dock), "none docked");
        assert!(state.sample_late_drawn.contains(&(S0, manual)), "its late body, for it");
        // (The right pane the whole height, the bottom pane under the
        // editor alone, beside it: Floating.png.)
        let pane = |name: &str| state.sample_panes.iter().rev().find(|p| p.0 == name).unwrap().1;
        let (right, bottom) = (pane(WINDOW_RIGHT), pane(WINDOW_BOTTOM));
        assert!(bottom.right() <= right.left() + 1.0, "{bottom:?} beside {right:?}");
        assert!(right.bottom() >= bottom.bottom() - 1.0 && right.top() < bottom.top());
        assert_eq!(tabs_of(&h, WINDOW_RIGHT), ["Sample 0", "Sample 2"]);
        assert_eq!(tabs_of(&h, &format!("{WINDOW_RIGHT}, Section 2")), ["Sample 1"]);
        assert_eq!(tabs_of(&h, WINDOW_BOTTOM), ["Sample 8", "Sample 9"]);
        assert!(h.state().right_rect.is_none(), "the Welcome page brings no Right Dock");
        let globals = ["Log", "Problems", "Search Results"];
        assert_eq!(switches_shown(&h), globals, "the window's own panels have no switch");
    }

    /// A seam between stacked sections dragged keeps the two sections'
    /// shares for the kind, and they stand so; a window's right pane
    /// stands its sections by the same shares.
    #[test]
    fn a_seam_dragged_keeps_the_sections_shares() {
        let mut h = sample_window(second_kind(), "welcome", 1600.0);
        let upper = h.get_by_label("Sample 2").rect().top();
        let lower = h.get_by_label("Sample 1").rect().top();
        let x = h.get_by_label("Sample 1").rect().center().x + 60.0;
        let seam = egui::pos2(x, lower - 1.0);
        drag(&mut h, seam, seam + egui::vec2(0.0, 120.0));
        let moved = h.get_by_label("Sample 1").rect().top();
        assert!((moved - lower - 120.0).abs() < 2.0, "{lower} to {moved}");
        assert_eq!(h.get_by_label("Sample 2").rect().top(), upper, "the upper bar stays");
        let kept = h.state().settings.dock_layouts.get("welcome").cloned().unwrap_or_default();
        assert_eq!(kept.shares.len(), 2, "{kept:?}");
        assert!(kept.shares[0] > kept.shares[1], "{kept:?}");
        // (The window's right pane, by the kind's shares.)
        let layout = crate::settings::DockLayout { shares: vec![1500, 500], ..Default::default() };
        let mut h = sample_window(Brings { floats: true, ..second_kind() }, "manual", 1600.0);
        h.state_mut().settings.dock_layouts.insert("manual".into(), layout);
        let surface = h.state_mut().dock.add_window(vec![crate::Tab::Manual]);
        let window = h.state_mut().dock.get_window_state_mut(surface).unwrap();
        window.set_position(egui::pos2(300.0, 100.0)).set_size(egui::vec2(900.0, 600.0));
        settle(&mut h);
        let top = h.get_by_label(WINDOW_RIGHT).query_by_label("Sample 0").unwrap().rect().top();
        let second = format!("{WINDOW_RIGHT}, Section 2");
        let next = h.get_by_label(&second).query_by_label("Sample 1").unwrap().rect().top();
        let right = h.state().sample_panes.iter().rev().find(|p| p.0 == WINDOW_RIGHT).unwrap().1;
        let share = (next - top) / (right.bottom() - top);
        assert!((share - 0.75).abs() < 0.03, "{share}");
    }

    /// The palettes asked for by name beside a kind that brings none keep
    /// nothing in the settings for that kind: their section is not its.
    #[test]
    fn the_palettes_asked_for_keep_nothing_for_another_kind() {
        let mut h = sample_window(Brings::default(), "welcome", 1600.0);
        h.state_mut().show_dock_panel(Panel::Preview);
        settle(&mut h);
        assert_eq!(h.state().right_dock_shows(), Some(Panel::Preview), "shown");
        h.state_mut().show_dock_panel(Panel::Palette);
        settle(&mut h);
        let saved = h.state().settings.saved_text();
        assert!(!saved.contains("dock_layouts"), "{saved}");
        // (With a kind's own sections beside them: theirs not kept either,
        // and the kind's kept by its own places.)
        let mut h = sample_window(second_kind(), "welcome", 1600.0);
        h.state_mut().show_dock_panel(Panel::Palette);
        settle(&mut h);
        h.get_by_label("Sample 2").click();
        settle(&mut h);
        let kept = h.state().settings.dock_layouts.get("welcome").cloned().unwrap_or_default();
        assert_eq!(kept.fronts, ["Sample 2"], "at its own place, the palettes' before it");
    }

    /// View › Reset Layout forgets the kinds' layouts.
    #[test]
    fn reset_layout_forgets_the_kinds_layouts() {
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let layout = crate::settings::DockLayout { shares: vec![1500, 500], ..Default::default() };
        app.settings.dock_layouts.insert("script".into(), layout);
        app.reset_layout(&egui::Context::default());
        assert!(app.settings.dock_layouts.is_empty());
    }

    /// A press in a panel an editor brings makes that editor's pane the
    /// plugins' pane last clicked in: the palette's for the Palette and the
    /// Preview, the area's for an area's other panels, none (left as it
    /// was) for an editor with no pane of its own.
    #[test]
    fn a_brought_panel_is_its_editor_s_pane_for_plugins() {
        use crate::plugins::Pane;
        let app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let app = &app;
        let welcome = crate::Tab::Welcome;
        assert_eq!(plugin_pane(app, Panel::Palette, Some(&welcome)), Some(Pane::Palette));
        assert_eq!(plugin_pane(app, Panel::Preview, None), Some(Pane::Palette));
        assert_eq!(plugin_pane(app, S0, Some(&welcome)), None, "the Welcome page has no pane");
        let area = mg_core::ResRef::from_str("area001").unwrap();
        let tab = crate::Tab::Area(area);
        assert_eq!(plugin_pane(app, S0, Some(&tab)), Some(Pane::Area(area)));
        assert_eq!(plugin_pane(app, Panel::Preview, Some(&tab)), Some(Pane::Palette));
        // A press in a docked panel of the Welcome page's: left as it was.
        let mut h = sample_window(second_kind(), "welcome", 1600.0);
        h.get_by_label("Sample 1 body").click();
        settle(&mut h);
        assert_eq!(h.state().plugins.pane, None);
        // A press in a floating area's window's pane: the area's.
        let brings = Brings { floats: true, ..second_kind() };
        let mut h = sample_window(brings, "area", 1600.0);
        let surface = h.state_mut().dock.add_window(vec![tab]);
        let window = h.state_mut().dock.get_window_state_mut(surface).unwrap();
        window.set_position(egui::pos2(300.0, 100.0)).set_size(egui::vec2(900.0, 600.0));
        settle(&mut h);
        h.get_by_label("Sample 0 body").click();
        settle(&mut h);
        assert_eq!(h.state().plugins.pane, Some(Pane::Area(area)));
    }

    /// A press in a stacked section off its tab bar, and F6 to a section,
    /// move the dock's panel there and write nothing; a tab clicked is a
    /// choice, kept (the review's B3).
    #[test]
    fn only_a_tab_chosen_is_kept() {
        let mut h = sample_window(second_kind(), "welcome", 1600.0);
        h.get_by_label("Sample 1 body").click();
        settle(&mut h);
        assert_eq!(h.state().right_panel, Some(S1), "the dock's panel");
        let ctx = h.ctx.clone();
        for _ in 0..4 {
            h.state_mut().step_pane(&ctx, 1);
            settle(&mut h);
        }
        assert!(
            h.state().settings.dock_layouts.is_empty(),
            "{:?}",
            h.state().settings.dock_layouts
        );
        h.get_by_label("Sample 2").click();
        settle(&mut h);
        assert!(h.state().settings.dock_layouts.contains_key("welcome"), "a tab chosen: kept");
    }

    /// The arrow keys leave the log where it is not drawn: the Bottom Dock
    /// closed (to the editor), or another of its tabs in front (to that
    /// tab's panel) (the review's B4).
    #[test]
    fn the_keys_leave_the_log_where_it_is_not_drawn() {
        use crate::palette_view::{Keys, give_keys, keys_are};
        let mut h = sample_window(second_kind(), "welcome", 1600.0);
        choose_bottom(h.state_mut(), Panel::Log);
        settle(&mut h);
        give_keys(&h.ctx, Keys::Log);
        settle(&mut h);
        assert!(keys_are(&h.ctx, Keys::Log), "the log drawn: kept");
        h.state_mut().settings.hide_log = true;
        settle(&mut h);
        assert!(!keys_are(&h.ctx, Keys::Log), "the dock closed");
        h.state_mut().settings.hide_log = false;
        settle(&mut h);
        give_keys(&h.ctx, Keys::Log);
        choose_bottom(h.state_mut(), B8);
        settle(&mut h);
        assert!(keys_are(&h.ctx, B8.keys()), "to the tab in front");
    }

    /// A kind may bring the Preview without the palettes (U11, U20, U21);
    /// the palettes asked for by name beside it add the Palette alone,
    /// never a second Preview in the dock; a floating editor's Preview and
    /// the dock's are drawn in one frame apart (the review's C2).
    #[test]
    fn a_kind_brings_the_preview_alone() {
        const OWN: &[Section] = &[&[Panel::Preview]];
        let brings = Brings { right: OWN, ..Brings::default() };
        let mut h = sample_window(brings, "welcome", 1600.0);
        assert_eq!(tabs_of(&h, RIGHT_DOCK), ["Preview"]);
        h.state_mut().show_dock_panel(Panel::Palette);
        settle(&mut h);
        assert_eq!(tabs_of(&h, RIGHT_DOCK), ["Palette"]);
        assert_eq!(tabs_of(&h, "Right Dock, Section 2"), ["Preview"], "one Preview");
        // (And one in a floating editor's window beside the dock's.)
        let mut h = sample_window(Brings { floats: true, ..brings }, "manual", 1600.0);
        h.state_mut().show_dock_panel(Panel::Preview);
        let surface = h.state_mut().dock.add_window(vec![crate::Tab::Manual]);
        let window = h.state_mut().dock.get_window_state_mut(surface).unwrap();
        window.set_position(egui::pos2(300.0, 100.0)).set_size(egui::vec2(900.0, 600.0));
        settle(&mut h);
        let said = "Choose a blueprint in the palette.";
        let both = h.query_all_by_label(said).count();
        assert_eq!(both, 2, "the dock's Preview and the window's");
    }

    /// View › Log shows the Log in front of the Bottom Dock, its row ticked
    /// only while it is; a second time closes the dock (the review's C3).
    #[test]
    fn view_log_shows_the_log_tab() {
        let mut h = sample_window(second_kind(), "welcome", 1600.0);
        assert_eq!(bottom_shown(h.state()), Some(B8));
        assert!(!h.state().panel_shown(Id::ViewLog), "another tab in front: not ticked");
        let ctx = h.ctx.clone();
        Id::ViewLog.run(h.state_mut(), &ctx);
        settle(&mut h);
        assert!(bottom_shows(h.state(), Panel::Log) && h.state().panel_shown(Id::ViewLog));
        Id::ViewLog.run(h.state_mut(), &ctx);
        settle(&mut h);
        assert!(h.state().settings.hide_log, "closed");
    }

    /// F6 stops at a floating editor's window's panes, after the editor
    /// area (the review's C5).
    #[test]
    fn f6_stops_at_a_window_s_panes() {
        use crate::palette_view::keys_are;
        let brings = Brings { floats: true, ..second_kind() };
        let mut h = sample_window(brings, "manual", 1600.0);
        h.state_mut().settings.hide_log = true;
        let surface = h.state_mut().dock.add_window(vec![crate::Tab::Manual]);
        let window = h.state_mut().dock.get_window_state_mut(surface).unwrap();
        window.set_position(egui::pos2(300.0, 100.0)).set_size(egui::vec2(900.0, 600.0));
        settle(&mut h);
        assert!(h.state().window_bars.contains(&S0), "{:?}", h.state().window_bars);
        let ctx = h.ctx.clone();
        let mut reached = false;
        for _ in 0..6 {
            h.state_mut().step_pane(&ctx, 1);
            settle(&mut h);
            reached |= keys_are(&ctx, S0.keys());
        }
        assert!(reached, "the window's pane reached");
    }

    /// The global Bottom Dock panels a kind names follow it into its
    /// window, after its own (the review's B2).
    #[test]
    fn a_window_s_bottom_pane_holds_the_globals_its_kind_names() {
        let brings = Brings { floats: true, window_bottom: &[Panel::Log], ..second_kind() };
        let mut h = sample_window(brings, "manual", 1600.0);
        let surface = h.state_mut().dock.add_window(vec![crate::Tab::Manual]);
        let window = h.state_mut().dock.get_window_state_mut(surface).unwrap();
        window.set_position(egui::pos2(300.0, 100.0)).set_size(egui::vec2(900.0, 600.0));
        settle(&mut h);
        assert_eq!(tabs_of(&h, WINDOW_BOTTOM), ["Sample 8", "Sample 9", "Log"]);
    }

    /// The placeholder that stands for no area keeps its layout as the
    /// area's; every tab is its own editor's until U19 maps one (C6).
    #[test]
    fn the_placeholder_s_layout_is_the_area_s() {
        let app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        assert_eq!(layout_kind(&app, &crate::Tab::NoArea), "area");
        assert_eq!(layout_kind(&app, &crate::Tab::Manual), "manual");
        assert_eq!(*owner_of(&app, &crate::Tab::Manual), crate::Tab::Manual);
    }

    #[test]
    fn area_right_layout_mutations_and_drop_targets() {
        use crate::settings::{AreaRightLayout, AreaRightPanel as P};
        let two = AreaRightLayout::two_columns();
        // Stack Properties below Palette | Preview.
        let stacked = apply_area_drop(&two, P::Properties, StackZone::Below { col: 0, sec: 0 });
        assert_eq!(stacked, AreaRightLayout::stacked());
        // Combine Properties into Palette | Preview tab bar.
        let tabbed = apply_area_drop(&two, P::Properties, StackZone::Into { col: 0, sec: 0 });
        assert_eq!(tabbed, AreaRightLayout::one_tab_bar());
        // Dragging Properties out of One Tab Bar into RightColumn restores Two Columns.
        let split_right = apply_area_drop(&tabbed, P::Properties, StackZone::Column { last: true });
        assert_eq!(split_right, two);
        // Dragging Properties to LeftColumn swaps columns.
        let props_left = apply_area_drop(&two, P::Properties, StackZone::Column { last: false });
        assert_eq!(
            props_left.columns,
            vec![vec![vec![P::Properties]], vec![vec![P::Palette, P::Preview]]]
        );
        // Reordering tabs in One Tab Bar.
        let reordered =
            apply_area_drop(&tabbed, P::Properties, StackZone::Tab { col: 0, sec: 0, index: 0 });
        assert_eq!(reordered.columns, vec![vec![vec![P::Properties, P::Palette, P::Preview]]]);
    }
}
