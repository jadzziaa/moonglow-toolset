//! The NUI Creator: the editor of a NUI window (a JUI of the module, with
//! the TXT of its name that holds the Creator's settings for it).
//!
//! This file holds the tab itself ([`ui`]): what it keeps between frames
//! ([`State`]), the header with Build and Export, the pages' row, the list
//! of what the checks found, the pages (Design, Bindings, Events &
//! variants, the two source pages and Images) and the properties of the
//! control selected. Beside it are New NUI Window ([`windows`], [`create`]),
//! Build ([`generate`]) and what a window has with it in the module
//! ([`companions`], [`stale_windows`]). The submodules hold the parts of
//! the pages.
//!
//! The window and its settings are edited as the module's other resources
//! are: through the workspace, one Undo step an edit. A field's value is
//! set when the field is left (a drag: when let go; `fields`), a source
//! page's text likewise, and text that doesn't read as JSON is kept in the
//! module as typed, where Undo takes it back.
use crate::{Action, Moonglow, Tab, widgets};
use egui::Ui;
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_nui::{Binding, Settings, Severity};
use mg_resman::ResKey;
use serde_json::{Value, json};

mod bindings;
mod design;
mod draw;
mod events;
mod fields;
pub(crate) mod headless;
mod images;
mod interaction;
mod layouts;
mod preview;
#[cfg(test)]
mod rename_tests;
mod shortcuts;
pub(crate) mod skin;
mod structure;
mod workflow;

/// What a NUI Creator tab keeps between frames.
#[derive(Default)]
pub(crate) struct State {
    /// The control selected, as a JSON pointer into the window (or into the
    /// variant edited in its place); empty for none, `/root` for the root.
    selected: String,
    /// What is typed in the palette's Find field.
    search: String,
    /// The canvas's zoom (0: Fit).
    zoom: f32,
    /// The GUI scale the canvas shows the window at, as the client's UI
    /// Scale option (0: not set yet, taken as 1).
    preview_scale: f32,
    /// The state the canvas shows the selected control in: 0 follows the
    /// pointer, then normal, hover, pressed and disabled.
    preview_state: usize,
    /// Resources: under the canvas's toolbar, where each picture and font
    /// came from and what was not found.
    preview_info: bool,
    /// The canvas shows the window alone, without the editor's guides.
    preview_clean: bool,
    /// Interact: the canvas's controls take values, in `runtime`.
    preview_interactive: bool,
    /// Where the canvas drew the window last frame.
    preview_rect: Option<egui::Rect>,
    /// The preview session while Interact is on: the values its controls
    /// were given, none of them the module's.
    runtime: Option<interaction::Session>,
    /// A control or the window being sized on the canvas, until release.
    resize: Option<interaction::Resize>,
    /// The rows of Layers that are folded, by path.
    collapsed: std::collections::BTreeSet<String>,
    /// The selection Layers last showed: a new one unfolds the rows over it.
    tree_selection: String,
    /// How far each list, group and text block of the canvas is scrolled,
    /// by path.
    list_scroll: std::collections::BTreeMap<String, f32>,
    group_scroll_x: std::collections::BTreeMap<String, f32>,
    group_scroll_y: std::collections::BTreeMap<String, f32>,
    text_scroll_x: std::collections::BTreeMap<String, f32>,
    text_scroll_y: std::collections::BTreeMap<String, f32>,
    /// The controls selected together, by path; `selected` is among them.
    selected_many: std::collections::BTreeSet<String>,
    /// A draw layer chosen in Layers: its control's path and its index.
    draw_item: Option<(String, usize)>,
    /// A draw layer being moved or sized on the canvas, until release.
    draw_drag: Option<draw::CanvasDrag>,
    /// What is typed in Layers' Find field.
    layer_search: String,
    /// A control dropped on another this frame: moved once the tree is drawn.
    move_pending: Option<(structure::Drag, String, structure::Position)>,
    /// A palette entry dropped on a control this frame: added once the tree
    /// is drawn.
    insert_pending: Option<(&'static str, String, structure::Position)>,
    /// The swap layout variant edited in the window's place, if one is.
    edit_view: Option<String>,
    /// The group (a swap layout's ID) a new variant is made for.
    layout_target: String,
    /// A swap layout just added, by its path, until its ID is noted in the
    /// settings.
    new_swap_slot: Option<String>,
    /// The IDs of the window's groups, as last parsed.
    layout_groups: std::collections::BTreeSet<String>,
    /// The window itself as last parsed: what Layers shows while a variant
    /// is edited in its place.
    main_doc: Option<Value>,
    /// The window and its settings as last parsed, and what they were parsed from.
    source: Option<Source>,
    /// Event script… or Edit code was pressed: the script to write and open
    /// at the end of the frame.
    event_script: Option<events::EventScript>,
    /// The property the Bind section of the properties is about.
    binding_property: String,
    /// What is typed in a picture chooser's Find field.
    asset_search: String,
    /// The canvas shows the window on a screen of `screen_size`.
    screen_preview: bool,
    screen_size: egui::Vec2,
    /// The window being moved on that screen, until release.
    window_move: Option<interaction::WindowMove>,
    /// The list of what the checks found is open.
    issues: bool,
    /// The errors there were last frame, to open the list when more come.
    issues_seen: usize,
    /// The page shown.
    page: Page,
    /// The hak Images adds pictures to: one the module uses, or None for a new one.
    images_hak: Option<String>,
    /// How the Design page shows the window.
    view_mode: ViewMode,
    /// What the checks found, for the bytes they were run on.
    checks: Option<Checks>,
    /// The source field being typed in.
    source_draft: Option<SourceDraft>,
}

/// A page of the Creator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Page {
    #[default]
    Design,
    Bindings,
    /// The window's JSON as text.
    JuiSource,
    /// The settings' JSON as text.
    BindingJson,
    /// Events & variants.
    Events,
    Images,
}

/// How the Design page shows the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ViewMode {
    /// The palette, the canvas and the properties.
    #[default]
    Editor,
    /// The canvas alone.
    Preview,
    /// The canvas beside the source.
    Split,
}

impl ViewMode {
    const ALL: [ViewMode; 3] = [ViewMode::Editor, ViewMode::Preview, ViewMode::Split];

    fn label(self) -> &'static str {
        match self {
            ViewMode::Editor => "Editor",
            ViewMode::Preview => "Preview only",
            ViewMode::Split => "Split source",
        }
    }
}

/// What is done to the selected control among its siblings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    MoveUp,
    MoveDown,
    Duplicate,
    Delete,
}

/// The API checks and build status of what the tab last showed.
struct Checks {
    /// The window, its settings and its scripts when they were checked.
    of: Vec<Held>,
    findings: std::rc::Rc<[mg_nui::Diagnostic]>,
    needs_build: bool,
}

/// A resource as the module held it (or did not): the same again while
/// nothing replaced it, which costs a pointer's comparison to find out.
type Held = Option<std::sync::Arc<[u8]>>;

fn same(a: &[Held], b: &[Held]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|pair| match pair {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        })
}

/// The window's document and settings, parsed once for the bytes they are.
struct Source {
    of: [Held; 2],
    window: Result<Value, String>,
    settings: Result<Settings, String>,
}

impl State {
    fn guides(&self) -> bool {
        !self.preview_clean
    }
}

/// The window that asks for a new NUI window's name (Tools › New NUI
/// Window…, the module tree's New). The name leaves room for the two
/// scripts' suffixes, which the name window does not know of: one that is
/// refused is said in the log, and the window stays.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    use crate::widgets::{NameWindow, Named};
    let Some(mut name) = app.new_nui.clone() else { return };
    let window = NameWindow {
        title: "New NUI Window",
        label: "Name (up to 14 characters)",
        restype: ResType::JUI,
        taken: "The module already has a NUI window of this name.",
        replaces: false,
        button: "Create",
    };
    let module = app.ws.as_ref().map(|w| &w.module);
    app.new_nui = match window.show(ui.ctx(), module, &mut name) {
        Named::Open => Some(name),
        Named::Cancelled => None,
        Named::Chosen(key) => {
            let name = key.resref.to_string();
            match module.map(|m| mg_nui::create(m, &name)) {
                Some(Err(e)) => {
                    app.log.error(e.to_string());
                    Some(name)
                }
                _ => {
                    app.actions.push(Action::NewNui(name));
                    None
                }
            }
        }
    };
}

pub(crate) fn create(app: &mut Moonglow, name: &str) {
    let Some(ws) = &app.ws else { return };
    match mg_nui::create(&ws.module, name) {
        Ok(cmd) => match app.apply(cmd) {
            Ok(()) => app.actions.extend(
                ResKey::parse(name, ResType::JUI).map(|key| Action::OpenTab(Tab::Nui(key))),
            ),
            Err(e) => app.log.error(e.to_string()),
        },
        Err(e) => app.log.error(e.to_string()),
    }
}

pub(crate) fn generate(app: &mut Moonglow, name: &str, export: bool) {
    let before = app.log.entries.len();
    build(app, name, export);
    // A failure stays by the Build button until the next Build.
    app.nui_build_error = app
        .log
        .entries
        .get(before..)
        .unwrap_or_default()
        .iter()
        .rev()
        .find(|(level, _)| *level == crate::Level::Error)
        .map(|(_, why)| (name.to_owned(), why.clone()));
}

