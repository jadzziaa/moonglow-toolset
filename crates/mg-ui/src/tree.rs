//! The module contents tree (Aurora's left pane).

use std::collections::HashMap;

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::Workspace;
use mg_gff::Gff;
use mg_resman::ResKey;
use mg_rules::GameData;

use crate::{Action, Moonglow, Tab};

/// What Make Placeables Static does, for its menu items.
const STATIC_TIP: &str = "Sets Static on the placeables that lose nothing by it: the game loads and \
    draws static placeables more cheaply. Left dynamic: those that are Useable, are tilted, \
    scaled or moved by a visual transform, or have a script, a conversation, a trap, an \
    inventory or an animation switched on";

const GROUPS: &[(&str, &[ResType])] = &[
    ("Areas", &[ResType::ARE]),
    ("Conversations", &[ResType::DLG]),
    ("Scripts", &[ResType::NSS]),
    ("Creatures", &[ResType::UTC]),
    ("Doors", &[ResType::UTD]),
    ("Encounters", &[ResType::UTE]),
    ("Items", &[ResType::UTI]),
    ("Merchants", &[ResType::UTM]),
    ("Placeables", &[ResType::UTP]),
    ("Sounds", &[ResType::UTS]),
    ("Triggers", &[ResType::UTT]),
    ("Waypoints", &[ResType::UTW]),
    ("Journal and factions", &[ResType::JRL, ResType::FAC]),
];

/// What lists of areas show and filter by, from an area's ARE.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AreaInfo {
    /// Its name in the editing language, else as the game shows it (its
    /// talk-table string); empty when it has none.
    pub name: String,
    pub tileset: ResRef,
    /// `Flags`: 1 interior, 2 underground, 4 natural.
    pub flags: u32,
}

/// The areas' names (and tilesets and flags), each read from its ARE once:
/// again when the ARE is replaced, and from the open document while it is
/// being edited.
#[derive(Debug, Default)]
pub(crate) struct AreaNames {
    /// By area: the stored ARE read (where its data is and its length:
    /// replaced data is elsewhere) and what it says.
    read: HashMap<ResRef, ((usize, usize), AreaInfo)>,
    /// Blueprints' names, likewise.
    names: HashMap<ResKey, ((usize, usize), String)>,
}

/// A blueprint's name in the editing language, else as the game shows it:
/// a creature's first and last names, another's `LocalizedName` or
/// `LocName`.
fn blueprint_name(game: Option<&GameData>, root: &mg_gff::Struct) -> String {
    let text = |label: &str| {
        root.locstring(label).map_or(String::new(), |name| {
            let edited = crate::text::edited_text(name);
            // (Its text in another language is a last resort: below.)
            if !edited.is_empty() {
                return edited;
            }
            game.and_then(|g| g.locstring(name))
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| crate::text::shown_text(name))
        })
    };
    let full = [text("FirstName"), text("LastName")];
    let full: Vec<&str> = full.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if !full.is_empty() {
        return full.join(" ");
    }
    let name = text("LocalizedName");
    if name.trim().is_empty() { text("LocName") } else { name }
}

impl AreaNames {
    pub(crate) fn info(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        area: ResRef,
    ) -> AreaInfo {
        let key = ResKey::new(area, ResType::ARE);
        let read = |are: &Gff| {
            let name = are.root.locstring("Name").map_or(String::new(), |name| {
                let edited = crate::text::edited_text(name);
                // (Its text in another language is a last resort: below.)
                if !edited.is_empty() {
                    return edited;
                }
                game.and_then(|g| g.locstring(name))
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| crate::text::shown_text(name))
            });
            AreaInfo {
                name,
                tileset: are.root.resref("Tileset").unwrap_or(ResRef::EMPTY),
                flags: are.root.integer("Flags").unwrap_or(0) as u32,
            }
        };
        if let Some(are) = ws.loaded(&key) {
            return read(are);
        }
        let Some(data) = ws.module.get(&key) else { return AreaInfo::default() };
        let stamp = (data.as_ptr() as usize, data.len());
        if let Some((s, info)) = self.read.get(&area)
            && *s == stamp
        {
            return info.clone();
        }
        let info = Gff::read(data).map(|g| read(&g)).unwrap_or_default();
        self.read.insert(area, (stamp, info.clone()));
        info
    }

    /// A blueprint's name (empty when it has none), read once as the
    /// areas' are.
    pub(crate) fn blueprint(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        key: ResKey,
    ) -> String {
        if let Some(doc) = ws.loaded(&key) {
            return blueprint_name(game, &doc.root);
        }
        let Some(data) = ws.module.get(&key) else { return String::new() };
        let stamp = (data.as_ptr() as usize, data.len());
        if let Some((s, name)) = self.names.get(&key)
            && *s == stamp
        {
            return name.clone();
        }
        let name = Gff::read(data).map(|g| blueprint_name(game, &g.root)).unwrap_or_default();
        self.names.insert(key, (stamp, name.clone()));
        name
    }

    /// The area's name in the editing language, else as the game shows it
    /// (its talk-table string); empty when it has none.
    pub(crate) fn name(&mut self, ws: &Workspace, game: Option<&GameData>, area: ResRef) -> String {
        self.info(ws, game, area).name
    }

    /// What the area is called in lists and tab titles: its name when
    /// `by_name` and it has one, else its ResRef.
    pub(crate) fn label(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        area: ResRef,
        by_name: bool,
    ) -> String {
        let name = if by_name { self.name(ws, game, area) } else { String::new() };
        if name.trim().is_empty() { area.to_string() } else { name }
    }
}

