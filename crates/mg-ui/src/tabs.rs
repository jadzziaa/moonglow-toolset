//! The central area's tabs.

use std::collections::HashMap;

use egui::{Id, Ui, WidgetText};
use egui_dock::TabViewer;
use mg_core::ResType;
use mg_resman::ResKey;

use crate::side_doc::SideDoc;
use crate::{
    Action, Moonglow, area_view, browser, dialog_view, faction_view, gff_view, journal_view,
    model_view, module_props, script_view,
};

/// A tab in the central area.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tab {
    /// Shown with no module open.
    Welcome,
    /// Module Properties (module.ifo).
    ModuleProperties,
    /// The middle of the window with a module open and no area: it keeps
    /// the middle's place, so that the panes beside it keep theirs.
    NoArea,
    /// A script source.
    Script(ResKey),
    /// A NUI window (a JUI), in the NUI Creator.
    Nui(ResKey),
    /// Any GFF resource, as an editable field tree.
    Gff(ResKey),
    /// The Faction Editor.
    Factions,
    /// The Journal Editor.
    Journal,
    /// The module's custom talk table.
    TalkTable,
    /// A hak in the hak editor (its id among the open haks).
    Hak(u32),
    /// A 2DA in the table editor (its id among the open tables).
    Table(u32),
    /// A tileset in the tileset editor (its id among the open tilesets).
    Tileset(u32),
    /// A conversation.
    Dialog(ResKey),
    /// A resource from the load order, read-only.
    Resource(ResKey),
    /// A copy of a resource as one layer of the load order has it (a hak's
    /// table that a higher hak's overrides), read-only: by the layer's
    /// label.
    ResourceCopy { key: ResKey, layer: String },
    /// The model viewer on a layer's copy of a model or blueprint, which a
    /// higher layer overrides (the resource browser's Preview of it).
    ModelCopy { key: ResKey, layer: String },
    /// A model, in the 3D viewer.
    Model(ResKey),
    /// A blueprint in its Properties editor.
    Blueprint(ResKey),
    /// Several blueprints of one type, edited together.
    Blueprints(Vec<ResKey>),
    /// An area, in the area viewer.
    Area(mg_core::ResRef),
    /// An area's Properties.
    AreaProperties(mg_core::ResRef),
    /// Several areas' Properties, edited together.
    AreasProperties(Vec<mg_core::ResRef>),
    /// Several objects of one type placed in an area, edited together.
    Instances { area: mg_core::ResRef, paths: Vec<mg_edit::GffPath> },
    /// An object placed in an area (its entry in the area's GIT), in its
    /// Properties editor.
    Instance { area: mg_core::ResRef, path: mg_edit::GffPath },
    /// Help › User Manual.
    Manual,
    /// Find References: where something is used.
    References,
    /// Advanced Search: objects by what they hold.
    AdvancedSearch,
    /// The Placeable Gallery: every placeable appearance as a picture.
    PlaceableGallery,
    /// Tools › Options.
    Options,
    /// Tile Properties of the tiles selected in an area: their lights,
    /// loops and, of one tile, its variants.
    TileProperties,
    /// What an object placed in an area looks like, in the 3D viewer.
    InstanceModel { area: mg_core::ResRef, path: mg_edit::GffPath },
    /// A tile of a tileset (its index), to look at: its model and what
    /// the tileset says of it (a double click in the tiles palette's
    /// Gallery).
    Tile { tileset: mg_core::ResRef, tile: u32 },
    /// A plugin's panel (API 0.6): what its code returns, drawn.
    PluginPanel { plugin: String, panel: String },
}

/// How much taller a window that is mostly a form is in the suite's
/// theme than in egui's own, which the windows' sizes were first found
/// in: a form's lines are controls of 22 points with 4 between them,
/// where they were 18 with 3.
pub(crate) const FORMS_TALLER: f32 = (22.0 + 4.0) / (18.0 + 3.0);

/// How much taller a window that is mostly rows (a tree, a list, a
/// table, code) is: its rows are 22 points with none between, where they
/// were 18 with 3 (a point more each), and only its toolbars and the
/// few lines of form under its rows grew as a form's do (5 points each).
/// Worked out for the conversation window (its toolbars and Data page
/// about 35 points more, its tree a point a row: some 7 hundredths), and
/// looked at in the pictures of the conversation, script, journal, raw
/// fields, resources and manual windows at the sizes they open at
/// (`screens.rs`, `windows_at_their_sizes`): 8.
pub(crate) const ROWS_TALLER: f32 = 1.08;

impl Tab {
    /// Whether it opens in the main dock whatever Options › General › Open
    /// editors says: areas (and the start page), which the rest open over
    /// in windows of their own (As Floating Windows), so a window never
    /// takes the area view's place, or beside as tabs (As Tabs).
    pub fn docks(&self) -> bool {
        matches!(self, Tab::Area(_) | Tab::Welcome | Tab::NoArea)
    }

    /// Whether it opens in a window of its own however editors open
    /// (Options › General › Open editors), unless its kind was put
    /// elsewhere: Tile Properties, which follows the tiles chosen in an
    /// area view (a tab of the area's group would cover the tiles it
    /// edits), and the model views, opened to look at a model beside the
    /// editor it came from (a blueprint's Pop Out, the palette's Preview,
    /// a tile's model).
    pub(crate) fn floats(&self) -> bool {
        matches!(
            self,
            Tab::TileProperties
                | Tab::Model(_)
                | Tab::InstanceModel { .. }
                | Tab::ModelCopy { .. }
                | Tab::Tile { .. }
        )
    }

    /// Whether Escape is its Cancel (Aurora's Escape): the tabs with OK
    /// and Cancel that set back what they changed, floating (as the
    /// window in front) or docked (ruling R1: in front of its group, with
    /// the pointer or the keyboard in it). The side editors' (a hak's, a
    /// 2DA's), whose Cancel asks about unsaved work, are not among them.
    pub(crate) fn escape_cancels(&self) -> bool {
        matches!(
            self,
            Tab::Blueprint(_)
                | Tab::Blueprints(_)
                | Tab::Instance { .. }
                | Tab::Instances { .. }
                | Tab::AreaProperties(_)
                | Tab::AreasProperties(_)
                | Tab::ModuleProperties
                | Tab::Options
                | Tab::TileProperties
        )
    }