fn build(app: &mut Moonglow, name: &str, export: bool) {
    // The compiler must see the same source the user is looking at. The
    // window's own scripts are what you edit while building it: they are
    // saved first. Other unsaved scripts may be included: they are not
    // compiled in their older saved state.
    for suffix in ["_e", "_o"] {
        if let Some(key) = ResKey::parse(&format!("{name}{suffix}"), ResType::NSS)
            && let Some(buf) = app.scripts.get_mut(&key).filter(|b| b.is_dirty())
        {
            let data = Some(buf.mark_saved());
            if let Err(e) = app
                .apply(Command::new(format!("Edit {key}"), vec![Edit::SetResource { key, data }]))
            {
                app.log.error(e.to_string());
                return;
            }
        }
    }
    let unsaved: Vec<String> =
        app.scripts.iter().filter(|(_, b)| b.is_dirty()).map(|(k, _)| k.to_string()).collect();
    if !unsaved.is_empty() {
        app.log.error(format!(
            "Save {} before building: a window's scripts may include them",
            unsaved.join(", ")
        ));
        return;
    }
    let external = app.external_compiler();
    // What Compile All remembers, as it would load it: Build adds to it.
    let kept = app.remembered_scripts(external.as_ref());
    let (Some(ws), Some(game)) = (&app.ws, &app.game) else {
        app.log.error("NUI generation needs the installed game's NWScript API; configure the game folder in Options");
        return;
    };
    let mut built = None;
    let result = mg_nui::generate(
        &ws.module,
        name,
        &mg_nui::Build {
            resman: &game.resman,
            external: external.as_ref(),
            debug: app.settings.debug_info,
            known: Some(&app.script_stamps),
        },
    )
    // (What it could not compile of the module's own scripts, and what is
    // remembered of those it did: said and kept once the rest is applied.)
    .map(|made| {
        built = Some((made.failed, made.remembered));
        made.command
    });
    // (For the window's Cancel: what Build replaces is set back with the
    // settings that say what Build made, or the two would not agree and
    // the next Build would take the opener for one written by hand.)
    if let (Ok(cmd), Some(key)) = (&result, ResKey::parse(name, ResType::JUI)) {
        let targets = [(key, mg_edit::GffPath::root())];
        let cmd = crate::blueprint::but_for(cmd, &events_script(name));
        crate::blueprint::note_command(app, &Tab::Nui(key), &targets, &cmd);
    }
    match result {
        Ok(cmd) => match if cmd.edits.is_empty() { Ok(()) } else { app.apply(cmd) } {
            Ok(()) => {
                if let Some((failed, remembered)) = built {
                    app.script_stamps = remembered;
                    if let Some(file) = &kept
                        && let Err(e) = app.script_stamps.write(file)
                    {
                        crate::trace::note(format!("compiled scripts not noted in {file:?}: {e}"));
                    }
                    for e in failed.iter().filter_map(|f| f.result.as_ref().err()) {
                        app.log.error(e.message.clone());
                    }
                }
                // (Into the module, as any edit: a nasher project's files
                // are written when it is saved. "Written" was read as on
                // disk, a builder's report.)
                let saved = match app.ws.as_ref().is_some_and(|w| w.module.project.is_some()) {
                    true => "File › Save writes them into the nasher project's sources",
                    false => "File › Save writes them into the module",
                };
                app.log.info(format!(
                    "Built {name}: {name}_o.nss made, {name}_e.nss made and compiled ({saved}). \
                     To open the window, include {name}_o in a script of yours and call \
                     Open_{name}(oPlayer)."
                ));
                if export {
                    let mut resources: Vec<ResKey> = [
                        ResKey::parse(name, ResType::JUI),
                        ResKey::parse(name, ResType::TXT),
                        ResKey::parse(&format!("{name}_o"), ResType::NSS),
                        ResKey::parse(&format!("{name}_e"), ResType::NSS),
                        ResKey::parse(&format!("{name}_e"), ResType::NCS),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();
                    if let Some(ws) = &app.ws
                        && let (Some(doc), Some(settings)) = (
                            ResKey::parse(name, ResType::JUI)
                                .and_then(|k| ws.module.get(&k))
                                .and_then(|b| mg_nui::parse(b).ok()),
                            ResKey::parse(name, ResType::TXT)
                                .and_then(|k| ws.module.get(&k))
                                .and_then(|b| Settings::parse(b).ok()),
                        )
                    {
                        for image in mg_nui::image_names(&doc, &settings) {
                            for ty in [
                                ResType::PNG,
                                ResType::TGA,
                                ResType::DDS,
                                ResType::JPG,
                                ResType::BMP,
                                ResType::TXI,
                            ] {
                                if let Some(k) =
                                    ResKey::parse(&image, ty).filter(|k| ws.module.contains(k))
                                {
                                    resources.push(k);
                                }
                            }
                        }
                    }
                    app.actions.push(Action::ExportDialog(resources));
                }
            }
            Err(e) => app.log.error(e.to_string()),
        },
        Err(e) => app.log.error(e.to_string()),
    }
}

/// The NUI Creator, over its OK and Cancel (`blueprint::ok_cancel`): Cancel
/// sets the window and its settings back to what they were when the tab was
/// opened (what Build made of them is built again from those).
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    if app.ws.is_none() {
        return editor(app, ui, key);
    }
    let targets = [(key, mg_edit::GffPath::root())];
    let pressed = crate::blueprint::ok_cancel(ui, &targets[0], false);
    let pending = app.actions.len();
    editor(app, ui, key);
    // The event script is not Cancel's to set back: code written in its
    // sections since the tab was opened (in the script editor, saved) is
    // the builder's, and went with the rest. It is rebuilt around the
    // handlers Cancel brings back, its sections' code kept as Build keeps
    // it (a handler's that is gone again: commented out under it).
    let name = key.resref.to_string();
    let kept = events_script(&name);
    let settings_key = ResKey::new(key.resref, ResType::TXT);
    let tab = Tab::Nui(key);
    crate::blueprint::window_drawn_but(app, pending, &tab, &targets, pressed, &kept, |app, set| {
        let settings = set.iter().find_map(|e| match e {
            Edit::SetResource { key, data } if *key == settings_key => data.as_deref(),
            _ => None,
        });
        events::after_cancel(app, &name, settings).into_iter().collect()
    });
}

/// A window's event script, as the resources Cancel leaves alone.
fn events_script(name: &str) -> Vec<ResKey> {
    ResKey::parse(&format!("{name}_e"), ResType::NSS).into_iter().collect()
}

/// The window and its settings as text, for one frame: as the module holds
/// them, and as the frame leaves them. Where the two differ, [`changes`]
/// writes the text.
struct Frame {
    /// The window: its JUI.
    key: ResKey,
    /// The window's name.
    name: String,
    /// The window's text as the frame leaves it.
    raw: String,
    /// The window's text as the module holds it.
    original: String,
    /// The settings: the TXT of the window's name.
    config_key: ResKey,
    /// The settings' text as the frame leaves it.
    config_raw: String,
    /// The settings' text as the module holds it (a window without
    /// settings: those of a new one).
    config_original: String,
}

/// The window and its settings as read from the module's text, and what the
/// checks say of them.
struct Checked {
    window: Result<Value, String>,
    settings: Result<Settings, String>,
    findings: std::rc::Rc<[mg_nui::Diagnostic]>,
    /// The window's scripts are not what Build would make of it now.
    needs_build: bool,
    /// The errors: the findings that are, and a document that does not read.
    errors: usize,
    warnings: usize,
}

/// The Creator itself: the header, the pages' row, the problems, the page
/// chosen, and at the end one command for what the frame changed.
fn editor(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    // What the tab asks for this frame starts here (its OK and Cancel note it).
    let asked = app.actions.len();
    let id = ui.id().with(("nui", key));
    // Floating dock windows have no native title bar, so egui permits dragging
    // their background. Absorb body drags before registering controls; child
    // text selections, sliders and canvas handles retain their own drag sense.
    // (Its own room only: OK and Cancel under it are not its body.)
    let body = ui.available_rect_before_wrap();
    ui.interact(body, id.with("body-drag-guard"), egui::Sense::drag());
    let mut state = app.nui_views.remove(&key).unwrap_or_default();
    let Some((mut frame, checked)) = open(app, ui, key, &mut state) else { return };
    header(app, ui, &frame.name, &checked);
    navigation(app, ui, &frame.name, &checked, &mut state);
    ui.separator();
    problems(ui, &mut frame, &checked, &mut state);
    fields::begin(ui.ctx());
    leave_source(&mut state, &mut frame.raw, &mut frame.config_raw);
    let page_edits = match state.page {
        Page::Images => images_page(app, ui, &mut frame, checked, &mut state),
        Page::Events => events_page(app, ui, &mut frame, checked, &mut state),
        Page::JuiSource | Page::BindingJson => source_page(ui, &mut frame, &mut state),
        Page::Bindings => bindings_page(ui, &mut frame, checked, &mut state),
        Page::Design => design_page(app, ui, &mut frame, checked, &mut state),
    };
    let name = frame.name.clone();
    let edits = changes(app, ui.ctx(), frame, &mut state, page_edits);
    if !edits.is_empty() {
        // Apply before a generation requested in this frame, including a text
        // field losing focus when its button was clicked.
        app.actions.insert(asked, Action::Apply(Command::new(format!("Edit NUI {name}"), edits)));
    }
    app.nui_views.insert(key, state);
}

/// The window `key` of the open module as this frame works on it, read and
/// checked; where there is nothing to edit, a line saying why.
fn open(app: &Moonglow, ui: &mut Ui, key: ResKey, state: &mut State) -> Option<(Frame, Checked)> {
    let Some(ws) = &app.ws else {
        ui.label("Open a module first.");
        return None;
    };
    let frame = texts(ui, &ws.module, key)?;
    let checked = check(&ws.module, &frame, state);
    Some((frame, checked))
}

/// The module's text of the window and of its settings; a line saying why
/// for a window that is gone or is not text.
fn texts(ui: &mut Ui, module: &mg_module::Module, key: ResKey) -> Option<Frame> {
    let Some(bytes) = module.get(&key) else {
        ui.label("This window is no longer in the module.");
        return None;
    };
    let Ok(source) = std::str::from_utf8(bytes) else {
        ui.label(
            "This window's JUI is not UTF-8 text: it is kept as it is and can't be edited here.",
        );
        return None;
    };
    let raw = source.to_owned();
    let original = raw.clone();
    let name = key.resref.to_string();
    let config_key = ResKey::new(key.resref, ResType::TXT);
    let config_raw = if let Some(config_bytes) = module.get(&config_key) {
        match std::str::from_utf8(config_bytes) {
            Ok(s) => s.to_owned(),
            Err(_) => {
                ui.label(
                    "This window's settings (its TXT) are not UTF-8 text: they are kept as they \
                     are.",
                );
                return None;
            }
        }
    } else {
        String::from_utf8(Settings::default().bytes()).unwrap_or_default()
    };
    let config_original = config_raw.clone();
    Some(Frame { key, name, raw, original, config_key, config_raw, config_original })
}

