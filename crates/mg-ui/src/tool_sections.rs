//! The area editor's tool sections of the Right Dock (M4's U29, the plan's
//! "Dialogs That Could Become Dock Panels"): Edit Areas Together, Custom
//! Light (the Light Maker) and Bake Walkmesh to Transform, each a window
//! before. Each is a panel of one section of its own, under the editor's
//! sections, while it is open: Edit Areas Together whatever the editor in
//! front, Custom Light and Bake beside their area alone (whatever the
//! editor in front where the area is dragged out into a window of its own,
//! whose right pane may be dropped in a small window). Opened, a tool shows the dock and its panel
//! in front, folded or hidden as the dock was, and holds the keys over the
//! area's view until a press elsewhere (M4's U28 review). Its actions are
//! a row at its own top, as Properties' Open in Tab is where its section
//! has no header (the Properties sections are the pattern: no board draws
//! these): its main action in the accent and Cancel, Enter and Escape
//! theirs while it holds the keys.

use egui::Ui;
use mg_core::ResRef;

use crate::Moonglow;
use crate::palette_view::Keys;
use crate::shell::Panel;

/// The tool panels, in their section's order.
pub(crate) const TOOLS: [Panel; 3] = [Panel::AreasTogether, Panel::LightMaker, Panel::Bake];

/// What the tools keep between frames.
#[derive(Debug, Default)]
pub(crate) struct Tools {
    /// Which were open as the last frame began (a tool opened since is
    /// brought to the front: [`follow`]).
    was: [bool; 3],
    /// A main action pressed, made as the next frame begins, every area's
    /// view in its place (one dragged out is taken from its place while
    /// its window draws).
    go: Option<Panel>,
    /// A tool asked to the front, open or not (its opener's menu drawn
    /// with the app borrowed): brought as the next frame begins.
    pub(crate) bring: Option<Panel>,
    /// The Right Dock's panel chosen before a tool was brought to its
    /// front: chosen again as the tool closes, where it is still the tool.
    before: Option<Option<Panel>>,
    /// The module the tools were opened on ([`mg_edit::Workspace::identity`]):
    /// another opened, they close (M4's U29 review: the ticks were kept
    /// by name, and edits went into the new module).
    workspace: Option<u64>,
    /// How often a walk over the areas Edit Areas Together has ticked was
    /// made (their audio settings, their grass, their claims): when they
    /// change, never every frame. For the tests.
    pub(crate) walks: u64,
    /// The share of a stacked Right Dock's column an open tool section
    /// takes, as its seam was last dragged this run ([`share`]).
    pub(crate) share: Option<f32>,
    /// The commands Edit Areas Together's shared pages pushed this frame,
    /// known again as they are applied ([`marked`]): the section's own
    /// steps, which its Cancel All takes back.
    pub(crate) marks: Vec<mg_edit::Command>,
    /// What a field of its pages holds typed, set should the field be gone
    /// before it is left (`Moonglow::held_text`): the section's too.
    pub(crate) held_mark: Option<mg_edit::Command>,
    /// Cancel All pressed: made as the next frame begins ([`follow`]).
    pub(crate) cancel_all: bool,
    /// Cancel All's Undo under way: the module's history's, never a side
    /// editor's (`Moonglow::local_undo`).
    pub(crate) history_only: bool,
}

/// A tool section's share of its column of stacked sections at first
/// (M4's U29 review: two thirds, the editor's sections the rest).
pub(crate) const SHARE: f32 = 2.0 / 3.0;

/// The share of its column a tool section takes: as its seam was dragged
/// this run, else [`SHARE`].
pub(crate) fn share(app: &Moonglow) -> f32 {
    app.tools.share.unwrap_or(SHARE)
}

/// Whether a Right Dock section is the tools' own.
pub(crate) fn is_tools(section: &[Panel]) -> bool {
    !section.is_empty() && section.iter().all(|p| TOOLS.contains(p))
}

