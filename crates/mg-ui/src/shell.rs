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
//! (320, 741) and the Right Dock stands in two columns, Palette and
//! Preview. Hints (T12) and the Outliner (T13) are in their own places.

use egui::{Margin, Ui};
use mg_uikit::shell::{self as kit, DockTab, DockTabs, FoldStrip, Side, StripClick, Yields};
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
    let wanted = app.settings.ui_scale.map_or(ctx.zoom_factor(), |s| f32::from(s) / 100.0);
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
/// editor's own panels and the global ones ([`Panel::BOTTOM_TABS`]: the
/// Log), one tab bar, each with its switch in the Status Bar. A dock
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
    /// area's placed objects by kind, each kind with an eye
    /// (`outliner.rs`).
    Outliner,
    /// The log: the Bottom Dock's global panel, whatever the editor in
    /// front, with its switch in the Status Bar.
    Log,
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
    pub(crate) const KEYED: [Panel; 6] = [
        Panel::Module,
        Panel::Resources,
        Panel::Outliner,
        Panel::Palette,
        Panel::Preview,
        Panel::Log,
    ];
    /// The Bottom Dock's global panels (shown whatever the editor in
    /// front), in their tabs' order after the editor's own: the Script
    /// board's Compiler, Bookmarks, then Search Results, Log (U13 puts
    /// Search Results here, before the Log).
    pub(crate) const BOTTOM_TABS: &'static [Panel] = &[Panel::Log];
    /// The global panels' switches before the editor's own in the Status
    /// Bar (the Script board's Log, Problems: U20 adds Problems), and
    /// after them (its Search Results: U13).
    pub(crate) const SWITCHES_BEFORE: &'static [Panel] = &[Panel::Log];
    pub(crate) const SWITCHES_AFTER: &'static [Panel] = &[];

    /// The dock it lives in.
    pub(crate) fn dock(self) -> Dock {
        match self {
            Panel::Module | Panel::Resources | Panel::Outliner => Dock::Left,
            Panel::Palette | Panel::Preview => Dock::Right,
            Panel::Log => Dock::Bottom,
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
            Panel::Preview => "Preview: the blueprint chosen in the palette",
            Panel::Outliner => "Outliner: what the area in front holds",
            Panel::Log => "Log: what Moonglow reports",
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
            _ => &[],
        }
    }

    /// Its switch's count badge in the Status Bar, where it has one (M4:
    /// Problems' errors, Search Results' hits, Checks'); the Log has none.
    /// `editor` is the editor it counts for (the active docked one).
    fn badge(self, app: &Moonglow, editor: Option<&crate::Tab>) -> Option<mg_uikit::Badge> {
        let _ = (app, editor);
        match self {
            #[cfg(test)]
            Panel::Sample(n) if n >= SAMPLE_BOTTOM => {
                Some(mg_uikit::Badge::count(usize::from(n - SAMPLE_BOTTOM)))
            }
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
    /// on the tab beside theirs; the Outliner where the active editor
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
            Panel::Preview => match brings.has(self) && !brings.has(Panel::Palette) {
                true => true,
                false => Panel::Palette.available_with(app, brings),
            },
            Panel::Outliner => brings.outliner,
            Panel::Log => true,
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
    /// The global Bottom Dock panels that follow the editor into its
    /// window, after its own on the window's bottom pane (Floating.png's
    /// script: Search Results, U13), where it `floats`.
    pub(crate) window_bottom: &'static [Panel],
}

impl Brings {
    /// Whether it brings `panel` to a side or the Bottom Dock.
    pub(crate) fn has(&self, panel: Panel) -> bool {
        self.right.iter().any(|s| s.contains(&panel)) || self.bottom.contains(&panel)
    }
}

/// The area editor's Right Dock (and the placeholder's in its place): the
/// Palette and the Preview, one section.
const AREA_RIGHT: &[Section] = &[&[Panel::Palette, Panel::Preview]];

/// What the editor `tab` brings to the docks (the plan's decision 2,
/// `editor_brings` by kind): an area's tab the Outliner (with a module
/// open: what is placed is read from the module) and, with the game's
/// data read and a module open (the palettes are the game's), the
/// Palette and the Preview; the placeholder that stands for no area the
/// palettes alone (it holds nothing to list, and no panel is shown
/// empty). Any other tab brings none until M4 gives it its own: the Left
/// Dock is then its tabs' panel alone, the whole height, and the Right
/// Dock is not drawn, folded or not, its sizes kept.
pub(crate) fn brings_of(app: &Moonglow, tab: &crate::Tab) -> Brings {
    let tab = owner_of(app, tab);
    #[cfg(test)]
    if let Some((kind, brings)) = app.sample_brings
        && kind == tab.kind()
    {
        return brings;
    }
    let palettes = app.game.is_some() && app.ws.is_some();
    let right = if palettes { AREA_RIGHT } else { &[] };
    match tab {
        crate::Tab::Area(_) => Brings { outliner: app.ws.is_some(), right, ..Brings::default() },
        crate::Tab::NoArea => Brings { right, ..Brings::default() },
        _ => Brings::default(),
    }
}

/// The editor a tab belongs to, whose panels it brings and whose layout
/// of the docks the settings keep: the tab itself, for every tab today.
/// A tab that is part of another editor (U19: the NUI designer's source,
/// a second editor group of the designer's) maps here to the designer's
/// tab, so that focusing it keeps the designer's Right Dock. A script's
/// group brings its own (the owner's ruling at T9's review).
pub(crate) fn owner_of<'a>(app: &Moonglow, tab: &'a crate::Tab) -> &'a crate::Tab {
    let _ = app;
    tab
}

