//! A module's open tabs, from one session to the next (Options › General:
//! Open a module with the tabs it had): the areas, scripts, blueprints,
//! conversations and tools that were open when it was closed open again
//! with it, in their order and their places (the middle, the palettes'
//! pane, a pane split off the middle, a window of their own where it was),
//! each pane's tab in front in front.
//!
//! They are kept in Moonglow's data folder, never in the module: a small
//! JSON file a module (`open-tabs/<name>-<hash>.json`, by its canonical
//! path, a module folder's or a project's by its folder), written when its
//! tabs change (so that a crash loses none of them; where its windows were
//! moved to since is written on closing), when the module is closed and
//! when Moonglow quits. A tab whose resource is gone is passed over, said
//! in the debug log. An area's view is made when its tab is first drawn,
//! so the areas behind the one in front cost nothing until they are shown.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use egui_dock::NodePath;
use mg_core::{ResRef, ResType};
use mg_edit::{GffPath, Step};
use mg_resman::ResKey;
use serde::{Deserialize, Serialize};

use crate::{Action, Moonglow, Place, Side, Tab, tabs, trace};

/// A module's tabs as they are kept.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Record {
    /// The panes with tabs to keep, in the order they lie (the main
    /// window's, then the windows').
    panes: Vec<Pane>,
    /// The tab in front of the pane that had the keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    front: Option<String>,
}

/// A pane's tabs (a window's: all its panes'), as [`tab_text`] names them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Pane {
    /// `middle`, `palettes`, `split left 300` (the side of the middle, and
    /// thousandths of the two) or `window`.
    place: String,
    tabs: Vec<String>,
    /// The tab in front.
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<String>,
    /// A window's place and size: left, top, width, height in points.
    #[serde(skip_serializing_if = "Option::is_none")]
    rect: Option<[f32; 4]>,
}

/// A kept pane, its tabs found again in the module.
#[derive(Debug, Clone)]
struct Kept {
    /// `None`: a place not understood; the tabs open where their kind does.
    place: Option<Place>,
    tabs: Vec<Tab>,
    active: Option<Tab>,
    rect: Option<egui::Rect>,
}

/// The open module's tabs as kept and as they are now.
#[derive(Debug, Default)]
pub(crate) struct OpenTabs {
    /// The tabs kept, to open once the module's first frame has drawn
    /// its panes (the palettes' among them), and the one in front.
    pending: Option<(Vec<Kept>, Option<Tab>)>,
    /// The panes and their tabs at the last frame, hashed: a change is
    /// written.
    shape: u64,
    /// What was last written (or read): not written again unchanged.
    written: Option<String>,
    /// A failure to write was said in the log (once).
    told: bool,
}

/// The file in `dir` that keeps the tabs of the module at `module` (named
/// as the locks' files are, by its canonical path).
pub fn file(dir: &Path, module: &Path) -> PathBuf {
    mg_module::locks::file(dir, module)
}

fn key_text(key: &ResKey) -> String {
    key.to_string()
}

fn parse_key(text: &str) -> Option<ResKey> {
    let (name, ext) = text.rsplit_once('.')?;
    ResKey::parse(name, ResType::from_extension(ext)?)
}

fn parse_resref(text: &str) -> Option<ResRef> {
    ResRef::from_str(text).ok()
}

/// A path to a struct in a GFF: `Creature List#3` for an item of a list,
/// a field's label, joined by `/`.
fn path_text(path: &GffPath) -> String {
    let steps: Vec<String> = (path.0.iter())
        .map(|s| match s {
            Step::Field(label) => label.clone(),
            Step::Item(label, index) => format!("{label}#{index}"),
        })
        .collect();
    steps.join("/")
}

fn parse_path(text: &str) -> Option<GffPath> {
    if text.is_empty() {
        return Some(GffPath::root());
    }
    let steps = text.split('/').map(|s| match s.rsplit_once('#') {
        Some((label, index)) => index.parse().ok().map(|i| Step::Item(label.to_string(), i)),
        None => Some(Step::Field(s.to_string())),
    });
    steps.collect::<Option<Vec<_>>>().map(GffPath)
}

fn list<T>(items: &[T], text: impl Fn(&T) -> String) -> String {
    items.iter().map(text).collect::<Vec<_>>().join(",")
}