/// The window and its settings parsed, and the checks run on them: each
/// again only when the module holds other bytes than last frame, kept in
/// `state` between.
fn check(module: &mg_module::Module, frame: &Frame, state: &mut State) -> Checked {
    let name = &frame.name;
    // Parsed when the module holds other bytes than last frame, not every
    // frame: the pages below work on copies of it.
    let held = |key: Option<ResKey>| key.and_then(|k| module.shared(&k).cloned());
    let of = [held(Some(frame.key)), held(Some(frame.config_key))];
    if state.source.as_ref().is_none_or(|s| !same(&s.of, &of)) {
        // (A window is an object: text that reads as a list or a number
        // is not one, and the pages, which set its fields, are not shown
        // it.)
        let window = mg_nui::parse(frame.raw.as_bytes()).map_err(|e| e.to_string()).and_then(|v| {
            match v.is_object() {
                true => Ok(v),
                false => Err("A window is a JSON object ({ … }): this text is not one".to_string()),
            }
        });
        if let Ok(doc) = &window {
            state.main_doc = Some(doc.clone());
            state.layout_groups = mg_nui::group_ids(doc);
        }
        let settings = Settings::parse(frame.config_raw.as_bytes()).map_err(|e| e.to_string());
        state.source = Some(Source { of: of.clone(), window, settings });
    }
    let (parsed, settings) =
        state.source.as_ref().map(|s| (s.window.clone(), s.settings.clone())).expect("just set");
    // Checking the whole document is not free: only redo it when the window,
    // its settings or its generated scripts change.
    let checked: Vec<Held> = of
        .into_iter()
        .chain(
            [
                ("_o", ResType::NSS),
                ("_o", ResType::NCS),
                ("_e", ResType::NSS),
                ("_e", ResType::NCS),
            ]
            .map(|(suffix, ty)| held(ResKey::parse(&format!("{name}{suffix}"), ty))),
        )
        .collect();
    if state.checks.as_ref().is_none_or(|c| !same(&c.of, &checked)) {
        let (findings, needs_build) = match (&parsed, &settings) {
            (Ok(v), Ok(s)) => (mg_nui::validate(v, s), !mg_nui::is_current(module, name, v, s)),
            _ => (Vec::new(), false),
        };
        state.checks = Some(Checks { of: checked, findings: findings.into(), needs_build });
    }
    let (findings, needs_build) =
        state.checks.as_ref().map(|c| (c.findings.clone(), c.needs_build)).expect("just set");
    let errors = findings.iter().filter(|d| d.severity == Severity::Error).count()
        + usize::from(parsed.is_err())
        + usize::from(settings.is_err());
    let warnings = findings.iter().filter(|d| d.severity == Severity::Warning).count();
    Checked { window: parsed, settings, findings, needs_build, errors, warnings }
}

/// The row over the pages: Export…, Build & compile with what the last
/// Build said, and How to open it….
fn header(app: &mut Moonglow, ui: &mut Ui, name: &str, checked: &Checked) {
    // Narrow, the build controls take a row of their own: right-aligned they
    // would be drawn over the ones on the left.
    let wide = ui.available_width() >= 900.0;
    ui.horizontal_wrapped(|ui| {
        let layout = if wide {
            egui::Layout::right_to_left(egui::Align::Center)
        } else {
            ui.end_row();
            egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true)
        };
        ui.with_layout(layout, |ui| {
            if ui
                .button("Export…")
                .on_hover_text(
                    "Builds the window, then exports it with its settings, its opener include \
                     and its compiled event script",
                )
                .clicked()
            {
                app.actions.push(Action::GenerateNui { name: name.to_owned(), export: true });
            }
            if ui
                .add_enabled(
                    checked.errors == 0,
                    egui::Button::new("Build & compile").fill(ui.visuals().selection.bg_fill),
                )
                .on_hover_text(
                    "Checks the opener include and compiles the event script. The window opens \
                     from a script of yours: How to open it… shows what to write.",
                )
                .on_disabled_hover_text(format!(
                    "Fix the {} first: the list under the tabs shows them",
                    count(checked.errors, "error")
                ))
                .clicked()
            {
                app.actions.push(Action::GenerateNui { name: name.to_owned(), export: false });
            }
            if let Some((_, why)) = app.nui_build_error.as_ref().filter(|(n, _)| n == name) {
                ui.colored_label(ui.visuals().error_fg_color, "Build failed").on_hover_text(why);
            } else if checked.needs_build {
                ui.weak("Not built")
                    .on_hover_text("Build before testing: the game runs what was last built");
            }
            // The window opens from a script of yours: what to put in it.
            ui.menu_button("How to open it…", |ui| {
                let code = format!(
                    "#include \"{name}_o\"\n\n\
                     void main()\n{{\n    Open_{name}(GetEnteringObject());\n}}\n"
                );
                ui.label("Open it from a script of yours, for example the module's OnClientEnter:");
                ui.add(egui::TextEdit::multiline(&mut code.as_str()).code_editor().desired_rows(6));
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(code.clone());
                    ui.close();
                }
                ui.weak("Pass the player who should see it. Build & compile first.");
            });
        });
    });
}

/// The pages' row: the pages, Event script…, how Design shows the window,
/// the Advanced menu (the source pages, Images, the opener) and the checks'
/// count, which opens the list of problems.
fn navigation(app: &mut Moonglow, ui: &mut Ui, name: &str, checked: &Checked, state: &mut State) {
    let Checked { settings, errors, warnings, .. } = checked;
    egui::ScrollArea::horizontal().id_salt("nui-navigation").auto_shrink([false, true]).show(
        ui,
        |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.selectable_value(&mut state.page, Page::Design, "Design");
                ui.selectable_value(&mut state.page, Page::Bindings, "Bindings");
                ui.selectable_value(&mut state.page, Page::Events, "Events & variants");
                if ui.add_enabled(settings.is_ok(), egui::Button::new("Event script…")).clicked()
                    && let Ok(s) = settings
                {
                    state.event_script = Some(events::EventScript {
                        settings: s.clone(),
                        handler: None,
                        code: None,
                    });
                }
                if state.page == Page::Design {
                    egui::ComboBox::from_id_salt("nui-layout-mode")
                        .selected_text(state.view_mode.label())
                        .show_ui(ui, |ui| {
                            for mode in ViewMode::ALL {
                                ui.selectable_value(&mut state.view_mode, mode, mode.label());
                            }
                        });
                }
                ui.menu_button("Advanced", |ui| {
                    if ui.selectable_label(state.page == Page::JuiSource, "JUI source").clicked() {
                        state.page = Page::JuiSource;
                        ui.close();
                    }
                    if ui
                        .selectable_label(state.page == Page::BindingJson, "Binding JSON")
                        .clicked()
                    {
                        state.page = Page::BindingJson;
                        ui.close();
                    }
                    if ui.selectable_label(state.page == Page::Images, "Images").clicked() {
                        state.page = Page::Images;
                        ui.close();
                    }
                    ui.separator();
                    let there = |k: &ResKey| app.ws.as_ref().is_some_and(|w| w.module.contains(k));
                    if let Some(k) = ResKey::parse(&format!("{name}_o"), ResType::NSS)
                        && ui.add_enabled(there(&k), egui::Button::new("Opener script")).clicked()
                    {
                        app.actions.push(Action::OpenTab(Tab::Script(k)));
                        ui.close();
                    }
                });
                ui.separator();
                let label = if errors + warnings == 0 {
                    "Checks passed".to_owned()
                } else {
                    format!("{} · {}", count(*errors, "error"), count(*warnings, "warning"))
                };
                ui.toggle_value(&mut state.issues, label)
                    .on_hover_text("What the checks of the window found");
            });
        },
    );
}

/// The list of problems, when it is open (new errors open it): what does
/// not read, then the findings, errors first, each with the control it is
/// about (a link that selects it) and the fixes there are for it.
fn problems(ui: &mut Ui, frame: &mut Frame, checked: &Checked, state: &mut State) {
    let Checked { window: parsed, settings, findings, errors, warnings, .. } = checked;
    // New errors open the list; the toggle closes it again.
    if *errors > state.issues_seen {
        state.issues = true;
    }
    state.issues_seen = *errors;
    if !state.issues {
        return;
    }
    egui::ScrollArea::vertical().id_salt("nui-diagnostics").max_height(160.0).show(ui, |ui| {
        if let Err(e) = parsed {
            ui.colored_label(ui.visuals().error_fg_color, format!("JUI: {e}"));
        }
        if let Err(e) = settings {
            ui.colored_label(ui.visuals().error_fg_color, format!("Settings: {e}"));
        }
        // Errors first: they stop a Build.
        let mut sorted: Vec<_> = findings.iter().enumerate().collect();
        sorted.sort_by_key(|(_, d)| d.severity != Severity::Error);
        for (i, d) in sorted {
            let (target, place) =
                parsed.as_ref().map_or((None, String::new()), |doc| describe_path(doc, &d.path));
            ui.push_id(i, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let (mark, color) = if d.severity == Severity::Error {
                        ("Error", ui.visuals().error_fg_color)
                    } else {
                        ("Warning", ui.visuals().warn_fg_color)
                    };
                    ui.colored_label(color, mark);
                    let place = egui::RichText::new(&place).strong();
                    match &target {
                        Some(path) => {
                            if ui.link(place).on_hover_text("Select it").clicked() {
                                state.selected = path.clone();
                                state.page = Page::Design;
                            }
                        }
                        None => {
                            ui.label(place);
                        }
                    }
                    ui.label(&d.message);
                    fixes(ui, frame, checked, d, target.as_deref());
                })
            });
        }
        if errors + warnings == 0 {
            ui.weak("The checks found nothing.");
        }
    });
    ui.separator();
}

/// The buttons after a finding that put it right: Bind it, Turn it off, Add
/// Close handler. `target` is the control the finding is about.
fn fixes(
    ui: &mut Ui,
    frame: &mut Frame,
    checked: &Checked,
    d: &mg_nui::Diagnostic,
    target: Option<&str>,
) {
    let Checked { window: parsed, settings, .. } = checked;
    if d.message.starts_with("What the player enters here is lost")
        && let (Some(path), Ok(doc), Ok(s)) = (target, parsed, settings)
        && ui.small_button("Bind it").on_hover_text("Its value becomes a new bind").clicked()
    {
        let (mut doc, mut s) = (doc.clone(), s.clone());
        if let Some(node) = doc.pointer_mut(path) {
            let mut i = 1;
            while s.bindings.contains_key(&format!("value_{i}")) {
                i += 1;
            }
            let bind = format!("value_{i}");
            let value = node["value"].take();
            s.bindings.insert(bind.clone(), Binding { value, ..Default::default() });
            node["value"] = json!({"bind": bind});
            write_window(&mut frame.raw, &doc);
            write_settings(&mut frame.config_raw, &s);
        }
    }
    // Clip to control does nothing but harm, so it isn't
    // offered; one written by hand is turned off here.
    if d.path.ends_with("/draw_list_scissor")
        && let (Ok(doc), Ok(s)) = (parsed, settings)
        && ui.small_button("Turn it off").clicked()
    {
        let (mut doc, mut s) = (doc.clone(), s.clone());
        let at = d.path.strip_prefix("/views/").and_then(|p| p.split_once('/'));
        let flag = match at {
            Some((view, rest)) => {
                s.views.get_mut(view).and_then(|v| v.pointer_mut(&format!("/{rest}")))
            }
            None => doc.pointer_mut(&d.path),
        };
        if let Some(flag) = flag {
            *flag = json!(false);
            write_window(&mut frame.raw, &doc);
            write_settings(&mut frame.config_raw, &s);
        }
    }
    if d.message.starts_with("The mg_close button has no Clicked handler")
        && let Ok(s) = settings
        && ui.small_button("Add Close handler").clicked()
    {
        let mut s = s.clone();
        s.actions.push(mg_nui::Route {
            event: "click".into(),
            element: "mg_close".into(),
            action: mg_nui::Action::Close,
            extra: Default::default(),
        });
        write_settings(&mut frame.config_raw, &s);
    }
}

