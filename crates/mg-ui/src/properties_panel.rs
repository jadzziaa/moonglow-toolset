//! The Properties panel (M4's U6, scope move S2): what is selected in an
//! area's view, in the Right Dock's lower section under the Palette and
//! the Preview (the Main, Narrow, WideArea, Floating and AreaPaint boards),
//! with every tool but Select Tiles (whose lower section is Tile
//! Properties, `tile_select::panel`).
//!
//! It is the placed object's Properties window without its OK and Cancel
//! (`blueprint::form_ui`, the body both draw): one object, or several of
//! one type edited together (each field changed set on each), each change
//! applied at once as one Undo step, nothing written by a control not
//! touched. Its pages are a rail of icons at its left (the kit's
//! `PageRail`), the page shown kept per type of object. Its header says
//! what is shown ("Placeable", "3 placeables") and holds Open in Tab, which
//! opens the Properties window of today (a tab or a window, with OK and
//! Cancel) on the same objects; while one of those is open on them, the
//! panel says so instead of drawing the same fields twice. In an area only
//! looked at (held by another builder) the fields are read-only, as the
//! window's are.

use egui::Ui;
use mg_area::ObjectKind;
use mg_core::{ResRef, ResType};
use mg_edit::GffPath;
use mg_resman::ResKey;

use crate::{Action, Moonglow, Tab};

/// What the panel says with nothing selected.
pub const NOTHING_SELECTED: &str =
    "Click an object to see its properties here, or drag a box over several of one type.";
/// Open in Tab's name and tip.
pub const OPEN_IN_TAB: &str = "Open in Tab";
const OPEN_IN_TAB_TIP: &str =
    "Open these properties in a tab or window of their own, with OK and Cancel";

/// What is selected in an area, as the panel shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Selected {
    /// Nothing (or the start location alone: `true`).
    Nothing(bool),
    /// Objects of one kind, by their places in its list (one or more).
    Objects(ObjectKind, Vec<usize>),
    /// Objects of several kinds: how many, of how many kinds.
    Mixed(usize, usize),
}

/// What is selected in `area`'s view.
pub(crate) fn selected(app: &Moonglow, area: ResRef) -> Selected {
    let Some(view) = app.area_views.get(&area) else { return Selected::Nothing(false) };
    let Some(&(kind, _)) = view.selection.first() else {
        return Selected::Nothing(view.start_selected);
    };
    if view.selection.iter().all(|(k, _)| *k == kind) {
        return Selected::Objects(kind, view.selection.iter().map(|(_, i)| *i).collect());
    }
    let mut kinds: Vec<ObjectKind> = view.selection.iter().map(|(k, _)| *k).collect();
    kinds.sort_by_key(|k| k.index());
    kinds.dedup();
    Selected::Mixed(view.selection.len(), kinds.len())
}

/// The area the editor `editor` is (the active docked one, or the floating
/// one in its window).
fn area_of(editor: Option<&Tab>) -> Option<ResRef> {
    match editor {
        Some(Tab::Area(area)) => Some(*area),
        _ => None,
    }
}

/// A kind's name, as the header says it: "Placeable", or for several
/// "3 placeables" (a name of each kind's own, not its type's spelling).
fn kind_name(kind: ObjectKind, count: usize) -> String {
    let (one, many) = match kind {
        ObjectKind::Creature => ("Creature", "creatures"),
        ObjectKind::Door => ("Door", "doors"),
        ObjectKind::Encounter => ("Encounter", "encounters"),
        ObjectKind::Item => ("Item", "items"),
        ObjectKind::Placeable => ("Placeable", "placeables"),
        ObjectKind::Sound => ("Sound", "sounds"),
        ObjectKind::Store => ("Store", "stores"),
        ObjectKind::Trigger => ("Trigger", "triggers"),
        ObjectKind::Waypoint => ("Waypoint", "waypoints"),
    };
    match count {
        1 => one.to_string(),
        n => format!("{n} {many}"),
    }
}

/// What the panel's header says after its title (the boards'
/// "Placeable"): the kind of the one object, "3 placeables", "5 objects"
/// of several kinds, "none selected".
pub(crate) fn detail(app: &Moonglow, editor: Option<&Tab>) -> String {
    match area_of(editor).map(|a| selected(app, a)) {
        Some(Selected::Objects(kind, indices)) => kind_name(kind, indices.len()),
        Some(Selected::Mixed(n, _)) => format!("{n} objects"),
        _ => "none selected".to_string(),
    }
}

/// The tab of today's Properties window on `area`'s selection (one object,
/// or several of one kind), if one is selected.
fn window_of(app: &Moonglow, area: ResRef) -> Option<Tab> {
    let Selected::Objects(kind, indices) = selected(app, area) else { return None };
    let paths: Vec<GffPath> =
        indices.iter().map(|i| GffPath::root().item(kind.list(), *i)).collect();
    Some(match paths.as_slice() {
        [one] => Tab::Instance { area, path: one.clone() },
        _ => Tab::Instances { area, paths },
    })
}

