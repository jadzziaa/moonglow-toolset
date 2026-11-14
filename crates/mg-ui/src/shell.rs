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
//! - the **Right Dock** ([`right_dock`]): the active docked editor's
//!   panels on the tabs of one bar: the Palette panel (the blueprint and
//!   tile palettes, `palette_view`: M3's T5) and the Preview panel (the
//!   blueprint chosen in the palette: `area_tools::preview_panel`, T6);
//!   drawn only where the active editor brings them (an area's tab, or the
//!   placeholder in its place) or the Palette was asked for by name; its
//!   fold button at its tab bar's end by the window's edge (in two
//!   columns, the last column's bar); folded, a strip with the chevron
//!   that brings it back at its top and a button for each panel;
//! - the **Bottom Dock** ([`bottom_dock`]): its tab bar (the Log's tab,
//!   with the log's Copy, Save… and Clear), the log under it; opened and
//!   closed by the Log's switch in the Status Bar (issue 10's one control
//!   per dock: the switch is the Bottom Dock's).
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
/// and the dock each lives in, fixed there by the owner's ruling R5). The
/// Left Dock holds Module and Resources, in that order ([`Panel::LEFT`]);
/// the Right Dock Palette and Preview, as tabs of one bar
/// ([`Panel::RIGHT`]), the panels the active docked editor brings ([`Panel::available`]: an
/// area's, or the placeholder that says none is open). Under the Left
/// Dock's tabs, split from them by a seam, the Outliner (T13), which the
/// active editor brings too (an area's tab: [`editor_brings_outliner`]):
/// not a tab, a panel of its own under the one shown. A dock whose panels
/// none is available is not drawn.
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
}

/// A dock that holds panels ([`Panel::dock`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dock {
    Left,
    Right,
}

impl Panel {
    /// The Left Dock's panels, in the order of its tabs (the Outliner is
    /// under them, no tab: [`Panel::OUTLINER`]).
    pub(crate) const LEFT: [Panel; 2] = [Panel::Module, Panel::Resources];
    /// The Left Dock's panel under its tabs, split from them.
    pub(crate) const OUTLINER: Panel = Panel::Outliner;
    /// The Right Dock's panels, in the order of its tabs.
    pub(crate) const RIGHT: [Panel; 2] = [Panel::Palette, Panel::Preview];

    /// The dock it lives in.
    pub(crate) fn dock(self) -> Dock {
        match self {
            Panel::Module | Panel::Resources | Panel::Outliner => Dock::Left,
            Panel::Palette | Panel::Preview => Dock::Right,
        }
    }

    /// Its name: its tab's words, its strip button's name.
    pub fn name(self) -> &'static str {
        match self {
            Panel::Module => "Module",
            Panel::Resources => "Resources",
            Panel::Palette => "Palette",
            Panel::Preview => "Preview",
            Panel::Outliner => "Outliner",
        }
    }

    /// Its icon on its dock's strip, folded (`Narrow.html`'s rail), and on
    /// its tab where the dock is too narrow for the tabs' words.
    fn icon(self) -> Icon {
        match self {
            Panel::Module => Icon::Module,
            Panel::Resources => Icon::Folder,
            Panel::Palette => Icon::Palette,
            Panel::Preview => Icon::Eye,
            Panel::Outliner => Icon::Outliner,
        }
    }

    /// How many header buttons it has on its dock's tab bar: the Module
    /// panel's Module Properties, New Area, Expand All and Collapse All;
    /// the Resources panel's Export and Refresh; none of the Palette
    /// panel's and the Preview panel's (their tools are their own rows,
    /// as the boards draw them).
    fn tools(self) -> usize {
        match self {
            Panel::Module => 4,
            Panel::Resources => 2,
            // (Its Expand All and Collapse All are on its own header.)
            Panel::Palette | Panel::Preview | Panel::Outliner => 0,
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
        }
    }

    /// Whether it has something to show now: the module tree a module
    /// open; the resource browser a module open, or (without one) once it
    /// was asked for (Tools › Resource Browser), for the run; the palettes
    /// where the active editor brings them ([`editor_brings_palette`]: an
    /// area's tab or the placeholder in front, the game's data read), or
    /// where they were asked for by name ([`Moonglow::palettes_asked`]:
    /// the game's standard palettes without a module, or with no game data
    /// read the panel saying so, beside any tab), unless a test turned
    /// them off (`Moonglow::palette_dock`); the Preview with the palettes,
    /// on the tab beside theirs; the Outliner where the active editor
    /// brings it ([`editor_brings_outliner`]: an area's tab in front).
    pub(crate) fn available(self, app: &Moonglow) -> bool {
        match self {
            Panel::Module => app.ws.is_some(),
            Panel::Resources => app.ws.is_some() || app.resources_alone,
            Panel::Palette => {
                app.palette_dock && (app.palettes_asked || editor_brings_palette(app))
            }
            Panel::Preview => Panel::Palette.available(app),
            Panel::Outliner => editor_brings_outliner(app),
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
        }
    }
}

