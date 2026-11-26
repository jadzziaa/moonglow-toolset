//! The NUI Creator: the editor of a NUI window (a JUI of the module, with
//! the TXT of its name that holds the Creator's settings for it).
//!
//! This file holds the tab itself ([`ui`]): what it keeps between frames
//! ([`State`]), its one working copy of the window shared with its docked
//! panels ([`Work`], [`begin`], [`end`]: M4's U18), the header (the pages'
//! switch, Build & Compile, Export…, More, Interact, Reset, View, OK and
//! Cancel), the row under it (Event Script…, Advanced, the build's state,
//! the checks' count), the list of what the checks found, the pages
//! (Design, Bindings, Events & Variants, Images, the two source pages) and
//! the properties of the control selected. Its Components, Properties,
//! Bindings and Layers are the docks' (`panels`). Beside it are New NUI Window ([`windows`], [`create`]),
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
use mg_uikit::Controls as _;
use serde_json::{Value, json};

mod bindings;
pub(crate) mod checks;
pub(crate) mod design;
mod draw;
mod events;
mod fields;
mod form;
pub(crate) mod headless;
mod images;
mod interaction;
mod layouts;
pub(crate) mod panels;
mod preview;
mod shortcuts;
pub(crate) mod skin;
pub(crate) mod source;
mod structure;
mod workflow;

/// The design group's share of it and the source group's beside it, where
/// the source opens (the NUI board's 640 of 1018: M4's U19).
pub(crate) const SOURCE_SHARE: f32 = 640.0 / 1018.0;
/// How much wider a floating designer's window grows for its source group
/// (the board's source group, 378, and room for its header).
pub(crate) const SOURCE_WIDTH: f32 = 420.0;

/// The source group's text laid out: what it was laid out for (its draft's
/// count of changes, egui's fonts made, the points per pixel), its colors, and
/// the galley.
type SourceGalley = ((u64, u64, f32), crate::script_view::Palette, std::sync::Arc<egui::Galley>);

/// What a NUI Creator tab keeps between frames.
#[derive(Default)]
pub(crate) struct State {
    /// The control selected, as a JSON pointer into the window (or into the
    /// variant edited in its place); empty for none, `/root` for the root.
    selected: String,
    /// A control a link asked to select (a problem's, a bind's use), on the
    /// Design page: selected as the next frame starts, so that a field
    /// being typed in when the link was clicked (in the same frame as it
    /// was pressed) is drawn once more and sets what was typed in it.
    select_next: Option<String>,
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
    /// The states controls are drawn in off the screen (`mg nui render
    /// --state`), counted as `preview_state` is, by ID (`None`: all but
    /// those given by ID).
    still_states: std::collections::BTreeMap<Option<String>, usize>,
    /// The states of controls of a list's rows off the screen, one row's
    /// (`--state STATE:ID@ROW`), by ID and row: over `still_states`.
    still_row_states: std::collections::BTreeMap<(String, usize), usize>,
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
    /// The errors there were last frame, to open the Checks panel when
    /// more come (M4's U19: only a failure opens the Bottom Dock).
    issues_seen: usize,
    /// The Checks panel's rows, for the findings they were made of.
    problem_rows: Option<ProblemRows>,
    /// The page shown.
    page: Page,
    /// The hak Images adds pictures to: one the module uses, or None for a new one.
    images_hak: Option<String>,
    /// How the Design page shows the window.
    view_mode: ViewMode,
    /// What the checks found, for the bytes they were run on.
    checks: Option<Checks>,
    /// The source field being typed in (the source group's: M4's U19).
    source_draft: Option<SourceDraft>,
    /// What the source group shows: the window's text (JUI Source) or its
    /// settings' (Binding JSON), its switch.
    source_of: SourceOf,
    /// The source group's text laid out a line at a time, kept between
    /// frames (`script_layout::LaidOut`).
    source_laid: crate::script_layout::LaidOut,
    /// The pass the source group was last drawn in.
    source_drawn: Option<u64>,
    /// The control the source group last showed chosen, and the cursor it
    /// last saw (char index): a change of one is followed by the other.
    source_shown: Option<String>,
    source_cursor: Option<usize>,
    /// The source's cursor's line, marked under the text (first, last: one
    /// line, the board's).
    source_lines: Option<(usize, usize)>,
    /// The control the source's cursor last chose in the design: the cursor
    /// moved by the keys goes on choosing only while the design shows it.
    source_chose: Option<String>,
    /// The source's text laid out, for how many times its draft changed.
    source_galley: Option<SourceGalley>,
    /// The variant the source group last saw edited: a variant chosen moves
    /// its switch to Binding JSON, where the variant is written, once.
    source_variant: Option<String>,
    /// A page asked for the source group on this switch position (the
    /// repair's Open JUI Source and Open Binding JSON).
    open_source: Option<SourceOf>,
    /// The window and its settings as this designer's parts edit them.
    work: Option<Work>,
    /// The pass this designer's frame began in ([`begin`]), and whether it
    /// was ended ([`end`]).
    began: Option<u64>,
    ended: bool,
    /// While a variant is edited in the window's place this frame: its name
    /// and the window's own root, put back as the frame ends.
    swapped: Option<(String, Value)>,
    /// What Layers and Components asked for this frame ([`Picked`]).
    picked: Picked,
    /// A source page's text typed this frame: the window's, the settings'.
    typed_raw: Option<String>,
    typed_config: Option<String>,
    /// What the pages added to the module this frame (a picture into a hak).
    page_edits: Vec<Edit>,
    /// What the fields showed last frame and were put back
    /// ([`fields::recover`]).
    reverted: Vec<fields::Reverted>,
    /// How to Open It… is shown.
    how_to_open: bool,
    /// The bind chosen in the Bindings panel's table (M4's U19), and whether
    /// the Bindings page is to bring its form into sight.
    bind_chosen: Option<String>,
    bind_reveal: bool,
    /// Where the window names each bind, walked once for its bytes.
    binds_seen: Option<design::BindsSeen>,
    /// How many times it walked the window for them (the tests').
    binds_walks: usize,
    /// Layers' rows as last listed ([`panels::LayerRows`]).
    layer_rows: Option<panels::LayerRows>,
    /// How many copies of the window [`check`] made to edit ([`Work`]).
    copies: usize,
    /// The window's generation: one more for each frame in which it may
    /// have changed before the canvas drew it. What is worked out of the
    /// window is kept under it from one frame to the next (the canvas's
    /// layout, `measured`; the names of its pictures and strings, `walked`;
    /// Interact's check that its session is of this window), so that a
    /// frame that changes nothing walks no more than is in sight.
    window_gen: u64,
    /// Which window [`end`] last found the window unchanged in, on a frame
    /// with no input that could change it: the copy edited and the variant
    /// in its place. A frame begun without it, or with another, is a new
    /// generation ([`settle`]).
    settled: Option<(usize, Option<String>)>,
    /// The pass [`settle`] last found the window unchanged in: only then is
    /// what was kept under its generation taken ([`trusted`]).
    proven_in: Option<u64>,
    measured: Option<((u64, u32, u32), preview::Measured)>,
    /// How many passes drew with the layout kept from the one before (the
    /// tests').
    carried: usize,
    /// The pictures and strings the window names, and the generation and
    /// Interact session (where its window is what is drawn) they are of.
    walked: Option<Walked>,
    /// Counts the typings ended by the window changing under them: part of
    /// Properties' fields' ids ([`begin`]).
    typing: u64,
    /// Its opener's and event script's keys, source and compiled.
    script_keys: Option<[Option<ResKey>; 4]>,
}

/// The control chosen in a designer (`Moonglow::nui_chosen`).
pub(crate) fn chosen(state: &State) -> String {
    state.selected.clone()
}

/// The control at `path` selected in a designer as a link selects one, as
/// the next frame starts (`Moonglow::nui_choose`).
pub(crate) fn choose(state: &mut State, path: &str) {
    state.select_next = Some(path.to_owned());
}

/// Whether a designer's canvas is in Interact (`Moonglow::nui_interacts`).
pub(crate) fn interacts(state: &State) -> bool {
    state.preview_interactive
}

/// The controls chosen together in a designer (`Moonglow::nui_chosen_many`).
pub(crate) fn chosen_many(state: &State) -> Vec<String> {
    state.selected_many.iter().cloned().collect()
}

/// Bind `name` chosen in a designer's Bindings table (`Moonglow::nui_choose_bind`).
pub(crate) fn choose_bind(state: &mut State, name: &str) {
    state.bind_chosen = Some(name.to_owned());
    if let Some(work) = &mut state.work {
        work.verified_clean = false;
    }
}

/// How many times a designer's Bindings table walked its window
/// (`Moonglow::nui_binds_walks`).
pub(crate) fn binds_walks(state: &State) -> usize {
    state.binds_walks
}

/// A designer's window's generation, and how many passes of its canvas
/// drew with the layout kept from the pass before (`Moonglow::nui_kept`).
pub(crate) fn kept(state: &State) -> (u64, usize) {
    (state.window_gen, state.carried)
}

/// How many copies of its window a designer made (`Moonglow::nui_copies`).
pub(crate) fn copies(state: &State) -> usize {
    state.copies
}

/// Designer `key` edits its variant `name` in its slot, as Layers' row
/// does (`Moonglow::nui_edit_variant`, for the timing).
pub(crate) fn edit_variant(app: &mut Moonglow, key: ResKey, name: &str) {
    let state = app.nui_views.entry(key).or_default();
    if let Some(work) = &mut state.work {
        work.verified_clean = false;
    }
    layouts::edit(state, Some(name.into()));
}

/// Where a designer's working copy keeps its words: the heap addresses of
/// the first key of the window, of its root, and of each variant and its
/// root, in the variants' order. The same from frame to frame while
/// nothing copies the window, its root or a variant, the one edited in
/// the root's place too (`Moonglow::nui_identity`, for the tests).
pub(crate) fn identity(state: &State) -> Option<Vec<usize>> {
    fn first(v: &Value) -> Option<usize> {
        Some(v.as_object()?.keys().next()?.as_ptr() as usize)
    }
    let work = state.work.as_ref()?;
    let mut at = vec![first(&work.window)?, first(&work.window["root"])?];
    for variant in work.settings.variants.values() {
        at.extend(first(variant));
        at.extend(variant.get("root").and_then(first));
    }
    Some(at)
}

/// A page of the Creator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Page {
    #[default]
    Design,
    Bindings,
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
    /// The canvas beside the source: the source group open (M4's U19).
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
    of: [Held; 6],
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
    /// Interact turned on or off (the header's toggle, the canvas's row's
    /// before M4's U18): a new simulation, from the start.
    fn interact(&mut self, on: bool) {
        if on != self.preview_interactive {
            self.preview_interactive = on;
            self.runtime = None;
            self.resize = None;
            self.forget_scrolls();
        }
    }

    /// Reset (the header's): the simulation's controls back to their
    /// initial values, in Interact.
    fn reset(&mut self) {
        if self.preview_interactive {
            self.runtime = None;
            self.forget_scrolls();
        }
    }

    fn guides(&self) -> bool {
        !self.preview_clean
    }

    /// Every list, group and text block of the canvas back at its start
    /// (a new simulation, Interact turned on or off, Reset). The five
    /// were cleared by hand in three places.
    fn forget_scrolls(&mut self) {
        for kept in [
            &mut self.list_scroll,
            &mut self.group_scroll_x,
            &mut self.group_scroll_y,
            &mut self.text_scroll_x,
            &mut self.text_scroll_y,
        ] {
            kept.clear();
        }
    }
}