    /// What its kind was called before, where it was another: the
    /// encounters', sounds', triggers' and waypoints' blueprints were all
    /// `blueprint` until each type had its own. A size or place kept by
    /// that name is used until one is kept by the new.
    pub(crate) fn kind_before(&self) -> Option<&'static str> {
        match self {
            Tab::Blueprint(k) => {
                matches!(k.restype, ResType::UTE | ResType::UTS | ResType::UTT | ResType::UTW)
                    .then_some("blueprint")
            }
            _ => None,
        }
    }

    /// The kind of tab, whatever it shows: what its window's size is
    /// remembered by.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Tab::Welcome => "welcome",
            Tab::NoArea => "no-area",
            Tab::ModuleProperties => "module-properties",
            Tab::Script(_) => "script",
            Tab::Nui(_) => "nui",
            Tab::Gff(_) => "gff",
            Tab::Factions => "factions",
            Tab::Journal => "journal",
            Tab::TalkTable => "talk-table",
            Tab::Hak(_) => "hak",
            Tab::Table(_) => "table",
            Tab::Tileset(_) => "tileset",
            Tab::Dialog(_) => "dialog",
            Tab::Resource(_) | Tab::ResourceCopy { .. } => "resource",
            Tab::Model(_) | Tab::InstanceModel { .. } | Tab::ModelCopy { .. } => "model",
            Tab::Blueprint(k) => {
                mg_area::ObjectKind::from_restype(k.restype).map_or("blueprint", |k| k.name())
            }
            Tab::Blueprints(_) => "blueprints",
            Tab::Area(_) => "area",
            Tab::AreaProperties(_) | Tab::AreasProperties(_) => "area-properties",
            Tab::Instances { .. } | Tab::Instance { .. } => "instance",
            Tab::Manual => "manual",
            Tab::References => "references",
            Tab::AdvancedSearch => "advanced-search",
            Tab::PlaceableGallery => "placeable-gallery",
            Tab::Options => "options",
            Tab::TileProperties => "tile-properties",
            Tab::Tile { .. } => "tile",
            Tab::PluginPanel { .. } => "plugin-panel",
        }
    }

    /// The kind of window this tab gathers in with others of its kind,
    /// where its option says so (Options: scripts, Properties,
    /// conversations): 0 scripts, 1 Properties, 2 conversations; and 3
    /// NUI windows, which have no option.
    pub(crate) fn gathers(&self) -> Option<u8> {
        match self {
            Tab::Script(_) => Some(0),
            Tab::Blueprint(_)
            | Tab::Blueprints(_)
            | Tab::Instance { .. }
            | Tab::Instances { .. } => Some(1),
            Tab::Dialog(_) => Some(2),
            Tab::Nui(_) => Some(3),
            _ => None,
        }
    }

    /// The size its own window opens at (before fitting the screen).
    pub(crate) fn window_size(&self) -> egui::Vec2 {
        let (w, h) = match self {
            // (Room for the text beside the lists, which open as wide as
            // their longest name.)
            Tab::Script(_) => (1120.0, 720.0),
            Tab::Nui(_) => (1140.0, 792.0),
            Tab::Manual | Tab::Resource(_) | Tab::ResourceCopy { .. } => (920.0, 700.0),
            Tab::Dialog(_) => (920.0, 692.0),
            Tab::Tileset(_) | Tab::Hak(_) => (920.0, 720.0),
            Tab::Table(_) => (1040.0, 732.0),
            Tab::Model(_) | Tab::InstanceModel { .. } | Tab::ModelCopy { .. } => (780.0, 680.0),
            Tab::Factions | Tab::Journal => (780.0, 632.0),
            Tab::Gff(_) => (780.0, 600.0),
            Tab::ModuleProperties | Tab::AreaProperties(_) => (760.0, 612.0),
            Tab::Options => crate::options::WINDOW_SIZE.into(),
            Tab::TileProperties => (560.0, 640.0),
            Tab::PluginPanel { .. } => (380.0, 560.0),
            // (Each with OK and Cancel under it: 32 points of its height.)
            Tab::Blueprint(_)
            | Tab::Blueprints(_)
            | Tab::Instance { .. }
            | Tab::Instances { .. } => (820.0, 672.0),
            _ => (820.0, 640.0),
        };
        // (The heights were found for controls of 18 points with 3
        // between: as much taller as the window's lines are now.)
        egui::vec2(w, (h * self.taller() / 4.0).round() * 4.0)
    }

    /// How much taller its window is than the size found for egui's own
    /// style: [`FORMS_TALLER`] for a window that is mostly a form,
    /// [`ROWS_TALLER`] for one that is mostly rows.
    pub(crate) fn taller(&self) -> f32 {
        match self {
            Tab::ModuleProperties
            | Tab::AreaProperties(_)
            | Tab::AreasProperties(_)
            | Tab::Options
            | Tab::TileProperties
            | Tab::Blueprint(_)
            | Tab::Blueprints(_)
            | Tab::Instance { .. }
            | Tab::Instances { .. }
            | Tab::Factions
            | Tab::AdvancedSearch
            | Tab::PluginPanel { .. } => FORMS_TALLER,
            _ => ROWS_TALLER,
        }
    }

    /// [`Tab::taller`] of the tabs whose [`Tab::kind`] is `kind`, as a
    /// remembered window size names them (`window_sizes`): `None` for a
    /// kind that never has a window of its own (an area, the Welcome
    /// page) or one this version does not know.
    pub(crate) fn taller_of_kind(kind: &str) -> Option<f32> {
        const FORMS: [&str; 10] = [
            "module-properties",
            "area-properties",
            "options",
            "tile-properties",
            "blueprint",
            "blueprints",
            "instance",
            "factions",
            "advanced-search",
            "plugin-panel",
        ];
        const ROWS: [&str; 15] = [
            "script",
            "nui",
            "gff",
            "journal",
            "talk-table",
            "hak",
            "table",
            "tileset",
            "dialog",
            "resource",
            "model",
            "manual",
            "references",
            "placeable-gallery",
            "tile",
        ];
        let blueprint = mg_area::ObjectKind::ALL.iter().any(|k| k.name() == kind);
        if FORMS.contains(&kind) || blueprint {
            Some(FORMS_TALLER)
        } else if ROWS.contains(&kind) {
            Some(ROWS_TALLER)
        } else {
            None
        }
    }

    /// The tab that edits a resource.
    pub fn for_resource(key: ResKey) -> Option<Tab> {
        match key.restype {
            ResType::NSS => Some(Tab::Script(key)),
            ResType::JUI => Some(Tab::Nui(key)),
            ResType::ARE => Some(Tab::Area(key.resref)),
            ResType::IFO => Some(Tab::ModuleProperties),
            ResType::FAC => Some(Tab::Factions),
            ResType::JRL => Some(Tab::Journal),
            ResType::DLG => Some(Tab::Dialog(key)),
            ResType::MDL => Some(Tab::Model(key)),
            t if crate::blueprint::has_editor(t) => Some(Tab::Blueprint(key)),
            t if t.is_gff() => Some(Tab::Gff(key)),
            _ => None,
        }
    }

    /// Whether it has a close button (and Close in its menu).
    pub fn closeable(&self) -> bool {
        !matches!(self, Tab::Welcome | Tab::NoArea)
    }

    /// The resource of the module tree's that the tab shows: what the tree
    /// goes to when it is chosen (an object placed in an area: the area).
    pub fn tree_key(&self) -> Option<ResKey> {
        let area = |r: &mg_core::ResRef| Some(ResKey::new(*r, ResType::ARE));
        match self {
            Tab::Area(r) | Tab::AreaProperties(r) => area(r),
            Tab::Instance { area: r, .. }
            | Tab::Instances { area: r, .. }
            | Tab::InstanceModel { area: r, .. } => area(r),
            Tab::Script(k) | Tab::Nui(k) | Tab::Dialog(k) | Tab::Blueprint(k) | Tab::Gff(k) => {
                Some(*k)
            }
            Tab::Journal => ResKey::parse("module", ResType::JRL),
            Tab::Factions => ResKey::parse("repute", ResType::FAC),
            _ => None,
        }
    }

    /// The resources the tab shows that the module tree lists:
    /// [`Tab::tree_key`]'s, or each of a tab of several (Properties of
    /// several blueprints or areas).
    pub fn tree_keys(&self) -> Vec<ResKey> {
        match self {
            Tab::Blueprints(keys) => keys.clone(),
            Tab::AreasProperties(areas) => {
                areas.iter().map(|a| ResKey::new(*a, ResType::ARE)).collect()
            }
            _ => self.tree_key().into_iter().collect(),
        }
    }

    /// Whether the tab shows one of `gone`, resources deleted or whose
    /// files went outside Moonglow ([`Tab::tree_keys`]'s; a tab of
    /// several has no place for one that is gone, and closes with it).
    /// The journal's and the factions' tabs stay: theirs is made again.
    pub(crate) fn shows_any(&self, gone: &[ResKey]) -> bool {
        !matches!(self, Tab::Journal | Tab::Factions)
            && self.tree_keys().iter().any(|k| gone.contains(k))
    }

    /// Points the tab at resources renamed: each resource it shows that
    /// the module tree lists ([`Tab::tree_keys`]'s, in a tab of several
    /// each of them) becomes what `moved` gives for it, where it gives
    /// one. The one place that knows which tabs show a resource, with
    /// `tree_key`: Rename, Delete and files gone outside each had a list
    /// of their own, and the lists differed.
    pub(crate) fn retarget(&mut self, moved: &dyn Fn(ResKey) -> Option<ResKey>) {
        let area = |r: &mut mg_core::ResRef| {
            if let Some(to) = moved(ResKey::new(*r, ResType::ARE)) {
                *r = to.resref;
            }
        };
        match self {
            Tab::Area(r) | Tab::AreaProperties(r) => area(r),
            Tab::Instance { area: r, .. }
            | Tab::Instances { area: r, .. }
            | Tab::InstanceModel { area: r, .. } => area(r),
            Tab::AreasProperties(areas) => areas.iter_mut().for_each(area),
            Tab::Script(k) | Tab::Nui(k) | Tab::Dialog(k) | Tab::Blueprint(k) | Tab::Gff(k) => {
                *k = moved(*k).unwrap_or(*k);
            }
            Tab::Blueprints(keys) => {
                for k in keys {
                    *k = moved(*k).unwrap_or(*k);
                }
            }
            _ => {}
        }
    }

    /// The resource the tab shows that can be renamed (a script, an area, a
    /// conversation, a blueprint, a GFF file), whether or not the module
    /// has it.
    pub fn renamable(&self) -> Option<ResKey> {
        match self {
            Tab::Script(k) | Tab::Dialog(k) | Tab::Blueprint(k) | Tab::Gff(k) | Tab::Nui(k) => {
                Some(*k)
            }
            Tab::Area(r) | Tab::AreaProperties(r) => Some(ResKey::new(*r, ResType::ARE)),
            _ => None,
        }
    }
}

/// Closes every tab that `keep` turns down, one at a time as its close
/// button does, so that a window left without tabs goes with its last.
/// (`DockState::retain_tabs` left such a window with an empty pane and
/// egui_dock panicked drawing it: deleting a conversation open in its
/// window closed Moonglow, Sacha's report.)
pub(crate) fn close_tabs(dock: &mut egui_dock::DockState<Tab>, keep: impl Fn(&Tab) -> bool) {
    while let Some(path) = dock.find_tab_from(|t| !keep(t)) {
        if dock.remove_tab(path).is_none() {
            break;
        }
    }
}

pub(crate) struct Viewer<'a> {
    pub(crate) app: &'a mut Moonglow,
}