/// The kind a tab's layout of the docks is kept under
/// (`Settings::dock_layouts`): its owner's `Tab::kind`, the placeholder
/// that stands for no area the area's (they bring the same section).
pub(crate) fn layout_kind(app: &Moonglow, tab: &crate::Tab) -> &'static str {
    match owner_of(app, tab) {
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
        let asked = AREA_RIGHT[0].iter().copied();
        sections.insert(0, asked.filter(|p| *p == Panel::Palette || !brings.has(*p)).collect());
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
    let layout = app.settings.dock_layouts.entry(kind.to_string()).or_default();
    if layout.fronts.len() <= index {
        layout.fronts.resize(index + 1, String::new());
    }
    layout.fronts[index] = panel.name().to_string();
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
    let brings = brings_of(app, tab);
    if !brings.floats {
        return;
    }
    let kind = Some(layout_kind(app, tab));
    let held = |s: Section| -> Vec<Panel> {
        s.iter().copied().filter(|p| p.available_with(app, &brings)).collect()
    };
    let right: Vec<Vec<Panel>> =
        brings.right.iter().map(|s| held(s)).filter(|s| !s.is_empty()).collect();
    let mut bottom = held(brings.bottom);
    for p in held(brings.window_bottom) {
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
    // editor alone, as Floating.png draws them.)
    if !right.is_empty() {
        let most = (ui.available_width() * 0.6).max(RIGHT_DOCK_LEAST);
        let shown = egui::Panel::right(pane.with("window-right"))
            .resizable(true)
            .default_size(RIGHT_DOCK_LEAST)
            .size_range(RIGHT_DOCK_LEAST.min(most)..=most)
            .frame(egui::Frame::new().fill(t.panel))
            .show(ui, |ui| {
                let whole = ui.max_rect();
                let stack = Stack { kind, brings: &brings };
                let places = section_rects(app, whole, &right, stack);
                let mut chosen = None;
                for (i, (place, section)) in places.iter().copied().zip(&right).enumerate() {
                    let name = match i {
                        0 => WINDOW_RIGHT.to_string(),
                        _ => format!("{WINDOW_RIGHT}, Section {}", i + 1),
                    };
                    let front = run_or_kept_front(app, kind, &brings, section);
                    let mut part =
                        ui.new_child(egui::UiBuilder::new().id_salt(&name).max_rect(place));
                    part.set_clip_rect(place.intersect(ui.clip_rect()));
                    if pressed(&part, place) {
                        app.plugins.pane = plugin_pane(front, Some(tab)).or(app.plugins.pane);
                    }
                    let mut at = section.iter().position(|p| *p == front).unwrap_or(0);
                    let mut tabs = DockTabs::new(&name);
                    for p in section {
                        tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
                    }
                    let bar = tabs.show(&mut part, &mut at, |_| ());
                    if bar.changed {
                        chosen = Some(section[at]);
                    }
                    app.window_bars_drawn.push(front);
                    panel_body(app, &mut part, front, Host::Window, pane, Some(tab));
                }
                let salt = format!("window-right-{}", pane.value());
                section_seams(app, ui, &places, &right, stack, &salt);
                #[cfg(test)]
                app.sample_panes.push((WINDOW_RIGHT, whole));
                ui.advance_cursor_after_rect(whole);
                chosen
            });
        if let Some(panel) = shown.inner {
            keep_front(app, kind, &brings, panel);
        }
    }
    if !bottom.is_empty() {
        let kept = (kind.and_then(|k| app.settings.dock_layouts.get(k)))
            .and_then(|l| l.bottom.as_deref())
            .and_then(Panel::named)
            .filter(|p| bottom.contains(p));
        let front = kept.unwrap_or(bottom[0]);
        let shown = egui::Panel::bottom(pane.with("window-bottom"))
            .resizable(true)
            .default_size(metrics::BOTTOM_DOCK)
            .frame(egui::Frame::new().fill(t.panel))
            .show(ui, |ui| {
                if pressed(ui, ui.max_rect()) {
                    app.plugins.pane = plugin_pane(front, Some(tab)).or(app.plugins.pane);
                }
                let mut at = bottom.iter().position(|p| *p == front).unwrap_or(0);
                let mut tabs = DockTabs::new(WINDOW_BOTTOM);
                for p in &bottom {
                    tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
                }
                #[cfg(test)]
                app.sample_panes.push((WINDOW_BOTTOM, ui.max_rect()));
                let bar = tabs.show(ui, &mut at, |_| ());
                app.window_bars_drawn.push(front);
                match front {
                    Panel::Log => {
                        let margin = Margin { left: 8, right: 8, top: 4, bottom: 4 };
                        egui::Frame::new().inner_margin(margin).show(ui, |ui| app.log_ui(ui));
                    }
                    _ => panel_body(app, ui, front, Host::Window, pane, Some(tab)),
                }
                bar.changed.then(|| bottom[at])
            });
        if let (Some(panel), Some(kind)) = (shown.inner, kind) {
            app.settings.dock_layouts.entry(kind.to_string()).or_default().bottom =
                Some(panel.name().to_string());
        }
    }
}

/// The name of an editor's window's right pane's tab bar (its first
/// section's; [`window_panes`]).
pub(crate) const WINDOW_RIGHT: &str = "Window's Right Pane";
/// The name of an editor's window's bottom pane's tab bar.
pub(crate) const WINDOW_BOTTOM: &str = "Window's Bottom Pane";

/// The panel the Right Dock shows (its tab in front), of those it holds:
/// the one chosen last (`Moonglow::right_panel`), else its first section's
/// front. `None` where it holds none (and is not drawn).
pub(crate) fn right_shown(app: &Moonglow) -> Option<Panel> {
    let sections = right_sections(app);
    let held = |p: &Panel| sections.iter().any(|s| s.contains(p));
    if let Some(chosen) = app.right_panel.filter(held) {
        return Some(chosen);
    }
    sections.first().map(|s| section_front(app, s))
}

/// The panel the Left Dock shows (its tab selected), of those it holds:
/// the one chosen last, else its first. `None` where it holds none.
pub(crate) fn left_shown(app: &Moonglow) -> Option<Panel> {
    let held = panels(app, Dock::Left);
    held.contains(&app.left_panel).then_some(app.left_panel).or(held.first().copied())
}

/// The panel the Bottom Dock shows (its tab in front), of those it holds:
/// the one chosen last (`Moonglow::bottom_panel`), else the one the
/// settings keep for the editor's kind (of its own panels), else its
/// first. `None` where it holds none.
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
    kept.or(held.first().copied())
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
    // (The windows' panes drawn at the last frame, for F6.)
    app.window_bars = std::mem::take(&mut app.window_bars_drawn);
    if !app.yields.left_dock_folded {
        app.left_unfolded_here = false;
    }
    if !app.yields.right_dock_folded {
        app.right_unfolded_here = false;
    }
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
    let Some(owner) = Panel::KEYED.into_iter().find(|p| keys_are(ctx, p.keys())) else { return };
    let held = owner.keys();
    let (front, drawn) = match owner.dock() {
        // (Drawn on its other tab, the Left Dock's tab bar hands them to
        // that tab's panel, the tree's to the browser's list and back.)
        Dock::Left => {
            let left = left_shown(app).filter(|_| !left_folded(app));
            let drawn = left.is_some() && (owner != Panel::OUTLINER || owner.available(app));
            (None, drawn)
        }
        Dock::Right => {
            let right = right_shown(app).filter(|_| !right_folded(app));
            let apart = app.right_columns && app.right_bars.iter().any(|q| q.keys() == held);
            (right, right.is_some_and(|p| p.keys() == held) || apart)
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
    let both = left_shown(app).is_some() && right_shown(app).is_some();
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

/// The Module panel's header buttons that run commands (today's
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
            let shown = name.map_or_else(|| gi.root.display().to_string(), |n| format!("…/{n}"));
            (shown, Some(gi.root.display().to_string()))
        }
        None => ("No game install".to_string(), None),
    };
    // The log's last line, while the dock that shows the log is closed:
    // its message's first line (the board's, with no time), in the log's
    // color for a warning or an error; its time and its first lines in
    // the tip, made only while the line is hovered or has the focus.
    let log = &app.log;
    let last = (!log_on).then(|| last_line(log)).flatten().map(|(i, line)| {
        let color = match log.entries[i].0 {
            crate::Level::Info => None,
            crate::Level::Warning => Some(t.warn),
            crate::Level::Error => Some(t.error),
        };
        (i, line, color)
    });
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
                .map(|c| match &c.tip {
                    Some(tip) => StatusItem::new(&c.text).tooltip(tip),
                    None => StatusItem::new(&c.text),
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
            // First of all, the switches show their icons alone: under
            // 1400 (`Yields::status_switch_icons`, the Narrow board's), or
            // where the bar at its fullest does not fit beside their words;
            // then the folder goes: under 1400 (`Yields::game_folder`), or
            // where it does not fit beside the middle at its fullest.
            let gaps = 2.0 * status::GAP;
            let full = with_folder + middle_full;
            let icons = yields.status_switch_icons
                || switches_width(ui, &switches) + gaps + full > ui.available_width();
            let switches: Vec<StatusSwitch> =
                switches.into_iter().map(|s| s.icon_only(icons)).collect();
            let left = switches_width(ui, &switches) + gaps;
            let room = ui.available_width() - left;
            let show_folder = yields.game_folder && full <= room;
            // (Where a count goes, the middle has gone before it: what the
            // count leaves is no room for the line.)
            let mut middle_on = true;
            if !show_folder {
                // (The middle yields before the counts: they go only where
                // the right-hand items alone are wider than the bar.)
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
                        let name = format!("Last log line: {text}");
                        let mut line = StatusLog::new(text.as_str()).name(name).max_width(width);
                        if let Some(color) = color {
                            line = line.color(color);
                        }
                        let mut shown = ui.add(line);
                        if shown.hovered() || shown.has_focus() {
                            tips_made.set(tips_made.get() + 1);
                            shown = shown.on_hover_text(last_line_tip(log, i));
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
        app.show_panel(Id::ViewLog, true);
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

/// The most of the bar the log's last line takes now: less beside an
/// area, whose hints are many.
pub(crate) fn last_line_cap(app: &Moonglow) -> f32 {
    match app.active_editor() {
        Some(crate::Tab::Area(_) | crate::Tab::NoArea) => LAST_LINE_CAP_AREA,
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
}

impl Shown {
    /// What it says, a line a hint (as a screen reader says it) and a
    /// line a count: two frames that say the same show the same.
    pub(crate) fn said(&self) -> Vec<String> {
        let hints = self.hints.iter().map(Hint::spoken);
        hints.chain(self.counts.iter().map(|c| c.text.clone())).collect()
    }
}

/// One of the Status Bar's counts ("11 by 11 tiles", "1 selected"), with
/// its tip where it has more to say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Count {
    pub(crate) text: String,
    pub(crate) tip: Option<String>,
}

impl Count {
    pub(crate) fn new(text: impl Into<String>) -> Count {
        Count { text: text.into(), tip: None }
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
    let escape = app.escape_hint_next.clone().filter(|_| !app.model_under_pointer_next);
    let named = escape.as_ref().map(|t| (t.clone(), crate::tabs::name(app, t)));
    let hovered = app.options_hovered_next;
    let next = shown(app, ctx, named.as_ref().map(|(t, n)| (t, n.as_str())), hovered);
    if next.said() != app.bar_shown.said() {
        ctx.request_repaint();
    }
    app.bar_shown = next;
}

/// Whether something in front takes the keys and presses the views'
/// would: an `egui::Modal` (a job's window, a plugin's question), a
/// dialog, a menu or popup.
pub(crate) fn keys_blocked(ctx: &egui::Context) -> bool {
    ctx.memory(|m| m.top_modal_layer()).is_some()
        || crate::widgets::dialog_open(ctx)
        || egui::Popup::is_any_open(ctx)
}

/// What the Status Bar shows for the active docked editor
/// ([`Moonglow::active_editor`], as the Right Dock follows it), or for the
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
) -> Shown {
    use crate::Tab;
    if let Some(shown) = &app.status_for_tests {
        return shown.clone();
    }
    use crate::keys::Cmd;
    let k = HintKeys::new(&app.keymap, ctx);
    let active = app.active_editor();
    let esc_mine = escape.is_some_and(|(t, _)| Some(t) == active);
    let mut shown = match active {
        Some(Tab::Area(area)) => {
            let mut shown = (app.area_views.get(area))
                .map(|v| crate::area_view::status(app, v, ctx))
                .unwrap_or_default();
            // (A modal, a dialog or a menu in front: the view's keys and
            // presses do nothing, `response.hovered()` false. The counts
            // stay.)
            if keys_blocked(ctx) {
                shown.hints.clear();
            }
            shown
        }
        Some(Tab::Welcome) => {
            let hints = [
                k.hint(Cmd::NewModule, "New module"),
                k.hint(Cmd::OpenModule, "Open module"),
                k.hint(Cmd::QuickOpen, "Go to resource or command"),
            ];
            Shown { hints: hints.into_iter().flatten().collect(), counts: Vec::new() }
        }
        Some(Tab::Script(_)) => {
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
            Shown { hints: hints.into_iter().flatten().collect(), counts: Vec::new() }
        }
        Some(tab) if tab.escape_cancels() => {
            let mut hints = Vec::new();
            let typing = ctx.memory(|m| m.focused().is_some());
            // (Not while Options › Keyboard waits for a key: it takes the
            // keys itself.)
            let recording = app.options.as_ref().is_some_and(|o| o.recording.is_some());
            // (Nor while a list drawn before the editor area has the keys:
            // the module tree takes Enter first. A dropdown open has the
            // keyboard: `typing`.)
            let list = crate::palette_view::a_list_has_the_keys(ctx);
            if *tab == Tab::Options && options_hovered && !typing && !recording && !list {
                hints.push(Hint::keys(["Enter"], "OK"));
            }
            if esc_mine {
                hints.push(Hint::keys(["Esc"], "Cancel"));
            }
            hints.push(Hint::keys(["Tab"], "Next field"));
            hints.push(Hint::keys(["Shift", "Tab"], "Previous field"));
            Shown { hints, counts: Vec::new() }
        }
        _ => Shown::default(),
    };
    if let Some((_, name)) = escape.filter(|_| !esc_mine) {
        shown.hints.insert(0, Hint::keys(["Esc"], format!("Cancel {name}")));
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
    }
    shown
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

/// Each dock's sizes at first, (tree, Right Dock, log), that versions
/// before this one kept as though chosen while nothing was dragged: the
/// module tree's 260 (this branch, M3 T5 to T13: its default stored on
/// the first frame the dock was drawn) or 240 (every release to 4.2.0:
/// the default its egui panel kept, which this branch then stored in
/// place of its own); the Right Dock's 320 (this branch alone: no release
/// had it); the log's 180 (this branch) or 120 (every release). Each was
/// written apart, where the dock was drawn on that first frame, so an
/// untouched file holds any of them in any mix, (240, 320, 180) and (260,
/// 320, 120) among them, or one alone (the tree folded, no area open for
/// the Right Dock, and since M3 T7 a new installation's log closed).
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
    use egui::containers::panel::PanelState;
    let id = egui::Id::new(id);
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
    let held = panels(app, Dock::Left);
    let Some(front) = left_shown(app) else {
        return;
    };
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
                    let key = if *p == front { fold_key.as_str() } else { "" };
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
    let window = ctx.content_rect().size();
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
    // Drawn at the width chosen (else the layout's: 260, 320 in a wide
    // window), held to what the window allows: at most two fifths of it,
    // and the editor area left its least beside the Right Dock (where the
    // dock is not unfolded by hand in a window too narrow for that); the
    // width chosen changes only as the edge is dragged.
    let beside = right_reserved(app, window.x) + kit::EDITOR_LEAST;
    let most = (window.x * 0.4).min(window.x - beside).max(LEFT_DOCK_LEAST);
    let range = LEFT_DOCK_LEAST..=most;
    let wide = chosen_size(app.settings.left_dock_width, app.yields.left_dock_width);
    draw_at(&ctx, "tree", 0, wide.clamp(*range.start(), *range.end()));
    let dragged = edge_dragged(&ctx, "tree");
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
            let at_first = app.yields.module_panel_height;
            let chosen = app.settings.module_panel_height.map_or(at_first, |h| h as f32);
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
                app.settings.module_panel_height = Some(top.response.rect.height().round() as u32);
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
    // (Where it and the panels' late bodies are drawn, this frame's alone:
    // a dock that folds or goes leaves none behind.)
    app.right_rect = None;
    app.preview_rect = None;
    app.late_slots.clear();
    app.right_columns = false;
    app.right_bars.clear();
    let sections = right_sections(app);
    let folded_as = editor_brings(app).folded;
    let held = folded_order(&sections, folded_as);
    let Some(front) = right_shown(app) else {
        return;
    };
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let line = panel_line(ui);
    if right_folded(app) {
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
    let wide = chosen_size(*right_dock_chosen(app), app.yields.right_dock_width);
    let drawn = if narrow { least } else { wide.clamp(*range.start(), *range.end()) };
    draw_at(&ctx, RIGHT_DOCK_ID, 0, drawn);
    let resizable = !narrow;
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
    let shown = panel.show(ui, |ui| {
        dock_pane(ui);
        let pane = ui.max_rect();
        let pane_id = ui.unique_id();
        let columns = yields.right_dock_columns(pane.width());
        let layout = arrange(&sections, folded_as, narrow, columns);
        // (The pane last clicked in, whose selection a plugin's command
        // from the Plugins menu or a key runs on: the panel's editor's.)
        if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(pane) {
            app.plugins.pane = plugin_pane(front, app.active_editor()).or(app.plugins.pane);
        }
        // One tab bar: the panel in front under it.
        if let [one] = layout.as_slice()
            && let [bar_panels] = one.as_slice()
        {
            let front =
                if bar_panels.contains(&front) { front } else { section_front(app, bar_panels) };
            let mut tab = bar_panels.iter().position(|p| *p == front).unwrap_or(0);
            let mut tabs = DockTabs::new(RIGHT_DOCK).tools_width(tools_width);
            for p in bar_panels {
                tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
            }
            let bar = tabs.show(ui, &mut tab, |ui| {
                kit::fold_button_tip(ui, Side::Right, RIGHT_DOCK, &fold).clicked()
            });
            // (A tab chosen shows its panel from the next frame: this one
            // draws the panel in front as the frame began.)
            let chosen = bar.changed.then(|| bar_panels[tab]);
            app.right_bars.push(front);
            panel_body(app, ui, front, Host::Dock, pane_id, editor.as_ref());
            return (chosen, None, bar.inner);
        }
        // In the wide layout, a dock wider than two narrow ones stands in
        // two columns (WideArea's `.dock.cols`), a line between them: the
        // first (the board's 340, half the dock where that is less) and
        // the second, each its sections stacked, each section its own tab
        // bar. Else one column of stacked sections.
        let rects = match layout.len() {
            2 => {
                let first = Yields::right_first_column(pane.width());
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
                if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(place) {
                    pressed = Some(front);
                    app.plugins.pane = plugin_pane(front, app.active_editor()).or(app.plugins.pane);
                }
                let builder = egui::UiBuilder::new().id_salt(&name).max_rect(place);
                let mut part = ui.new_child(builder);
                part.set_clip_rect(place.intersect(ui.clip_rect()));
                let mut tab = section.iter().position(|p| *p == front).unwrap_or(0);
                let mut tabs = DockTabs::new(&name);
                for p in section {
                    tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
                }
                // (The dock's fold button at the outer end of the last
                // column's top bar, by the window's edge: where it is in
                // one column, and not mid-dock, where WideArea draws none.)
                let fold_here = last_column && s == 0;
                if fold_here {
                    tabs = tabs.tools_width(tools_width);
                }
                let bar = tabs.show(&mut part, &mut tab, |ui| {
                    fold_here && kit::fold_button_tip(ui, Side::Right, RIGHT_DOCK, &fold).clicked()
                });
                folded |= bar.inner;
                if bar.changed {
                    chosen = Some(section[tab]);
                }
                app.right_bars.push(front);
                panel_body(app, &mut part, front, Host::Dock, pane_id, editor.as_ref());
                index += 1;
            }
            section_seams(app, ui, &places, column, stack, &format!("right-dock-column-{c}"));
        }
        ui.advance_cursor_after_rect(pane);
        app.right_columns = true;
        (chosen, pressed, folded)
    });
    let dock = shown.response.rect;
    if dragged {
        *right_dock_chosen(app) = Some(dock.width().round() as u32);
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

/// The plugins' "pane last clicked in" for a press in `panel`: a panel an
/// editor brings counts as that editor's pane (U2, `17-plugin-api.md`):
/// the Palette's and the Preview's the palette (the Preview shows the
/// palette's blueprint), another an area's (`editor`, the editor it
/// draws for: the active docked one, or the floating one in its window)
/// the area's; an editor with no pane of its own for plugins leaves it as
/// it was (`None`).
fn plugin_pane(panel: Panel, editor: Option<&crate::Tab>) -> Option<crate::plugins::Pane> {
    match panel {
        Panel::Palette | Panel::Preview => Some(crate::plugins::Pane::Palette),
        _ => match editor {
            Some(crate::Tab::Area(area)) => Some(crate::plugins::Pane::Area(*area)),
            _ => None,
        },
    }
}

/// The least height of a stacked section of the Right Dock: its tab bar
/// and a few rows of its panel.
pub(crate) const SECTION_LEAST: f32 = 80.0;

/// Whose stacked sections they are ([`section_rects`], [`section_seams`]):
/// the editor's kind and what it brings, by which their shares are kept.
#[derive(Clone, Copy)]
struct Stack<'a> {
    kind: Option<&'static str>,
    brings: &'a Brings,
}

impl Stack<'_> {
    /// A section's share as kept (`Settings::dock_layouts`, by its place
    /// among the kind's own: [`own_index`]), 1000 where none is.
    fn share(&self, app: &Moonglow, section: &[Panel]) -> f32 {
        let layout = self.kind.and_then(|k| app.settings.dock_layouts.get(k));
        let kept = layout.zip(own_index(self.brings, section)).and_then(|(l, i)| l.shares.get(i));
        kept.map_or(1000.0, |s| f32::from((*s).max(1)))
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
    let heights = section_heights(column.height() - (count - 1) as f32, &shares);
    let mut rects = Vec::with_capacity(count);
    let mut y = column.top();
    for h in &heights {
        rects.push(egui::Rect::from_x_y_ranges(column.x_range(), y..=y + h));
        y += h + 1.0;
    }
    rects
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
    let t = mg_uikit::tokens(ui.ctx());
    for i in 0..rects.len().saturating_sub(1) {
        let top = rects[i].bottom();
        let line = egui::Rect::from_x_y_ranges(rects[i].x_range(), top..=top + 1.0);
        ui.painter().rect_filled(line, 0.0, t.line);
        let grip = line.expand2(egui::vec2(0.0, 3.0));
        let seam = ui.interact(grip, egui::Id::new((salt, "seam", i)), egui::Sense::drag());
        if seam.hovered() || seam.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        let (Some(at), Some(kind)) = (seam.interact_pointer_pos(), stack.kind) else { continue };
        if !seam.dragged() {
            continue;
        }
        let pair = rects[i].height() + rects[i + 1].height();
        let least = SECTION_LEAST.min(pair / 2.0);
        let upper = (at.y - rects[i].top()).clamp(least, pair - least);
        let mut now: Vec<f32> = rects.iter().map(egui::Rect::height).collect();
        now[i] = upper;
        now[i + 1] = pair - upper;
        let total: f32 = now.iter().sum();
        let count = now.len() as f32;
        let own: Vec<Option<usize>> = sections.iter().map(|s| own_index(stack.brings, s)).collect();
        let layout = app.settings.dock_layouts.entry(kind.to_string()).or_default();
        if layout.shares.len() < stack.brings.right.len() {
            layout.shares.resize(stack.brings.right.len(), 1000);
        }
        for (h, index) in now.iter().zip(own) {
            if let Some(index) = index {
                layout.shares[index] = ((h / total) * 1000.0 * count).round().max(1.0) as u16;
            }
        }
    }
}

/// The heights of stacked sections in `room` (the lines between them left
/// out) by their `shares`: each its share of the room, at least
/// [`SECTION_LEAST`] where the room has that for each, the last taking
/// what is left so that they fill it in whole points.
pub(crate) fn section_heights(room: f32, shares: &[f32]) -> Vec<f32> {
    let count = shares.len();
    if count == 0 {
        return Vec::new();
    }
    let total: f32 = shares.iter().sum::<f32>().max(f32::EPSILON);
    let least = SECTION_LEAST.min(room / count as f32);
    let mut heights = Vec::with_capacity(count);
    let mut left = room;
    for (i, share) in shares.iter().enumerate() {
        let after = (count - i - 1) as f32 * least;
        let h = match i + 1 == count {
            true => left,
            false => (room * share / total).round().clamp(least, (left - after).max(least)),
        };
        heights.push(h.max(0.0));
        left -= h;
    }
    heights
}

/// The Right Dock's width as chosen for the layout of this frame: the one
/// dragged to in the wide layout (from 2560, where it stands in two
/// columns wider than 560) apart from the one dragged to below it, so that
/// neither layout's choice is the other's (`Settings::right_dock_width`,
/// `right_dock_width_wide`).
fn right_dock_chosen(app: &mut Moonglow) -> &mut Option<u32> {
    match app.yields.wide {
        true => &mut app.settings.right_dock_width_wide,
        false => &mut app.settings.right_dock_width,
    }
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
/// docked editor in a dock, the floating editor in its window.
pub(crate) fn panel_body(
    app: &mut Moonglow,
    ui: &mut Ui,
    panel: Panel,
    host: Host,
    pane: egui::Id,
    editor: Option<&crate::Tab>,
) {
    let margin = Margin { left: 8, right: 8, top: 6, bottom: 4 };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        // (Laid out in the host's room and cut at its edges: a widget
        // wider than the dock never widens it, which would push its edge
        // past the window's and draw the editor area over it.)
        let room = ui.available_rect_before_wrap();
        match panel {
            Panel::Palette => {
                let mut body = ui.new_child(egui::UiBuilder::new().max_rect(room));
                body.set_clip_rect(room.intersect(ui.clip_rect()));
                crate::palette_view::ui(app, &mut body);
            }
            // (Drawn here, where it comes in Tab's order after the tab
            // bar; where an editor's tab may show the same model, after
            // the editor area in this room, so that the tab keeps its
            // view.)
            Panel::Preview if room.is_positive() => {
                if host == Host::Dock {
                    app.preview_rect = Some(room);
                }
                let chosen = app.palette.selected.map(crate::model_view::Source::Resource);
                if chosen.is_some_and(|s| crate::model_view::a_tab_may_show(app, &s)) {
                    let layer = ui.layer_id();
                    let editor = editor.cloned();
                    app.late_slots.push(LateSlot { panel, rect: room, layer, pane, host, editor });
                } else {
                    let builder = egui::UiBuilder::new().max_rect(room);
                    let builder = LateSlot::salted(builder, host, "preview-panel", pane);
                    let mut body = ui.new_child(builder);
                    body.set_clip_rect(room.intersect(ui.clip_rect()));
                    crate::area_tools::preview_body(app, &mut body, room);
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
    let kept = match app.yields.wide {
        true => app.settings.right_dock_width_wide,
        false => app.settings.right_dock_width,
    };
    let wide = chosen_size(kept, app.yields.right_dock_width);
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
    // (The tab bar, a line of the log and the room round it.)
    let least = metrics::DOCK_TABS + line + 8.0 + panel_line(ui);
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
            let mut tabs = DockTabs::new(BOTTOM_DOCK).tools_width(button * most as f32);
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
                chosen
            });
            // (A tab chosen shows its panel from the next frame.)
            let picked = (bar.changed && tab_fits).then(|| held[tab]);
            app.bottom_rect = Some(ui.max_rect());
            let margin = Margin { left: 8, right: 8, top: 4, bottom: 4 };
            match front {
                Panel::Log => {
                    egui::Frame::new().inner_margin(margin).show(ui, |ui| app.log_ui(ui));
                }
                _ => panel_body(app, ui, front, Host::Dock, pane_id, editor.as_ref()),
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
        assert_eq!(tabs_of(&h, BOTTOM_DOCK), ["Sample 8", "Sample 9", "Log"]);
        assert_eq!(switches_shown(&h), ["Log", "Sample 8", "Sample 9"]);
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
        assert_eq!(switches_shown(&h), ["Log"], "the window's own panels have no switch");
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
        let welcome = crate::Tab::Welcome;
        assert_eq!(plugin_pane(Panel::Palette, Some(&welcome)), Some(Pane::Palette));
        assert_eq!(plugin_pane(Panel::Preview, None), Some(Pane::Palette));
        assert_eq!(plugin_pane(S0, Some(&welcome)), None, "the Welcome page has no pane");
        let area = mg_core::ResRef::from_str("area001").unwrap();
        let tab = crate::Tab::Area(area);
        assert_eq!(plugin_pane(S0, Some(&tab)), Some(Pane::Area(area)));
        assert_eq!(plugin_pane(Panel::Preview, Some(&tab)), Some(Pane::Palette));
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
        assert_eq!(owner_of(&app, &crate::Tab::Manual), &crate::Tab::Manual);
    }
}
