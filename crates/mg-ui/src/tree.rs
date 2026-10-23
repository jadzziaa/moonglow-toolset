//! The module contents tree (Aurora's left pane).

use std::collections::HashMap;
use std::sync::Arc;

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
    scaled or moved by a visual transform, have a script, a conversation, a trap, an \
    inventory or an animation switched on, or have variables, or the project's moonglow.toml \
    says so";

pub(crate) const GROUPS: &[(&str, &[ResType])] = &[
    ("Areas", &[ResType::ARE]),
    ("Conversations", &[ResType::DLG]),
    ("Scripts", &[ResType::NSS]),
    ("NUI windows", &[ResType::JUI]),
    ("Creatures", &[ResType::UTC]),
    ("Doors", &[ResType::UTD]),
    ("Encounters", &[ResType::UTE]),
    ("Items", &[ResType::UTI]),
    ("Placeables", &[ResType::UTP]),
    ("Sounds", &[ResType::UTS]),
    ("Stores", &[ResType::UTM]),
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
    /// What each of the tree's groups lists, as made for a revision of the
    /// workspace and the two options that shape it (areas by name, ResRefs
    /// beside names): not made again every frame.
    groups: Option<Groups>,
}

/// A group's resources in the order the tree shows them, and the names of
/// those shown by name.
type GroupList = (Vec<ResKey>, HashMap<ResKey, String>);

/// The groups' lists and what they were made for: the workspace's
/// revision and how many resources the module has (which also tells one
/// put in past the workspace's commands), areas by name, ResRefs beside
/// names.
#[derive(Debug)]
struct Groups {
    shape: (u64, usize, bool, bool),
    lists: Arc<Vec<GroupList>>,
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
/// and its objects.
pub(crate) type AreaContents = Vec<KindContents>;

/// The objects of one kind placed in an area, for the module tree.
pub(crate) struct KindContents {
    kind: mg_area::ObjectKind,
    /// Each object's row (by its place in the area's list).
    rows: Vec<ObjectRow>,
    /// The places in the area's list, in the order listed: by the rows'
    /// text, as Aurora sorts them (those alike in the area's order).
    order: Vec<usize>,
}

/// An object's row in the module tree.
struct ObjectRow {
    /// What the row says: its name, or for a waypoint its tag (waypoints
    /// often share a name, and their tags are what scripts and
    /// transitions go by: Aurora lists them by tag); the one, where the
    /// other is empty.
    text: String,
    /// Its name and tag, lowered, for the filter.
    sought: String,
    /// What the pointer over the row shows: the tag of a row that says
    /// the name, the name of one that says the tag.
    tip: String,
}

/// What is placed in an area, as the workspace has it now: every kind,
/// its objects by name (a waypoint by tag; either, where the other is
/// empty), sorted.
fn area_contents(ws: &mut Workspace, game: Option<&GameData>, area: ResRef) -> AreaContents {
    let Ok(git) = ws.doc(&ResKey::new(area, ResType::GIT)) else { return Vec::new() };
    mg_area::ObjectKind::ALL
        .into_iter()
        .map(|kind| {
            let rows: Vec<ObjectRow> = (git.root.list(kind.list()).unwrap_or(&[]).iter())
                .map(|o| {
                    let name = blueprint_name(game, o).trim().to_string();
                    let tag = crate::text::decode(o.string("Tag").unwrap_or_default());
                    let tag = tag.trim().to_string();
                    let sought = format!("{name}\n{tag}").to_lowercase();
                    let by_tag = kind == mg_area::ObjectKind::Waypoint;
                    let (text, other) = if by_tag { (tag, name) } else { (name, tag) };
                    let (text, tip) = match (text.is_empty(), other.is_empty()) {
                        (true, _) => (other, String::new()),
                        (false, true) => (text, String::new()),
                        (false, false) => {
                            let tip = if by_tag {
                                format!("Name: {other}")
                            } else {
                                format!("Tag: {other}")
                            };
                            (text, tip)
                        }
                    };
                    ObjectRow { text, sought, tip }
                })
                .collect();
            let mut order: Vec<usize> = (0..rows.len()).collect();
            let keys: Vec<String> = rows.iter().map(|r| r.text.to_lowercase()).collect();
            // (Stable: alike rows keep the area's order.)
            order.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
            KindContents { kind, rows, order }
        })
        .collect()
}

/// What was asked of an object in an area's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    /// Shown in the area's view.
    Go,
    Properties,
    /// Its variables, in the Variables window (as Aurora's list offers).
    Variables,
    /// In the model viewer, as it is placed.
    Preview,
    Copy,
    Delete,
    /// A plugin's command run on it: its place among the plugins'
    /// commands for menus.
    Plugin(usize),
}

/// Whether a placed object of `kind` has a picture for the model viewer.
pub(crate) fn previewable(kind: mg_area::ObjectKind) -> bool {
    use mg_area::ObjectKind as K;
    matches!(kind, K::Creature | K::Door | K::Item | K::Placeable)
}

