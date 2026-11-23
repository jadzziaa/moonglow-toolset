//! The NUI designer's panels (M4's U18, the NUI board): **Components** over
//! **Properties** over **Bindings** in the Right Dock (a floating
//! designer's window's right pane), and **Layers** in the Outliner (its
//! window's left pane, or the fold strip). Each is the designer in front's
//! (`editor`, the shell's), drawn before its editor: each begins the
//! designer's frame or goes on with it ([`super::begin`]) and edits its one
//! copy of the window ([`super::Work`]); its editor sets what they changed
//! as the frame ends ([`super::end`]), so that its Cancel sets it back.
//!
//! What Layers and Components choose is done as the frame ends
//! ([`super::Picked`]): the editor drawn after them shows the window, and
//! the variant in its place, the frame began with. (Typed text keeps to its
//! control otherwise: egui ends a field's focus on the press, a row or a
//! button acts on the release.)
//!
//! They are brought whenever the designer is in front, on every page and in
//! every Layout (the coordinator's ruling at U18's review: the board draws
//! the docks beside the source and the design both; the editor keeps its
//! width as pages switch). On a page with no canvas, Components adds to the
//! control chosen, as a click there always did.
use super::*;
use crate::palette_view::{Keys, give_keys, keys_are};

/// The designer `editor` is, where it is one (or its source group, a part
/// of it: M4's U19).
pub(crate) fn nui_of(editor: Option<&Tab>) -> Option<ResKey> {
    match editor? {
        Tab::Nui(key) | Tab::NuiSource(key) => Some(*key),
        _ => None,
    }
}

/// What a panel's header says after its title: Properties' control chosen
/// ("Button · Send", "Window"), Bindings' count ("3 binds").
pub(crate) fn detail(app: &Moonglow, editor: Option<&Tab>, panel: crate::Panel) -> Option<String> {
    let state = app.nui_views.get(&nui_of(editor)?)?;
    let work = state.work.as_ref()?;
    match panel {
        crate::Panel::Properties => {
            // (The window's own, where a variant is edited in its place:
            // the selection is the variant's.)
            let node = work.window.pointer(&state.selected)?;
            Some(match state.selected.is_empty() {
                true => "Window".to_string(),
                false if layouts::is_swap(&work.settings, node) => {
                    format!("Swap layout · {}", node["id"].as_str().unwrap_or_default())
                }
                false => design::node_name(node),
            })
        }
        crate::Panel::Bindings => Some(count(work.settings.bindings.len(), "bind")),
        _ => None,
    }
}

/// A panel of designer `editor`'s begins: its state taken out (put back by
/// [`close`]), the frame begun or gone on with, the keys' outline drawn, a
/// press in it giving it the keys (`keys`); and whether F6 just gave it
/// them (its first field takes the focus).
fn open(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>, keys: Keys) -> Option<Opened> {
    let key = nui_of(editor)?;
    let ws = app.ws.as_ref()?;
    let mut state = app.nui_views.remove(&key).unwrap_or_default();
    if let Err(why) = begin(&ws.module, ui.ctx(), key, &mut state) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.weak(why);
        });
        app.nui_views.insert(key, state);
        return None;
    }
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        give_keys(ui.ctx(), keys);
    }
    // (Properties': as the area's, `Moonglow::props_focus`.)
    let focus = match keys {
        Keys::Properties => std::mem::take(&mut app.props_focus),
        keys => app.dialog_panel_focus.take_if(|k| *k == keys).is_some(),
    };
    Some(Opened { key, state, focus })
}

/// A panel of a designer's, open ([`open`]).
struct Opened {
    key: ResKey,
    state: State,
    focus: bool,
}

/// The panel ends: its designer's state put back.
fn close(app: &mut Moonglow, opened: Opened) {
    app.nui_views.insert(opened.key, opened.state);
}

/// The room round a panel's body (the board's `.content`: 8 at its left
/// and top, 4 at its right before the scroll bar, 8 at its foot).
const BODY: egui::Margin = egui::Margin { left: 8, right: 10, top: 8, bottom: 8 };

// The palette's components live in `super::design::COMPONENTS` so panel chips
// and layer/check titles share a single catalog.

/// The Components panel (the board's top section): Find a control…, then
/// each group's heading and its components as buttons that wrap. A click
/// adds one to the control chosen (its layout, or the nearest one holding
/// it), as the frame ends; a drag puts one where it is dropped in Layers or
/// on the canvas. Off, with why, where nothing chosen can hold one.
pub(crate) fn components(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(mut o) = open(app, ui, editor, Keys::NuiComponents) else { return };
    let state = &mut o.state;
    let room = ui.max_rect();
    // (The search field in the panel's bar, as the board draws it.)
    egui::Frame::new().inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 4 }).show(
        ui,
        |ui| {
            let field = ui.id().with(("nui-palette-filter", o.key));
            let shown = widgets::SearchField::new(field, &mut state.search)
                .name("Find a control")
                .hint("Find a control…")
                .find_over()
                .show(ui);
            if o.focus {
                shown.response.request_focus();
            }
        },
    );
    let can_add = state.resize.is_none()
        && state
            .work
            .as_ref()
            .is_some_and(|w| design::insertion_parent(&w.window, &state.selected).is_some());
    let query = mg_core::text::Needle::new(&state.search);
    let scroll = egui::ScrollArea::vertical().id_salt(("nui-components", o.key)).auto_shrink(false);
    scroll.show(ui, |ui| {
        egui::Frame::new()
            .inner_margin(egui::Margin { left: 8, right: 10, top: 0, bottom: 8 })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                let t = mg_uikit::tokens(ui.ctx());
                for (group, items) in super::design::COMPONENTS {
                    let found: Vec<_> = (items.iter())
                        .filter(|c| query.is_empty() || query.finds_folded(c.search))
                        .collect();
                    if found.is_empty() {
                        continue;
                    }
                    let small = egui::TextStyle::Small.resolve(ui.style()).size;
                    ui.label(egui::RichText::new(*group).size(small).color(t.text_3));
                    ui.horizontal_wrapped(|ui| {
                        for c in found {
                            let button = mg_uikit::KitButton::new(None, c.title);
                            let r = ui.add_enabled(can_add, button);
                            let r = r.interact(egui::Sense::click_and_drag());
                            let summary = c.summary;
                            let r = r.on_hover_ui(move |ui| {
                                ui.label(format!(
                                    "{summary}. Click to add to the selected layout, or drag into the \
                                     Outliner or onto the canvas."
                                ));
                            });
                            if can_add {
                                r.dnd_set_drag_payload(design::Insert(c.ty));
                            }
                            if r.clicked() {
                                state.picked.insert = Some(c.ty);
                            }
                        }
                    });
                }
                if !can_add {
                    ui.weak("Select a row, column or list to add controls.");
                }
            });
    });
    let _ = room;
    close(app, o);
}

