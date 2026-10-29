//! The module contents tree (Aurora's left pane).

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use egui::Ui;
use mg_core::text::Needle;
use mg_core::{ResRef, ResType};
use mg_edit::Workspace;
use mg_gff::Gff;
use mg_resman::ResKey;
use mg_rules::{GameData, ObjectKind};

use crate::{Action, Moonglow, Tab};

/// What Make Placeables Static does, for its menu items.
const STATIC_TIP: &str = "Sets Static on the placeables that lose nothing by it: the game loads and \
    draws static placeables more cheaply. Left dynamic: those that are Useable, are tilted, \
    scaled or moved by a visual transform, have a script, a conversation, a trap, an \
    inventory or an animation switched on, or have variables, or the project's moonglow.toml \
    says so";

/// The tree's groups: the module's own kinds of resource, then each
/// kind of blueprint (`mg_rules::ObjectKind`'s order and names), then the
/// journal and factions.
pub(crate) const GROUPS: &[(&str, &[ResType])] = &{
    let mut groups: [(&str, &[ResType]); 14] = [("", &[]); 14];
    groups[0] = ("Areas", &[ResType::ARE]);
    groups[1] = ("Conversations", &[ResType::DLG]);
    groups[2] = ("Scripts", &[ResType::NSS]);
    groups[3] = ("NUI windows", &[ResType::JUI]);
    let mut i = 0;
    while i < ObjectKind::ALL.len() {
        let kind = ObjectKind::ALL[i];
        groups[4 + i] = (kind.label(), kind.restypes());
        i += 1;
    }
    groups[13] = ("Journal and factions", &[ResType::JRL, ResType::FAC]);
    groups
};

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
pub(crate) fn blueprint_name(game: Option<&GameData>, root: &mg_gff::Struct) -> String {
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

/// A placed object of an area, by its kind and its place in its kind's
/// list.
pub(crate) type Placed = (mg_area::ObjectKind, usize);

/// What an area's placed objects were read from, for the module tree's
/// copy of them: the open document at a revision of the workspace (it is
/// being edited), else the stored GIT's data (where it is and its length:
/// replaced data is elsewhere). They are read again when it changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Read {
    Revision(u64),
    Data(usize, usize),
}

/// What an area's placed objects would be read from now ([`Read`]).
fn read_of(ws: &Workspace, area: ResRef) -> Read {
    let git = ResKey::new(area, ResType::GIT);
    if ws.loaded(&git).is_some() {
        return Read::Revision(ws.revision());
    }
    ws.module.get(&git).map_or(Read::Data(0, 0), |d| Read::Data(d.as_ptr() as usize, d.len()))
}