/// The window that asks for a new NUI window's name (Tools › New NUI
/// Window…, the module tree's New). The name leaves room for the two
/// scripts' suffixes, which the name window does not know of: one that is
/// refused is said in the log, and the window stays.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    use crate::widgets::{Already, NameWindow, Named};
    // Import…'s question: windows whose resources the module has.
    if let Some(windows) = &app.nui_import {
        let taken: Vec<(String, Option<String>)> = (windows.iter())
            .flat_map(|w| w.taken.iter().map(|k| (k.to_string(), Some(w.from.clone()))))
            .collect();
        let answer = crate::widgets::files_already(
            ui.ctx(),
            &mut app.settings.files_there,
            "Module",
            "The module has these already, otherwise than the files imported:",
            &taken,
            [
                "Imports every window, replacing these",
                "Imports only the windows the module doesn't have",
            ],
        );
        if let Some(answer) = answer {
            let windows = app.nui_import.take().unwrap_or_default();
            match answer {
                Already::Replace => bring_in(app, &windows),
                Already::Skip => {
                    let new: Vec<_> = windows.into_iter().filter(|w| w.taken.is_empty()).collect();
                    bring_in(app, &new);
                }
                Already::Cancel => {}
            }
        }
    }
    let Some(mut name) = app.new_nui.clone() else { return };
    let window = NameWindow {
        new: true,
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

/// A nasher project whose include takes in scripts and JSON only would
/// neither pack a window's files nor read them back: said, with the line
/// that takes them in.
fn say_left_out(app: &mut Moonglow, what: &str, files: &[ResKey]) {
    let Some(p) = app.ws.as_ref().and_then(|ws| ws.module.project.as_deref()) else { return };
    let left = p.uncovered(files);
    if let Some(line) = p.include_for(&left) {
        let names: Vec<String> = files.iter().map(ToString::to_string).collect();
        let said = format!(
            "{what}: nasher.cfg's include ({}) leaves out the window's files ({}): nasher won't \
             pack them, and the project won't read them back. Add the line {line} to the \
             target's sources, or save and use Build › Add Left-Out Files to nasher.cfg.",
            p.target().includes.join(", "),
            names.join(", ")
        );
        app.log.warn(said);
    }
}

pub(crate) fn create(app: &mut Moonglow, name: &str) {
    if app.ws.is_none() {
        return;
    }
    let files: Vec<ResKey> =
        [ResType::JUI, ResType::TXT].iter().filter_map(|ty| ResKey::parse(name, *ty)).collect();
    say_left_out(app, name, &files);
    let Some(ws) = &app.ws else { return };
    let cmd = match mg_nui::create(&ws.module, name) {
        Ok(cmd) => cmd,
        Err(e) => return app.log.error(e.to_string()),
    };
    // (A window made: the plugins' resource-created handlers see it.)
    let made = crate::plugin_events::Made::new_in(ws, &cmd, "new-nui");
    let cmd = crate::plugin_events::with_plugins(app, cmd, &made);
    match app.apply(cmd) {
        Ok(()) => app
            .actions
            .extend(ResKey::parse(name, ResType::JUI).map(|key| Action::OpenTab(Tab::Nui(key)))),
        Err(e) => app.log.error(e.to_string()),
    }
}

/// Import… (the Creator's): NUI windows from the files chosen (`.jui`
/// files with their settings and event script beside them, and ERF and
/// zip archives of them), as `mg nui import` brings them. Windows whose
/// resources the module has otherwise are asked about first.
pub(crate) fn import(app: &mut Moonglow) {
    if app.ws.is_none() {
        return;
    }
    let files = app.dialogs.open_files(crate::FileKind::NuiWindows, None);
    if files.is_empty() {
        return;
    }
    let mut found = mg_nui::import::Found::new();
    for file in &files {
        if let Err(e) = mg_nui::import::gather(file, &mut found) {
            app.log.error(format!("Import NUI windows: {e}"));
            return;
        }
    }
    let Some(ws) = &app.ws else { return };
    match mg_nui::import::incoming(&ws.module, &found) {
        Ok(windows) if windows.iter().any(|w| !w.taken.is_empty()) => {
            app.nui_import = Some(windows);
        }
        Ok(windows) => bring_in(app, &windows),
        Err(e) => app.log.error(format!(
            "Import NUI windows: {}: {e}",
            files.iter().map(|f| f.display().to_string()).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// The windows imported, as one Undo step, the first opened in the
/// Creator; what is to be said of them, said.
fn bring_in(app: &mut Moonglow, windows: &[mg_nui::import::Incoming]) {
    if windows.is_empty() {
        return;
    }
    for w in windows {
        for note in &w.notes {
            app.log.warn(note.clone());
        }
        if w.errors > 0 {
            app.log.warn(format!(
                "{}.jui has {}: open it to see them",
                w.name,
                count(w.errors, "error")
            ));
        }
        say_left_out(app, &w.name, &w.resources);
    }
    let made = mg_nui::import::command(windows);
    // (Imported: the plugins' resource-created handlers see what is new.)
    let new = app.ws.as_ref().map(|ws| crate::plugin_events::Made::new_in(ws, &made, "import"));
    let made = match new {
        Some(new) => crate::plugin_events::with_plugins(app, made, &new),
        None => made,
    };
    if !made.edits.is_empty()
        && let Err(e) = app.apply(made)
    {
        app.log.error(e.to_string());
        return;
    }
    let names: Vec<&str> = windows.iter().map(|w| w.name.as_str()).collect();
    app.log.info(format!(
        "Imported {}: {}. Build writes each one's scripts.",
        count(windows.len(), "NUI window"),
        names.join(", ")
    ));
    app.actions.extend(
        ResKey::parse(&windows[0].name, ResType::JUI).map(|key| Action::OpenTab(Tab::Nui(key))),
    );
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

/// What a Build of the window `name` made beside its command, once that is
/// applied: the code of the builder's it changed said, what is remembered
/// of the scripts compiled kept (in `kept`, the compile memory's file, as
/// Compile All keeps it), the module's scripts that did not compile said.
pub(crate) fn after_build(
    app: &mut Moonglow,
    name: &str,
    failed: &[mg_module::build::ScriptResult],
    remembered: mg_module::build::Remembered,
    refits: &[mg_nui::Refit],
    kept: Option<&std::path::Path>,
) {
    for r in refits {
        app.log.info(format!(
            "{name}_e.nss line {}: in the section \"{}\", {} is now {}, to fit the event \
             script's function",
            r.line, r.section, r.was, r.now
        ));
    }
    app.script_stamps = remembered;
    if let Some(file) = kept
        && let Err(e) = app.script_stamps.write(file)
    {
        crate::trace::note(format!("compiled scripts not noted in {file:?}: {e}"));
    }
    for e in failed.iter().filter_map(|f| f.result.as_ref().err()) {
        app.log.error(e.message.clone());
    }
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
        built = Some((made.failed, made.remembered, made.refits));
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
                if let Some((failed, remembered, refits)) = built {
                    after_build(app, name, &failed, remembered, &refits, kept.as_deref());
                }
                // (Into the module, as any edit: a nasher project's files
                // are written when it is saved. "Written" was read as on
                // disk, a builder's report.)
                let saved = match app.ws.as_ref().is_some_and(|w| w.module.project.is_some()) {
                    true => "File › Save writes them into the nasher project's sources",
                    false => "File › Save writes them into the module",
                };
                let function = (app.ws.as_ref())
                    .and_then(|ws| ws.module.get(&ResKey::parse(name, ResType::TXT)?))
                    .and_then(|b| Settings::parse(b).ok())
                    .is_some_and(|s| s.event_function);
                app.log.info(if function {
                    format!(
                        "Built {name}: {name}_o.nss and {name}_e.nss made ({saved}). To open the \
                         window, include {name}_o in a script of yours and call \
                         Open_{name}(oPlayer); its events go to the module's OnNuiEvent script: \
                         include {name}_e there and call {}().",
                        mg_nui::event_function(name)
                    )
                } else {
                    format!(
                        "Built {name}: {name}_o.nss made, {name}_e.nss made and compiled \
                         ({saved}). To open the window, include {name}_o in a script of yours \
                         and call Open_{name}(oPlayer)."
                    )
                });
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
                    let mut left = Vec::new();
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
                            if !resources
                                .iter()
                                .any(|k| k.resref.to_string().eq_ignore_ascii_case(&image))
                            {
                                left.push(image);
                            }
                        }
                    }
                    let resman = app.game.as_deref().map(|g| &g.resman);
                    if let Some(said) = images::pictures_left_behind(resman, &left) {
                        app.log.warn(said);
                    }
                    app.actions.push(Action::ExportDialog(resources));
                }
            }
            Err(e) => app.log.error(e.to_string()),
        },
        Err(e) => app.log.error(e.to_string()),
    }
}

/// The NUI designer's tab (M4's U18, the NUI board): its header (the pages'
/// switch, Build & Compile, Export…, More, Interact, Reset, View, OK and
/// Cancel; `header`) over the page shown. Its Components, Properties and
/// Bindings are the Right Dock's (its window's right pane where it floats)
/// and its Layers the Outliner's (`panels`), drawn before it: what they
/// changed this frame is set here ([`end`]), with what the editor changed,
/// after its Cancel starts noting, so that Cancel sets it back too. Cancel
/// sets the window and its settings back to what they were when the tab was
/// opened (what Build made of them is built again from those).
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    if app.ws.is_none() {
        editor(app, ui, key);
        return;
    }
    let targets = [(key, mg_edit::GffPath::root())];
    let pending = app.actions.len();
    let (pressed, asked) = editor(app, ui, key);
    // (Its window's Layers unfolded from its strip, over the canvas: drawn
    // after the editor, as a conversation's are, and before the frame ends:
    // what they choose is done with the rest.)
    crate::outliner::over(app, ui, crate::outliner::Owner::Nui(key));
    if let Some(mut state) = app.nui_views.remove(&key) {
        end(app, ui.ctx(), key, &mut state, asked);
        app.nui_views.insert(key, state);
    }
    // The event script is not Cancel's to set back: code written in its
    // sections since the tab was opened (in the script editor, saved) is
    // the builder's, and went with the rest. It is rebuilt around the
    // handlers Cancel brings back, its sections' code kept as Build keeps
    // it (a handler's that is gone again: commented out under it).
    // (`cancel_more`.)
    let kept = events_script(&key.resref.to_string());
    let tab = Tab::Nui(key);
    crate::blueprint::window_drawn_but(app, pending, &tab, &targets, pressed, &kept);
}

/// What the window's Cancel adds to what it sets back (`set`): its event
/// script rebuilt around the handlers brought back (`ui`).
pub(crate) fn cancel_more(app: &Moonglow, key: ResKey, set: &[Edit]) -> Vec<Edit> {
    let settings_key = ResKey::new(key.resref, ResType::TXT);
    let settings = set.iter().find_map(|e| match e {
        Edit::SetResource { key, data } if *key == settings_key => data.as_deref(),
        _ => None,
    });
    events::after_cancel(app, &key.resref.to_string(), settings).into_iter().collect()
}

/// A window's event script, as the resources Cancel leaves alone.
fn events_script(name: &str) -> Vec<ResKey> {
    ResKey::parse(&format!("{name}_e"), ResType::NSS).into_iter().collect()
}

/// The window and its settings as this designer's parts edit them: its
/// panels, its Layers, its pages and its canvas, in place, one copy for all
/// of them. Kept between frames while the module holds the same bytes
/// (`of`), made again from what [`check`] read where it holds others (an
/// edit set, Undo): no copy of the window is made in a frame that changes
/// nothing.
struct Work {
    of: [Held; 2],
    window: Value,
    settings: Settings,
    /// True after a full comparison confirmed `window` and `settings` match
    /// `state.source`, with no active widget state that could mutate them on a
    /// subsequent quiet frame without events.
    verified_clean: bool,
}

/// What Layers, Components, Properties' swap layout buttons and the
/// variants' bar asked for in a frame (they are drawn before the editor's
/// canvas and source, or in the editor before them): done as the frame ends
/// ([`end`]), so that what the editor draws this frame is the window, and
/// the variant in its place, the frame began with. (It is not what keeps
/// typed text to its control: egui ends a field's focus on the press, the
/// row or button acts on the release.)
#[derive(Default)]
struct Picked {
    /// Layers' selection as it left it: the controls chosen, the variant
    /// edited, the draw layer chosen.
    chosen: Option<Chosen>,
    /// A component clicked: added to the control chosen.
    insert: Option<&'static str>,
    /// An Element action chosen (the Properties panel's header, Layers').
    act: Option<panels::Act>,
}

/// The selection, as Layers leaves it.
#[derive(Clone, PartialEq)]
struct Chosen {
    selected: String,
    selected_many: std::collections::BTreeSet<String>,
    edit_view: Option<String>,
    draw_item: Option<(String, usize)>,
    layout_target: String,
}

impl Chosen {
    fn of(state: &State) -> Chosen {
        Chosen {
            selected: state.selected.clone(),
            selected_many: state.selected_many.clone(),
            edit_view: state.edit_view.clone(),
            draw_item: state.draw_item.clone(),
            layout_target: state.layout_target.clone(),
        }
    }

