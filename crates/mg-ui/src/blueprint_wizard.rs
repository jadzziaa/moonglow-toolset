//! Aurora's blueprint wizards (Wizards › Door Wizard and the others, and the
//! palette's New): each type's own steps, the palette category, the name,
//! and Launch Properties. What they make is `mg_module::blueprints`'.
//!
//! The Creature Wizard has its own window (`creature_wizard`).

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit};
use mg_gff::Struct;
use mg_module::blueprints::{self, SoundStyle};
use mg_module::palette::{BlueprintKind, Palette, PaletteNode};
use mg_resman::ResKey;
use mg_rules::GameData;

use crate::{Action, Moonglow, Tab};

/// The types with a wizard.
pub const KINDS: [BlueprintKind; 8] = [
    BlueprintKind::Door,
    BlueprintKind::Encounter,
    BlueprintKind::Item,
    BlueprintKind::Store,
    BlueprintKind::Placeable,
    BlueprintKind::Sound,
    BlueprintKind::Trigger,
    BlueprintKind::Waypoint,
];

/// A wizard page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// Items: the base item.
    ItemType,
    /// Waypoints: the tag and appearance.
    WaypointTag,
    /// Sounds: looping or single shots.
    Timing,
    /// Sounds: where they play from.
    Positioning,
    /// Sounds: the waves.
    Waves,
    /// Encounters: the creatures.
    Creatures,
    Category,
    Name,
}

fn steps(kind: BlueprintKind) -> &'static [Step] {
    use Step::*;
    match kind {
        BlueprintKind::Waypoint => &[WaypointTag, Category],
        BlueprintKind::Sound => &[Category, Timing, Positioning, Waves, Name],
        BlueprintKind::Encounter => &[Category, Creatures, Name],
        BlueprintKind::Item => &[ItemType, Name, Category],
        _ => &[Category, Name],
    }
}

/// Where a sound plays from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Positioning {
    AreaWide,
    Random,
    Positional,
}

/// A blueprint wizard's choices.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintWizard {
    pub kind: BlueprintKind,
    step: usize,
    pub category: Option<u8>,
    /// The name (a waypoint's: its tag).
    pub name: String,
    /// Whether the name was typed (else it follows the category).
    named: bool,
    pub launch: bool,
    pub appearance: u8,
    pub looping: Option<bool>,
    pub positioning: Option<Positioning>,
    pub sounds: Vec<ResRef>,
    pub creatures: Vec<Struct>,
    pub base_item: Option<u32>,
}

impl BlueprintWizard {
    pub fn new(kind: BlueprintKind) -> BlueprintWizard {
        BlueprintWizard {
            kind,
            step: 0,
            category: None,
            name: String::new(),
            named: false,
            // Aurora launches the properties after the Sound Wizard only.
            launch: kind == BlueprintKind::Sound,
            appearance: 1,
            looping: None,
            positioning: None,
            sounds: Vec::new(),
            creatures: Vec::new(),
            base_item: None,
        }
    }

    fn page(&self) -> Step {
        steps(self.kind)[self.step]
    }

    fn last(&self) -> bool {
        self.step + 1 == steps(self.kind).len()
    }

    /// Whether the page's choices let the wizard go on.
    fn complete(&self) -> bool {
        match self.page() {
            Step::ItemType => self.base_item.is_some(),
            Step::WaypointTag => !self.name.trim().is_empty(),
            Step::Timing => self.looping.is_some(),
            Step::Positioning => self.positioning.is_some(),
            Step::Waves => !self.sounds.is_empty(),
            Step::Creatures => !self.creatures.is_empty(),
            Step::Category => self.category.is_some(),
            Step::Name => !self.name.trim().is_empty(),
        }
    }

