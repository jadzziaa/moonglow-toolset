//! Custom Light… (the lights proposal's way B, behind an option that is
//! off by default): a light of the color, reach and shadows chosen, made
//! as content ([`mg_module::lights`]: a model with one light node and a
//! placeables.2da row) written into a hak the builder chooses in its
//! Right Dock panel (M4's U29; a window before), which says plainly what
//! it writes where, and that the module's players must have that hak,
//! before anything is written.

use egui::Ui;
use glam::Vec3;
use mg_area::ObjectKind;
use mg_core::ResRef;
use mg_module::lights::{Flicker, HakChoice, LightSpec, Plan};
use mg_uikit::Controls as _;

use crate::Moonglow;

/// Which hak the light goes into, as chosen in its panel.
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

/// The Custom Light panel's state (M4's U29: its Right Dock panel; a window
/// before).
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

/// The Right Dock's Custom Light (Change Light where it changes lights
/// Moonglow made; M4's U29, a window before), while it is open: its main
/// action (Write and Place, Place, Write and Change, Change) and Cancel at
/// its top (`tool_sections::actions`), then the light's settings, the hak
/// it is written into and what writing it does. `detail`: its header's
/// words where its section has no header.
pub(crate) fn panel(
    app: &mut Moonglow,
    ui: &mut Ui,
    detail: Option<&str>,
) -> Option<crate::tool_sections::Answer> {
    let mut d = app.light_draft.take()?;
    let key = (d.spec.normalized(), d.hak.clone());
    if d.planned.as_ref().is_none_or(|(k, _)| *k != key) {
        d.planned = Some((key, plan_for(app, d.spec, &d.hak)));
    }
    // (The module's own haks, found in the hak folders as it opens: not
    // each of them looked for every frame.)
    let names = crate::area_tools::kept_while_shown(
        ui.ctx(),
        egui::Id::new("custom-light-haks"),
        || -> Vec<String> { listed(app).into_iter().map(|(n, _)| n).collect() },
    );
    let ok = matches!(d.planned, Some((_, Ok(_))));
    let label = words_of(&d);
    let tip = match d.change.is_empty() {
        true => "Write the light into the hak and place it",
        false => "Write the light into the hak and give it to the lights selected",
    };
    let keys = crate::palette_view::Keys::LightMaker;
    let leave = ("Cancel", "Close it without writing anything");
    let why = match &d.planned {
        Some((_, Err(e))) => Some(e.clone()),
        _ => None,
    };
    let main = Some((label, ok, tip));
    let answer = crate::tool_sections::actions(ui, keys, main, leave, detail, why.as_deref());
    crate::tool_sections::scrolled(ui, "custom-light", |ui| {
        // (Properties' grid, M4's U8 to U10: its label column, its fields,
        // the check boxes in the fields' column. M4's U29 review.)
        let grid = crate::blueprint::form_grid(ui, "custom-light").labels().spacing([12.0, 6.0]);
        grid.show(ui, |ui| {
            crate::widgets::field_label(ui, "Color");
            ui.color_edit_button_srgb(&mut d.spec.color);
            ui.end_row();
            crate::widgets::field_label(ui, "Radius (m)");
            crate::widgets::spin_draft(ui, &mut d.spec.radius, 0.1..=100.0, 0.5, |x| {
                x.speed(0.05).max_decimals(2)
            });
            ui.end_row();
            crate::widgets::field_label(ui, "Multiplier")
                .on_hover_text("How much further the light reaches, not how bright it is");
            crate::widgets::spin_draft(ui, &mut d.spec.multiplier, 0.1..=10.0, 0.1, |x| {
                x.speed(0.01).max_decimals(2)
            });
            ui.end_row();
            crate::widgets::field_label(ui, "Priority").on_hover_text(
                "1 to 5: where too many lights meet, the game keeps priority 1 first",
            );
            let mut p = u32::from(d.spec.priority);
            crate::widgets::spin_draft(ui, &mut p, 1..=5, 1.0, |x| x);
            d.spec.priority = p.clamp(1, 5) as u8;
            ui.end_row();
            crate::widgets::field_label(ui, "Flicker").on_hover_text(
                "How the light flickers. A flickering light costs the game more.",
            );
            crate::widgets::dropdown("custom-light-flicker")
                .selected_text(d.spec.flicker.name())
                .width(110.0)
                .show(ui, &mut d.spec.flicker, &Flicker::ALL, |f| f.name());
            ui.end_row();
            // (Each wrapped at the page's edge: a cell's width is the
            // grid's last frame's.)
            let boxed = |ui: &mut Ui, on: &mut bool, text: &str| {
                let room = crate::blueprint::room_right(ui).max(40.0);
                ui.scope(|ui| {
                    ui.set_max_width(room);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    ui.check_box(on, text)
                })
                .inner
            };
            ui.label("");
            boxed(ui, &mut d.spec.shadows, "Creatures' shadows are cast from it");
            ui.end_row();
            ui.label("");
            boxed(ui, &mut d.spec.lights_objects, "Lights creatures and placeables too")
                .on_hover_text("The light node's affectdynamic: when off, the light illuminates the ground only");
            ui.end_row();
        });
        ui.weak(format!("Its light ends {} m out.", (d.spec.reach() * 100.0).round() / 100.0));
        if d.spec.flickers() {
            // (Said where it is chosen: the game plays no static
            // placeable's flicker, and a dynamic one costs it more.)
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "It is placed as a dynamic placeable, as the game plays no static \
                         placeable's flicker. A dynamic placeable costs the game more than a \
                         static one.",
                    )
                    .weak(),
                )
                .wrap(),
            );
        }
        ui.add_space(crate::widgets::SECTION_GAP);
        crate::widgets::section_heading(ui, "Written Into");
        for name in &names {
            let on = d.hak == HakPick::Listed(name.clone());
            if ui.radio_button(on, format!("{name}.hak")).clicked() {
                d.hak = HakPick::Listed(name.clone());
            }
        }
        ui.horizontal_wrapped(|ui| {
            let typed = match &d.hak {
                HakPick::New(n) => Some(n.clone()),
                _ => None,
            };
            let chosen =
                ui.radio_button(typed.is_some(), "A new hak:").clicked() && typed.is_none();
            if chosen {
                d.hak = HakPick::New(String::new());
            }
            let mut name = typed.clone().unwrap_or_default();
            let width = ui.available_width().clamp(80.0, 160.0);
            let r = ui.add_enabled(
                typed.is_some() || chosen,
                mg_uikit::text_field(&mut name).desired_width(width).hint_text("name"),
            );
            if chosen {
                r.request_focus();
            }
            if r.changed() {
                d.hak = HakPick::New(name);
            }
        });
        // (What it will write; where it cannot, why is said under its
        // actions, in sight.)
        if let Some((_, Ok(p))) = &d.planned {
            ui.add_space(crate::widgets::SECTION_GAP);
            ui.add(egui::Label::new(p.describe()).wrap());
        }
    });
    let ok = matches!(d.planned, Some((_, Ok(_))));
    app.light_draft = Some(d);
    answer.or_else(|| crate::tool_sections::enter_after(ui, keys, ok, true))
}