/// Whether `panel` is open (its state held).
pub(crate) fn is_open(app: &Moonglow, panel: Panel) -> bool {
    match panel {
        Panel::AreasTogether => app.area_chooser.is_some(),
        Panel::LightMaker => app.light_draft.is_some(),
        Panel::Bake => app.bake.draft.is_some(),
        _ => false,
    }
}

/// The area a tool is for (Custom Light's, Bake's).
pub(crate) fn area_of(app: &Moonglow, panel: Panel) -> Option<ResRef> {
    match panel {
        Panel::LightMaker => app.light_draft.as_ref().map(|d| d.area),
        Panel::Bake => app.bake.draft.as_ref().map(|d| d.area),
        _ => None,
    }
}

/// Whether `panel` is shown in the Right Dock beside the active docked
/// editor, the area `area` (`None`: not an area's editor): Edit Areas
/// Together whatever the editor, Custom Light and Bake beside their area,
/// or whatever the editor where their area is dragged out into a window of
/// its own (whose right pane gives way in a small window: the tool is
/// never out of reach).
pub(crate) fn shown_beside(app: &Moonglow, panel: Panel, area: Option<ResRef>) -> bool {
    match panel {
        Panel::AreasTogether => app.ws.is_some() && app.area_chooser.is_some(),
        Panel::LightMaker | Panel::Bake => {
            let Some(theirs) = area_of(app, panel) else { return false };
            area == Some(theirs) || floats(app, theirs)
        }
        _ => false,
    }
}

/// Whether `area`'s tab is in a window of its own.
fn floats(app: &Moonglow, area: ResRef) -> bool {
    app.dock.find_tab(&crate::Tab::Area(area)).is_some_and(|at| !at.surface.is_main())
}

/// The tools' section beside the active docked editor, the area `area`
/// ([`shown_beside`]): none where no tool is open.
pub(crate) fn section(app: &Moonglow, area: Option<ResRef>) -> Vec<Panel> {
    TOOLS.into_iter().filter(|p| shown_beside(app, *p, area)).collect()
}

/// What a tool's header says after its title (`Panel::detail`): the areas
/// ticked, what the light is made for, the placeables baked.
pub(crate) fn detail(app: &Moonglow, panel: Panel) -> Option<String> {
    let count = |n: usize, one: &str, many: &str| match n {
        1 => format!("1 {one}"),
        n => format!("{n} {many}"),
    };
    match panel {
        Panel::AreasTogether => {
            let n = app.area_chooser.as_ref().map_or(0, |c| c.chosen.len());
            Some(match n {
                0 => "No areas checked".to_string(),
                n => format!("{} checked", count(n, "area", "areas")),
            })
        }
        Panel::LightMaker => {
            let d = app.light_draft.as_ref()?;
            Some(match (d.change.len(), d.at) {
                (0, Some(_)) => "At the point clicked".to_string(),
                (0, None) => "Placed with a click".to_string(),
                (n, _) => format!("Changing {}", count(n, "light", "lights")),
            })
        }
        Panel::Bake => {
            let d = app.bake.draft.as_ref()?;
            Some(count(d.objects.len(), "placeable", "placeables"))
        }
        _ => None,
    }
}