    fn put(self, state: &mut State) {
        state.selected = self.selected;
        state.selected_many = self.selected_many;
        state.edit_view = self.edit_view;
        state.draw_item = self.draw_item;
        state.layout_target = self.layout_target;
    }
}

/// Why a designer has nothing to edit, in words for its tab and panels.
type Nothing = &'static str;

/// What a page says where the module no longer holds the window.
const GONE: Nothing = "This window is no longer in the module.";

/// The pictures and the strings (talk table rows) a window names.
#[derive(PartialEq)]
struct Walked {
    /// The window's generation, and the Interact session's stamp where its
    /// window is what was walked.
    at: (u64, Option<u64>),
    names: std::collections::BTreeSet<String>,
    refs: std::collections::BTreeSet<i64>,
}

impl Walked {
    fn of(at: (u64, Option<u64>), v: &Value, s: &Settings) -> Walked {
        let mut names = std::collections::BTreeSet::new();
        skin::names(v, s, &mut names);
        Walked { at, names, refs: skin::strrefs(v, s) }
    }
}

/// The window may have changed: what is kept of it is of another
/// generation ([`State::window_gen`]).
fn unsettle(state: &mut State) {
    state.settled = None;
    state.window_gen += 1;
}

/// Whether the window is the one the last frame drew: [`end`] found it
/// unchanged then, on a frame with no input, and nothing in this frame
/// could have changed it before the canvas draws it (no input, no field
/// held, no popup, nothing dragged). Otherwise a new generation. Once a
/// frame, before the canvas.
fn settle(ctx: &egui::Context, state: &mut State) {
    let proven = state.settled.take() == Some((state.copies, state.edit_view.clone()));
    let quiet = ctx.memory(|m| m.focused().is_none())
        && !egui::Popup::is_any_open(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
        && ctx.dragged_id().is_none()
        && !egui::DragAndDrop::has_any_payload(ctx)
        && ctx.data(|d| d.get_temp::<bool>(mutated_id()).is_none())
        && ctx.input(|i| {
            i.events
                .iter()
                .all(|e| matches!(e, egui::Event::PointerMoved(_) | egui::Event::PointerGone))
                && !i.pointer.any_down()
                && !i.pointer.any_pressed()
                && !i.pointer.any_released()
                && i.keys_down.is_empty()
        });
    state.proven_in = (proven && quiet).then(|| ctx.cumulative_pass_nr());
    if !(proven && quiet) {
        unsettle(state);
    }
}

/// The window's generation, where [`settle`] found it unchanged in this
/// pass: what was kept under it may be taken (a canvas drawn without it,
/// as tests draw one, keeps nothing from frame to frame).
fn trusted(ctx: &egui::Context, state: &State) -> Option<u64> {
    (state.proven_in == Some(ctx.cumulative_pass_nr())).then_some(state.window_gen)
}

fn mutated_id() -> egui::Id {
    egui::Id::new("nui-view-mutated")
}

/// Marks that a deferred `ctx` temp consumer mutated `work` this frame so
/// [`end`] runs the diff even on a quiet frame.
pub(super) fn mark_mutated(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(mutated_id(), true));
}

/// Designer `key`'s frame begins, or goes on (each of its parts calls it
/// before it draws: its Layers, its panels, its editor): the window and its
/// settings read and checked ([`check`]); once a frame, as its first part
/// draws, its fields' stores emptied, a control a link asked for chosen, a
/// variant edited put in the window's place and a selection whose control
/// is gone let go of. What is drawn is then [`State::work`]; [`end`] sets
/// what the frame changed.
fn begin(
    module: &mg_module::Module,
    ctx: &egui::Context,
    key: ResKey,
    state: &mut State,
) -> Result<Checked, Nothing> {
    let copies = state.copies;
    let checked = check(module, key, state)?;
    fields::enter(ctx, key);
    // The window changed under a field still typed in (an edit from
    // elsewhere: a plugin, a reload, an action of other code): its typing
    // ends, what was typed let go of (its field's id is another from now on,
    // so egui drops its keyboard without setting it), never set on the
    // control that took its place.
    if state.copies != copies && !state.reverted.is_empty() {
        state.reverted.clear();
        state.typing += 1;
    }
    let pass = ctx.cumulative_pass_nr();
    if state.began == Some(pass) {
        return Ok(checked);
    }
    state.began = Some(pass);
    state.ended = false;
    fields::begin(ctx, key);
    if let Some(path) = state.select_next.take() {
        state.selected = path;
        state.page = Page::Design;
        if let Some(work) = &mut state.work {
            work.verified_clean = false;
        }
    }
    let Some(work) = &mut state.work else { return Ok(checked) };
    // A variant is edited in the window's place (the panels act on it on
    // every page; the pages but Design put the window's own root back
    // while they draw, `main_root`), and put back as the frame ends.
    // (Taken out of the settings, not copied: its place there holds null
    // until the frame ends.)
    let view = state.edit_view.clone();
    if let Some((name, variant)) =
        view.and_then(|n| Some((n.clone(), work.settings.variants.get_mut(&n)?)))
    {
        let variant = std::mem::take(variant);
        let base = std::mem::replace(&mut work.window["root"], variant);
        state.swapped = Some((name, base));
    }
    // A selection whose control is gone (Undo took it back, the source was
    // edited) is no selection.
    let v = &work.window;
    if v.pointer(&state.selected).is_none() {
        state.selected.clear();
    }
    state.selected_many.retain(|path| v.pointer(path).is_some());
    if !state.selected_many.contains(&state.selected) {
        state.selected_many.clear();
    }
    if let Some((path, layer)) = &state.draw_item
        && v.pointer(&format!("{path}/draw_list/{layer}")).is_none()
    {
        state.draw_item = None;
    }
    Ok(checked)
}

/// Designer `key`'s frame ends (its editor, after the page; or, where its
/// editor was not drawn after its panels this frame, [`follow`]): what
/// Layers and Components asked for done, a control dropped put where it
/// was dropped, the variant edited put back in its place, the values the
/// fields only show taken out (and a field's typed in last frame and not
/// drawn now set), and what changed set as one command, inserted at `at`
/// (where the editor began asking: before a Build it asked for). Once a
/// frame.
fn end(app: &mut Moonglow, ctx: &egui::Context, key: ResKey, state: &mut State, at: Option<usize>) {
    let pass = ctx.cumulative_pass_nr();
    if state.began != Some(pass) || state.ended {
        return;
    }
    state.ended = true;
    fields::enter(ctx, key);
    let name = key.resref.to_string();
    let mut raw = state.typed_raw.take();
    let mut config = state.typed_config.take();
    let picked = std::mem::take(&mut state.picked);
    if let Some(mut work) = state.work.take() {
        let had_pending = picked.chosen.is_some()
            || picked.act.is_some()
            || picked.insert.is_some()
            || state.move_pending.is_some()
            || state.insert_pending.is_some()
            || state.new_swap_slot.is_some()
            || state.resize.is_some()
            || state.draw_drag.is_some()
            || state.window_move.is_some()
            || !state.reverted.is_empty()
            || state.source_draft.as_ref().is_some_and(|d| d.typed)
            || raw.is_some()
            || config.is_some()
            || ctx.data_mut(|d| d.remove_temp::<bool>(mutated_id())).is_some();
        if let Some(chosen) = picked.chosen {
            chosen.put(state);
            ctx.request_repaint();
        }
        if let Some(a) = picked.act {
            panels::act(ctx, a, &mut work.window, &mut work.settings, state);
            ctx.request_repaint();
        }
        if let Some(ty) = picked.insert {
            match ty {
                "swap" => layouts::insert_slot(&mut work.window, &mut work.settings, state),
                ty => design::insert(&mut work.window, &mut state.selected, ty),
            }
            ctx.request_repaint();
        }
        structure::finish(&mut work.window, state);
        layouts::finish_insert(&work.window, &mut work.settings, state);
        if let Some((view, base)) = state.swapped.take() {
            let root = std::mem::replace(&mut work.window["root"], base);
            work.settings.variants.insert(view, root);
        }
        let last = std::mem::take(&mut state.reverted);
        fields::recover(ctx, &last, &mut work.window, &mut work.settings);
        // (Armed below only where nothing can have changed the window.)
        state.settled = None;
        if let Some(Source { window: Ok(was), settings: Ok(was_settings), .. }) = &state.source {
            let has_unset = fields::has_unset(ctx);
            let has_active_ui = ctx.memory(|m| m.focused().is_some())
                || egui::Popup::is_any_open(ctx)
                || mg_uikit::popover::Popover::any_open(ctx)
                || ctx.dragged_id().is_some()
                || egui::DragAndDrop::has_any_payload(ctx);
            let quiet_input = ctx.input(|i| {
                i.events
                    .iter()
                    .all(|e| matches!(e, egui::Event::PointerMoved(_) | egui::Event::PointerGone))
                    && !i.pointer.any_down()
                    && !i.pointer.any_pressed()
                    && !i.pointer.any_released()
                    && i.keys_down.is_empty()
            });
            let can_skip_diff =
                work.verified_clean && !had_pending && !has_unset && !has_active_ui && quiet_input;
            if can_skip_diff {
                state.settled = Some((state.copies, state.edit_view.clone()));
            }
            if has_unset {
                let mut now = Vec::new();
                fields::put_back(ctx, was, &mut work.window, &fields::Place::Window, &mut now);
                fields::put_back_settings(ctx, was_settings, &mut work.settings, &mut now);
                state.reverted = now;
            } else {
                state.reverted.clear();
            }
            if !can_skip_diff {
                let window_changed = raw.is_some() || work.window != *was;
                if raw.is_none() && window_changed {
                    raw = serde_json::to_string_pretty(&work.window).ok();
                }
                let settings_changed = config.is_some() || work.settings != *was_settings;
                if config.is_none() && settings_changed {
                    config = String::from_utf8(work.settings.bytes()).ok();
                }
                work.verified_clean = !window_changed
                    && !settings_changed
                    && state.reverted.is_empty()
                    && !has_active_ui
                    && state.resize.is_none()
                    && state.draw_drag.is_none()
                    && state.window_move.is_none();
            }
        }
        state.work = Some(work);
        if state.settled.is_none() {
            unsettle(state);
        }
    } else {
        state.swapped = None;
        unsettle(state);
    }
    let edits = changes(app, ctx, key, (raw, config), state);
    if edits.is_empty() {
        return;
    }
    // (Where the editor began asking: before a generation requested in this
    // frame, including a text field losing focus when its button was
    // clicked, and among what its Cancel notes.)
    let at = at.unwrap_or(app.actions.len()).min(app.actions.len());
    app.actions.insert(at, Action::Apply(Command::new(format!("Edit NUI {name}"), edits)));
}