/// Whether the active docked editor ([`Moonglow::active_editor`], as
/// [`editor_brings_palette`] reads it, in either mode of the Open editors
/// switch) brings the Outliner: an area's tab, with a module open (the
/// game's data or not: what is placed is read from the module). The
/// placeholder that stands for no area brings none: it holds nothing to
/// list, and no panel is shown empty. Any other tab in front brings none
/// (M4 gives the other editors theirs): the Left Dock is then its tabs'
/// panel alone, the whole height.
pub(crate) fn editor_brings_outliner(app: &Moonglow) -> bool {
    app.ws.is_some() && matches!(app.active_editor(), Some(crate::Tab::Area(_)))
}

/// Whether the active docked editor ([`Moonglow::active_editor`]) brings
/// the Right Dock's panels, the Palette and the Preview: an area's tab, or
/// the placeholder that stands in its place where none is open, with the
/// game's data read (the palettes are its). Any other tab in front (Module
/// Properties, a script, a blueprint, the manual) brings none: the Right
/// Dock is then not drawn, folded or not, its sizes kept.
pub(crate) fn editor_brings_palette(app: &Moonglow) -> bool {
    app.game.is_some()
        && app.ws.is_some()
        && matches!(app.active_editor(), Some(crate::Tab::Area(_) | crate::Tab::NoArea))
}

/// The docks follow the active editor: the Palette asked for by name
/// (Tools › Palettes beside a tab that brings none, or with no module) is
/// held until the active editor brings it itself; from then on the plain
/// rule has it ([`editor_brings_palette`]), and a tab that brings none
/// takes the Right Dock away again.
pub(crate) fn follow_editor(app: &mut Moonglow) {
    if app.palettes_asked && editor_brings_palette(app) {
        app.palettes_asked = false;
    }
}

/// The panels `dock` holds now, in their tabs' order (none: the dock is
/// not drawn).
pub(crate) fn panels(app: &Moonglow, dock: Dock) -> Vec<Panel> {
    let all: &[Panel] = match dock {
        Dock::Left => &Panel::LEFT,
        Dock::Right => &Panel::RIGHT,
    };
    all.iter().copied().filter(|p| p.dock() == dock && p.available(app)).collect()
}

/// The panel the Right Dock shows (its tab in front), of those it holds:
/// the one chosen last (`Moonglow::right_panel`), else its first. `None`
/// where it holds none (and is not drawn).
pub(crate) fn right_shown(app: &Moonglow) -> Option<Panel> {
    let held = panels(app, Dock::Right);
    held.contains(&app.right_panel).then_some(app.right_panel).or(held.first().copied())
}