/// An area's placed objects under its row of the module tree, kind by
/// kind as Aurora lists them. A click goes to the object in the area's
/// view; a double click opens its Properties; a right click has both, and
/// Copy, Delete and the plugins' commands for its kind. With a filter that
/// some of them match (lower case), only those are listed. Only the rows
/// in view are laid out (an area can hold thousands of a kind).
#[allow(clippy::too_many_arguments)]
fn contents_ui(
    ui: &mut Ui,
    area: ResRef,
    contents: &AreaContents,
    filter: &str,
    go: &mut Option<(ResRef, mg_area::ObjectKind, usize, Asked)>,
    // The object in hand (its row marked), and whether to go to it.
    in_hand: Option<(mg_area::ObjectKind, usize, bool)>,
    keys: &mut ContentKeys,
    // The plugins' commands for right-click menus.
    commands: &[crate::plugins::PluginCommand],
) -> bool {
    let mut went = false;
    let matches = |row: &ObjectRow| row.sought.contains(filter);
    let filtered = !filter.is_empty() && contents.iter().any(|c| c.rows.iter().any(&matches));
    ui.indent(("area-contents", area), |ui| {
        for KindContents { kind, rows, order } in contents {
            // (By their places in the area's list, in the order listed.)
            let shown: Vec<usize> = if filtered {
                order.iter().copied().filter(|&i| matches(&rows[i])).collect()
            } else {
                order.clone()
            };
            let title = if filtered {
                format!("{} ({} of {})", kind.label(), shown.len(), rows.len())
            } else {
                format!("{} ({})", kind.label(), rows.len())
            };
            if shown.is_empty() {
                if !filtered {
                    ui.weak(title);
                }
                continue;
            }
            let at_kind = keys.cursor == Some(TreeAt::Kind(area, *kind));
            let mut header = egui::CollapsingHeader::new(title)
                .id_salt(("area-kind", area, *kind))
                // (The keys' cursor on a kind shows as its row marked.)
                .show_background(at_kind);
            // The keys' cursor at one of its objects, by its place among
            // those listed.
            let at_object = match keys.cursor {
                Some(TreeAt::Object(a, k, i)) if a == area && k == *kind => Some(i),
                _ => None,
            };
            let sought =
                at_object.filter(|_| keys.moved).and_then(|i| shown.iter().position(|&s| s == i));
            // (What is looked for shows without opening each kind; so does
            // the object to go to.)
            let marked = in_hand.filter(|(k, ..)| k == kind);
            let goes = marked.filter(|m| m.2).and_then(|m| shown.iter().position(|&i| i == m.1));
            // (One the filter leaves out is not gone to.)
            if marked.is_some_and(|m| m.2) && goes.is_none() {
                went = true;
            }
            if filtered || goes.is_some() {
                header = header.open(Some(true));
            } else if let Some((_, open)) = keys.fold.filter(|f| f.0 == TreeAt::Kind(area, *kind)) {
                header = header.open(Some(open));
            }
            let clicked = &mut keys.clicked;
            let shown_header = header.show(ui, |ui| {
                // The rows in view, and room for those above and below.
                let font = egui::TextStyle::Button.resolve(ui.style());
                let spacing = ui.spacing();
                let row = (ui.fonts_mut(|f| f.row_height(&font)) + 2.0 * spacing.button_padding.y)
                    .max(spacing.interact_size.y);
                let step = row + spacing.item_spacing.y;
                let top = ui.cursor().top();
                // The row to go to, in view or not (rows out of view are
                // not laid out: by where it would be). The tree scrolls
                // only as far as it shows it: a row in view stays where it
                // is (under the pointer, for the second click of a double
                // click).
                if let Some(at) = goes {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.cursor().left(), top + at as f32 * step),
                        egui::vec2(1.0, row),
                    );
                    ui.scroll_to_rect(rect, None);
                    went = true;
                }
                // (And the keys' row, as near as it need be.)
                if let Some(at) = sought {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.cursor().left(), top + at as f32 * step),
                        egui::vec2(1.0, row),
                    );
                    ui.scroll_to_rect(rect, None);
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
                    let object = &rows[index];
                    let text =
                        if object.text.is_empty() { "(no name)" } else { object.text.as_str() };
                    let is_marked =
                        marked.is_some_and(|m| m.1 == index) || at_object == Some(index);
                    let how = "Click to go to it in the area; double-click for its Properties; \
                               right-click for more";
                    let tip = if object.tip.is_empty() {
                        how.to_string()
                    } else {
                        format!("{}\n{how}", object.tip)
                    };
                    let r = ui.selectable_label(is_marked, text).on_hover_text(tip);
                    if r.double_clicked() {
                        *go = Some((area, *kind, index, Asked::Properties));
                    } else if r.clicked() {
                        *go = Some((area, *kind, index, Asked::Go));
                    }
                    if r.clicked() {
                        *clicked = Some(TreeAt::Object(area, *kind, index));
                    }
                    r.context_menu(|ui| {
                        // Variables and Properties last, as in Aurora.
                        for (label, asked) in [
                            ("Go To", Asked::Go),
                            ("Preview", Asked::Preview),
                            ("Copy", Asked::Copy),
                            ("Delete", Asked::Delete),
                            ("Variables…", Asked::Variables),
                            ("Properties", Asked::Properties),
                        ] {
                            if asked == Asked::Preview && !previewable(*kind) {
                                continue;
                            }
                            if asked == Asked::Variables {
                                // (The plugins' commands for it, before.)
                                let kinds = [mg_plugin::Target::Object(*kind)];
                                if let Some(i) = crate::plugins::menu_commands(ui, commands, &kinds)
                                {
                                    *go = Some((area, *kind, index, Asked::Plugin(i)));
                                }
                                ui.separator();
                            }
                            if ui.button(label).clicked() {
                                *go = Some((area, *kind, index, asked));
                                ui.close();
                            }
                        }
                    });
                }
                room(ui, shown.len() - last);
            });
            if at_kind && keys.moved {
                shown_header.header_response.scroll_to_me(None);
            }
            if shown_header.header_response.clicked() {
                keys.clicked = Some(TreeAt::Kind(area, *kind));
            }
            keys.listed.push((*kind, shown_header.openness > 0.5, shown));
        }
    });
    went
}

