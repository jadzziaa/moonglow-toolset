//! The central area's tabs.

use egui::{Id, Ui, WidgetText};
use egui_dock::TabViewer;
use mg_core::ResType;
use mg_resman::ResKey;

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
    /// Any GFF resource, as an editable field tree.
    Gff(ResKey),
    /// The resource browser.
    Resources,
    /// The Faction Editor.
    Factions,
    /// The Journal Editor.
    Journal,
    /// The module's custom talk table.
    TalkTable,
    /// A hak in the hak editor (its id among the open haks).
    Hak(u32),
    /// A tileset in the tileset editor (its id among the open tilesets).
    Tileset(u32),
    /// A conversation.
    Dialog(ResKey),
    /// A resource from the load order, read-only.
    Resource(ResKey),
    /// A model, in the 3D viewer.
    Model(ResKey),
    /// The blueprint palettes.
    Palette,
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
    /// The Placeable Gallery: every placeable appearance as a picture.
    PlaceableGallery,
    /// Tools › Options.
    Options,
    /// Tile Properties of the tiles selected in an area: their lights,
    /// loops and, of one tile, its variants.
    TileProperties,
    /// What an object placed in an area looks like, in the 3D viewer.
    InstanceModel { area: mg_core::ResRef, path: mg_edit::GffPath },
}

impl Tab {
    /// Whether it opens in the main dock: areas (and the start page), which
    /// the rest open over in windows of their own, so a window never takes
    /// the area view's place.
    pub fn docks(&self) -> bool {
        matches!(self, Tab::Area(_) | Tab::Welcome | Tab::NoArea)
    }

    /// The kind of tab, whatever it shows: what its window's size is
    /// remembered by.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Tab::Welcome => "welcome",
            Tab::NoArea => "no-area",
            Tab::ModuleProperties => "module-properties",
            Tab::Script(_) => "script",
            Tab::Gff(_) => "gff",
            Tab::Resources => "resources",
            Tab::Factions => "factions",
            Tab::Journal => "journal",
            Tab::TalkTable => "talk-table",
            Tab::Hak(_) => "hak",
            Tab::Tileset(_) => "tileset",
            Tab::Dialog(_) => "dialog",
            Tab::Resource(_) => "resource",
            Tab::Model(_) | Tab::InstanceModel { .. } => "model",
            Tab::Palette => "palette",
            Tab::Blueprint(k) => match k.restype {
                ResType::UTC => "creature",
                ResType::UTI => "item",
                ResType::UTP => "placeable",
                ResType::UTD => "door",
                ResType::UTM => "store",
                _ => "blueprint",
            },
            Tab::Blueprints(_) => "blueprints",
            Tab::Area(_) => "area",
            Tab::AreaProperties(_) | Tab::AreasProperties(_) => "area-properties",
            Tab::Instances { .. } | Tab::Instance { .. } => "instance",
            Tab::Manual => "manual",
            Tab::References => "references",
            Tab::PlaceableGallery => "placeable-gallery",
            Tab::Options => "options",
            Tab::TileProperties => "tile-properties",
        }
    }

    /// The size its own window opens at (before fitting the screen).
    pub(crate) fn window_size(&self) -> egui::Vec2 {
        let (w, h) = match self {
            // (Room for the text beside the lists, which open as wide as
            // their longest name.)
            Tab::Script(_) => (1120.0, 720.0),
            Tab::Manual | Tab::Resources | Tab::Resource(_) => (920.0, 700.0),
            Tab::Dialog(_) => (920.0, 660.0),
            Tab::Tileset(_) | Tab::Hak(_) => (920.0, 720.0),
            Tab::Model(_) | Tab::InstanceModel { .. } => (780.0, 680.0),
            Tab::Factions | Tab::Journal | Tab::Gff(_) => (780.0, 600.0),
            Tab::ModuleProperties | Tab::AreaProperties(_) => (760.0, 580.0),
            Tab::Options => crate::options::WINDOW_SIZE.into(),
            Tab::TileProperties => (560.0, 640.0),
            _ => (820.0, 640.0),
        };
        egui::vec2(w, h)
    }

    /// The tab that edits a resource.
    pub fn for_resource(key: ResKey) -> Option<Tab> {
        match key.restype {
            ResType::NSS => Some(Tab::Script(key)),
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

    /// The resource the tab shows that can be renamed (a script, an area, a
    /// conversation, a blueprint, a GFF file), whether or not the module
    /// has it.
    /// Whether it has a close button (and Close in its menu).
    pub fn closeable(&self) -> bool {
        !matches!(self, Tab::Welcome | Tab::NoArea)
    }

    pub fn renamable(&self) -> Option<ResKey> {
        match self {
            Tab::Script(k) | Tab::Dialog(k) | Tab::Blueprint(k) | Tab::Gff(k) => Some(*k),
            Tab::Area(r) | Tab::AreaProperties(r) => Some(ResKey::new(*r, ResType::ARE)),
            _ => None,
        }
    }
}

pub(crate) struct Viewer<'a> {
    pub(crate) app: &'a mut Moonglow,
}