/// The panel the Left Dock shows (its tab selected), of those it holds:
/// the one chosen last, else its first. `None` where it holds none.
pub(crate) fn left_shown(app: &Moonglow) -> Option<Panel> {
    let held = panels(app, Dock::Left);
    held.contains(&app.left_panel).then_some(app.left_panel).or(held.first().copied())
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
/// side dock panel holding them is not drawn this frame (its dock folded
/// by the window's width or by the builder, its dock holding no panel, as
/// beside a script's tab, or the Outliner no longer brought by the editor
/// in front), they go to the editor area's active editor
/// ([`Moonglow::editor_keys`]), and stay there when it is drawn again (the
/// orchestrator's decision of T14's review, reversible). Where its dock is
/// drawn on another of its tabs, they go with the tab, to that tab's
/// panel, in either side dock: the Left Dock's tab bar hands them over as
/// it draws ([`left_dock`]), the Right Dock's here (the Palette's to the
/// Preview and back). Run once a frame before the docks. (Whether the
/// Right Dock stands in two columns, each drawing its panel, is the last
/// frame's: in two columns the keys' column is the dock's panel,
/// [`Moonglow::right_panel`], so the frame it changes moves no keys.)
pub(crate) fn keys_leave_panels_not_drawn(app: &mut Moonglow, ctx: &egui::Context) {
    use crate::palette_view::{Keys, give_keys, keys_are};
    let left = left_shown(app).filter(|_| !left_folded(app));
    let right = right_shown(app).filter(|_| !right_folded(app));
    let drawn = |keys: Keys| match keys {
        // (The Left Dock drawn on its other tab: its tab bar hands them
        // to that tab's panel, the tree's to the browser's list and back,
        // as it draws, [`left_dock`].)
        Keys::Tree | Keys::Browser => left.is_some(),
        Keys::Outliner => left.is_some() && Panel::OUTLINER.available(app),
        Keys::Palette | Keys::Preview => {
            right.is_some_and(|p| p.keys() == keys || app.right_columns)
        }
        _ => true,
    };
    let panels = [Keys::Tree, Keys::Browser, Keys::Outliner, Keys::Palette, Keys::Preview];
    let held = panels.into_iter().find(|k| keys_are(ctx, *k));
    let Some(held) = held.filter(|k| !drawn(*k)) else { return };
    // (The Right Dock on its other tab: to that tab's panel.)
    if let Some(shown) = right.filter(|_| matches!(held, Keys::Palette | Keys::Preview)) {
        give_keys(ctx, shown.keys());
        app.palette.to_first |= shown == Panel::Palette;
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
/// Where the Preview panel's body is drawn in this frame
/// ([`crate::area_tools::preview_panel`], after the editor area): its
/// rectangle in the Right Dock, the dock's layer, and the dock's panel
/// (its pane in the accessibility tree) that its widgets are under.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PreviewSlot {
    pub(crate) rect: egui::Rect,
    pub(crate) layer: egui::LayerId,
    pub(crate) pane: egui::Id,
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
    let log_on = !app.settings.hide_log;
    let log_tip = format!("Show the log, or close it{}", key_note(app, &ctx, Id::ViewLog));
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
            let switch = StatusSwitch::new(Icon::Log, "Log", log_on).tooltip(log_tip);
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
            let icons =
                yields.status_switch_icons || switch.width(ui) + gaps + full > ui.available_width();
            let switch = switch.icon_only(icons);
            let left = switch.width(ui) + gaps;
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
                |ui| ui.add(switch).clicked(),
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
    if clicked {
        app.show_panel(Id::ViewLog, !log_on);
    } else if last_clicked {
        app.show_panel(Id::ViewLog, true);
    }
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
/// ([`editor_brings_outliner`]), the Outliner under them, split by a seam
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
    // is shown, so that the tabs keep their form (words, or their icons
    // alone where the words and those tools do not fit) and their place
    // when another tab is chosen: the dock's width alone decides it.
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
        Panel::Palette | Panel::Preview | Panel::Outliner => {}
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
/// docked editor brings panels (an area's tab or the placeholder in front:
/// the Palette and the Preview) or the Palette was asked for by name
/// ([`Panel::available`]); not drawn at all where it holds none (no strip,
/// no fold button: its fold and its width kept for the next area). Its tab
/// bar (the kit's `DockTabs`: Palette and Preview, as the Main board draws
/// them; their icons alone where the dock is too narrow for their words,
/// by its width alone, as the Left Dock's) and under it the panel in
/// front, filling the dock: the Palette (`palette_view::ui`) or the
/// Preview (its body drawn after the editor area,
/// `area_tools::preview_panel`, in the room kept for it here). 320 px at
/// first, at least [`RIGHT_DOCK_LEAST`], at most two fifths of the window
/// (as the Left Dock), resized by its inner edge; its fold button at its
/// tab bar's end by the window's edge (in two columns, the last column's
/// bar). Folded (`Settings::hide_palettes`, or by the window's width), the
/// strip: the chevron that unfolds it at its top, then its panels' icons,
/// each unfolding it on its panel. A press in the dock
/// makes the palette the plugins' "pane last clicked in"
/// (`plugins::Pane::Palette`), as a press in the palettes' tab did.
pub(crate) fn right_dock(app: &mut Moonglow, ui: &mut Ui) {
    follow_editor(app);
    // (Where it and the Preview's body are drawn, this frame's alone: a
    // dock that folds or goes leaves none behind.)
    app.right_rect = None;
    app.preview_rect = None;
    app.preview_slot = None;
    app.right_columns = false;
    let held = panels(app, Dock::Right);
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
                Panel::Preview => key_note(app, &ctx, Id::PreviewWindow),
                _ => key_note(app, &ctx, Id::Palettes),
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
    // panels' it holds (neither has any) and the dock's fold button after
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
    let shown = panel.show(ui, |ui| {
        dock_pane(ui);
        let pane = ui.max_rect();
        let pane_id = ui.unique_id();
        // (The pane last clicked in, whose selection a plugin's command
        // from the Plugins menu or a key runs on: the palette's, the
        // Preview's being the palette's chosen blueprint.)
        if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(pane) {
            app.plugins.pane = Some(crate::plugins::Pane::Palette);
        }
        // In the wide layout, a dock wider than two narrow ones stands in
        // two columns (WideArea's `.dock.cols`), each panel in its own
        // with its own tab bar, a line between them: the Palette in the
        // first (the board's 340, half the dock where that is less), the
        // Preview in the second.
        if held.len() == 2 && yields.right_dock_columns(pane.width()) {
            let first = Yields::right_first_column(pane.width());
            let (one, rest) = pane.split_left_right_at_x(pane.left() + first);
            let two = rest.with_min_x(rest.left() + 1.0);
            let line = egui::Rect::from_x_y_ranges(one.right()..=two.left(), pane.y_range());
            ui.painter().rect_filled(line, 0.0, t.line);
            // (A press in a column makes its panel the dock's, the one
            // View › Right Dock's strip and the keys come back to.)
            let (mut pressed, mut folded) = (None, false);
            for (rect, p, name) in [(one, held[0], RIGHT_DOCK), (two, held[1], RIGHT_COLUMN_TWO)] {
                if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(rect) {
                    pressed = Some(p);
                }
                let builder = egui::UiBuilder::new().id_salt(name).max_rect(rect);
                let mut column = ui.new_child(builder);
                column.set_clip_rect(rect.intersect(ui.clip_rect()));
                let mut tab = 0;
                let mut tabs = DockTabs::new(name).with(DockTab::new(p.name()).icon(p.icon()));
                // (The dock's fold button at the outer end of the last
                // column's bar, by the window's edge: where it is in one
                // column, and not mid-dock, where WideArea draws none.)
                let last = name == RIGHT_COLUMN_TWO;
                if last {
                    tabs = tabs.tools_width(tools_width);
                }
                let bar = tabs.show(&mut column, &mut tab, |ui| {
                    last && kit::fold_button_tip(ui, Side::Right, RIGHT_DOCK, &fold).clicked()
                });
                folded |= bar.inner;
                right_body(app, &mut column, p, pane_id);
            }
            ui.advance_cursor_after_rect(pane);
            app.right_columns = true;
            return (pressed, folded);
        }
        let mut tab = held.iter().position(|p| *p == front).unwrap_or(0);
        let mut tabs = DockTabs::new(RIGHT_DOCK).tools_width(tools_width);
        for p in &held {
            tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
        }
        let bar = tabs.show(ui, &mut tab, |ui| {
            kit::fold_button_tip(ui, Side::Right, RIGHT_DOCK, &fold).clicked()
        });
        // (A tab chosen shows its panel from the next frame: this one
        // draws the panel in front as the frame began.)
        let chosen = bar.changed.then(|| held[tab]);
        right_body(app, ui, front, pane_id);
        (chosen, bar.inner)
    });
    let dock = shown.response.rect;
    if dragged {
        *right_dock_chosen(app) = Some(dock.width().round() as u32);
    }
    app.right_rect = Some(dock);
    let (chosen, folded) = shown.inner;
    if folded {
        app.show_panel(Id::ViewPalettes, false);
    }
    if let Some(panel) = chosen {
        app.right_panel = panel;
    }
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

/// A Right Dock panel's body under its tab bar (`.panel-body`: 8 px round
/// it), filling what is left of `ui`: the Palette (`palette_view::ui`) or
/// the Preview (its body drawn after the editor area where an editor's tab
/// may show the same model, in the room kept for it here: [`PreviewSlot`]).
/// `pane` is the dock's panel (its widgets are under it for a screen
/// reader).
fn right_body(app: &mut Moonglow, ui: &mut Ui, front: Panel, pane: egui::Id) {
    let margin = Margin { left: 8, right: 8, top: 6, bottom: 4 };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        // (Laid out in the dock's room and cut at its edges: a widget
        // wider than the dock never widens it, which would push its edge
        // past the window's and draw the editor area over it.)
        let room = ui.available_rect_before_wrap();
        match front {
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
                app.preview_rect = Some(room);
                let chosen = app.palette.selected.map(crate::model_view::Source::Resource);
                if chosen.is_some_and(|s| crate::model_view::a_tab_may_show(app, &s)) {
                    let layer = ui.layer_id();
                    app.preview_slot = Some(PreviewSlot { rect: room, layer, pane });
                } else {
                    let builder = egui::UiBuilder::new().id_salt("preview-panel").max_rect(room);
                    let mut body = ui.new_child(builder);
                    body.set_clip_rect(room.intersect(ui.clip_rect()));
                    crate::area_tools::preview_body(app, &mut body, room);
                }
            }
            // (Not the Right Dock's.)
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
/// (`Settings::hide_log`): its tab bar (the Log's tab; Copy, Save… and
/// Clear at its right, which yield whole from the end where the dock is
/// too narrow for them beside the tab, the tab its icon alone), the log
/// under it, as low as the tab bar and a
/// line and at most half the window.
pub(crate) fn bottom_dock(app: &mut Moonglow, ui: &mut Ui) {
    app.bottom_rect = None;
    if app.settings.hide_log {
        return;
    }
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let window = ctx.content_rect().size();
    let line = ui.text_style_height(&egui::TextStyle::Monospace);
    // (The tab bar, a line of the log and the room round it.)
    let least = metrics::DOCK_TABS + line + 8.0 + panel_line(ui);
    let enabled = !app.log.entries.is_empty();
    let buttons = [
        (Id::CopyLog, Icon::Copy, "Copy Log", "Copy the whole log to the clipboard"),
        (Id::SaveLog, Icon::Save, "Save Log", "Save the whole log into a text file"),
        (Id::ClearLog, Icon::Eraser, "Clear Log", "Empty the log"),
    ];
    let tips: Vec<String> =
        buttons.iter().map(|b| format!("{}{}", b.3, key_note(app, &ctx, b.0))).collect();
    // (Drawn at the height chosen, held to what the window allows: as the
    // side docks' widths.)
    let range = least..=(window.y * 0.5).max(least);
    let high = chosen_size(app.settings.bottom_dock_height, metrics::BOTTOM_DOCK);
    draw_at(&ctx, "log", 1, high.clamp(*range.start(), *range.end()));
    let dragged = edge_dragged(&ctx, "log");
    let shown = egui::Panel::bottom("log")
        .resizable(true)
        .default_size(metrics::BOTTOM_DOCK)
        .size_range(range)
        .frame(egui::Frame::new().fill(t.panel))
        .show(ui, |ui| {
            dock_pane(ui);
            let mut tab = 0;
            // The Log's tab turns to its icon where its words and the
            // tools do not fit side by side; narrower still, the tools
            // yield whole from the end (Clear Log first), each still in
            // View's menu: the tab and the tools never overlap.
            let button = IconButton::new(Icon::Copy, "").icon_only(true).width(ui);
            let mut tabs = DockTabs::new(BOTTOM_DOCK)
                .tools_width(button * buttons.len() as f32)
                .with(DockTab::new("Log").icon(Icon::Log));
            let bar_width = ui.available_width();
            // (Narrower than its tab's icon, in a window too narrow for
            // the docks beside the editor area, the bar is drawn empty,
            // its tools gone first: the log under it, the Status Bar's
            // switch closing it.)
            let tab_fits = tabs.tabs_width(ui, true) <= bar_width;
            if !tab_fits {
                tabs = DockTabs::new(BOTTOM_DOCK);
            }
            let icons = tabs.icons_in(ui, bar_width);
            let room = bar_width
                - tabs.tabs_width(ui, icons)
                - kit::TOOLS_PADDING.0
                - kit::TOOLS_PADDING.1;
            // (The tools go before the tab does: none without it.)
            let fitting = if tab_fits {
                ((room / button).floor().max(0.0) as usize).min(buttons.len())
            } else {
                0
            };
            let bar = tabs.show(ui, &mut tab, |ui| {
                let mut chosen = None;
                for ((id, icon, name, _), tip) in buttons.iter().zip(&tips).take(fitting) {
                    let button = IconButton::new(*icon, *name).icon_only(true);
                    let r = ui.add_enabled(enabled, button);
                    if r.on_hover_text(tip).on_disabled_hover_text(tip).clicked() {
                        chosen = Some(*id);
                    }
                }
                chosen
            });
            app.bottom_rect = Some(ui.max_rect());
            let margin = Margin { left: 8, right: 8, top: 4, bottom: 4 };
            egui::Frame::new().inner_margin(margin).show(ui, |ui| app.log_ui(ui));
            bar.inner
        });
    if dragged {
        app.settings.bottom_dock_height = Some(shown.response.rect.height().round() as u32);
    }
    if let Some(id) = shown.inner {
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
