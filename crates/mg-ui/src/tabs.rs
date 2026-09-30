//! The central area's tabs.

use egui::{Id, Ui, WidgetText};
use egui_dock::TabViewer;
use mg_core::ResType;
use mg_resman::ResKey;

use crate::{
    Action, Moonglow, browser, dialog_view, faction_view, gff_view, journal_view, module_props,
    script_view,
};

/// A tab in the central area.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tab {
    /// Shown with no module open.
    Welcome,
    /// Module Properties (module.ifo).
    ModuleProperties,
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
    /// A conversation.
    Dialog(ResKey),
    /// A resource from the load order, read-only.
    Resource(ResKey),
}

impl Tab {
    /// The tab that edits a resource.
    pub fn for_resource(key: ResKey) -> Option<Tab> {
        match key.restype {
            ResType::NSS => Some(Tab::Script(key)),
            ResType::IFO => Some(Tab::ModuleProperties),
            ResType::FAC => Some(Tab::Factions),
            ResType::JRL => Some(Tab::Journal),
            ResType::DLG => Some(Tab::Dialog(key)),
            t if t.is_gff() => Some(Tab::Gff(key)),
            _ => None,
        }
    }
}

pub(crate) struct Viewer<'a> {
    pub(crate) app: &'a mut Moonglow,
}

impl TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> Id {
        Id::new(("tab", &*tab))
    }

    fn title(&mut self, tab: &mut Tab) -> WidgetText {
        match tab {
            Tab::Welcome => "Welcome".into(),
            Tab::ModuleProperties => "Module Properties".into(),
            Tab::Script(k) => {
                let dirty = self.app.scripts.get(k).is_some_and(|b| b.is_dirty());
                format!("{k}{}", if dirty { " *" } else { "" }).into()
            }
            Tab::Gff(k) => k.to_string().into(),
            Tab::Resources => "Resources".into(),
            Tab::Factions => "Factions".into(),
            Tab::Journal => "Journal".into(),
            Tab::Dialog(k) => k.to_string().into(),
            Tab::Resource(k) => format!("{k} (read-only)").into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Tab) {
        match tab {
            Tab::Welcome => welcome(self.app, ui),
            Tab::ModuleProperties => module_props::ui(self.app, ui),
            Tab::Script(k) => script_view::ui(self.app, ui, *k),
            Tab::Gff(k) => gff_view::ui(self.app, ui, *k),
            Tab::Resources => browser::ui(self.app, ui),
            Tab::Factions => faction_view::ui(self.app, ui),
            Tab::Journal => journal_view::ui(self.app, ui),
            Tab::Dialog(k) => dialog_view::ui(self.app, ui, *k),
            Tab::Resource(k) => browser::resource_ui(self.app, ui, *k),
        }
    }

    fn is_closeable(&self, tab: &Tab) -> bool {
        *tab != Tab::Welcome
    }
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