impl Viewer<'_> {
    /// Whether an Escape pressed now would be `tab`'s Cancel (a docked tab
    /// with OK and Cancel, on `layer`), by the rule `body` takes it by
    /// (ruling R1, T10's): for the Status Bar's Esc hint (M3's T12), so
    /// that it shows for the tab that would in truth take the key. Known
    /// only where nothing has the keyboard (where something has, which
    /// tab's it is shows only at the Escape, egui's focus gone by then:
    /// no hint), no list drawn before the editor area has the keys (the
    /// module tree, the Resources panel, the log: each takes Escape
    /// first), no dialog, menu or popup is open, no button is held,
    /// Options › Keyboard is not waiting for a key and no tool is in hand
    /// in an area (`busy`); then it is the tab's where the last press was
    /// in it, or the pointer is over it and the last press was in no
    /// window and no other tab with OK and Cancel.
    fn foresee_escape(&mut self, ui: &Ui, tab: &Tab, layer: egui::LayerId, busy: bool) {
        // (A list drawn before the editor area with the keys takes Escape
        // first: the module tree's, the Resources panel's, the log's.)
        use crate::palette_view::{Keys, keys_are};
        let list =
            [Keys::Tree, Keys::Browser, Keys::Log].into_iter().any(|k| keys_are(ui.ctx(), k));
        let quiet = !busy
            && !list
            && !crate::widgets::dialog_open(ui.ctx())
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.memory(|m| m.focused().is_none())
            && !ui.input(|i| i.pointer.any_down());
        if !quiet || self.app.escape_hint_next.is_some() {
            return;
        }
        let pointer = ui.rect_contains_pointer(ui.max_rect());
        let at = self.app.last_press.map(|(p, l)| (l, p));
        let in_it = at.is_some_and(|(l, p)| l == Some(layer) && ui.max_rect().contains(p));
        let takes = at.is_some_and(|(l, p)| {
            l.is_some_and(|l| l.order == egui::Order::Middle)
                || (self.app.cancel_rects.iter()).any(|(c, r)| l == Some(*c) && r.contains(p))
        });
        if in_it || (!takes && pointer) {
            self.app.escape_hint_next = Some(tab.clone());
        }
    }

    /// What a tab shows.
    fn body(&mut self, ui: &mut Ui, tab: &mut Tab) {
        crate::trace::changed(&format!("drawn {tab:?}"), || format!("in {:?}", ui.max_rect()));
        // (The dock's windows have no margin: `Moonglow::ui`. What a tab
        // shows has the usual one.)
        ui.spacing_mut().window_margin = ui.ctx().global_style().spacing.window_margin;
        // Escape closes a model's window (opened to look, and in the way
        // after), while the pointer is over it and nothing is being typed;
        // and a Properties window, when it is the one in front, as its
        // Cancel does (Aurora's Escape is its Cancel: what the window
        // changed is set back). Dialogs over it close first.
        // (Not during a drag, which Escape drops.)
        // (Escape in a text or number field is Cancel too, as in Aurora;
        // in a search field (an inventory's Find) it is the field's, which
        // clears what is typed: egui has taken the keyboard from it by
        // now, `escape_left_search`.)
        let escape = crate::widgets::escape_past_dialogs(ui.ctx())
            && !crate::widgets::escape_left_search(ui.ctx())
            && ui.memory(|m| m.focused().is_none())
            && !ui.input(|i| i.pointer.any_down());
        let layer = ui.layer_id();
        let floating = layer.order == egui::Order::Middle;
        // (A model's view docked, as a tab of an editor group, is a tab as
        // any: Escape does not close it, ruling R1.)
        let model = floating
            && matches!(
                tab,
                Tab::Model(_)
                    | Tab::InstanceModel { .. }
                    | Tab::ModelCopy { .. }
                    | Tab::Tile { .. }
            )
            && ui.rect_contains_pointer(ui.max_rect());
        // (The keyboard in a docked tab with OK and Cancel, drawn before
        // the windows: that tab's Escape, not the window's in front.)
        let front = floating
            && self.app.escape_docked_seen.is_none()
            && ui.memory(|m| {
                // (The last of the windows showing: closed ones keep their
                // place in the order.)
                let showing =
                    |l: &egui::LayerId| l.order == egui::Order::Middle && m.areas().is_visible(l);
                m.layer_ids().filter(showing).last()
            }) == Some(layer)
            && tab.escape_cancels();
        // (With a tool in hand in the area, Escape over the area drops the
        // tool and no more; over this window, it closes the window.)
        let tool = ui.ctx().cumulative_pass_nr().saturating_sub(self.app.area_tool_at) <= 1
            && !ui.rect_contains_pointer(ui.max_rect());
        // (Options › Keyboard, waiting for a key, takes Escape itself.)
        let recording = *tab == Tab::Options
            && self.app.options.as_ref().is_some_and(|o| o.recording.is_some());
        // A docked tab with OK and Cancel (ruling R1): Escape is its Cancel
        // too, in front of its group, where the keyboard is in it (the
        // widget with the keyboard, else the last press: `keyboard_at`),
        // whatever the pointer is over; or, the keyboard in nothing that
        // takes an Escape (no widget had it, and the last press was in no
        // window and no other tab with OK and Cancel), with the pointer
        // over it (nothing in front of it there). An Escape something drawn
        // before it took (the module tree's Filter, the log, a field of its
        // own) is not its. Seen now, it is taken in the next frame where no
        // window took this one and the key is still there once the editor
        // area is drawn (`Moonglow::escape_docked`).
        if !floating && tab.escape_cancels() {
            self.app.cancel_rects_next.push((layer, ui.max_rect()));
            self.foresee_escape(ui, tab, layer, tool || recording);
        }
        if *tab == Tab::Options && ui.rect_contains_pointer(ui.max_rect()) {
            self.app.options_hovered_next = true;
        }
        if escape && !floating && !recording && !tool && tab.escape_cancels() {
            let pointer = ui.rect_contains_pointer(ui.max_rect());
            let at = self.app.keyboard_at;
            let in_it = at.is_some_and(|(l, p)| l == Some(layer) && ui.max_rect().contains(p));
            let takes = at.is_some_and(|(l, p)| {
                l.is_some_and(|l| l.order == egui::Order::Middle)
                    || (self.app.cancel_rects.iter()).any(|(c, r)| l == Some(*c) && r.contains(p))
            });
            if in_it || (!self.app.keyboard_focused && !takes && pointer) {
                self.app.escape_docked_seen = Some(tab.clone());
            }
        }
        let docked = self.app.escape_docked.as_ref() == Some(tab) && !floating;
        if (escape && !recording && (model || (front && !tool))) || docked {
            if !docked {
                self.app.escape_taken = true;
            }
            self.app.escape_docked = None;
            // (Tile Properties with changes not applied asks first, as
            // its X does: `may_close`.)
            // (Options' Cancel is its close, which drops what wasn't
            // applied.)
            if *tab == Tab::TileProperties {
                self.app.actions.push(crate::Action::CloseTabs(vec![tab.clone()]));
            } else if model || *tab == Tab::Options {
                // (Options with changes not applied says they are dropped.)
                let draft = self.app.options.as_ref().filter(|_| *tab == Tab::Options);
                if draft.is_some_and(|d| d.apply(&self.app.settings) != self.app.settings) {
                    let name = name(self.app, tab);
                    self.app.log.info(format!("Escape canceled {name}: its changes are set back."));
                }
                self.app.actions.push(crate::Action::CloseTab(tab.clone()));
            } else {
                self.app.escape_cancel = Some(tab.clone());
            }
        }
        // (The pane clicked in last, whose selection a plugin's command
        // from the Plugins menu or a key runs on.)
        if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
            // (The palettes' press is the Right Dock's: `shell::right_dock`.)
            if let Tab::Area(area) = tab {
                self.app.plugins.pane = Some(crate::plugins::Pane::Area(*area));
            }
        }
        // (A side editor's Undo and Redo are its own with the pointer over
        // it.)
        if crate::side_doc::is_side(tab) {
            crate::side_doc::note_pointer(self.app, ui, tab);
        }
        match tab {
            Tab::Welcome => welcome(self.app, ui),
            Tab::NoArea => no_area(self.app, ui),
            Tab::ModuleProperties => module_props::ui(self.app, ui),
            Tab::Script(k) => script_view::ui(self.app, ui, *k),
            Tab::Nui(k) => crate::nui_view::ui(self.app, ui, *k),
            Tab::Gff(k) => gff_view::ui(self.app, ui, *k),
            Tab::Blueprint(k) => crate::blueprint::ui(self.app, ui, *k),
            Tab::Factions => faction_view::ui(self.app, ui),
            Tab::Journal => journal_view::ui(self.app, ui),
            Tab::TalkTable => crate::talk_view::ui(self.app, ui),
            Tab::Hak(id) => crate::hak_view::ui(self.app, ui, *id),
            Tab::Table(id) => crate::table_view::ui(self.app, ui, *id),
            Tab::Tileset(id) => crate::tileset_view::ui(self.app, ui, *id),
            Tab::Dialog(k) => dialog_view::ui(self.app, ui, *k),
            Tab::Resource(k) => browser::resource_ui(self.app, ui, *k, None),
            Tab::ResourceCopy { key, layer } => {
                browser::resource_ui(self.app, ui, *key, Some(layer.as_str()))
            }
            // A blueprint without a model: what there is to see of it.
            Tab::Model(k) if !model_view::previewable(k.restype) => {
                crate::area_tools::summary_view(self.app, ui, *k)
            }
            Tab::Model(k) => model_view::ui(self.app, ui, model_view::Source::Resource(*k)),
            Tab::InstanceModel { area, path } => {
                let source = model_view::Source::Instance { area: *area, path: path.clone() };
                model_view::ui(self.app, ui, source);
            }
            Tab::ModelCopy { key, layer } => {
                let source = model_view::Source::Copy { key: *key, layer: layer.clone() };
                model_view::ui(self.app, ui, source);
            }
            Tab::Area(r) => area_view::ui(self.app, ui, *r),
            Tab::AreaProperties(area) => crate::area_props::ui(self.app, ui, *area, &[]),
            Tab::AreasProperties(areas) => {
                let (first, rest) = areas.split_first().expect("at least one");
                crate::area_props::ui(self.app, ui, *first, rest);
            }
            Tab::Instances { area, paths } => {
                let git = ResKey::new(*area, ResType::GIT);
                let (first, rest) = paths.split_first().expect("at least one");
                let rest = rest.iter().map(|p| (git, p.clone())).collect();
                crate::blueprint::edit_many(self.app, ui, git, first.clone(), rest);
            }
            Tab::Blueprints(keys) => {
                let (first, rest) = keys.split_first().expect("at least one");
                let root = mg_edit::GffPath::root();
                let rest = rest.iter().map(|k| (*k, root.clone())).collect();
                crate::blueprint::edit_many(self.app, ui, *first, root, rest);
            }
            Tab::Instance { area, path } => {
                let git = ResKey::new(*area, ResType::GIT);
                crate::blueprint::edit(self.app, ui, git, path.clone());
            }
            Tab::Manual => crate::manual::ui(self.app, ui),
            Tab::References => crate::references::ui(self.app, ui),
            Tab::AdvancedSearch => crate::search::ui(self.app, ui),
            Tab::PlaceableGallery => crate::appearance_gallery::ui(self.app, ui),
            Tab::Options => crate::options::ui(self.app, ui),
            Tab::TileProperties => crate::tile_select::ui(self.app, ui),
            Tab::Tile { tileset, tile } => crate::tile_view::ui(self.app, ui, *tileset, *tile),
            Tab::PluginPanel { plugin, panel } => {
                crate::plugin_kinds::panel_ui(self.app, ui, plugin, panel)
            }
        }
    }
}