/// What is placed in an area, for the module tree: each kind of object
/// and its objects' names (by their place in the area's list).
pub(crate) type AreaContents = Vec<(mg_area::ObjectKind, Vec<String>)>;

/// What is placed in an area, as the workspace has it now: every kind,
/// its objects by name (a tag for one without a name).
fn area_contents(ws: &mut Workspace, game: Option<&GameData>, area: ResRef) -> AreaContents {
    let Ok(git) = ws.doc(&ResKey::new(area, ResType::GIT)) else { return Vec::new() };
    mg_area::ObjectKind::ALL
        .into_iter()
        .map(|kind| {
            let names = git.root.list(kind.list()).unwrap_or(&[]).iter().map(|o| {
                let name = blueprint_name(game, o);
                let tag = || String::from_utf8_lossy(o.string("Tag").unwrap_or_default()).into();
                if name.trim().is_empty() { tag() } else { name }
            });
            (kind, names.collect())
        })
        .collect()
}

/// What was asked of an object in an area's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    /// Shown in the area's view.
    Go,
    Properties,
    Copy,
    Delete,
}

/// An area's placed objects under its row of the module tree, kind by
/// kind as Aurora lists them. A click goes to the object in the area's
/// view; a double click opens its Properties; a right click has both, and
/// Copy and Delete. With a filter that some of them match (lower case),
/// only those are listed. Only the rows in view are laid out (an area can
/// hold thousands of a kind).
fn contents_ui(
    ui: &mut Ui,
    area: ResRef,
    contents: &AreaContents,
    filter: &str,
    go: &mut Option<(ResRef, mg_area::ObjectKind, usize, Asked)>,
    // The object in hand (its row marked), and whether to go to it.
    in_hand: Option<(mg_area::ObjectKind, usize, bool)>,
) -> bool {
    let mut went = false;
    let matches = |name: &str| name.to_lowercase().contains(filter);
    let filtered =
        !filter.is_empty() && contents.iter().any(|(_, names)| names.iter().any(|n| matches(n)));
    ui.indent(("area-contents", area), |ui| {
        for (kind, names) in contents {
            let shown: Vec<usize> = if filtered {
                (0..names.len()).filter(|&i| matches(&names[i])).collect()
            } else {
                (0..names.len()).collect()
            };
            let title = if filtered {
                format!("{} ({} of {})", kind.plural(), shown.len(), names.len())
            } else {
                format!("{} ({})", kind.plural(), names.len())
            };
            if shown.is_empty() {
                if !filtered {
                    ui.weak(title);
                }
                continue;
            }
            let mut header = egui::CollapsingHeader::new(title).id_salt(("area-kind", area, *kind));
            // (What is looked for shows without opening each kind; so does
            // the object to go to.)
            let marked = in_hand.filter(|(k, ..)| k == kind);
            let goes = marked.filter(|m| m.2).and_then(|m| shown.iter().position(|&i| i == m.1));
            if filtered || goes.is_some() {
                header = header.open(Some(true));
            }
            header.show(ui, |ui| {
                // The rows in view, and room for those above and below.
                let font = egui::TextStyle::Button.resolve(ui.style());
                let spacing = ui.spacing();
                let row = (ui.fonts_mut(|f| f.row_height(&font)) + 2.0 * spacing.button_padding.y)
                    .max(spacing.interact_size.y);
                let step = row + spacing.item_spacing.y;
                let top = ui.cursor().top();
                // The row to go to, in view or not (rows out of view are
                // not laid out: by where it would be).
                if let Some(at) = goes {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.cursor().left(), top + at as f32 * step),
                        egui::vec2(1.0, row),
                    );
                    ui.scroll_to_rect(rect, Some(egui::Align::Center));
                    went = true;
                }
                let clip = ui.clip_rect();
                let first =
                    (((clip.top() - top) / step).floor().max(0.0) as usize).min(shown.len());
                let last = (((clip.bottom() - top) / step).ceil().max(0.0) as usize)
                    .clamp(first, shown.len());
                let room = |ui: &mut Ui, rows: usize| {
                    if rows > 0 {
                        let tall = rows as f32 * step - spacing_gap(ui);
                        ui.allocate_space(egui::vec2(1.0, tall.max(0.0)));
                    }
                };
                room(ui, first);
                for &index in &shown[first..last] {
                    let name = &names[index];
                    let text = if name.is_empty() { "(no name)" } else { name.as_str() };
                    let is_marked = marked.is_some_and(|m| m.1 == index);
                    let r = ui.selectable_label(is_marked, text).on_hover_text(
                        "Click to go to it in the area; double-click for its Properties; \
                         right-click for more",
                    );
                    if r.double_clicked() {
                        *go = Some((area, *kind, index, Asked::Properties));
                    } else if r.clicked() {
                        *go = Some((area, *kind, index, Asked::Go));
                    }
                    r.context_menu(|ui| {
                        for (label, asked) in [
                            ("Go To", Asked::Go),
                            ("Properties", Asked::Properties),
                            ("Copy", Asked::Copy),
                            ("Delete", Asked::Delete),
                        ] {
                            if ui.button(label).clicked() {
                                *go = Some((area, *kind, index, asked));
                                ui.close();
                            }
                        }
                    });
                }
                room(ui, shown.len() - last);
            });
        }
    });
    went
}