/// The Images page: the window's pictures, and what it adds to the module
/// (its edits).
fn images_page(
    app: &mut Moonglow,
    ui: &mut Ui,
    frame: &mut Frame,
    checked: Checked,
    state: &mut State,
) -> Vec<Edit> {
    let (Ok(mut v), Ok(s)) = (checked.window, checked.settings) else {
        repair(ui, state);
        return Vec::new();
    };
    let before = v.clone();
    let page_edits = images::page(ui, app, &frame.name, &mut v, &s, state);
    if v != before {
        write_window(&mut frame.raw, &v);
    }
    page_edits
}

/// The Events & variants page.
fn events_page(
    app: &mut Moonglow,
    ui: &mut Ui,
    frame: &mut Frame,
    checked: Checked,
    state: &mut State,
) -> Vec<Edit> {
    let (Ok(mut v), Ok(mut s)) = (checked.window, checked.settings) else {
        repair(ui, state);
        return Vec::new();
    };
    let name = &frame.name;
    let before = v.clone();
    let config_before = s.clone();
    let script = ResKey::parse(&format!("{name}_e"), ResType::NSS)
        .and_then(|k| events::current_script(app, &k));
    workflow::ui(ui, &mut v, &mut s, state, name, script.as_deref());
    fields::put_back(ui.ctx(), &before, &mut v);
    fields::put_back_settings(ui.ctx(), &config_before, &mut s);
    if v != before {
        write_window(&mut frame.raw, &v);
    }
    if s != config_before {
        write_settings(&mut frame.config_raw, &s);
    }
    Vec::new()
}

/// The JUI source and Binding JSON pages: the window's text, or its
/// settings', to be edited as text.
fn source_page(ui: &mut Ui, frame: &mut Frame, state: &mut State) -> Vec<Edit> {
    let is_jui = state.page == Page::JuiSource;
    widgets::section_heading(ui, if is_jui { "JUI source" } else { "Binding JSON" });
    ui.weak(
        "What is typed is set when the field is left. Text that doesn't read as JSON is \
         kept in the module as typed, and Undo takes it back.",
    );
    let (of, text) = if is_jui {
        (SourceOf::Window, &mut frame.raw)
    } else {
        (SourceOf::Settings, &mut frame.config_raw)
    };
    egui::ScrollArea::both().id_salt(("nui-source", state.page)).show(ui, |ui| {
        if let Some(typed) = source_field(ui, &mut state.source_draft, of, text, 26) {
            *text = typed;
        }
    });
    Vec::new()
}

/// The Bindings page: the binds, with the window where it names them.
fn bindings_page(ui: &mut Ui, frame: &mut Frame, checked: Checked, state: &mut State) -> Vec<Edit> {
    let Ok(mut s) = checked.settings else {
        ui.label("The window's settings don't read as JSON: Binding JSON shows them.");
        if ui.button("Open Binding JSON").clicked() {
            state.page = Page::BindingJson;
        }
        return Vec::new();
    };
    let before = s.clone();
    let mut doc = checked.window.as_ref().ok().cloned();
    let doc_before = doc.clone();
    design::bindings(ui, &mut s, doc.as_mut(), state);
    fields::put_back_settings(ui.ctx(), &before, &mut s);
    if let (Some(was), Some(doc)) = (&doc_before, &mut doc) {
        fields::put_back(ui.ctx(), was, doc);
    }
    if s != before {
        write_settings(&mut frame.config_raw, &s);
    }
    if doc != doc_before
        && let Some(doc) = doc
    {
        write_window(&mut frame.raw, &doc);
    }
    Vec::new()
}

/// The Design page: the variants' bar, then the window (or the variant
/// being edited, in the window's place) in the view chosen.
fn design_page(
    app: &mut Moonglow,
    ui: &mut Ui,
    frame: &mut Frame,
    checked: Checked,
    state: &mut State,
) -> Vec<Edit> {
    let (Ok(mut v), Ok(mut s)) = (checked.window, checked.settings) else {
        repair(ui, state);
        return Vec::new();
    };
    let before = v.clone();
    let before_settings = s.clone();
    layouts::bar(ui, &s, state);
    // (The bar may have left the variant whose source was typed in.)
    leave_source(state, &mut frame.raw, &mut frame.config_raw);
    let edited_view = state.edit_view.clone();
    // A variant is edited in the window's place, and put back after.
    let mut base_root = None;
    if let Some(view) = state.edit_view.as_ref().and_then(|n| s.views.get(n)) {
        base_root = Some(std::mem::replace(&mut v["root"], view.clone()));
    }
    // A selection whose control is gone (Undo took it back, the
    // source was edited) is no selection.
    if v.pointer(&state.selected).is_none() {
        state.selected.clear();
    }
    state.selected_many.retain(|path| v.pointer(path).is_some());
    if let Some((path, layer)) = &state.draw_item
        && v.pointer(&format!("{path}/draw_list/{layer}")).is_none()
    {
        state.draw_item = None;
    }
    let split_raw = design_view(app, ui, &mut v, &mut s, state);
    layouts::finish_insert(&v, &mut s, state);
    if let Some(name) = &edited_view {
        let root = match base_root {
            Some(base) => std::mem::replace(&mut v["root"], base),
            None => v["root"].clone(),
        };
        s.views.insert(name.clone(), root);
    }
    fields::put_back(ui.ctx(), &before, &mut v);
    fields::put_back_settings(ui.ctx(), &before_settings, &mut s);
    if v != before {
        write_window(&mut frame.raw, &v);
    }
    if s != before_settings {
        write_settings(&mut frame.config_raw, &s);
    }
    if let Some(source) = split_raw {
        if edited_view.is_some() {
            frame.config_raw = source;
        } else {
            frame.raw = source;
        }
    }
    Vec::new()
}

/// The Design page's view of `v` (the window, or the variant in its place):
/// the editor with its palette and properties, the canvas alone, or the
/// canvas beside the source, whose text is returned when it was typed in.
fn design_view(
    app: &mut Moonglow,
    ui: &mut Ui,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
) -> Option<String> {
    let ws = app.ws.as_ref()?;
    let (asset_doc, asset_settings) = state
        .runtime
        .as_ref()
        .filter(|r| state.preview_interactive && r.matches(v, s))
        .map_or((&*v, &*s), |r| (&r.doc, &r.settings));
    app.nui_assets.prepare(
        ui.ctx(),
        app.game.as_deref().map(|g| &g.resman),
        &ws.module,
        ws.revision(),
        asset_doc,
        asset_settings,
    );
    app.nui_assets.prepare_strings(app.game.as_deref(), asset_doc, asset_settings);
    let mut split_raw = None;
    match state.view_mode {
        ViewMode::Preview => preview::canvas(ui, v, s, state, &mut app.nui_assets),
        ViewMode::Split => ui.columns(2, |columns| {
            preview::canvas(&mut columns[0], v, s, state, &mut app.nui_assets);
            let (of, source, title) = if state.edit_view.is_some() {
                (
                    SourceOf::Variants,
                    serde_json::to_string_pretty(s).unwrap_or_default(),
                    "Variants and binding JSON",
                )
            } else {
                (
                    SourceOf::SplitWindow,
                    serde_json::to_string_pretty(v).unwrap_or_default(),
                    "JUI source",
                )
            };
            widgets::section_heading(&mut columns[1], title);
            egui::ScrollArea::both().id_salt("split-jui").show(&mut columns[1], |ui| {
                split_raw = source_field(ui, &mut state.source_draft, of, &source, 28);
            });
        }),
        ViewMode::Editor => design::editor(ui, v, s, state, &app.keymap, &mut app.nui_assets),
    }
    split_raw
}

/// What the frame changed, as the edits of one command: the page's own
/// (`edits`), a picture chosen from disk, the event script where it follows
/// the handlers or was asked for, and the window's and the settings' text
/// where the frame left them other than the module holds them.
fn changes(
    app: &mut Moonglow,
    ctx: &egui::Context,
    frame: Frame,
    state: &mut State,
    mut edits: Vec<Edit>,
) -> Vec<Edit> {
    let Frame { key, name, raw, original, config_key, mut config_raw, config_original } = frame;
    edits.extend(images::answer_from_disk(ctx, app));
    if (raw != original || config_raw != config_original)
        && let Some(edit) =
            events::follow_controls(app, &name, &original, &raw, &config_original, &mut config_raw)
                .or_else(|| events::sync_script(app, &name, &config_original, &config_raw))
    {
        edits.push(edit);
    }
    if let Some(ask) = state.event_script.take()
        && let Some(edit) = events::open_script(app, &name, ask)
    {
        edits.push(edit);
    }
    if raw != original {
        edits.push(Edit::SetResource { key, data: Some(raw.into_bytes()) });
    }
    if config_raw != config_original {
        edits.push(Edit::SetResource { key: config_key, data: Some(config_raw.into_bytes()) });
    }
    edits
}

fn array_position(path: &str) -> Option<(String, usize, bool)> {
    let cell = path.ends_with("/0") && path.rsplit('/').nth(2) == Some("row_template");
    let path = if cell { path.strip_suffix("/0")? } else { path };
    let (parent, index) = path.rsplit_once('/')?;
    if !parent.ends_with("/children") && !parent.ends_with("/row_template") {
        return None;
    }
    Some((parent.into(), index.parse().ok()?, cell))
}

/// A source field: the text a page shows to be edited as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SourceOf {
    /// The JUI source page: the window's text as the module holds it.
    Window,
    /// The Binding JSON page: the settings' text as the module holds it.
    Settings,
    /// Split source: the window, written out.
    SplitWindow,
    /// Split source while a variant is edited: the settings, written out.
    Variants,
}

/// What is typed in a source field, until the field is left: a draft that
/// doesn't read yet isn't written to the module at every key.
struct SourceDraft {
    of: SourceOf,
    text: String,
    /// Typed in since it was last set.
    typed: bool,
}

/// A source field that isn't shown any more (another page, another view,
/// another thing edited) was left: what was typed in it is set.
fn leave_source(state: &mut State, raw: &mut String, config_raw: &mut String) {
    let shows = match (state.page, state.view_mode) {
        (Page::JuiSource, _) => Some(SourceOf::Window),
        (Page::BindingJson, _) => Some(SourceOf::Settings),
        (Page::Design, ViewMode::Split) if state.edit_view.is_some() => Some(SourceOf::Variants),
        (Page::Design, ViewMode::Split) => Some(SourceOf::SplitWindow),
        _ => None,
    };
    if let Some(draft) = state.source_draft.take_if(|d| Some(d.of) != shows)
        && draft.typed
    {
        match draft.of {
            SourceOf::Window | SourceOf::SplitWindow => *raw = draft.text,
            SourceOf::Settings | SourceOf::Variants => *config_raw = draft.text,
        }
    }
}