/// The keys' part in an area's contents: where their cursor is, whether
/// it just moved, a kind they open or close; and back, the row clicked
/// and what was listed.
struct ContentKeys {
    cursor: Option<TreeAt>,
    moved: bool,
    fold: Option<(TreeAt, bool)>,
    clicked: Option<TreeAt>,
    listed: Vec<(mg_area::ObjectKind, bool, Vec<usize>)>,
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
    // Ctrl+F, with the keys the tree's and the pointer over its pane,
    // goes to the Filter; Down from the Filter goes into the rows.
    let find = crate::palette_view::keys_are(ui.ctx(), crate::palette_view::Keys::Tree)
        && app.tree_cursor.is_some()
        && !ui.ctx().egui_wants_keyboard_input()
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F));
    // (Escape there clears what is typed; again, or Down, back to the
    // rows.)
    let field_id = egui::Id::new("tree-filter-field");
    let left = crate::widgets::filter_keys(ui, field_id, &mut filter, find);
    // As the palettes' pane: the search across the pane, with what it
    // looks in said and a button to clear it; under it the list's own
    // small toolbar.
    // (The button that folds the pane away is on its edge: GitHub issue
    // 10, `Moonglow::pane_edge`.)
    ui.horizontal(|ui| {
        let clear = !filter.is_empty();
        let room = ui.available_width() - if clear { 26.0 } else { 0.0 };
        ui.add(
            egui::TextEdit::singleline(&mut filter)
                .id(field_id)
                .hint_text("🔍 Filter name, resref")
                .desired_width(room.max(60.0)),
        );
        if clear && ui.small_button("×").on_hover_text("Clear the filter").clicked() {
            filter.clear();
        }
    });
    if let Some(left) = left {
        if left == crate::widgets::FilterLeft::Down || app.tree_cursor.is_none() {
            app.tree_cursor = Some(TreeAt::Group(GROUPS[0].0));
        }
        app.tree_cursor_moved = true;
        crate::palette_view::give_keys(ui.ctx(), crate::palette_view::Keys::Tree);
    }
    app.buffers.insert(filter_id, filter.clone());
    // (As the names it is looked for in are lowered: every letter.)
    let filter = filter.to_lowercase();
    // Every group opened or closed at once (this frame). A filter typed
    // opens every group with a match, as it is typed: they can be closed
    // again after.
    let typed = crate::widgets::text_changed(ui, filter_id, &filter) && !filter.is_empty();
    let mut fold = None;
    ui.horizontal(|ui| {
        fold = crate::palette_view::fold_icons(ui, "group");
        let resources = app.ws.as_ref().map_or(0, |ws| ws.module.len());
        ui.weak(format!("{resources} resource{}", if resources == 1 { "" } else { "s" }))
            .on_hover_text("Resources in the module");
    });
    ui.separator();
    let fold = fold.or(typed.then_some(true));
    let mut rows = egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        // A name too long for the pane is cut short (the pointer over it
        // shows it whole), rather than widen the pane over the middle.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        tree_rows(app, ui, &filter, fold);
    });
    crate::widgets::home_and_end(ui, &mut rows);
    crate::palette_view::keys_outline(ui, rows.inner_rect, crate::palette_view::Keys::Tree);
}