/// Each frame's start: a tool whose reason went is closed (Custom Light
/// with generated lights turned off, or either area tool with its area's
/// tab closed; Edit Areas Together with the module), one opened since the
/// last frame is brought to the front of the dock with the keys, and a main
/// action pressed last frame is made.
pub(crate) fn follow(app: &mut Moonglow) {
    if app.light_draft.is_some() && !crate::light_maker::generated_lights(app) {
        app.light_draft = None;
    }
    let closed =
        |app: &Moonglow, area: ResRef| app.dock.find_tab(&crate::Tab::Area(area)).is_none();
    if let Some(area) = area_of(app, Panel::LightMaker)
        && closed(app, area)
    {
        app.light_draft = None;
    }
    if let Some(area) = area_of(app, Panel::Bake)
        && closed(app, area)
    {
        app.bake.draft = None;
    }
    let module = app.ws.as_ref().map(mg_edit::Workspace::identity);
    if module != app.tools.workspace {
        app.tools.workspace = module;
        app.area_chooser = None;
        app.light_draft = None;
        app.bake.draft = None;
    }
    // (What Edit Areas Together's pages pushed last frame was applied as
    // it ended: none of it is left for a command made elsewhere, the
    // section hidden or not. Its held text's stays while that text is
    // held, to be applied as its field goes.)
    app.tools.marks.clear();
    let held = app.held_text.as_ref().and_then(|h| h.command.as_ref());
    if held != app.tools.held_mark.as_ref() {
        app.tools.held_mark = None;
    }
    if std::mem::take(&mut app.tools.cancel_all) {
        cancel_all(app);
    }
    match app.tools.go.take() {
        Some(Panel::LightMaker) => crate::light_maker::finish(app),
        Some(Panel::Bake) => crate::bake_view::finish(app),
        _ => {}
    }
    let asked = app.tools.bring.take();
    for (i, panel) in TOOLS.into_iter().enumerate() {
        let open = is_open(app, panel);
        if open && (!app.tools.was[i] || asked == Some(panel)) {
            bring(app, panel);
        }
        // (Closed, however: the panel in front before it, again.)
        if !open && app.tools.was[i] && app.right_panel == Some(panel) {
            let before = app.tools.before.take().flatten();
            app.right_panel = before.filter(|p| !TOOLS.contains(p));
        }
        app.tools.was[i] = open;
    }
}

/// Whether `cmd`, about to be applied, is one Edit Areas Together's shared
/// pages pushed ([`Tools::marks`]): taken from the marks if so.
pub(crate) fn marked(app: &mut Moonglow, cmd: &mg_edit::Command) -> bool {
    if app.area_chooser.is_none() {
        app.tools.marks.clear();
        app.tools.held_mark = None;
        return false;
    }
    if let Some(i) = app.tools.marks.iter().position(|m| m == cmd) {
        app.tools.marks.swap_remove(i);
        return true;
    }
    app.tools.held_mark.take_if(|m| m == cmd).is_some()
}

/// A command of Edit Areas Together's just applied ([`marked`]): its step
/// kept as the section's.
pub(crate) fn made(app: &mut Moonglow) {
    let Some(ws) = &app.ws else { return };
    let (Some(step), depth) = (ws.last_step(), ws.undo_depth()) else { return };
    if let Some(c) = &mut app.area_chooser {
        c.made_step(step, depth);
    }
}

/// Edit Areas Together's Cancel All (ruling 66): the run of the section's
/// own steps on top of the history ([`crate::area_props::AreaChooser::run`])
/// made one step and taken back, which one Redo brings back. Never a step
/// it did not make: an edit made elsewhere, or Undo by hand, stops the run.
fn cancel_all(app: &mut Moonglow) {
    // (Q and E's turning or a Ctrl + wheel scaling under way is what Undo
    // takes back first: after it.)
    let busy = |v: &crate::area_view::AreaView| v.key_turning() || v.wheel_scaling();
    if app.area_views.values().any(busy) {
        app.tools.cancel_all = true;
        return;
    }
    let (Some(c), Some(ws)) = (&mut app.area_chooser, &mut app.ws) else { return };
    let run = c.run(ws);
    if run.is_empty() {
        return;
    }
    let n = run.len();
    let top = ws.undo_depth();
    if n > 1 && ws.join_last(n, format!("{n} Edits in Edit Areas Together")) {
        c.joined(&run, ws);
        // (Their typed lines of the talk table go with the one step; a step
        // undone above them, its lines with it, is redone that much lower.)
        app.typed.joined(n, top);
    }
    app.tools.history_only = true;
    app.run(crate::Action::Undo);
    app.tools.history_only = false;
}