/// The gap between rows (taken off a block of rows left out, which is
/// followed by one itself).
fn spacing_gap(ui: &Ui) -> f32 {
    ui.spacing().item_spacing.y
}

/// The module tree's pane: the Filter and Expand All / Collapse All, which
/// stay in sight, over the tree, which scrolls (Home and End go to its
/// top and bottom while the pointer is over it).
pub(crate) fn module_tree(app: &mut Moonglow, ui: &mut Ui) {
    if app.ws.is_none() {
        return;
    }
    let filter_id = egui::Id::new("tree-filter");
    let mut filter = app.buffers.get(&filter_id).cloned().unwrap_or_default();
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Filter");
        // (As wide as the pane has room for, not wider.)
        ui.add(egui::TextEdit::singleline(&mut filter).desired_width(f32::INFINITY));
    });
    app.buffers.insert(filter_id, filter.clone());
    let filter = filter.to_ascii_lowercase();
    // Every group opened or closed at once (this frame). A filter typed
    // opens every group with a match, as it is typed: they can be closed
    // again after.
    let typed = crate::widgets::text_changed(ui, filter_id, &filter) && !filter.is_empty();
    let fold = crate::widgets::fold_buttons(ui, "group").or(typed.then_some(true));
    let mut rows = egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        // A name too long for the pane is cut short (the pointer over it
        // shows it whole), rather than widen the pane over the middle.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        tree_rows(app, ui, &filter, fold);
    });
    crate::widgets::home_and_end(ui, &mut rows);
}

/// The one object selected in the area in front, if it is another than
/// the tree shows: the tree then goes to it (its kind's list opened),
/// where its area is opened out there. An area left closed in the tree
/// stays closed: the tree does not move.
fn follow_selection(app: &mut Moonglow) {
    let selected = app.palette.area.and_then(|area| {
        let view = app.area_views.get(&area)?;
        match view.selection.as_slice() {
            [(kind, index)] => Some((area, *kind, *index)),
            _ => None,
        }
    });
    if selected != app.tree_object {
        app.tree_object = selected;
        app.tree_object_pending = selected.is_some();
    }
}

/// A row of the module tree the arrow keys can be at: a group's, or a
/// resource's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeAt {
    Group(&'static str),
    Resource(ResKey),
}