impl TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> Id {
        // (A placed object's window by its own id, which it keeps as it
        // follows its object to another place.)
        let window = |app: &mut Moonglow, area, path: &mg_edit::GffPath| {
            crate::blueprint::follow::window_id(app, area, path)
        };
        match tab {
            Tab::Instance { area, path } => window(self.app, *area, path).with("instance"),
            Tab::InstanceModel { area, path } => window(self.app, *area, path).with("model"),
            Tab::Instances { area, paths } if !paths.is_empty() => {
                window(self.app, *area, &paths[0]).with(("instances", paths.len()))
            }
            _ => Id::new(("tab", &*tab)),
        }
    }

    /// The tab's title as [`prepare`] laid it out this frame (its name,
    /// cut short where its bar is crowded, with room for its kind icon,
    /// modified dot and close button).
    fn title(&mut self, tab: &mut Tab) -> WidgetText {
        match self.app.doc_tabs.get(tab) {
            Some(look) => WidgetText::Galley(look.title.galley.clone()),
            // (Not prepared: one opened as the dock is drawn.)
            None => name(self.app, tab).into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Tab) {
        // A little room round what a tab shows. (The dock is given no window
        // margin, `Moonglow::ui`, and takes its tabs' from that: without
        // this they sat flush against their pane's edge.)
        let margin = egui::Margin { left: 6, right: 6, top: 2, bottom: 6 };
        // In a window of its own, a drag on what isn't a control is the
        // tab's, behind all it shows, not the window's (egui_dock's windows
        // have no title bar, so egui moves them by any part): the window
        // moves by its tab bar alone (Taro49's report: a click beside a
        // control moved the conversation editor), unless Options › General
        // asks for anywhere. (The main window's tabs don't move.)
        if !self.app.settings.drag_windows_anywhere
            && ui.layer_id().order != egui::Order::Background
        {
            // (Not a stop of Tab: `Sense::DRAG` alone.)
            ui.interact(ui.max_rect(), ui.id().with("no-window-drag"), egui::Sense::DRAG);
        }
        let framed = |viewer: &mut Self, ui: &mut Ui, tab: &mut Tab| {
            egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
                // (All the room, so a view that fills its tab still does.)
                ui.set_min_size(ui.available_size());
                viewer.body(ui, tab);
            });
        };
        if fits(tab) {
            framed(self, ui, tab);
            return;
        }
        // The others scroll when wider or taller, in a scroll area of
        // Moonglow's own, not egui_dock's: that one stretches what a tab
        // shows to its room rounded to pixels, a sliver past itself at a
        // display scale such as 125%, and at some sizes its scroll bars
        // came and went every frame and the tab with them (GitHub issue 38,
        // and Troubletcat's report after 3.1.0 in the area view beside a
        // conversation). What the tab shows is held to the room in sight
        // in whole pixels: only what is truly wider or taller scrolls.
        let ppp = ui.ctx().pixels_per_point();
        egui::ScrollArea::both().id_salt("tab-body").auto_shrink(false).show_viewport(
            ui,
            |ui, sight| {
                let room = (sight.size() * ppp).floor() / ppp;
                let layout = egui::Layout::top_down(egui::Align::Min);
                ui.allocate_ui_with_layout(room, layout, |ui| framed(self, ui, tab));
            },
        );
    }

    /// The tab's menu (before egui_dock's Eject): Rename… for a resource of
    /// the module, which renames it everywhere it is named, and the rest.
    fn context_menu(&mut self, ui: &mut Ui, tab: &mut Tab, _path: egui_dock::NodePath) {
        mg_uikit::menu_rows(ui);
        let ours = tab
            .renamable()
            .filter(|k| self.app.ws.as_ref().is_some_and(|ws| ws.module.contains(k)));
        if let Some(key) = ours
            && ui.button("Rename…").clicked()
        {
            self.app.actions.push(crate::Action::RenameDialog(key));
            ui.close();
        }
        // Its row in the module tree (an area's opened out to what is
        // placed in it).
        let in_tree = tab.tree_key().filter(|k| crate::tree::listed(self.app, k));
        if let Some(key) = in_tree
            && ui
                .button("Show in Module Tree")
                .on_hover_text(if key.restype == ResType::ARE {
                    "Its row in the module tree, opened out to what is placed in it"
                } else {
                    "Its row in the module tree"
                })
                .clicked()
        {
            self.app.tree_reveal = Some((key, true));
            // (The Left Dock on the Module panel, the tree's, where it
            // showed the Resources panel.)
            self.app.left_panel = crate::Panel::Module;
            ui.close();
        }
        // Claim This Area or Release Claim (the project's claims table).
        if let Tab::Area(area) = tab
            && crate::claims::menu_items(
                crate::claims::of(&self.app.ws),
                &mut self.app.actions,
                ui,
                *area,
            )
        {
            ui.close();
        }
        // A window of its own fills the main pane, and goes back.
        if !tab.docks() {
            let label = if self.app.maximized.contains_key(tab) { "Restore" } else { "Maximize" };
            if ui.button(label).on_hover_text("Also: double-click the tab").clicked() {
                self.app.actions.push(crate::Action::ToggleMaximize(tab.clone()));
                ui.close();
            }
        }
        // The plugins' commands for what it edits (a Properties window's
        // object, blueprint or area, or several edited together; another
        // editor's resource), run on it.
        let edited = crate::plugins::tab_selection(tab);
        let commands = self.app.plugin_menu_commands();
        if !edited.is_empty() && !commands.is_empty() {
            crate::plugins::menu_on(ui, &commands, &edited, &mut self.app.actions);
        }
        // Closing it, and those of its pane.
        if tab.closeable() {
            let pane: Vec<Tab> = (self.app.panes_tabs.iter())
                .find(|tabs| tabs.contains(tab))
                .cloned()
                .unwrap_or_else(|| vec![tab.clone()]);
            let at = pane.iter().position(|t| t == tab).unwrap_or(0);
            let closing = |tabs: &[Tab]| -> Vec<Tab> {
                tabs.iter().filter(|t| t.closeable()).cloned().collect()
            };
            let others: Vec<Tab> = closing(&pane).into_iter().filter(|t| t != tab).collect();
            let right = closing(&pane[at + 1..]);
            let mut close = None;
            // (Its own Close: egui_dock's goes with its close buttons,
            // which the document tabs draw themselves.)
            if ui.button("Close").clicked() {
                close = Some(vec![tab.clone()]);
            }
            if ui.add_enabled(!others.is_empty(), egui::Button::new("Close Others")).clicked() {
                close = Some(others);
            }
            if ui
                .add_enabled(!right.is_empty(), egui::Button::new("Close Tabs to the Right"))
                .clicked()
            {
                close = Some(right);
            }
            if ui.button("Close All").on_hover_text("Every tab of this pane").clicked() {
                close = Some(closing(&pane));
            }
            if let Some(tabs) = close {
                self.app.actions.push(crate::Action::CloseTabs(tabs));
                ui.close();
            }
        }
    }

    /// A double click on a window's tab maximizes it, and restores it.
    fn on_tab_button(&mut self, tab: &mut Tab, response: &egui::Response) {
        // (With the part of it in sight: egui_dock scrolls a full bar and
        // still gives each tab out of sight its button.)
        let shown = response.rect.intersect(response.interact_rect);
        self.app.tab_buttons.push((tab.clone(), response.rect, response.layer_id, shown));
        // A tab by its name for a screen reader (egui_dock names none), said
        // selected where it is its pane's in front, and "Modified" where
        // its document has unsaved changes; its whole name in its tip where
        // it is cut short.
        if let Some(look) = self.app.doc_tabs.get(tab) {
            let front = self.app.fronts.contains(tab);
            let name = look.name.clone();
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, front, &name)
            });
            let modified = look.modified;
            response.ctx.accesskit_node_builder(response.id, |node| {
                node.set_role(egui::accesskit::Role::Tab);
                node.set_selected(front);
                if modified {
                    node.set_description("Modified");
                }
            });
            if look.title.cut {
                let _ = response.clone().on_hover_text(&look.name);
            }
        }
        // (A press on a side editor's tab is work in it: the active
        // editor, `Moonglow::note_worked`.)
        let side = crate::side_doc::is_side(tab) || *tab == Tab::TalkTable;
        if side && response.contains_pointer() && response.ctx.input(|i| i.pointer.any_pressed()) {
            self.app.pressed_side_tab = Some(tab.clone());
        }
        // A tab chosen (the one in front too): the module tree goes to
        // what it shows.
        if response.clicked() {
            crate::tree::reveal_tab(self.app, tab);
        }
        if response.middle_clicked() && tab.closeable() {
            self.app.actions.push(crate::Action::CloseTabs(vec![tab.clone()]));
        }
        if response.double_clicked() && !tab.docks() {
            self.app.actions.push(crate::Action::ToggleMaximize(tab.clone()));
        }
    }

    /// The tab being dragged keeps its kind icon, dot and close button,
    /// painted where it stood on the layer that follows the pointer.
    fn on_tab_dragged(
        &mut self,
        ctx: &egui::Context,
        tab: &mut Tab,
        layer: egui::LayerId,
        rect: egui::Rect,
    ) {
        self.app.dragged_tab = Some(tab.clone());
        let Some(look) = self.app.doc_tabs.get(tab) else { return };
        let mark = mg_uikit::dock::DocTabMark {
            tab: (),
            rect,
            layer,
            shown: rect,
            icon: look.icon,
            title_width: look.title.width,
            modified: look.modified,
            active: true,
            close: tab.closeable().then_some(mg_uikit::icons::ButtonState::Rest),
        };
        mg_uikit::dock::paint_doc_tab_parts(&ctx.layer_painter(layer), &mark);
    }

    /// A window's collapse arrow and close all in the Floating board's look
    /// (`.fw-collapse` and the bar's Close: icon buttons of 22 points, the
    /// arrow a chevron with a `line` after it), named and tipped as the
    /// style guide has it. egui_dock still does what they do.
    fn window_bar_button(
        &mut self,
        ui: &mut Ui,
        button: egui_dock::WindowBarButton,
        response: &egui::Response,
    ) -> bool {
        paint_window_bar_button(ui, button, response);
        true
    }

    /// A side editor with unsaved changes asks first; a closed one is let
    /// go.
    fn on_close(&mut self, tab: &mut Tab) -> egui_dock::tab_viewer::OnCloseResponse {
        use egui_dock::tab_viewer::OnCloseResponse;
        if self.app.may_close(tab) {
            self.app.closed(tab);
            OnCloseResponse::Close
        } else {
            OnCloseResponse::Focus
        }
    }

    fn is_closeable(&self, tab: &Tab) -> bool {
        tab.closeable()
    }
    /// The script editor scrolls its own text, lists and messages, and fits
    /// its pane (in a pane that scrolled, its side lists ran past the pane's
    /// edge and were cut off); so does the manual, whose text wraps at the
    /// pane's width, and the Options, whose page scrolls between its panels;
    /// and the properties windows and the journal, conversation, talk
    /// table and tileset editors, whose page scrolls over OK and Cancel at
    /// their foot (in a pane that scrolled, at a display scale of 125% the
    /// pane's scroll bars came and went each frame at some sizes, egui's
    /// content rounded to pixels a sliver past it, and the foot went up
    /// and down with them: GitHub issue 38). The others scroll when wider
    /// or taller.
    fn scroll_bars(&self, _tab: &Tab) -> [bool; 2] {
        // (None of egui_dock's: those that scroll have Moonglow's, `ui`.)
        [false, false]
    }
}