/// What designers' panels changed in a frame their editors were not drawn
/// after them (a tab closed meanwhile, a window folded): set as the frame
/// ends, with the frame's other actions; never kept for a later frame. Its
/// Cancel does not set it back: no window drew it. Called once a frame,
/// after the editor area is drawn.
pub(crate) fn follow(app: &mut Moonglow, ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    // A source group whose designer is gone goes with it (closed, or a
    // record that kept the source alone: M4's U19).
    if app.dock.iter_all_tabs().any(|(_, t)| matches!(t, Tab::NuiSource(_))) {
        let designers: Vec<ResKey> = (app.dock.iter_all_tabs())
            .filter_map(|(_, t)| match t {
                Tab::Nui(k) => Some(*k),
                _ => None,
            })
            .collect();
        crate::tabs::close_tabs(&mut app.dock, |t| match t {
            Tab::NuiSource(k) => designers.contains(k),
            _ => true,
        });
    }
    // (A floating designer's source group closed by its own ×: its window
    // as wide as before, as Split source's Layout does.)
    let narrowed: Vec<ResKey> = (app.nui_widened.keys())
        .filter(|k| app.dock.find_tab(&Tab::NuiSource(**k)).is_none())
        .copied()
        .collect();
    for key in narrowed {
        app.actions.push(Action::NuiSource(key, false));
        ctx.request_repaint();
    }
    // What was typed in a source group not drawn this frame is set.
    let typed: Vec<ResKey> = (app.nui_views.iter())
        .filter(|(_, s)| s.source_draft.as_ref().is_some_and(|d| d.typed))
        .map(|(k, _)| *k)
        .collect();
    for key in typed {
        let Some(mut state) = app.nui_views.remove(&key) else { continue };
        source::left(app, ctx, key, &mut state);
        app.nui_views.insert(key, state);
    }
    let open: Vec<ResKey> = (app.nui_views.iter())
        .filter(|(_, s)| s.began == Some(pass) && !s.ended)
        .map(|(k, _)| *k)
        .collect();
    for key in open {
        let Some(mut state) = app.nui_views.remove(&key) else { continue };
        end(app, ctx, key, &mut state, None);
        app.nui_views.insert(key, state);
    }
    // A designer closed while one of its fields was typed in: what was
    // typed set, as a field whose panel goes is (the old fields' rule, "set
    // when left"; the tip lost it).
    let closed: Vec<ResKey> = (app.nui_views.iter())
        .filter(|(k, s)| {
            !s.reverted.is_empty()
                && s.began != Some(pass)
                && app.dock.find_tab(&crate::Tab::Nui(**k)).is_none()
        })
        .map(|(k, _)| *k)
        .collect();
    for key in closed {
        let Some(mut state) = app.nui_views.remove(&key) else { continue };
        // (Only onto the bytes it was typed over: the window gone, or
        // changed from elsewhere meanwhile, lets what was typed go, as
        // `begin` does for a designer still open.)
        let now = app.ws.as_ref().and_then(|ws| held_raw(&ws.module, key).ok());
        let unchanged = state.source.as_ref().zip(now).is_some_and(|(s, now)| same(&s.of, &now));
        if !unchanged {
            state.reverted.clear();
            app.nui_views.insert(key, state);
            continue;
        }
        fields::begin(ctx, key);
        (state.began, state.ended) = (Some(pass), false);
        end(app, ctx, key, &mut state, None);
        state.reverted.clear();
        app.nui_views.insert(key, state);
    }
}

/// Save with a designer's field still typed in: what it shows is set
/// first, as leaving it would (the module saved is what the window shows),
/// only onto the bytes it was typed over. Its keyboard stays.
pub(crate) fn store_typed(app: &mut Moonglow) {
    // (The source group's draft too, M4's U19 review.)
    let sources: Vec<ResKey> = (app.nui_views.iter())
        .filter(|(_, s)| s.source_draft.as_ref().is_some_and(|d| d.typed))
        .map(|(k, _)| *k)
        .collect();
    for key in sources {
        let Some(mut state) = app.nui_views.remove(&key) else { continue };
        source::store(app, key, &mut state);
        app.nui_views.insert(key, state);
    }
    let keys: Vec<ResKey> =
        (app.nui_views.iter()).filter(|(_, s)| !s.reverted.is_empty()).map(|(k, _)| *k).collect();
    for key in keys {
        let Some(mut state) = app.nui_views.remove(&key) else { continue };
        let typed = std::mem::take(&mut state.reverted);
        unsettle(&mut state);
        let now = app.ws.as_ref().and_then(|ws| held_raw(&ws.module, key).ok());
        let source = state.source.as_ref().filter(|s| now.is_some_and(|n| same(&s.of, &n)));
        let was = source.and_then(|s| Some((s.window.as_ref().ok()?, s.settings.as_ref().ok()?)));
        let (Some((was, was_settings)), Some(work)) = (was, state.work.as_mut()) else {
            app.nui_views.insert(key, state);
            continue;
        };
        work.verified_clean = false;
        fields::set_typed(typed.iter(), &mut work.window, &mut work.settings);
        let raw = (work.window != *was).then(|| serde_json::to_string_pretty(&work.window).ok());
        let config =
            (work.settings != *was_settings).then(|| String::from_utf8(work.settings.bytes()).ok());
        let edits = written(app, key, (raw.flatten(), config.flatten()), &mut state);
        app.nui_views.insert(key, state);
        if !edits.is_empty() {
            let name = key.resref.to_string();
            app.run_now(Action::Apply(Command::new(format!("Edit NUI {name}"), edits)));
        }
    }
}

/// The window and its settings as read from the module's text, and what the
/// checks say of them.
struct Checked {
    /// Why the window or its settings don't read, where they don't.
    window_error: Option<String>,
    settings_error: Option<String>,
    findings: std::rc::Rc<[mg_nui::Diagnostic]>,
    /// The errors: the findings that are, and a document that does not read.
    errors: usize,
    warnings: usize,
}

/// The Creator itself: the header, the page chosen, and at the end one
/// command for what the frame changed (its panels' with it). OK or Cancel,
/// where one was pressed. Its source is a group of its own (`source`), its
/// checks the Bottom Dock's Checks (`checks`): M4's U19.
fn editor(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
) -> (Option<crate::blueprint::Pressed>, Option<usize>) {
    // What the tab asks for this frame starts here (its OK and Cancel note it).
    let asked = app.actions.len();
    let id = ui.id().with(("nui", key));
    // Floating dock windows have no native title bar, so egui permits dragging
    // their background. Absorb body drags before registering controls; child
    // text selections, sliders and canvas handles retain their own drag sense.
    let body = ui.available_rect_before_wrap();
    // (Not a stop of Tab: `Sense::DRAG` alone.)
    ui.interact(body, id.with("body-drag-guard"), egui::Sense::DRAG);
    let Some(ws) = &app.ws else {
        ui.label("Open a module first.");
        return (None, None);
    };
    let mut state = app.nui_views.remove(&key).unwrap_or_default();
    let checked = match begin(&ws.module, ui.ctx(), key, &mut state) {
        Ok(checked) => checked,
        Err(why) => {
            ui.label(why);
            app.nui_views.insert(key, state);
            return (None, None);
        }
    };
    let name = key.resref.to_string();
    // (Layout's Split source is the source group open, M4's U19: closed by
    // its tab, the Layout is the Editor's again.)
    // (The dock is the editor area's while it is drawn: its panes' tabs as
    // the frame began.)
    let source = Tab::NuiSource(key);
    let source_open = app.panes_tabs.iter().any(|tabs| tabs.contains(&source));
    match (state.view_mode, source_open) {
        (ViewMode::Split, false) => state.view_mode = ViewMode::Editor,
        (_, true) => state.view_mode = ViewMode::Split,
        _ => {}
    }
    let mode_was = state.view_mode;
    // (Where the pointer is over the editor, the Top Bar's Undo and Redo
    // are the module's, as they are for the rest of it.)
    // (Where it floats, what the last Build left is said in its header,
    // as the Floating board says a script's state there: the Status Bar is
    // the main window's docked editor's. M4's U19.)
    let floating = ui.layer_id().order != egui::Order::Background;
    let built = build_state(app, &name, &state).filter(|_| floating);
    let asked_of = header(ui, &checked, &mut state, built);
    // (Split source chosen opens the source group; another Layout closes
    // it. The page's repair asks for it on a switch position.)
    if let Some(of) = state.open_source.take() {
        source::switch_to(&mut state, of);
        state.view_mode = ViewMode::Split;
        // (Brought forward where it is open behind another tab.)
        app.actions.push(Action::NuiSource(key, true));
    }
    if state.view_mode != mode_was {
        let open = state.view_mode == ViewMode::Split;
        app.actions.push(Action::NuiSource(key, open));
    }
    checks::note_failures(app, key, &checked, &mut state);
    // A floating designer's Layers folded to a strip at the canvas's left,
    // under the header, where its window has not the room for their pane.
    let owner = crate::outliner::Owner::Nui(key);
    if app.outliner.strips_now.contains(&owner) {
        crate::outliner::strip(app, ui, owner);
    }
    match state.page {
        Page::Images => images_page(app, ui, &name, &mut state),
        Page::Events => events_page(app, ui, &name, &mut state),
        Page::Bindings => bindings_page(ui, &mut state),
        Page::Design => design_page(app, ui, key, &mut state),
    }
    // (Done after the page, as the frame's last: a page switched to is drawn
    // from the next frame, its parts beginning on it.)
    run_header(app, &name, &asked_of, &mut state);
    how_to_open(ui, &name, &mut state);
    app.nui_views.insert(key, state);
    // (The frame ends in `ui`, after its window's Layers over the canvas.)
    (asked_of.pressed, Some(asked))
}

/// What the module holds of window `key`: its text and its settings' text,
/// as the shared bytes they are (a pointer each, no copy). Does not scan UTF-8;
/// [`check_utf8`] checks bytes only when the held handles change.
fn held_raw(module: &mg_module::Module, key: ResKey) -> Result<[Held; 2], Nothing> {
    let Some(bytes) = module.shared(&key) else { return Err(GONE) };
    let config = module.shared(&ResKey::new(key.resref, ResType::TXT)).cloned();
    Ok([Some(bytes.clone()), config])
}

fn check_utf8(of: &[Held; 2]) -> Result<(), Nothing> {
    if of[0].as_deref().is_some_and(|bytes| std::str::from_utf8(bytes).is_err()) {
        return Err(
            "This window's JUI is not UTF-8 text: it is kept as it is and can't be edited here.",
        );
    }
    if of[1].as_deref().is_some_and(|c| std::str::from_utf8(c).is_err()) {
        return Err(
            "This window's settings (its TXT) are not UTF-8 text: they are kept as they are.",
        );
    }
    Ok(())
}

/// What a JUI that reads as JSON but is no object says: in the editor, and
/// by `mg nui layout` and `render` ([`headless`]).
const NOT_A_WINDOW: &str = "A window is a JSON object ({ … }): this text is not one";

/// The window and its settings parsed, and the checks run on them: each
/// again only when the module holds other bytes than last frame, kept in
/// `state` between ([`State::source`], [`State::checks`]); the designer's
/// copy to edit made again where they were parsed again.
fn check(module: &mg_module::Module, key: ResKey, state: &mut State) -> Result<Checked, Nothing> {
    let of = held_raw(module, key)?;
    if state.source.as_ref().is_none_or(|s| !same(&s.of, &of)) {
        if let Err(why) = check_utf8(&of) {
            state.source = None;
            state.work = None;
            state.checks = None;
            return Err(why);
        }
        // (A window is an object: text that reads as a list or a number
        // is not one, and the pages, which set its fields, are not shown
        // it.)
        let text = of[0].as_deref().unwrap_or_default();
        let window =
            mg_nui::parse(text).map_err(|e| e.to_string()).and_then(|v| match v.is_object() {
                true => Ok(v),
                false => Err(NOT_A_WINDOW.to_string()),
            });
        if let Ok(doc) = &window {
            state.main_doc = Some(doc.clone());
            state.layout_groups = mg_nui::group_ids(doc);
        }
        let settings = match &of[1] {
            Some(bytes) => Settings::parse(bytes).map_err(|e| e.to_string()),
            None => Ok(Settings::default()),
        };
        state.work = match (&window, &settings) {
            (Ok(w), Ok(s)) => {
                state.copies += 1;
                Some(Work {
                    of: of.clone(),
                    window: w.clone(),
                    settings: s.clone(),
                    verified_clean: false,
                })
            }
            _ => None,
        };
        state.source = Some(Source { of: of.clone(), window, settings });
    }
    let source = state.source.as_ref().expect("just set");
    // Checking the whole document is not free: only redo it when the window,
    // its settings or its generated scripts change.
    // (Its scripts' keys made once for the designer, not for each of its
    // parts in every frame.)
    let scripts = *state.script_keys.get_or_insert_with(|| {
        let name = key.resref.to_string();
        [("_o", ResType::NSS), ("_o", ResType::NCS), ("_e", ResType::NSS), ("_e", ResType::NCS)]
            .map(|(suffix, ty)| ResKey::parse(&format!("{name}{suffix}"), ty))
    });
    let shared = |key: Option<ResKey>| key.and_then(|k| module.shared(&k).cloned());
    let [jui, txt] = of;
    let checked: [Held; 6] =
        [jui, txt, shared(scripts[0]), shared(scripts[1]), shared(scripts[2]), shared(scripts[3])];
    if state.checks.as_ref().is_none_or(|c| !same(&c.of, &checked)) {
        let name = key.resref.to_string();
        let (findings, needs_build) = match (&source.window, &source.settings) {
            (Ok(v), Ok(s)) => (mg_nui::validate(v, s), !mg_nui::is_current(module, &name, v, s)),
            _ => (Vec::new(), false),
        };
        state.checks = Some(Checks { of: checked, findings: findings.into(), needs_build });
    }
    let findings = state.checks.as_ref().map(|c| c.findings.clone()).expect("just set");
    let window_error = source.window.as_ref().err().cloned();
    let settings_error = source.settings.as_ref().err().cloned();
    let errors = findings.iter().filter(|d| d.severity == Severity::Error).count()
        + usize::from(window_error.is_some())
        + usize::from(settings_error.is_some());
    let warnings = findings.iter().filter(|d| d.severity == Severity::Warning).count();
    Ok(Checked { window_error, settings_error, findings, errors, warnings })
}