/// A Properties window open on any of `area`'s objects at `paths` (a tab
/// or a window, of one object or of several): the panel leaves those to
/// it rather than draw the same fields twice. (Read from the panes' tabs
/// as the frame began where the dock is out of reach: a dragged out
/// area's window draws its panel while the editor area is drawn.)
fn open_on(app: &Moonglow, area: ResRef, paths: &[GffPath]) -> Option<Tab> {
    let theirs = |t: &Tab| match t {
        Tab::Instance { area: a, path } => *a == area && paths.contains(path),
        Tab::Instances { area: a, paths: theirs } => {
            *a == area && theirs.iter().any(|p| paths.contains(p))
        }
        _ => false,
    };
    let docked = app.dock.iter_all_tabs().map(|(_, t)| t);
    docked.chain(app.panes_tabs.iter().flatten()).find(|t| theirs(t)).cloned()
}

/// Open in Tab, on the panel's header (at its bar's end, before the
/// dock's fold button where that is there too): enabled while objects of
/// one kind are selected. Whether it was clicked.
pub(crate) fn open_in_tab_button(app: &Moonglow, ui: &mut Ui, editor: Option<&Tab>) -> bool {
    let can = area_of(editor).is_some_and(|a| window_of(app, a).is_some());
    let button = mg_uikit::IconButton::new(mg_uikit::Icon::OpenInTab, OPEN_IN_TAB).icon_only(true);
    ui.add_enabled(can, button)
        .on_hover_text(OPEN_IN_TAB_TIP)
        .on_disabled_hover_text("Select an object, or several of one type, to open them")
        .clicked()
}

/// Opens today's Properties window on the selection of `editor`'s area
/// (Open in Tab): a tab or a window, as Options' Open editors says, with
/// OK and Cancel; on the page the panel shows.
pub(crate) fn open_in_tab(app: &mut Moonglow, editor: Option<&Tab>) {
    let Some(area) = area_of(editor) else { return };
    let Some(tab) = window_of(app, area) else { return };
    let git = ResKey::new(area, ResType::GIT);
    let first = match &tab {
        Tab::Instance { path, .. } => path.clone(),
        Tab::Instances { paths, .. } => paths[0].clone(),
        _ => return,
    };
    if let Some(restype) = crate::blueprint::instance_type(&first)
        && let Some(page) = app.panel_pages.get(&restype).copied()
    {
        app.blueprint_pages.insert((git, first), page);
    }
    app.actions.push(Action::OpenTab(tab));
}

/// A page's icon on the rail (the Main board's: Basic's `info`,
/// Inventory, Lock, Trap, Scripts, Advanced, Variables, Visuals,
/// Description; the others' by the same set).
pub(crate) fn page_icon(page: &str) -> mg_uikit::Icon {
    use mg_uikit::Icon;
    match page {
        "Basic" | "General" => Icon::Info,
        "Statistics" => Icon::Statistics,
        "Appearance" => Icon::Appearance,
        "Classes" => Icon::Class,
        "Skills" => Icon::Skill,
        "Feats" => Icon::Feat,
        "Spells" => Icon::Spell,
        "Special Abilities" => Icon::Ability,
        "Inventory" => Icon::Inventory,
        "Scripts" => Icon::Script,
        "Advanced" => Icon::Advanced,
        "Variables" => Icon::Variables,
        "Visuals" => Icon::Visuals,
        "Description" => Icon::Description,
        "Comments" => Icon::Comment,
        "Lock" => Icon::Lock,
        "Trap" => Icon::Trap,
        "Area Transition" => Icon::Link,
        "Creature List" => Icon::Creature,
        "Restrictions" => Icon::Filter,
        "Positioning" => Icon::Compass,
        "Properties" => Icon::Properties,
        // (Module and Area Properties' own.)
        "Events" => Icon::Bolt,
        "Custom Content" => Icon::Hak,
        "Visual" => Icon::Light,
        "Grass" => Icon::Grass,
        "Audio" => Icon::Sound,
        _ => Icon::Properties,
    }
}