impl Viewer<'_> {
    /// What a tab shows.
    fn body(&mut self, ui: &mut Ui, tab: &mut Tab) {
        crate::trace::changed(&format!("drawn {tab:?}"), || format!("in {:?}", ui.max_rect()));
        // (The dock's windows have no margin: `Moonglow::ui`. What a tab
        // shows has the usual one.)
        ui.spacing_mut().window_margin = ui.ctx().global_style().spacing.window_margin;
        // Escape closes a model's window (opened to look, and in the way
        // after), while the pointer is over it and nothing is being typed;
        // and a Properties window, when it is the one in front (what it
        // changed is kept, as when its tab is closed: Undo takes it back).
        // Dialogs over it close first.
        // (Not during a drag, which Escape drops.)
        let escape = crate::widgets::escape_past_dialogs(ui.ctx())
            && ui.memory(|m| m.focused().is_none())
            && !ui.input(|i| i.pointer.any_down());
        let model = matches!(tab, Tab::Model(_) | Tab::InstanceModel { .. })
            && ui.rect_contains_pointer(ui.max_rect());
        let layer = ui.layer_id();
        let front = layer.order == egui::Order::Middle
            && ui.memory(|m| {
                // (The last of the windows showing: closed ones keep their
                // place in the order.)
                let showing =
                    |l: &egui::LayerId| l.order == egui::Order::Middle && m.areas().is_visible(l);
                m.layer_ids().filter(showing).last()
            }) == Some(layer)
            && matches!(
                tab,
                Tab::Blueprint(_)
                    | Tab::Blueprints(_)
                    | Tab::Instance { .. }
                    | Tab::Instances { .. }
                    | Tab::AreaProperties(_)
                    | Tab::AreasProperties(_)
                    | Tab::ModuleProperties
                    | Tab::Options
                    | Tab::TileProperties
            );
        // (With a tool in hand in the area, Escape over the area drops the
        // tool and no more; over this window, it closes the window.)
        let tool = ui.ctx().cumulative_pass_nr().saturating_sub(self.app.area_tool_at) <= 1
            && !ui.rect_contains_pointer(ui.max_rect());
        // (Options › Keyboard, waiting for a key, takes Escape itself.)
        let recording = *tab == Tab::Options
            && self.app.options.as_ref().is_some_and(|o| o.recording.is_some());
        if escape && !recording && (model || (front && !tool)) {
            self.app.actions.push(crate::Action::CloseTab(tab.clone()));
        }
        match tab {
            Tab::Welcome => welcome(self.app, ui),
            Tab::NoArea => no_area(self.app, ui),
            Tab::ModuleProperties => module_props::ui(self.app, ui),
            Tab::Script(k) => script_view::ui(self.app, ui, *k),
            Tab::Gff(k) => gff_view::ui(self.app, ui, *k),
            Tab::Resources => browser::ui(self.app, ui),
            Tab::Palette => crate::palette_view::ui(self.app, ui),
            Tab::Blueprint(k) => crate::blueprint::ui(self.app, ui, *k),
            Tab::Factions => faction_view::ui(self.app, ui),
            Tab::Journal => journal_view::ui(self.app, ui),
            Tab::TalkTable => crate::talk_view::ui(self.app, ui),
            Tab::Hak(id) => crate::hak_view::ui(self.app, ui, *id),
            Tab::Tileset(id) => crate::tileset_view::ui(self.app, ui, *id),
            Tab::Dialog(k) => dialog_view::ui(self.app, ui, *k),
            Tab::Resource(k) => browser::resource_ui(self.app, ui, *k),
            // A blueprint without a model: what there is to see of it.
            Tab::Model(k) if !model_view::previewable(k.restype) => {
                crate::area_tools::summary_view(self.app, ui, *k)
            }
            Tab::Model(k) => model_view::ui(self.app, ui, model_view::Source::Resource(*k)),
            Tab::InstanceModel { area, path } => {
                let source = model_view::Source::Instance { area: *area, path: path.clone() };
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
            Tab::PlaceableGallery => crate::appearance_gallery::ui(self.app, ui),
            Tab::Options => crate::options::ui(self.app, ui),
            Tab::TileProperties => crate::tile_select::ui(self.app, ui),
        }
    }
}