/// What the Status Bar shows for designer `key` in front (M4's U19, the NUI
/// board's): the canvas's ways and keys where the canvas takes them (`canvas`:
/// the design is the active editor, not its source group; the Design page,
/// nothing typed in, no popover, dialog or menu open), each as Options ›
/// Keyboard binds it: Click Select, Drag Resize by handle, Esc Cancel resize,
/// and with a control chosen Ctrl + D Duplicate, Alt + Up Move up, Delete;
/// in Interact, Click Try a control. Its counts: what the last Build left
/// ("Build failed", the reason in its tip, or "Not built"), in `warn`, then
/// "3 binds, 3 handlers".
pub(crate) fn shown(
    app: &Moonglow,
    k: &crate::shell::HintKeys<'_>,
    ctx: &egui::Context,
    key: ResKey,
    canvas: bool,
) -> crate::shell::Shown {
    use crate::keys::Cmd;
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let Some(state) = app.nui_views.get(&key) else { return Shown::default() };
    let mut hints = Vec::new();
    let quiet = !crate::shell::keys_blocked(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
        && ctx.memory(|m| m.focused().is_none());
    if canvas && quiet && state.page == Page::Design && state.work.is_some() {
        if state.preview_interactive {
            hints.push(Hint::keys(["Click"], "Try a control"));
        } else {
            hints.push(Hint::keys(["Click"], "Select"));
            hints.push(Hint::keys(["Drag"], "Resize by handle"));
            hints.push(Hint::keys(["Esc"], "Cancel resize"));
            let keys = state.view_mode != ViewMode::Preview && !state.selected.is_empty();
            if keys && state.selected != "/root" {
                hints.extend(k.hint(Cmd::NuiDuplicate, "Duplicate"));
                hints.extend(k.hint(Cmd::NuiMoveUp, "Move up"));
                hints.extend(k.hint(Cmd::NuiDelete, "Delete"));
            }
        }
    }
    let mut counts = Vec::new();
    if let Some((words, tip)) = build_state(app, &key.resref.to_string(), state) {
        let mut c = Count::new(words);
        c.tip = Some(tip);
        c.warn = true;
        counts.push(c);
    }
    if let Some(work) = &state.work {
        let s = &work.settings;
        let words =
            format!("{}, {}", count(s.bindings.len(), "bind"), count(s.handlers.len(), "handler"));
        counts.push(Count::new(words));
    }
    Shown { hints, counts, note: None }
}

/// The name of the designer's header (the kit's group, the tests'), the
/// board's.
pub const NUI_HEADER: &str = "NUI window header";

/// The header's commands' names (its record's, the tests').
pub const BUILD: &str = "Build & Compile";
pub const EXPORT: &str = "Export…";
pub const IMPORT: &str = "Import…";
pub const HOW_TO_OPEN: &str = "How to Open It…";
pub const INTERACT: &str = "Interact";
pub const RESET: &str = "Reset";
pub const VIEW: &str = "View";
/// The pages' switch's name.
pub const PAGE: &str = "Page";
/// What the last Build left, in a floating designer's header (M4's U19).
pub const BUILD_STATE: &str = "Build state";

/// What the last Build of window `name` left, where it is to be said: "Build
/// failed" (the reason its tip), else "Not built" (its scripts are not what
/// Build makes of it now); `None` where it is built.
fn build_state(app: &Moonglow, name: &str, state: &State) -> Option<(&'static str, String)> {
    if let Some((_, why)) = app.nui_build_error.as_ref().filter(|(n, _)| n == name) {
        return Some(("Build failed", why.clone()));
    }
    state
        .checks
        .as_ref()
        .is_some_and(|c| c.needs_build)
        .then(|| ("Not built", "Build before testing: the game runs what was last built".into()))
}

/// What the header asked for, done after the page ([`run_header`]).
#[derive(Default)]
struct HeaderAsked {
    page: Option<Page>,
    build: bool,
    export: bool,
    import: bool,
    how_to_open: bool,
    pressed: Option<crate::blueprint::Pressed>,
}

/// The pages of the switch, in its order, with their words and tips (the
/// board's).
const PAGES: [(Page, &str, &str); 4] = [
    (Page::Design, "Design", "The window on its canvas, its controls and their properties"),
    (Page::Bindings, "Bindings", "The window's binds: their values, watches and handlers"),
    (Page::Events, "Events & Variants", "Every handler and swap layout variant"),
    (Page::Images, "Images", "The pictures the window shows and where the game finds them"),
];

/// The designer's header (M4's U18, the NUI board's `Nui_fix_hdr4.png`): the
/// pages' switch (Design, Bindings, Events & Variants, Images); a line;
/// Build & Compile, the primary icon button; Export…; More (Import…, How to
/// Open It…); at the right Interact and Reset, and View, a popover of how
/// the canvas shows the window (Layout, UI scale, State, Resources, Clean
/// view, Screen); then OK and Cancel at its end (ruling 10). What has no
/// room loses its words, then goes into More.
fn header(
    ui: &mut Ui,
    checked: &Checked,
    state: &mut State,
    built: Option<(&'static str, String)>,
) -> HeaderAsked {
    use mg_uikit::Icon;
    use mg_uikit::header::{EditorHeader, Item};
    use mg_uikit::popover::{PopCheck, Popover};
    use std::cell::Cell;
    let shown = Some(state.page);
    let page = Cell::new(shown);
    let choices: Vec<(Option<Page>, &str, &str)> =
        PAGES.iter().map(|(p, words, tip)| (Some(*p), *words, *tip)).collect();
    // (Measured as the kit lays it out: each choice its words and the
    // padding at each side, a line between them; as the area header's Day
    // | Night.)
    let switch_width = form::segmented_width(
        ui,
        egui::Id::new("nui-pages-switch-w"),
        &[PAGES[0].1, PAGES[1].1, PAGES[2].1, PAGES[3].1],
    );
    let switch = |ui: &mut Ui| {
        let mut chosen = page.get();
        // (In a room of its own size: the group's place is its choices'.)
        let room = egui::vec2(switch_width, mg_uikit::metrics::CONTROL_HEIGHT);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(room, layout, |ui| {
            mg_uikit::segmented(ui, PAGE, &mut chosen, &choices);
        });
        page.set(chosen);
    };
    let build_tip = match checked.errors {
        0 => "Build & Compile: check the opener include and compile the event script".to_string(),
        n => format!("Fix the {} first: Checks shows them", count(n, "error")),
    };
    let mut interact = state.preview_interactive;
    // The View popover: how the canvas shows the window, each as its old
    // control did (the canvas's row, the Design page's layout dropdown).
    let mut mode = state.view_mode;
    let mut scale = state.preview_scale;
    let mut shown_state = state.preview_state;
    let zoom_c = Cell::new(state.zoom);
    let mut resources = state.preview_info;
    let mut clean = state.preview_clean;
    let mut on_screen = state.screen_preview;
    let mut screen = state.screen_size;
    let interacting = state.preview_interactive;
    let (mode_c, scale_c, state_c, screen_c) =
        (Cell::new(mode), Cell::new(scale), Cell::new(shown_state), Cell::new(screen));
    let view = Popover::new(VIEW)
        .title(None, VIEW)
        .min_width(VIEW_LABEL + VIEW_COMBO + 24.0)
        .custom(|ui| view_combo(ui, "Layout", &mode_c, &ViewMode::ALL, |m| m.label().into(), true))
        .custom(|ui| {
            let scales = [1.0_f32, 1.25, 1.5, 2.0];
            view_combo(ui, "UI scale", &scale_c, &scales, |s| format!("{:.0}%", s * 100.0), true);
        })
        .custom(|ui| {
            // (The zoom, as the canvas's corner has it: Fit, 100%.)
            let words = |z: &f32| if *z == 0.0 { "Fit".into() } else { "100%".into() };
            view_combo(ui, "Zoom", &zoom_c, &[0.0, 1.0], words, true);
        })
        .custom(|ui| {
            view_combo(
                ui,
                "State",
                &state_c,
                &[0, 1, 2, 3, 4],
                |i| STATES[*i].into(),
                !interacting,
            );
        })
        .separator()
        .check(
            PopCheck::new(&mut resources, "Resources")
                .tip("Where each picture and font came from, and what was not found"),
        )
        .check(
            PopCheck::new(&mut clean, "Clean view")
                .tip("The window alone, without the editor's guides"),
        )
        .separator()
        .check(
            PopCheck::new(&mut on_screen, "Preview on screen")
                .tip("The window on a screen of the size chosen: drag its title to place it"),
        )
        .custom(|ui| {
            let sizes =
                [egui::vec2(1280.0, 720.0), egui::vec2(1920.0, 1080.0), egui::vec2(2560.0, 1440.0)];
            view_combo(ui, "Screen", &screen_c, &sizes, |s| format!("{} × {}", s.x, s.y), true);
        });
    let built_width = built.as_ref().map_or(0.0, |(words, _)| {
        let font = egui::TextStyle::Body.resolve(ui.style());
        ui.fonts_mut(|f| f.layout_no_wrap((*words).to_string(), font, egui::Color32::WHITE))
            .size()
            .x
            + 8.0
    });
    let built_item = built.map(|(words, tip)| {
        let warn = mg_uikit::tokens(ui.ctx()).warn;
        let add = move |ui: &mut Ui| {
            ui.colored_label(warn, words).on_hover_text(tip.as_str());
        };
        Item::custom(BUILD_STATE, built_width, add)
    });
    let mut header = EditorHeader::new(NUI_HEADER);
    if let Some(item) = built_item {
        header = header.right(item);
    }
    let header = header
        .more_after(EXPORT)
        .left(Item::custom(PAGE, switch_width, switch).more(switch_width, switch).primary())
        .left(Item::separator())
        .left(
            Item::button(Icon::Compile, BUILD)
                .look(mg_uikit::ButtonLook::Primary)
                .icon_only()
                .primary()
                .enabled(checked.errors == 0)
                .tip(build_tip),
        )
        .left(
            Item::icon(Icon::Export, EXPORT)
                .tip("Export…: build the window, then export it with its settings and scripts"),
        )
        .left(
            Item::button(Icon::Import, IMPORT)
                .in_more()
                .tip("Bring NUI windows into the module from .jui files or ERF and zip archives"),
        )
        .left(
            Item::button(Icon::Help, HOW_TO_OPEN)
                .in_more()
                .tip("What to write in a script of yours to open the window"),
        )
        .right(
            Item::toggle(Some(Icon::Play), INTERACT, &mut interact)
                .icon_only()
                .tip("Interact: try the controls on the canvas"),
        )
        .right(
            Item::icon(Icon::Restart, RESET)
                .enabled(interacting)
                .tip("Reset: the controls' initial values (in Interact)"),
        )
        .right(
            Item::popover(None, view)
                .tip("Layout, UI scale, state, zoom, resources, clean view and screen"),
        );
    let out =
        crate::widgets::header_with_ok_cancel(ui, header, None, crate::blueprint::OK_CANCEL_TIPS);
    (mode, scale, shown_state, screen) =
        (mode_c.get(), scale_c.get(), state_c.get(), screen_c.get());
    // (Interact turned on or off, Reset: a new simulation, from the start,
    // as the canvas's row's did.)
    state.interact(interact);
    if out.clicked(RESET) {
        state.reset();
    }
    state.view_mode = mode;
    state.zoom = zoom_c.get();
    state.preview_scale = scale;
    state.preview_state = shown_state;
    state.preview_info = resources;
    state.preview_clean = clean;
    // (A screen size chosen puts the window on it, as the Screen menu's
    // sizes did.)
    if screen != state.screen_size {
        on_screen = true;
    }
    state.screen_preview = on_screen;
    state.screen_size = screen;
    HeaderAsked {
        page: page.get().filter(|p| Some(*p) != shown),
        build: out.clicked(BUILD),
        export: out.clicked(EXPORT),
        import: out.clicked(IMPORT),
        how_to_open: out.clicked(HOW_TO_OPEN),
        pressed: out.end,
    }
}

/// The states the canvas shows the chosen control in.
const STATES: [&str; 5] = ["Follow pointer", "Normal", "Hover", "Pressed", "Disabled"];

/// A row of the View popover: its label and a combo box of `choices`, the
/// one in `value` shown by `words`; off where not `enabled` (State, while
/// Interact is on). A combo box that opens in the popover leaves it open.
fn view_combo<T: Copy + PartialEq>(
    ui: &mut Ui,
    label: &str,
    value: &std::cell::Cell<T>,
    choices: &[T],
    words: impl Fn(&T) -> String,
    enabled: bool,
) {
    // (Its own width, the label's and the combo box's: a row taking the
    // room it is given would widen the popover to the window's.)
    let size = egui::vec2(VIEW_LABEL + VIEW_COMBO + 8.0, mg_uikit::metrics::CONTROL_HEIGHT);
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    ui.allocate_ui_with_layout(size, layout, |ui| {
        // (The label in its column, the cursor past it: every row's combo
        // box at one left edge, as a form's.)
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(VIEW_LABEL, size.y), egui::Sense::hover());
        let at = egui::UiBuilder::new().max_rect(rect).layout(layout);
        let lead = ui.new_child(at).add(egui::Label::new(label).truncate());
        let mut v = value.get();
        ui.add_enabled_ui(enabled, |ui| {
            let r = widgets::dropdown(("nui-view", label))
                .selected_text(words(&v))
                .width(VIEW_COMBO)
                .show(ui, &mut v, choices, &words);
            let r = r.labelled_by(lead.id);
            ui.ctx().accesskit_node_builder(r.id, |node| node.set_label(label));
        });
        value.set(v);
    });
}

/// The View popover's labels' and combo boxes' widths.
const VIEW_LABEL: f32 = 96.0;
const VIEW_COMBO: f32 = 160.0;

/// Does what the header asked for: a page chosen (from the next frame),
/// Build & Compile, Export…, Import…, How to Open It….
fn run_header(app: &mut Moonglow, name: &str, asked: &HeaderAsked, state: &mut State) {
    if let Some(page) = asked.page {
        state.page = page;
    }
    if asked.build {
        app.actions.push(Action::GenerateNui { name: name.to_owned(), export: false });
    }
    if asked.export {
        app.actions.push(Action::GenerateNui { name: name.to_owned(), export: true });
    }
    if asked.import {
        app.actions.push(Action::ImportNui);
    }
    if asked.how_to_open {
        state.how_to_open = true;
    }
}

/// How to Open It… (More's): what to write in a script of the builder's to
/// open the window, to copy; a dialog over the designer (Escape closes it
/// alone, as every dialog's), as the header's menu was, until closed.
fn how_to_open(ui: &mut Ui, name: &str, state: &mut State) {
    if !state.how_to_open {
        return;
    }
    let code = format!(
        "#include \"{name}_o\"\n\n\
         void main()\n{{\n    Open_{name}(GetEnteringObject());\n}}\n"
    );
    let mut open = true;
    let footed = widgets::dialog(ui.ctx(), "How to Open It", &mut open)
        .collapsible(false)
        .resizable(false)
        .show_foot(ui.ctx(), |ui| {
            ui.label("Open it from a script of yours, for example the module's OnClientEnter:");
            let mut shown = code.as_str();
            let field = egui::TextEdit::multiline(&mut shown).code_editor().desired_rows(6);
            crate::widgets::text_area(ui, ui.id().with("nui-open-code"), field);
            ui.weak("Pass the player who should see it. Build & Compile first.");
            ((), widgets::Foot::new().main("Copy", true).leave("Close"))
        });
    if let Some(footed) = &footed {
        if footed.pressed.main() {
            ui.ctx().copy_text(code);
            state.how_to_open = false;
        } else if footed.pressed.cancelled() {
            state.how_to_open = false;
        }
    }
    if !open {
        state.how_to_open = false;
    }
}

/// The list of problems as it is shown: the findings in its order, each
/// with the control it is about (its path, to select it) and that place in
/// words, for the findings they were made of.
struct ProblemRows {
    of: std::rc::Rc<[mg_nui::Diagnostic]>,
    rows: Vec<(usize, (Option<String>, String))>,
}

/// The buttons after a finding that put it right: Bind It, Turn It Off, Add
/// Close Handler. `target` is the control the finding is about; `doc` and
/// `s` the window and its settings, changed where one is pressed.
fn fixes(
    ui: &mut Ui,
    doc: &mut Value,
    s: &mut Settings,
    d: &mg_nui::Diagnostic,
    target: Option<&str>,
) {
    if d.message.starts_with("What the player enters here is lost")
        && let Some(path) = target
        && ui.small_button("Bind It").on_hover_text("Its value becomes a new bind").clicked()
        && let Some(node) = doc.pointer_mut(path)
    {
        let mut i = 1;
        while s.bindings.contains_key(&format!("value_{i}")) {
            i += 1;
        }
        let bind = format!("value_{i}");
        let value = node["value"].take();
        s.bindings.insert(bind.clone(), Binding { value, ..Default::default() });
        node["value"] = json!({"bind": bind});
    }
    // Clip to control does nothing but harm, so it isn't
    // offered; one written by hand is turned off here.
    if d.path.ends_with("/draw_list_scissor") && ui.small_button("Turn It Off").clicked() {
        let at = d.path.strip_prefix("/views/").and_then(|p| p.split_once('/'));
        let flag = match at {
            Some((view, rest)) => {
                s.variants.get_mut(view).and_then(|v| v.pointer_mut(&format!("/{rest}")))
            }
            None => doc.pointer_mut(&d.path),
        };
        if let Some(flag) = flag {
            *flag = json!(false);
        }
    }
    if d.message.starts_with("The mg_close button has no Clicked handler")
        && ui.small_button("Add Close Handler").clicked()
    {
        s.handlers.push(mg_nui::Handler {
            event: "click".into(),
            element: "mg_close".into(),
            action: mg_nui::Action::Close,
            extra: Default::default(),
        });
    }
}

/// The Images page: the window's pictures, and what it adds to the module
/// (its edits).
fn images_page(app: &mut Moonglow, ui: &mut Ui, name: &str, state: &mut State) {
    let Some(mut work) = state.work.take() else {
        repair(ui, state);
        return;
    };
    main_root(&mut work, state, true);
    let edits = images::page(ui, app, name, &mut work.window, &work.settings, state);
    main_root(&mut work, state, false);
    state.page_edits.extend(edits);
    state.work = Some(work);
}

/// The Events & Variants page.
fn events_page(app: &mut Moonglow, ui: &mut Ui, name: &str, state: &mut State) {
    let Some(mut work) = state.work.take() else {
        repair(ui, state);
        return;
    };
    let script = ResKey::parse(&format!("{name}_e"), ResType::NSS)
        .and_then(|k| events::current_script(app, &k));
    main_root(&mut work, state, true);
    workflow::ui(ui, &mut work.window, &mut work.settings, state, name, script.as_deref());
    main_root(&mut work, state, false);
    state.work = Some(work);
}

/// The Bindings page: the binds, with the window where it names them.
fn bindings_page(ui: &mut Ui, state: &mut State) {
    let Some(mut work) = state.work.take() else {
        let window_reads = state.source.as_ref().is_some_and(|s| s.window.is_ok());
        if window_reads {
            ui.label("The window's settings don't read as JSON: Binding JSON shows them.");
            if ui.button("Open Binding JSON").clicked() {
                state.open_source = Some(SourceOf::Settings);
            }
        } else {
            repair(ui, state);
        }
        return;
    };
    main_root(&mut work, state, true);
    design::bindings(ui, &mut work.settings, Some(&mut work.window), Some(&work.of), state);
    main_root(&mut work, state, false);
    state.work = Some(work);
}

/// The window as it is kept (its own root, the variant edited back in the
/// settings: `to_main`) or as the panels edit it (the variant in the
/// root's place): the pages but Design work on the window as it is kept,
/// and call this before they draw and after. Nothing is copied.
fn main_root(work: &mut Work, state: &mut State, to_main: bool) {
    let Some((name, held)) = &mut state.swapped else { return };
    let Some(slot) = work.settings.variants.get_mut(name.as_str()) else {
        // (Renamed or removed by the page meanwhile: the window's own root
        // stays, and the variant goes where the page put it.)
        if !to_main {
            state.swapped = None;
        }
        return;
    };
    if to_main {
        std::mem::swap(&mut work.window["root"], held);
        std::mem::swap(slot, held);
    } else {
        std::mem::swap(slot, held);
        std::mem::swap(&mut work.window["root"], held);
    }
}

/// The Design page: the variants' bar, then the window (or the variant
/// being edited, in the window's place) in the view chosen. Its palette,
/// properties, binds and layers are the docks' (`panels`).
fn design_page(app: &mut Moonglow, ui: &mut Ui, key: ResKey, state: &mut State) {
    let Some(mut work) = state.work.take() else {
        repair(ui, state);
        return;
    };
    // (A variant chosen on the bar is edited from the next frame on, as the
    // frame ends: this frame's canvas and source are the window it began
    // with.)
    let before = Chosen::of(state);
    layouts::bar(ui, &work.settings, state);
    let now = Chosen::of(state);
    if now != before {
        state.picked.chosen = Some(now);
        before.put(state);
    }
    design_view(app, ui, key, &mut work.window, &mut work.settings, state);
    state.work = Some(work);
}

/// The Design page's view of `v` (the window, or the variant in its place):
/// the canvas with the editor's keys (its panels are the docks'), or the
/// canvas alone (Preview only). The source group beside it (Split source,
/// M4's U19) is a tab of its own (`source`): what is typed there shows on
/// the canvas as it is typed, where it reads, before it is set.
fn design_view(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
) {
    let Some(ws) = app.ws.as_ref() else { return };
    settle(ui.ctx(), state);
    let generation = trusted(ui.ctx(), state);
    let interacting = state.preview_interactive
        && state.runtime.as_mut().is_some_and(|r| r.matches(generation, v, s));
    let (asset_doc, asset_settings, session) = match &state.runtime {
        Some(r) if interacting => (&r.doc, &r.settings, Some(r.revision)),
        _ => (&*v, &*s, None),
    };
    // The pictures and strings it names, walked for again only where the
    // window (or Interact's) may have changed.
    let at = (state.window_gen, session);
    let kept = state.walked.take().filter(|w| generation.is_some() && w.at == at);
    if let Some(w) = &kept {
        debug_assert!(*w == Walked::of(at, asset_doc, asset_settings), "a window changed");
    }
    let walked = kept.unwrap_or_else(|| Walked::of(at, asset_doc, asset_settings));
    app.nui_assets.prepare_names(
        ui.ctx(),
        app.game.as_deref().map(|g| &g.resman),
        &ws.module,
        ws.revision(),
        &walked.names,
    );
    app.nui_assets.prepare_strings_of(app.game.as_deref(), &walked.refs);
    state.walked = Some(walked);
    // The canvas shows the source as it is typed, where it reads, before
    // the field is left; it edits nothing of it meanwhile.
    if let Some((mut shown, settings)) = typed_source(state, v) {
        let settings = settings.as_ref().unwrap_or(s);
        // (With the pictures and fonts it names.)
        app.nui_assets.prepare(
            ui.ctx(),
            app.game.as_deref().map(|g| &g.resman),
            &ws.module,
            ws.revision(),
            &shown,
            settings,
        );
        preview::canvas(ui, &mut shown, settings, state, &mut app.nui_assets);
        return;
    }
    preview::canvas(ui, v, s, state, &mut app.nui_assets);
    if state.view_mode == ViewMode::Preview {
        return;
    }
    // (The editor's keys, with the pointer over the canvas, where no list
    // or panel has them.)
    // (A press on the canvas gives them back to it.)
    let canvas = ui.min_rect();
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(canvas) {
        crate::palette_view::give_arrows(ui.ctx(), false);
    }
    let ctx = ui.ctx().clone();
    let theirs = crate::palette_view::has_arrows(&ctx)
        && !crate::palette_view::keys_are(&ctx, crate::palette_view::Keys::Area);
    // (Only the active editor's: another designer under the pointer takes
    // none, as U16's grid and U17's list; nor its own source group, which
    // has the keys while it is the active editor, M4's U19.)
    let active = crate::table_view::active_tab(app) == Some(Tab::Nui(key));
    if !theirs && active {
        shortcuts::keys(ui, v, s, state, &app.keymap);
    }
}

/// What the frame changed, as the edits of one command: the pages' own
/// (`State::page_edits`), a picture chosen from disk, the event script
/// where it follows the handlers or was asked for, and the window's and the
/// settings' text where the frame wrote them (`texts`: the window's, the
/// settings'; `None` where it left them as the module holds them).
fn changes(
    app: &mut Moonglow,
    ctx: &egui::Context,
    key: ResKey,
    (raw, config): (Option<String>, Option<String>),
    state: &mut State,
) -> Vec<Edit> {
    let mut edits = std::mem::take(&mut state.page_edits);
    if let Some(edit) = images::answer_from_disk(ctx, app) {
        if let Some(work) = &mut state.work {
            work.verified_clean = false;
        }
        edits.push(edit);
    }
    edits.extend(written(app, key, (raw, config), state));
    edits
}

/// The edits that write `raw` and `config` (the window's and its settings'
/// new text, where they changed), the event script following them.
fn written(
    app: &mut Moonglow,
    key: ResKey,
    (raw, config): (Option<String>, Option<String>),
    state: &mut State,
) -> Vec<Edit> {
    let name = key.resref.to_string();
    let config_key = ResKey::new(key.resref, ResType::TXT);
    let mut edits = Vec::new();
    let original = |app: &Moonglow, k: &ResKey| {
        app.ws
            .as_ref()
            .and_then(|ws| ws.module.get(k))
            .map(|b| String::from_utf8_lossy(b).into_owned())
    };
    let mut config = config;
    if raw.is_some() || config.is_some() {
        let window_was = original(app, &key).unwrap_or_default();
        let config_was = original(app, &config_key)
            .unwrap_or_else(|| String::from_utf8(Settings::default().bytes()).unwrap_or_default());
        let window_now = raw.clone().unwrap_or_else(|| window_was.clone());
        let mut config_now = config.clone().unwrap_or_else(|| config_was.clone());
        let followed = events::follow_controls(
            app,
            &name,
            &window_was,
            &window_now,
            &config_was,
            &mut config_now,
        )
        .or_else(|| events::sync_script(app, &name, &config_was, &config_now));
        if let Some(edit) = followed {
            edits.push(edit);
        }
        if config_now != config_was {
            config = Some(config_now);
        }
    }
    if let Some(ask) = state.event_script.take()
        && let Some(edit) = events::open_script(app, &name, ask)
    {
        edits.push(edit);
    }
    if let Some(raw) = raw {
        edits.push(Edit::SetResource { key, data: Some(raw.into_bytes()) });
    }
    if let Some(config) = config {
        edits.push(Edit::SetResource { key: config_key, data: Some(config.into_bytes()) });
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

/// What the source group shows (its switch, M4's U19): the window's text
/// as the module holds it (JUI Source), or its settings' (Binding JSON).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum SourceOf {
    #[default]
    Window,
    Settings,
}

/// What is typed in the source group, until the field is left: a draft that
/// doesn't read yet isn't written to the module at every key.
struct SourceDraft {
    of: SourceOf,
    text: String,
    /// Typed in since it was last set.
    typed: bool,
    /// What `text` reads as, for the canvas ([`typed_source`]): read once
    /// for a text (`None`: not since it changed), not every frame.
    read: Option<Option<TypedSource>>,
    /// The bytes it was read from (the module's, a pointer): read again
    /// only where the module holds others, and set onto only those.
    from: Held,
    /// Its lines, counted as it changes; and how many times it changed (its
    /// galley kept for that, `State::source_galley`).
    lines: usize,
    made: u64,
}

/// A source field's text, read.
enum TypedSource {
    Window(Value),
    Settings(Box<Settings>),
}

/// What the source group's field holds while it is typed in, where it
/// reads: the window to show (`v` with the variant typed in its place, for
/// the settings' source, or `v` with the settings typed) and the settings
/// typed (`None`: the module's). `None` where nothing is typed or it
/// doesn't read yet. The text is read when it changes, not every frame it
/// is shown.
fn typed_source(state: &mut State, v: &Value) -> Option<(Value, Option<Settings>)> {
    let draft = state.source_draft.as_mut().filter(|d| d.typed)?;
    let read = draft.read.get_or_insert_with(|| match draft.of {
        SourceOf::Window => (mg_nui::parse(draft.text.as_bytes()).ok())
            .filter(|window| window.get("root").is_some())
            .map(TypedSource::Window),
        SourceOf::Settings => (Settings::parse(draft.text.as_bytes()).ok())
            .map(|settings| TypedSource::Settings(Box::new(settings))),
    });
    match read.as_ref()? {
        TypedSource::Window(window) => Some((window.clone(), None)),
        TypedSource::Settings(settings) => {
            let mut shown = v.clone();
            if let Some(variant) = state.edit_view.as_ref().and_then(|n| settings.variants.get(n)) {
                shown["root"] = variant.clone();
            }
            Some((shown, Some((**settings).clone())))
        }
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
        // (The source group on that switch position, M4's U19.)
        if ui.button("Open JUI Source").clicked() {
            state.open_source = Some(SourceOf::Window);
        }
        if ui.button("Open Binding JSON").clicked() {
            state.open_source = Some(SourceOf::Settings);
        }
    });
}

/// What a page says when the window's JSON doesn't read.
const REPAIR: &str = "The window's JSON doesn't read: JUI Source and Binding JSON show it as it \
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

/// The properties of the control selected (or of the window): the Title
/// bar check box, the sections (each a bold heading that folds on a click)
/// with the fields that are set, Add Property for those that are not, and
/// Advanced Properties for what is left.
fn properties(ui: &mut Ui, node: &mut Value, s: &mut Settings) {
    let Some(obj) = node.as_object_mut() else {
        ui.weak("Edit this value in JUI Source.");
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
        let layout_sizes = *section == "Size & Layout" && ty != "window";
        if !layout_sizes
            && !keys
                .iter()
                .any(|k| obj.get(*k).is_some_and(|v| !v.is_null() || has_property_default(&ty, k)))
        {
            continue;
        }
        let revealing = reveal.as_deref().is_some_and(|key| keys.contains(&key));
        let id = ("nui-properties", *section);
        form::section(ui, id, section, *open, revealing.then_some(true), None, |ui| {
            let mut skip_height = false;
            for key in *keys {
                if *key == "height" && skip_height {
                    continue;
                }
                if layout_sizes && matches!(*key, "width" | "height") && !obj.contains_key(*key) {
                    let (label, fixed) =
                        if *key == "width" { ("Width", 150.0) } else { ("Height", 30.0) };
                    form::row(ui, label, None, |ui, _| {
                        // It has none: the layout decides; the button gives it one.
                        ui.weak("Automatic");
                        let set = ui
                            .small_button("Set Size")
                            .on_hover_text("Give it a fixed size instead")
                            .clicked();
                        if set {
                            obj.insert((*key).into(), json!(fitting_size(obj, key, fixed)));
                        }
                    });
                    continue;
                }
                // Width and Height on one row, where both are numbers set
                // and the row has the room (the board's).
                if *key == "width" && size_row(ui, obj, &ty, s, &mut remove) {
                    skip_height = true;
                    continue;
                }
                if let Some(value) = obj.get_mut(*key)
                    && (!value.is_null() || has_property_default(&ty, key))
                    && !(*key == "title" && *value == json!(false))
                    // Optional window settings show once added.
                    && !(value.is_null()
                        && matches!(*key, "size_constraint" | "edge_constraint" | "collapsed"))
                {
                    let field = ui.push_id(*key, |ui| {
                        property(ui, key, &ty, value, s, &mut remove);
                    });
                    if reveal.as_deref() == Some(*key) {
                        field.response.scroll_to_me(Some(egui::Align::Center));
                    }
                }
            }
        });
    }
    ui.add_space(form::ROW_GAP);
    add_property(ui, obj, &ty, reveal_id);
    let advanced_keys = advanced_keys(obj, &ty);
    if !advanced_keys.is_empty() {
        form::section(
            ui,
            "nui-advanced-properties",
            "Advanced Properties",
            false,
            None,
            None,
            |ui| {
                for key in &advanced_keys {
                    let Some(value) = obj.get_mut(key) else { continue };
                    ui.push_id(key, |ui| {
                        property(ui, key, &ty, value, s, &mut remove);
                    });
                }
            },
        );
    }
    if let Some(key) = remove {
        obj.remove(&key);
    }
}

/// The room beside Width's label that Width and Height take on one row
/// (the board's two number fields, Height's label between, as a dock of
/// the Right Dock's default width has them: 52 each, the label 48) and
/// their one More.
const SIZE_ROW: f32 = SIZE_FIELD + SIZE_LABEL + SIZE_FIELD + 26.0;
const SIZE_FIELD: f32 = 52.0;
const SIZE_LABEL: f32 = 48.0;

/// Width and Height on one row (the board's "Width [100] Height [34]"),
/// one More after them with each one's rows, where both are numbers set
/// (not bound) and the row has the room; whether it was drawn so.
fn size_row(
    ui: &mut Ui,
    obj: &mut serde_json::Map<String, Value>,
    ty: &str,
    s: &mut Settings,
    remove: &mut Option<String>,
) -> bool {
    let both = ["width", "height"].iter().all(|k| obj.get(*k).is_some_and(Value::is_number));
    if !both || !form::beside(ui, "Width", SIZE_ROW) {
        return false;
    }
    form::row_least(ui, "Width", None, SIZE_ROW, |ui, lead| {
        for (key, words) in [("width", "Width"), ("height", "Height")] {
            ui.push_id(key, |ui| {
                if key == "height" {
                    // (Height's label at the right of its room.)
                    let size = egui::vec2(SIZE_LABEL - ui.spacing().item_spacing.x, 22.0);
                    let at = egui::Layout::right_to_left(egui::Align::Center);
                    ui.allocate_ui_with_layout(size, at, |ui| {
                        ui.set_min_size(size);
                        form::label(ui, words, None);
                    });
                }
                let Some(value) = obj.get_mut(key) else { return };
                if let Some(field) = ui
                    .scope(|ui| {
                        ui.spacing_mut().interact_size.x = SIZE_FIELD;
                        scalar(ui, value)
                    })
                    .inner
                {
                    if key == "width" {
                        field.labelled_by(lead.id);
                    } else {
                        field.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, words)
                        });
                    }
                }
            });
        }
        let more = widgets::icon_only(mg_uikit::Icon::MoreHorizontal, "More")
            .name("More for Width and Height");
        let menu = more.menu(ui, |ui| {
            mg_uikit::menu_rows(ui);
            for (key, words) in [("width", "Width"), ("height", "Height")] {
                mg_uikit::menu_button(ui, words, |ui| {
                    if let Some(value) = obj.get_mut(key) {
                        more_rows(ui, key, ty, value, s, remove);
                    }
                });
            }
        });
        menu.response.on_hover_text("Bind them, or remove one");
    });
    true
}