/// A source field showing `current`, `rows` high: the text typed in it,
/// once, when the keyboard leaves it (also one that doesn't read as JSON:
/// it is kept in the module as typed, where Undo takes it back).
fn source_field(
    ui: &mut Ui,
    held: &mut Option<SourceDraft>,
    of: SourceOf,
    current: &str,
    rows: usize,
) -> Option<String> {
    let draft = held.take_if(|d| d.of == of).unwrap_or_else(|| SourceDraft {
        of,
        text: current.to_owned(),
        typed: false,
    });
    let draft = held.insert(draft);
    let field = ui.add(
        egui::TextEdit::multiline(&mut draft.text)
            .id(ui.id().with(("nui-source-field", of)))
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(rows),
    );
    crate::widgets::leaves_at_a_press(ui.ctx(), &field);
    draft.typed |= field.changed();
    if crate::widgets::has_keys(&field) {
        return None;
    }
    let typed = std::mem::take(&mut draft.typed) && draft.text != current;
    if !typed && draft.text != current {
        // Changed underneath (Undo, the form): show it.
        draft.text = current.to_owned();
    }
    typed.then(|| draft.text.clone())
}

/// The window's JSON text into `raw`. (Left as it is should it not write,
/// which a document of JSON values always does.)
fn write_window(raw: &mut String, v: &Value) {
    if let Ok(text) = serde_json::to_string_pretty(v) {
        *raw = text;
    }
}

/// The settings' text into `config_raw`, as [`write_window`].
fn write_settings(config_raw: &mut String, s: &Settings) {
    if let Ok(text) = String::from_utf8(s.bytes()) {
        *config_raw = text;
    }
}

fn rearrange(v: &mut Value, selected: &mut String, op: Op) {
    let Some((parent, i, cell)) = array_position(selected) else { return };
    let Some(a) = v.pointer(&parent).and_then(Value::as_array) else { return };
    if i >= a.len() {
        return;
    }
    // Group must retain its single child. Replace its child's contents through
    // the source editor, or edit the row/column nested inside it.
    let group = parent
        .strip_suffix("/children")
        .and_then(|p| v.pointer(p))
        .is_some_and(|n| n["type"] == "group");
    if group {
        return;
    }
    let mut copy = a[i].clone();
    assign_ids(&mut copy, v);
    let Some(a) = v.pointer_mut(&parent).and_then(Value::as_array_mut) else { return };
    let mut at = i;
    match op {
        Op::MoveUp if i > 0 => {
            a.swap(i, i - 1);
            at -= 1;
        }
        Op::MoveDown if i + 1 < a.len() => {
            a.swap(i, i + 1);
            at += 1;
        }
        Op::Duplicate => {
            a.insert(i + 1, copy);
            at += 1;
        }
        Op::Delete => {
            a.remove(i);
            *selected = if a.is_empty() {
                parent.rsplit_once('/').map_or("/root", |(p, _)| p).into()
            } else {
                format!("{parent}/{}{}", i.min(a.len() - 1), if cell { "/0" } else { "" })
            };
            return;
        }
        Op::MoveUp | Op::MoveDown => {}
    }
    *selected = format!("{parent}/{at}{}", if cell { "/0" } else { "" });
}

/// A page that needs the window's JSON when it doesn't read.
fn repair(ui: &mut Ui, state: &mut State) {
    ui.label(REPAIR);
    ui.horizontal(|ui| {
        if ui.button("Open JUI source").clicked() {
            state.page = Page::JuiSource;
        }
        if ui.button("Open Binding JSON").clicked() {
            state.page = Page::BindingJson;
        }
    });
}

/// What a page says when the window's JSON doesn't read.
const REPAIR: &str = "The window's JSON doesn't read: JUI source and Binding JSON show it as it \
                      is, to be put right.";

/// IDs for a copy going into `document`: one it already has is kept unless
/// the document uses it (a paste into another window keeps its handlers' names).
fn assign_ids(node: &mut Value, document: &Value) {
    fn ids(v: &Value, set: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(id) = o.get("id").and_then(Value::as_str) {
                    set.insert(id.into());
                }
                for v in o.values() {
                    ids(v, set);
                }
            }
            Value::Array(a) => {
                for v in a {
                    ids(v, set);
                }
            }
            _ => {}
        }
    }
    fn assign(v: &mut Value, set: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                let own = o.get("id").and_then(Value::as_str).filter(|id| !set.contains(*id));
                if let Some(id) = own {
                    set.insert(id.to_owned());
                } else if let Some(ty) = o.get("type").and_then(Value::as_str) {
                    let mut i = 1;
                    while set.contains(&format!("{ty}_{i}")) {
                        i += 1;
                    }
                    let id = format!("{ty}_{i}");
                    set.insert(id.clone());
                    o.insert("id".into(), id.into());
                }
                for v in o.values_mut() {
                    assign(v, set);
                }
            }
            Value::Array(a) => {
                for v in a {
                    assign(v, set);
                }
            }
            _ => {}
        }
    }
    let mut used = std::collections::BTreeSet::new();
    ids(document, &mut used);
    assign(node, &mut used);
}

/// The room under a property: its label is over its field, so one property
/// stands apart from the next by more than a label from its field.
const FIELD_GAP: f32 = 8.0;

/// The properties of the control selected (or of the window): the Title
/// bar checkbox, the sections with the fields that are set, + Add property
/// for those that are not, and Advanced properties for what is left.
fn properties(ui: &mut Ui, node: &mut Value, s: &mut Settings) {
    let Some(obj) = node.as_object_mut() else {
        ui.weak("Edit this value in Advanced › JUI source.");
        return;
    };
    let ty = obj.get("type").and_then(Value::as_str).unwrap_or("window").to_owned();
    if ty == "window" {
        title_bar(ui, obj, s);
    }
    let reveal_id = ui.id().with("reveal-nui-property");
    let reveal = ui.ctx().data_mut(|d| d.remove_temp::<String>(reveal_id));
    let mut remove = None;
    for (section, keys, open) in SECTIONS {
        let layout_sizes = *section == "Size & layout" && ty != "window";
        if !layout_sizes
            && !keys.iter().any(|k| {
                obj.get(*k).is_some_and(|v| !v.is_null() || property_default(&ty, k).is_some())
            })
        {
            continue;
        }
        let revealing = reveal.as_deref().is_some_and(|key| keys.contains(&key));
        egui::CollapsingHeader::new(*section)
            .id_salt(("nui-properties", section))
            .open(revealing.then_some(true))
            .default_open(*open)
            .show(ui, |ui| {
                for key in *keys {
                    if layout_sizes && matches!(*key, "width" | "height") && !obj.contains_key(*key)
                    {
                        let (label, fixed) =
                            if *key == "width" { ("Width", 150.0) } else { ("Height", 30.0) };
                        ui.horizontal(|ui| {
                            widgets::field_label(ui, label);
                            // It has none: the layout decides; the button gives it one.
                            ui.weak("Automatic");
                            if ui
                                .small_button("Set size")
                                .on_hover_text("Give it a fixed size instead")
                                .clicked()
                            {
                                obj.insert((*key).into(), json!(fitting_size(obj, key, fixed)));
                            }
                        });
                        ui.add_space(FIELD_GAP);
                        continue;
                    }
                    if let Some(value) = obj.get_mut(*key)
                        && (!value.is_null() || property_default(&ty, key).is_some())
                        && !(*key == "title" && *value == json!(false))
                        // Optional window settings show once added.
                        && !(value.is_null()
                            && matches!(*key, "size_constraint" | "edge_constraint" | "collapsed"))
                    {
                        let field = ui.push_id(*key, |ui| {
                            property(ui, key, &ty, value, s, &mut remove);
                            ui.add_space(FIELD_GAP);
                        });
                        if reveal.as_deref() == Some(*key) {
                            field.response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                }
            });
    }
    add_property(ui, obj, &ty, reveal_id);
    let advanced_keys = advanced_keys(obj, &ty);
    if !advanced_keys.is_empty() {
        egui::CollapsingHeader::new("Advanced properties").show(ui, |ui| {
            for key in &advanced_keys {
                let Some(value) = obj.get_mut(key) else { continue };
                ui.push_id(key, |ui| {
                    property(ui, key, &ty, value, s, &mut remove);
                });
            }
        });
    }
    if let Some(key) = remove {
        obj.remove(&key);
    }
}

/// The properties' sections in the order shown: the title, the fields of
/// each, and whether it starts open.
const SECTIONS: &[(&str, &[&str], bool)] = &[
    ("Content", &["title", "id", "label", "value", "elements"], true),
    (
        "Size & layout",
        &[
            "geometry",
            "width",
            "height",
            "aspect",
            "margin",
            "padding",
            "row_height",
            "row_count",
            "direction",
            "size_constraint",
            "edge_constraint",
        ],
        true,
    ),
    ("Tooltips", &["tooltip", "disabled_tooltip"], true),
    (
        "Appearance",
        &[
            "font",
            "foreground_color",
            "border",
            "text_halign",
            "text_valign",
            "image_aspect",
            "image_halign",
            "image_valign",
            "image_region",
        ],
        false,
    ),
    (
        "Behavior",
        &[
            "enabled",
            "visible",
            "encouraged",
            "scrollbars",
            "collapsed",
            "resizable",
            "closable",
            "transparent",
            "accepts_input",
            "multiline",
            "wordwrap",
            "min",
            "max",
            "step",
        ],
        false,
    ),
];

/// The window's Title bar checkbox. NWN EE 8193.37 draws no title bar for a
/// window whose title is false and that has neither a close nor a collapse
/// button: hiding it keeps all three in the settings, to bring them back.
fn title_bar(ui: &mut Ui, obj: &mut serde_json::Map<String, Value>, s: &mut Settings) {
    let mut shown = obj.get("title") != Some(&json!(false));
    if ui
        .checkbox(&mut shown, "Title bar")
        .on_hover_text("Without it the window has no title, close or collapse button")
        .changed()
    {
        const BAR: [&str; 3] = ["title", "closable", "collapsed"];
        if shown {
            let kept = s.extra.remove("title_bar").unwrap_or_default();
            for key in BAR {
                obj.insert(key.into(), kept.get(key).cloned().unwrap_or(json!("")));
            }
            if !kept.is_object() {
                obj.insert("closable".into(), json!(true));
                obj.insert("collapsed".into(), Value::Null);
            }
        } else {
            let kept: serde_json::Map<_, _> = BAR
                .iter()
                .map(|k| ((*k).into(), obj.get(*k).cloned().unwrap_or_default()))
                .collect();
            s.extra.insert("title_bar".into(), Value::Object(kept));
            obj.insert("title".into(), json!(false));
            obj.insert("closable".into(), json!(false));
            obj.insert("collapsed".into(), json!(false));
        }
    }
}

/// The + Add property menu: the properties an element (or the window) of
/// type `ty` can have and has not; the one added is shown (`reveal_id`).
fn add_property(
    ui: &mut Ui,
    obj: &mut serde_json::Map<String, Value>,
    ty: &str,
    reveal_id: egui::Id,
) {
    let (fit_width, fit_height) =
        (fitting_size(obj, "width", 150.0), fitting_size(obj, "height", 30.0));
    ui.menu_button("+ Add property", |ui| {
        for (k, label, val) in [
            ("id", "Element ID", json!("element")),
            ("width", "Width", json!(fit_width)),
            ("height", "Height", json!(fit_height)),
            // The client's own margin: adding the field changes nothing yet.
            ("margin", "Margin", json!(2.0)),
            ("padding", "Padding", json!(0.0)),
            ("enabled", "Enabled", json!(true)),
            ("visible", "Visible", json!(true)),
            ("tooltip", "Tooltip", json!("")),
            ("disabled_tooltip", "Tooltip when disabled", json!("")),
            ("encouraged", "Encouraged", json!(false)),
            ("foreground_color", "Foreground color", json!({"r":255,"g":255,"b":255,"a":255})),
            ("size_constraint", "Window size limits", json!({"x":0.0,"y":0.0,"w":0.0,"h":0.0})),
            (
                "edge_constraint",
                "Distance from screen edges",
                json!({"x":0.0,"y":0.0,"w":0.0,"h":0.0}),
            ),
            ("collapsed", "Collapsed", json!(false)),
            ("aspect", "Aspect ratio", json!(1.0)),
            ("font", "Font", json!("")),
        ] {
            let applicable = match k {
                "size_constraint" | "edge_constraint" | "collapsed" => ty == "window",
                // Window size is edited through geometry; element modifiers are not window options.
                _ => ty != "window",
            };
            if applicable && obj.get(k).is_none_or(Value::is_null) && ui.button(label).clicked() {
                obj.insert(k.into(), val);
                ui.ctx().data_mut(|d| d.insert_temp(reveal_id, k.to_owned()));
                ui.ctx().request_repaint();
                ui.close();
            }
        }
    });
}

/// The fields under Advanced properties: those no section shows and no
/// editor of its own has.
fn advanced_keys(obj: &serde_json::Map<String, Value>, ty: &str) -> Vec<String> {
    obj.iter()
        .filter_map(|(key, value)| {
            if matches!(key.as_str(), "type" | "version" | "root" | "children" | "row_template") {
                return None;
            }
            // NuiElement's unused label/value slots are null placeholders, not
            // additional authorable properties (e.g. Label.label, Button.value).
            if value.is_null()
                && matches!(key.as_str(), "label" | "value")
                && property_default(ty, key).is_none()
            {
                return None;
            }
            if SECTIONS.iter().any(|(_, keys, _)| keys.contains(&key.as_str()))
                && (!value.is_null() || property_default(ty, key).is_some())
            {
                return None;
            }
            if (key == "draw_list" && value.is_array())
                || (key == "draw_list_scissor" && value.is_boolean())
            {
                // These have a dedicated Draw layers editor, not a second raw editor.
                return None;
            }
            Some(key.clone())
        })
        .collect()
}

/// A fixed size for a row's height or a column's width that its children
/// fit in, margins included (the client refuses the window otherwise), and a
/// `fallback`-sized control too; for anything else, `fallback`.
fn fitting_size(node: &serde_json::Map<String, Value>, key: &str, fallback: f64) -> f64 {
    let across = match node.get("type").and_then(Value::as_str) {
        Some("row") => "height",
        Some("col") => "width",
        _ => return fallback,
    };
    if key != across {
        return fallback;
    }
    node.get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let size = c.get(key)?.as_f64()?;
            Some(size + 2.0 * c.get("margin").and_then(Value::as_f64).unwrap_or(2.0))
        })
        // Room for a new control of the default size and its margins too.
        .fold(fallback + 4.0, f64::max)
}