impl TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> Id {
        Id::new(("tab", &*tab))
    }

    fn title(&mut self, tab: &mut Tab) -> WidgetText {
        match tab {
            Tab::Welcome => "Welcome".into(),
            Tab::NoArea => "No Area Open".into(),
            Tab::ModuleProperties => "Module Properties".into(),
            Tab::Script(k) => {
                let dirty = self.app.scripts.get(k).is_some_and(|b| b.is_dirty());
                format!("{k}{}", if dirty { " *" } else { "" }).into()
            }
            Tab::Gff(k) => k.to_string().into(),
            Tab::Resources => "Resources".into(),
            Tab::Palette => "Palettes".into(),
            Tab::Blueprint(k) if self.app.ws.as_ref().is_some_and(|ws| ws.is_viewed(k)) => {
                format!("{k} (the game's: view only)").into()
            }
            Tab::Blueprint(k) => k.to_string().into(),
            Tab::Factions => "Factions".into(),
            Tab::Journal => "Journal".into(),
            Tab::Tileset(id) => {
                let doc = self.app.tilesets.iter().find(|d| d.id == *id);
                doc.map_or("Tileset".into(), |d| d.title()).into()
            }
            Tab::Hak(id) => {
                let doc = self.app.haks.iter().find(|d| d.id == *id);
                doc.map_or("Hak".into(), |d| d.title()).into()
            }
            Tab::TalkTable => {
                let dirty = self.app.talk.as_ref().is_some_and(|t| t.is_dirty());
                format!("Talk Table{}", if dirty { " *" } else { "" }).into()
            }
            Tab::Dialog(k) => k.to_string().into(),
            Tab::Resource(k) => format!("{k} (read-only)").into(),
            Tab::Model(k) => k.to_string().into(),
            Tab::Area(r) => area_label(self.app, *r).into(),
            Tab::Instance { area, path } => instance_title(self.app, *area, path).into(),
            Tab::AreaProperties(area) => {
                format!("{} (Area Properties)", area_label(self.app, *area)).into()
            }
            Tab::AreasProperties(areas) => {
                format!("{} areas (Area Properties)", areas.len()).into()
            }
            Tab::Instances { area, paths } => format!("{} objects ({area})", paths.len()).into(),
            Tab::Blueprints(keys) => {
                let ext = keys[0].restype.extension().unwrap_or_default();
                format!("{} blueprints (.{ext})", keys.len()).into()
            }
            Tab::Manual => "User Manual".into(),
            Tab::References => "References".into(),
            Tab::PlaceableGallery => "Appearance Gallery".into(),
            Tab::Options => "Options".into(),
            Tab::TileProperties => "Tile Properties".into(),
            Tab::InstanceModel { area, path } => {
                format!("{} (preview)", instance_title(self.app, *area, path)).into()
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Tab) {
        // A little room round what a tab shows. (The dock is given no window
        // margin, `Moonglow::ui`, and takes its tabs' from that: without
        // this they sat flush against their pane's edge.)
        let margin = egui::Margin { left: 6, right: 6, top: 2, bottom: 6 };
        egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
            // (All the room, so a view that fills its tab still does.)
            ui.set_min_size(ui.available_size());
            self.body(ui, tab);
        });
    }

    /// The tab's menu (beside egui_dock's Eject and Close): Rename… for a
    /// resource of the module, which renames it everywhere it is named.
    fn context_menu(&mut self, ui: &mut Ui, tab: &mut Tab, _path: egui_dock::NodePath) {
        let ours = tab
            .renamable()
            .filter(|k| self.app.ws.as_ref().is_some_and(|ws| ws.module.contains(k)));
        if let Some(key) = ours
            && ui.button("Rename…").clicked()
        {
            self.app.actions.push(crate::Action::RenameDialog(key));
            ui.close();
        }
        // An area: its row in the module tree, opened out to what is
        // placed in it.
        if let Tab::Area(area) = tab
            && ui
                .button("Show in Module Tree")
                .on_hover_text("Its row in the module tree, opened out to what is placed in it")
                .clicked()
        {
            self.app.tree_reveal = Some((*area, true));
            ui.close();
        }
        // A window of its own fills the main pane, and goes back.
        if !tab.docks() && *tab != Tab::Palette {
            let label = if self.app.maximized.contains_key(tab) { "Restore" } else { "Maximize" };
            if ui.button(label).on_hover_text("Also: double-click the tab").clicked() {
                self.app.actions.push(crate::Action::ToggleMaximize(tab.clone()));
                ui.close();
            }
        }
        // Closing those of its pane (its own Close is the dock's, below).
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
        self.app.tab_buttons.push((tab.clone(), response.rect, response.layer_id));
        // An area's tab chosen: the module tree goes to the area.
        if let Tab::Area(area) = tab
            && response.clicked()
        {
            self.app.tree_reveal = Some((*area, false));
        }
        if response.middle_clicked() && tab.closeable() {
            self.app.actions.push(crate::Action::CloseTabs(vec![tab.clone()]));
        }
        if response.double_clicked() && !tab.docks() && *tab != Tab::Palette {
            self.app.actions.push(crate::Action::ToggleMaximize(tab.clone()));
        }
    }

    /// A hak with unsaved changes asks first; a closed hak is let go.
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
    /// pane's width, and the Options, whose page scrolls between its panels.
    /// The others scroll when wider or taller.
    fn scroll_bars(&self, tab: &Tab) -> [bool; 2] {
        let fits = matches!(tab, Tab::Script(_) | Tab::Manual | Tab::Options);
        [!fits, !fits]
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
    let areas = app.ws.as_ref().and_then(|ws| ws.module.areas().ok()).unwrap_or_default();
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
        if !app.settings.recent.is_empty() {
            ui.add_space(20.0);
            ui.strong("Recent modules");
            for p in app.settings.recent.clone() {
                let name =
                    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if ui.link(name).on_hover_text(p.display().to_string()).clicked() {
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