/// The properties' sections in the order shown: the title, the fields of
/// each, and whether it starts open.
const SECTIONS: &[(&str, &[&str], bool)] = &[
    ("Content", &["title", "id", "label", "value", "elements"], true),
    (
        "Size & Layout",
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
        .check_box(&mut shown, "Title bar")
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
    form::plus("Add Property").menu(ui, |ui| {
        mg_uikit::menu_rows(ui);
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
                && !has_property_default(ty, key)
            {
                return None;
            }
            if SECTIONS.iter().any(|(_, keys, _)| keys.contains(&key.as_str()))
                && (!value.is_null() || has_property_default(ty, key))
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

fn template_for(ty: &str) -> Option<&'static Value> {
    static TEMPLATES: std::sync::OnceLock<std::collections::HashMap<&'static str, Value>> =
        std::sync::OnceLock::new();
    let map = TEMPLATES.get_or_init(|| {
        let mut m = std::collections::HashMap::with_capacity(mg_nui::ELEMENTS.len() + 1);
        m.insert("window", mg_nui::window());
        for &el in mg_nui::ELEMENTS {
            m.insert(el, mg_nui::template(el));
        }
        m
    });
    map.get(ty)
}

fn has_property_default(ty: &str, key: &str) -> bool {
    template_for(ty).and_then(|t| t.get(key)).is_some_and(|v| !v.is_null())
        || matches!(
            key,
            "id" | "tooltip"
                | "disabled_tooltip"
                | "font"
                | "width"
                | "height"
                | "margin"
                | "padding"
                | "aspect"
                | "enabled"
                | "visible"
                | "encouraged"
                | "collapsed"
                | "foreground_color"
                | "size_constraint"
                | "edge_constraint"
                | "image_region"
        )
}

/// A missing value must use the property's type, not a generic JSON type menu.
/// Unknown/custom fields remain losslessly editable in the source editor.
fn property_default(ty: &str, key: &str) -> Option<Value> {
    if let Some(value) = template_for(ty).and_then(|t| t.get(key)).filter(|v| !v.is_null()) {
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
        "draw_list" => "Draw Layers".into(),
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
        "settings" => "Window Identity",
        "views" => "Swap Layout Variants",
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

/// A property's row in Properties (the board's form): its label in its
/// column and its field beside it (text, a number, a color, a choice, a
/// check box; the label over the field where the row has not the room), or,
/// for a bound value, an object of several fields or a list, its label over
/// what it holds. Its More (`…`) at the row's end: bind it, use a
/// talk-table string, remove it.
fn property(
    ui: &mut Ui,
    key: &str,
    ty: &str,
    value: &mut Value,
    s: &mut Settings,
    remove: &mut Option<String>,
) {
    let label = display_label(key, ty);
    let tip = property_tip(key);
    let bound = value.get("bind").is_some();
    // A bound property: its bind's chip beside its label (M4's U19, the NUI
    // board's `Nui_right.png`); a click shows it in Property Binding, where
    // its Source changes the bind or Constant takes it off, its initial
    // value and its formatting are.
    if let Some(name) = value.get("bind").and_then(Value::as_str) {
        let name = name.to_owned();
        form::row(ui, &label, tip, |ui, lead| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                more(ui, key, ty, &label, value, s, remove);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let tip = match s.bindings.contains_key(&name) {
                        true => "Its bind: click to change it or take it off in Property Binding",
                        false => {
                            "There is no such bind: click to choose another in Property Binding"
                        }
                    };
                    let chip = form::bind_chip(ui, &name).labelled_by(lead.id).on_hover_text(tip);
                    if chip.clicked() {
                        let asked = form::chip_asked(ui.ctx());
                        ui.ctx().data_mut(|d| d.insert_temp(asked, key.to_owned()));
                        ui.ctx().request_repaint();
                    }
                });
            });
        });
        return;
    }
    if !bound && inline(key, value) {
        form::row(ui, &label, tip, |ui, lead| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                more(ui, key, ty, &label, value, s, remove);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    inline_field(ui, key, ty, value, lead);
                });
            });
        });
        return;
    }
    let heading = ui
        .horizontal(|ui| {
            let heading = form::label(ui, &label, tip);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                more(ui, key, ty, &label, value, s, remove);
            });
            heading
        })
        .inner;
    let bind = value.get("bind").and_then(Value::as_str).map(str::to_owned);
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
    } else if key == "elements"
        && let Some(elements) = value.as_array_mut()
    {
        fields::options(ui, elements, ty == "combo");
    } else if key == "size_constraint" && value.is_object() {
        for (field, words) in [
            ("x", "Minimum width"),
            ("y", "Minimum height"),
            ("w", "Maximum width"),
            ("h", "Maximum height"),
        ] {
            form::row(ui, words, None, |ui, lead| {
                if let Some(field) = scalar(ui, &mut value[field]) {
                    field.labelled_by(lead.id);
                }
            });
        }
    } else if inline(key, value) {
        inline_field(ui, key, ty, value, &heading);
    } else if let Some(field) = scalar(ui, value) {
        field.labelled_by(heading.id);
    }
    ui.add_space(form::ROW_GAP);
}