fn parse_list<T>(text: &str, parse: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    let items = text.split(',').map(parse).collect::<Option<Vec<_>>>()?;
    (!items.is_empty()).then_some(items)
}

/// How a tab is kept; `None` for those that are not: the start page, the
/// pane that says no area is open, the palettes (shown or folded away as
/// they were left, for every module), the Options and Tile Properties (a
/// draft of the moment), what shows the result of a search, and the side
/// editors (a hak's, a 2DA's, a tileset's: files of their own).
fn tab_text(tab: &Tab) -> Option<String> {
    Some(match tab {
        Tab::ModuleProperties => "module-properties".into(),
        Tab::Resources => "resources".into(),
        Tab::Factions => "factions".into(),
        Tab::Journal => "journal".into(),
        Tab::TalkTable => "talk-table".into(),
        Tab::Manual => "manual".into(),
        Tab::AdvancedSearch => "advanced-search".into(),
        Tab::Script(k) => format!("script:{}", key_text(k)),
        Tab::Nui(k) => format!("nui:{}", key_text(k)),
        Tab::Gff(k) => format!("gff:{}", key_text(k)),
        Tab::Dialog(k) => format!("dialog:{}", key_text(k)),
        Tab::Resource(k) => format!("resource:{}", key_text(k)),
        Tab::Model(k) => format!("model:{}", key_text(k)),
        Tab::Blueprint(k) => format!("blueprint:{}", key_text(k)),
        Tab::Blueprints(keys) => format!("blueprints:{}", list(keys, key_text)),
        Tab::Area(r) => format!("area:{r}"),
        Tab::AreaProperties(r) => format!("area-properties:{r}"),
        Tab::AreasProperties(rs) => format!("areas-properties:{}", list(rs, |r| r.to_string())),
        Tab::Instance { area, path } => format!("instance:{area}:{}", path_text(path)),
        Tab::InstanceModel { area, path } => {
            format!("instance-model:{area}:{}", path_text(path))
        }
        Tab::Instances { area, paths } => format!("instances:{area}:{}", list(paths, path_text)),
        Tab::Welcome
        | Tab::NoArea
        | Tab::Palette
        | Tab::Options
        | Tab::TileProperties
        | Tab::References
        | Tab::PlaceableGallery
        | Tab::Hak(_)
        | Tab::Table(_)
        | Tab::Tileset(_)
        | Tab::ResourceCopy { .. }
        | Tab::Tile { .. } => return None,
    })
}

/// The tab [`tab_text`] names.
fn parse_tab(text: &str) -> Option<Tab> {
    let (kind, rest) = text.split_once(':').unwrap_or((text, ""));
    let area_and = |rest: &str| {
        let (area, rest) = rest.split_once(':')?;
        Some((parse_resref(area)?, rest.to_string()))
    };
    Some(match kind {
        "module-properties" => Tab::ModuleProperties,
        "resources" => Tab::Resources,
        "factions" => Tab::Factions,
        "journal" => Tab::Journal,
        "talk-table" => Tab::TalkTable,
        "manual" => Tab::Manual,
        "advanced-search" => Tab::AdvancedSearch,
        "script" => Tab::Script(parse_key(rest)?),
        "nui" => Tab::Nui(parse_key(rest)?),
        "gff" => Tab::Gff(parse_key(rest)?),
        "dialog" => Tab::Dialog(parse_key(rest)?),
        "resource" => Tab::Resource(parse_key(rest)?),
        "model" => Tab::Model(parse_key(rest)?),
        "blueprint" => Tab::Blueprint(parse_key(rest)?),
        "blueprints" => Tab::Blueprints(parse_list(rest, parse_key)?),
        "area" => Tab::Area(parse_resref(rest)?),
        "area-properties" => Tab::AreaProperties(parse_resref(rest)?),
        "areas-properties" => Tab::AreasProperties(parse_list(rest, parse_resref)?),
        "instance" => {
            let (area, path) = area_and(rest)?;
            Tab::Instance { area, path: parse_path(&path)? }
        }
        "instance-model" => {
            let (area, path) = area_and(rest)?;
            Tab::InstanceModel { area, path: parse_path(&path)? }
        }
        "instances" => {
            let (area, paths) = area_and(rest)?;
            Tab::Instances { area, paths: parse_list(&paths, parse_path)? }
        }
        _ => return None,
    })
}