/// A tab's name: its title, with no mark of unsaved changes (the tab's
/// modified dot says that, [`Look::modified`]): what its tab shows and a
/// screen reader says.
pub(crate) fn name(app: &mut Moonglow, tab: &Tab) -> String {
    match tab {
        Tab::Welcome => "Welcome".into(),
        Tab::NoArea => "No Area Open".into(),
        Tab::Nui(k) => k.to_string(),
        Tab::ModuleProperties => "Module Properties".into(),
        Tab::Script(k) => k.to_string(),
        Tab::Gff(k) => k.to_string(),
        Tab::Blueprint(k) if app.ws.as_ref().is_some_and(|ws| ws.is_viewed(k)) => {
            format!("{k} (the game's: view only)")
        }
        Tab::Blueprint(k) => k.to_string(),
        Tab::Factions => "Factions".into(),
        Tab::Journal => "Journal".into(),
        Tab::Tileset(id) => {
            let doc = app.tilesets.iter().find(|d| d.id == *id);
            doc.map_or("Tileset".into(), SideDoc::title)
        }
        Tab::Hak(id) => {
            let doc = app.haks.iter().find(|d| d.id == *id);
            doc.map_or("Hak".into(), SideDoc::title)
        }
        Tab::Table(id) => {
            let doc = app.tables.iter().find(|d| d.id == *id);
            doc.map_or("2DA".into(), SideDoc::title)
        }
        Tab::TalkTable => "Talk Table".into(),
        Tab::Dialog(k) => k.to_string(),
        Tab::Resource(k) => format!("{k} (read-only)"),
        Tab::ResourceCopy { key, layer } => format!("{key} from {layer} (read-only)"),
        Tab::Model(k) => k.to_string(),
        Tab::ModelCopy { key, layer } => format!("{key} from {layer}"),
        Tab::Area(r) => area_label(app, *r),
        Tab::Instance { area, path } => instance_title(app, *area, path),
        Tab::AreaProperties(area) => {
            format!("{} (Area Properties)", area_label(app, *area))
        }
        Tab::AreasProperties(areas) => {
            format!("{} areas (Area Properties)", areas.len())
        }
        Tab::Instances { area, paths } => format!("{} objects ({area})", paths.len()),
        Tab::Blueprints(keys) => {
            let ext = keys[0].restype.extension().unwrap_or_default();
            format!("{} blueprints (.{ext})", keys.len())
        }
        Tab::Manual => "User Manual".into(),
        Tab::References => "References".into(),
        Tab::AdvancedSearch => "Advanced Search".into(),
        Tab::PlaceableGallery => "Appearance Gallery".into(),
        Tab::Options => "Options".into(),
        Tab::TileProperties => "Tile Properties".into(),
        Tab::Tile { tileset, tile } => {
            // (As an open area has the tileset: not read for a title.)
            let model = (app.open_tileset(*tileset))
                .and_then(|t| t.set.tiles.get(*tile as usize).map(|t| t.model.clone()));
            format!("{} (tile)", model.unwrap_or_else(|| format!("{tileset} tile {tile}")))
        }
        Tab::InstanceModel { area, path } => {
            format!("{} (preview)", instance_title(app, *area, path))
        }
        Tab::PluginPanel { plugin, panel } => crate::plugin_kinds::panel_title(app, plugin, panel),
    }
}

// ------------------------------------------------------------------ the document tabs' look

/// A document tab as this frame draws it ([`prepare`]): its kind icon, its
/// name, whether its own document has unsaved changes (its modified dot),
/// and its title as laid out for egui_dock.
#[derive(Clone, Debug)]
pub(crate) struct Look {
    pub(crate) icon: mg_uikit::Icon,
    pub(crate) name: String,
    pub(crate) modified: bool,
    pub(crate) title: mg_uikit::dock::DocTabTitle,
}

/// What [`prepare`] keeps from frame to frame: the titles laid out (by
/// what they were laid out from), and whether each module resource shown
/// in a tab differs from the module as saved (by when it last changed).
#[derive(Debug, Default)]
pub(crate) struct LookCache {
    titles: HashMap<TitleKey, mg_uikit::dock::DocTabTitle>,
    resources: HashMap<ResKey, Watched>,
    /// How many times a document was compared with its saved copy (for
    /// tests: `Moonglow::dot_comparisons`).
    pub(crate) comparisons: u64,
}

/// What a tab's module resource was last seen as: its stamp, whether it
/// differs from the saved module, whether that is known (`exact`) or only
/// taken for so as it changed, and when it last changed (the window's
/// time).
#[derive(Debug)]
struct Watched {
    stamp: ResourceStamp,
    differs: bool,
    exact: bool,
    since: f64,
}

/// How long a document must rest (no change, no pointer button held)
/// before it is compared with its saved copy: an edit frame, a drag's
/// frames, never pay for it.
const REST: f64 = 0.2;

/// A title laid out from: its name, the widest it may be (the bits of the
/// cap, `u32::MAX` for none), its dot, and the font, the display's scale
/// and how many times egui has made its fonts (`script_layout::fonts_made`:
/// a galley kept from before is drawn from the old atlas's places).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TitleKey(String, u32, bool, u32, u32, u64);

/// When a module resource last changed ([`mg_edit::Workspace::changed_at`])
/// and which data the module and the saved module hold for it. The data
/// are held, and told apart by what they are (`Arc::ptr_eq`), never by an
/// address: one freed (a resource taken out and saved) could be given to
/// data made later. (`changed_at` alone moves with every edit of the
/// resource, but not with a save, which changes the saved data only.)
#[derive(Debug, Clone)]
struct ResourceStamp(u64, Option<std::sync::Arc<[u8]>>, Option<std::sync::Arc<[u8]>>);

