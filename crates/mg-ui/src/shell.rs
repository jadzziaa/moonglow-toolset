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
//! - the **Left Dock** ([`left_dock`]): its tab bar (the Module panel's
//!   tab, its header buttons, and the one button that folds the dock),
//!   the Module panel under it (the Filter over the module tree, whose
//!   Areas group scrolls in a well of its own: M3's T3); folded, a strip
//!   with the chevron that brings it back;
//! - the **Bottom Dock** ([`bottom_dock`]): its tab bar (the Log's tab,
//!   with the log's Copy, Save… and Clear), the log under it; opened and
//!   closed by the Log's switch in the Status Bar (issue 10's one control
//!   per dock: the switch is the Bottom Dock's).
//!
//! The order [`Moonglow::ui`] draws them in makes the layout: the Top Bar,
//! then the notices under it, the Status Bar, the Left Dock (the whole
//! height between the bars), the Bottom Dock (under the editor area
//! only), and the editor area in what is left. The panels keep the ids
//! the window's panes had (`"tree"`, `"log"`), so the widths and heights
//! eframe kept for them carry over; `Settings::hide_tree` is "the Left
//! Dock folded" and `Settings::hide_log` "the Bottom Dock closed".
//!
//! Later tasks fill it: the Left Dock's Resources tab (T4), the Right Dock
//! with the Palette and Preview panels (T5, T6), the Log's last line and
//! the jobs in the Status Bar (T7), Quick Open in the Top Bar's middle
//! (T8), hints (T12) and the widths the shell yields at (T14; T2 takes
//! only the build actions' words, below 1300).

use egui::{Margin, Ui};
use mg_uikit::shell::{self as kit, DockTab, DockTabs, FoldStrip, Side, Yields};
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

/// The Left Dock's name: its tab bar's and, folded, its strip's group.
pub(crate) const LEFT_DOCK: &str = "Left Dock";
/// The Bottom Dock's name: its tab bar's group.
pub(crate) const BOTTOM_DOCK: &str = "Bottom Dock";
/// The Test Module arrow's name (it opens the menu of the other ways to
/// test: Choose Character).
pub(crate) const TEST_MENU: &str = "More Ways to Test";

/// Every icon the main window's bars show: the Top Bar's, Test Module's
/// arrow, and the Module panel's header buttons.
#[cfg(test)]
pub(crate) fn icons() -> Vec<Icon> {
    let mut all: Vec<Icon> = TOP_BAR.icons().collect();
    all.push(Icon::ChevronDown);
    all.extend(MODULE_TOOLS.iter().map(|t| t.1));
    all.extend([Icon::ExpandAll, Icon::CollapseAll]);
    all
}

/// The Module panel's header buttons that run commands (today's
/// toolbar's Properties and New Area), as the Top Bar's: command, icon,
/// name, tip. (Expand All and Collapse All after them are the tree's:
/// [`module_tools`].) The tree's row of the same name is apart from the
/// button: in the group [`tree::MODULE_TREE`], a selectable row.
const MODULE_TOOLS: [TopBarButton; 2] = [
    (Id::ModuleProperties, Icon::Module, "Module Properties", "Module properties"),
    (Id::AreaWizard, Icon::Plus, "New Area", "A new area, by the Area Wizard"),
];

/// The Left Dock's least width: its tab bar's Module tab, the Module
/// panel's four header buttons and the fold button, side by side.
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

/// The Left Dock (`Panel::left("tree")`), with a module open: its tab bar
/// (the Module tab; the Module panel's Module Properties, New Area, Expand
/// All and Collapse All, and the fold button at its right), the Module
/// panel under it (`tree::module_tree`). Folded (`Settings::hide_tree`),
/// the strip of its panels' icons under the chevron that brings it back.
pub(crate) fn left_dock(app: &mut Moonglow, ui: &mut Ui) {
    if app.ws.is_none() {
        return;
    }
    let ctx = ui.ctx().clone();
    let t = mg_uikit::tokens(&ctx);
    let line = panel_line(ui);
    let fold_key = key_note(app, &ctx, Id::ViewTree);
    if app.settings.hide_tree {
        app.left_fold = None;
        let unfold = format!("Show the Left Dock{fold_key}");
        let module_tip = format!("Module: the module tree{fold_key}");
        let clicked = egui::Panel::left("tree-strip")
            .exact_size(kit::STRIP + line)
            .resizable(false)
            .frame(egui::Frame::new().fill(t.bar))
            .show(ui, |ui| {
                FoldStrip::new(LEFT_DOCK, Side::Left)
                    .unfold_tip(unfold)
                    .panel(Icon::Module, "Module", module_tip)
                    .show(ui)
            })
            .inner;
        if clicked.is_some() {
            app.show_panel(Id::ViewTree, true);
        }
        return;
    }
    let window = ctx.content_rect().size();
    let tools: Vec<Prepared> = MODULE_TOOLS.iter().map(|b| Prepared::of(app, &ctx, *b)).collect();
    let fold_tip = format!("Fold the Left Dock{fold_key}");
    // The fold button stands at the dock's inner edge, where the panel is
    // resized from: under it (or pressed on it), the button has the
    // pointer, not the edge (egui reads the edge after the panel's
    // contents).
    let on_button = app.left_fold.is_some_and(|fold| {
        ctx.input(|i| match (i.pointer.any_down(), i.pointer.press_origin()) {
            (true, Some(origin)) => fold.contains(origin),
            _ => i.pointer.hover_pos().is_some_and(|p| fold.contains(p)),
        })
    });
    let shown = egui::Panel::left("tree")
        .resizable(!on_button)
        .default_size(metrics::LEFT_DOCK)
        .size_range(LEFT_DOCK_LEAST..=(window.x * 0.4).max(LEFT_DOCK_LEAST))
        .frame(egui::Frame::new().fill(t.panel))
        .show(ui, |ui| {
            let mut tab = 0;
            let keymap = &app.keymap;
            let bar =
                DockTabs::new(LEFT_DOCK).with(DockTab::new("Module")).show(ui, &mut tab, |ui| {
                    let asked = module_tools(ui, &tools, keymap);
                    let fold = kit::fold_button_tip(ui, Side::Left, LEFT_DOCK, &fold_tip);
                    (asked, fold.clicked(), fold.rect)
                });
            let ((_, folded), ..) = bar.inner;
            // (The tree's rows reach the panel's sides, as the mock's;
            // its Filter has a margin of its own.)
            let margin = Margin { left: 0, right: 0, top: 2, bottom: 2 };
            let body = egui::Frame::new().inner_margin(margin);
            body.show(ui, |ui| tree::module_tree(app, ui, folded));
            bar.inner
        });
    app.tree_width = Some(shown.response.rect.width());
    let ((clicked, _), fold, fold_rect) = shown.inner;
    app.left_fold = Some(fold_rect);
    if let Some(id) = clicked {
        id.run(app, &ctx);
    }
    if fold {
        app.show_panel(Id::ViewTree, false);
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
    let chosen = egui::Panel::bottom("log")
        .resizable(true)
        .default_size(metrics::BOTTOM_DOCK)
        .size_range(least..=(window.y * 0.5).max(least))
        .frame(egui::Frame::new().fill(t.panel))
        .show(ui, |ui| {
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
        })
        .inner;
    if let Some(id) = chosen {
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