    fn style(&self) -> SoundStyle {
        match (self.looping.unwrap_or(true), self.positioning.unwrap_or(Positioning::Positional)) {
            (true, Positioning::AreaWide) => SoundStyle::LoopingAreaWide,
            (true, _) => SoundStyle::LoopingPositional,
            (false, Positioning::AreaWide) => SoundStyle::SingleShotAreaWide,
            (false, Positioning::Random) => SoundStyle::SingleShotRandom,
            (false, Positioning::Positional) => SoundStyle::SingleShotPositional,
        }
    }

    /// The blueprint, with the resref it takes in the module.
    fn build(&self, game: &GameData, taken: impl Fn(&ResRef) -> bool) -> (ResRef, mg_gff::Gff) {
        let name = self.name.trim();
        let resref = blueprints::resref(name, taken);
        let category = self.category.unwrap_or(0);
        let g = match self.kind {
            BlueprintKind::Waypoint => {
                blueprints::waypoint(resref, &blueprints::tag(name), self.appearance, category)
            }
            BlueprintKind::Sound => {
                blueprints::sound(resref, name, category, self.style(), &self.sounds)
            }
            BlueprintKind::Trigger => blueprints::trigger(game, resref, name, category),
            BlueprintKind::Encounter => {
                blueprints::encounter(resref, name, category, self.creatures.clone())
            }
            BlueprintKind::Store => blueprints::store(resref, name, category),
            BlueprintKind::Placeable => blueprints::placeable(resref, name, category),
            BlueprintKind::Door => blueprints::door(resref, name, category),
            BlueprintKind::Item => {
                blueprints::item(game, resref, name, self.base_item.unwrap_or(0), category)
            }
            BlueprintKind::Creature => unreachable!("the Creature Wizard's own window"),
        };
        (resref, g)
    }
}

/// The type's palette categories, as Aurora's wizards list them: by name,
/// without the empty "assign" branch.
pub(crate) fn categories(game: &GameData, kind: BlueprintKind) -> Vec<PaletteNode> {
    let skeleton = game
        .resman
        .get_named(&format!("{}pal", kind.name()), ResType::ITP)
        .ok()
        .and_then(|d| mg_gff::Gff::read(&d).ok())
        .map(|g| Palette::read(&g))
        .unwrap_or_default();
    fn sorted(nodes: &[PaletteNode], game: &GameData) -> Vec<PaletteNode> {
        let mut out: Vec<PaletteNode> = nodes
            .iter()
            .filter(|n| n.id.is_some() || !n.children.is_empty())
            .map(|n| PaletteNode { children: sorted(&n.children, game), ..n.clone() })
            .collect();
        out.sort_by_key(|n| n.name.text(game).to_lowercase());
        out
    }
    sorted(&skeleton.nodes, game)
}

fn category_name(nodes: &[PaletteNode], id: u8, game: &GameData) -> Option<String> {
    nodes.iter().find_map(|n| {
        if n.id == Some(id) {
            Some(n.name.text(game))
        } else {
            category_name(&n.children, id, game)
        }
    })
}

pub(crate) fn category_tree(
    ui: &mut Ui,
    nodes: &[PaletteNode],
    game: &GameData,
    chosen: &mut Option<u8>,
) {
    for n in nodes {
        let name = n.name.text(game);
        match n.id {
            Some(id) if n.children.is_empty() => {
                if ui.selectable_label(*chosen == Some(id), name).clicked() {
                    *chosen = Some(id);
                }
            }
            _ => {
                egui::CollapsingHeader::new(&name)
                    .id_salt(("wizard-category", &name))
                    .show(ui, |ui| category_tree(ui, &n.children, game, chosen));
            }
        }
    }
}

/// The base items the Item Wizard offers, by name.
fn item_types(game: &GameData) -> Vec<(u32, String)> {
    let Ok(t) = game.table("baseitems") else { return Vec::new() };
    let mut out: Vec<(u32, String)> = (0..t.len())
        .filter(|&r| t.get_int(r, "StorePanel").is_some())
        .filter_map(|r| {
            let name = game.string(mg_core::StrRef(t.get_int(r, "Name")? as u32))?;
            (!name.is_empty()).then_some((r as u32, name))
        })
        .collect();
    out.sort_by_key(|(_, n)| n.to_lowercase());
    out
}