impl PartialEq for ResourceStamp {
    fn eq(&self, other: &Self) -> bool {
        let same = |a: &Option<std::sync::Arc<[u8]>>, b: &Option<std::sync::Arc<[u8]>>| match (a, b)
        {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        self.0 == other.0 && same(&self.1, &other.1) && same(&self.2, &other.2)
    }
}

/// Every tab's kind icon: the drawn icon of the resource type it shows
/// (the Resources panel's and Quick Open's for the type), or the editor's
/// own for a tool that shows no resource.
pub(crate) fn icon(tab: &Tab) -> mg_uikit::Icon {
    use mg_uikit::Icon as I;
    let of = crate::browser::kind_icon;
    // (A placed object: its kind's, by its list in the area.)
    let placed = |path: &mg_edit::GffPath| {
        let kind = path.first().and_then(|s| s.item()).map(|(list, _)| list);
        kind.and_then(mg_area::ObjectKind::from_list).map_or(I::Properties, crate::icons::object)
    };
    match tab {
        Tab::Welcome => I::Home,
        Tab::ModuleProperties => I::Module,
        Tab::NoArea | Tab::Area(_) => I::Area,
        Tab::Script(_) => I::Script,
        Tab::Nui(_) => I::Nui,
        Tab::Gff(k) | Tab::Blueprint(k) | Tab::Resource(k) => of(k.restype),
        Tab::ResourceCopy { key, .. } => of(key.restype),
        Tab::Model(k) | Tab::ModelCopy { key: k, .. } => {
            if k.restype == ResType::MDL {
                I::Model
            } else {
                of(k.restype)
            }
        }
        Tab::Blueprints(keys) => keys.first().map_or(I::Properties, |k| of(k.restype)),
        Tab::Factions => I::Faction,
        Tab::Journal => I::Journal,
        Tab::TalkTable => I::TalkTable,
        Tab::Hak(_) => I::Hak,
        Tab::Table(_) => I::Table,
        Tab::Tileset(_) => I::Tileset,
        Tab::Dialog(_) => I::Conversation,
        Tab::AreaProperties(_) | Tab::AreasProperties(_) => I::AreaProperties,
        Tab::Instance { path, .. } => placed(path),
        Tab::Instances { paths, .. } => paths.first().map_or(I::Properties, placed),
        Tab::InstanceModel { .. } => I::Model,
        Tab::Manual => I::Book,
        Tab::References => I::References,
        Tab::AdvancedSearch => I::Search,
        Tab::PlaceableGallery => I::Gallery,
        Tab::Options => I::Settings,
        Tab::TileProperties | Tab::Tile { .. } => I::Tile,
        // (The plugin API names no icons: a panel is the plugins'.)
        Tab::PluginPanel { .. } => I::Plugin,
    }
}

/// The module's resources a tab's own document is made of, whose changes
/// since the module was saved its modified dot shows: an area's three
/// files, a resource's own, Module Properties' `module.ifo`; none for a
/// tab with a document of its own ([`own_unsaved`]) or none at all (a
/// placed object's Properties edit its area's, whose tab shows it).
fn resources_of(app: &Moonglow, tab: &Tab) -> Vec<ResKey> {
    let area = |r: mg_core::ResRef, types: &[ResType]| -> Vec<ResKey> {
        types.iter().map(|t| ResKey::new(r, *t)).collect()
    };
    match tab {
        Tab::Area(r) => area(*r, &[ResType::ARE, ResType::GIT, ResType::GIC]),
        Tab::AreaProperties(r) => area(*r, &[ResType::ARE, ResType::GIT]),
        Tab::AreasProperties(areas) => {
            areas.iter().flat_map(|r| area(*r, &[ResType::ARE, ResType::GIT])).collect()
        }
        Tab::Blueprint(k) | Tab::Dialog(k) | Tab::Gff(k) | Tab::Nui(k) => {
            // (The game's blueprint, looked at: nothing of the module's.)
            if app.ws.as_ref().is_some_and(|ws| ws.is_viewed(k)) { Vec::new() } else { vec![*k] }
        }
        Tab::Blueprints(keys) => keys.clone(),
        Tab::ModuleProperties => ResKey::parse("module", ResType::IFO).into_iter().collect(),
        Tab::Factions | Tab::Journal => tab.tree_key().into_iter().collect(),
        _ => Vec::new(),
    }
}

/// Whether an editor with a document of its own (a script's text, the talk
/// table, a hak, a 2DA, a tileset) has changes not saved where they go:
/// `None` for a tab without one.
fn own_unsaved(app: &Moonglow, tab: &Tab) -> Option<bool> {
    match tab {
        Tab::Script(k) => Some(app.scripts.get(k).is_some_and(|b| b.is_dirty())),
        Tab::TalkTable => Some(app.talk.as_ref().is_some_and(|t| t.is_dirty())),
        Tab::Hak(_) | Tab::Table(_) | Tab::Tileset(_) => {
            Some(app.side_doc(tab).is_some_and(|d| d.has_unsaved()))
        }
        _ => None,
    }
}

/// Whether a tab's own document has unsaved changes: its editor's own
/// state ([`own_unsaved`]), else whether any of its module resources
/// differs from the module as saved ([`resources_of`]; asked of the
/// workspace only after the resource changed, `cache`).
/// The resources' watch over a frame: the window's time, whether the
/// pointer is free (no button held: no drag going on), the comparisons
/// run, and when a resource still waiting to be compared rests long
/// enough (seconds from now).
struct Watch<'a> {
    resources: &'a mut HashMap<ResKey, Watched>,
    now: f64,
    free: bool,
    comparisons: &'a mut u64,
    wake: Option<f64>,
}

/// Whether a tab's own document has unsaved changes: its editor's own
/// state ([`own_unsaved`]), else whether any of its module resources
/// differs from the module as saved ([`resources_of`]). A resource is
/// asked about only when it changed: at once where the workspace tells it
/// without comparing (its data, a save); else (a document edited) it is
/// taken to differ as it changes, and compared with its saved copy only
/// once it has rested [`REST`] with no button held, so that an edit's
/// frame and a drag's never pay for the comparison (up to a millisecond on
/// a large area), and an edit undone back to what is saved loses its dot
/// a moment later (the window asks for that frame: `Watch::wake`).
fn modified(app: &Moonglow, tab: &Tab, watch: &mut Watch<'_>) -> bool {
    if let Some(own) = own_unsaved(app, tab) {
        return own;
    }
    let Some(ws) = &app.ws else { return false };
    resources_of(app, tab).iter().fold(false, |any, key| {
        let stamp = ResourceStamp(
            ws.changed_at(key),
            ws.module.shared(key).cloned(),
            ws.saved().shared(key).cloned(),
        );
        let seen = watch.resources.get_mut(key).filter(|w| w.stamp == stamp);
        let differs = match seen {
            Some(w) if w.exact => w.differs,
            Some(w) => {
                let rested = watch.now - w.since;
                if watch.free && rested >= REST {
                    *watch.comparisons += 1;
                    w.differs = ws.differs_from_saved(key);
                    w.exact = true;
                } else {
                    let left = (REST - rested).max(0.02);
                    watch.wake = Some(watch.wake.map_or(left, |w| w.min(left)));
                }
                w.differs
            }
            None => {
                let at_once = ws.differs_from_saved_at_once(key);
                if at_once.is_none() {
                    watch.wake = Some(watch.wake.map_or(REST, |w| w.min(REST)));
                }
                let w = Watched {
                    stamp,
                    differs: at_once.unwrap_or(true),
                    exact: at_once.is_some(),
                    since: watch.now,
                };
                let differs = w.differs;
                watch.resources.insert(*key, w);
                differs
            }
        };
        any || differs
    })
}

/// The room a pane's tab bar leaves its tabs: the main surface's, all but
/// the Split Editor button's at its right end ([`SPLIT_ROOM`]); a window's,
/// all but its collapse arrow at its left ([`WINDOW_BAR_START`]), its
/// close all ([`WINDOW_CLOSE_ALL`]) and the room before it for Maximize
/// ([`WINDOW_END_ROOM`]).
pub(crate) fn tab_room(main: bool, width: f32) -> f32 {
    if main {
        width - SPLIT_ROOM
    } else {
        width - WINDOW_BAR_START - WINDOW_CLOSE_ALL - WINDOW_END_ROOM
    }
}

/// The room at the right end of each tab bar of the main surface, which
/// egui_dock leaves free of tabs for the Split Editor button (the mock's
/// `.doctabs .tools`: a 22 px button and 3 px at each side).
pub(crate) const SPLIT_ROOM: f32 = 28.0;

/// The room a window's collapse arrow takes at its bar's left: egui_dock's
/// button (24 points; the board's 22 point button with 3 and 2 round it,
/// a point off).
pub(crate) const WINDOW_BAR_START: f32 = 24.0;

/// The room a window's Maximize and Close take at its bar's right:
/// egui_dock's close all (the Close painted in it) and Maximize a point
/// before it, 22 points (the board's `.fw-bar .tools`: Maximize where the
/// board has it, the Close's icon two points nearer the edge).
pub(crate) const WINDOW_BAR_END: f32 = WINDOW_CLOSE_ALL + 1.0 + mg_uikit::metrics::ICON_BUTTON;

/// The name of a window's Close (egui_dock's close all on a window of one
/// pane).
pub(crate) const WINDOW_CLOSE: &str = "Close Window";

/// The name of a pane's Close in a window split in panes.
pub(crate) const PANE_CLOSE: &str = "Close Pane";

/// egui_dock's close all button at a window's bar's right end (the Close
/// is painted in it).
pub(crate) const WINDOW_CLOSE_ALL: f32 = 24.0;

/// The room egui_dock leaves free of tabs before a window's close all
/// (`DockArea::window_end_room`, the vendored crate's): Maximize's, which
/// stands beside the Close a point further in than egui_dock's button
/// reaches, and a point between it and the last tab, so that no tab and
/// no tab's close lies under Maximize or the Close.
pub(crate) const WINDOW_END_ROOM: f32 = WINDOW_BAR_END - WINDOW_CLOSE_ALL + 1.0;

/// The least width of a floating window (`DockArea::window_min_width`):
/// its frame, the collapse arrow, one tab cut to its least (a modified
/// one's, with its dot) and Maximize and the Close, so that every button
/// of its bar has its room at any width it can be made.
pub(crate) fn window_least_width() -> f32 {
    let tab = mg_uikit::shell::doc_tab_width(mg_uikit::dock::DOC_TAB_LEAST_TITLE, true);
    (2.0 + WINDOW_BAR_START + tab + WINDOW_CLOSE_ALL + WINDOW_END_ROOM).ceil()
}

