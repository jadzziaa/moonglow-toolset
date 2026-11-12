//! The main window's frame (the UI redesign's shell, proposal section 5
//! and its milestone M3), around the editor area that stays the one
//! egui_dock `DockArea` of [`Moonglow::ui`]:
//!
//! - the **Top Bar** ([`top_bar`]): the eight menus, the quick actions
//!   (New, Open, Save; Undo, Redo) as icons, and at the right end the
//!   build actions (Compile, Verify, Test Module with Choose Character
//!   under its arrow), from [`commands::TOP_BAR`]; the middle is kept for
//!   Quick Open (M3's T8);
//! - the **Status Bar** ([`status_bar`]): the Bottom Dock's switches at
//!   the left (the Log's), the module's name and the game folder at the
//!   right (hints and counts are T7 and T12's);
//! - the **Left Dock** ([`left_dock`]): its tab bar (the tabs of its
//!   panels, Module and Resources, and the shown panel's header buttons),
//!   the panel under it: the Module panel (the Filter over the module
//!   tree, whose Areas group scrolls in a well of its own: M3's T3) or the
//!   Resources panel (the resource browser: T4); the one button that
//!   folds the dock beside its inner edge, half way down; folded, a strip
//!   with a button for each panel, and beside its edge, where the fold
//!   button was, the button that brings it back. Which panels a dock
//!   holds is the registry's, [`Panel`];
//! - the **Right Dock** ([`right_dock`]): the active docked editor's
//!   panels on the tabs of one bar: the Palette panel (the blueprint and
//!   tile palettes, `palette_view`: M3's T5) and the Preview panel (the
//!   blueprint chosen in the palette: `area_tools::preview_panel`, T6);
//!   drawn only where the active editor brings them (an area's tab, or the
//!   placeholder in its place) or the Palette was asked for by name; its
//!   fold button beside its inner edge, half way down, as the Left Dock's,
//!   mirrored; folded, a strip with a button for each panel and the button
//!   that brings it back beside its edge;
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
//! is "the Left Dock folded", `Settings::hide_palettes` "the Right Dock
//! folded" and `Settings::hide_log` "the Bottom Dock closed".
//!
//! Later tasks fill it: the Log's last line and the jobs in the Status
//! Bar (T7), Quick Open in the Top Bar's middle (T8), hints (T12), the
//! Left Dock's Outliner (T13, which the area brings as it brings the Right
//! Dock's panels) and the widths the shell yields at (T14; T2 takes only
//! the build actions' words, below 1300).

use egui::{Margin, Ui};
use mg_uikit::shell::{self as kit, DockTab, DockTabs, FoldStrip, Side, StripClick, Yields};
use mg_uikit::status::{self, StatusItem, StatusSwitch};
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
/// area's, or the placeholder that says none is open). The Left Dock's
/// Outliner is added by the task that makes it (T13), in its dock's list,
/// with its dock in [`Panel::dock`] and a rule in [`Panel::available`]. A
/// dock whose panels none is available is not drawn.
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
}

/// A dock that holds panels ([`Panel::dock`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dock {
    Left,
    Right,
}

impl Panel {
    /// The Left Dock's panels, in the order of its tabs.
    pub(crate) const LEFT: [Panel; 2] = [Panel::Module, Panel::Resources];
    /// The Right Dock's panels, in the order of its tabs.
    pub(crate) const RIGHT: [Panel; 2] = [Panel::Palette, Panel::Preview];

    /// The dock it lives in.
    pub(crate) fn dock(self) -> Dock {
        match self {
            Panel::Module | Panel::Resources => Dock::Left,
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
            Panel::Palette | Panel::Preview => 0,
        }
    }