/// `panel` shown: the Right Dock open on it (or its area's window's right
/// pane, where the area floats), the keys its, held over the area's view
/// until a press elsewhere. Edit Areas Together's commands call it again
/// with it open (its areas chosen anew).
pub(crate) fn bring(app: &mut Moonglow, panel: Panel) {
    if !app.right_panel.is_some_and(|p| TOOLS.contains(&p)) {
        app.tools.before = Some(app.right_panel);
    }
    app.show_dock_panel(panel);
    app.dialog_panel_focus = Some(panel.keys());
}

/// Its answer this frame ([`actions`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// Its main action: clicked, or Enter while it holds the keys.
    Main,
    /// Cancel (or Close): clicked, or Escape while it holds the keys.
    Leave,
}

/// A tool's actions, the row at its top: `main` (its words, whether it is
/// enabled, its tip) in the accent where it has one, then `leave` (Cancel,
/// Close); Escape presses `leave` while the tool holds the keys, with no
/// menu, popover or dialog open and no text field holding the keyboard
/// (where Escape leaves the field: what was typed is not lost with the
/// tool). Enter is [`enter_after`]'s, once the fields are drawn. `detail`:
/// where its section has no header over it (one tab bar of several), its
/// header's words at the row's left. `why`: where the main action is
/// disabled, why, in sight under the row (M4's U29 review: a tip alone,
/// or a line at the foot of the scroll, was out of sight).
pub(crate) fn actions(
    ui: &mut Ui,
    keys: Keys,
    main: Option<(&str, bool, &str)>,
    leave: (&str, &str),
    detail: Option<&str>,
    why: Option<&str>,
) -> Option<Answer> {
    actions_with(ui, keys, main, None, leave, detail, why).0
}

/// [`actions`] with `other` (its words, whether it is enabled, its tip)
/// between the main action and `leave`, the dialog pattern's order (Edit
/// Areas Together's Cancel All); and whether `other` was clicked.
pub(crate) fn actions_with(
    ui: &mut Ui,
    keys: Keys,
    main: Option<(&str, bool, &str)>,
    other: Option<(&str, bool, &str)>,
    (leave, leave_tip): (&str, &str),
    detail: Option<&str>,
    why: Option<&str>,
) -> (Option<Answer>, bool) {
    let margin = egui::Margin { left: 8, right: 8, top: 6, bottom: 6 };
    let mut answer = None;
    let mut pressed = false;
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.horizontal(|ui| {
            if let Some((words, enabled, tip)) = main {
                let b = mg_uikit::KitButton::new(None, words)
                    .look(mg_uikit::ButtonLook::Primary)
                    .enabled(enabled)
                    .tip(tip);
                if ui.add(b).clicked() && enabled {
                    answer = Some(Answer::Main);
                }
            }
            if let Some((words, enabled, tip)) = other {
                let b = mg_uikit::KitButton::new(None, words).enabled(enabled).tip(tip);
                pressed = ui.add(b).clicked() && enabled;
            }
            let b = mg_uikit::KitButton::new(None, leave).tip(leave_tip);
            if ui.add(b).clicked() {
                answer = Some(Answer::Leave);
            }
            if let Some(detail) = detail {
                ui.add(egui::Label::new(egui::RichText::new(detail).weak()).truncate());
            }
        });
        if let (Some((_, false, _)), Some(why)) = (main, why) {
            let text = egui::RichText::new(why).color(ui.visuals().warn_fg_color);
            ui.add(egui::Label::new(text).wrap());
        }
    });
    let line = ui.min_rect().bottom();
    let t = mg_uikit::tokens(ui.ctx());
    ui.painter().hline(ui.max_rect().x_range(), line, egui::Stroke::new(1.0, t.line));
    // (Escape in a field gives up the field alone: egui has let go of it
    // by now, so the field that had the keyboard as the last frame ended
    // is asked, the Filter's as any other's. M4's U29 review.)
    if answer.is_none()
        && holds_keys(ui.ctx(), keys)
        && !crate::widgets::field_had_keyboard(ui.ctx())
        && !crate::widgets::search_has_keyboard(ui.ctx())
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        answer = Some(Answer::Leave);
    }
    (answer, pressed)
}