/// Where a window's Close is painted in egui_dock's close all button
/// (`rect`, at the bar's right end and two points short of its foot):
/// 22 points square, a point from the bar's right end, in the middle of
/// the bar's height.
pub(crate) fn window_close_rect(rect: egui::Rect) -> egui::Rect {
    let side = mg_uikit::metrics::ICON_BUTTON;
    let middle = (rect.top() + rect.bottom() + 2.0) / 2.0;
    egui::Rect::from_min_size(
        egui::pos2(rect.right() - 1.0 - side, middle - side / 2.0),
        egui::vec2(side, side),
    )
}

/// A window's collapse arrow and close all ([`TabViewer::window_bar_button`]),
/// in the Floating board's look, named and tipped.
fn paint_window_bar_button(
    ui: &mut Ui,
    button: egui_dock::WindowBarButton,
    response: &egui::Response,
) {
    use egui_dock::WindowBarButton;
    use mg_uikit::icons::{ButtonState, Icon, paint_icon_button};
    let t = mg_uikit::tokens(ui.ctx());
    let side = mg_uikit::metrics::ICON_BUTTON;
    let enabled = ui.is_enabled();
    let state = ButtonState::of(response);
    match button {
        WindowBarButton::Collapse { collapsed, whole_window } => {
            let rect = response.rect;
            let at = egui::Rect::from_center_size(rect.center(), egui::vec2(side, side));
            let icon = if collapsed { Icon::ChevronRight } else { Icon::ChevronDown };
            paint_icon_button(ui.painter(), &t, at, icon, state, false);
            // (The line between it and the first tab: `.fw-bar
            // .doctab:first-of-type`.)
            let px = 1.0 / ui.ctx().pixels_per_point();
            ui.painter().vline(
                rect.right() - px / 2.0,
                rect.y_range(),
                egui::Stroke::new(px, t.line),
            );
            let (name, tip) = match (collapsed, whole_window) {
                (false, true) => ("Collapse", "Collapse the window to its tab bar"),
                (false, false) => ("Collapse", "Collapse the pane to its tab bar"),
                (true, true) => ("Expand", "Show the window again"),
                (true, false) => ("Expand", "Show the pane again"),
            };
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name));
            let _ = response.clone().on_hover_text(tip);
        }
        WindowBarButton::CloseAll { disabled, whole_window } => {
            let at = window_close_rect(response.rect);
            if disabled {
                let icon = egui::Rect::from_center_size(at.center(), egui::Vec2::splat(16.0));
                Icon::Close.paint(ui.painter(), icon, t.text_off);
            } else {
                paint_icon_button(ui.painter(), &t, at, Icon::Close, state, false);
            }
            let tip = match (disabled, whole_window) {
                (true, _) => "A tab of it can't be closed",
                (false, true) => "Close the window and its tabs",
                (false, false) => "Close this pane's tabs",
            };
            // (Named for what it closes: a dialog's Close, and a tab's own
            // "Close" button (the hak editor's view), may be in sight.)
            let name = if whole_window { WINDOW_CLOSE } else { PANE_CLOSE };
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled && !disabled, name)
            });
            let _ = response.clone().on_hover_text(tip);
        }
    }
}

/// Lays out every tab's title for the frame ahead, before the dock is
/// drawn ([`Look`], in `Moonglow::doc_tabs`): each pane's titles whole
/// where its bar has the room, else cut alike to fit it
/// ([`mg_uikit::dock::doc_tab_cap`], not under its least), and laid out
/// again only where what they are laid out from changed ([`LookCache`]).
pub(crate) fn prepare(app: &mut Moonglow, ctx: &egui::Context) {
    let font = egui::TextStyle::Button.resolve(&ctx.global_style());
    let made = (ctx.pixels_per_point().to_bits(), crate::script_layout::fonts_made(ctx));
    let panes: Vec<(bool, f32, Vec<Tab>)> = (app.dock.iter_leaves())
        .map(|(p, l)| (p.surface.is_main(), l.rect.width(), l.tabs.clone()))
        .collect();
    let mut cache = std::mem::take(&mut app.look_cache);
    let mut titles = HashMap::new();
    let mut looks = HashMap::new();
    let mut lay = |name: &str, cap: Option<f32>, modified: bool| {
        let key = TitleKey(
            name.to_string(),
            cap.map_or(u32::MAX, f32::to_bits),
            modified,
            font.size.to_bits(),
            made.0,
            made.1,
        );
        if let Some(title) = titles.get(&key) {
            return std::clone::Clone::clone(title);
        }
        let title = cache.titles.remove(&key).unwrap_or_else(|| {
            let max = cap.unwrap_or(f32::INFINITY);
            mg_uikit::dock::doc_tab_title(ctx, font.clone(), name, max, modified)
        });
        titles.insert(key, title.clone());
        title
    };
    let mut watch = Watch {
        resources: &mut cache.resources,
        now: ctx.input(|i| i.time),
        free: !ctx.input(|i| i.pointer.any_down()),
        comparisons: &mut cache.comparisons,
        wake: None,
    };
    let mut named_panes = Vec::new();
    for (main, width, tabs) in panes {
        let named: Vec<(Tab, String, bool)> = tabs
            .into_iter()
            .map(|t| {
                let name = name(app, &t);
                let modified = modified(app, &t, &mut watch);
                (t, name, modified)
            })
            .collect();
        named_panes.push((main, width, named));
    }
    // (A resource changed and not yet compared: a frame once it has
    // rested, which compares it.)
    if let Some(wake) = watch.wake {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(wake));
    }
    for (main, width, named) in named_panes {
        let whole: Vec<(f32, bool)> =
            named.iter().map(|(_, n, m)| (lay(n, None, *m).width, *m)).collect();
        // (A pane not drawn yet, a group or window just made, has no width
        // to fit: its titles whole until it has.)
        let cap = (width > 0.0)
            .then(|| mg_uikit::dock::doc_tab_cap(tab_room(main, width), &whole))
            .flatten();
        for (tab, name, modified) in named {
            let title = lay(&name, cap, modified);
            looks.insert(tab.clone(), Look { icon: icon(&tab), name, modified, title });
        }
    }
    // (Only the titles laid out this frame are kept, and the resources of
    // the tabs open.)
    cache.titles = titles;
    let open: std::collections::HashSet<ResKey> =
        looks.keys().flat_map(|t| resources_of(app, t)).collect();
    cache.resources.retain(|k, _| open.contains(k));
    // (And the saved copies read to compare them.)
    if let Some(ws) = &app.ws {
        ws.forget_saved_copies(|k| open.contains(k));
    }
    app.look_cache = cache;
    app.doc_tabs = looks;
}

/// Whether a tab fits its pane, scrolling within itself, or scrolls in it
/// when wider or taller ([`TabViewer::ui`]).
fn fits(tab: &Tab) -> bool {
    {
        matches!(
            tab,
            Tab::Script(_)
                | Tab::Nui(_)
                | Tab::Manual
                | Tab::Options
                | Tab::ModuleProperties
                | Tab::AreaProperties(_)
                | Tab::AreasProperties(_)
                | Tab::Blueprint(_)
                | Tab::Blueprints(_)
                | Tab::Instance { .. }
                | Tab::Instances { .. }
                | Tab::Gff(_)
                | Tab::Journal
                | Tab::Dialog(_)
                | Tab::TalkTable
                | Tab::Tileset(_)
                | Tab::PluginPanel { .. }
        )
    }
}

/// An area in a tab's title: its ResRef, or its name (Options › General:
/// Show areas by name).
pub(crate) fn area_label(app: &mut Moonglow, area: mg_core::ResRef) -> String {
    let Some(ws) = &app.ws else { return area.to_string() };
    app.area_names.label(ws, app.game.as_deref(), area, app.settings.area_names)
}

/// A placed object's tab title: its tag and where it is.
fn instance_title(app: &mut Moonglow, area: mg_core::ResRef, path: &mg_edit::GffPath) -> String {
    let git = ResKey::new(area, ResType::GIT);
    let tag = app
        .ws
        .as_mut()
        .and_then(|ws| ws.doc(&git).ok())
        .and_then(|g| path.get(&g.root))
        .and_then(|s| s.string("Tag"))
        .map(|t| String::from_utf8_lossy(t).into_owned())
        .unwrap_or_default();
    format!("{tag} ({area})")
}

/// The middle of the window while no area is open: how to open one, and
/// the module's areas to open with a click.
fn no_area(app: &mut Moonglow, ui: &mut Ui) {
    let areas = app.ws.as_ref().map(|ws| mg_module::areas::every(&ws.module)).unwrap_or_default();
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.heading("No area open");
        ui.label("Double-click an area in the module tree to open it here.");
        ui.add_space(12.0);
        if ui.button("Area Wizard…").on_hover_text("Make a new area").clicked() {
            app.actions.push(Action::AreaWizard);
        }
        // Those opened lately first (those still in the module), then
        // the module's first.
        let by_name = app.settings.area_names;
        let recent: Vec<mg_core::ResRef> = (app.module_path())
            .map(|m| app.settings.recent_areas(&m).to_vec())
            .unwrap_or_default()
            .iter()
            .filter_map(|a| std::str::FromStr::from_str(a).ok())
            .filter(|a| areas.contains(a))
            .collect();
        let link = |app: &mut Moonglow, ui: &mut Ui, area: mg_core::ResRef| {
            let label = match &app.ws {
                Some(ws) => app.area_names.label(ws, app.game.as_deref(), area, by_name),
                None => area.to_string(),
            };
            if ui.link(label).clicked() {
                app.actions.push(Action::OpenTab(Tab::Area(area)));
            }
        };
        if !recent.is_empty() {
            ui.add_space(16.0);
            ui.strong("Opened lately");
            for area in &recent {
                link(app, ui, *area);
            }
        }
        if !areas.is_empty() {
            ui.add_space(16.0);
            ui.strong("Areas");
            for area in areas.iter().take(12) {
                link(app, ui, *area);
            }
            if areas.len() > 12 {
                ui.weak(format!("and {} more in the module tree", areas.len() - 12));
            }
        }
    });
}