fn place_text(place: Place) -> String {
    match place {
        Place::Window => "window".into(),
        Place::Middle => "middle".into(),
        Place::Palettes => "palettes".into(),
        Place::Split(side, share) => format!("split {} {share}", side.name()),
    }
}

fn parse_place(text: &str) -> Option<Place> {
    let mut words = text.split_whitespace();
    Some(match (words.next()?, words.next()) {
        ("window", None) => Place::Window,
        ("middle", None) => Place::Middle,
        ("palettes", None) => Place::Palettes,
        ("split", Some(side)) => {
            let side = match side {
                "left" => Side::Left,
                "right" => Side::Right,
                "above" => Side::Above,
                "below" => Side::Below,
                _ => return None,
            };
            Place::Split(side, words.next()?.parse::<u16>().ok()?.clamp(50, 950))
        }
        _ => return None,
    })
}

impl Moonglow {
    /// Whether the open module's tabs are kept: it has a file, Moonglow a
    /// data folder, and Options › General keeps them.
    fn tabs_file(&self) -> Option<PathBuf> {
        if self.settings.no_reopen_tabs {
            return None;
        }
        Some(file(self.open_tabs_dir.as_deref()?, &self.module_path()?))
    }

    /// Whether a kept tab can be shown still: its resource is in the
    /// module (a read-only one's, in the load order), a placed object's
    /// in its area.
    fn tab_there(&self, tab: &Tab) -> bool {
        let Some(ws) = &self.ws else { return false };
        let module = &ws.module;
        let area = |a: &ResRef| module.contains(&ResKey::new(*a, ResType::ARE));
        let placed = |a: &ResRef, paths: &[GffPath]| {
            let git = module.gff(&ResKey::new(*a, ResType::GIT));
            matches!(git, Some(Ok(git)) if paths.iter().all(|p| p.get(&git.root).is_some()))
        };
        match tab {
            Tab::Resource(k) | Tab::Model(k) => {
                module.contains(k) || self.game.as_ref().is_some_and(|g| g.resman.contains(k))
            }
            Tab::Blueprints(keys) => keys.iter().all(|k| module.contains(k)),
            Tab::AreasProperties(areas) => areas.iter().all(area),
            Tab::Instance { area, path } | Tab::InstanceModel { area, path } => {
                placed(area, std::slice::from_ref(path))
            }
            Tab::Instances { area, paths } => placed(area, paths),
            tab => tab.renamable().is_none_or(|k| module.contains(&k)),
        }
    }

