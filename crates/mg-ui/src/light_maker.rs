//! Custom Light… (the lights proposal's way B, behind an option that is
//! off by default): a light of the color, reach and shadows chosen, made
//! as content ([`mg_module::lights`]: a model with one light node and a
//! placeables.2da row) written into a hak the builder chooses in the
//! window, which says plainly what it writes where, and that the
//! module's players must have that hak, before anything is written.

use egui::Ui;
use glam::Vec3;
use mg_area::ObjectKind;
use mg_core::ResRef;
use mg_module::lights::{HakChoice, LightSpec, Plan};

use crate::Moonglow;

/// Which hak the light goes into, as chosen in the window.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum HakPick {
    /// None yet: nothing is written until one is chosen.
    #[default]
    None,
    /// One of the module's haks, by name.
    Listed(String),
    /// A new hak, by the name typed, made in the user's `hak/` and put at
    /// the top of the module's list.
    New(String),
}

/// The Custom Light window's state.
#[derive(Debug, Clone)]
pub struct LightDraft {
    pub area: ResRef,
    /// Where it goes (a ground point: placed a little above it); `None`:
    /// placed with a click in the view afterwards (the palette's), or the
    /// lights in `change` given it.
    pub at: Option<Vec3>,
    /// Lights Moonglow made, selected, to be given these settings instead
    /// (Change Light…).
    pub change: Vec<(ObjectKind, usize)>,
    pub spec: LightSpec,
    pub hak: HakPick,
    /// What writing it would do, worked out for these settings and this
    /// hak (not every frame: placeables.2da is long).
    planned: Option<((LightSpec, HakPick), Result<Plan, String>)>,
}

impl LightDraft {
    pub fn new(app: &Moonglow, area: ResRef, at: Option<Vec3>) -> LightDraft {
        // (The project's hak, where it is one of the module's, chosen at
        // first; else none, until the builder picks one.)
        let hak = project_rules(app)
            .hak
            .filter(|h| listed(app).iter().any(|(n, _)| n.eq_ignore_ascii_case(h)))
            .map_or(HakPick::None, HakPick::Listed);
        LightDraft { area, at, change: Vec::new(), spec: app.last_light, hak, planned: None }
    }
}

fn project_rules(app: &Moonglow) -> mg_module::project_file::LightRules {
    app.ws.as_ref().map(|ws| ws.module.project_file.lights.clone()).unwrap_or_default()
}

/// Whether lights are made as content: the project's `[lights] generated`
/// where it says, else Options › Areas' (off by default).
pub fn generated_lights(app: &Moonglow) -> bool {
    app.ws.is_some() && project_rules(app).generated_or(app.settings.generated_lights)
}

/// The module's haks that are the builder's own (not the game's), by
/// name, with their files.
fn listed(app: &Moonglow) -> Vec<(String, std::path::PathBuf)> {
    let (Some(_), Some(install)) = (&app.ws, &app.install) else { return Vec::new() };
    // (As the game data has them layered: the module's list, its unsaved
    // changes too.)
    app.haks_layered
        .iter()
        .cloned()
        .filter_map(|name| {
            let path = install.find_hak(&name)?;
            (!path.starts_with(&install.root)).then_some((name, path))
        })
        .collect()
}

/// The hak chosen, as `mg_module::lights` takes it.
fn choice(app: &Moonglow, pick: &HakPick) -> Result<HakChoice, String> {
    match pick {
        HakPick::None => Err("Choose the hak the light goes into".into()),
        HakPick::Listed(name) => {
            let (name, path) = listed(app)
                .into_iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("{name}.hak is not one of the module's"))?;
            Ok(HakChoice { name, path, new: false })
        }
        HakPick::New(name) => {
            let dir = app.install.as_ref().and_then(|i| i.user_hak_dir());
            let dir = dir.ok_or("no user folder for a new hak")?;
            let c = mg_module::lights::new_hak(&dir, name)?;
            if c.path.exists() {
                return Err(format!(
                    "{} is there already: add it to the module's haks (Module Properties) to \
                     write into it",
                    c.path.display()
                ));
            }
            Ok(c)
        }
    }
}

fn plan_for(app: &Moonglow, spec: LightSpec, pick: &HakPick) -> Result<Plan, String> {
    let hak = choice(app, pick)?;
    // (Open in the hak editor: its own edits would be lost, or ours.)
    if app.haks.iter().any(|d| d.hak.path.as_deref() == Some(hak.path.as_path())) {
        return Err(format!("{}.hak is open in the hak editor: close it there first", hak.name));
    }
    let game = app.game.as_deref().ok_or("no game data")?;
    let first = project_rules(app).first_row;
    mg_module::lights::plan(&game.resman, game.codepage(), spec, hak, first)
}

