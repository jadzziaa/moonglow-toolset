//! The Creature Wizard (Wizards › Creature Wizard, Aurora's
//! `TdlgCreatureWizard`): racial type, classes and levels, gender,
//! appearance and portrait, faction, name, palette category, a review of
//! the creature, then Finish (and Launch Creature Properties). What it
//! makes is `mg_module::blueprints::creature`'s.

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit};
use mg_module::blueprints::{self, CreatureSpec};
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;
use mg_rules::{Choice, ChoiceColumns, GameData};

use crate::images::Loader;
use crate::{Action, Moonglow, Tab};

/// The wizard's pages, in order.
const PAGES: [&str; 9] = [
    "Creature Wizard",
    "Monster Type",
    "Class and Level",
    "Appearance and Portrait",
    "Faction",
    "Name",
    "Select Palette Category",
    "Review Statistics",
    "Finish",
];

/// The Creature Wizard's choices.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatureWizard {
    pub page: usize,
    pub race: Option<u32>,
    /// (class, level).
    pub classes: Vec<(u32, u32)>,
    pub chosen_class: Option<u32>,
    pub gender: u8,
    pub appearance: u16,
    pub portrait: Option<u16>,
    pub faction: u16,
    pub first_name: String,
    pub last_name: String,
    pub category: Option<u8>,
    pub launch: bool,
}

impl Default for CreatureWizard {
    fn default() -> CreatureWizard {
        CreatureWizard {
            page: 0,
            race: None,
            classes: Vec::new(),
            chosen_class: None,
            gender: 0,
            appearance: 0,
            portrait: None,
            faction: 1,
            first_name: String::new(),
            last_name: String::new(),
            category: None,
            launch: false,
        }
    }
}

fn choices(game: &GameData, table: &str, name: &str, label: &str) -> Vec<Choice> {
    game.choices(table, ChoiceColumns { name: Some(name), label: Some(label) }).unwrap_or_default()
}

/// The portraits of a race and gender (portraits.2da, not plot ones).
fn portraits(game: &GameData, race: u32, gender: u8) -> Vec<(u16, String)> {
    let Ok(t) = game.table("portraits") else { return Vec::new() };
    (0..t.len())
        .filter(|&r| {
            t.get_int(r, "Race") == Some(race as i32)
                && t.get_int(r, "Sex") == Some(i32::from(gender))
                && t.get_int(r, "Plot").unwrap_or(0) == 0
        })
        .filter_map(|r| Some((r as u16, t.get(r, "BaseResRef")?.to_string())))
        .collect()
}

impl CreatureWizard {
    /// Choosing a racial type: its appearance and default class (level 1),
    /// a male's portrait to choose again.
    fn choose_race(&mut self, game: &GameData, race: u32) {
        if self.race == Some(race) {
            return;
        }
        let t = game.table("racialtypes").ok();
        let int = |c: &str| t.as_ref().and_then(|t| t.get_int(race as usize, c));
        self.race = Some(race);
        self.appearance = int("Appearance").unwrap_or(0).max(0) as u16;
        let class = int("ToolsetDefaultClass").unwrap_or(4).max(0) as u32;
        self.classes = vec![(class, 1)];
        self.portrait = None;
    }

    /// Whether Next may go on from the page.
    fn complete(&self) -> bool {
        match self.page {
            1 => self.race.is_some(),
            2 => !self.classes.is_empty(),
            3 => self.portrait.is_some(),
            5 => !self.first_name.trim().is_empty(),
            6 => self.category.is_some(),
            _ => true,
        }
    }

    fn spec(&self, resref: ResRef) -> CreatureSpec {
        CreatureSpec {
            resref,
            first_name: self.first_name.trim().to_string(),
            last_name: self.last_name.trim().to_string(),
            race: self.race.unwrap_or(6),
            gender: self.gender,
            appearance: self.appearance,
            portrait: self.portrait.unwrap_or(0),
            faction: self.faction,
            classes: self.classes.clone(),
            category: self.category.unwrap_or(0),
        }
    }
}

/// Reads an item blueprint: the module's, else the game's.
fn item_reader<'a>(app: &'a Moonglow) -> impl Fn(ResRef) -> Option<mg_gff::Struct> + 'a {
    move |r: ResRef| {
        let k = ResKey::new(r, ResType::UTI);
        let data = app
            .ws
            .as_ref()
            .and_then(|w| w.module.get(&k).map(<[u8]>::to_vec))
            .or_else(|| app.game.as_deref()?.resman.get(&k).ok().map(|d| d.into_owned()))?;
        mg_gff::Gff::read(&data).ok().map(|g| g.root)
    }
}