    /// Reads the tabs kept for the module just opened, to open at its
    /// first frame: whether there are any (the area opened last is not
    /// opened then).
    pub(crate) fn read_open_tabs(&mut self) -> bool {
        self.open_tabs = OpenTabs::default();
        let Some(path) = self.tabs_file() else { return false };
        let Ok(text) = std::fs::read_to_string(&path) else { return false };
        let record: Record = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                trace::note(format!("open tabs: {} not read: {e}", path.display()));
                return false;
            }
        };
        let found = |text: &str| match parse_tab(text) {
            Some(tab) if self.tab_there(&tab) => Some(tab),
            Some(_) => {
                trace::note(format!("open tabs: {text} passed over: not in the module now"));
                None
            }
            None => {
                trace::note(format!("open tabs: {text} passed over: not understood"));
                None
            }
        };
        let mut panes = Vec::new();
        for pane in &record.panes {
            let tabs: Vec<Tab> = pane.tabs.iter().filter_map(|t| found(t)).collect();
            if tabs.is_empty() {
                continue;
            }
            let active = pane.active.as_deref().and_then(parse_tab).filter(|t| tabs.contains(t));
            let rect = pane
                .rect
                .map(|[x, y, w, h]| egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h)));
            panes.push(Kept { place: parse_place(&pane.place), tabs, active, rect });
        }
        if panes.is_empty() {
            return false;
        }
        let front = record.front.as_deref().and_then(parse_tab);
        let front = front.filter(|f| panes.iter().any(|p| p.tabs.contains(f)));
        trace::note(format!("open tabs: {} panes from {}", panes.len(), path.display()));
        self.open_tabs.pending = Some((panes, front));
        self.open_tabs.written = Some(text);
        true
    }

    /// The open module's tabs, as they are kept.
    fn tabs_record(&self) -> Record {
        let mut panes: Vec<Pane> = Vec::new();
        // (The windows' panes, by the window: one pane each.)
        let mut windows: Vec<(egui_dock::SurfaceIndex, usize)> = Vec::new();
        let focused = self.dock.focused_leaf();
        for (path, leaf) in self.dock.iter_leaves() {
            let tabs: Vec<String> = leaf.tabs.iter().filter_map(tab_text).collect();
            if tabs.is_empty() {
                continue;
            }
            let active = leaf.tabs.get(leaf.active.0).and_then(tab_text);
            if !path.surface.is_main() {
                if let Some(&(_, i)) = windows.iter().find(|(s, _)| *s == path.surface) {
                    panes[i].tabs.extend(tabs);
                    if focused == Some(path) {
                        panes[i].active = active;
                    }
                    continue;
                }
                // (As it was before it was maximized.)
                let rect = leaf.tabs.iter().find_map(|t| {
                    (self.maximized.get(t).map(|m| m.0)).or(self.windows.get(t).map(|w| w.now))
                });
                let rect = rect.map(|r| [r.left(), r.top(), r.width(), r.height()]);
                windows.push((path.surface, panes.len()));
                panes.push(Pane { place: place_text(Place::Window), tabs, active, rect });
                continue;
            }
            let place = if leaf.tabs.contains(&Tab::Palette) {
                Place::Palettes
            } else {
                self.split_of(path.node).unwrap_or(Place::Middle)
            };
            panes.push(Pane { place: place_text(place), tabs, active, rect: None });
        }
        let pane = focused.or_else(|| self.tab_pane());
        let leaf = pane.and_then(|p| self.dock.leaf(p).ok());
        let front = leaf.and_then(|l| l.tabs.get(l.active.0)).and_then(tab_text);
        Record { panes, front }
    }

    /// Writes the open module's tabs, if they are kept and changed since
    /// they were last written.
    pub(crate) fn keep_open_tabs(&mut self) {
        if self.open_tabs.pending.is_some() || self.ws.is_none() {
            return;
        }
        let Some(path) = self.tabs_file() else { return };
        let Ok(text) = serde_json::to_string_pretty(&self.tabs_record()) else { return };
        if self.open_tabs.written.as_deref() == Some(text.as_str()) {
            return;
        }
        // (Whole or not at all: written beside it, then put in its place.)
        let partial = path.with_extension("json.partial");
        let written = (path.parent().map_or(Ok(()), std::fs::create_dir_all))
            .and_then(|()| std::fs::write(&partial, &text))
            .and_then(|()| std::fs::rename(&partial, &path));
        match written {
            Ok(()) => trace::note(format!("open tabs written: {}", path.display())),
            Err(e) if !std::mem::replace(&mut self.open_tabs.told, true) => self.log.warn(format!(
                "The open tabs could not be kept for the next time: {}: {e}",
                path.display()
            )),
            Err(_) => {}
        }
        self.open_tabs.written = Some(text);
    }

    /// At the end of each frame: the kept tabs opened, at the module's
    /// first; then, the tabs written whenever they change.
    pub(crate) fn open_tabs_frame(&mut self, ctx: &egui::Context) {
        // (Not in the frame a module was opened in from a menu: its
        // palettes' pane opens at the next, as the module tree shows.)
        if self.open_tabs.pending.is_some() && self.open_palette && self.ws.is_some() {
            ctx.request_repaint();
            return;
        }
        if let Some((panes, front)) = self.open_tabs.pending.take() {
            if self.ws.is_some() {
                self.restore_tabs(panes, front);
                ctx.request_repaint();
            }
            return;
        }
        if self.ws.is_none() || self.open_tabs_dir.is_none() || self.settings.no_reopen_tabs {
            return;
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for (path, leaf) in self.dock.iter_leaves() {
            (path.surface.0, path.node.0, leaf.active.0, &leaf.tabs).hash(&mut h);
        }
        self.dock.focused_leaf().map(|p| (p.surface.0, p.node.0)).hash(&mut h);
        let shape = h.finish();
        if std::mem::replace(&mut self.open_tabs.shape, shape) != shape {
            self.keep_open_tabs();
        }
    }

    /// Opens the kept tabs in place of the module's first (Module
    /// Properties, docked in the middle), pane by pane: each where it
    /// was, or where its kind opens if that place is not there now.
    fn restore_tabs(&mut self, panes: Vec<Kept>, front: Option<Tab>) {
        let palette = self.dock.find_tab(&Tab::Palette).map(|p| (p.surface, p.node));
        let middle = |app: &Moonglow| {
            (app.dock.iter_leaves().map(|(p, _)| p))
                .find(|p| p.surface.is_main() && Some((p.surface, p.node)) != palette)
        };
        // (The pane that says no area is open keeps the middle's place
        // while they come, and goes once they have: `keep_middle`.)
        if self.dock.find_tab(&Tab::NoArea).is_none()
            && let Some(at) = middle(self)
        {
            self.dock.set_focused_node_and_surface(at);
            self.dock.push_to_focused_leaf(Tab::NoArea);
        }
        tabs::close_tabs(&mut self.dock, |t| *t != Tab::ModuleProperties);
        let pane_of = |app: &Moonglow, tab: &Tab| {
            app.dock.find_tab(tab).map(|p| NodePath { surface: p.surface, node: p.node })
        };
        for pane in panes {
            let tabs: Vec<Tab> =
                pane.tabs.into_iter().filter(|t| self.dock.find_tab(t).is_none()).collect();
            let Some(first) = tabs.first().cloned() else { continue };
            let at = match pane.place {
                Some(Place::Middle) => (self.dock.find_tab(&Tab::NoArea))
                    .map(|p| NodePath { surface: p.surface, node: p.node })
                    .or_else(|| middle(self)),
                Some(Place::Palettes) => pane_of(self, &Tab::Palette),
                Some(Place::Split(side, share)) => self.split_middle(side, share, &first),
                Some(Place::Window) => {
                    self.open_window(first.clone());
                    if let Some(rect) = pane.rect {
                        let screen = self.screen.unwrap_or(rect);
                        let size = rect.size().min(screen.size());
                        let at = rect.min.clamp(screen.min, (screen.max - size).max(screen.min));
                        self.put_window(&first, egui::Rect::from_min_size(at, size));
                    }
                    pane_of(self, &first)
                }
                None => None,
            };
            for tab in tabs {
                match at {
                    _ if self.dock.find_tab(&tab).is_some() => {}
                    Some(at) => {
                        self.dock.set_focused_node_and_surface(at);
                        self.dock.push_to_focused_leaf(tab);
                    }
                    // (Where its kind opens.)
                    None => self.run_now(Action::OpenTab(tab)),
                }
            }
            if let Some(path) = pane.active.and_then(|t| self.dock.find_tab(&t)) {
                let _ = self.dock.set_active_tab(path);
            }
        }
        if let Some(front) = front {
            self.show_tab(front);
        }
        trace::note(format!("open tabs restored: {}", self.panes()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_are_named_and_read_back() {
        let key = |s: &str| parse_key(s).unwrap();
        let area = ResRef::from_str("field").unwrap();
        let path = GffPath::root().item("Creature List", 3);
        let tabs = [
            Tab::ModuleProperties,
            Tab::Script(key("hello.nss")),
            Tab::Blueprint(key("nw_chest.utp")),
            Tab::Blueprints(vec![key("a.uti"), key("b.uti")]),
            Tab::Dialog(key("talk.dlg")),
            Tab::Area(area),
            Tab::AreasProperties(vec![area, ResRef::from_str("cave").unwrap()]),
            Tab::Instance { area, path: path.clone() },
            Tab::InstanceModel { area, path: path.clone() },
            Tab::Instances { area, paths: vec![path, GffPath::root().item("Door List", 0)] },
        ];
        for tab in tabs {
            let text = tab_text(&tab).unwrap();
            assert_eq!(parse_tab(&text), Some(tab), "{text}");
        }
        assert_eq!(tab_text(&Tab::Palette), None);
        assert_eq!(parse_tab("script:"), None);
        assert_eq!(parse_tab("something-new:x"), None);
    }

    #[test]
    fn places_are_named_and_read_back() {
        for place in [Place::Window, Place::Middle, Place::Palettes, Place::Split(Side::Above, 300)]
        {
            assert_eq!(parse_place(&place_text(place)), Some(place));
        }
        assert_eq!(parse_place("split sideways 300"), None);
        assert_eq!(parse_place("split left 2000"), Some(Place::Split(Side::Left, 950)));
    }
}