/// Where the tree's keys take the cursor from `at`, among the rows shown
/// (in order, each group's row before its resources'): the row to go to,
/// a group to open or close, or the resource to open.
#[derive(Debug, PartialEq)]
enum Step {
    To(TreeAt),
    Fold(&'static str, bool),
    Open(ResKey),
    None,
}

/// The rows shown: each group, whether it is open, and its resources.
type Shown = Vec<(&'static str, bool, Vec<ResKey>)>;

fn step(shown: &Shown, at: Option<TreeAt>, key: egui::Key) -> Step {
    use egui::Key;
    let rows: Vec<TreeAt> = shown
        .iter()
        .flat_map(|(name, open, keys)| {
            let inside = keys.iter().filter(move |_| *open).map(|k| TreeAt::Resource(*k));
            std::iter::once(TreeAt::Group(name)).chain(inside)
        })
        .collect();
    let Some(first) = rows.first().copied() else { return Step::None };
    let last = rows[rows.len() - 1];
    let index = at.and_then(|a| rows.iter().position(|r| *r == a));
    let by = |n: isize| match index {
        Some(i) => Step::To(rows[(i as isize + n).clamp(0, rows.len() as isize - 1) as usize]),
        // (No row yet, or one that is gone: the first.)
        None => Step::To(first),
    };
    let group_of = |k: &ResKey| shown.iter().find(|(_, _, keys)| keys.contains(k)).map(|g| g.0);
    match (key, index.map(|i| rows[i])) {
        (Key::ArrowDown, _) => by(1),
        (Key::ArrowUp, _) => by(-1),
        (Key::PageDown, _) => by(10),
        (Key::PageUp, _) => by(-10),
        (Key::Home, _) => Step::To(first),
        (Key::End, _) => Step::To(last),
        // Right opens a group, then goes into it; Left goes to a row's
        // group, then closes it.
        (Key::ArrowRight, Some(TreeAt::Group(name))) => match shown.iter().find(|g| g.0 == name) {
            Some((_, true, keys)) => {
                keys.first().map_or(Step::None, |k| Step::To(TreeAt::Resource(*k)))
            }
            Some((_, false, _)) => Step::Fold(name, true),
            None => Step::None,
        },
        (Key::ArrowLeft, Some(TreeAt::Group(name))) => Step::Fold(name, false),
        (Key::ArrowLeft, Some(TreeAt::Resource(k))) => {
            group_of(&k).map_or(Step::None, |g| Step::To(TreeAt::Group(g)))
        }
        (Key::Enter, Some(TreeAt::Resource(k))) => Step::Open(k),
        (Key::Enter, Some(TreeAt::Group(name))) => {
            let open = shown.iter().any(|g| g.0 == name && g.1);
            Step::Fold(name, !open)
        }
        _ => Step::None,
    }
}

fn tree_rows(app: &mut Moonglow, ui: &mut Ui, filter: &str, fold: Option<bool>) {
    follow_selection(app);
    // The rows shown, for the keys; a group the keys opened or closed.
    let mut shown: Shown = Vec::new();
    let folded = app.tree_fold.take();
    let cursor = app.tree_cursor;
    let cursor_moved = std::mem::take(&mut app.tree_cursor_moved);
    let mut clicked = None;
    let Some(ws) = &app.ws else { return };
    let revision = ws.revision();
    // Areas opened out whose contents are to be read (after the tree is
    // drawn), and the object clicked.
    let mut read = Vec::new();
    let mut go = None;
    let (by_name, resrefs) = (app.settings.area_names, app.settings.name_resrefs);
    let filter = filter.to_string();
    if ui.selectable_label(false, "Module Properties").clicked() {
        app.actions.push(Action::OpenTab(Tab::ModuleProperties));
    }
    let mut open = None;
    let mut make = None;
    let mut listed = 0;
    for (name, types) in GROUPS {
        let mut keys: Vec<ResKey> =
            ws.module.keys().filter(|k| types.contains(&k.restype)).copied().collect();
        keys.sort();
        listed += keys.len();
        // Areas by name (Options › General): named, and in the names'
        // order; the filter finds a name or a ResRef.
        let mut names: HashMap<ResKey, String> = HashMap::new();
        let named = types == &[ResType::ARE]
            || types.iter().all(|t| mg_area::ObjectKind::from_restype(*t).is_some());
        if by_name && named {
            let game = app.game.as_deref();
            for k in &keys {
                let name = if k.restype == ResType::ARE {
                    app.area_names.name(ws, game, k.resref)
                } else {
                    app.area_names.blueprint(ws, game, *k)
                };
                // (One without a name keeps its ResRef.)
                if !name.trim().is_empty() {
                    names.insert(*k, named_label(&name, k.resref, resrefs));
                }
            }
            let shown =
                |k: &ResKey| names.get(k).map_or_else(|| k.resref.to_string(), String::clone);
            keys.sort_by_cached_key(|k| (shown(k).to_lowercase(), *k));
        }
        // (An area opened out stays for what is placed in it, too.)
        let holds = |k: &ResKey| {
            k.restype == ResType::ARE
                && ui.data(|d| d.get_temp(egui::Id::new(("tree-area-open", k.resref))))
                    == Some(true)
                && app.area_contents.get(&k.resref).is_some_and(|(_, contents)| {
                    contents
                        .iter()
                        .any(|(_, names)| names.iter().any(|n| n.to_lowercase().contains(&filter)))
                })
        };
        keys.retain(|k| {
            filter.is_empty()
                || k.to_string().contains(&filter)
                || names.get(k).is_some_and(|n| n.to_lowercase().contains(&filter))
                || holds(k)
        });
        // What makes a new resource of the group (its wizard, or the New
        // window): on the group's and its resources' right-click menus.
        let new = new_command(types);
        let new_label = new.map(|id| new_label(&id.name()));
        let at_group = cursor == Some(TreeAt::Group(name));
        let listed_keys = keys.clone();
        let header = egui::CollapsingHeader::new(format!("{name} ({})", keys.len()))
            .id_salt(name)
            .default_open(*name == "Areas")
            // (The keys' cursor on a group shows as its row marked.)
            .show_background(at_group)
            // An area to bring into view opens the areas' group (a filter
            // typed opens every group, through `fold`); the keys open and
            // close the one they are at.
            .open(
                (*name == "Areas" && app.tree_reveal.is_some())
                    .then_some(true)
                    .or(folded.filter(|(g, _)| g == name).map(|(_, open)| open))
                    .or(fold),
            )
            .show(ui, |ui| {
                for k in keys {
                    let label = match (names.get(&k), k.restype) {
                        (Some(name), _) => name.clone(),
                        (None, ResType::ARE | ResType::DLG | ResType::NSS) => k.resref.to_string(),
                        _ => k.to_string(),
                    };
                    // Blueprints drag into an area, as from the palette.
                    let blueprint = mg_area::ObjectKind::from_restype(k.restype).is_some();
                    let sense = if blueprint {
                        egui::Sense::click_and_drag()
                    } else {
                        egui::Sense::click()
                    };
                    // An area opens out to what is placed in it.
                    let opened = egui::Id::new(("tree-area-open", k.resref));
                    let is_area = k.restype == ResType::ARE;
                    let mut out = is_area && ui.data(|d| d.get_temp(opened).unwrap_or(false));
                    // The area to bring into view (a tab chosen, or its
                    // menu's Show in Module Tree, which opens it out too).
                    let reveal = app.tree_reveal.filter(|(a, _)| is_area && *a == k.resref);
                    if let Some((area, open_out)) = reveal {
                        app.tree_area = Some(area);
                        if open_out && !out {
                            out = true;
                            ui.data_mut(|d| d.insert_temp(opened, true));
                        }
                    }
                    let in_hand = is_area && app.tree_area == Some(k.resref);
                    let at_row = cursor == Some(TreeAt::Resource(k));
                    // A row out of sight takes its room only (a module can
                    // list thousands of a kind: laid out every frame, they
                    // slowed everything down while their group was open).
                    let height = ui.spacing().interact_size.y;
                    let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(1.0, height));
                    // (The keys' row out of sight is drawn, to scroll to it.)
                    let sought = at_row && cursor_moved;
                    if !out && reveal.is_none() && !sought && !ui.is_rect_visible(room) {
                        ui.allocate_space(egui::vec2(1.0, height));
                        continue;
                    }
                    let row = ui.horizontal(|ui| {
                        if is_area {
                            let arrow = if out { "⏷" } else { "⏵" };
                            let toggle = ui
                                .small_button(arrow)
                                .on_hover_text("What is placed in the area, by kind");
                            if toggle.clicked() {
                                out = !out;
                                ui.data_mut(|d| d.insert_temp(opened, out));
                            }
                        }
                        ui.add(egui::Button::selectable(in_hand || at_row, label).sense(sense))
                    });
                    // (A row out of sight is given this height.)
                    debug_assert!(
                        (row.response.rect.height() - height).abs() < 0.5,
                        "a tree row is {} high, not {height}",
                        row.response.rect.height()
                    );
                    let mut r = row.inner;
                    if reveal.is_some() {
                        r.scroll_to_me(Some(egui::Align::Center));
                    }
                    if sought {
                        r.scroll_to_me(None);
                    }
                    if r.clicked() {
                        clicked = Some(TreeAt::Resource(k));
                    }
                    if out {
                        match app.area_contents.get(&k.resref).filter(|c| c.0 == revision) {
                            Some((_, contents)) => {
                                let in_hand = app
                                    .tree_object
                                    .filter(|(a, ..)| *a == k.resref)
                                    .map(|(_, kind, i)| (kind, i, app.tree_object_pending));
                                if contents_ui(ui, k.resref, contents, &filter, &mut go, in_hand) {
                                    app.tree_object_pending = false;
                                }
                            }
                            None => read.push(k.resref),
                        }
                    }
                    if names.contains_key(&k) {
                        r = r.on_hover_text(k.resref.to_string());
                    }
                    if blueprint && r.drag_started() {
                        r.dnd_set_drag_payload(crate::palette_view::Dragged(k));
                    }
                    if r.double_clicked() {
                        open = Some(k);
                    }
                    // Delete on the row under the pointer asks, as its
                    // menu's Delete… does (not while text is typed).
                    if r.hovered()
                        && !ui.ctx().egui_wants_keyboard_input()
                        && ui.input(|i| i.key_pressed(egui::Key::Delete))
                    {
                        app.actions.push(Action::DeleteDialog(k));
                    }
                    r.context_menu(|ui| {
                        if let (Some(id), Some(label)) = (new, &new_label) {
                            if ui.button(label).clicked() {
                                make = Some(id);
                            }
                            ui.separator();
                        }
                        if k.restype != ResType::ARE {
                            if ui.button("Edit").clicked() {
                                open = Some(k);
                            }
                            ui.separator();
                        }
                        if k.restype == ResType::ARE {
                            if ui
                                .button("Edit Areas Together…")
                                .on_hover_text(
                                    "Choose more areas to set their properties with this one's",
                                )
                                .clicked()
                            {
                                app.area_chooser =
                                    Some(crate::area_props::AreaChooser::with(k.resref));
                            }
                            if ui.button("View Area").clicked() {
                                app.actions.push(Action::OpenTab(Tab::Area(k.resref)));
                            }
                            if ui.button("Properties").clicked() {
                                app.actions.push(Action::OpenTab(Tab::AreaProperties(k.resref)));
                            }
                            if ui
                                .button("Export Minimap…")
                                .on_hover_text("The area's map as the game shows it, as a PNG")
                                .clicked()
                            {
                                app.actions.push(Action::ExportMinimap(k.resref));
                            }
                            if ui
                                .button("Make Placeables Static")
                                .on_hover_text(STATIC_TIP)
                                .clicked()
                            {
                                app.actions.push(Action::StaticPlaceables(vec![k.resref]));
                            }
                            if ui
                                .button("Make Placeables Dynamic")
                                .on_hover_text("Clears Static on every placeable of the area")
                                .clicked()
                            {
                                app.actions.push(Action::DynamicPlaceables(vec![k.resref]));
                            }
                            for t in [ResType::ARE, ResType::GIT] {
                                let g = ResKey::new(k.resref, t);
                                if ui.button(format!("Fields of {g}")).clicked() {
                                    app.actions.push(Action::OpenTab(Tab::Gff(g)));
                                }
                            }
                            ui.separator();
                        }
                        if mg_module::rename::renamable(k.restype) {
                            if ui.button("Find References").clicked() {
                                app.actions.push(Action::FindReferences(k));
                            }
                            if ui.button("Rename…").clicked() {
                                app.actions.push(Action::RenameDialog(k));
                            }
                        }
                        if ui
                            .button("Copy…")
                            .on_hover_text("A copy in the module, under a ResRef you give")
                            .clicked()
                        {
                            app.actions.push(Action::CopyDialog(k));
                        }
                        if ui.button("Delete…").clicked() {
                            app.actions.push(Action::DeleteDialog(k));
                        }
                        if ui.button("Export…").clicked() {
                            app.actions.push(Action::ExportDialog(vec![k]));
                        }
                        let with = match k.restype {
                            ResType::NSS => " (with its compiled script)",
                            ResType::ARE => " (with its .git and .gic)",
                            _ => "",
                        };
                        if ui
                            .button("Export as Files…")
                            .on_hover_text(format!("Write {k} into a folder{with}"))
                            .clicked()
                        {
                            app.actions.push(Action::ExportFiles {
                                keys: vec![k],
                                dependencies: false,
                                scratch: false,
                            });
                        }
                        let tip = crate::transfer::scratch_tip(
                            app.settings.scratch_dir.as_deref(),
                            &format!("{k}{with}"),
                        );
                        if ui.button("Copy to Scratch Folder").on_hover_text(tip).clicked() {
                            app.actions.push(Action::ExportFiles {
                                keys: vec![k],
                                dependencies: false,
                                scratch: true,
                            });
                        }
                    });
                }
            });
        // (Brought into view, or not among the areas shown: done.)
        if *name == "Areas" {
            app.tree_reveal = None;
            // The object selected was gone to if its area is opened out;
            // if not (or the areas are folded away), the tree stays as it
            // is. (One whose area is being read again is gone to then.)
            if !app.tree_object.is_some_and(|(area, ..)| read.contains(&area)) {
                app.tree_object_pending = false;
            }
        }
        if header.header_response.clicked() {
            clicked = Some(TreeAt::Group(name));
        }
        if at_group && cursor_moved {
            header.header_response.scroll_to_me(None);
        }
        shown.push((*name, !header.fully_closed(), listed_keys));
        if let (Some(id), Some(label)) = (new, &new_label) {
            header.header_response.context_menu(|ui| {
                if ui.button(label).clicked() {
                    make = Some(id);
                }
                if types.contains(&ResType::ARE)
                    && ui
                        .button("Make Placeables Static in Every Area")
                        .on_hover_text(STATIC_TIP)
                        .clicked()
                {
                    let areas = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
                    app.actions.push(Action::StaticPlaceables(areas));
                }
                if types.contains(&ResType::ARE)
                    && ui
                        .button("Make Placeables Dynamic in Every Area")
                        .on_hover_text("Clears Static on every placeable of the module")
                        .clicked()
                {
                    let areas = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
                    app.actions.push(Action::DynamicPlaceables(areas));
                }
            });
        }
    }
    let others = ws.module.len() - listed;
    ui.weak(format!("{others} other resources"));
    // A click in the tree gives it the arrow keys (as the palette's):
    // with the pointer over it, Up and Down go from row to row (Page Up
    // and Down by ten, Home and End to the ends), Right opens a group and
    // goes into it, Left goes back to the group and closes it, Enter
    // opens the row's resource, Escape hands the keys back.
    if let Some(at) = clicked {
        app.tree_cursor = Some(at);
        crate::palette_view::give_arrows(ui.ctx(), true);
    }
    let here = ui.rect_contains_pointer(ui.clip_rect());
    if here
        && crate::palette_view::has_arrows(ui.ctx())
        && !ui.ctx().egui_wants_keyboard_input()
        && app.tree_cursor.is_some()
    {
        use egui::Key;
        let keys = [
            Key::ArrowDown,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::Home,
            Key::End,
            Key::ArrowRight,
            Key::ArrowLeft,
            Key::Enter,
        ];
        let pressed =
            ui.input_mut(|i| keys.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            app.tree_cursor = None;
            crate::palette_view::give_arrows(ui.ctx(), false);
        }
        match pressed.map_or(Step::None, |key| step(&shown, app.tree_cursor, key)) {
            Step::To(at) => {
                app.tree_cursor = Some(at);
                app.tree_cursor_moved = true;
                ui.ctx().request_repaint();
            }
            Step::Fold(group, open) => {
                app.tree_fold = Some((group, open));
                ui.ctx().request_repaint();
            }
            Step::Open(k) => open = Some(k),
            Step::None => {}
        }
    }
    if let Some(k) = open {
        app.actions.push(Action::OpenResource(k));
    }
    // What the areas opened out hold, read for the next frame.
    if !read.is_empty()
        && let Some(ws) = app.ws.as_mut()
    {
        for area in read {
            let contents = area_contents(ws, app.game.as_deref(), area);
            app.area_contents.insert(area, (revision, contents));
        }
        ui.ctx().request_repaint();
    }
    // An object in an area's list: gone to in the area's view, its
    // Properties opened, copied or deleted.
    if let Some((area, kind, index, asked)) = go {
        let git = ResKey::new(area, ResType::GIT);
        match asked {
            Asked::Properties => {
                let path = mg_edit::GffPath::root().item(kind.list(), index);
                app.actions.push(Action::OpenTab(Tab::Instance { area, path }));
            }
            Asked::Go => {
                app.area_focus = Some((area, kind, index));
                app.actions.push(Action::OpenTab(Tab::Area(area)));
            }
            Asked::Copy => {
                // As Copy in the area's view: pasted in any area (Ctrl+V).
                let placed = app
                    .ws
                    .as_mut()
                    .and_then(|ws| ws.doc(&git).ok()?.root.list(kind.list())?.get(index).cloned());
                if let (Some(s), Some(game)) = (placed, app.game.as_deref()) {
                    let o = mg_area::AreaObject::read(game, kind, index, &s);
                    let anchor = o.position;
                    app.object_clip =
                        Some(crate::area_view::ObjectClip { objects: vec![(o, s, 0.0)], anchor });
                    crate::widgets::mark_clipboard(ui.ctx(), "1 object");
                    app.log.info("1 object copied: paste it in an area (Ctrl+V)");
                }
            }
            Asked::Delete => {
                let edits = mg_area::edit::delete_edits(git, &[(kind, index)]);
                // (Objects after it move up their list: what is selected
                // in the area's view, and the Properties open, would be
                // others'.)
                if let Some(view) = app.area_views.get_mut(&area) {
                    view.selection.clear();
                }
                app.dock.retain_tabs(|t| {
                    !matches!(t, Tab::Instance { area: a, .. } | Tab::Instances { area: a, .. }
                        if *a == area)
                });
                if !edits.is_empty() {
                    app.actions.push(Action::Apply(mg_edit::Command::new("Delete", edits)));
                }
            }
        }
    }
    if let Some(id) = make {
        id.run(app, ui.ctx());
    }
}

/// A named resource in the tree: its name, and its ResRef in parentheses
/// when asked for (Options › General).
fn named_label(name: &str, resref: ResRef, resrefs: bool) -> String {
    if resrefs { format!("{name} ({resref})") } else { name.to_string() }
}

/// The menu entry for a command that makes something new, from its name:
/// "Area Wizard" is "New Area…", "New Script" is "New Script…".
fn new_label(command: &str) -> String {
    let what = command.trim_end_matches(" Wizard");
    if what.starts_with("New ") { format!("{what}…") } else { format!("New {what}…") }
}

/// The command that makes a new resource of a tree group's types: the
/// Wizards menu's for it.
fn new_command(types: &[ResType]) -> Option<crate::commands::Id> {
    use crate::commands::Id;
    use mg_module::palette::BlueprintKind;
    Some(match *types.first()? {
        ResType::ARE => Id::AreaWizard,
        ResType::DLG => Id::NewConversation,
        ResType::NSS => Id::NewScript,
        ResType::UTC => Id::CreatureWizard,
        ResType::UTD => Id::Wizard(BlueprintKind::Door),
        ResType::UTE => Id::Wizard(BlueprintKind::Encounter),
        ResType::UTI => Id::Wizard(BlueprintKind::Item),
        ResType::UTM => Id::Wizard(BlueprintKind::Store),
        ResType::UTP => Id::Wizard(BlueprintKind::Placeable),
        ResType::UTS => Id::Wizard(BlueprintKind::Sound),
        ResType::UTT => Id::Wizard(BlueprintKind::Trigger),
        ResType::UTW => Id::Wizard(BlueprintKind::Waypoint),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_steps() {
        use egui::Key;
        let key = |name: &str, t| ResKey::parse(name, t).unwrap();
        let (a, b, c) = (key("a", ResType::ARE), key("b", ResType::ARE), key("c", ResType::NSS));
        let shown: Shown = vec![
            ("Areas", true, vec![a, b]),
            ("Conversations", false, vec![]),
            ("Scripts", false, vec![c]),
        ];
        let (areas, scripts) = (TreeAt::Group("Areas"), TreeAt::Group("Scripts"));
        let at = |r| Some(TreeAt::Resource(r));
        // Down and up, row by row, past a closed group's resources.
        assert_eq!(step(&shown, Some(areas), Key::ArrowDown), Step::To(TreeAt::Resource(a)));
        assert_eq!(step(&shown, at(b), Key::ArrowDown), Step::To(TreeAt::Group("Conversations")));
        assert_eq!(step(&shown, Some(scripts), Key::ArrowDown), Step::To(scripts));
        assert_eq!(step(&shown, Some(areas), Key::ArrowUp), Step::To(areas));
        assert_eq!(step(&shown, at(a), Key::End), Step::To(scripts));
        assert_eq!(step(&shown, at(b), Key::Home), Step::To(areas));
        assert_eq!(step(&shown, Some(areas), Key::PageDown), Step::To(scripts));
        // Right opens a group, then goes into it; Left the other way.
        assert_eq!(step(&shown, Some(scripts), Key::ArrowRight), Step::Fold("Scripts", true));
        assert_eq!(step(&shown, Some(areas), Key::ArrowRight), Step::To(TreeAt::Resource(a)));
        assert_eq!(step(&shown, at(b), Key::ArrowLeft), Step::To(areas));
        assert_eq!(step(&shown, Some(areas), Key::ArrowLeft), Step::Fold("Areas", false));
        // Enter opens a resource, and opens or closes a group.
        assert_eq!(step(&shown, at(a), Key::Enter), Step::Open(a));
        assert_eq!(step(&shown, Some(scripts), Key::Enter), Step::Fold("Scripts", true));
        // A row that is gone (its group closed, a filter typed): the first.
        assert_eq!(step(&shown, at(c), Key::ArrowDown), Step::To(areas));
        assert_eq!(step(&Vec::new(), None, Key::ArrowDown), Step::None);
    }

    #[test]
    fn each_group_of_resources_has_its_new_command() {
        use crate::commands::Id;
        let label = |types: &[ResType]| new_command(types).map(|id| new_label(&id.name()));
        assert_eq!(label(&[ResType::ARE]).as_deref(), Some("New Area…"));
        assert_eq!(label(&[ResType::UTC]).as_deref(), Some("New Creature…"));
        assert_eq!(label(&[ResType::UTM]).as_deref(), Some("New Store…"));
        assert_eq!(label(&[ResType::NSS]).as_deref(), Some("New Script…"));
        assert_eq!(label(&[ResType::DLG]).as_deref(), Some("New Conversation…"));
        // The journal and factions are one of each: nothing to make.
        assert_eq!(new_command(&[ResType::JRL, ResType::FAC]), None);
        // Every group of blueprints has one.
        for (_, types) in GROUPS.iter().filter(|(n, _)| *n != "Journal and factions") {
            assert!(new_command(types).is_some(), "{types:?}");
        }
        assert_eq!(
            new_command(&[ResType::UTI]),
            Some(Id::Wizard(mg_module::palette::BlueprintKind::Item))
        );
    }
}
