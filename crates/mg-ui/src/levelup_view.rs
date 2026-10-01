//! The Creature Levelup Wizard (Aurora's `TfrmCreatureLevelupWizard`, from
//! a creature's Classes page or its context menu in the area viewer): the
//! classes to level up to, up to eight, a class the creature does not have
//! added from the list; OK levels it up by the classes' packages
//! (`GameData::level_up`) as one undoable command.

use mg_edit::{Command, Edit, GffPath};
use mg_gff::Struct;
use mg_resman::ResKey;
use mg_rules::ChoiceColumns;

use crate::{Action, Moonglow};

/// The labels levelling up changes.
const CHANGED: [&str; 12] = [
    "ClassList",
    "FeatList",
    "SkillList",
    "HitPoints",
    "CurrentHitPoints",
    "SkillPoints",
    "Str",
    "Dex",
    "Con",
    "Int",
    "Wis",
    "Cha",
];

/// The wizard's state.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelupWizard {
    /// The creature: a blueprint (`path` empty) or one in an area's GIT.
    pub key: ResKey,
    pub path: GffPath,
    /// (class, level, the level it has): the creature's classes first.
    pub slots: Vec<(u32, u32, u32)>,
    /// The class chosen in the list.
    pub chosen: Option<u32>,
}

impl LevelupWizard {
    /// The wizard for a creature, its classes as they are.
    pub fn new(key: ResKey, path: GffPath, creature: &Struct) -> LevelupWizard {
        let slots = creature
            .list("ClassList")
            .unwrap_or(&[])
            .iter()
            .map(|c| {
                let class = c.integer("Class").unwrap_or(0).max(0) as u32;
                let level = c.integer("ClassLevel").unwrap_or(1).max(1) as u32;
                (class, level, level)
            })
            .collect();
        LevelupWizard { key, path, slots, chosen: None }
    }
}

/// Opens the wizard for the creature at `path` of `key`.
pub(crate) fn open(app: &mut Moonglow, key: ResKey, path: GffPath) {
    let Some(ws) = app.ws.as_mut() else { return };
    let Ok(doc) = ws.doc(&key) else { return };
    let Some(creature) = path.get(&doc.root) else { return };
    app.levelup = Some(LevelupWizard::new(key, path.clone(), creature));
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.levelup.take() else { return };
    let names = app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("classes", ChoiceColumns { name: Some("Name"), label: Some("Label") }).ok()
        })
        .unwrap_or_default();
    let name = |class: u32| {
        names
            .iter()
            .find(|c| c.row == class as usize)
            .map_or_else(|| format!("({class})"), |c| c.text.clone())
    };
    let (mut ok, mut cancel) = (false, false);
    egui::Window::new("Creature Levelup Wizard").collapsible(false).resizable(false).show(ctx, |ui| {
        ui.heading("Add Classes and Levels");
        ui.label("Please choose 1 to 8 classes for this creature and select the level for each class.");
        ui.label(
            "The class determines the skills and feats the creature has as well as how the \
             creature's abilities differ from the default values.",
        );
        ui.label(
            "The higher the level in a class, the more skills and feats the creature will have \
             and the higher its ability scores will be.",
        );
        ui.horizontal_top(|ui| {
            egui::ScrollArea::vertical().max_height(220.0).id_salt("levelup-classes").show(ui, |ui| {
                ui.set_width(160.0);
                for c in &names {
                    let class = c.row as u32;
                    if w.slots.iter().any(|(k, _, _)| *k == class) {
                        continue;
                    }
                    let picked = w.chosen == Some(class);
                    let r = ui.selectable_label(picked, &c.text).on_hover_text("Select a Class to add");
                    if r.clicked() {
                        w.chosen = Some(class);
                    }
                    if r.double_clicked() && w.slots.len() < 8 {
                        w.slots.push((class, 1, 0));
                        w.chosen = None;
                    }
                }
            });
            ui.vertical(|ui| {
                let can_add = w.chosen.is_some() && w.slots.len() < 8;
                if ui
                    .add_enabled(can_add, egui::Button::new("Add Class"))
                    .on_hover_text("Add the currently selected class")
                    .clicked()
                    && let Some(class) = w.chosen.take()
                {
                    w.slots.push((class, 1, 0));
                }
                let mut remove = None;
                egui::Grid::new("levelup-slots").num_columns(3).show(ui, |ui| {
                    ui.strong("Class");
                    ui.strong("Level");
                    ui.end_row();
                    for (i, (class, level, had)) in w.slots.iter_mut().enumerate() {
                        ui.label(name(*class));
                        ui.add(egui::DragValue::new(level).range((*had).max(1)..=60))
                            .on_hover_text("Adjust the Level of this Class");
                        // Classes the creature has stay.
                        if *had == 0 && ui.small_button("✖").on_hover_text("Remove this Class").clicked() {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
                if let Some(i) = remove {
                    w.slots.remove(i);
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            ok = ui.button("OK").on_hover_text("Accept changes").clicked();
            cancel = ui.button("Cancel").on_hover_text("Discard changes").clicked();
        });
    });
    if ok {
        level_up(app, &w);
    } else if !cancel {
        app.levelup = Some(w);
    }
}

/// Levels the creature up and applies what changed.
fn level_up(app: &mut Moonglow, w: &LevelupWizard) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_ref()) else { return };
    let Ok(doc) = ws.doc(&w.key) else { return };
    let Some(creature) = w.path.get(&doc.root).cloned() else { return };
    let targets: Vec<(u32, u32)> = w.slots.iter().map(|&(c, l, _)| (c, l)).collect();
    let after = game.level_up(&creature, &targets);
    let edits: Vec<Edit> = CHANGED
        .iter()
        .filter(|l| after.get(l) != creature.get(l))
        .map(|l| Edit::SetField {
            key: w.key,
            path: w.path.clone(),
            label: (*l).into(),
            value: after.get(l).cloned(),
        })
        .collect();
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Levelup Wizard", edits)));
    }
}