/// The one object selected in the area in front, if it is another than
/// the tree shows: the tree then goes to it (its kind's list opened),
/// where its area is opened out there. An area left closed in the tree
/// is opened out to it, as in Aurora, unless Options › General says to
/// leave it closed (the tree then does not move).
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
        // The keys' cursor on an object's row (a row clicked) goes to the
        // one selected now: one row marked, the one selected last, however
        // it was selected (it stayed marked too, a builder's report).
        if matches!(app.tree_cursor, Some(TreeAt::Object(..))) {
            app.tree_cursor = selected.map(|(area, kind, index)| TreeAt::Object(area, kind, index));
        }
        if let Some((area, ..)) = selected.filter(|_| !app.settings.tree_keeps_areas_closed) {
            app.tree_reveal = Some((area, true));
        }
    }
}

/// A row of the module tree the arrow keys can be at: a group's, a
/// resource's, and under an area opened out a kind of object's and a
/// placed object's (by its place in the kind's list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeAt {
    Group(&'static str),
    Resource(ResKey),
    Kind(ResRef, mg_area::ObjectKind),
    Object(ResRef, mg_area::ObjectKind, usize),
}

/// The areas opened out, for the keys: each one's kinds as listed, whether
/// a kind is open, and its objects as listed.
type Outs = HashMap<ResRef, Vec<(mg_area::ObjectKind, bool, Vec<usize>)>>;

/// The rows the keys go through, in order: each with how deep it is and,
/// if it opens, whether it is open.
fn key_rows(shown: &Shown, outs: &Outs) -> Vec<(TreeAt, usize, Option<bool>)> {
    let mut rows = Vec::new();
    for (name, open, keys) in shown {
        rows.push((TreeAt::Group(name), 0, Some(*open)));
        for k in keys.iter().filter(|_| *open) {
            let out = outs.get(&k.resref).filter(|_| k.restype == ResType::ARE);
            let opens = (k.restype == ResType::ARE).then_some(out.is_some());
            rows.push((TreeAt::Resource(*k), 1, opens));
            for (kind, kind_open, objects) in out.into_iter().flatten() {
                rows.push((TreeAt::Kind(k.resref, *kind), 2, Some(*kind_open)));
                let inside = objects.iter().filter(|_| *kind_open);
                rows.extend(inside.map(|i| (TreeAt::Object(k.resref, *kind, *i), 3, None)));
            }
        }
    }
    rows
}

/// Where the tree's keys take the cursor from `at`, among the rows shown
/// (in order, each group's row before its resources'): the row to go to,
/// a group to open or close, or the resource to open.
#[derive(Debug, PartialEq)]
enum Step {
    To(TreeAt),
    /// Open or close the row (a group, an area, a kind of object).
    Fold(TreeAt, bool),
    Open(ResKey),
    /// Go to a placed object in its area's view.
    Go(ResRef, mg_area::ObjectKind, usize),
    None,
}

/// The tree's first row, where the keys begin.
pub(crate) fn first_row() -> TreeAt {
    TreeAt::Group(GROUPS[0].0)
}