/// Its main action's words: Write and Place, Place (a light written
/// before, its row used again), Write and Change, Change.
fn words_of(d: &LightDraft) -> &'static str {
    let reused = matches!(&d.planned, Some((_, Ok(p))) if p.reused);
    match (reused, d.change.is_empty()) {
        (true, true) => "Place",
        (true, false) => "Change",
        (false, true) => "Write and Place",
        (false, false) => "Write and Change",
    }
}

/// Its main action's words where it is open and enabled (the Status Bar's
/// Enter hint).
pub(crate) fn main_words(app: &Moonglow) -> Option<&'static str> {
    let d = app.light_draft.as_ref()?;
    matches!(d.planned, Some((_, Ok(_)))).then(|| words_of(d))
}

/// Its main action pressed (made as the frame begins:
/// `tool_sections::follow`): the light written and placed, or given to the
/// lights chosen, the panel closed.
pub(crate) fn finish(app: &mut Moonglow) {
    let Some(mut d) = app.light_draft.take() else { return };
    if let Some((_, Ok(plan))) = d.planned.take() {
        app.last_light = d.spec.normalized();
        made(app, &d, &plan);
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
        crate::area_view::change_light(app, &view, plan.row, plan.spec.flickers());
    } else if let Some(at) = d.at {
        let flickers = plan.spec.flickers();
        crate::area_view::add_light_row(app, &mut view, plan.row, &plan.spec.label(), at, flickers);
    } else {
        // (The palette's: placed with a click in the view, as its lights.)
        app.palette.placing_light = Some(plan.row);
    }
    app.area_views.insert(d.area, view);
}