/// The Properties panel (the board's middle section): the chosen control's
/// properties (or the window's), as the editor's pane had them: its
/// binding, Content, Size & Layout, Appearance, Behavior, + Add property,
/// its list's rows, images, chart, Events, draw layers and swap layout
/// variants, on one scroll. Element actions are its header's ([`actions`]).
/// Its fields' ids are the control's, so that what is typed for one is never
/// shown or set in another's.
pub(crate) fn properties(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>, titled: bool) {
    // (Sharing a tab bar, under 1400: its detail and Element Actions at its
    // own top, as the header has them.)
    if !titled {
        let words = detail(app, editor, crate::Panel::Properties).unwrap_or_default();
        egui::Frame::new()
            .inner_margin(egui::Margin { left: 8, right: 4, top: 2, bottom: 0 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.weak(words);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        actions(app, ui, editor);
                    });
                });
            });
    }
    let Some(mut o) = open(app, ui, editor, Keys::Properties) else { return };
    // (Element actions chosen on the panel's header this frame.)
    if let Some(action) =
        ui.ctx().data_mut(|d| d.remove_temp::<Act>(act_id(o.key))).filter(|a| *a != Act::None)
    {
        o.state.picked.act = Some(action);
    }
    let Some(mut work) = o.state.work.take() else {
        ui.add_space(8.0);
        ui.weak(REPAIR);
        close(app, o);
        return;
    };
    let scroll = egui::ScrollArea::vertical().id_salt(("nui-properties", o.key)).auto_shrink(false);
    let focus = o.focus;
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(BODY).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let assets = &mut app.nui_assets;
            if focus {
                // (F6: its first field takes the focus, as it is drawn.)
                ui.memory_mut(|m| m.move_focus(egui::FocusDirection::Next));
            }
            // (What it chooses, a swap layout's variant to edit, its first
            // contents, is done as the frame ends, as Layers' choice is:
            // the editor drawn after it shows the window it began with.)
            let before = Chosen::of(&o.state);
            design::inspector(
                ui,
                &mut work.window,
                &mut work.settings,
                &mut o.state,
                assets,
                o.key,
            );
            let now = Chosen::of(&o.state);
            if now != before {
                o.state.picked.chosen = Some(now);
                before.put(&mut o.state);
            }
        });
    });
    o.state.work = Some(work);
    close(app, o);
}

/// Element actions on the Properties panel's header (the board's `…`):
/// Move Up, Move Down, Duplicate, Delete, Wrap in a column, row or group,
/// Copy, Cut and Paste, each as its keys are, for the control chosen. What
/// is chosen is done by the panel's body ([`properties`]) as the frame ends.
pub(crate) fn actions(app: &Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(key) = nui_of(editor) else { return };
    let Some(state) = app.nui_views.get(&key) else { return };
    let Some(work) = state.work.as_ref() else { return };
    let more = widgets::icon_only(mg_uikit::Icon::MoreHorizontal, ELEMENT_ACTIONS);
    let menu = more.menu(ui, |ui| {
        mg_uikit::menu_rows(ui);
        if let Some(act) = act_rows(ui, &work.window, state, &app.keymap) {
            ui.ctx().data_mut(|d| d.insert_temp(act_id(key), act));
            ui.close();
        }
    });
    menu.response.on_hover_text("Element Actions: move, duplicate, wrap, delete");
}

/// Element actions' name (the Properties header's `…`).
pub const ELEMENT_ACTIONS: &str = "Element Actions";
/// The same actions on the Outliner's header (its `…`).
pub const OUTLINER_ACTIONS: &str = "Outliner Actions";

/// Where an Element action chosen on a header waits for its panel's body.
fn act_id(key: ResKey) -> egui::Id {
    egui::Id::new(("nui-element-action", key))
}

/// An Element action, done as the frame ends.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(super) enum Act {
    #[default]
    None,
    Op(Op),
    Wrap(&'static str),
    /// Copy (0), Cut (1), Paste (2).
    Clip(usize),
}

/// Element actions' rows, each enabled where it can be done to the control
/// chosen in `v`; the one clicked.
pub(super) fn act_rows(
    ui: &mut Ui,
    v: &Value,
    state: &State,
    keymap: &crate::keys::Keymap,
) -> Option<Act> {
    use crate::keys::Cmd;
    if state.resize.is_some() {
        mg_uikit::menu_note(ui, "Finish resizing to change elements.");
        return None;
    }
    let mut chosen = None;
    for (label, op, cmd) in [
        ("Move Up", Op::MoveUp, Cmd::NuiMoveUp),
        ("Move Down", Op::MoveDown, Cmd::NuiMoveDown),
        ("Duplicate Element", Op::Duplicate, Cmd::NuiDuplicate),
        ("Delete Element", Op::Delete, Cmd::NuiDelete),
    ] {
        let keys = keymap.label(cmd, ui.ctx());
        let enabled = shortcuts::can_edit(v, &state.selected, op);
        let row = ui.add_enabled_ui(enabled, |ui| {
            mg_uikit::menu_row_with_keys(ui, egui::Button::new(label), &keys)
        });
        if row.inner.clicked() {
            chosen = Some(Act::Op(op));
        }
    }
    for (label, ty) in
        [("Wrap in Column", "col"), ("Wrap in Row", "row"), ("Wrap in Group", "group")]
    {
        if ui.add_enabled(!state.selected.is_empty(), egui::Button::new(label)).clicked() {
            chosen = Some(Act::Wrap(ty));
        }
    }
    mg_uikit::menu_separator(ui);
    for (label, key, op, enabled) in [
        ("Copy Element", egui::Key::C, 0, !state.selected.is_empty()),
        ("Cut Element", egui::Key::X, 1, shortcuts::can_edit(v, &state.selected, Op::Delete)),
        (
            "Paste Element",
            egui::Key::V,
            2,
            design::insertion_parent(v, &state.selected).is_some()
                && shortcuts::has_clipboard(ui.ctx()),
        ),
    ] {
        let shortcut =
            ui.ctx().format_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key));
        let row = ui.add_enabled_ui(enabled, |ui| {
            mg_uikit::menu_row_with_keys(ui, egui::Button::new(label), &shortcut)
        });
        if row.inner.clicked() {
            chosen = Some(Act::Clip(op));
        }
    }
    chosen
}