    /// Its strip button's tip, without the keys.
    fn strip_tip(self) -> &'static str {
        match self {
            Panel::Module => "Module: the module tree",
            Panel::Resources => "Resources: the resource browser",
            Panel::Palette => "Palette: the blueprint and tile palettes",
            Panel::Preview => "Preview: the blueprint chosen in the palette",
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
    /// on the tab beside theirs.
    pub(crate) fn available(self, app: &Moonglow) -> bool {
        match self {
            Panel::Module => app.ws.is_some(),
            Panel::Resources => app.ws.is_some() || app.resources_alone,
            Panel::Palette => {
                app.palette_dock && (app.palettes_asked || editor_brings_palette(app))
            }
            Panel::Preview => Panel::Palette.available(app),
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
        }
    }
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
    let window = ctx.content_rect().width();
    // (The editor's width as it was last drawn: only the window's is read
    // in T2, for the build actions' words.)
    let yields = Yields::of(window, app.dock_width.unwrap_or(window));
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
    // (Decided once the menus are laid out, for both groups: the quick
    // actions where they fit after the menus, the build actions only where
    // both fit, so a window that narrows loses the build actions first.)
    let show_build = std::cell::Cell::new(false);
    let (quick_clicked, (), build_clicked) = kit::top_bar(
        ui,
        yields.quick_open_cap,
        |ui| {
            commands::menu_bar(app, ui);
            let mut clicked = None;
            let menus_end = ui.cursor().min.x + kit::TOP_BAR_GAP;
            let right = ui.max_rect().right();
            let show_quick = menus_end + quick_width <= right;
            show_build.set(show_quick && menus_end + quick_width + build_width <= right);
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
        // (Quick Open's place, M3's T8: nothing until then.)
        |_, _| {},
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
    if let Some(id) = quick_clicked {
        id.run(app, &ctx);
    }
    if let Some(id) = build_clicked {
        id.run(app, &ctx);
    }
}

/// The Status Bar (`Panel::bottom("status")`): the Log's switch at the
/// left; the module's name (with "(modified)", its path in the tip) and
/// the game folder at the right, the folder left out where both do not
/// fit beside the switch, and then the name cut short with an ellipsis
/// ("(modified)" kept whole).
pub(crate) fn status_bar(app: &mut Moonglow, ui: &mut Ui) {
    let t = mg_uikit::tokens(ui.ctx());
    let size = metrics::STATUS_BAR + panel_line(ui);
    let ctx = ui.ctx().clone();
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
    let clicked = egui::Panel::bottom("status")
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
            // (Its whole words in its tip, where it is cut short.)
            let note = looking.map(|n| StatusItem::new(n).tooltip(n));
            // The right-hand items share the room left after the switch
            // and the gap before them by one rule: the game folder goes
            // first; then the note and the name are cut short with an
            // ellipsis (`share`), each to half the room where both are
            // wider, or to what the other leaves (the name's path in its
            // tip, "(modified)" kept whole).
            let between = 2.0 * status::GAP + status::SEPARATOR_WIDTH;
            let left = switch.width(ui) + 2.0 * status::GAP;
            let room = ui.available_width() - left;
            let note_gap = if note.is_some() { between } else { 0.0 };
            let note_width = note.as_ref().map_or(0.0, |n| n.width(ui));
            let with_folder =
                item.width(ui) + note_width + note_gap + between + folder_item.width(ui);
            let show_folder = with_folder <= room;
            let mut note = note;
            if !show_folder {
                let (name_max, note_max) = share(room - note_gap, item.width(ui), note_width);
                item = item.max_width(name_max);
                note = note.map(|n| n.max_width(note_max));
            }
            let right = if show_folder {
                with_folder
            } else {
                item.width(ui) + note.as_ref().map_or(0.0, |n| n.width(ui)) + note_gap
            };
            let (clicked, (), ()) = status::bar_sized(
                ui,
                right,
                |ui| ui.add(switch).clicked(),
                // (The hints and the log's last line: T12 and T7.)
                |_, _| {},
                |ui| {
                    if let Some(note) = note {
                        ui.add(note);
                        status::separator(ui);
                    }
                    ui.add(item);
                    if show_folder {
                        status::separator(ui);
                        ui.add(folder_item);
                    }
                },
            );
            clicked
        })
        .inner;
    if clicked {
        app.show_panel(Id::ViewLog, !log_on);
    }
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

/// The size of the dock whose egui panel is `id` as the builder chose it
/// (`kept`, one of `Settings::left_dock_width`, `right_dock_width` and
/// `bottom_dock_height`), along `axis` (0 a width, 1 a height): where none
/// is chosen yet, the size egui kept for the panel (in an earlier run,
/// before the chosen sizes were kept apart from those drawn), taken once,
/// else `default`.
pub(crate) fn chosen_size(
    ctx: &egui::Context,
    kept: &mut Option<u32>,
    id: &str,
    axis: usize,
    default: f32,
) -> f32 {
    let size = *kept.get_or_insert_with(|| {
        let was = egui::containers::panel::PanelState::load(ctx, egui::Id::new(id));
        let size = was.map_or(default, |s| s.size()[axis]);
        size.round().max(0.0) as u32
    });
    size as f32
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
/// panel under it (`tree::module_tree`). Folded (`Settings::hide_tree`),
/// the strip of its panels' icons under the chevron that brings it back.
pub(crate) fn left_dock(app: &mut Moonglow, ui: &mut Ui) {
    let held = panels(app, Dock::Left);
    let Some(front) = left_shown(app) else {
        app.left_fold = None;
        return;
    };
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let line = panel_line(ui);
    let fold_key = key_note(app, &ctx, Id::ViewTree);
    if app.settings.hide_tree {
        // (The strip without a chevron of its own: the dock comes back by
        // the button beside its edge, where the one that folded it was.)
        let shown = egui::Panel::left("tree-strip")
            .exact_size(kit::STRIP + line)
            .resizable(false)
            .frame(egui::Frame::new().fill(t.bar))
            .show(ui, |ui| {
                let mut strip = FoldStrip::new(LEFT_DOCK, Side::Left).chevron(false);
                for p in &held {
                    // (The key unfolds the dock as it was, on the panel shown.)
                    let key = if *p == front { fold_key.as_str() } else { "" };
                    strip = strip.panel(p.icon(), p.name(), format!("{}{key}", p.strip_tip()));
                }
                strip.show(ui)
            });
        let strip = shown.response.rect;
        app.left_fold =
            Some(kit::edge_fold_rect_beside(strip.right(), strip.y_range(), Side::Left));
        let unfold = left_dock_fold(app, ui);
        match shown.inner {
            Some(StripClick::Panel(i)) => app.show_dock_panel(held[i]),
            Some(StripClick::Unfold) => app.show_panel(Id::ViewTree, true),
            None if unfold => app.show_panel(Id::ViewTree, true),
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
    let button = mg_uikit::IconButton::new(Icon::Plus, "").icon_only(true).width(ui);
    let most = held.iter().map(|p| p.tools()).max().unwrap_or(0);
    let tools_width = button * most as f32;
    // The fold button stands beside the dock's inner edge, where the panel
    // is resized from ([`left_dock_fold`]): under it (or pressed on it),
    // the button has the pointer, not the edge.
    let on_button = app.left_fold.is_some_and(|fold| {
        ctx.input(|i| match (i.pointer.any_down(), i.pointer.press_origin()) {
            (true, Some(origin)) => fold.contains(origin),
            _ => i.pointer.hover_pos().is_some_and(|p| fold.contains(p)),
        })
    });
    // Drawn at the width chosen, held to what the window allows; the
    // width chosen changes only as the edge is dragged.
    let range = LEFT_DOCK_LEAST..=(window.x * 0.4).max(LEFT_DOCK_LEAST);
    let wide = chosen_size(&ctx, &mut app.settings.left_dock_width, "tree", 0, metrics::LEFT_DOCK);
    draw_at(&ctx, "tree", 0, wide.clamp(*range.start(), *range.end()));
    let dragged = edge_dragged(&ctx, "tree");
    let shown = egui::Panel::left("tree")
        .resizable(!on_button)
        .default_size(metrics::LEFT_DOCK)
        .size_range(range)
        .frame(egui::Frame::new().fill(t.panel))
        .show(ui, |ui| {
            dock_pane(ui);
            let pane = ui.max_rect();
            let mut tab = held.iter().position(|p| *p == front).unwrap_or(0);
            let keymap = &app.keymap;
            let mut tabs = DockTabs::new(LEFT_DOCK).tools_width(tools_width);
            for p in &held {
                tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
            }
            // (The tools are the panel's shown: the Module panel's four, or
            // the Resources panel's Export and Refresh.)
            let bar = tabs.show(ui, &mut tab, |ui| match &browser {
                None => (module_tools(ui, &tools, keymap), None),
                Some(header) => ((None, None), Some(crate::browser::tools(ui, header))),
            });
            // (A tab chosen shows its panel from the next frame: this one
            // draws the panel the tools were made for.)
            let chosen = bar.changed.then(|| held[tab]);
            let ((_, folded), _) = bar.inner;
            match front {
                Panel::Module => {
                    // (The tree's rows reach the panel's sides, as the
                    // mock's; its Filter has a margin of its own.)
                    let margin = Margin { left: 0, right: 0, top: 2, bottom: 2 };
                    let body = egui::Frame::new().inner_margin(margin);
                    body.show(ui, |ui| tree::module_tree(app, ui, folded));
                }
                Panel::Resources => crate::browser::ui(app, ui, pane),
                // (Not the Left Dock's.)
                Panel::Palette | Panel::Preview => {}
            }
            (bar.inner, chosen)
        });
    app.tree_width = Some(shown.response.rect.width());
    if dragged {
        app.settings.left_dock_width = Some(shown.response.rect.width().round() as u32);
    }
    let (((clicked, _), browser_asked), chosen) = shown.inner;
    // (Its fold button beside its inner edge, half way down, the line
    // along the edge the dock's: after the dock's own controls for Tab.)
    let dock = shown.response.rect;
    app.left_fold = Some(kit::edge_fold_rect_beside(dock.right(), dock.y_range(), Side::Left));
    if left_dock_fold(app, ui) {
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

/// The Left Dock's fold button (issue 10's one button a pane has, on the
/// edge it faces the middle by, where builders knew it before the shell;
/// the boards draw no fold button on the tab bar), at `app.left_fold`:
/// beside the dock's inner edge, half way down, toward the editor area,
/// so that it is never over the dock's lists or their scroll bars;
/// folded, the button that unfolds it, beside the strip's edge at the
/// same height and size. Its key and tip are View › Left Dock's. Drawn
/// right after the dock, so that Tab reaches it after the dock's own
/// controls, on a layer of its own just over the window's panels (a
/// sublayer of the dock's), so that it is over the editor area beside
/// the dock and under any window floating over it. Whether it was
/// clicked.
fn left_dock_fold(app: &Moonglow, ui: &mut Ui) -> bool {
    let Some(rect) = app.left_fold else { return false };
    edge_fold(app, ui, rect, Side::Left, !app.settings.hide_tree)
}

/// A side dock's fold button, or folded the button that unfolds it, at
/// `rect` (beside the dock's inner edge, or its strip's): the dock at
/// `side`, `open` or folded. Its key and tip are View's row of the dock
/// (Left Dock, Right Dock). Drawn on a layer of its own just over the
/// window's panels (a sublayer of the dock's), so that it is over the
/// editor area beside the dock and under any window floating over it.
/// Whether it was clicked.
fn edge_fold(app: &Moonglow, ui: &mut Ui, rect: egui::Rect, side: Side, open: bool) -> bool {
    let ctx = ui.ctx().clone();
    let (dock, view, id) = match side {
        Side::Left => (LEFT_DOCK, Id::ViewTree, "left-dock-fold"),
        Side::Right => (RIGHT_DOCK, Id::ViewPalettes, "right-dock-fold"),
    };
    let key = key_note(app, &ctx, view);
    let tip = if open { format!("Fold the {dock}{key}") } else { format!("Show the {dock}{key}") };
    let area = egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Background)
        .fixed_pos(rect.min)
        .movable(false)
        .constrain(false);
    let parent = ui.layer_id();
    let shown = area.show(&ctx, |ui| {
        ui.set_min_size(rect.size());
        if open {
            kit::edge_fold_button(ui, rect, side, dock, &tip)
        } else {
            kit::edge_unfold_button(ui, rect, side, dock, &tip)
        }
        .clicked()
    });
    ctx.set_sublayer(parent, shown.response.layer_id);
    shown.inner
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
/// (as the Left Dock), resized by its inner edge; its fold button beside
/// that edge, half way down, as the Left Dock's, mirrored. Folded
/// (`Settings::hide_palettes`), the strip of its panels' icons, each
/// unfolding it on its panel, and the button that unfolds it beside the
/// strip's edge, where the one that folded it was. A press in the dock
/// makes the palette the plugins' "pane last clicked in"
/// (`plugins::Pane::Palette`), as a press in the palettes' tab did.
pub(crate) fn right_dock(app: &mut Moonglow, ui: &mut Ui) {
    follow_editor(app);
    // (Where it and the Preview's body are drawn, this frame's alone: a
    // dock that folds or goes leaves none behind.)
    app.right_rect = None;
    app.preview_rect = None;
    app.preview_slot = None;
    let held = panels(app, Dock::Right);
    let Some(front) = right_shown(app) else {
        app.right_fold = None;
        return;
    };
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let line = panel_line(ui);
    if app.settings.hide_palettes {
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
        let shown = egui::Panel::right("right-dock-strip")
            .exact_size(kit::STRIP + line)
            .resizable(false)
            .frame(egui::Frame::new().fill(t.bar))
            .show(ui, |ui| {
                let mut strip = FoldStrip::new(RIGHT_DOCK, Side::Right).chevron(false);
                for (p, key) in held.iter().zip(&keys) {
                    strip = strip.panel(p.icon(), p.name(), format!("{}{key}", p.strip_tip()));
                }
                strip.show(ui)
            });
        let strip = shown.response.rect;
        app.right_fold =
            Some(kit::edge_fold_rect_beside(strip.left(), strip.y_range(), Side::Right));
        let unfold = app.right_fold.is_some_and(|r| edge_fold(app, ui, r, Side::Right, false));
        match shown.inner {
            Some(StripClick::Panel(i)) => app.show_dock_panel(held[i]),
            Some(StripClick::Unfold) => app.show_panel(Id::ViewPalettes, true),
            None if unfold => app.show_panel(Id::ViewPalettes, true),
            None => {}
        }
        return;
    }
    let window = ctx.content_rect().size();
    // (Under its fold button, or pressed on it, the button has the
    // pointer, not the dock's edge.)
    let on_button = app.right_fold.is_some_and(|fold| {
        ctx.input(|i| match (i.pointer.any_down(), i.pointer.press_origin()) {
            (true, Some(origin)) => fold.contains(origin),
            _ => i.pointer.hover_pos().is_some_and(|p| fold.contains(p)),
        })
    });
    // (The tools' width the tabs are laid out beside: the widest of the
    // panels' it holds, whichever is in front, so that the tabs keep
    // their form by the dock's width alone. Neither has any: the tabs'
    // words go only where the bar is narrower than they are.)
    let tools_width = {
        let button = IconButton::new(Icon::Plus, "").icon_only(true).width(ui);
        button * held.iter().map(|p| p.tools()).max().unwrap_or(0) as f32
    };
    // (In a window too narrow for its least width beside the Left Dock,
    // as narrow as the room left: never past the window's edge, and the
    // editor area left at least the room of the two docks' fold buttons,
    // each beside its own dock. The editor area is then left little: the
    // shell's yields, M3 T14, fold the Left Dock in narrow windows.)
    let buttons = 2.0 * kit::EDGE_FOLD_WIDTH;
    let room = (ui.available_rect_before_wrap().width() - buttons).max(0.0);
    let least = RIGHT_DOCK_LEAST.min(room);
    let range = least..=(window.x * 0.4).max(least).min(room.max(least));
    // (Drawn at the width chosen, held to what the window allows: as the
    // Left Dock.)
    let wide = chosen_size(
        &ctx,
        &mut app.settings.right_dock_width,
        RIGHT_DOCK_ID,
        0,
        metrics::RIGHT_DOCK,
    );
    draw_at(&ctx, RIGHT_DOCK_ID, 0, wide.clamp(*range.start(), *range.end()));
    let dragged = edge_dragged(&ctx, RIGHT_DOCK_ID);
    let shown = egui::Panel::right(RIGHT_DOCK_ID)
        .resizable(!on_button)
        .default_size(metrics::RIGHT_DOCK.min(room))
        .size_range(range)
        .frame(egui::Frame::new().fill(t.panel))
        .show(ui, |ui| {
            dock_pane(ui);
            let pane = ui.max_rect();
            let pane_id = ui.unique_id();
            // (The pane last clicked in, whose selection a plugin's
            // command from the Plugins menu or a key runs on: the palette's,
            // the Preview's being the palette's chosen blueprint.)
            if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(pane) {
                app.plugins.pane = Some(crate::plugins::Pane::Palette);
            }
            let mut tab = held.iter().position(|p| *p == front).unwrap_or(0);
            let mut tabs = DockTabs::new(RIGHT_DOCK).tools_width(tools_width);
            for p in &held {
                tabs = tabs.with(DockTab::new(p.name()).icon(p.icon()));
            }
            let bar = tabs.show(ui, &mut tab, |_| ());
            // (A tab chosen shows its panel from the next frame: this one
            // draws the panel in front as the frame began.)
            let chosen = bar.changed.then(|| held[tab]);
            // The panel's body (`.panel-body`: 8 px round it).
            let margin = Margin { left: 8, right: 8, top: 6, bottom: 4 };
            egui::Frame::new().inner_margin(margin).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                // (Laid out in the dock's room and cut at its edges: a
                // widget wider than the dock never widens it, which would
                // push its edge past the window's and draw the editor area
                // over it.)
                let room = ui.available_rect_before_wrap();
                match front {
                    Panel::Palette => {
                        let mut body = ui.new_child(egui::UiBuilder::new().max_rect(room));
                        body.set_clip_rect(room.intersect(ui.clip_rect()));
                        crate::palette_view::ui(app, &mut body);
                    }
                    // (Drawn here, where it comes in Tab's order after the
                    // tab bar; where an editor's tab may show the same
                    // model, after the editor area in this room, so that
                    // the tab keeps its view.)
                    Panel::Preview if room.is_positive() => {
                        app.preview_rect = Some(room);
                        let chosen = app.palette.selected.map(crate::model_view::Source::Resource);
                        if chosen.is_some_and(|s| crate::model_view::a_tab_may_show(app, &s)) {
                            let layer = ui.layer_id();
                            app.preview_slot =
                                Some(PreviewSlot { rect: room, layer, pane: pane_id });
                        } else {
                            let builder =
                                egui::UiBuilder::new().id_salt("preview-panel").max_rect(room);
                            let mut body = ui.new_child(builder);
                            body.set_clip_rect(room.intersect(ui.clip_rect()));
                            crate::area_tools::preview_body(app, &mut body, room);
                        }
                    }
                    // (Not the Right Dock's.)
                    _ => {}
                }
            });
            chosen
        });
    let dock = shown.response.rect;
    if dragged {
        app.settings.right_dock_width = Some(dock.width().round() as u32);
    }
    app.right_rect = Some(dock);
    app.right_fold = Some(kit::edge_fold_rect_beside(dock.left(), dock.y_range(), Side::Right));
    let fold = app.right_fold.is_some_and(|r| edge_fold(app, ui, r, Side::Right, true));
    if fold {
        app.show_panel(Id::ViewPalettes, false);
    }
    if let Some(panel) = shown.inner {
        app.right_panel = panel;
    }
}

/// The Bottom Dock (`Panel::bottom("log")`), unless closed
/// (`Settings::hide_log`): its tab bar (the Log's tab; Copy, Save… and
/// Clear at its right), the log under it, as low as the tab bar and a
/// line and at most half the window.
pub(crate) fn bottom_dock(app: &mut Moonglow, ui: &mut Ui) {
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
    let high =
        chosen_size(&ctx, &mut app.settings.bottom_dock_height, "log", 1, metrics::BOTTOM_DOCK);
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
            let bar =
                DockTabs::new(BOTTOM_DOCK).with(DockTab::new("Log")).show(ui, &mut tab, |ui| {
                    let mut chosen = None;
                    for ((id, icon, name, _), tip) in buttons.iter().zip(&tips) {
                        let button = IconButton::new(*icon, *name).icon_only(true);
                        let r = ui.add_enabled(enabled, button);
                        if r.on_hover_text(tip).on_disabled_hover_text(tip).clicked() {
                            chosen = Some(*id);
                        }
                    }
                    chosen
                });
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