/// What a property's label says under the pointer, where its row says more
/// than its name (the board's descriptions as tooltips, one short sentence
/// each).
fn property_tip(key: &str) -> Option<&'static str> {
    Some(match key {
        "disabled_tooltip" => "Shown on hover while the control is disabled, as why it is off",
        "size_constraint" => "The window's least and greatest size: 0 leaves a limit open",
        "edge_constraint" => "How far the window keeps from the screen's edges",
        "geometry" => "Where the window opens and its size (-1 centers it)",
        _ => return None,
    })
}

/// Whether a property's field stands beside its label: text, a talk-table
/// string, a number, a color, a choice by name, a check box, or none set.
fn inline(key: &str, value: &Value) -> bool {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null => key != "elements",
        Value::Object(o) => {
            o.contains_key("strref")
                || (o.len() == 4
                    && ["r", "g", "b", "a"].iter().all(|k| o.get(*k).is_some_and(Value::is_number)))
        }
        Value::Array(_) => false,
    }
}

/// A property's field beside its label (`lead`, which names it).
fn inline_field(ui: &mut Ui, key: &str, ty: &str, value: &mut Value, lead: &egui::Response) {
    if let Some(name) = value.get("bind").and_then(Value::as_str) {
        // (Bound this frame from its More: shown as a bound one is.)
        let words = format!("Dynamic · {name}");
        ui.label(egui::RichText::new(words).color(ui.visuals().selection.bg_fill));
        return;
    }
    if let Value::Bool(flag) = value {
        ui.check_box(flag, "").labelled_by(lead.id);
    } else if let Value::String(text) = value {
        let picture = matches!((ty, key), ("image", "value") | ("button_image", "label"));
        let hint = if picture { "Picture name" } else { "" };
        if let Some(field) = fields::hinted(ui, text, f32::INFINITY, hint) {
            field.labelled_by(lead.id);
        }
    } else if let Some(id) = value.get_mut("strref") {
        form::label(ui, "StrRef", None);
        if let Some(field) = scalar(ui, id) {
            field.labelled_by(lead.id);
        }
    } else if value.is_null() {
        if let Some(default) = property_default(ty, key) {
            if ui.button("Set Value…").clicked() {
                *value = default;
            }
        } else {
            ui.weak("Not set, or a value of another kind: edit it in Advanced › JUI Source.");
        }
    } else if let Some(names) = named_choices(key)
        && let Some(n) = value.as_i64()
    {
        // The NUI_* constants of nw_inc_nui, by name.
        let mut n = n;
        let all: Vec<i64> = (0..names.len() as i64).collect();
        widgets::dropdown(key)
            .selected_text(names.get(n as usize).copied().unwrap_or("Other"))
            .width(ui.available_width().max(60.0))
            .show(ui, &mut n, &all, |i| names[*i as usize])
            .labelled_by(lead.id);
        *value = json!(n);
    } else if let Some(field) = scalar(ui, value) {
        field.labelled_by(lead.id);
    }
}