/// An Element action done to `v` and `s` for the control chosen.
pub(super) fn act(
    ctx: &egui::Context,
    act: Act,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
) {
    match act {
        Act::Op(Op::Delete) => structure::delete_selected(v, state),
        Act::Op(op) => rearrange(v, &mut state.selected, op),
        Act::Wrap(ty) => structure::wrap(v, &state.selected, ty),
        Act::Clip(op) => shortcuts::clipboard(ctx, v, s, &mut state.selected, op),
        Act::None => {}
    }
}

/// Where the Bindings panel's header's plus asks for Create Bind's dialog
/// (M4's U19, the board's `+` on the Bindings header), for designer `key`.
pub(crate) fn create_asked(key: ResKey) -> egui::Id {
    egui::Id::new(("nui-create-bind", key))
}

/// The Bindings panel's header's plus (the board's): Create Bind.
pub(crate) fn bindings_plus(ui: &mut Ui, editor: Option<&Tab>) {
    let Some(key) = nui_of(editor) else { return };
    let plus = widgets::icon_only(mg_uikit::Icon::Plus, "New Bind");
    if ui.add(plus).on_hover_text("New Bind: a named value the event script can change").clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(create_asked(key), true));
    }
}

/// The Bindings panel (the board's lowest section, M4's U19): the window's
/// binds as a table (Name, Initial Value, Watch) with Create Bind on its
/// header's plus, the window's settings under it, and the bind chosen's
/// form; on the Bindings page, whose body is every bind's form, the table
/// and the settings alone (one of each field).
pub(crate) fn bindings(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>, titled: bool) {
    // (Sharing a tab bar, under 1400: its count and its plus at its own
    // top, as its header has them.)
    if !titled {
        let words = detail(app, editor, crate::Panel::Bindings).unwrap_or_default();
        egui::Frame::new()
            .inner_margin(egui::Margin { left: 8, right: 4, top: 2, bottom: 0 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.weak(words);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        bindings_plus(ui, editor);
                    });
                });
            });
    }
    let Some(mut o) = open(app, ui, editor, Keys::NuiBindings) else { return };
    let page = o.state.page == super::Page::Bindings;
    let create = ui.ctx().data_mut(|d| d.remove_temp::<bool>(create_asked(o.key))).is_some();
    let Some(mut work) = o.state.work.take() else {
        ui.add_space(8.0);
        ui.weak(REPAIR);
        close(app, o);
        return;
    };
    if create {
        work.verified_clean = false;
    }
    let focus = o.focus;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("nui-bindings-panel", o.key)).auto_shrink(false);
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(BODY).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if focus {
                ui.memory_mut(|m| m.move_focus(egui::FocusDirection::Next));
            }
            // (The window as it is kept, the variant edited back in the
            // settings: its binds' uses and renames reach both, as the page's.)
            super::main_root(&mut work, &mut o.state, true);
            let of = work.of.clone();
            design::bindings_table(
                ui,
                (&mut work.settings, &mut work.window, &of),
                &mut o.state,
                page,
                create,
            );
            super::main_root(&mut work, &mut o.state, false);
        });
    });
    o.state.work = Some(work);
    close(app, o);
}

/// One of Layers' rows, as they are listed ([`rows`]).
#[derive(Clone, Debug)]
enum Row {
    /// A control (or the window, path ""): its path, depth, words, whether
    /// it holds others (an arrow), whether it is the window's own tree's
    /// (not a variant's being edited) and its icon.
    Control {
        path: String,
        depth: usize,
        words: String,
        /// Weak words after them: the window's title on its row, a
        /// control's ID, else the bind its value takes (M4's U19, the
        /// board's "Button · Send  send", "Label  fee_text").
        detail: Option<String>,
        /// What a check found of it, the first (errors first): whether it
        /// is an error, and its message (the board's warning mark).
        mark: Option<(bool, String)>,
        branch: bool,
        main: bool,
        icon: mg_uikit::Icon,
        tip: String,
    },
    /// One of a control's draw layers.
    Draw { path: String, index: usize, depth: usize, words: String, main: bool },
    /// A swap layout's first contents (choosing it leaves the variant).
    Initial { path: String, depth: usize },
    /// One of a swap layout's variants: its group and name.
    Variant { group: String, name: String, depth: usize, active: bool },
}

/// Whether rows belonging to `main` are currently editable (`main` when editing
/// the main window, non-`main` when editing a swap-layout variant).
fn is_editable_tree(main: bool, edit_view: Option<&str>) -> bool {
    main == edit_view.is_none()
}

/// Index in `rows` of the currently selected control row in the active tree.
fn chosen_control_index(rows: &[Row], selected: &str, edit_view: Option<&str>) -> Option<usize> {
    rows.iter().position(|r| {
        matches!(
            r,
            Row::Control { path, main, .. } if *path == selected && is_editable_tree(*main, edit_view)
        )
    })
}

type LayerRowsKey = (
    Option<[Held; 2]>,
    std::collections::BTreeSet<String>,
    String,
    Option<String>,
    Option<std::rc::Rc<[mg_nui::Diagnostic]>>,
);