/// The rows shown: each group, whether it is open, and its resources.
type Shown = Vec<(&'static str, bool, Vec<ResKey>)>;

/// The next row after `at` (round the end) whose name begins with
/// `letter`: a group's name, or a resource's name as its row shows it
/// (`named`: an area's or a blueprint's name where they are listed by
/// name; else its ResRef). A letter typed in the tree goes there.
fn seek(
    shown: &Shown,
    at: Option<TreeAt>,
    letter: char,
    named: &dyn Fn(&ResKey) -> Option<String>,
) -> Option<TreeAt> {
    let rows: Vec<(TreeAt, String)> = shown
        .iter()
        .flat_map(|(name, open, keys)| {
            let inside = keys
                .iter()
                .filter(move |_| *open)
                .map(|k| (TreeAt::Resource(*k), named(k).unwrap_or_else(|| k.resref.to_string())));
            std::iter::once((TreeAt::Group(name), name.to_string())).chain(inside)
        })
        .collect();
    let from = at.and_then(|at| rows.iter().position(|r| r.0 == at));
    let n = rows.len();
    (1..=n)
        .map(|by| from.map_or(by - 1, |from| (from + by) % n))
        .find(|i| rows[*i].1.chars().next().is_some_and(|c| c.eq_ignore_ascii_case(&letter)))
        .map(|i| rows[i].0)
}

fn step(shown: &Shown, outs: &Outs, at: Option<TreeAt>, key: egui::Key) -> Step {
    use egui::Key;
    let rows = key_rows(shown, outs);
    let Some(first) = rows.first().map(|r| r.0) else { return Step::None };
    let last = rows[rows.len() - 1].0;
    let index = at.and_then(|a| rows.iter().position(|r| r.0 == a));
    let by = |n: isize| match index {
        Some(i) => Step::To(rows[(i as isize + n).clamp(0, rows.len() as isize - 1) as usize].0),
        // (No row yet, or one that is gone: the first.)
        None => Step::To(first),
    };
    match (key, index.map(|i| (i, rows[i]))) {
        (Key::ArrowDown, _) => by(1),
        (Key::ArrowUp, _) => by(-1),
        (Key::PageDown, _) => by(10),
        (Key::PageUp, _) => by(-10),
        (Key::Home, _) => Step::To(first),
        (Key::End, _) => Step::To(last),
        // Right opens a row that opens, then goes to the first row in it;
        // Left closes one that is open, else goes to the row it is in.
        (Key::ArrowRight, Some((_, (at, _, Some(false))))) => Step::Fold(at, true),
        (Key::ArrowRight, Some((i, (_, depth, Some(true))))) => {
            rows.get(i + 1).filter(|r| r.1 > depth).map_or(Step::None, |r| Step::To(r.0))
        }
        (Key::ArrowLeft, Some((_, (at, _, Some(true))))) => Step::Fold(at, false),
        (Key::ArrowLeft, Some((i, (_, depth, _)))) => {
            rows[..i].iter().rev().find(|r| r.1 < depth).map_or(Step::None, |r| Step::To(r.0))
        }
        (Key::Enter, Some((_, (TreeAt::Resource(k), ..)))) => Step::Open(k),
        (Key::Enter, Some((_, (TreeAt::Object(area, kind, i), ..)))) => Step::Go(area, kind, i),
        (Key::Enter, Some((_, (at, _, Some(open))))) => Step::Fold(at, !open),
        _ => Step::None,
    }
}

fn tree_rows(app: &mut Moonglow, ui: &mut Ui, filter: &str, fold: Option<bool>) {
    follow_selection(app);
    // The rows shown, for the keys; a group the keys opened or closed.
    let mut shown: Shown = Vec::new();
    let mut outs = Outs::new();
    let folded = app.tree_fold.take();
    let cursor = app.tree_cursor;
    let cursor_moved = std::mem::take(&mut app.tree_cursor_moved);
    let mut clicked = None;
    // The resource row under the pointer.
    let mut pointed = None;
    // The plugins' commands for right-click menus, and one chosen on a
    // resource's row.
    let menu_commands = app.plugin_menu_commands();
    let mut plugin_on: Option<(usize, ResKey)> = None;
    let Some(ws) = &app.ws else { return };
    let revision = ws.revision();
    // Areas opened out whose contents are to be read (after the tree is
    // drawn), and the object clicked.
    let mut read = Vec::new();
    let mut go = None;
    // The object selected is still to be gone to, next frame.
    let mut object_waits = false;
    let (by_name, resrefs) = (app.settings.area_names, app.settings.name_resrefs);
    let filter = filter.to_string();
    if ui.selectable_label(false, "Module Properties").clicked() {
        app.actions.push(Action::OpenTab(Tab::ModuleProperties));
    }
    let mut open = None;
    let mut make = None;
    let mut listed = 0;
    // What the groups list: made when the module or the options that
    // shape the lists change (a module can hold thousands of a kind, and
    // every group was gathered, named and sorted each frame, open or not).
    let shape = (revision, ws.module.len(), by_name, resrefs);
    if app.area_names.groups.as_ref().is_none_or(|g| g.shape != shape) {
        let game = app.game.as_deref();
        let mut lists = Vec::with_capacity(GROUPS.len());
        for (_, types) in GROUPS {
            let mut keys: Vec<ResKey> =
                ws.module.keys().filter(|k| types.contains(&k.restype)).copied().collect();
            keys.sort();
            // Areas by name (Options › General): named, and in the names'
            // order; the filter finds a name or a ResRef.
            let mut names: HashMap<ResKey, String> = HashMap::new();
            let named = types == &[ResType::ARE]
                || types.iter().all(|t| mg_area::ObjectKind::from_restype(*t).is_some());
            if by_name && named {
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
            lists.push((keys, names));
        }
        app.area_names.groups = Some(Groups { shape, lists: Arc::new(lists) });
    }
    let lists = app.area_names.groups.as_ref().map(|g| g.lists.clone()).expect("made above");
    for ((name, types), (all, names)) in GROUPS.iter().zip(lists.iter()) {
        listed += all.len();
        // (An area opened out stays for what is placed in it, too.)
        let holds = |k: &ResKey| {
            k.restype == ResType::ARE
                && ui.data(|d| d.get_temp(egui::Id::new(("tree-area-open", k.resref))))
                    == Some(true)
                && app.area_contents.get(&k.resref).is_some_and(|(_, contents)| {
                    contents.iter().any(|c| c.rows.iter().any(|r| r.sought.contains(&filter)))
                })
        };
        let found: Option<Vec<ResKey>> = (!filter.is_empty()).then(|| {
            let finds = |k: &ResKey| {
                k.to_string().contains(&filter)
                    || names.get(k).is_some_and(|n| n.to_lowercase().contains(&filter))
                    || holds(k)
            };
            all.iter().copied().filter(finds).collect()
        });
        let keys: &[ResKey] = found.as_deref().unwrap_or(all);
        // What makes a new resource of the group (its wizard, or the New
        // window): on the group's and its resources' right-click menus.
        let new = new_command(types);
        let new_label = new.map(|id| new_label(&id.name()));
        let at_group = cursor == Some(TreeAt::Group(name));
        let listed_keys = keys.to_vec();
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
                    .or(folded.filter(|(g, _)| *g == TreeAt::Group(name)).map(|(_, open)| open))
                    .or(fold),
            )
            .show(ui, |ui| {
                for &k in keys {
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
                    // (Opened out or closed by the keys.)
                    if let Some((_, open)) =
                        folded.filter(|f| is_area && f.0 == TreeAt::Resource(k))
                    {
                        out = open;
                        ui.data_mut(|d| d.insert_temp(opened, out));
                    }
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
                        let r = ui
                            .add(egui::Button::selectable(in_hand || at_row, label).sense(sense));
                        // Who holds it (the project's claims table).
                        if is_area {
                            crate::claims::tree_mark(crate::claims::of(&app.ws), ui, k.resref);
                        }
                        r
                    });
                    // (A row out of sight is given this height.)
                    debug_assert!(
                        (row.response.rect.height() - height).abs() < 0.5,
                        "a tree row is {} high, not {height}",
                        row.response.rect.height()
                    );
                    let mut r = row.inner;
                    // (Opened out with its object marked: the object's row
                    // is brought into view instead, not the area's, which
                    // may be far above it.)
                    let object_here =
                        out && app.tree_object.is_some_and(|(a, ..)| a == k.resref);
                    if reveal.is_some() && object_here {
                        app.tree_object_pending = true;
                    } else if reveal.is_some() {
                        r.scroll_to_me(None);
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
                                let mut keys = ContentKeys {
                                    cursor,
                                    moved: cursor_moved,
                                    fold: folded,
                                    clicked: None,
                                    listed: Vec::new(),
                                };
                                let went = contents_ui(
                                    ui,
                                    k.resref,
                                    contents,
                                    &filter,
                                    &mut go,
                                    in_hand,
                                    &mut keys,
                                    &menu_commands,
                                );
                                if went {
                                    app.tree_object_pending = false;
                                } else if in_hand.is_some_and(|h| h.2) {
                                    // (Its kind's list is still opening:
                                    // gone to once its rows are laid out.)
                                    object_waits = true;
                                    ui.ctx().request_repaint();
                                }
                                clicked = keys.clicked.or(clicked);
                                outs.insert(k.resref, keys.listed);
                            }
                            None => {
                                read.push(k.resref);
                                outs.insert(k.resref, Vec::new());
                            }
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
                    // (With the keys in the tree, the key is read below.)
                    if r.hovered() {
                        pointed = Some(k);
                    }
                    if r.hovered()
                        && cursor.is_none()
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
                            if crate::model_view::previewable(k.restype)
                                && ui
                                    .button("Preview")
                                    .on_hover_text("See it in the 3D viewer")
                                    .clicked()
                            {
                                app.actions.push(Action::OpenTab(Tab::Model(k)));
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
                            let p = crate::claims::of(&app.ws);
                            crate::claims::menu_items(p, &mut app.actions, ui, k.resref);
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
                        } else if k.restype == ResType::JUI
                            && ui
                                .button("Rename…")
                                .on_hover_text(
                                    "With its settings and its scripts, and the scripts that open it",
                                )
                                .clicked()
                        {
                            app.actions.push(Action::RenameDialog(k));
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
                        // The plugins' commands for it (an area, a
                        // blueprint), before an area's Properties.
                        let target = match k.restype {
                            ResType::ARE => Some(mg_plugin::Target::Area),
                            t => mg_area::ObjectKind::from_restype(t).map(mg_plugin::Target::Object),
                        };
                        if let Some(t) = target
                            && let Some(i) = crate::plugins::menu_commands(ui, &menu_commands, &[t])
                        {
                            plugin_on = Some((i, k));
                        }
                        // An area's Properties last, as in Aurora.
                        if k.restype == ResType::ARE {
                            ui.separator();
                            if ui.button("Properties").clicked() {
                                app.actions.push(Action::OpenTab(Tab::AreaProperties(k.resref)));
                            }
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
            if !object_waits && !app.tree_object.is_some_and(|(area, ..)| read.contains(&area)) {
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
        crate::palette_view::give_keys(ui.ctx(), crate::palette_view::Keys::Tree);
    }
    if crate::palette_view::keys_are(ui.ctx(), crate::palette_view::Keys::Tree)
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
        // A letter or a digit typed, with the pointer over the tree (a
        // letter is a command elsewhere: W, A, S, D, Q and E in an area's
        // view): the next row that begins with it.
        let over = ui.rect_contains_pointer(ui.clip_rect());
        let typed = ui.input_mut(|i| {
            if !over || !i.modifiers.is_none() {
                return None;
            }
            let found = i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, .. } => {
                    let name = key.name();
                    let c = name.chars().next().filter(|_| name.len() == 1)?;
                    c.is_ascii_alphanumeric().then_some((*key, c))
                }
                _ => None,
            })?;
            i.events.retain(|e| match e {
                egui::Event::Key { key, .. } => *key != found.0,
                egui::Event::Text(t) => !t.eq_ignore_ascii_case(&found.1.to_string()),
                _ => true,
            });
            Some(found.1)
        });
        let named = |k: &ResKey| lists.iter().find_map(|(_, names)| names.get(k).cloned());
        if let Some(at) = typed.and_then(|c| seek(&shown, app.tree_cursor, c, &named)) {
            app.tree_cursor = Some(at);
            app.tree_cursor_moved = true;
            ui.ctx().request_repaint();
        }
        // F2 and Delete on the row under the pointer, else at the
        // cursor's resource, as its menu's Rename… and Delete… (both ask).
        let at_cursor = match app.tree_cursor {
            Some(TreeAt::Resource(k)) => Some(k),
            _ => None,
        };
        if let Some(k) = pointed.or(at_cursor) {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F2)) {
                app.actions.push(Action::RenameDialog(k));
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Delete)) {
                app.actions.push(Action::DeleteDialog(k));
            }
        }
        match pressed.map_or(Step::None, |key| step(&shown, &outs, app.tree_cursor, key)) {
            Step::To(at) => {
                app.tree_cursor = Some(at);
                app.tree_cursor_moved = true;
                ui.ctx().request_repaint();
            }
            Step::Fold(at, open) => {
                app.tree_fold = Some((at, open));
                ui.ctx().request_repaint();
            }
            Step::Open(k) => open = Some(k),
            Step::Go(area, kind, index) => go = Some((area, kind, index, Asked::Go)),
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
    // A plugin's command on a resource's row: on the area or blueprint.
    if let Some((i, k)) = plugin_on {
        let on = match k.restype {
            ResType::ARE => mg_plugin::Selected::Area(k.resref),
            _ => mg_plugin::Selected::Blueprint(k),
        };
        app.actions.push(crate::plugins::run_on(&menu_commands[i], vec![on]));
    }
    // An object in an area's list: gone to in the area's view, its
    // Properties opened, copied or deleted, or a plugin's command run on
    // it.
    if let Some((area, kind, index, asked)) = go {
        let git = ResKey::new(area, ResType::GIT);
        match asked {
            Asked::Plugin(i) => {
                let on = mg_plugin::Selected::Object { area, kind, index };
                app.actions.push(crate::plugins::run_on(&menu_commands[i], vec![on]));
            }
            Asked::Properties => {
                let path = mg_edit::GffPath::root().item(kind.list(), index);
                app.actions.push(Action::OpenTab(Tab::Instance { area, path }));
            }
            Asked::Variables => {
                let path = mg_edit::GffPath::root().item(kind.list(), index);
                crate::widgets::open_variables(app, git, path);
            }
            Asked::Preview => {
                let path = mg_edit::GffPath::root().item(kind.list(), index);
                app.actions.push(Action::OpenTab(Tab::InstanceModel { area, path }));
            }
            Asked::Go => {
                app.area_focus = Some((area, kind, index));
                // (The row clicked is the one marked: the tree stays as
                // it is, rather than go to the area's row and back.)
                app.tree_object = Some((area, kind, index));
                app.tree_object_pending = false;
                if let Some(view) = app.area_views.get_mut(&area) {
                    view.selection = vec![(kind, index)];
                }
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
                    // (As high above the ground as it stands, where its
                    // area's view knows the ground: it was pasted on it.)
                    let lift = app.area_views.get(&area).map_or(0.0, |v| v.lift_of(&o));
                    let objects = vec![(o, s, lift)];
                    app.object_clip = Some(crate::area_view::ObjectClip { objects, anchor });
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
        ResType::JUI => Id::NewNui,
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
        let outs = Outs::new();
        // Down and up, row by row, past a closed group's resources.
        assert_eq!(step(&shown, &outs, Some(areas), Key::ArrowDown), Step::To(TreeAt::Resource(a)));
        assert_eq!(
            step(&shown, &outs, at(b), Key::ArrowDown),
            Step::To(TreeAt::Group("Conversations"))
        );
        assert_eq!(step(&shown, &outs, Some(scripts), Key::ArrowDown), Step::To(scripts));
        assert_eq!(step(&shown, &outs, Some(areas), Key::ArrowUp), Step::To(areas));
        assert_eq!(step(&shown, &outs, at(a), Key::End), Step::To(scripts));
        assert_eq!(step(&shown, &outs, at(b), Key::Home), Step::To(areas));
        assert_eq!(step(&shown, &outs, Some(areas), Key::PageDown), Step::To(scripts));
        // Right opens a group, then goes into it; Left the other way.
        assert_eq!(step(&shown, &outs, Some(scripts), Key::ArrowRight), Step::Fold(scripts, true));
        assert_eq!(
            step(&shown, &outs, Some(areas), Key::ArrowRight),
            Step::To(TreeAt::Resource(a))
        );
        assert_eq!(step(&shown, &outs, at(b), Key::ArrowLeft), Step::To(areas));
        assert_eq!(step(&shown, &outs, Some(areas), Key::ArrowLeft), Step::Fold(areas, false));
        // Enter opens a resource, and opens or closes a group.
        assert_eq!(step(&shown, &outs, at(a), Key::Enter), Step::Open(a));
        // A letter: the next row that begins with it, round the end.
        let none = |_: &ResKey| None;
        assert_eq!(seek(&shown, Some(areas), 's', &none), Some(scripts));
        assert_eq!(seek(&shown, Some(scripts), 'A', &none), Some(areas));
        assert_eq!(seek(&shown, None, 'c', &none), Some(TreeAt::Group("Conversations")));
        assert_eq!(seek(&shown, Some(areas), 'z', &none), None);
        // Listed by name, a row is found by its name's letter, not its
        // ResRef's.
        let named = |k: &ResKey| (*k == b).then(|| "Zhentil Keep (b)".to_string());
        assert_eq!(seek(&shown, Some(areas), 'z', &named), at(b));
        assert_eq!(seek(&shown, at(a), 'b', &named), None);
        assert_eq!(step(&shown, &outs, Some(scripts), Key::Enter), Step::Fold(scripts, true));
        // A row that is gone (its group closed, a filter typed): the first.
        assert_eq!(step(&shown, &outs, at(c), Key::ArrowDown), Step::To(areas));
        assert_eq!(step(&Vec::new(), &outs, None, Key::ArrowDown), Step::None);
    }

    /// Right opens an area out and goes into it, to a kind and to its
    /// objects; Left comes back out, a row at a time (GitHub issue 12).
    #[test]
    fn the_keys_go_into_an_area_opened_out() {
        use egui::Key;
        use mg_area::ObjectKind::{Creature, Door};
        let key = |name: &str, t| ResKey::parse(name, t).unwrap();
        let (a, b) = (key("a", ResType::ARE), key("b", ResType::ARE));
        let shown: Shown = vec![("Areas", true, vec![a, b])];
        let (area, next) = (TreeAt::Resource(a), TreeAt::Resource(b));
        let (creatures, doors) = (TreeAt::Kind(a.resref, Creature), TreeAt::Kind(a.resref, Door));
        let object = |i| TreeAt::Object(a.resref, Creature, i);
        // Closed: Right opens it out; Left goes to its group.
        let mut outs = Outs::new();
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowRight), Step::Fold(area, true));
        assert_eq!(
            step(&shown, &outs, Some(area), Key::ArrowLeft),
            Step::To(TreeAt::Group("Areas"))
        );
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowDown), Step::To(next));
        // Opened out, its kinds closed.
        outs.insert(a.resref, vec![(Creature, false, vec![0, 2]), (Door, false, vec![0])]);
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowRight), Step::To(creatures));
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowDown), Step::To(creatures));
        assert_eq!(step(&shown, &outs, Some(creatures), Key::ArrowDown), Step::To(doors));
        assert_eq!(step(&shown, &outs, Some(doors), Key::ArrowDown), Step::To(next));
        assert_eq!(
            step(&shown, &outs, Some(creatures), Key::ArrowRight),
            Step::Fold(creatures, true)
        );
        assert_eq!(step(&shown, &outs, Some(creatures), Key::ArrowLeft), Step::To(area));
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowLeft), Step::Fold(area, false));
        // A kind open: into its objects (those listed), and back.
        outs.insert(a.resref, vec![(Creature, true, vec![0, 2]), (Door, false, vec![0])]);
        assert_eq!(step(&shown, &outs, Some(creatures), Key::ArrowRight), Step::To(object(0)));
        assert_eq!(step(&shown, &outs, Some(object(0)), Key::ArrowDown), Step::To(object(2)));
        assert_eq!(step(&shown, &outs, Some(object(2)), Key::ArrowDown), Step::To(doors));
        assert_eq!(step(&shown, &outs, Some(object(2)), Key::ArrowRight), Step::None);
        assert_eq!(step(&shown, &outs, Some(object(2)), Key::ArrowLeft), Step::To(creatures));
        assert_eq!(
            step(&shown, &outs, Some(creatures), Key::ArrowLeft),
            Step::Fold(creatures, false)
        );
        // Enter goes to an object in its area, and opens or closes a kind.
        assert_eq!(
            step(&shown, &outs, Some(object(2)), Key::Enter),
            Step::Go(a.resref, Creature, 2)
        );
        assert_eq!(step(&shown, &outs, Some(doors), Key::Enter), Step::Fold(doors, true));
        assert_eq!(step(&shown, &outs, Some(area), Key::Enter), Step::Open(a));
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