/// The review: what the creature comes out as.
fn review(app: &Moonglow, w: &CreatureWizard) -> String {
    let Some(game) = app.game.as_deref() else { return String::new() };
    let item = item_reader(app);
    let g = blueprints::creature(game, &w.spec(ResRef::EMPTY), &item);
    let c = &g.root;
    let name_of = |table: &str, row: i64| {
        choices(game, table, if table == "appearance" { "STRING_REF" } else { "Name" }, "Label")
            .into_iter()
            .find(|x| x.row as i64 == row)
            .map(|x| x.text)
            .unwrap_or_default()
    };
    let int = |l: &str| c.integer(l).unwrap_or(0);
    let rating = c.float("ChallengeRating").unwrap_or(0.0);
    let cr = mg_rules::Challenge { calculated: rating, rating }.text();
    let stats = game.creature_stats(&mg_rules::CreatureSheet::from_gff(c));
    let mut out = format!(
        "{} {}\n\nMonster Type: {}\nGender: {}\nAlignment: True Neutral\nAppearance: {}\n\n\
         Challenge Rating: {cr}\n\n",
        w.first_name.trim(),
        w.last_name.trim(),
        name_of("racialtypes", int("Race")),
        if w.gender == 1 { "Female" } else { "Male" },
        name_of("appearance", int("Appearance_Type")),
    );
    for (label, a) in [
        ("Strength", "Str"),
        ("Dexterity", "Dex"),
        ("Constitution", "Con"),
        ("Intelligence", "Int"),
        ("Wisdom", "Wis"),
        ("Charisma", "Cha"),
    ] {
        out.push_str(&format!("{label}: {}\n", int(a)));
    }
    out.push_str(&format!(
        "\nHit Points: {}\nHit Dice: {}\n\nAC: {}\n",
        int("MaxHitPoints"),
        stats.level,
        int("NaturalAC")
    ));
    out
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.creature_wizard.take() else { return };
    if app.game.is_none() {
        app.log.error("The Creature Wizard needs the game data");
        return;
    }
    let (mut finish, mut cancel) = (false, false);
    let summary = if w.page == 7 { review(app, &w) } else { String::new() };
    // The game data, and its pictures (the portraits).
    let mut loader = app.loader().expect("checked");
    // Most of the screen to begin with, and resizable: the pages' lists and
    // the review fill what is between the heading and the buttons.
    let screen = ctx.content_rect();
    let size = egui::vec2(
        (screen.width() * 0.6).clamp(560.0, 960.0),
        (screen.height() * 0.75).clamp(440.0, 800.0),
    );
    egui::Window::new("Creature Wizard")
        .collapsible(false)
        .resizable(true)
        .default_size(size)
        .min_width(480.0)
        .min_height(360.0)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(screen.center())
        .show(ctx, |ui| {
            egui::Panel::bottom("cw-buttons").show(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(w.page > 0, egui::Button::new("< Back")).clicked() {
                        w.page -= 1;
                    }
                    let last = w.page + 1 == PAGES.len();
                    if ui.add_enabled(!last && w.complete(), egui::Button::new("Next >")).clicked()
                        || (!last && w.complete() && crate::widgets::enter(ui))
                    {
                        w.page += 1;
                    }
                    finish = ui.add_enabled(last, egui::Button::new("Finish")).clicked()
                        || (last && crate::widgets::enter(ui));
                    cancel = ui.button("Cancel").clicked();
                });
            });
            ui.heading(PAGES[w.page]);
            page(ui, &mut loader, &mut w, &summary);
        });
    if finish {
        make(app, &w);
    } else if !cancel {
        app.creature_wizard = Some(w);
    }
}