/// Layers' rows, made again only where what they are made of changed
/// ([`LayerRows::of`]): a window of hundreds of controls is not walked, and
/// each row's words not written, every frame.
pub(super) struct LayerRows {
    of: LayerRowsKey,
    rows: std::sync::Arc<[Row]>,
}

/// The rows of Layers for `state` (its window, its folds, what is typed in
/// its Find, the variant edited).
fn rows(state: &mut State) -> std::sync::Arc<[Row]> {
    // (The checks' findings, by where they are kept: new findings mark
    // the rows anew.)
    let findings = state.checks.as_ref().map(|c| c.findings.clone());
    let of = (
        state.work.as_ref().map(|w| w.of.clone()),
        state.collapsed.clone(),
        state.layer_search.clone(),
        state.edit_view.clone(),
        findings.clone(),
    );
    if let Some(kept) = &state.layer_rows
        && kept.of.1 == of.1
        && kept.of.2 == of.2
        && kept.of.3 == of.3
        && match (&kept.of.4, &of.4) {
            (Some(a), Some(b)) => std::rc::Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
        && match (&kept.of.0, &of.0) {
            (Some(a), Some(b)) => same(a, b),
            (None, None) => true,
            _ => false,
        }
    {
        return kept.rows.clone();
    }
    let mut out = Vec::new();
    let search = mg_core::text::Needle::new(&state.layer_search);
    // Each control a check names, its first finding (errors first).
    let mut marks: std::collections::BTreeMap<String, (bool, String)> = Default::default();
    if let (Some(findings), Some(doc)) =
        (&findings, state.source.as_ref().and_then(|s| s.window.as_ref().ok()))
    {
        for d in findings.iter() {
            let error = d.severity == Severity::Error;
            let Some(path) = describe_path(doc, &d.path).0 else { continue };
            let held = marks.entry(path).or_insert((error, d.message.clone()));
            if error && !held.0 {
                *held = (true, d.message.clone());
            }
        }
    }
    if let Some(work) = &state.work {
        // While a variant is edited, the tree is still the window's own.
        let main = match &state.swapped {
            Some(_) => state.main_doc.as_ref().unwrap_or(&work.window),
            None => &work.window,
        };
        let tree = Tree {
            s: &work.settings,
            marks: &marks,
            main: true,
            active_root: &work.window["root"],
            search: &search,
            collapsed: &state.collapsed,
            edit_view: state.edit_view.as_deref(),
        };
        walk(main, "", 0, tree, &mut out);
    }
    let rows: std::sync::Arc<[Row]> = out.into();
    state.layer_rows = Some(LayerRows { of, rows: rows.clone() });
    rows
}

/// What the rows of one tree share.
#[derive(Clone, Copy)]
struct Tree<'a> {
    s: &'a Settings,
    /// The controls the checks name, by path ([`Row::Control`]'s mark).
    marks: &'a std::collections::BTreeMap<String, (bool, String)>,
    /// The window's own tree, not that of a variant being edited under its
    /// swap layout.
    main: bool,
    /// What is being edited: the window, or the variant in its place.
    active_root: &'a Value,
    search: &'a mg_core::text::Needle,
    collapsed: &'a std::collections::BTreeSet<String>,
    edit_view: Option<&'a str>,
}

/// A control's rows and those under it: its own, its draw layers, then its
/// children (a swap layout: its first contents and its variants).
fn walk(node: &Value, path: &str, depth: usize, tree: Tree<'_>, out: &mut Vec<Row>) {
    if depth > 64 {
        return;
    }
    // (With nothing typed every control is listed.)
    if !tree.search.is_empty() && !design::holds(node, tree.search) {
        return;
    }
    let swap = layouts::is_swap(tree.s, node);
    let words = if path.is_empty() {
        "Window".into()
    } else if swap {
        format!("Swap layout · {}", node["id"].as_str().unwrap_or_default())
    } else {
        design::node_name(node)
    };
    let branch = path.is_empty() || node["children"].is_array() || node["row_template"].is_array();
    let tip = match node["id"].as_str() {
        Some(id) => format!("{words}\nElement ID: {id}"),
        None => words.clone(),
    };
    let ty = if path.is_empty() { "window" } else { node["type"].as_str().unwrap_or_default() };
    // (The window's row says its title after its name, the board's
    // "Window  The Town Crier"; a control its ID, else the bind its value
    // takes, M4's U19.)
    let detail = match path.is_empty() {
        true => node["title"].as_str().filter(|t| !t.is_empty()).map(str::to_owned),
        false => (node["id"].as_str())
            .or_else(|| node["value"]["bind"].as_str())
            .filter(|t| !t.is_empty())
            .map(str::to_owned),
    };
    // (Marked where the window's own tree is: a variant edited in its
    // place is checked where it is kept.)
    let mark = tree.main.then(|| tree.marks.get(path).cloned()).flatten();
    out.push(Row::Control {
        path: path.into(),
        depth,
        words,
        detail,
        mark,
        branch,
        main: tree.main,
        icon: kind_icon(ty),
        tip,
    });
    if tree.collapsed.contains(path) && tree.search.is_empty() {
        return;
    }
    if let Some(items) = node["draw_list"].as_array() {
        for (i, item) in items.iter().enumerate() {
            let kind =
                draw::KINDS.get(item["type"].as_u64().unwrap_or(99) as usize).unwrap_or(&"Unknown");
            out.push(Row::Draw {
                path: path.into(),
                index: i,
                depth: depth + 1,
                words: format!("{kind} {}", i + 1),
                main: tree.main,
            });
        }
    }
    if path.is_empty() {
        walk(&node["root"], "/root", depth + 1, tree, out);
    }
    if node["type"] == "group" && swap {
        let group = node["id"].as_str().unwrap_or_default();
        out.push(Row::Initial { path: path.into(), depth: depth + 1 });
        if let Some(children) = node["children"].as_array() {
            for (i, child) in children.iter().enumerate() {
                walk(child, &format!("{path}/children/{i}"), depth + 2, tree, out);
            }
        }
        for name in layouts::variants(tree.s, group) {
            let active = tree.edit_view == Some(name.as_str());
            let held = active && layouts::target_for(tree.s, &name).as_deref() == Some(group);
            out.push(Row::Variant { group: group.into(), name, depth: depth + 1, active });
            if held {
                walk(tree.active_root, "/root", depth + 2, Tree { main: false, ..tree }, out);
            }
        }
        return;
    }
    if let Some(children) = node["children"].as_array() {
        for (i, child) in children.iter().enumerate() {
            walk(child, &format!("{path}/children/{i}"), depth + 1, tree, out);
        }
    }
    if let Some(cells) = node["row_template"].as_array() {
        for (i, cell) in cells.iter().enumerate() {
            walk(&cell[0], &format!("{path}/row_template/{i}/0"), depth + 1, tree, out);
        }
    }
}