/// What is placed in an area, as the workspace has it now: every kind,
/// its objects by name (a waypoint by tag; either, where the other is
/// empty), sorted. (A GIT not open is read, not opened: the Filter reads
/// every area's.)
fn area_contents(ws: &Workspace, game: Option<&GameData>, area: ResRef) -> AreaContents {
    let key = ResKey::new(area, ResType::GIT);
    let stored;
    let git = match ws.loaded(&key) {
        Some(git) => git,
        None => {
            let Some(Ok(git)) = ws.module.get(&key).map(Gff::read) else { return Vec::new() };
            stored = git;
            &stored
        }
    };
    mg_area::ObjectKind::ALL
        .into_iter()
        .map(|kind| {
            let rows: Vec<ObjectRow> = (git.root.list(kind.list()).unwrap_or(&[]).iter())
                .map(|o| {
                    let name = blueprint_name(game, o).trim().to_string();
                    let tag = crate::text::decode(o.string("Tag").unwrap_or_default());
                    let tag = tag.trim().to_string();
                    let sought = mg_core::text::fold(&format!("{name}\n{tag}"));
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
            // (Case and accents aside: `Árbol` beside `arbol`, not after `z`.)
            let keys: Vec<_> = rows.iter().map(|r| mg_core::text::sort_key(&r.text)).collect();
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
    /// Written out as a blueprint of its own, in an archive (Export as
    /// Blueprint…).
    Export,
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
/// Copy, Delete, Export as Blueprint… and the plugins' commands for its
/// kind. Ctrl and Shift with a click choose several (`chosen`, marked),
/// which a right click on one of them acts on together. With a filter
/// that some of them match (lower case), only those are listed. Only the
/// rows in view are laid out (an area can hold thousands of a kind).
#[allow(clippy::too_many_arguments)]
fn contents_ui(
    ui: &mut Ui,
    area: ResRef,
    contents: &AreaContents,
    filter: &Needle,
    go: &mut Option<(ResRef, mg_area::ObjectKind, usize, Asked)>,
    // The object in hand (its row marked), and whether to go to it.
    in_hand: Option<(mg_area::ObjectKind, usize, bool)>,
    // The objects chosen together, when there are several.
    chosen: Option<&BTreeSet<Placed>>,
    keys: &mut ContentKeys,
    // The plugins' commands for right-click menus.
    commands: &[crate::plugins::PluginCommand],
) -> bool {
    // (Of one kind: Properties opens them together.)
    let one_kind =
        chosen.is_some_and(|c| c.iter().all(|(k, _)| Some(k) == c.first().map(|f| &f.0)));
    let mut went = false;
    let matches = |row: &ObjectRow| filter.finds_folded(&row.sought);
    let filtered = !filter.is_empty() && contents.iter().any(|c| c.rows.iter().any(&matches));
    // (Collapse All closed its kinds too, as they are next shown.)
    let closing = ui.data_mut(|d| {
        d.remove_temp::<bool>(egui::Id::new(("tree-area-kinds-closed", area))).is_some()
    });
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
            } else if closing {
                header = header.open(Some(false));
            }
            let (clicked, hovered) = (&mut keys.clicked, &mut keys.hovered);
            let (picked, together) = (&mut keys.picked, &mut keys.together);
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
                    reveal_into_view(ui, rect);
                    went = true;
                }
                // (And the keys' row, as near as it need be.)
                if let Some(at) = sought {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.cursor().left(), top + at as f32 * step),
                        egui::vec2(1.0, row),
                    );
                    bring_into_view(ui, rect);
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
                    let text = if object.text.is_empty() { NO_NAME } else { object.text.as_str() };
                    let several = chosen.is_some_and(|c| c.contains(&(*kind, index)));
                    let is_marked =
                        marked.is_some_and(|m| m.1 == index) || at_object == Some(index) || several;
                    let how = "Click to go to it in the area; double-click for its Properties; \
                               Ctrl or Shift and a click to choose several; right-click for more";
                    let tip = if object.tip.is_empty() {
                        how.to_string()
                    } else {
                        format!("{}\n{how}", object.tip)
                    };
                    let r = ui.selectable_label(is_marked, text).on_hover_text(tip);
                    let m = ui.input(|i| i.modifiers);
                    let (command, shift) = (m.command || m.ctrl, m.shift);
                    if r.clicked() && (command || shift) {
                        // Chosen with the others (after the rows are laid
                        // out: `tree_rows`).
                        *picked = Some((TreeAt::Object(area, *kind, index), command, shift));
                    } else if r.double_clicked() {
                        *go = Some((area, *kind, index, Asked::Properties));
                    } else if r.clicked() {
                        *go = Some((area, *kind, index, Asked::Go));
                    }
                    if r.clicked() {
                        *clicked = Some(TreeAt::Object(area, *kind, index));
                    }
                    if r.hovered() {
                        *hovered = Some(TreeAt::Object(area, *kind, index));
                    }
                    r.context_menu(|ui| {
                        // Several chosen, this one among them: what is
                        // done to them together.
                        if several {
                            let n = chosen.map_or(0, BTreeSet::len);
                            ui.weak(format!("{n} objects chosen"));
                            ui.separator();
                        }
                        // Variables and Properties last, as in Aurora.
                        for (label, asked) in [
                            ("Go To", Asked::Go),
                            ("Preview", Asked::Preview),
                            ("Copy", Asked::Copy),
                            ("Delete", Asked::Delete),
                            ("Export as Blueprint…", Asked::Export),
                            ("Variables…", Asked::Variables),
                            ("Properties", Asked::Properties),
                        ] {
                            if asked == Asked::Preview && (several || !previewable(*kind)) {
                                continue;
                            }
                            if asked == Asked::Variables {
                                // (The plugins' commands for it, or for
                                // the kinds of those chosen, before.)
                                let mut kinds = vec![mg_plugin::Target::Object(*kind)];
                                if several {
                                    let all = chosen.into_iter().flatten();
                                    kinds =
                                        all.map(|(k, _)| mg_plugin::Target::Object(*k)).collect();
                                    kinds.dedup();
                                }
                                if let Some(i) = crate::plugins::menu_commands(ui, commands, &kinds)
                                {
                                    *go = Some((area, *kind, index, Asked::Plugin(i)));
                                    *together = several;
                                }
                                ui.separator();
                            }
                            // (One at a time: its variables, and a
                            // blueprint of it; Properties of several of one
                            // kind.)
                            let one = matches!(asked, Asked::Export | Asked::Variables);
                            if several && (one || (asked == Asked::Properties && !one_kind)) {
                                continue;
                            }
                            let mut b = ui.button(label);
                            if asked == Asked::Export {
                                b = b.on_hover_text(
                                    "Write it into an archive (.erf) as a blueprint of its own, \
                                     with the items it holds, as Add to Palette makes one: for \
                                     another module's Import",
                                );
                            }
                            if b.clicked() {
                                *go = Some((area, *kind, index, asked));
                                *together = several;
                                ui.close();
                            }
                        }
                    });
                }
                room(ui, shown.len() - last);
            });
            if at_kind && keys.moved {
                bring_into_view(ui, shown_header.header_response.rect);
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
/// it just moved, a kind they open or close; and back, the row clicked,
/// the object's row under the pointer and what was listed.
struct ContentKeys {
    cursor: Option<TreeAt>,
    moved: bool,
    fold: Option<(TreeAt, bool)>,
    clicked: Option<TreeAt>,
    hovered: Option<TreeAt>,
    /// A row clicked with Ctrl (`.1`) or Shift (`.2`).
    picked: Option<(TreeAt, bool, bool)>,
    /// What was asked from the menu of a row among several chosen: for
    /// them all.
    together: bool,
    listed: Vec<(mg_area::ObjectKind, bool, Vec<usize>)>,
}

/// The gap between rows (taken off a block of rows left out, which is
/// followed by one itself).
fn spacing_gap(ui: &Ui) -> f32 {
    ui.spacing().item_spacing.y
}

/// Brings a row (`rect`) into view up or down only: the tree scrolled
/// sideways stays so (egui's own goes to a long row's start).
fn bring_into_view(ui: &Ui, rect: egui::Rect) {
    let x = ui.clip_rect().center().x;
    ui.scroll_to_rect(egui::Rect::from_x_y_ranges(x..=x, rect.y_range()), None);
}

/// Brings a row gone to from elsewhere (a tab chosen, an object selected
/// in the area) into view: one out of sight to the middle of the pane,
/// rather than its foot or top (where the tree scrolled only as far as it
/// must, a row brought up from below sat at the foot, the rows after it
/// out of sight); one in sight stays where it is (under the pointer, for
/// a double click's second click).
fn reveal_into_view(ui: &Ui, rect: egui::Rect) {
    let clip = ui.clip_rect();
    if clip.y_range().contains(rect.top()) && clip.y_range().contains(rect.bottom()) {
        return;
    }
    let x = clip.center().x;
    let to = egui::Rect::from_x_y_ranges(x..=x, rect.y_range());
    ui.scroll_to_rect(to, Some(egui::Align::Center));
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
    // As the palettes' pane: the search across the pane, with what it
    // looks in said and a button to clear it; under it the list's own
    // small toolbar. Find's keys (Ctrl+F), with the keys the tree's, go to
    // it; Escape there clears what is typed; again, back to the rows, and
    // Down to the first row it finds.
    // (The button that folds the pane away is on its edge: GitHub issue
    // 10, `Moonglow::pane_edge`.)
    let here = crate::palette_view::keys_are(ui.ctx(), crate::palette_view::Keys::Tree)
        && app.tree_cursor.is_some();
    let left = crate::widgets::SearchField::new(egui::Id::new("tree-filter-field"), &mut filter)
        .hint("🔍 Filter name, resref")
        .clear_tip("Clear the filter")
        .find_when(here)
        .down()
        .show(ui)
        .left;
    if let Some(left) = left {
        if left == crate::widgets::FilterLeft::Down {
            app.tree_to_match = true;
        } else if app.tree_cursor.is_none() {
            app.tree_cursor = Some(first_row());
        }
        app.tree_cursor_moved = true;
        crate::palette_view::give_keys(ui.ctx(), crate::palette_view::Keys::Tree);
    }
    app.buffers.insert(filter_id, filter.clone());
    // (Every word, case and accents aside: `arbol` finds `Árbol`.)
    let filter = Needle::new(&filter);
    // Every group opened or closed at once (this frame). A filter typed
    // opens every group with a match, as it is typed: they can be closed
    // again after.
    let typed = crate::widgets::text_changed(ui, filter_id, filter.as_str()) && !filter.is_empty();
    let mut fold = None;
    ui.horizontal(|ui| {
        fold = crate::palette_view::fold_icons(ui, "group", &app.keymap);
        let resources = app.ws.as_ref().map_or(0, |ws| ws.module.len());
        ui.weak(format!("{resources} resource{}", if resources == 1 { "" } else { "s" }))
            .on_hover_text("Resources in the module");
    });
    ui.separator();
    // (Their keys too, while the keys are the tree's.)
    let keyed = crate::palette_view::keys_are(ui.ctx(), crate::palette_view::Keys::Tree)
        && !ui.ctx().egui_wants_keyboard_input();
    if keyed {
        fold = fold.or_else(|| crate::palette_view::fold_keys(ui, &app.keymap));
    }
    // Collapse All closes every area opened out to its objects too, and
    // the kinds under it (Expand All leaves them: an area's objects can be
    // hundreds of rows).
    if fold == Some(false)
        && let Some(ws) = app.ws.as_ref()
    {
        for area in ws.module.keys_of(ResType::ARE).map(|k| k.resref) {
            ui.data_mut(|d| {
                d.insert_temp(egui::Id::new(("tree-area-open", area)), false);
                d.insert_temp(egui::Id::new(("tree-area-kinds-closed", area)), true);
            });
        }
    }
    let fold = fold.or(typed.then_some(true));
    let mut rows = egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        // A name too long for the pane goes on past it, and the tree
        // scrolls sideways to it, as Aurora's does (a builder's report:
        // only widening the pane showed long names whole).
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        let left = ui.min_rect().left();
        tree_rows(app, ui, &filter, fold);
        // (Rows out of sight are not laid out: the widest drawn since the
        // filter changed keeps the width scrolled through, which then
        // doesn't change as rows go by.)
        let widest = egui::Id::new(("tree-widest", filter.as_str()));
        let drawn = ui.min_rect().right() - left;
        let width = ui.data_mut(|d| {
            let w = d.get_temp_mut_or(widest, 0.0_f32);
            *w = w.max(drawn);
            *w
        });
        ui.expand_to_include_x(left + width);
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
    let seen = app.palette.area.and_then(|area| {
        let view = app.area_views.get(&area)?;
        Some((area, view.selection.clone()))
    });
    // Several selected there: chosen in the tree too, each row marked
    // (as the selection changes: those chosen in the tree of an area not
    // shown stay).
    if seen != app.tree_seen_selection {
        if let Some((area, selection)) = &seen {
            let several = selection.len() > 1;
            app.tree_objects = several.then(|| (*area, selection.iter().copied().collect()));
        }
        app.tree_seen_selection = seen.clone();
    }
    let selected = seen.and_then(|(area, selection)| match selection.as_slice() {
        [(kind, index)] => Some((area, *kind, *index)),
        _ => None,
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
            app.tree_reveal = Some((ResKey::new(area, ResType::ARE), true));
        }
    }
}

/// Whether the module tree lists a resource: the module's, of a kind of
/// its groups.
pub(crate) fn listed(app: &Moonglow, key: &ResKey) -> bool {
    GROUPS.iter().any(|(_, types)| types.contains(&key.restype))
        && app.ws.as_ref().is_some_and(|ws| ws.module.contains(key))
}

/// The module tree goes to what a tab chosen shows (its row marked, its
/// group opened, the row brought into view), however it was chosen: a
/// click on it, the keys, opened. The keys stay with the tab. Where
/// Options › General says the tree stays, only an area's row is marked,
/// as the one in front (Aurora marks it so, and its tree follows nothing
/// else).
pub(crate) fn reveal_tab(app: &mut Moonglow, tab: &Tab) {
    // Module Properties: its row, at the top.
    if *tab == Tab::ModuleProperties {
        if !app.settings.tree_stays {
            app.tree_reveal = Some((module_row(), false));
            app.tree_selected.clear();
        }
        return;
    }
    let keys: Vec<ResKey> = tab.tree_keys().into_iter().filter(|k| listed(app, k)).collect();
    let Some(&key) = keys.first() else { return };
    if !app.settings.tree_stays {
        app.tree_reveal = Some((key, false));
        // A tab of several (Properties of several blueprints or areas):
        // each of them chosen, the first brought into view.
        app.tree_selected = keys.iter().copied().collect();
        app.tree_anchor = Some(key);
    } else if key.restype == ResType::ARE && keys.len() == 1 {
        app.tree_marked = Some(key);
        app.tree_area = Some(key.resref);
    }
}

/// What marks the tree's Module Properties row ([`Moonglow::tree_marked`],
/// [`Moonglow::tree_reveal`]): the module's `module.ifo`, which the tree
/// lists no other way.
pub(crate) fn module_row() -> ResKey {
    ResKey::new(ResRef::from_str("module").expect("a valid ResRef"), ResType::IFO)
}

/// The tab chosen since the tree last looked, which it goes to
/// ([`reveal_tab`]): the one in front of the pane given the focus (a
/// window's, or the middle's), else one come to the front of its pane
/// (opened, or reached by the keys).
pub(crate) fn follow_front_tab(app: &mut Moonglow) {
    let front = |leaf: &egui_dock::LeafNode<Tab>| leaf.tabs.get(leaf.active.0).cloned();
    let fronts: Vec<Tab> = (app.dock.iter_leaves())
        .filter_map(|(_, leaf)| front(leaf))
        .filter(|t| *t != Tab::Palette)
        .collect();
    let focused = (app.dock.focused_leaf())
        .and_then(|p| app.dock.leaf(p).ok())
        .and_then(front)
        .filter(|t| *t != Tab::Palette);
    let (was_focused, was) = std::mem::take(&mut app.tree_front);
    let chosen = if focused.is_some() && focused != was_focused {
        focused.clone()
    } else {
        fronts.iter().find(|t| !was.contains(t)).cloned()
    };
    app.tree_front = (focused, fronts);
    if let Some(tab) = chosen {
        reveal_tab(app, &tab);
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
fn key_rows(shown: &Shown<'_>, outs: &Outs) -> Vec<(TreeAt, usize, Option<bool>)> {
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

/// The rows shown: each group, whether it is open, and its resources (the
/// group's list as made, borrowed, unless the Filter keeps some: not
/// copied every frame).
type Shown<'a> = Vec<(&'static str, bool, std::borrow::Cow<'a, [ResKey]>)>;

/// The row the letters typed in quick succession (`typed`) go to, as in
/// the conversation tree ([`crate::dialog_view::type_ahead`]): one
/// letter, or the same again and again, the next row after `at` beginning
/// with it; else the first from `at` on (itself first, round the end)
/// whose name begins with them all, case and accents aside. The rows are
/// those in sight, in order ([`key_rows`]): an area opened out has its
/// kinds' rows, and a kind open its placed objects'. A row's name is what
/// it shows (`text`: a group's or a kind's name without its count; a
/// resource's, an area's or a blueprint's name where they are listed by
/// name, else its ResRef; an object's name, or a waypoint's tag).
fn seek(
    shown: &Shown<'_>,
    outs: &Outs,
    at: Option<TreeAt>,
    typed: &str,
    text: &dyn Fn(TreeAt) -> String,
) -> Option<TreeAt> {
    let rows: Vec<TreeAt> = key_rows(shown, outs).into_iter().map(|r| r.0).collect();
    // (No row: from the first, itself first.)
    let from = at.and_then(|at| rows.iter().position(|r| *r == at));
    let from = from.unwrap_or(rows.len().saturating_sub(1));
    let texts: Vec<String> = rows.iter().map(|r| mg_core::text::fold(&text(*r))).collect();
    let to = crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))?;
    Some(rows[to])
}

/// A row's text as the module tree shows it, for the letters typed
/// ([`seek`]): `named` gives a resource's name where it is listed by name,
/// `contents` an area's placed objects as read.
fn row_text(
    at: TreeAt,
    named: &dyn Fn(&ResKey) -> Option<String>,
    contents: &HashMap<ResRef, (Read, AreaContents)>,
) -> String {
    match at {
        TreeAt::Group(name) => name.to_string(),
        TreeAt::Resource(k) => named(&k).unwrap_or_else(|| k.resref.to_string()),
        TreeAt::Kind(_, kind) => kind.label().to_string(),
        TreeAt::Object(area, kind, index) => {
            let row = (contents.get(&area).into_iter())
                .flat_map(|(_, c)| c.iter())
                .find(|c| c.kind == kind)
                .and_then(|c| c.rows.get(index));
            match row {
                Some(r) if !r.text.is_empty() => r.text.clone(),
                _ => NO_NAME.to_string(),
            }
        }
    }
}

/// What a placed object's row says that has no name (nor tag).
const NO_NAME: &str = "(no name)";

fn step(shown: &Shown<'_>, outs: &Outs, at: Option<TreeAt>, key: egui::Key) -> Step {
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

/// Where the keys' cursor (`cursor`) is once the placed object `deleted`
/// is deleted: at it, on the row after it among its kind's (else the one
/// before, else its kind's row); at another of its kind's, at that one
/// still (the objects after it move up their area's list).
fn past_deleted(shown: &Shown<'_>, outs: &Outs, cursor: TreeAt, deleted: TreeAt) -> TreeAt {
    let TreeAt::Object(area, kind, index) = deleted else { return cursor };
    let moved_up = |at: TreeAt| match at {
        TreeAt::Object(a, k, i) if a == area && k == kind && i > index => {
            TreeAt::Object(a, k, i - 1)
        }
        at => at,
    };
    if cursor != deleted {
        return moved_up(cursor);
    }
    let rows = key_rows(shown, outs);
    let at = rows.iter().position(|r| r.0 == deleted);
    let sibling = |r: &&(TreeAt, usize, Option<bool>)| matches!(r.0, TreeAt::Object(a, k, _) if a == area && k == kind);
    let next = at.and_then(|at| rows.get(at + 1)).filter(sibling);
    let before = at.and_then(|at| at.checked_sub(1)).and_then(|at| rows.get(at)).filter(sibling);
    next.or(before).map_or(TreeAt::Kind(area, kind), |r| moved_up(r.0))
}

fn tree_rows(app: &mut Moonglow, ui: &mut Ui, filter: &Needle, fold: Option<bool>) {
    follow_selection(app);
    // A group the keys opened or closed (the rows shown, for the keys,
    // once the groups' lists are in hand).
    let mut outs = Outs::new();
    let folded = app.tree_fold.take();
    let cursor = app.tree_cursor;
    let cursor_moved = std::mem::take(&mut app.tree_cursor_moved);
    let mut clicked = None;
    // The resource row under the pointer, and the placed object's.
    let mut pointed = None;
    let mut pointed_object = None;
    // The plugins' commands for right-click menus, and one chosen on a
    // resource's row.
    let menu_commands = app.plugin_menu_commands();
    let mut plugin_on: Option<(usize, ResKey)> = None;
    // One chosen on a row among several chosen: on them all.
    let mut plugin_several: Option<usize> = None;
    let Some(ws) = &app.ws else { return };
    let revision = ws.revision();
    // Areas opened out whose contents are to be read (after the tree is
    // drawn), and the object clicked.
    let mut read = Vec::new();
    let mut go = None;
    // The object selected is still to be gone to, next frame.
    let mut object_waits = false;
    let (by_name, resrefs) = (app.settings.area_names, app.settings.name_resrefs);
    let filter = filter.clone();
    // (Marked while its tab is the one chosen.)
    let module = ui.selectable_label(app.tree_marked == Some(module_row()), "Module Properties");
    if app.tree_reveal.is_some_and(|(k, _)| k == module_row()) {
        app.tree_marked = Some(module_row());
        reveal_into_view(ui, module.rect);
    }
    if module.clicked() {
        app.actions.push(Action::OpenTab(Tab::ModuleProperties));
    }
    // A row clicked (with Ctrl or Shift: chosen with others), resolved
    // once every row is laid out.
    let mut picked: Option<(TreeAt, bool, bool)> = None;
    let mut together = false;
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
                keys.sort_by_cached_key(|k| (mg_core::text::sort_key(&shown(k)), *k));
            }
            lists.push((keys, names));
        }
        app.area_names.groups = Some(Groups { shape, lists: Arc::new(lists) });
    }
    let lists = app.area_names.groups.as_ref().map(|g| g.lists.clone()).expect("made above");
    let mut shown: Shown<'_> = Vec::new();
    // The resource to bring into view is in a group still opening: gone
    // to when it is open (its rows are where they will be then).
    let mut opening = false;
    // The resources the filter finds by their own ResRef or name (not
    // only by what is placed in them).
    let mut matched: Vec<ResKey> = Vec::new();
    // A tab chosen while the Filter has something typed: its rows (a tab
    // of several's, each) are listed under it too, for as long as it has
    // the same typed (the tree goes to them; the Filter stays).
    if let Some((k, _)) = app.tree_reveal.filter(|_| !filter.is_empty()) {
        let mut keys: Vec<ResKey> = app.tree_selected.iter().copied().collect();
        if !keys.contains(&k) {
            keys.push(k);
        }
        app.tree_kept = Some((keys, filter.as_str().to_string()));
    }
    for ((name, types), (all, names)) in GROUPS.iter().zip(lists.iter()) {
        listed += all.len();
        // (An area stays for what is placed in it, too, opened out or not:
        // every area's objects are read while the Filter has something
        // typed, below.)
        let holds = |k: &ResKey| {
            k.restype == ResType::ARE
                && app.area_contents.get(&k.resref).is_some_and(|(_, contents)| {
                    contents.iter().any(|c| c.rows.iter().any(|r| filter.finds_folded(&r.sought)))
                })
        };
        // (The rows of a tab chosen while the Filter has this typed stay
        // in sight under it: the tree goes to them.)
        let kept = |k: &ResKey| {
            (app.tree_kept.as_ref())
                .is_some_and(|(keys, f)| f == filter.as_str() && keys.contains(k))
        };
        // (Every word in its ResRef or its name, in any order.)
        let own = |k: &ResKey| {
            let name = names.get(k).map(|n| mg_core::text::fold(n)).unwrap_or_default();
            filter.finds_in(&[&k.to_string().to_ascii_lowercase(), &name])
        };
        let found: Option<Vec<ResKey>> = (!filter.is_empty()).then(|| {
            let mut found = Vec::new();
            for &k in all {
                if own(&k) {
                    matched.push(k);
                    found.push(k);
                } else if holds(&k) || kept(&k) {
                    found.push(k);
                }
            }
            found
        });
        let listed_keys: std::borrow::Cow<'_, [ResKey]> =
            found.map_or(std::borrow::Cow::Borrowed(all.as_slice()), std::borrow::Cow::Owned);
        let keys: &[ResKey] = &listed_keys;
        // What makes a new resource of the group (its wizard, or the New
        // window): on the group's and its resources' right-click menus.
        let new = new_command(types);
        let new_label = new.map(|id| new_label(&id.name()));
        let at_group = cursor == Some(TreeAt::Group(name));
        let header = egui::CollapsingHeader::new(format!("{name} ({})", keys.len()))
            .id_salt(name)
            .default_open(*name == "Areas")
            // (The keys' cursor on a group shows as its row marked.)
            .show_background(at_group)
            // A resource to bring into view opens its group (a filter
            // typed opens every group, through `fold`); the keys open and
            // close the one they are at.
            .open(
                (app.tree_reveal.is_some_and(|(k, _)| types.contains(&k.restype)))
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
                    // (Shown opened out while the Filter finds what is
                    // placed in it.)
                    out |= !filter.is_empty() && holds(&k);
                    // (Opened out or closed by the keys.)
                    if let Some((_, open)) =
                        folded.filter(|f| is_area && f.0 == TreeAt::Resource(k))
                    {
                        out = open;
                        ui.data_mut(|d| d.insert_temp(opened, out));
                    }
                    // The resource to bring into view (a tab chosen, or
                    // its menu's Show in Module Tree, which opens an area
                    // out too).
                    let reveal = app.tree_reveal.filter(|(key, _)| *key == k);
                    if let Some((key, open_out)) = reveal {
                        app.tree_marked = Some(key);
                        if is_area {
                            app.tree_area = Some(k.resref);
                        }
                        if open_out && is_area && !out {
                            out = true;
                            ui.data_mut(|d| d.insert_temp(opened, true));
                        }
                    }
                    let in_hand = app.tree_marked == Some(k);
                    let at_row = cursor == Some(TreeAt::Resource(k));
                    let chosen = app.tree_selected.len() > 1 && app.tree_selected.contains(&k);
                    // A row out of sight takes its room only (a module can
                    // list thousands of a kind: laid out every frame, they
                    // slowed everything down while their group was open).
                    let height = ui.spacing().interact_size.y;
                    // (Up and down only: scrolled sideways past its start,
                    // a long name's row is still in sight.)
                    let top = ui.cursor().top();
                    let room =
                        egui::Rect::from_x_y_ranges(ui.clip_rect().x_range(), top..=top + height);
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
                        let marked = in_hand || at_row || chosen;
                        let r = ui.add(egui::Button::selectable(marked, label).sense(sense));
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
                        reveal_into_view(ui, r.rect);
                    }
                    if sought {
                        bring_into_view(ui, r.rect);
                    }
                    if r.clicked() {
                        clicked = Some(TreeAt::Resource(k));
                        let m = ui.input(|i| i.modifiers);
                        picked = Some((TreeAt::Resource(k), m.command || m.ctrl, m.shift));
                    }
                    if out {
                        let now = read_of(ws, k.resref);
                        match app.area_contents.get(&k.resref).filter(|c| c.0 == now) {
                            Some((_, contents)) => {
                                let in_hand = app
                                    .tree_object
                                    .filter(|(a, ..)| *a == k.resref)
                                    .map(|(_, kind, i)| (kind, i, app.tree_object_pending));
                                let chosen = (app.tree_objects.as_ref())
                                    .filter(|(a, set)| *a == k.resref && set.len() > 1)
                                    .map(|(_, set)| set);
                                let mut keys = ContentKeys {
                                    cursor,
                                    moved: cursor_moved,
                                    fold: folded,
                                    clicked: None,
                                    hovered: None,
                                    picked: None,
                                    together: false,
                                    listed: Vec::new(),
                                };
                                let went = contents_ui(
                                    ui,
                                    k.resref,
                                    contents,
                                    &filter,
                                    &mut go,
                                    in_hand,
                                    chosen,
                                    &mut keys,
                                    &menu_commands,
                                );
                                picked = keys.picked.or(picked);
                                together |= keys.together;
                                if went {
                                    app.tree_object_pending = false;
                                } else if in_hand.is_some_and(|h| h.2) {
                                    // (Its kind's list is still opening:
                                    // gone to once its rows are laid out.)
                                    object_waits = true;
                                    ui.ctx().request_repaint();
                                }
                                clicked = keys.clicked.or(clicked);
                                pointed_object = keys.hovered.or(pointed_object);
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
                        if chosen {
                            let keys: Vec<ResKey> = app.tree_selected.iter().copied().collect();
                            let asked = several_menu(ui, &keys, &menu_commands, &mut app.actions);
                            plugin_several = asked.or(plugin_several);
                            return;
                        }
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
                        if k.restype == ResType::ARE
                            && ui
                                .button("Export Model Files…")
                                .on_hover_text(
                                    "Write the files the game draws the area with into a \
                                     folder: its tileset, its tiles' models, walkmeshes and \
                                     textures, its skybox, and the models of what is placed in \
                                     it, from the haks and wherever else they are (the game's \
                                     own are left out, unless all of it is the game's)",
                                )
                                .clicked()
                        {
                            app.actions.push(Action::ExportAreaFiles(k.resref));
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
        if *name == "Areas" {
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
            bring_into_view(ui, header.header_response.rect);
        }
        opening |= header.openness < 1.0
            && app.tree_reveal.is_some_and(|(k, _)| types.contains(&k.restype));
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
    // (Brought into view, or not among the rows shown: done.)
    if !opening {
        app.tree_reveal = None;
    }
    // While the Filter has something typed, what every area holds is
    // read (not only the areas opened out), for what it finds there.
    if !filter.is_empty() {
        for k in ws.module.keys_of(ResType::ARE) {
            let now = read_of(ws, k.resref);
            if app.area_contents.get(&k.resref).is_none_or(|c| c.0 != now)
                && !read.contains(&k.resref)
            {
                read.push(k.resref);
            }
        }
    }
    let others = ws.module.len() - listed;
    ui.weak(format!("{others} other resources"));
    // Down from the Filter: to the first row in sight that it finds (a
    // resource by its ResRef or name, a placed object: an area listed for
    // its objects alone is passed over for them), else the first row.
    // (An object listed under the filter is one it found, or one of an
    // area found itself, which comes first.)
    if std::mem::take(&mut app.tree_to_match) {
        let rows = key_rows(&shown, &outs);
        let found = |at: &TreeAt| match at {
            TreeAt::Resource(k) => matched.contains(k),
            TreeAt::Object(..) => true,
            _ => false,
        };
        let first = rows.iter().map(|r| r.0).find(|at| !filter.is_empty() && found(at));
        app.tree_cursor = first.or(Some(first_row()));
        app.tree_cursor_moved = true;
        ui.ctx().request_repaint();
    }
    // A row clicked: with Ctrl, added to (or taken from) those chosen;
    // with Shift, those from the row clicked before to it, as listed. A
    // placed object's: the area view's selection follows.
    match picked {
        Some((TreeAt::Resource(k), command, shift)) => {
            let rows = key_rows(&shown, &outs);
            let listed: Vec<ResKey> = (rows.iter())
                .filter_map(|r| match r.0 {
                    TreeAt::Resource(k) => Some(k),
                    _ => None,
                })
                .collect();
            let (selected, anchor) = (&mut app.tree_selected, &mut app.tree_anchor);
            crate::widgets::select_in_list(selected, anchor, &listed, k, command, shift);
        }
        Some((TreeAt::Object(area, kind, index), command, shift)) => {
            let rows = key_rows(&shown, &outs);
            let listed: Vec<Placed> = (rows.iter())
                .filter_map(|r| match r.0 {
                    TreeAt::Object(a, k, i) if a == area => Some((k, i)),
                    _ => None,
                })
                .collect();
            // (Another area's chosen before: from the one in hand here.)
            let (mut set, mut anchor) = match app.tree_objects.take() {
                Some((a, set)) if a == area => (set, app.tree_objects_anchor),
                _ => {
                    let here = app.tree_object.filter(|o| o.0 == area).map(|o| (o.1, o.2));
                    (here.into_iter().collect(), here)
                }
            };
            crate::widgets::select_in_list(
                &mut set,
                &mut anchor,
                &listed,
                (kind, index),
                command,
                shift,
            );
            app.tree_objects_anchor = anchor;
            if let Some(view) = app.area_views.get_mut(&area) {
                view.selection = set.iter().copied().collect();
            }
            app.tree_objects = (!set.is_empty()).then_some((area, set));
        }
        _ => {}
    }
    // A click in the tree gives it the arrow keys (as the palette's),
    // wherever the pointer goes after until it is in an area's view: Up
    // and Down go from row to row (Page Up and Down by ten, Home and End
    // to the ends), Right opens a group and goes into it, Left goes back
    // to the group and closes it, Enter opens the row's resource, Escape
    // hands the keys back.
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
        // Letters typed while the tree has the keys, wherever the pointer
        // is, go to the row whose name begins with them, as in Aurora's
        // (Windows') tree: a pause starts over. (The pointer in an area's
        // view gives the keys back to it, and its letters to the camera.)
        // (Not while something is dragged: a blueprint dragged from the
        // tree onto an area, whose view has Q and E to turn it.)
        let dragging = ui.ctx().dragged_id().is_some();
        if !dragging {
            // (Not the camera's too, held down: the area is drawn after.)
            crate::palette_view::take_letters(ui.ctx());
        }
        let typed_id = egui::Id::new("tree-typed");
        let now = ui.input(|i| i.time);
        let (mut typed, mut typed_at) =
            ui.data(|d| d.get_temp::<(String, f64)>(typed_id)).unwrap_or_default();
        let going_on = !typed.is_empty() && now - typed_at <= crate::dialog_view::TYPE_AHEAD;
        let letters = ui.input_mut(|i| {
            let m = i.modifiers;
            if dragging || m.ctrl || m.alt || m.command || m.mac_cmd {
                return Vec::new();
            }
            // What is typed, as a name has it ("Beggar's Nest: T"; a
            // space goes on with letters being typed, and starts none).
            let mut letters: Vec<char> = Vec::new();
            i.events.retain(|e| {
                let egui::Event::Text(t) = e else { return true };
                let on = going_on || !letters.is_empty();
                let name_s = |c: char| (!c.is_whitespace() && !c.is_control()) || (c == ' ' && on);
                let taken = !t.is_empty() && t.chars().all(name_s);
                if taken {
                    letters.extend(t.chars());
                }
                !taken
            });
            // (A key that types nothing, as a test's: its letter or digit.)
            let letter = |key: &egui::Key| {
                let name = key.name();
                let c = name.chars().next().filter(|_| name.len() == 1)?;
                c.is_ascii_alphanumeric().then(|| c.to_ascii_lowercase())
            };
            if letters.is_empty() && m.is_none() {
                letters = (i.events.iter())
                    .filter_map(|e| match e {
                        egui::Event::Key { key, pressed: true, .. } => letter(key),
                        _ => None,
                    })
                    .collect();
            }
            // (Their keys too: no command has them meanwhile.)
            i.events.retain(|e| match e {
                egui::Event::Key { key: egui::Key::Space, .. } => !letters.contains(&' '),
                egui::Event::Key { key, .. } => {
                    letter(key).is_none_or(|c| !letters.iter().any(|t| t.eq_ignore_ascii_case(&c)))
                }
                _ => true,
            });
            letters
        });
        let named = |k: &ResKey| lists.iter().find_map(|(_, names)| names.get(k).cloned());
        let text = |at: TreeAt| row_text(at, &named, &app.area_contents);
        let mut to = None;
        for c in letters.iter().copied() {
            if now - typed_at > crate::dialog_view::TYPE_AHEAD {
                typed.clear();
            }
            typed.push(c);
            typed_at = now;
            to = seek(&shown, &outs, to.or(app.tree_cursor), &typed, &text).or(to);
        }
        if !letters.is_empty() {
            ui.data_mut(|d| d.insert_temp(typed_id, (typed, typed_at)));
        }
        if let Some(at) = to {
            app.tree_cursor = Some(at);
            app.tree_cursor_moved = true;
            ui.ctx().request_repaint();
        }
        // F2 and Delete on the row under the pointer, else at the
        // cursor's: a resource's as its menu's Rename… and Delete… (both
        // ask); a placed object's Delete as its menu's (undone as any
        // change; the keys go on to the row after it). F2 does nothing at
        // a placed object: its menu has no Rename (nor has Aurora's), its
        // name and tag are its Properties'.
        let under = pointed.map(TreeAt::Resource).or(pointed_object).or(app.tree_cursor);
        match under {
            Some(TreeAt::Resource(k)) => {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F2)) {
                    app.actions.push(Action::RenameDialog(k));
                }
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Delete)) {
                    app.actions.push(Action::DeleteDialog(k));
                }
            }
            Some(at @ TreeAt::Object(area, kind, index))
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Delete)) =>
            {
                go = Some((area, kind, index, Asked::Delete));
                together = true;
                app.tree_cursor = app.tree_cursor.map(|c| past_deleted(&shown, &outs, c, at));
                ui.ctx().request_repaint();
            }
            _ => {}
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
    // What the areas opened out hold, read for the next frame (those the
    // Filter looks in after them; a module's hundreds of areas over a
    // few frames, not all in one).
    if !read.is_empty()
        && let Some(ws) = app.ws.as_ref()
    {
        let began = std::time::Instant::now();
        for area in read {
            let contents = area_contents(ws, app.game.as_deref(), area);
            app.area_contents.insert(area, (read_of(ws, area), contents));
            if began.elapsed() > std::time::Duration::from_millis(12) {
                break;
            }
        }
        ui.ctx().request_repaint();
    }
    // A plugin's command chosen on a row among several chosen: on each.
    if let Some(i) = plugin_several {
        let on = (app.tree_selected.iter())
            .map(|k| match k.restype {
                ResType::ARE => mg_plugin::Selected::Area(k.resref),
                _ => mg_plugin::Selected::Blueprint(*k),
            })
            .collect();
        app.actions.push(crate::plugins::run_on(&menu_commands[i], on));
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
        // Asked of a row among several chosen (its menu, the Delete key):
        // of them all.
        let targets: Vec<Placed> = match &app.tree_objects {
            Some((a, set)) if together && *a == area && set.contains(&(kind, index)) => {
                set.iter().copied().collect()
            }
            _ => vec![(kind, index)],
        };
        let path = |(kind, index): Placed| mg_edit::GffPath::root().item(kind.list(), index);
        match asked {
            Asked::Plugin(i) => {
                let on = (targets.iter())
                    .map(|&(kind, index)| mg_plugin::Selected::Object { area, kind, index })
                    .collect();
                app.actions.push(crate::plugins::run_on(&menu_commands[i], on));
            }
            Asked::Properties if targets.len() > 1 => {
                let paths = targets.iter().copied().map(path).collect();
                app.actions.push(Action::OpenTab(Tab::Instances { area, paths }));
            }
            Asked::Properties => {
                let path = path((kind, index));
                app.actions.push(Action::OpenTab(Tab::Instance { area, path }));
            }
            Asked::Variables => crate::widgets::open_variables(app, git, path((kind, index))),
            Asked::Preview => {
                let path = path((kind, index));
                app.actions.push(Action::OpenTab(Tab::InstanceModel { area, path }));
            }
            Asked::Export => app.actions.push(Action::ExportObject { area, kind, index }),
            Asked::Go if targets.len() > 1 => {
                // Each of them selected in the area's view (once it is
                // open), the camera at the one clicked.
                app.area_focus = Some((area, kind, index));
                app.area_select = Some((area, targets.clone()));
                if let Some(view) = app.area_views.get_mut(&area) {
                    view.selection = targets;
                }
                app.actions.push(Action::OpenTab(Tab::Area(area)));
            }
            Asked::Go => {
                app.area_focus = Some((area, kind, index));
                // (The row clicked is the one marked: the tree stays as
                // it is, rather than go to the area's row and back.)
                app.tree_object = Some((area, kind, index));
                app.tree_object_pending = false;
                // (And the one a Shift+click goes from.)
                app.tree_objects = Some((area, [(kind, index)].into()));
                app.tree_objects_anchor = Some((kind, index));
                if let Some(view) = app.area_views.get_mut(&area) {
                    view.selection = vec![(kind, index)];
                }
                app.actions.push(Action::OpenTab(Tab::Area(area)));
            }
            Asked::Copy => {
                // As Copy in the area's view: pasted in any area (Ctrl+V),
                // each as high above the ground as it stands (the area's
                // ground read for it where its view isn't open).
                let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else {
                    return;
                };
                let list = |ws: &mut Workspace, (kind, index): Placed| {
                    ws.doc(&git).ok()?.root.list(kind.list())?.get(index).cloned()
                };
                let placed: Vec<(Placed, mg_gff::Struct)> =
                    targets.iter().filter_map(|&t| Some((t, list(ws, t)?))).collect();
                let ground = match app.area_views.get(&area) {
                    Some(_) => None,
                    None => crate::area_view::ground_of(game, ws, area),
                };
                let mut objects = Vec::new();
                for ((kind, index), s) in placed {
                    let o = mg_area::AreaObject::read(game, kind, index, &s);
                    let lift = match app.area_views.get(&area) {
                        Some(view) => view.lift_of(&o),
                        None => crate::area_view::lift_on(ground.as_ref(), &o),
                    };
                    objects.push((o, s, lift));
                }
                if let Some(anchor) = objects.first().map(|o| o.0.position) {
                    let n = objects.len();
                    let what = if n == 1 { "1 object".to_string() } else { format!("{n} objects") };
                    app.object_clip = Some(crate::area_view::ObjectClip { objects, anchor });
                    crate::widgets::mark_clipboard(ui.ctx(), &what);
                    app.log.info(format!("{what} copied: paste in an area (Ctrl+V)"));
                }
            }
            Asked::Delete => {
                let edits = mg_area::edit::delete_edits(git, &targets);
                // (Objects after it move up their list: what is selected
                // in the area's view, and the Properties open, would be
                // others'.)
                if let Some(view) = app.area_views.get_mut(&area) {
                    view.selection.clear();
                }
                // (Nor is the one marked: the keys' cursor stays where it
                // was put, rather than follow the selection cleared.)
                if app.tree_object.is_some_and(|(a, ..)| a == area) {
                    app.tree_object = None;
                    app.tree_object_pending = false;
                }
                if app.tree_objects.as_ref().is_some_and(|(a, _)| *a == area) {
                    app.tree_objects = None;
                    app.tree_objects_anchor = None;
                    // (Several gone: the keys go to the kind's row.)
                    if targets.len() > 1
                        && let Some(TreeAt::Object(a, k, _)) = app.tree_cursor
                    {
                        app.tree_cursor = Some(TreeAt::Kind(a, k));
                    }
                }
                crate::tabs::close_tabs(&mut app.dock, |t| {
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

/// The right-click menu of a row among several resources chosen in the
/// module tree (`keys`): what is done to them together. Edit Together for
/// blueprints of one kind or for areas (Properties of several), Export…,
/// Export as Files…, Copy to Scratch Folder; for areas, Make Placeables
/// Static and Dynamic; the plugins' commands for them all (the one chosen
/// returned).
fn several_menu(
    ui: &mut Ui,
    keys: &[ResKey],
    commands: &[crate::plugins::PluginCommand],
    actions: &mut Vec<Action>,
) -> Option<usize> {
    let n = keys.len();
    ui.weak(format!("{n} resources chosen"));
    ui.separator();
    let first = keys.first()?.restype;
    let one_type = keys.iter().all(|k| k.restype == first);
    let areas: Vec<ResRef> = keys.iter().map(|k| k.resref).collect();
    let together = "One editor: what you change is set on each";
    if one_type && first == ResType::ARE {
        if ui.button(format!("Edit {n} Together")).on_hover_text(together).clicked() {
            actions.push(Action::OpenTab(Tab::AreasProperties(areas.clone())));
            ui.close();
        }
        if ui.button("Make Placeables Static").on_hover_text(STATIC_TIP).clicked() {
            actions.push(Action::StaticPlaceables(areas.clone()));
            ui.close();
        }
        if ui
            .button("Make Placeables Dynamic")
            .on_hover_text("Clears Static on every placeable of the areas")
            .clicked()
        {
            actions.push(Action::DynamicPlaceables(areas));
            ui.close();
        }
        ui.separator();
    } else if one_type
        && mg_area::ObjectKind::from_restype(first).is_some()
        && ui.button(format!("Edit {n} Together")).on_hover_text(together).clicked()
    {
        actions.push(Action::OpenTab(Tab::Blueprints(keys.to_vec())));
        ui.close();
    }
    if ui.button("Export…").clicked() {
        actions.push(Action::ExportDialog(keys.to_vec()));
        ui.close();
    }
    let files = "Write them into a folder (a script with its compiled script, an area with its \
                 .git and .gic)";
    if ui.button("Export as Files…").on_hover_text(files).clicked() {
        let keys = keys.to_vec();
        actions.push(Action::ExportFiles { keys, dependencies: false, scratch: false });
        ui.close();
    }
    if ui.button("Copy to Scratch Folder").clicked() {
        let keys = keys.to_vec();
        actions.push(Action::ExportFiles { keys, dependencies: false, scratch: true });
        ui.close();
    }
    // The plugins' commands for them, where they are of one kind (areas,
    // or blueprints of one type).
    let target = match first {
        _ if !one_type => None,
        ResType::ARE => Some(mg_plugin::Target::Area),
        t => mg_area::ObjectKind::from_restype(t).map(mg_plugin::Target::Object),
    };
    target.and_then(|t| crate::plugins::menu_commands(ui, commands, &[t]))
}

/// A named resource in the tree: its name, and its ResRef in parentheses
/// when asked for (Options › General).
fn named_label(name: &str, resref: ResRef, resrefs: bool) -> String {
    if resrefs { format!("{name} ({resref})") } else { name.to_string() }
}

/// The menu entry for a command that makes something new, from its name:
/// "Area Wizard" is "New Area…", "New Script" is "New Script…".
pub(crate) fn new_label(command: &str) -> String {
    let what = command.trim_end_matches(" Wizard");
    if what.starts_with("New ") { format!("{what}…") } else { format!("New {what}…") }
}

/// The command that makes a new resource of a tree group's types: the
/// Wizards menu's for it.
fn new_command(types: &[ResType]) -> Option<crate::commands::Id> {
    use crate::commands::Id;
    Some(match *types.first()? {
        ResType::ARE => Id::AreaWizard,
        ResType::DLG => Id::NewConversation,
        ResType::NSS => Id::NewScript,
        ResType::JUI => Id::NewNui,
        t => Id::new_blueprint(ObjectKind::from_restype(t)?),
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
            ("Areas", true, vec![a, b].into()),
            ("Conversations", false, vec![].into()),
            ("Scripts", false, vec![c].into()),
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
        let contents = HashMap::new();
        let by = |named: &dyn Fn(&ResKey) -> Option<String>, at, typed: &str| {
            seek(&shown, &outs, at, typed, &|r| row_text(r, named, &contents))
        };
        let none = |_: &ResKey| None;
        assert_eq!(by(&none, Some(areas), "s"), Some(scripts));
        assert_eq!(by(&none, Some(scripts), "A"), Some(areas));
        assert_eq!(by(&none, None, "c"), Some(TreeAt::Group("Conversations")));
        assert_eq!(by(&none, Some(areas), "z"), None);
        // Listed by name, a row is found by its name's letter, not its
        // ResRef's.
        let named = |k: &ResKey| (*k == b).then(|| "Zhentil Keep (b)".to_string());
        assert_eq!(by(&named, Some(areas), "z"), at(b));
        assert_eq!(by(&named, at(a), "b"), None);
        // Letters typed in quick succession: the row that begins with them
        // all, case and accents aside, a space among them.
        let names =
            |k: &ResKey| Some(if *k == a { "Shop" } else { "Shop Thing, Árbol" }.to_string());
        assert_eq!(by(&names, Some(areas), "sho"), at(a), "the first, itself first");
        assert_eq!(by(&names, Some(areas), "shopt"), None);
        assert_eq!(by(&names, Some(areas), "SHOP T"), at(b));
        assert_eq!(by(&names, Some(areas), "shop thing, arb"), at(b));
        assert_eq!(by(&names, at(a), "shop"), at(a));
        assert_eq!(by(&names, at(a), "ss"), at(b), "the same letter: the next");
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
        let shown: Shown = vec![("Areas", true, vec![a, b].into())];
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

    /// An area opened out and the placed objects of a kind open, for the
    /// keys: area `a`'s creatures Goblin (0) and Bandit (1), listed Bandit
    /// first, its door Gate in a kind closed and its waypoint by its tag
    /// WP_GATE; area `b` after it.
    fn opened_out() -> (Shown<'static>, Outs, HashMap<ResRef, (Read, AreaContents)>, ResKey, ResKey)
    {
        use mg_area::ObjectKind::{Creature, Door, Waypoint};
        let key = |name: &str| ResKey::parse(name, ResType::ARE).unwrap();
        let (a, b) = (key("a"), key("b"));
        let shown: Shown = vec![("Areas", true, vec![a, b].into())];
        let mut outs = Outs::new();
        outs.insert(
            a.resref,
            vec![(Creature, true, vec![1, 0]), (Door, false, vec![0]), (Waypoint, true, vec![0])],
        );
        let row = |text: &str| ObjectRow {
            text: text.to_string(),
            sought: mg_core::text::fold(text),
            tip: String::new(),
        };
        let kind = |kind, rows: Vec<ObjectRow>, order| KindContents { kind, rows, order };
        let contents = vec![
            kind(Creature, vec![row("Goblin"), row("Bandit")], vec![1, 0]),
            kind(Door, vec![row("Gate")], vec![0]),
            kind(Waypoint, vec![row("WP_GATE")], vec![0]),
        ];
        let contents = HashMap::from([(a.resref, (Read::Revision(0), contents))]);
        (shown, outs, contents, a, b)
    }

    /// Letters typed go to the rows in sight as they show, an area's kinds
    /// and placed objects among them: a kind by its name, an object by its
    /// name (a waypoint by its tag), in the order listed; not to one in a
    /// kind closed.
    #[test]
    fn letters_go_to_kinds_and_placed_objects_in_sight() {
        use mg_area::ObjectKind::{Creature, Door, Waypoint};
        let (shown, outs, contents, a, b) = opened_out();
        let none = |_: &ResKey| None;
        let by =
            |at, typed: &str| seek(&shown, &outs, at, typed, &|r| row_text(r, &none, &contents));
        let areas = Some(TreeAt::Group("Areas"));
        let object = |kind, i| Some(TreeAt::Object(a.resref, kind, i));
        assert_eq!(by(areas, "c"), Some(TreeAt::Kind(a.resref, Creature)));
        assert_eq!(by(areas, "goblin"), object(Creature, 0));
        assert_eq!(by(areas, "ba"), object(Creature, 1));
        // (The door is in a kind closed: its kind's row is found.)
        assert_eq!(by(areas, "g"), object(Creature, 0));
        assert_eq!(by(areas, "d"), Some(TreeAt::Kind(a.resref, Door)));
        assert_eq!(by(areas, "wp"), object(Waypoint, 0), "a waypoint by its tag");
        // From an object on, in the order listed, round the end.
        assert_eq!(by(object(Creature, 1), "w"), Some(TreeAt::Kind(a.resref, Waypoint)));
        let waypoints = Some(TreeAt::Kind(a.resref, Waypoint));
        assert_eq!(by(waypoints, "ww"), object(Waypoint, 0), "the same letter: the next");
        assert_eq!(by(object(Waypoint, 0), "b"), Some(TreeAt::Resource(b)));
        assert_eq!(by(Some(TreeAt::Resource(b)), "b"), object(Creature, 1));
    }

    /// Deleted by Delete, a placed object the keys were at leaves them at
    /// the row after it among its kind's, else the one before, else at the
    /// kind's; at another of its kind's after it, they follow it up its
    /// area's list.
    #[test]
    fn the_keys_go_past_a_placed_object_deleted() {
        use mg_area::ObjectKind::{Creature, Waypoint};
        let (shown, outs, _, a, _) = opened_out();
        let object = |kind, i| TreeAt::Object(a.resref, kind, i);
        let (bandit, goblin) = (object(Creature, 1), object(Creature, 0));
        // Bandit, listed first: Goblin after it (still 0).
        assert_eq!(past_deleted(&shown, &outs, bandit, bandit), goblin);
        // Goblin, listed last: Bandit before it, now 0.
        assert_eq!(past_deleted(&shown, &outs, goblin, goblin), object(Creature, 0));
        // The only waypoint: its kind.
        let wp = object(Waypoint, 0);
        assert_eq!(past_deleted(&shown, &outs, wp, wp), TreeAt::Kind(a.resref, Waypoint));
        // The keys at Bandit while Goblin is deleted: Bandit moves up.
        assert_eq!(past_deleted(&shown, &outs, bandit, goblin), object(Creature, 0));
        assert_eq!(past_deleted(&shown, &outs, goblin, bandit), goblin);
        let group = TreeAt::Group("Areas");
        assert_eq!(past_deleted(&shown, &outs, group, goblin), group);
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
        // Each kind of blueprint is a group by the name the toolset calls
        // it by, and its New… is its wizard's, as the palettes' is.
        for k in ObjectKind::ALL {
            assert!(GROUPS.contains(&(k.label(), &[k.restype()])), "{k:?}");
            assert_eq!(new_command(&[k.restype()]), Some(Id::new_blueprint(k)));
        }
        assert_eq!(GROUPS.iter().filter(|(n, _)| n.is_empty()).count(), 0);
        assert_eq!(label(&[ResType::UTW]).as_deref(), Some("New Waypoint…"));
        assert_eq!(
            new_command(&[ResType::UTI]),
            Some(Id::Wizard(mg_module::palette::BlueprintKind::Item))
        );
    }
}