/// A property's More (`…`, named "More for" its label): Make Dynamic
/// (Bind) or Use a Constant Value, Use Existing Bind, Use TLK String
/// (StrRef) or Use Typed Text, Remove Property.
fn more(
    ui: &mut Ui,
    key: &str,
    ty: &str,
    label: &str,
    value: &mut Value,
    s: &mut Settings,
    remove: &mut Option<String>,
) {
    // (Named with its property: each row has one.)
    let more = widgets::icon_only(mg_uikit::Icon::MoreHorizontal, "More")
        .name(format!("More for {label}"));
    let menu = more.menu(ui, |ui| {
        mg_uikit::menu_rows(ui);
        more_rows(ui, key, ty, value, s, remove);
    });
    menu.response.on_hover_text("Bind it, use a talk-table string, or remove it");
}

/// What a property's More offers, as its menu's rows.
fn more_rows(
    ui: &mut Ui,
    key: &str,
    ty: &str,
    value: &mut Value,
    s: &mut Settings,
    remove: &mut Option<String>,
) {
    let can_bind = bindable(ty, key);
    let bind = value.get("bind").and_then(Value::as_str).map(str::to_owned);
    if let Some(name) = &bind {
        if ui.button("Use a Constant Value").clicked() {
            *value = s.bindings.get(name).map_or(Value::Null, |b| b.value.clone());
            ui.close();
        }
    } else if can_bind && ui.button("Make Dynamic (Bind)").clicked() {
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
        mg_uikit::menu_button(ui, "Use Existing Bind", |ui| {
            for name in s.bindings.keys() {
                if ui.selectable_label(bind.as_ref() == Some(name), name).clicked() {
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
        || (key == "label" && matches!(ty, "button" | "button_select" | "check" | "textedit"))
        || (key == "text" && ty == "draw")
        || (key == "legend" && ty == "chart_slot");
    if bind.is_none() && text_property {
        if value.get("strref").is_some() {
            if ui.button("Use Typed Text").clicked() {
                *value = json!("");
                ui.close();
            }
        } else if value.is_string() && ui.button("Use TLK String (StrRef)").clicked() {
            *value = json!({"strref":0});
            ui.close();
        }
    }
    if !matches!(key, "geometry" | "title") && ui.button("Remove Property").clicked() {
        *remove = Some(key.into());
        ui.close();
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
        Value::Bool(b) => return Some(ui.check_box(b, "On")),
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
        h.get_by_label("Add Property").click();
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
        let mut h = Harness::builder().build_ui(|ui| {
            draw::editor(ui, &mut spacer, &mut Settings::default(), &assets);
        });
        h.run();
        h.get_by_label("Draw Layers").click();
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
            assert!(h.query_by_label("Advanced Properties").is_none(), "standard template: {ty}");
            if let Some(header) = h.query_by_label("Advanced Properties") {
                header.click();
            }
            h.run();
            assert!(h.query_by_label("Set Value…").is_none(), "{ty}");
        }
        let mut node = mg_nui::template("button");
        node["label"] = Value::Null;
        let mut h = Harness::builder().build_ui(|ui| {
            properties(ui, &mut node, &mut Settings::default());
        });
        h.run();
        assert!(h.query_by_label("Advanced Properties").is_none());
        h.get_by_label("Set Value…").click();
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
            assert!(h.query_by_label("Advanced Properties").is_none());
            assert!(h.query_by_label("Size & Layout").is_some());
            if before["type"] == "row" {
                assert_eq!(h.query_all_by_label("Set Size").count(), 2);
            }
            drop(h);
            assert_eq!(node, before, "opening properties must not mutate the document");
            node["vendor_extension"] = json!({"payload":[1,2,3]});
            let before = node.clone();
            let mut h = Harness::builder()
                .build_ui(|ui| properties(ui, &mut node, &mut Settings::default()));
            h.run();
            assert!(h.query_by_label("Advanced Properties").is_some());
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
        h.get_by_label_contains("More for ").click();
        h.run();
        h.get_by_label("Use TLK String (StrRef)").click();
        h.run();
        assert_eq!(h.state().0, json!({"strref":0}));
        h.get_by_label_contains("More for ").click();
        h.run();
        h.get_by_label("Make Dynamic (Bind)").click();
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
            h.get_by_label_contains("More for ").click();
            h.run();
            h.query_by_label_contains("Use Existing Bind")
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
        h.get_by_label_contains("More for ").click();
        h.run();
        assert!(
            h.query_by_label_contains("Use Existing Bind").is_none(),
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

    #[test]
    fn check_rejects_non_utf8_and_recovers_on_valid_bytes() {
        let mut module = mg_module::Module::new();
        let key = ResKey::parse("badutf8", ResType::JUI).unwrap();
        let valid = serde_json::to_vec(&mg_nui::window()).unwrap();
        module.set(key, valid.clone());
        let mut state = State::default();
        assert!(check(&module, key, &mut state).is_ok());
        assert!(state.source.is_some());
        assert!(state.work.is_some());

        module.set(key, vec![0xff, 0xfe]);
        assert!(check(&module, key, &mut state).is_err());
        assert!(state.source.is_none());
        assert!(state.work.is_none());
        assert!(state.checks.is_none());

        module.set(key, valid);
        let txt = ResKey::new(key.resref, ResType::TXT);
        module.set(txt, vec![0x80]);
        assert!(check(&module, key, &mut state).is_err());
        assert!(state.source.is_none());
        assert!(state.work.is_none());

        module.set(txt, mg_nui::Settings::default().bytes());
        assert!(check(&module, key, &mut state).is_ok());
        assert!(state.source.is_some());
        assert!(state.work.is_some());
    }
}