/// A control's icon in Layers, by its type (the board's: the window's,
/// a column's and a row's, a label's tag, a text input's pencil, a
/// button's).
fn kind_icon(ty: &str) -> mg_uikit::Icon {
    use mg_uikit::Icon;
    match ty {
        "window" => Icon::Window,
        "col" => Icon::Column,
        "row" => Icon::Row,
        "group" => Icon::Frame,
        "spacer" => Icon::Minus,
        "label" | "text" => Icon::Tag,
        "textedit" => Icon::Edit,
        "button" | "button_select" => Icon::PlayCircle,
        "button_image" | "image" => Icon::Texture,
        "check" => Icon::Check,
        "combo" | "options" | "tabbar" => Icon::List,
        "slider" | "sliderf" | "progress" => Icon::Timeline,
        "color_picker" => Icon::Swatch,
        "list" => Icon::List,
        "chart" => Icon::Statistics,
        _ => Icon::Nui,
    }
}

/// Whether designer `key`'s Layers have the keys (`Keys::Marks`).
pub(crate) fn layers_have_keys(app: &Moonglow, ctx: &egui::Context, key: ResKey) -> bool {
    keys_are(ctx, Keys::Marks) && app.outliner.marks_keys == Some(crate::outliner::Owner::Nui(key))
}

/// The indent of a row a level deeper (the module tree's).
const INDENT: f32 = 14.0;