/// A missing value must use the property's type, not a generic JSON type menu.
/// Unknown/custom fields remain losslessly editable in the source editor.
fn property_default(ty: &str, key: &str) -> Option<Value> {
    let template = if ty == "window" {
        mg_nui::window()
    } else if mg_nui::ELEMENTS.contains(&ty) {
        mg_nui::template(ty)
    } else {
        Value::Null
    };
    if let Some(value) = template.get(key).filter(|v| !v.is_null()) {
        return Some(value.clone());
    }
    Some(match key {
        "id" | "tooltip" | "disabled_tooltip" | "font" => json!(""),
        "width" => json!(150.0),
        "height" => json!(30.0),
        "margin" => json!(2.0),
        "padding" => json!(0.0),
        "aspect" => json!(1.0),
        "enabled" | "visible" => json!(true),
        "encouraged" | "collapsed" => json!(false),
        "foreground_color" => json!({"r":255,"g":255,"b":255,"a":255}),
        "size_constraint" | "edge_constraint" | "image_region" => {
            json!({"x":0.0,"y":0.0,"w":0.0,"h":0.0})
        }
        _ => return None,
    })
}

fn bindable(ty: &str, key: &str) -> bool {
    (matches!(ty, "draw" | "chart_slot")
        || matches!(
            key,
            "value"
                | "label"
                | "title"
                | "geometry"
                | "size_constraint"
                | "edge_constraint"
                | "enabled"
                | "visible"
                | "tooltip"
                | "resizable"
                | "collapsed"
                | "closable"
                | "foreground_color"
                | "elements"
                | "row_count"
                | "min"
                | "step"
                | "image_region"
                | "image_aspect"
                | "image_halign"
                | "image_valign"
                | "font"
                | "text_halign"
                | "text_valign"
                | "disabled_tooltip"
                | "encouraged"
        )
        || (matches!(key, "border" | "transparent" | "accepts_input") && ty == "window")
        || (key == "max" && ty != "textedit"))
        && !matches!(key, "type" | "order" | "render" | "arrayBinds")
        && !(key == "value" && ty == "chart")
        && !(key == "elements" && matches!(ty, "options" | "tabbar"))
}

/// What a property is called wherever the Creator names it.
fn display_label(key: &str, ty: &str) -> String {
    if ty == "draw" {
        let name = match key {
            "rect" => Some("Position and size"),
            "points" => Some("Points (x, y, x, y, …)"),
            "a" => Some("Start"),
            "b" => Some("End"),
            "ctrl0" => Some("Control point 1"),
            "ctrl1" => Some("Control point 2"),
            "c" => Some("Center"),
            "amin" => Some("Start angle (radians)"),
            "amax" => Some("End angle (radians)"),
            "line_thickness" => Some("Line width"),
            _ => None,
        };
        if let Some(name) = name {
            return name.into();
        }
    }
    match key {
        "id" => "Element ID".into(),
        "disabled_tooltip" => "Tooltip when disabled".into(),
        "value" if matches!(ty, "label" | "text" | "textedit") => "Text".into(),
        "value" if ty == "image" => "Image".into(),
        "label" if ty == "button_image" => "Image".into(),
        "label" if ty == "textedit" => "Placeholder".into(),
        "geometry" => "Window bounds".into(),
        "size_constraint" => "Window size limits".into(),
        "edge_constraint" => "Distance from screen edges".into(),
        "draw_list_scissor" => "Clip to control".into(),
        "draw_list" => "Draw layers".into(),
        _ => {
            let mut label = key.replace('_', " ");
            if let Some(c) = label.get_mut(..1) {
                c.make_ascii_uppercase();
            }
            label
        }
    }
}

/// Where a check's path points, for people: the control (and its JSON
/// path, to select it) and the property.
fn describe_path(doc: &Value, path: &str) -> (Option<String>, String) {
    let mut at = path.to_owned();
    loop {
        let first = path.trim_start_matches('/').split('/').next().unwrap_or_default();
        let window = at.is_empty() && doc.get(first).is_some();
        if let Some(node) = doc.pointer(&at).filter(|n| n.get("type").is_some() || window) {
            let rest = path[at.len()..].trim_start_matches('/');
            let key = rest.split('/').next().unwrap_or_default();
            let ty = node["type"].as_str().unwrap_or("window");
            let name = if at.is_empty() { "Window".into() } else { design::node_name(node) };
            let text = if key.is_empty() {
                name
            } else {
                format!("{name} › {}", display_label(key, ty))
            };
            return (Some(at), text);
        }
        match at.rsplit_once('/') {
            Some((up, _)) => at = up.to_owned(),
            None => break,
        }
    }
    let what = match path.trim_start_matches('/').split('/').next().unwrap_or_default() {
        "actions" => "Events",
        "bindings" => "Binds",
        "settings" => "Window identity",
        "views" => "Swap layout variants",
        _ => "Window",
    };
    (None, what.into())
}

/// What a NUI window (`key`, a JUI) has beside it in the module: its
/// settings, its opener include and its event script, source and compiled.
/// None where the text of its name is not its settings: the scripts are
/// then not known to be its own.
pub(crate) fn companions(module: &mg_module::Module, key: ResKey) -> Vec<ResKey> {
    let settings = ResKey::new(key.resref, ResType::TXT);
    if key.restype != ResType::JUI
        || module.get(&settings).is_none_or(|b| Settings::parse(b).is_err())
    {
        return Vec::new();
    }
    let scripts = ["_o", "_e"].into_iter().flat_map(|suffix| {
        [ResType::NSS, ResType::NCS, ResType::NDB]
            .into_iter()
            .filter_map(move |ty| ResKey::parse(&format!("{}{suffix}", key.resref), ty))
    });
    std::iter::once(settings).chain(scripts).filter(|k| module.contains(k)).collect()
}

/// The module's NUI windows whose scripts are older than their design: the
/// game would run what was last built.
pub(crate) fn stale_windows(module: &mg_module::Module) -> Vec<String> {
    module
        .keys()
        .filter(|k| k.restype == ResType::JUI)
        .filter_map(|k| {
            let name = k.resref.to_string();
            let settings =
                Settings::parse(module.get(&ResKey::new(k.resref, ResType::TXT))?).ok()?;
            let window = mg_nui::parse(module.get(k)?).ok()?;
            (!mg_nui::is_current(module, &name, &window, &settings)).then_some(name)
        })
        .collect()
}

/// "1 error", "2 errors".
fn count(n: usize, what: &str) -> String {
    format!("{n} {what}{}", if n == 1 { "" } else { "s" })
}