/// The panel's body (module comment), drawn for `editor` (the active
/// docked area, or a dragged out one in its window); `titled`: its
/// section's header is over it (else, sharing the Right Dock's one tab
/// bar under 1400, it draws its detail and Open in Tab at its own top).
pub(crate) fn panel(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>, titled: bool) {
    let Some(area) = area_of(editor) else { return };
    // (The pane F6 is at outlined, as the other panes are.)
    let keys = crate::palette_view::Keys::Properties;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    let focus = std::mem::take(&mut app.props_focus);
    if !titled {
        let mut opened = false;
        let margin = egui::Margin { left: 8, right: 4, top: 2, bottom: 2 };
        egui::Frame::new().inner_margin(margin).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.weak(detail(app, editor));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    opened = open_in_tab_button(app, ui, editor);
                });
            });
        });
        if opened {
            open_in_tab(app, editor);
        }
    }
    let (kind, indices) = match selected(app, area) {
        Selected::Objects(kind, indices) => (kind, indices),
        Selected::Nothing(start) => {
            padded(ui, |ui| {
                ui.weak(match start {
                    true => "The start location: Adjust Location in its menu places it exactly.",
                    false => NOTHING_SELECTED,
                })
            });
            return;
        }
        Selected::Mixed(n, kinds) => {
            padded(ui, |ui| {
                ui.weak(format!(
                    "{n} objects of {kinds} types are selected: select objects of one type to \
                     edit them together."
                ))
            });
            return;
        }
    };
    let git = ResKey::new(area, ResType::GIT);
    let paths: Vec<GffPath> =
        indices.iter().map(|i| GffPath::root().item(kind.list(), *i)).collect();
    // (Their Properties window open: the fields are there, not twice.)
    if let Some(tab) = open_on(app, area, &paths) {
        let shown = padded(ui, |ui| {
            ui.weak("These properties are open in a tab or window of their own.");
            ui.button("Show It").on_hover_text("Bring that tab or window to the front")
        });
        if shown.clicked() {
            app.actions.push(Action::OpenTab(tab));
        }
        return;
    }
    let restype = kind.restype();
    let pages = crate::blueprint::instance_pages(restype);
    let mut page = app.panel_pages.get(&restype).copied().filter(|p| pages.contains(p));
    let mut at = page.and_then(|p| pages.iter().position(|q| *q == p)).unwrap_or(0);
    let whole = ui.available_rect_before_wrap();
    // The rail at the left, the whole height.
    let mut rail = mg_uikit::pages::PageRail::new(("properties-pages", area));
    for p in &pages {
        rail = rail.page(page_icon(p), *p);
    }
    let mut left = ui.new_child(egui::UiBuilder::new().max_rect(whole));
    let shown = rail.show(&mut left, &mut at);
    if focus && let Some(r) = shown.pages.get(at) {
        r.request_focus();
    }
    page = Some(pages[at]);
    app.panel_pages.insert(restype, pages[at]);
    // The page beside it, 8 in from the rail and 4 from the dock's edge,
    // 8 under its top (the boards' `.content`).
    let room = whole.with_min_x(whole.left() + mg_uikit::pages::PAGE_RAIL + 8.0);
    let room = room.with_max_x(room.right() - 4.0).with_min_y(room.top() + 8.0);
    let mut body = ui.new_child(egui::UiBuilder::new().max_rect(room).id_salt("page"));
    body.set_clip_rect(room.intersect(ui.clip_rect()));
    let path = paths[0].clone();
    let also: Vec<(ResKey, GffPath)> = paths[1..].iter().map(|p| (git, p.clone())).collect();
    let Some(root) = object_now(app, &path, git) else {
        body.weak("This object is no longer there.");
        return;
    };
    // (The one object's blueprint, place and facing under the page's
    // name, as the boards draw it.)
    let readout = match indices.as_slice() {
        [one] => readout(app, area, kind, *one),
        _ => None,
    };
    let (held, is_viewed) = crate::blueprint::looked_at(app, &git);
    let page = page.unwrap_or(pages[0]);
    let shown = crate::blueprint::FormShown {
        key: git,
        path,
        also,
        root,
        restype,
        page,
        held,
        is_viewed,
        heading: Some((page, readout)),
        all: None,
    };
    // (Its fields' ids follow their objects as a window's do, should an
    // edit renumber them while one is typed in: `follow::window_id`.)
    for p in &paths {
        crate::blueprint::follow::window_id(app, area, p);
    }
    app.panel_form = true;
    crate::blueprint::form_ui(app, &mut body, shown);
    app.panel_form = false;
    // (A field typed in here is the panel's: no window drawn after it
    // this frame claims it for its Cancel.)
    let pass = ui.ctx().cumulative_pass_nr();
    if let Some(held) = app.held_text.as_mut().filter(|h| h.window.is_none() && h.drawn == pass) {
        held.panel = true;
    }
    ui.advance_cursor_after_rect(whole);
}

/// What the panel says in place of the fields, in a page's room (8 in).
fn padded<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let margin = egui::Margin { left: 8, right: 8, top: 8, bottom: 4 };
    egui::Frame::new().inner_margin(margin).show(ui, add).inner
}

/// The object at `path` of `area`'s GIT `git` as it is now.
fn object_now(app: &mut Moonglow, path: &GffPath, git: ResKey) -> Option<mg_gff::Struct> {
    let ws = app.ws.as_mut()?;
    ws.doc(&git).ok().and_then(|g| path.get(&g.root).cloned())
}

/// One object's blueprint, place and facing ("plc_plaguepyre at 24.28,
/// 65.05, 0.00, facing 0°"), as the area's view has it.
fn readout(app: &Moonglow, area: ResRef, kind: ObjectKind, index: usize) -> Option<String> {
    let view = app.area_views.get(&area)?;
    let o = view.model.as_ref()?.object(kind, index)?;
    let blueprint = o.template.map(|t| t.to_string()).unwrap_or_default();
    Some(format!(
        "{blueprint} at {:.2}, {:.2}, {:.2}, facing {:.0}°",
        o.position.x,
        o.position.y,
        o.position.z,
        o.facing().to_degrees().rem_euclid(360.0),
    ))
}