/// Designer `key`'s Layers, in the Outliner (`window`: its window's left
/// pane): the header, "Outliner" and the window's name, with Expand All,
/// Collapse All and Element Actions at its end; Find a layer or ID…; and
/// the window as a tree, each control a row with its kind's icon, those
/// holding others an arrow that folds them, each control's draw layers
/// and a swap layout's first contents and variants under it; only the rows
/// in sight laid out. A click chooses a control on the canvas (Ctrl+click
/// several), a drag moves it before, into or after another, a component
/// dropped on a row is added there; the canvas's choice is shown here and
/// brought into sight. The keys (a click, F6): Up and Down from control to
/// control, Page Up and Down, Home and End, Right and Left unfold and fold
/// (Left on a folded one goes to its parent), Enter folds or unfolds,
/// letters go to a control by its words, Delete deletes the controls
/// chosen, Escape hands them back. What is chosen is done as the frame ends.
pub(crate) fn layers(app: &mut Moonglow, ui: &mut Ui, key: ResKey, window: bool) {
    let ctx = ui.ctx().clone();
    let owner = crate::outliner::Owner::Nui(key);
    let Some(ws) = app.ws.as_ref() else { return };
    let mut state = app.nui_views.remove(&key).unwrap_or_default();
    let header = mg_uikit::shell::PanelHeader::new(crate::outliner::OUTLINER)
        .detail(key.resref.to_string())
        .name(crate::outliner::OUTLINER_HEADER);
    if begin(&ws.module, &ctx, key, &mut state).is_err() {
        header.show(ui, |_| ());
        app.nui_views.insert(key, state);
        return;
    }
    // A new selection (the canvas's, a link's) unfolds the rows over it,
    // and is brought into sight: before Expand All and Collapse All.
    let reveal = state.tree_selection != state.selected;
    if reveal {
        let selected = state.selected.clone();
        state.collapsed.retain(|p| !p.is_empty() && !selected.starts_with(&format!("{p}/")));
        state.tree_selection.clone_from(&state.selected);
    }
    // (A press in Layers gives them the keys, as a row's click does.)
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        app.outliner.marks_keys = Some(owner);
        give_keys(&ctx, Keys::Marks);
    }
    let before = Chosen::of(&state);
    let mut act_chosen = None;
    let (expand, collapse) = header
        .show(ui, |ui| {
            let expand = ui
                .add(
                    mg_uikit::IconButton::new(mg_uikit::Icon::ExpandAll, "Expand All")
                        .icon_only(true),
                )
                .on_hover_text("Show every control in the window")
                .clicked();
            let collapse = ui
                .add(
                    mg_uikit::IconButton::new(mg_uikit::Icon::CollapseAll, "Collapse All")
                        .icon_only(true),
                )
                .on_hover_text("Fold every control that holds others")
                .clicked();
            if let Some(work) = &state.work {
                let more = widgets::icon_only(mg_uikit::Icon::MoreHorizontal, OUTLINER_ACTIONS);
                let menu = more.menu(ui, |ui| {
                    mg_uikit::menu_rows(ui);
                    if let Some(a) = act_rows(ui, &work.window, &state, &app.keymap) {
                        act_chosen = Some(a);
                        ui.close();
                    }
                });
                menu.response.on_hover_text("What can be done to the chosen layers");
            }
            (expand, collapse)
        })
        .inner;
    if let Some(a) = act_chosen {
        state.picked.act = Some(a);
    }
    if expand {
        state.collapsed.clear();
    }
    if collapse && let Some(work) = &state.work {
        state.collapsed.extend(design::tree_paths(&work.window, &Default::default()));
        state.selected.clear();
        // (Its own choice: not a new one to bring into sight.)
        state.tree_selection.clear();
    }
    // (Folded to its header in a dock too short for its rows.)
    if ui.available_height() < mg_uikit::row_height(ui) {
        end_layers(app, key, state, before);
        return;
    }
    egui::Frame::new().inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 }).show(
        ui,
        |ui| {
            let field = ui.id().with(("nui-layers-filter", key));
            widgets::SearchField::new(field, &mut state.layer_search)
                .name("Find a layer or ID")
                .hint("Find a layer or ID…")
                .show(ui);
        },
    );
    if state.selected_many.len() > 1 {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.weak(format!("{} selected · Delete removes them", state.selected_many.len()));
        });
    }
    app.outliner.marks_now.push((owner, window));
    let rows = rows(&mut state);
    let here = layers_have_keys(app, &ctx, key) && crate::tree::keys_free(ui);
    let chosen_at = chosen_control_index(&rows, &state.selected, state.edit_view.as_deref());
    let moved = here && app.outliner.marks_moved == Some(owner);
    if moved {
        app.outliner.marks_moved = None;
    }
    let mut clicked: Option<(usize, bool)> = None;
    let mut folded: Option<String> = None;
    let mut go_to_checks = false;
    let t = mg_uikit::tokens(&ctx);
    let scroll =
        egui::ScrollArea::vertical().id_salt(("nui-layers", key, window)).auto_shrink(false);
    let shown = scroll.show_viewport(ui, |ui, view| {
        mg_uikit::list_rows(ui);
        let pitch = mg_uikit::row_pitch(ui);
        let count = rows.len();
        ui.set_height(2.0 + count as f32 * pitch);
        let top = ui.max_rect().top() + 2.0;
        if (reveal || moved)
            && let Some(at) = chosen_at
        {
            let y = top + at as f32 * pitch;
            let row = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), y..=y + pitch);
            ui.scroll_to_rect(row, None);
        }
        let first = (((view.min.y - 2.0) / pitch).floor().max(0.0) as usize).min(count);
        let last = ((((view.max.y - 2.0) / pitch).ceil().max(0.0) as usize) + 1).min(count);
        let width = ui.max_rect().width();
        for (i, row) in rows.iter().enumerate().take(last).skip(first) {
            let y = top + i as f32 * pitch;
            let line = egui::Rect::from_min_size(
                egui::pos2(ui.max_rect().left(), y),
                egui::vec2(width, pitch),
            );
            let depth = match row {
                Row::Control { depth, .. }
                | Row::Draw { depth, .. }
                | Row::Initial { depth, .. }
                | Row::Variant { depth, .. } => *depth,
            };
            let indent = (depth as f32 * INDENT).min((width - 90.0).max(0.0));
            let place = line.with_min_x(line.left() + indent);
            let mut row_ui =
                ui.new_child(egui::UiBuilder::new().id_salt(("nui-layer", i)).max_rect(place));
            match row {
                Row::Control { path, words, detail, mark, branch, main, icon, tip, .. } => {
                    let editable_tree = is_editable_tree(*main, state.edit_view.as_deref());
                    let marked = editable_tree
                        && (state.selected == *path || state.selected_many.contains(path));
                    let mut tree_row = mg_uikit::TreeRow::new(words.as_str()).icon(*icon).fit();
                    if let Some(detail) = detail {
                        tree_row = tree_row.second(detail.as_str());
                    }
                    // (Room kept at its right for the checks' mark.)
                    if mark.is_some() {
                        tree_row = tree_row.count("\u{2003}").label(words.as_str());
                    }
                    let mut tree_row =
                        tree_row.selected(marked).sense(egui::Sense::click_and_drag());
                    let arrow_place = mg_uikit::tree::arrow_column(&row_ui);
                    if *branch {
                        tree_row = tree_row.arrow_apart();
                    }
                    let r = tree_row.show(&mut row_ui).on_hover_text(tip.as_str());
                    // (Its arrow over the row's arrow column, after it: the
                    // arrow takes the press there.)
                    if *branch {
                        let closed = state.collapsed.contains(path);
                        let (icon, verb) = match closed {
                            true => (mg_uikit::Icon::ChevronRight, "Expand"),
                            false => (mg_uikit::Icon::ChevronDown, "Collapse"),
                        };
                        let arrow =
                            widgets::icon_only(icon, &format!("{verb} {words}")).expanded(!closed);
                        let tip = if closed {
                            "Show the controls in it"
                        } else {
                            "Hide the controls in it"
                        };
                        if row_ui.put(arrow_place, arrow).on_hover_text(tip).clicked() {
                            folded = Some(path.clone());
                        }
                    }
                    if !path.is_empty()
                        && editable_tree
                        && r.drag_started()
                        && let Some(work) = &state.work
                    {
                        let doc = match &state.swapped {
                            Some(_) if *main => state.main_doc.as_ref().unwrap_or(&work.window),
                            _ => &work.window,
                        };
                        if let Some(node) = doc.pointer(path) {
                            egui::DragAndDrop::set_payload(
                                &ctx,
                                structure::Drag { path: path.clone(), node: node.clone() },
                            );
                        }
                    }
                    if editable_tree {
                        structure::drag_target(&mut row_ui, &r, path, &mut state, false);
                        // (The window read where it is, not copied: only a
                        // component dragged over the row reads it.)
                        if let Some(work) = state.work.take() {
                            let root = &work.window["root"];
                            design::palette_target(&mut row_ui, &r, path, root, &mut state);
                            state.work = Some(work);
                        }
                    }
                    if r.clicked() {
                        clicked = Some((i, row_ui.input(|i| i.modifiers.command)));
                    }
                    // The checks' mark (the board's warning at the row's
                    // right): its tip the finding's message, a click going
                    // to Checks, the control chosen.
                    if let Some((error, message)) = mark {
                        let side = mg_uikit::metrics::ICON + 4.0;
                        let at = egui::Rect::from_center_size(
                            egui::pos2(line.right() - side / 2.0 - 4.0, line.center().y),
                            egui::vec2(side, side),
                        );
                        let (icon, color, word) = match error {
                            true => (mg_uikit::Icon::Error, t.error, "Error"),
                            false => (mg_uikit::Icon::Warning, t.warn, "Warning"),
                        };
                        let resp = row_ui.interact(
                            at,
                            row_ui.id().with("check-mark"),
                            egui::Sense::click(),
                        );
                        icon.paint(row_ui.painter(), at.shrink(2.0), color);
                        resp.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("{word}: {message}"),
                            )
                        });
                        if resp.on_hover_text(message.as_str()).clicked() {
                            go_to_checks = true;
                            clicked = Some((i, false));
                        }
                    }
                }
                Row::Draw { path, index, words, main, .. } => {
                    let chosen = is_editable_tree(*main, state.edit_view.as_deref())
                        && state.selected == *path
                        && state.draw_item == Some((path.clone(), *index));
                    let r = row_ui.add(
                        mg_uikit::IconButton::new(mg_uikit::Icon::Layers, words.as_str())
                            .selectable(chosen)
                            .truncate(),
                    );
                    let kind = words.rsplit_once(' ').map_or(words.as_str(), |w| w.0);
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("Draw layer {} · {kind}", index + 1),
                        )
                    });
                    if r.clicked() {
                        if *main && state.edit_view.is_some() {
                            layouts::edit(&mut state, None);
                        }
                        structure::select(&mut state, path, false);
                        state.draw_item = Some((path.clone(), *index));
                        let owner = egui::Id::new((key, path.as_str()));
                        ctx.data_mut(|d| d.insert_temp(draw::reveal_of(owner), *index));
                    }
                }
                Row::Initial { path, .. } => {
                    let b = egui::Button::new("Initial Contents")
                        .selected(state.edit_view.is_none())
                        .frame(false);
                    if row_ui.add(b).clicked() {
                        layouts::edit(&mut state, None);
                        state.selected = format!("{path}/children/0");
                    }
                }
                Row::Variant { group, name, active, .. } => {
                    let b = egui::Button::new(format!("Variant · {name}"))
                        .selected(*active)
                        .frame(false);
                    if row_ui.add(b).clicked() {
                        state.layout_target = group.clone();
                        layouts::edit(&mut state, Some(name.clone()));
                    }
                }
            }
        }
    });
    if here {
        crate::palette_view::keys_outline(ui, shown.inner_rect, Keys::Marks);
    }
    if let Some(path) = folded {
        if state.collapsed.remove(&path) {
        } else {
            state.collapsed.insert(path.clone());
            if path.is_empty() || state.selected.starts_with(&format!("{path}/")) {
                state.selected = path;
            }
        }
    }
    // (The checks' mark: Checks brought to the front, its dock opened.)
    if go_to_checks {
        app.nui_checks_forward.push(key);
    }
    if let Some((i, additive)) = clicked
        && let Row::Control { path, main, .. } = &rows[i]
    {
        if *main && state.edit_view.is_some() {
            layouts::edit(&mut state, None);
        }
        structure::select(&mut state, path, additive);
        app.outliner.marks_keys = Some(owner);
        give_keys(&ctx, Keys::Marks);
    }
    // The keys, while they are these Layers' (none while a dialog, a menu
    // or a popover is open over them).
    let blocked = crate::shell::keys_blocked(&ctx) || mg_uikit::popover::Popover::any_open(&ctx);
    if here && !blocked {
        keys(app, ui, &mut state, &rows, shown.inner_rect.height(), owner);
    }
    end_layers(app, key, state, before);
}