fn window(title: &str) -> egui::Window<'_> {
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
}

/// The open blueprint wizard.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut w) = app.blueprint_wizard.take() else { return };
    if app.game.is_none() {
        return;
    }
    let ctx = ui.ctx().clone();
    let mut close = false;
    let mut finish = false;
    let title = format!("{} Wizard", w.kind.label().trim_end_matches('s'));
    window(&title).show(&ctx, |ui| {
        ui.set_min_width(420.0);
        let key_taken = |app: &Moonglow, r: &ResRef| {
            app.ws.as_ref().is_some_and(|ws| ws.module.contains(&ResKey::new(*r, w.kind.restype())))
        };
        match w.page() {
            Step::ItemType => {
                ui.heading("Item Type");
                ui.label("Please choose the type of item you wish to create");
                let types = item_types(app.game.as_ref().expect("checked"));
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (row, name) in types {
                        if ui.selectable_label(w.base_item == Some(row), name).clicked() {
                            w.base_item = Some(row);
                        }
                    }
                });
            }
            Step::WaypointTag => {
                ui.heading("Waypoint Wizard");
                egui::Grid::new("wizard-waypoint").num_columns(2).show(ui, |ui| {
                    ui.label("Tag");
                    ui.text_edit_singleline(&mut w.name);
                    ui.end_row();
                    ui.label("Appearance");
                    let looks = app
                        .game
                        .as_ref()
                        .expect("checked")
                        .choices(
                            "waypoint",
                            mg_rules::ChoiceColumns { name: Some("STRREF"), label: Some("LABEL") },
                        )
                        .unwrap_or_default();
                    let shown = looks
                        .iter()
                        .find(|c| c.row == w.appearance as usize)
                        .map_or_else(String::new, |c| c.text.clone());
                    egui::ComboBox::from_id_salt("wizard-waypoint-look")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for c in &looks {
                                ui.selectable_value(&mut w.appearance, c.row as u8, &c.text);
                            }
                        });
                    ui.end_row();
                });
            }
            Step::Timing => {
                ui.heading("Timing");
                ui.label("Choose how you want the Sound Object to play.");
                if ui.radio(w.looping == Some(true), "Seamlessly looping").clicked() {
                    w.looping = Some(true);
                    w.sounds.clear();
                }
                ui.weak("One wave that seamlessly repeats over and over again with no breaks");
                if ui.radio(w.looping == Some(false), "Single-shot(s)").clicked() {
                    w.looping = Some(false);
                    w.sounds.clear();
                }
                ui.weak(
                    "One or more waves that are played separately with a random delay between each",
                );
            }
            Step::Positioning => {
                ui.heading("Positioning");
                ui.label("Choose where you would like this Sound Object to play from.");
                let single = w.looping == Some(false);
                if w.positioning == Some(Positioning::Random) && !single {
                    w.positioning = None;
                }
                for (p, text, enabled) in [
                    (Positioning::AreaWide, "Area-wide", true),
                    (Positioning::Random, "Random Positional", single),
                    (Positioning::Positional, "Positional", true),
                ] {
                    if ui
                        .add_enabled(
                            enabled,
                            egui::RadioButton::new(w.positioning == Some(p), text),
                        )
                        .clicked()
                    {
                        w.positioning = Some(p);
                    }
                }
            }
            Step::Waves => {
                ui.heading("Wave List");
                ui.label("Select the Wave files that this Sound Object will play");
                let pick_id = egui::Id::new("wizard-waves");
                if let Some(r) = app.take_pick(pick_id).filter(|r| !r.is_empty()) {
                    // A looping sound plays one wave.
                    if w.looping == Some(true) {
                        w.sounds.clear();
                    }
                    w.sounds.push(r);
                }
                let mut remove = None;
                for (i, s) in w.sounds.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(s.to_string());
                        if ui.small_button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    w.sounds.remove(i);
                }
                let can_add = w.looping == Some(false) || w.sounds.is_empty();
                if ui.add_enabled(can_add, egui::Button::new("Add Sounds…")).clicked() {
                    app.open_picker(pick_id, "Add a sound", &[ResType::WAV]);
                    if let Some(p) = app.picker.as_mut() {
                        p.filter = if w.looping == Some(true) { "al_" } else { "as_" }.into();
                    }
                }
            }
            Step::Creatures => {
                ui.heading("Creature List");
                ui.label("Select the creatures that this Encounter can spawn.");
                let add = crate::blueprint::picker::palette_picker(
                    app,
                    ui,
                    BlueprintKind::Creature,
                    "Add Creature",
                    egui::Id::new("wizard-creatures"),
                );
                if let Some(r) = add {
                    w.creatures.push(crate::blueprint::encounter::creature_entry(app, r));
                }
                let mut remove = None;
                for (i, c) in w.creatures.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let r = c.resref("ResRef").unwrap_or(ResRef::EMPTY);
                        ui.label(format!("{r}  (CR {})", c.float("CR").unwrap_or(0.0)));
                        if ui.small_button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    w.creatures.remove(i);
                }
            }
            Step::Category => {
                ui.heading("Assign Palette Category");
                ui.label("Choose the palette category that this new blueprint should appear under");
                let game = app.game.as_ref().expect("checked");
                let nodes = categories(game, w.kind);
                let before = w.category;
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    category_tree(ui, &nodes, game, &mut w.category);
                });
                // The suggested name follows the category.
                if w.category != before
                    && !w.named
                    && w.kind != BlueprintKind::Item
                    && w.kind != BlueprintKind::Waypoint
                    && let Some(name) = w.category.and_then(|id| category_name(&nodes, id, game))
                {
                    w.name = (1..)
                        .map(|n| blueprints::default_name(&name, n))
                        .find(|s| !key_taken(app, &blueprints::resref(s, |_| false)))
                        .expect("a free name");
                }
            }
            Step::Name => {
                ui.heading("Name");
                ui.label("Please enter a name for the new blueprint");
                let edit = egui::TextEdit::singleline(&mut w.name).hint_text("Name");
                if ui.add(edit).changed() {
                    w.named = true;
                }
            }
        }
        if w.last() {
            ui.checkbox(&mut w.launch, "Launch Properties Dialog");
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.add_enabled(w.step > 0, egui::Button::new("< Back")).clicked() {
                w.step -= 1;
            }
            let ok = w.complete();
            if ui.add_enabled(ok && !w.last(), egui::Button::new("Next >")).clicked()
                || (ok && !w.last() && crate::widgets::enter(ui))
            {
                w.step += 1;
            }
            if ui.add_enabled(ok && w.last(), egui::Button::new("Finish")).clicked()
                || (ok && w.last() && crate::widgets::enter(ui))
            {
                finish = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if finish {
        let taken = |r: &ResRef| {
            app.ws.as_ref().is_some_and(|ws| ws.module.contains(&ResKey::new(*r, w.kind.restype())))
        };
        let (resref, g) = w.build(app.game.as_ref().expect("checked"), taken);
        let key = ResKey::new(resref, w.kind.restype());
        match g.to_bytes() {
            Ok(data) => {
                app.actions.push(Action::Apply(Command::new(
                    format!("New {}", w.kind.label().trim_end_matches('s').to_lowercase()),
                    vec![Edit::SetResource { key, data: Some(data) }],
                )));
                if w.launch {
                    app.actions.push(Action::OpenTab(Tab::Blueprint(key)));
                }
            }
            Err(e) => app.log.error(e.to_string()),
        }
        close = true;
    }
    if !close {
        app.blueprint_wizard = Some(w);
    }
}