fn property(
    ui: &mut Ui,
    key: &str,
    ty: &str,
    value: &mut Value,
    s: &mut Settings,
    remove: &mut Option<String>,
) {
    let label = display_label(key, ty);
    let can_bind = bindable(ty, key);
    let mut bind = value.get("bind").and_then(Value::as_str).map(str::to_owned);
    let heading = ui
        .horizontal(|ui| {
            let heading = if let Value::Bool(flag) = value {
                ui.checkbox(flag, &label)
            } else {
                widgets::field_label(ui, &label)
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("…", |ui| {
                    if let Some(name) = &bind {
                        if ui.button("Use a constant value").clicked() {
                            *value = s.bindings.get(name).map_or(Value::Null, |b| b.value.clone());
                            ui.close();
                        }
                    } else if can_bind && ui.button("Make dynamic (bind)").clicked() {
                        let mut i = 1;
                        while s.bindings.contains_key(&format!("{key}_{i}")) {
                            i += 1;
                        }
                        let name = format!("{key}_{i}");
                        s.bindings.insert(
                            name.clone(),
                            Binding { value: value.clone(), watch: false, ..Default::default() },
                        );
                        *value = json!({"bind":name});
                        ui.close();
                    }
                    if can_bind && !s.bindings.is_empty() {
                        ui.menu_button("Use existing bind", |ui| {
                            for name in s.bindings.keys() {
                                if ui.selectable_label(bind.as_ref() == Some(name), name).clicked()
                                {
                                    if value.get("bind").is_some() {
                                        value["bind"] = json!(name);
                                    } else {
                                        *value = json!({"bind":name});
                                    }
                                    ui.close();
                                }
                            }
                        });
                    }
                    let text_property = matches!(key, "title" | "tooltip" | "disabled_tooltip")
                        || (key == "value" && matches!(ty, "label" | "text" | "textedit"))
                        || (key == "label"
                            && matches!(ty, "button" | "button_select" | "check" | "textedit"))
                        || (key == "text" && ty == "draw")
                        || (key == "legend" && ty == "chart_slot");
                    if bind.is_none() && text_property {
                        if value.get("strref").is_some() {
                            if ui.button("Use typed text").clicked() {
                                *value = json!("");
                                ui.close();
                            }
                        } else if value.is_string()
                            && ui.button("Use TLK string (StrRef)").clicked()
                        {
                            *value = json!({"strref":0});
                            ui.close();
                        }
                    }
                    if !matches!(key, "geometry" | "title")
                        && ui.button("Remove property").clicked()
                    {
                        *remove = Some(key.into());
                        ui.close();
                    }
                });
            });
            heading
        })
        .inner;
    bind = value.get("bind").and_then(Value::as_str).map(str::to_owned);
    if key == "disabled_tooltip" {
        ui.weak(
            "Shown on hover while this control is disabled, e.g. why an action is unavailable.",
        );
    }
    if let Some(name) = bind {
        ui.label(
            egui::RichText::new(format!("Dynamic · {name}")).color(ui.visuals().selection.bg_fill),
        );
        if let Some(binding) = s.bindings.get_mut(&name) {
            widgets::field_label(ui, "Initial value");
            fields::value(ui, &mut binding.value, 0);
        } else {
            ui.weak("Add an initial value in Bindings.");
        }
        fields::binding_options(ui, value);
    } else if let Value::String(text) = value {
        if let Some(field) = fields::text(ui, text, f32::INFINITY) {
            field.labelled_by(heading.id);
        }
    } else if let Some(id) = value.get_mut("strref") {
        ui.horizontal(|ui| {
            widgets::field_label(ui, "TLK StrRef");
            scalar(ui, id);
        });
    } else if key == "elements"
        && let Some(elements) = value.as_array_mut()
    {
        fields::options(ui, elements, ty == "combo");
    } else if key == "size_constraint" && value.is_object() {
        for (field, label) in [
            ("x", "Minimum width"),
            ("y", "Minimum height"),
            ("w", "Maximum width"),
            ("h", "Maximum height"),
        ] {
            ui.horizontal(|ui| {
                let label = widgets::field_label(ui, label);
                if let Some(field) = scalar(ui, &mut value[field]) {
                    field.labelled_by(label.id);
                }
            });
        }
        ui.weak("0 leaves that limit unrestricted.");
    } else if value.is_null() {
        if let Some(default) = property_default(ty, key) {
            if ui.button("Set value…").clicked() {
                *value = default;
            }
        } else {
            ui.weak("Not set, or a value of another kind: edit it in Advanced › JUI source.");
        }
    } else if let Some(names) = named_choices(key)
        && let Some(n) = value.as_i64()
    {
        // The NUI_* constants of nw_inc_nui, by name.
        let mut n = n;
        egui::ComboBox::from_id_salt(key)
            .selected_text(names.get(n as usize).copied().unwrap_or("Other"))
            .show_ui(ui, |ui| {
                for (i, name) in names.iter().enumerate() {
                    ui.selectable_value(&mut n, i as i64, *name);
                }
            })
            .response
            .labelled_by(heading.id);
        *value = json!(n);
    } else if !value.is_boolean()
        && let Some(field) = scalar(ui, value)
    {
        field.labelled_by(heading.id);
    }
}

/// Names of the values of a property the stock API gives constants for.
fn named_choices(key: &str) -> Option<&'static [&'static str]> {
    Some(match key {
        "image_aspect" => {
            &["Fit", "Fill", "Fit, at most 100%", "Exact size", "Exact size, scaled", "Stretch"]
        }
        "image_halign" | "text_halign" => &["Center", "Left", "Right"],
        "image_valign" | "text_valign" => &["Middle", "Top", "Bottom"],
        "scrollbars" => &["None", "Horizontal", "Vertical", "Both", "When needed"],
        "direction" => &["Horizontal", "Vertical"],
        _ => return None,
    })
}

/// A value's field: text, a number or a color, set as the other editors set
/// theirs (see `fields`). The response of a field that is one widget.
fn scalar(ui: &mut Ui, v: &mut Value) -> Option<egui::Response> {
    match v {
        Value::String(text) => return fields::text(ui, text, ui.available_width().max(0.0)),
        Value::Bool(b) => return Some(ui.checkbox(b, "On")),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Some(fields::number(ui, v, i, |d| d));
            } else if let Some(f) = n.as_f64() {
                return Some(fields::number(ui, v, f, |d| d.speed(0.1)));
            }
        }
        // A colour as the rest of Moonglow shows one: a swatch that opens a picker.
        Value::Object(o)
            if o.len() == 4
                && ["r", "g", "b", "a"].iter().all(|k| o.get(*k).is_some_and(Value::is_number)) =>
        {
            fields::color(ui, o);
        }
        // Lined up in a grid, so the values start at one edge.
        Value::Object(o) if o.len() <= 4 && o.values().all(Value::is_number) => {
            egui::Grid::new(ui.id().with("numbers")).num_columns(2).show(ui, |ui| {
                for (k, v) in o {
                    widgets::field_label(
                        ui,
                        match k.as_str() {
                            "w" => "Width",
                            "h" => "Height",
                            "x" => "X",
                            "y" => "Y",
                            "r" => "Red",
                            "g" => "Green",
                            "b" => "Blue",
                            "a" => "Alpha",
                            _ => k,
                        },
                    );
                    scalar(ui, v);
                    ui.end_row();
                }
            });
        }
        Value::Null | Value::Array(_) | Value::Object(_) => fields::value(ui, v, 0),
    }
    None
}