/// Layers' keys ([`layers`]): Options › Keyboard's NUI keys, as the canvas
/// takes them (Previous and Next, Fold and Unfold, Delete, Duplicate, Move
/// Up and Move Down; the platform's Copy, Cut and Paste), and the list's own
/// (Page Up and Down, Home and End, Enter folding or unfolding, letters
/// going to a control by its words, Escape handing them back). What they
/// change is done as the frame ends.
fn keys(
    app: &mut Moonglow,
    ui: &mut Ui,
    state: &mut State,
    rows: &[Row],
    high: f32,
    owner: crate::outliner::Owner,
) {
    use crate::keys::Cmd;
    use egui::Key;
    let ctx = ui.ctx().clone();
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
        app.outliner.marks_keys = None;
        crate::palette_view::give_arrows(&ctx, false);
        return;
    }
    let keymap = &app.keymap;
    let cmd = |c: Cmd| ui.input(|i| keymap.pressed(i, c));
    for (c, op) in [
        (Cmd::NuiDelete, Op::Delete),
        (Cmd::NuiDuplicate, Op::Duplicate),
        (Cmd::NuiMoveUp, Op::MoveUp),
        (Cmd::NuiMoveDown, Op::MoveDown),
    ] {
        if cmd(c) {
            state.picked.act = Some(Act::Op(op));
        }
    }
    let (copy, cut, paste) = crate::widgets::copy_cut_paste(ui);
    for (op, pressed) in [copy, cut, paste].into_iter().enumerate() {
        if pressed {
            state.picked.act = Some(Act::Clip(op));
        }
    }
    let (previous, next, fold, unfold) =
        (cmd(Cmd::NuiPrevious), cmd(Cmd::NuiNext), cmd(Cmd::NuiFold), cmd(Cmd::NuiUnfold));
    let pressed = ui.input_mut(|i| {
        [Key::PageDown, Key::PageUp, Key::Home, Key::End, Key::Enter]
            .into_iter()
            .find(|k| i.consume_key(egui::Modifiers::NONE, *k))
    });
    let controls_cell = std::cell::OnceCell::new();
    let get_controls = || -> &[usize] {
        controls_cell
            .get_or_init(|| {
                (rows.iter().enumerate())
                    .filter(|(_, r)| matches!(r, Row::Control { .. }))
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>()
            })
            .as_slice()
    };
    let tree_at = |controls: &[usize], c: usize| {
        matches!(
            &rows[controls[c]],
            Row::Control { main, .. } if is_editable_tree(*main, state.edit_view.as_deref())
        )
    };
    let path_of = |controls: &[usize], c: usize| match &rows[controls[c]] {
        Row::Control { path, .. } => path.clone(),
        _ => String::new(),
    };
    let selected_at = |controls: &[usize]| -> Option<usize> {
        (0..controls.len())
            .find(|&c| tree_at(controls, c) && path_of(controls, c) == state.selected)
    };
    let mut to = None;
    if previous || next || fold || unfold || pressed.is_some() {
        let controls = get_controls();
        if controls.is_empty() {
            return;
        }
        let at = selected_at(controls);
        let last = controls.len() - 1;
        let page = ((high / mg_uikit::row_pitch(ui)) as usize).max(1);
        let from = at.unwrap_or(0);
        let branch = |c: usize| matches!(&rows[controls[c]], Row::Control { branch: true, .. });
        let parent = |c: usize| {
            let path = path_of(controls, c);
            let up = array_position(&path)
                .map(|(p, _, _)| p.rsplit_once('/').map_or(String::new(), |(p, _)| p.to_owned()))
                .or_else(|| (state.edit_view.is_none() && path == "/root").then(String::new))?;
            (0..controls.len()).find(|&c| tree_at(controls, c) && path_of(controls, c) == up)
        };
        to = if next {
            Some(at.map_or(0, |a| (a + 1).min(last)))
        } else if previous {
            Some(from.saturating_sub(1))
        } else if fold {
            let path = path_of(controls, from);
            if at.is_some_and(branch) && !state.collapsed.contains(&path) {
                state.collapsed.insert(path);
                Some(from)
            } else {
                parent(from).or(Some(from))
            }
        } else if unfold && at.is_some_and(branch) {
            match state.collapsed.remove(&path_of(controls, from)) {
                true => Some(from),
                false => Some((from + 1).min(last)),
            }
        } else {
            None
        };
        to = to.or(match pressed {
            Some(Key::PageDown) => Some((from + page).min(last)),
            Some(Key::PageUp) => Some(from.saturating_sub(page)),
            Some(Key::Home) => Some(0),
            Some(Key::End) => Some(last),
            Some(Key::Enter) if at.is_some_and(branch) => {
                let path = path_of(controls, from);
                if !state.collapsed.remove(&path) {
                    state.collapsed.insert(path);
                }
                Some(from)
            }
            _ => None,
        });
    }
    // Letters go to the control whose words begin with them; defer collecting
    // control indices and folded labels until text input actually arrives.
    let seek = |from: Option<usize>, typed: &str| -> Option<usize> {
        let controls = get_controls();
        if controls.is_empty() {
            return None;
        }
        let from = from.or_else(|| selected_at(controls)).unwrap_or(controls.len() - 1);
        let texts: Vec<String> = (0..controls.len())
            .map(|c| match &rows[controls[c]] {
                Row::Control { words, .. } => mg_core::text::fold(words),
                _ => String::new(),
            })
            .collect();
        crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))
    };
    let typed = crate::tree::typed_row(ui, egui::Id::new(("nui-layers-typed", owner)), None, &seek);
    if let Some(to) = to.or(typed) {
        let controls = get_controls();
        let path = path_of(controls, to);
        if let Row::Control { main, .. } = &rows[controls[to]]
            && *main
            && state.edit_view.is_some()
        {
            layouts::edit(state, None);
        }
        structure::select(state, &path, false);
        app.outliner.marks_moved = Some(owner);
        ctx.request_repaint();
    }
}