/// A recent module as the welcome page lists it: its name, a folder's
/// (a module folder, a nasher project) with a `/` after it, and, grayed
/// after it, the folder it is in. A module folder
/// beside its `.mod` (GitHub issue 30) was `castle` beside `castle.mod`.
fn recent_entry(ui: &Ui, path: &std::path::Path) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    let mut name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    // (Told by its name, not the disk: recent modules are `.mod` files,
    // and folders.)
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mod")) {
        name.push('/');
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut job = LayoutJob::default();
    let own = TextFormat {
        font_id: font.clone(),
        color: egui::Color32::PLACEHOLDER,
        ..Default::default()
    };
    job.append(&name, 0.0, own);
    // (The folder's name alone, `in modules`: the whole path is on hover.)
    if let Some(place) = path.parent().and_then(|p| p.file_name()) {
        let weak = TextFormat {
            font_id: font,
            color: ui.visuals().weak_text_color(),
            ..Default::default()
        };
        job.append(&format!("  in {}", place.to_string_lossy()), 0.0, weak);
    }
    job
}

fn welcome(app: &mut Moonglow, ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.heading("Moonglow Toolset");
        ui.label("A module toolset for Neverwinter Nights: Enhanced Edition.");
        ui.add_space(20.0);
        if ui.button("New Module…").clicked() {
            app.actions.push(Action::NewModuleDialog);
        }
        if ui.button("Open Module…").clicked() {
            app.actions.push(Action::OpenModuleDialog);
        }
        let folder =
            ui.button("Open Folder…").on_hover_text("A module folder, or a nasher project");
        if folder.clicked() {
            app.actions.push(Action::OpenFolderDialog);
        }
        if !app.settings.recent.is_empty() {
            ui.add_space(20.0);
            ui.strong("Recent modules");
            for p in app.settings.recent.clone() {
                if ui.link(recent_entry(ui, &p)).on_hover_text(p.display().to_string()).clicked() {
                    app.actions.push(Action::OpenModule(p));
                }
            }
        }
        if app.game.is_none() {
            ui.add_space(10.0);
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "No game data: choose your Neverwinter Nights installation in Tools > Options.",
            );
            if ui.button("Options…").clicked() {
                app.actions.push(Action::OptionsDialog);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each blueprint type's windows are kept by its own name, and the
    /// four once kept together as `blueprint` say so, for the sizes and
    /// places kept by it before.
    #[test]
    fn each_blueprint_type_is_a_kind_of_window() {
        let tab = |t| Tab::Blueprint(ResKey::parse("x", t).unwrap());
        let kinds: Vec<_> =
            mg_area::ObjectKind::ALL.map(|k| (tab(k.restype()).kind(), k.name())).into();
        assert!(kinds.iter().all(|(a, b)| a == b), "{kinds:?}");
        assert_eq!(tab(ResType::UTC).kind_before(), None);
        assert_eq!(tab(ResType::UTW).kind_before(), Some("blueprint"));
        assert_eq!(Tab::Factions.kind_before(), None);
    }

    /// Every kind of tab has its kind's icon (`icon` matches every
    /// variant, with no catch-all: a new kind of tab must be given one), the
    /// resource type's where it shows a resource; none is the plain file's.
    #[test]
    fn every_kind_of_tab_has_its_icon() {
        use mg_uikit::Icon as I;
        let k = |n: &str, t| ResKey::parse(n, t).unwrap();
        let r = mg_core::ResRef::from_str("area").unwrap();
        let path = mg_edit::GffPath::root().item("Creature List", 0);
        let every = vec![
            (Tab::Welcome, I::Home),
            (Tab::ModuleProperties, I::Module),
            (Tab::NoArea, I::Area),
            (Tab::Script(k("s", ResType::NSS)), I::Script),
            (Tab::Nui(k("n", ResType::JUI)), I::Nui),
            (Tab::Gff(k("g", ResType::FAC)), I::Faction),
            (Tab::Factions, I::Faction),
            (Tab::Journal, I::Journal),
            (Tab::TalkTable, I::TalkTable),
            (Tab::Hak(1), I::Hak),
            (Tab::Table(1), I::Table),
            (Tab::Tileset(1), I::Tileset),
            (Tab::Dialog(k("d", ResType::DLG)), I::Conversation),
            (Tab::Resource(k("t", ResType::TGA)), I::Texture),
            (Tab::ResourceCopy { key: k("t", ResType::TWODA), layer: "hak".into() }, I::Table),
            (Tab::ModelCopy { key: k("m", ResType::MDL), layer: "hak".into() }, I::Model),
            (Tab::Model(k("m", ResType::MDL)), I::Model),
            (Tab::Model(k("c", ResType::UTC)), I::Creature),
            (Tab::Blueprint(k("p", ResType::UTP)), I::Placeable),
            (Tab::Blueprints(vec![k("i", ResType::UTI)]), I::Item),
            (Tab::Area(r), I::Area),
            (Tab::AreaProperties(r), I::AreaProperties),
            (Tab::AreasProperties(vec![r]), I::AreaProperties),
            (Tab::Instances { area: r, paths: vec![path.clone()] }, I::Creature),
            (Tab::Instance { area: r, path: path.clone() }, I::Creature),
            (Tab::Manual, I::Book),
            (Tab::References, I::References),
            (Tab::AdvancedSearch, I::Search),
            (Tab::PlaceableGallery, I::Gallery),
            (Tab::Options, I::Settings),
            (Tab::TileProperties, I::Tile),
            (Tab::InstanceModel { area: r, path }, I::Model),
            (Tab::Tile { tileset: r, tile: 0 }, I::Tile),
            (Tab::PluginPanel { plugin: "p".into(), panel: "q".into() }, I::Plugin),
        ];
        // (Every variant is in the list: this match fails to compile
        // where one is not named.)
        for (tab, _) in &every {
            match tab {
                Tab::Welcome
                | Tab::ModuleProperties
                | Tab::NoArea
                | Tab::Script(_)
                | Tab::Nui(_)
                | Tab::Gff(_)
                | Tab::Factions
                | Tab::Journal
                | Tab::TalkTable
                | Tab::Hak(_)
                | Tab::Table(_)
                | Tab::Tileset(_)
                | Tab::Dialog(_)
                | Tab::Resource(_)
                | Tab::ResourceCopy { .. }
                | Tab::ModelCopy { .. }
                | Tab::Model(_)
                | Tab::Blueprint(_)
                | Tab::Blueprints(_)
                | Tab::Area(_)
                | Tab::AreaProperties(_)
                | Tab::AreasProperties(_)
                | Tab::Instances { .. }
                | Tab::Instance { .. }
                | Tab::Manual
                | Tab::References
                | Tab::AdvancedSearch
                | Tab::PlaceableGallery
                | Tab::Options
                | Tab::TileProperties
                | Tab::InstanceModel { .. }
                | Tab::Tile { .. }
                | Tab::PluginPanel { .. } => {}
            }
        }
        // Each kind's taller by its name, as a remembered size names it,
        // that of its tabs (the kinds with no window of their own none).
        for (tab, _) in &every {
            let by_name = Tab::taller_of_kind(tab.kind());
            match tab.kind() {
                "welcome" | "no-area" | "area" => assert_eq!(by_name, None, "{tab:?}"),
                _ => assert_eq!(by_name, Some(tab.taller()), "{tab:?}"),
            }
        }
        for kind in mg_area::ObjectKind::ALL {
            let tab = Tab::Blueprint(k("b", kind.restype()));
            assert_eq!(Tab::taller_of_kind(kind.name()), Some(tab.taller()), "{kind:?}");
        }
        assert_eq!(Tab::taller_of_kind("blueprint"), Some(FORMS_TALLER), "an older name");
        assert_eq!(Tab::taller_of_kind("frobnicate"), None, "unknown");
        for (tab, want) in every {
            assert_eq!(icon(&tab), want, "{tab:?}");
            assert_ne!(icon(&tab), I::File, "{tab:?}");
        }
    }

    /// A window's last tab closed takes the window with it, and the
    /// others stay where they are; the window drawn after it is whole.
    #[test]
    fn closing_a_window_s_last_tab_closes_the_window() {
        let talk = ResKey::parse("talk", ResType::DLG).unwrap();
        let other = ResKey::parse("other", ResType::DLG).unwrap();
        let mut dock = egui_dock::DockState::new(vec![Tab::ModuleProperties]);
        dock.add_window(vec![Tab::Dialog(talk)]);
        dock.add_window(vec![Tab::Dialog(other), Tab::Dialog(talk)]);
        close_tabs(&mut dock, |t| *t != Tab::Dialog(talk));
        let windows: Vec<Vec<Tab>> = dock
            .iter_surfaces()
            .filter(|s| matches!(s, egui_dock::Surface::Window(..)))
            .map(|s| s.iter_all_tabs().map(|(_, t)| t.clone()).collect())
            .collect();
        assert_eq!(windows, [vec![Tab::Dialog(other)]]);
        assert!(dock.find_tab(&Tab::ModuleProperties).is_some());
        // Every window left has a pane with tabs to title it by, the one
        // in focus if one is (`DockState::retain_tabs` left it in focus
        // emptied).
        for s in dock.iter_surfaces() {
            if let egui_dock::Surface::Window(tree, _) = s {
                assert!(tree.focused_leaf().is_none_or(|n| tree[n].is_leaf()));
                assert!(tree.iter().any(|n| n.tabs_count() > 0));
            }
        }
    }
}