fn page(ui: &mut Ui, loader: &mut Loader<'_>, w: &mut CreatureWizard, summary: &str) {
    let game = loader.game;
    match w.page {
        0 => {
            ui.label("Welcome to the Creature Wizard.");
            ui.label(
                "This wizard allows you to quickly and efficiently create new Creature Blueprints.",
            );
            ui.label("Click Next to continue.");
        }
        1 => {
            ui.label("Choose a Racial Type for your new Creature Blueprint.");
            ui.label(
                "The Racial Type determines the default abilities, class, and appearance of the \
                 Creature.",
            );
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                for c in choices(game, "racialtypes", "Name", "Label") {
                    if ui.selectable_label(w.race == Some(c.row as u32), &c.text).clicked() {
                        w.choose_race(game, c.row as u32);
                    }
                }
            });
        }
        2 => {
            ui.label("Please choose 1 to 8 classes for this creature and select the level for each class.");
            let names = choices(game, "classes", "Name", "Label");
            let name = |k: u32| {
                names.iter().find(|c| c.row == k as usize).map_or(String::new(), |c| c.text.clone())
            };
            ui.horizontal_top(|ui| {
                // A list down the left (a scroll area takes the layout of
                // the row it is in: classes side by side, drawn over each
                // other).
                ui.vertical(|ui| {
                    ui.set_width(160.0);
                    egui::ScrollArea::vertical().id_salt("cw-classes").show(ui, |ui| {
                        ui.set_width(160.0);
                        for c in &names {
                            let k = c.row as u32;
                            if w.classes.iter().any(|(x, _)| *x == k) {
                                continue;
                            }
                            let r = ui.selectable_label(w.chosen_class == Some(k), &c.text);
                            if r.clicked() {
                                w.chosen_class = Some(k);
                            }
                            if r.double_clicked() && w.classes.len() < 8 {
                                w.classes.push((k, 1));
                            }
                        }
                    });
                });
                ui.vertical(|ui| {
                    if ui
                        .add_enabled(
                            w.chosen_class.is_some() && w.classes.len() < 8,
                            egui::Button::new("Add Class"),
                        )
                        .clicked()
                        && let Some(k) = w.chosen_class.take()
                    {
                        w.classes.push((k, 1));
                    }
                    let mut remove = None;
                    egui::Grid::new("cw-slots").num_columns(3).show(ui, |ui| {
                        ui.strong("Class");
                        ui.strong("Level");
                        ui.end_row();
                        let n = w.classes.len();
                        for (i, (k, level)) in w.classes.iter_mut().enumerate() {
                            ui.label(name(*k));
                            ui.add(egui::DragValue::new(level).range(1..=60));
                            if n > 1
                                && ui.small_button("✖").on_hover_text("Remove this Class").clicked()
                            {
                                remove = Some(i);
                            }
                            ui.end_row();
                        }
                    });
                    if let Some(i) = remove {
                        w.classes.remove(i);
                    }
                });
            });
        }
        3 => {
            ui.label("Please choose an Appearance and Portrait for this creature.");
            egui::Grid::new("cw-look").num_columns(2).show(ui, |ui| {
                crate::widgets::field_label(ui, "Gender");
                let before = w.gender;
                egui::ComboBox::from_id_salt("cw-gender")
                    .selected_text(if w.gender == 1 { "Female" } else { "Male" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut w.gender, 0, "Male");
                        ui.selectable_value(&mut w.gender, 1, "Female");
                    });
                if w.gender != before {
                    w.portrait = None;
                }
                ui.end_row();
                crate::widgets::field_label(ui, "Appearance");
                let appearances = choices(game, "appearance", "STRING_REF", "LABEL");
                let shown = appearances
                    .iter()
                    .find(|c| c.row == usize::from(w.appearance))
                    .map_or(String::new(), |c| c.text.clone());
                egui::ComboBox::from_id_salt("cw-appearance")
                    .selected_text(shown)
                    .height(300.0)
                    .show_ui(ui, |ui| {
                        for c in &appearances {
                            ui.selectable_value(&mut w.appearance, c.row as u16, &c.text);
                        }
                    });
                ui.end_row();
            });
            crate::widgets::field_label(ui, "Portrait");
            // The race's and gender's portraits as pictures, the chosen one
            // large beside them (as Select Portrait shows them).
            let list = portraits(game, w.race.unwrap_or(6), w.gender);
            // The first chosen until another is, so the page opens with one.
            if w.portrait.is_none_or(|p| !list.iter().any(|(r, _)| *r == p)) {
                w.portrait = list.first().map(|(r, _)| *r);
            }
            // As many to a row as the window's width holds, beside the
            // chosen one's large picture.
            let preview = 140.0;
            let width = (ui.available_width() - preview - 2.0 * ui.spacing().item_spacing.x)
                .max(crate::images::PORTRAIT_THUMB + 40.0);
            let (per_row, row_height) = crate::images::portrait_grid(ui, width);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    egui::ScrollArea::vertical()
                        .id_salt(("cw-portraits", w.race, w.gender))
                        .auto_shrink([false, false])
                        .show_rows(ui, row_height, list.len().div_ceil(per_row), |ui, lines| {
                            for line in lines {
                                ui.horizontal(|ui| {
                                    for (row, base) in
                                        list.iter().skip(line * per_row).take(per_row)
                                    {
                                        let base = base.to_lowercase();
                                        let r = loader
                                            .portrait(
                                                ui,
                                                &base,
                                                'm',
                                                crate::images::PORTRAIT_THUMB,
                                                egui::Sense::click(),
                                            )
                                            .on_hover_text(&base);
                                        if w.portrait == Some(*row) {
                                            ui.painter().rect_stroke(
                                                r.rect.expand(1.0),
                                                2.0,
                                                ui.visuals().selection.stroke,
                                                egui::StrokeKind::Outside,
                                            );
                                        }
                                        if r.clicked() {
                                            w.portrait = Some(*row);
                                        }
                                    }
                                });
                            }
                        });
                });
                ui.vertical(|ui| {
                    ui.set_width(preview);
                    let chosen = w.portrait.and_then(|p| list.iter().find(|(r, _)| *r == p));
                    if let Some((_, base)) = chosen {
                        let base = base.to_lowercase();
                        let large = loader.picture(ui.ctx(), &format!("po_{base}l")).is_some();
                        let size = if large { 'l' } else { 'm' };
                        loader.portrait(ui, &base, size, 128.0, egui::Sense::hover());
                        ui.label(base);
                    }
                });
            });
        }
        4 => {
            ui.label("Choose a Faction for this creature.");
            ui.label(
                "The Faction determines how this creature reacts to players and other creatures.",
            );
            for (i, name) in mg_module::factions::STANDARD.iter().enumerate().skip(1) {
                if ui.selectable_label(usize::from(w.faction) == i, *name).clicked() {
                    w.faction = i as u16;
                }
            }
        }
        5 => {
            // A random name to begin with, as Aurora gives (from the race's
            // letter tables; none for races without).
            let race = w.race.unwrap_or(6);
            let mut rng = fastrand::Rng::new();
            if w.first_name.is_empty() && w.last_name.is_empty() {
                w.first_name =
                    game.random_name(race, w.gender, false, &mut rng).unwrap_or_default();
                w.last_name = game.random_name(race, w.gender, true, &mut rng).unwrap_or_default();
            }
            ui.label("Please enter a name for this new Creature.");
            egui::Grid::new("cw-name").num_columns(2).show(ui, |ui| {
                crate::widgets::field_label(ui, "Name");
                ui.end_row();
                let field = ui.text_edit_singleline(&mut w.first_name);
                crate::widgets::autofocus(ui, &field);
                if ui.button("Random").clicked()
                    && let Some(n) = game.random_name(race, w.gender, false, &mut rng)
                {
                    w.first_name = n;
                }
                ui.end_row();
                crate::widgets::field_label(ui, "Last Name:");
                ui.end_row();
                ui.text_edit_singleline(&mut w.last_name);
                if ui.button("Random").clicked()
                    && let Some(n) = game.random_name(race, w.gender, true, &mut rng)
                {
                    w.last_name = n;
                }
                ui.end_row();
            });
        }
        6 => {
            ui.label("Please select the category in the palette that you wish this blueprint to appear under.");
            let nodes = crate::blueprint_wizard::categories(game, BlueprintKind::Creature);
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                crate::blueprint_wizard::category_tree(ui, &nodes, game, &mut w.category);
            });
        }
        7 => {
            ui.label(
                "Check that the following statistics are correct. Click Back to make any changes.",
            );
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                ui.label(summary);
            });
        }
        _ => {
            ui.label("This creature is now complete.");
            ui.label("If you wish, you can fine-tune it using the Creature Properties.");
            ui.checkbox(&mut w.launch, "Launch Creature Properties");
        }
    }
}

/// Finish: the blueprint, as one undoable command.
fn make(app: &mut Moonglow, w: &CreatureWizard) {
    let (key, bytes) = {
        let Some(game) = app.game.as_deref() else { return };
        let taken = |r: &ResRef| {
            app.ws.as_ref().is_some_and(|ws| ws.module.contains(&ResKey::new(*r, ResType::UTC)))
        };
        let resref = blueprints::resref(w.first_name.trim(), taken);
        let spec = w.spec(resref);
        let item = item_reader(app);
        let mut g = blueprints::creature(game, &spec, &item);
        g.root.set("Tag", mg_gff::Value::String(blueprints::tag(&spec.first_name).into_bytes()));
        (ResKey::new(resref, ResType::UTC), g.to_bytes())
    };
    match bytes {
        Ok(data) => {
            app.actions.push(Action::Apply(Command::new(
                "New creature",
                vec![Edit::SetResource { key, data: Some(data) }],
            )));
            if w.launch {
                app.actions.push(Action::OpenTab(Tab::Blueprint(key)));
            }
        }
        Err(e) => app.log.error(e.to_string()),
    }
}