/// Layers end: the selection they made kept for the frame's end
/// ([`super::Picked`]), the one before put back meanwhile; the state put
/// back.
fn end_layers(app: &mut Moonglow, key: ResKey, mut state: State, before: Chosen) {
    let now = Chosen::of(&state);
    if now != before {
        state.picked.chosen = Some(now);
        before.put(&mut state);
    }
    app.nui_views.insert(key, state);
}

/// A frame of designer `key` as the shell lays it out, for the unit tests
/// that draw the tab alone: its Layers at the left, Components over
/// Properties over Bindings at the right, where it brings them, and its tab
/// in the middle.
#[cfg(test)]
pub(crate) fn test_frame(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let tab = Tab::Nui(key);
    if app.ws.is_some() {
        egui::Panel::left("test-layers").exact_size(260.0).resizable(false).show(ui, |ui| {
            layers(app, ui, key, false);
        });
        egui::Panel::right("test-right").exact_size(320.0).resizable(false).show(ui, |ui| {
            let high = ui.available_height();
            egui::Panel::top("test-components")
                .exact_size(high * 0.2)
                .resizable(false)
                .show(ui, |ui| components(app, ui, Some(&tab)));
            egui::Panel::bottom("test-bindings")
                .exact_size(high * 0.15)
                .resizable(false)
                .show(ui, |ui| bindings(app, ui, Some(&tab), true));
            egui::CentralPanel::default().show(ui, |ui| properties(app, ui, Some(&tab), false));
        });
    }
    egui::CentralPanel::default().show(ui, |ui| super::ui(app, ui, key));
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_components_are_the_controls_the_designer_names() {
        let mut seen = std::collections::BTreeSet::new();
        for (_, items) in super::super::design::COMPONENTS {
            for c in *items {
                assert!(seen.insert(c.ty), "duplicate component type {}", c.ty);
                let title = c.title.split(' ').all(|w| w.starts_with(char::is_uppercase));
                assert!(title, "{} in title case", c.title);
                assert_eq!(
                    c.title.to_ascii_lowercase(),
                    c.sentence.to_ascii_lowercase(),
                    "{} vs {}",
                    c.title,
                    c.sentence
                );
                assert_eq!(
                    c.search,
                    mg_core::text::fold(&format!("{} {}", c.ty, c.title)),
                    "search key for {}",
                    c.ty
                );
            }
        }
        assert_eq!(seen.len(), 22);
    }

    #[test]
    fn chosen_control_index_matches_active_main_or_variant_tree() {
        let row = |path: &str, main: bool| super::Row::Control {
            path: path.into(),
            depth: 0,
            words: path.into(),
            detail: None,
            mark: None,
            branch: false,
            main,
            icon: mg_uikit::Icon::Nui,
            tip: String::new(),
        };
        let rows = vec![row("/root", true), row("/root", false)];
        assert_eq!(super::chosen_control_index(&rows, "/root", None), Some(0));
        assert_eq!(super::chosen_control_index(&rows, "/root", Some("compact")), Some(1));
    }
}