fn resolved<'a>(v: &'a Value, s: &'a Settings) -> &'a Value {
    v.get("bind").and_then(Value::as_str).and_then(|b| s.bindings.get(b)).map_or(v, |b| &b.value)
}
#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    #[test]
    fn option_labels_are_static_but_selection_and_combo_entries_are_bindable() {
        for ty in ["options", "tabbar"] {
            assert!(!super::bindable(ty, "elements"));
            assert!(super::bindable(ty, "value"));
            assert!(super::bindable(ty, "enabled"));
        }
        assert!(super::bindable("combo", "elements"));
    }

    #[test]
    fn a_duplicate_keeps_extensions_and_gets_an_id_of_its_own() {
        let mut v = mg_nui::window();
        v["root"]["children"][0]["id"] = "first".into();
        v["root"]["children"][0]["extension"] = json!({"untouched":[1,2,3]});
        let mut selected = "/root/children/0".to_string();
        rearrange(&mut v, &mut selected, Op::Duplicate);
        assert_eq!(selected, "/root/children/1");
        assert_eq!(v["root"]["children"][1]["extension"], json!({"untouched":[1,2,3]}));
        assert_ne!(v["root"]["children"][0]["id"], v["root"]["children"][1]["id"]);
        rearrange(&mut v, &mut selected, Op::MoveDown);
        assert_eq!(selected, "/root/children/2");
        rearrange(&mut v, &mut selected, Op::Delete);
        assert_eq!(v["root"]["children"].as_array().unwrap().len(), 2);
        v["root"] = mg_nui::template("list");
        selected = "/root/row_template/0/0".into();
        v["root"]["row_template"] = json!([[mg_nui::template("label"), 150.0, true]]);
        rearrange(&mut v, &mut selected, Op::Duplicate);
        assert_eq!(v["root"]["row_template"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_new_control_goes_into_the_selected_layout_or_its_parent() {
        let mut v = mg_nui::window();
        let mut selected = "/root/children/0".to_owned();
        design::insert(&mut v, &mut selected, "check");
        assert_eq!(selected, "/root/children/2");
        assert_eq!(v.pointer(&selected).unwrap()["type"], "check");
        let first_id = v.pointer(&selected).unwrap()["id"].clone();
        design::insert(&mut v, &mut selected, "check");
        assert_ne!(v.pointer(&selected).unwrap()["id"], first_id);
        v["root"] = mg_nui::template("list");
        selected = "/root".into();
        design::insert(&mut v, &mut selected, "button");
        assert_eq!(selected, "/root/row_template/0/0");
        assert_eq!(v.pointer(&selected).unwrap()["type"], "button");
    }

    #[test]
    fn a_problem_s_path_reads_as_its_control_and_property() {
        let doc = json!({"root":{"type":"col","children":[
            {"type":"label","value":"Header"},
            {"type":"textedit","label":"Name","max":0}]}});
        assert_eq!(
            describe_path(&doc, "/root/children/1/max"),
            (Some("/root/children/1".into()), "Text input · Name › Max".into())
        );
        assert_eq!(describe_path(&doc, "/actions"), (None, "Events".into()));
    }

    /// Named NUI_* choices instead of bare numbers.
    #[test]
    fn a_numbered_choice_reads_by_its_name() {
        let mut image = mg_nui::template("image");
        image["image_aspect"] = json!(5);
        let mut h =
            Harness::builder().build_ui(|ui| properties(ui, &mut image, &mut Settings::default()));
        h.run();
        h.get_by_label("Appearance").click();
        h.run();
        assert!(h.query_by_label("Stretch").is_some() || h.query_by_value("Stretch").is_some());
    }

    /// A window's optional settings wait in Add property until wanted.
    #[test]
    fn a_window_s_unset_options_wait_in_add_property() {
        let mut window = mg_nui::window();
        window["size_constraint"] = Value::Null;
        window["edge_constraint"] = Value::Null;
        let mut h =
            Harness::builder().build_ui(|ui| properties(ui, &mut window, &mut Settings::default()));
        h.run();
        assert!(h.query_by_label("Window size limits").is_none());
        h.get_by_label("+ Add property").click();
        h.run();
        h.get_by_label("Distance from screen edges").click();
        h.run();
        drop(h);
        assert!(window["edge_constraint"].is_object());
    }

    /// Draw layers: what one draws comes first, by its name.
    #[test]
    fn a_draw_layer_s_geometry_comes_before_its_color() {
        let mut spacer = mg_nui::template("spacer");
        spacer["draw_list"] = json!([{"type":1,"enabled":true,"color":{"r":1,"g":2,"b":3,"a":255},
            "fill":false,"line_thickness":2.0,"order":1,"render":0,"arrayBinds":false,
            "a":{"x":0.0,"y":0.0},"b":{"x":10.0,"y":10.0},"ctrl0":{"x":0.0,"y":5.0},"ctrl1":{"x":5.0,"y":10.0}}]);
        let assets = skin::Assets::default();
        let mut h = Harness::builder()
            .build_ui(|ui| draw::editor(ui, &mut spacer, &mut Settings::default(), &assets));
        h.run();
        h.get_by_label("Draw layers").click();
        h.run();
        if let Some(header) = h.query_by_label_contains("1 · Curve") {
            header.click();
            h.run();
        }
        let start = h.get_by_label("Start").rect();
        let color = h.get_by_label("Color").rect();
        assert!(start.top() < color.top(), "geometry before colour");
        assert!(h.get_by_label("Control point 1").rect().top() < color.top());
        assert!(h.query_by_label("Per-row values (in a list)").is_some());
    }

    #[test]
    fn the_title_bar_hides_and_comes_back_with_its_title() {
        let mut window = mg_nui::window();
        window["title"] = json!("Character");
        let mut s = Settings::default();
        let mut h = Harness::builder().build_ui_state(
            |ui, (w, s): &mut (Value, Settings)| properties(ui, w, s),
            (window, s.clone()),
        );
        h.run();
        h.get_by_label("Title bar").click();
        h.run();
        let (hidden, kept) = h.state().clone();
        // What NWN EE 8193.37 needs to draw no title bar.
        assert_eq!(hidden["title"], json!(false));
        assert_eq!(hidden["closable"], json!(false));
        assert_eq!(hidden["collapsed"], json!(false));
        assert!(
            mg_nui::validate(&hidden, &kept).iter().all(|d| d.severity != Severity::Error),
            "{:?}",
            mg_nui::validate(&hidden, &kept)
        );
        h.get_by_label("Title bar").click();
        h.run();
        let (shown, _) = h.state().clone();
        assert_eq!(shown["title"], json!("Character"));
        assert_eq!(shown["closable"], mg_nui::window()["closable"]);
        s.extra.clear();
    }

    #[test]
    fn an_unset_property_is_set_to_its_default_not_to_a_json_type() {
        for ty in mg_nui::ELEMENTS {
            let mut node = mg_nui::template(ty);
            let mut h = Harness::builder().build_ui(|ui| {
                properties(ui, &mut node, &mut Settings::default());
            });
            h.run();
            assert!(h.query_by_label("Advanced properties").is_none(), "standard template: {ty}");
            if let Some(header) = h.query_by_label("Advanced properties") {
                header.click();
            }
            h.run();
            assert!(h.query_by_label("Set value…").is_none(), "{ty}");
        }
        let mut node = mg_nui::template("button");
        node["label"] = Value::Null;
        let mut h = Harness::builder().build_ui(|ui| {
            properties(ui, &mut node, &mut Settings::default());
        });
        h.run();
        assert!(h.query_by_label("Advanced properties").is_none());
        h.get_by_label("Set value…").click();
        h.run();
        assert!(h.query_by_label("Array").is_none());
        drop(h);
        assert_eq!(node["label"], "Button");
        assert!(node["value"].is_null());
    }

    #[test]
    fn standard_properties_stay_out_of_advanced_and_extensions_are_kept() {
        for mut node in [mg_nui::window(), mg_nui::template("row"), mg_nui::template("image")] {
            let window = node.get("type").is_none();
            if !window {
                node["aspect"] = json!(1.5);
                node["encouraged"] = json!(true);
            }
            if node["type"] == "image" {
                node["image_region"] = json!({"x":0,"y":0,"w":32,"h":32});
                node["draw_list"] = json!([]);
                node["draw_list_scissor"] = json!(false);
            }
            let before = node.clone();
            let mut h = Harness::builder()
                .build_ui(|ui| properties(ui, &mut node, &mut Settings::default()));
            h.run();
            assert!(h.query_by_label("Advanced properties").is_none());
            assert!(h.query_by_label("Size & layout").is_some());
            if before["type"] == "row" {
                assert_eq!(h.query_all_by_label("Set size").count(), 2);
            }
            drop(h);
            assert_eq!(node, before, "opening properties must not mutate the document");
            node["vendor_extension"] = json!({"payload":[1,2,3]});
            let before = node.clone();
            let mut h = Harness::builder()
                .build_ui(|ui| properties(ui, &mut node, &mut Settings::default()));
            h.run();
            assert!(h.query_by_label("Advanced properties").is_some());
            drop(h);
            assert_eq!(node, before);
        }
    }

    #[test]
    fn a_talk_table_string_is_set_then_bound() {
        let mut h = Harness::builder().build_ui_state(
            |ui, state: &mut (Value, Settings)| {
                property(ui, "value", "label", &mut state.0, &mut state.1, &mut None);
            },
            (json!("Text"), Settings::default()),
        );
        h.run();
        h.get_by_label("…").click();
        h.run();
        h.get_by_label("Use TLK string (StrRef)").click();
        h.run();
        assert_eq!(h.state().0, json!({"strref":0}));
        h.get_by_label("…").click();
        h.run();
        h.get_by_label("Make dynamic (bind)").click();
        h.run();
        assert_eq!(h.state().0, json!({"bind":"value_1"}));
        assert_eq!(h.state().1.bindings["value_1"].value, json!({"strref":0}));
    }

    #[test]
    fn an_existing_bind_is_reused_with_its_formatting_kept() {
        for initial in [json!("Label"), json!({"bind":"old","number_precision":2,"text_flags":1})] {
            let mut settings = Settings::default();
            settings
                .bindings
                .insert("shared".into(), Binding { value: json!(-12.345), ..Default::default() });
            settings
                .bindings
                .insert("old".into(), Binding { value: json!("Unchanged"), ..Default::default() });
            let original = settings.clone();
            let mut h = Harness::builder().build_ui_state(
                |ui, state: &mut (Value, Settings)| {
                    property(ui, "value", "label", &mut state.0, &mut state.1, &mut None);
                },
                (initial.clone(), settings),
            );
            h.run();
            h.get_by_label("…").click();
            h.run();
            h.query_by_label_contains("Use existing bind")
                .expect("Missing existing-bind selector")
                .click();
            h.run();
            h.get_by_label("shared").click();
            h.run();
            assert_eq!(h.state().0["bind"], "shared");
            if initial.is_object() {
                assert_eq!(h.state().0["number_precision"], 2);
                assert_eq!(h.state().0["text_flags"], 1);
            }
            assert_eq!(h.state().1, original, "Reusing a bind must not create or change bindings");
            let mut doc = mg_nui::window();
            doc["root"]["children"][0]["value"] = h.state().0.clone();
            assert!(
                !mg_nui::validate(&doc, &h.state().1).iter().any(|d| d.severity == Severity::Error)
            );
        }
        let mut settings = Settings::default();
        settings
            .bindings
            .insert("shared".into(), Binding { value: json!(10), ..Default::default() });
        let mut h = Harness::builder().build_ui_state(
            |ui, state: &mut (Value, Settings)| {
                property(ui, "max", "textedit", &mut state.0, &mut state.1, &mut None);
            },
            (json!(128), settings),
        );
        h.run();
        h.get_by_label("…").click();
        h.run();
        assert!(
            h.query_by_label_contains("Use existing bind").is_none(),
            "TextEdit max must remain static"
        );
    }

    #[test]
    fn a_pasted_list_cell_keeps_its_width_and_brings_its_bind() {
        let ctx = egui::Context::default();
        let mut source = mg_nui::window();
        source["root"] = mg_nui::template("list");
        source["root"]["row_template"] = json!([[{"type":"label","value":{"bind":"caption"},"custom":{"unchanged":true}}, 230.5, false]]);
        let mut settings = Settings::default();
        settings.bindings.insert(
            "caption".into(),
            Binding { value: "Source".into(), watch: true, ..Default::default() },
        );
        let mut selected = "/root/row_template/0/0".into();
        shortcuts::clipboard(&ctx, &mut source, &mut settings, &mut selected, 0);
        assert!(ctx.output(|o| o.commands.iter().any(|c| matches!(
            c,
            egui::OutputCommand::CopyText(text) if text == "Moonglow Toolset: NUI element"
        ))));
        let mut target = source.clone();
        let mut target_settings = Settings::default();
        target_settings
            .bindings
            .insert("caption".into(), Binding { value: "Target".into(), ..Default::default() });
        selected = "/root".into();
        shortcuts::clipboard(&ctx, &mut target, &mut target_settings, &mut selected, 2);
        let cell = &target["root"]["row_template"][1];
        assert_eq!(cell[1], 230.5);
        assert_eq!(cell[2], false);
        assert_eq!(cell[0]["custom"], json!({"unchanged":true}));
        assert_eq!(cell[0]["value"]["bind"], "caption_2");
        assert_eq!(target_settings.bindings["caption"].value, "Target");
        assert_eq!(target_settings.bindings["caption_2"], settings.bindings["caption"]);
        assert!(!shortcuts::can_edit(&target, "/root", Op::Delete));
        target["root"] = mg_nui::template("group");
        assert!(!shortcuts::can_edit(&target, "/root/children/0", Op::Delete));
    }

    /// A list cell or root that isn't what the editors expect is left alone.
    #[test]
    fn odd_cells_and_roots_are_left_alone() {
        let s = Settings::default();
        for cell in [json!([1]), json!("x"), json!(null), json!([{"type":"label"}])] {
            let mut doc = json!({"root":{"type":"list","row_template":[cell.clone()]}});
            let before = doc.clone();
            egui::__run_test_ui(|ui| {
                fields::list(ui, &mut doc, "/root/row_template/0/0", &mut s.clone());
            });
            assert_eq!(doc, before);
        }
        for root in [json!(5), json!("x"), json!([1])] {
            let mut doc = json!({"root": root});
            let before = doc.clone();
            images::background(&mut doc, &s, "my_bg");
            assert_eq!(doc, before);
        }
        let mut doc = json!(5);
        images::background(&mut doc, &s, "my_bg");
        let mut node = json!("label");
        egui::__run_test_ui(|ui| {
            draw::editor(ui, &mut node, &mut s.clone(), &skin::Assets::default());
        });
    }
}