/// Enter for a tool's main action (`enabled`), after its fields are drawn:
/// while the tool holds the keys, with no menu, popover or dialog open and
/// no control holding the keyboard. `from_field`: also on the frame a
/// one-line field lets go of the keyboard at Enter, as a dialog's footer
/// takes it (Custom Light's, Bake's: nothing is written before it); not
/// where the fields are written as they change (Edit Areas Together's
/// pages), whose Enter commits the field and the section stays (M4's U29
/// review).
pub(crate) fn enter_after(ui: &Ui, keys: Keys, enabled: bool, from_field: bool) -> Option<Answer> {
    let field = !from_field && crate::widgets::field_had_keyboard(ui.ctx());
    (enabled && !field && holds_keys(ui.ctx(), keys) && crate::widgets::enter(ui))
        .then_some(Answer::Main)
}

/// Whether a tool's keys (`keys`) are its now: given to it (by its command
/// or a press in it), with no menu, popover or dialog open.
pub(crate) fn holds_keys(ctx: &egui::Context, keys: Keys) -> bool {
    crate::palette_view::keys_are(ctx, keys)
        && !crate::shell::keys_blocked(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
        && !crate::widgets::escape_closes_a_popup(ctx)
}

/// The keys given to a tool where its command (or F6) asked for them, as
/// it is drawn in `ui`'s room, and taken by a press in it (the outline
/// that shows they are its is painted after its body: [`body`]).
pub(crate) fn take_keys(app: &mut Moonglow, ui: &Ui, keys: Keys) {
    if app.dialog_panel_focus.take_if(|k| *k == keys).is_some() {
        crate::palette_view::hold_keys(ui.ctx(), keys);
    }
    // (A press in it holds them too: a field edited, the pointer gone
    // over the area's view, Enter is still the tool's. A press anywhere
    // else takes them from it, Properties over it included, but for one on
    // a list or menu it opened: M4's U29 review.)
    let press = ui.input(|i| i.pointer.any_pressed().then(|| i.pointer.press_origin()).flatten());
    if let Some(at) = press {
        let popup = (ui.ctx().layer_id_at(at))
            .is_some_and(|l| matches!(l.order, egui::Order::Foreground | egui::Order::Tooltip));
        if ui.max_rect().contains(at) && ui.clip_rect().contains(at) {
            crate::palette_view::hold_keys(ui.ctx(), keys);
        } else if !popup && crate::palette_view::keys_are(ui.ctx(), keys) {
            let (to, table) = app.editor_keys();
            crate::palette_view::give_keys(ui.ctx(), to);
            if let Some(id) = table {
                crate::table_view::take_keys(ui.ctx(), id);
            }
        }
    }
}

/// A tool's fields under its actions, on a scroll of the room left: a
/// field or button given the keyboard's focus (Tab) is brought into sight.
pub(crate) fn scrolled<R>(ui: &mut Ui, salt: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    scrolled_in(ui, salt, egui::Margin { left: 8, right: 8, top: 6, bottom: 6 }, add)
}

/// [`scrolled`], inside `margin` (the Plugins tab's page).
pub(crate) fn scrolled_in<R>(
    ui: &mut Ui,
    salt: &str,
    margin: egui::Margin,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let id = ui.id().with((salt, "focus-seen"));
    egui::ScrollArea::vertical()
        .id_salt(salt)
        .auto_shrink([false, false])
        .min_scrolled_height(0.0)
        .show(ui, |ui| {
            let out = egui::Frame::new().inner_margin(margin).show(ui, add).inner;
            // (Not in a frame with a press: egui's `read_response` takes the
            // focus from a widget pressed elsewhere.)
            let focused = ui.memory(|m| m.focused());
            let seen = ui.data(|d| d.get_temp::<Option<egui::Id>>(id)).flatten();
            let pressed = ui.input(|i| i.pointer.any_pressed() || i.pointer.any_click());
            if focused != seen && !pressed {
                ui.data_mut(|d| d.insert_temp(id, focused));
                if let Some(r) = focused.and_then(|f| ui.ctx().read_response(f))
                    && ui.clip_rect().intersects(r.rect.expand(1e4))
                    && !ui.clip_rect().contains_rect(r.rect)
                    && ui.min_rect().contains_rect(r.rect)
                {
                    ui.scroll_to_rect(r.rect, None);
                }
            }
            out
        })
        .inner
}

/// A tool's body (`shell::panel_body`): `titled`, its section's header over
/// it with its title and detail.
pub(crate) fn body(app: &mut Moonglow, ui: &mut Ui, panel: Panel, titled: bool) {
    let keys = panel.keys();
    take_keys(app, ui, keys);
    let detail = (!titled).then(|| detail(app, panel)).flatten();
    let answer = match panel {
        Panel::AreasTogether => crate::area_props::together_panel(app, ui, detail.as_deref()),
        Panel::LightMaker => crate::light_maker::panel(app, ui, detail.as_deref()),
        Panel::Bake => crate::bake_view::panel(app, ui, detail.as_deref()),
        _ => None,
    };
    match answer {
        Some(Answer::Main) => {
            app.tools.go = Some(panel);
            ui.ctx().request_repaint();
        }
        Some(Answer::Leave) => close(app, panel),
        None => {}
    }
    // (Painted after its body, round the whole section: before it, Edit
    // Areas Together's list and rail were drawn over it and only its
    // actions' row showed it. M4's U29 review.)
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
}

/// A tool closed (its Cancel or Close): its state let go of, its panel
/// gone from the dock.
pub(crate) fn close(app: &mut Moonglow, panel: Panel) {
    match panel {
        Panel::AreasTogether => app.area_chooser = None,
        Panel::LightMaker => app.light_draft = None,
        Panel::Bake => app.bake.draft = None,
        _ => {}
    }
}

/// The Status Bar's hints while a tool holds the keys (the U28 review's
/// lesson: hints for every pane that holds them, none where they do not
/// apply), in the editor's place, its counts kept: the list's keys on Edit
/// Areas Together's Areas page, Enter its main action where it is enabled,
/// Escape its Cancel or Close (not in a text field, which Escape leaves).
pub(crate) fn hints(app: &Moonglow, ctx: &egui::Context) -> Option<Vec<mg_uikit::status::Hint>> {
    use mg_uikit::status::Hint;
    let panel = TOOLS.into_iter().find(|p| is_open(app, *p) && holds_keys(ctx, p.keys()))?;
    let typing = crate::report_table::text_has_keys(ctx);
    let mut hints = Vec::new();
    let (main, leave) = match panel {
        Panel::AreasTogether => {
            let c = app.area_chooser.as_ref()?;
            if c.list_keys() && !typing {
                hints.push(Hint::either(["Up", "Down"], "Move in the list"));
                hints.push(Hint::keys(["Space"], "Check"));
            }
            let main = if c.enter_opens() { "Open in Tab" } else { "" };
            (main, "Close")
        }
        Panel::LightMaker => (crate::light_maker::main_words(app).unwrap_or(""), "Cancel"),
        Panel::Bake => (crate::bake_view::main_words(app).unwrap_or(""), "Cancel"),
        _ => return None,
    };
    if !main.is_empty() {
        hints.push(Hint::keys(["Enter"], main));
    }
    if !typing {
        hints.push(Hint::keys(["Esc"], leave));
    }
    hints.push(Hint::keys(["Tab"], "Next field"));
    Some(hints)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What Edit Areas Together's pages pushed and was not applied with
    /// the frame (a command held back, the section hidden after) is not
    /// left to count a command made elsewhere as the section's (ruling 66).
    #[test]
    fn the_section_s_marks_go_with_the_frame_shown_or_not() {
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.area_chooser = Some(crate::area_props::AreaChooser::default());
        let cmd = mg_edit::Command::new("Change", Vec::new());
        app.tools.marks.push(cmd.clone());
        app.tools.held_mark = Some(cmd.clone());
        follow(&mut app);
        assert!(!marked(&mut app, &cmd), "not the section's a frame later");
    }
}