/// Draws the window, when open.
pub(crate) fn window(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut d) = app.light_draft.take() else { return };
    // (The option turned off meanwhile: nothing of it shows.)
    if !generated_lights(app) {
        return;
    }
    let key = (d.spec.normalized(), d.hak.clone());
    if d.planned.as_ref().is_none_or(|(k, _)| *k != key) {
        d.planned = Some((key.clone(), plan_for(app, d.spec, &d.hak)));
    }
    let mut open = true;
    let mut done = None;
    let names: Vec<String> = listed(app).into_iter().map(|(n, _)| n).collect();
    let title = if d.change.is_empty() { "Custom Light" } else { "Change Light" };
    crate::widgets::dialog(ui.ctx(), title, &mut open).resizable(false).collapsible(false).show(
        ui.ctx(),
        |ui| {
            ui.set_max_width(420.0);
            egui::Grid::new("custom-light").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "Color");
                ui.color_edit_button_srgb(&mut d.spec.color);
                ui.end_row();
                crate::widgets::field_label(ui, "Radius (m)");
                crate::widgets::spin_draft(ui, &mut d.spec.radius, 0.1..=100.0, 0.5, |x| {
                    x.speed(0.05).max_decimals(2)
                });
                ui.end_row();
                crate::widgets::field_label(ui, "Multiplier").on_hover_text(
                    "How much further it reaches: the game lengthens a light's reach by it, \
                     not its brightness",
                );
                crate::widgets::spin_draft(ui, &mut d.spec.multiplier, 0.1..=10.0, 0.1, |x| {
                    x.speed(0.01).max_decimals(2)
                });
                ui.end_row();
                crate::widgets::field_label(ui, "Priority").on_hover_text(
                    "1 to 5: where more lights shine at one place than the game draws, those of \
                     priority 1 are kept first",
                );
                let mut p = u32::from(d.spec.priority);
                crate::widgets::spin_draft(ui, &mut p, 1..=5, 1.0, |x| x);
                d.spec.priority = p.clamp(1, 5) as u8;
                ui.end_row();
            });
            ui.checkbox(&mut d.spec.shadows, "Creatures' shadows are cast from it");
            ui.checkbox(&mut d.spec.lights_objects, "Lights creatures and placeables too")
                .on_hover_text("The light node's affectdynamic: off, it lights the ground alone");
            ui.weak(format!("Its light ends {} m out.", (d.spec.reach() * 100.0).round() / 100.0));
            ui.add_space(crate::widgets::SECTION_GAP);
            crate::widgets::section_heading(ui, "Written Into");
            for name in &names {
                let on = d.hak == HakPick::Listed(name.clone());
                if ui.radio(on, format!("{name}.hak")).clicked() {
                    d.hak = HakPick::Listed(name.clone());
                }
            }
            ui.horizontal(|ui| {
                let typed = match &d.hak {
                    HakPick::New(n) => Some(n.clone()),
                    _ => None,
                };
                let chosen = ui.radio(typed.is_some(), "A new hak:").clicked() && typed.is_none();
                if chosen {
                    d.hak = HakPick::New(String::new());
                }
                let mut name = typed.clone().unwrap_or_default();
                let r = ui.add_enabled(
                    typed.is_some() || chosen,
                    egui::TextEdit::singleline(&mut name).desired_width(160.0).hint_text("name"),
                );
                if chosen {
                    r.request_focus();
                }
                if r.changed() {
                    d.hak = HakPick::New(name);
                }
            });
            ui.add_space(crate::widgets::SECTION_GAP);
            let plan = d.planned.as_ref().map(|(_, p)| p);
            match plan {
                Some(Ok(p)) => {
                    ui.add(egui::Label::new(p.describe()).wrap());
                }
                Some(Err(e)) => {
                    ui.colored_label(ui.visuals().warn_fg_color, e);
                }
                None => {}
            }
            ui.horizontal(|ui| {
                let ok = matches!(plan, Some(Ok(_)));
                let reused = matches!(plan, Some(Ok(p)) if p.reused);
                let label = match (reused, d.change.is_empty()) {
                    (true, true) => "Place",
                    (true, false) => "Change",
                    (false, true) => "Write and Place",
                    (false, false) => "Write and Change",
                };
                if ui.add_enabled(ok, egui::Button::new(label)).clicked() {
                    done = Some(true);
                }
                if crate::widgets::cancel(ui) {
                    done = Some(false);
                }
            });
        },
    );
    match done {
        Some(true) => {
            if let Some((_, Ok(plan))) = d.planned.take() {
                app.last_light = d.spec.normalized();
                made(app, &d, &plan);
            }
        }
        Some(false) => {}
        None if open => app.light_draft = Some(d),
        None => {}
    }
}

/// Writes the plan's model and table into its hak, has the game data read
/// it, and places the light (or changes the lights chosen to it).
fn made(app: &mut Moonglow, d: &LightDraft, plan: &Plan) {
    let codepage = app.game.as_deref().map_or_else(Default::default, |g| g.codepage());
    if let Err(e) = mg_module::lights::write(plan, codepage) {
        app.log.error(format!("Custom Light: {e}"));
        return;
    }
    if !plan.reused {
        app.log.info(format!(
            "Custom Light: wrote {} and row {} of placeables.2da into {}",
            plan.model,
            plan.row,
            plan.hak.path.display()
        ));
    }
    // A new hak goes at the top of the module's list; one that is there
    // is read again.
    if plan.hak.new {
        let Some(ws) = app.ws.as_mut() else { return };
        let info_key = crate::module_props::info_key();
        let Ok(info) = ws.doc(&info_key) else { return };
        let list = mg_module::attach::hak_list(&info.root, std::slice::from_ref(&plan.hak.name));
        let edit = mg_edit::Edit::SetField {
            key: info_key,
            path: mg_edit::GffPath::root(),
            label: mg_schema::ifo::MOD_HAK_LIST.label.to_string(),
            value: Some(list),
        };
        let what = format!("Add hak {}", plan.hak.name);
        if let Err(e) = app.apply(mg_edit::Command::new(what, vec![edit])) {
            app.log.error(format!("Custom Light: {e}"));
            return;
        }
        app.sync_haks();
    } else {
        app.reload_resources(false);
    }
    let Some(mut view) = app.area_views.remove(&d.area) else { return };
    if !d.change.is_empty() {
        view.selection = d.change.clone();
        crate::area_view::set_appearance(app, &view, "Change light", plan.row);
    } else if let Some(at) = d.at {
        crate::area_view::add_light_row(app, &mut view, plan.row, &plan.spec.label(), at);
    } else {
        // (The palette's: placed with a click in the view, as its lights.)
        app.palette.placing_light = Some(plan.row);
    }
    app.area_views.insert(d.area, view);
}
