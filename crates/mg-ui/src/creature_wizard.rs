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
use mg_rules::{Choice, GameData};
use mg_uikit::Controls as _;

use crate::images::Loader;
use crate::widgets::ResRefText;
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
    /// The racial type and gender chosen have no portraits to choose from
    /// (a custom race without any): the wizard goes on without one.
    pub no_portraits: bool,
    pub faction: u16,
    pub first_name: String,
    pub last_name: String,
    pub category: Option<u8>,
    pub launch: bool,
    /// What narrows the Appearance page's pictures.
    pub appearance_find: String,
    /// The Appearance page offers every creature's portrait, not the
    /// race's and gender's alone.
    pub all_portraits: bool,
    /// A ResRef and a Tag of the builder's own; empty, they are made from
    /// the name as Aurora's wizard makes them.
    pub resref: String,
    pub tag: String,
    /// Whether the default random name for the chosen race was already seeded.
    pub name_seeded: bool,
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
            no_portraits: false,
            faction: 1,
            first_name: String::new(),
            last_name: String::new(),
            category: None,
            launch: false,
            appearance_find: String::new(),
            all_portraits: false,
            resref: String::new(),
            tag: String::new(),
            name_seeded: false,
        }
    }
}

/// The portraits of a race and gender (portraits.2da, not plot ones), as
/// Aurora's wizard lists them (its Select Portrait with Race and Gender
/// ticked, captured 2026-10-30 in `w1probe`): the gender's exactly (no
/// rows of `Sex` 4, none: a human male's list has no `kid_` of row 238),
/// in the table's order, the module's own portraits among them. A race
/// with none of the gender, as a monster's (an outsider's are all `Sex`
/// 4), has those of no gender (`Sex` 2 both, 3 other, 4 none), where
/// Aurora's list is empty (a senior builder's review: the wizard is to
/// offer a monster's portraits).
fn portraits(
    game: &GameData,
    ws: Option<&mg_edit::Workspace>,
    race: u32,
    gender: u8,
    all: bool,
) -> Vec<(u16, String)> {
    let Ok(t) = game.table("portraits") else { return Vec::new() };
    let pictured = crate::images::Portraits::of(game, ws);
    let rows = |fits: &dyn Fn(Option<i32>) -> bool| -> Vec<(u16, String)> {
        (0..t.len())
            .filter(|&r| {
                // Every creature's (not a door's or placeable's), for a
                // creature that isn't to look like its race. (`door01_` is
                // listed under the fey's race.)
                t.get_int(r, "InanimateType").is_none()
                    && (all
                        || t.get_int(r, "Race") == Some(race as i32)
                            && fits(t.get_int(r, "Sex"))
                            && t.get_int(r, "Plot").unwrap_or(0) == 0)
            })
            .filter_map(|r| Some((r as u16, t.get(r, "BaseResRef")?.to_string())))
            .filter(|(_, base)| pictured.has(base))
            .collect()
    };
    let of_gender = rows(&|sex| sex == Some(i32::from(gender)));
    if of_gender.is_empty() && !all {
        return rows(&|sex| sex.is_some_and(|s| s >= 2));
    }
    of_gender
}

/// [`portraits`] for the wizard's choices, kept while the table, the
/// module (its revision: a portrait of its own) and the choices are the
/// same: every row of portraits.2da was looked up as a picture each frame
/// the page was shown.
fn portraits_kept(
    ctx: &egui::Context,
    game: &GameData,
    ws: Option<&mg_edit::Workspace>,
    w: &CreatureWizard,
) -> std::sync::Arc<Vec<(u16, String)>> {
    let Ok(table) = game.table("portraits") else { return Default::default() };
    let (race, gender, all) = (w.race.unwrap_or(6), w.gender, w.all_portraits);
    let key = (ws.map(mg_edit::Workspace::revision), race, gender, all);
    let id = egui::Id::new("creature-wizard-portraits");
    crate::blueprint::kept_for(ctx, id, &table, &key, || {
        std::sync::Arc::new(portraits(game, ws, race, gender, all))
    })
}

/// The racial types a creature can be given: racialtypes.2da's rows that
/// have a name (its `DELETED` and `INVALID_RACE` rows have none).
fn races(game: &GameData) -> std::sync::Arc<Vec<Choice>> {
    crate::widgets::table_choices(Some(game), "racialtypes", Some("Name"), None)
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
        self.name_seeded = false;
    }

    /// Seeds a default random name once per chosen race when both name
    /// fields are empty, without refilling after the user clears First Name.
    fn seed_name_once(&mut self, game: &GameData, race: u32, rng: &mut fastrand::Rng) {
        if self.name_seeded {
            return;
        }
        self.name_seeded = true;
        if self.first_name.is_empty() && self.last_name.is_empty() {
            self.first_name = game.random_name(race, self.gender, false, rng).unwrap_or_default();
            self.last_name = game.random_name(race, self.gender, true, rng).unwrap_or_default();
        }
    }

    /// Whether Next may go on from the page.
    fn complete(&self) -> bool {
        match self.page {
            1 => self.race.is_some(),
            2 => !self.classes.is_empty(),
            3 => self.portrait.is_some() || self.no_portraits,
            5 => {
                let own = self.resref.trim();
                !self.first_name.trim().is_empty()
                    && (own.is_empty() || ResRef::from_str(own).is_ok())
            }
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
        let module = app.ws.as_ref().map(|w| &w.module);
        mg_module::gff_root(module, app.game.as_deref()?, &ResKey::new(r, ResType::UTI))
    }
}

/// The review: what the creature comes out as.
fn review(app: &Moonglow, w: &CreatureWizard) -> String {
    let Some(game) = app.game.as_deref() else { return String::new() };
    let item = item_reader(app);
    let g = blueprints::creature(game, &w.spec(ResRef::EMPTY), &item);
    let c = &g.root;
    let name_of = |table: &str, row: i64| {
        let name = if table == "appearance" { "STRING_REF" } else { "Name" };
        let names = crate::widgets::table_choices(Some(game), table, Some(name), Some("Label"));
        names.iter().find(|x| x.row as i64 == row).map(|x| x.text.clone()).unwrap_or_default()
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
    let load_order = app.load_order;
    let (mut finish, mut cancel) = (false, false);
    let summary = if w.page == 7 { review(app, &w) } else { String::new() };
    // The ResRef the name gives, as Finish makes it (on the names' page).
    let names = Names {
        naming: crate::blueprint_wizard::naming(app, ResType::UTC),
        taken: match (w.page == NAMES_PAGE, app.ws.as_ref()) {
            (true, Some(ws)) => {
                let ws_key = (ws.identity(), ws.revision());
                crate::blueprint::kept_for_key(
                    ctx,
                    egui::Id::new("creature-wizard-taken"),
                    &ws_key,
                    || {
                        std::sync::Arc::new(
                            ws.module.keys_of(ResType::UTC).map(|k| k.resref).collect(),
                        )
                    },
                )
            }
            _ => Default::default(),
        },
    };
    // The game data, and its pictures (the portraits).
    let (mut loader, ready) = app.loader_and_thumbnails().expect("checked");
    let mut looks = Looks { ready, shown: Vec::new() };
    // Most of the screen to begin with, and resizable: the pages' lists and
    // the review fill what is between the heading and the buttons.
    let screen = ctx.content_rect();
    let size = egui::vec2(
        (screen.width() * 0.6).clamp(560.0, 960.0),
        (screen.height() * 0.75).clamp(440.0, 800.0),
    );
    let last = w.page + 1 == PAGES.len();
    let footed = crate::widgets::modal(ctx, "Creature Wizard")
        .resizable(true)
        .default_size(size)
        .min_width(480.0)
        .min_height(360.0)
        .flush()
        .show_foot(ctx, |ui| {
            // The step list at the left (the Dialogs board's), the page
            // under its bar beside it.
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if crate::widgets::step_list_fits(ui) {
                    steps(ui, &w);
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    mg_uikit::dialog::page_bar(ui, Some(PAGE_ICONS[w.page]), PAGES[w.page]);
                    let pad = mg_uikit::dialog::PAGE_PADDING;
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(pad.x as i8, pad.y as i8))
                        .show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            page(ui, &mut loader, load_order, &mut w, &summary, &mut looks, &names);
                        });
                });
            });
            ((), wizard_foot(w.page > 0, !last && w.complete(), last))
        });
    if let Some(f) = footed {
        if f.pressed.clicked("< Back") {
            w.page -= 1;
        } else if f.pressed.main() && !last {
            w.page += 1;
        }
        finish = last && f.pressed.main();
        cancel = f.pressed.cancelled();
    }
    // The appearances' pictures in sight: kept while they are, and those
    // still to make are made a few a frame.
    let shown = looks.shown;
    app.thumbnails.show(shown.into_iter().map(crate::model_view::Pictured::Look));
    if finish {
        make(app, &w);
    } else if !cancel {
        app.creature_wizard = Some(w);
    }
}

/// The icon of each page's bar (the Dialogs and Pickers boards draw Class
/// and Level's and Appearance and Portrait's).
const PAGE_ICONS: [mg_uikit::Icon; 9] = [
    mg_uikit::Icon::Wizard,
    mg_uikit::Icon::Creature,
    mg_uikit::Icon::Class,
    mg_uikit::Icon::Appearance,
    mg_uikit::Icon::Faction,
    mg_uikit::Icon::Tag,
    mg_uikit::Icon::Palette,
    mg_uikit::Icon::Statistics,
    mg_uikit::Icon::CheckCircle,
];

/// The wizard's step list (the Dialogs board's `.steps`): the pages before
/// the one shown done, it current, those after to come.
fn steps(ui: &mut Ui, w: &CreatureWizard) {
    use mg_uikit::dialog::{StepList, StepState};
    let mut list = StepList::new(format!("Step {} of {}", w.page + 1, PAGES.len()));
    for (i, name) in PAGES.iter().enumerate() {
        let state = match i.cmp(&w.page) {
            std::cmp::Ordering::Less => StepState::Done,
            std::cmp::Ordering::Equal => StepState::Current,
            std::cmp::Ordering::Greater => StepState::ToCome,
        };
        list = list.step(*name, state);
    }
    let _ = list.show(ui);
}

/// A wizard's footer (the Dialogs board's page bar of buttons): < Back
/// (where `back`), Next > (where `next`) in the accent, Finish and Cancel;
/// on the last page (`last`) Finish is the main action, Enter's, and Next
/// stays off in its place.
pub(crate) fn wizard_foot(back: bool, next: bool, last: bool) -> crate::widgets::Foot {
    let foot = crate::widgets::Foot::new().button("< Back", back);
    let foot = match last {
        false => foot.main("Next >", next).button("Finish", false),
        true => foot.button("Next >", false).main("Finish", true),
    };
    foot.cancel()
}

/// The pictures of creatures' looks the Appearance page shows: those made,
/// and those in sight (made or not).
struct Looks<'a> {
    ready: &'a crate::model_view::Thumbnails,
    shown: Vec<mg_preview::CreatureLook>,
}

/// The page of the creature's names (and its ResRef and tag).
const NAMES_PAGE: usize = 5;

/// What a ResRef made from the name is made by: the project's naming, and
/// the module's creatures' ResRefs.
struct Names {
    naming: mg_module::project_file::Naming,
    taken: std::sync::Arc<std::collections::HashSet<ResRef>>,
}

/// The most classes a creature is given.
const MOST_CLASSES: usize = 8;

/// The Class and Level page under its words (the Dialogs board's Creature
/// Wizard): the classes not chosen in a well at the left (a click chooses
/// one, a double-click adds it); at the right Add Class across the column,
/// the classes added under small Class and Level headings, a row each (its
/// name, its level and a × that takes it out, where it is not the only
/// one), and how many of eight there are. The columns are the board's 280
/// and 330 where the page has the room, shares of it where it is narrower
/// (the least window), each row's name cut to its column.
fn class_page(ui: &mut Ui, w: &mut CreatureWizard, names: &[Choice], name: &dyn Fn(u32) -> String) {
    let gap = 12.0;
    let room = ui.available_size();
    let right = (room.x * 0.54).clamp(150.0, 330.0);
    let list = (room.x - gap - right).clamp(80.0, 280.0);
    let row = mg_uikit::row_pitch(ui);
    let high = room.y.max(4.0 * row + 4.0);
    // (A cell `width` wide holding what `add` puts in, at its left.)
    fn cell<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
        let size = egui::vec2(width, ui.spacing().interact_size.y);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(size, layout, |ui| {
            ui.set_min_size(size);
            add(ui)
        })
        .inner
    }
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        let well = egui::vec2(list, high);
        let id = egui::Id::new("cw-classes");
        crate::widgets::picture_well(ui, id, well, egui::Margin::ZERO, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                mg_uikit::list_rows(ui);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                for c in names {
                    let k = c.row as u32;
                    if w.classes.iter().any(|(x, _)| *x == k) {
                        continue;
                    }
                    let r = ui.selectable_label(w.chosen_class == Some(k), &c.text);
                    if r.clicked() {
                        w.chosen_class = Some(k);
                    }
                    if r.double_clicked() && w.classes.len() < MOST_CLASSES {
                        w.classes.push((k, 1));
                    }
                }
            });
        });
        ui.vertical(|ui| {
            ui.set_width(right);
            ui.spacing_mut().item_spacing.y = 8.0;
            let can = w.chosen_class.is_some() && w.classes.len() < MOST_CLASSES;
            let add =
                mg_uikit::KitButton::new(Some(mg_uikit::Icon::Plus), "Add Class").at_least(right);
            if ui.add_enabled(can, add).clicked()
                && let Some(k) = w.chosen_class.take()
            {
                w.classes.push((k, 1));
            }
            // (The board's `.slots`: the name, a 72-point level and a
            // 22-point ×, 10 apart.)
            let (field, close, apart) = (72.0, 22.0, 10.0);
            let named = (right - field - close - 2.0 * apart).max(40.0);
            let t = mg_uikit::tokens(ui.ctx());
            let small = |s: &str| {
                egui::RichText::new(s).size(mg_uikit::metrics::SMALL_SIZE).color(t.text_2)
            };
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = apart;
                let size = egui::vec2(named, mg_uikit::metrics::SMALL_SIZE + 4.0);
                ui.allocate_ui_with_layout(
                    size,
                    egui::Layout::left_to_right(egui::Align::Max),
                    |ui| {
                        ui.set_min_size(size);
                        ui.label(small("Class"));
                    },
                );
                ui.label(small("Level"));
            });
            // (The rows scroll where eight don't fit: the count stays under
            // them.)
            let count = ui.text_style_height(&egui::TextStyle::Body) + 8.0;
            let rows = (ui.available_height() - count).max(row);
            let mut remove = None;
            egui::ScrollArea::vertical()
                .id_salt("cw-slots")
                .max_height(rows)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let n = w.classes.len();
                    for (i, (k, level)) in w.classes.iter_mut().enumerate() {
                        let class = name(*k);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = apart;
                            cell(ui, named, |ui| ui.add(egui::Label::new(&class).truncate()));
                            ui.scope(|ui| {
                                let arrows = crate::widgets::SPIN_ARROW_WIDTH + 3.0;
                                ui.spacing_mut().interact_size.x = field - arrows;
                                crate::widgets::name_next_number(ui, &class);
                                crate::widgets::spin_draft(ui, level, 1..=60, 1.0, |d| d);
                            });
                            if n > 1
                                && crate::widgets::icon_button(
                                    ui,
                                    mg_uikit::Icon::Close,
                                    &format!("Remove {class}"),
                                    "Remove this class",
                                )
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                    }
                });
            if let Some(i) = remove {
                w.classes.remove(i);
            }
            ui.weak(format!("{} of {MOST_CLASSES} classes", w.classes.len()));
        });
    });
}

/// The height the Appearance and Portrait page takes with its wells at
/// their least: shorter, the page scrolls.
const LOOK_ROOM: f32 = 380.0;

/// The Appearance and Portrait page: `short`, in a scroll (the least
/// window), its wells their least height, not shares of the room.
fn look_page(
    ui: &mut Ui,
    loader: &mut Loader<'_>,
    w: &mut CreatureWizard,
    looks: &mut Looks,
    short: bool,
) {
    let game = loader.game;
    ui.label("Please choose an Appearance and Portrait for this creature.");
    egui::Grid::new("cw-look").num_columns(2).show(ui, |ui| {
        crate::widgets::field_label(ui, "Gender");
        let before = w.gender;
        let name = |g: &u8| if *g == 1 { "Female" } else { "Male" };
        crate::widgets::dropdown("cw-gender").selected_text(name(&w.gender)).show(
            ui,
            &mut w.gender,
            &[0, 1],
            name,
        );
        if w.gender != before {
            w.portrait = None;
        }
        ui.end_row();
    });
    // The appearances as pictures (a plain body of each), narrowed by
    // Find: the list of names alone is long, and many are alike.
    let appearances =
        crate::widgets::table_choices(Some(game), "appearance", Some("STRING_REF"), Some("LABEL"));
    let appearances = crate::blueprint::by_name_kept(ui.ctx(), "creature-wizard", &appearances);
    let chosen = appearances
        .iter()
        .find(|c| c.row == usize::from(w.appearance))
        .map_or(String::new(), |c| c.text.clone());
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Appearance");
        // (The name cut where the row is short of room, the field kept
        // in the window: the least window.)
        let find = 160.0f32.min(ui.available_width() * 0.5);
        let named = (ui.available_width() - find - ui.spacing().item_spacing.x).max(0.0);
        let size = egui::vec2(named, ui.spacing().interact_size.y);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(size, layout, |ui| {
            ui.add(egui::Label::new(chosen).truncate());
        });
        crate::widgets::SearchField::new(
            egui::Id::new("creature-wizard-appearance-find"),
            &mut w.appearance_find,
        )
        .hint("Find an appearance")
        .width(find)
        .find_over()
        .show(ui);
    });
    let found =
        crate::blueprint::found_kept(ui.ctx(), "creature-wizard", &appearances, &w.appearance_find);
    // (Pictures from 96 points, grown to share the width.)
    let side = 96.0;
    let tall = side + mg_uikit::gallery::label_height(false, 2, false);
    let gender = w.gender;
    // (Where the one chosen is among those found, once for them.)
    let current = crate::blueprint::kept_for(
        ui.ctx(),
        egui::Id::new("cw-appearance-at"),
        &found,
        &w.appearance,
        || found.iter().position(|&idx| usize::from(w.appearance) == appearances[idx].row),
    );
    // (In a well with its drawn scroll bar: the Pickers board's,
    // half the page's room left.)
    let well = egui::vec2(
        ui.available_width(),
        match short {
            true => tall + 8.0 + 2.0,
            false => (ui.available_height() * 0.5).max(tall + 8.0 + 2.0),
        },
    );
    let pad = egui::Margin { left: 8, right: 8, top: 4, bottom: 4 };
    let shown_area =
        crate::widgets::picture_well(ui, egui::Id::new("cw-appearances"), well, pad, |ui| {
            let id = egui::Id::new("cw-appearance-pictures");
            let keys = !egui::Popup::is_any_open(ui.ctx());
            let grid = mg_uikit::gallery::GalleryGrid::new(id, found.len(), side)
                .cursor(current)
                .keys_allowed(keys)
                .name("Appearances");
            let look_of = |c: &Choice| {
                let mut look = mg_preview::CreatureLook::new(c.row as u16);
                look.gender = gender;
                look
            };
            let out = grid.show(ui, |ui, i, rect, tile, r, cell| {
                let c = &appearances[found[i]];
                let made = looks.ready.made(crate::model_view::Pictured::Look(look_of(c)));
                let on = usize::from(w.appearance) == c.row;
                let picture = crate::widgets::cell_picture(made);
                mg_uikit::gallery::GalleryCell::new(&c.text, picture)
                    .kind(mg_uikit::Icon::Creature)
                    .selected(on)
                    .paint(ui, rect, tile, cell, r);
                if r.clone().on_hover_text(format!("{} (row {})", c.text, c.row)).clicked() {
                    w.appearance = c.row as u16;
                }
            });
            // (The keys choose as they move: a picker's.)
            if let Some(i) = out.moved.or(out.activated) {
                w.appearance = appearances[found[i]].row as u16;
            }
            looks.shown.extend(out.ahead.clone().map(|i| look_of(&appearances[found[i]])));
            out
        });
    // (The well's room taken: the page's rows go on under it.)
    // (Opened at the appearance chosen, once each time the page is
    // shown: a pass missed is the page left.)
    let placed_id = egui::Id::new("cw-appearances-placed");
    let pass = ui.ctx().cumulative_pass_nr();
    let (last, placed) = ui.data(|d| d.get_temp::<(u64, bool)>(placed_id)).unwrap_or((0, false));
    let fresh = last + 1 < pass;
    // (The page just shown: its appearances' grid given the keys, so
    // the one chosen is ringed in the accent as on the board, and the
    // arrows move it, as Select Portrait's.)
    if fresh {
        ui.memory_mut(|m| m.request_focus(egui::Id::new("cw-appearance-pictures")));
    }
    let placed = placed && !fresh;
    let placed = placed
        || match current {
            Some(at) => {
                crate::widgets::open_gallery_at(ui.ctx(), &shown_area, &shown_area.inner, at)
            }
            None => true,
        };
    ui.data_mut(|d| d.insert_temp(placed_id, (pass, placed)));
    ui.separator();
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Portrait");
        if ui
            .check_box(&mut w.all_portraits, "All portraits")
            .on_hover_text("Every creature's portrait, not this race's and gender's alone")
            .changed()
        {
            w.portrait = None;
        }
    });
    {
        // The race's and gender's portraits as pictures, the chosen one
        // large beside them (as Select Portrait shows them).
        let list = portraits_kept(ui.ctx(), game, loader.ws, w);
        w.no_portraits = list.is_empty();
        if list.is_empty() {
            ui.weak(
                "This racial type has no portraits: choose one in Creature Properties \
             (Portraits…) afterwards.",
            );
        }
        // The first chosen until another is, so the page opens with one.
        if w.portrait.is_none_or(|p| !list.iter().any(|(r, _)| *r == p)) {
            w.portrait = list.first().map(|(r, _)| *r);
        }
        // As many to a row as the window's width holds, beside the
        // chosen one's large picture.
        let preview = 140.0;
        let width = (ui.available_width() - preview - 2.0 * ui.spacing().item_spacing.x)
            .max(crate::images::PORTRAIT_THUMB + 40.0);
        ui.horizontal_top(|ui| {
            // (In a well with its drawn scroll bar, justified rows of
            // 54 by 84 that reach it, as Select Portrait's.)
            let id = egui::Id::new(("cw-portraits", w.race, w.gender, w.all_portraits));
            let at = crate::blueprint::kept_for(
                ui.ctx(),
                egui::Id::new("cw-portrait-at"),
                &list,
                &w.portrait,
                || list.iter().position(|(r, _)| Some(*r) == w.portrait),
            );
            let keys = !egui::Popup::is_any_open(ui.ctx());
            let (tile, gaps) = (crate::images::PORTRAIT_TILE, crate::images::PORTRAIT_GAPS);
            let grid = mg_uikit::gallery::GalleryGrid::new(id.with("grid"), list.len(), tile.x)
                .justified(tile, gaps.0)
                .row_gap(gaps.1)
                .label(0.0)
                .cursor(at)
                .keys_allowed(keys)
                .name("Portraits");
            let high = match short {
                true => tile.y + 12.0,
                false => ui.available_height().max(tile.y + 12.0),
            };
            let size = egui::vec2(width, high);
            let pad = egui::Margin { left: 6, right: 6, top: 4, bottom: 4 };
            let shown = crate::widgets::picture_well(ui, id, size, pad, |ui| {
                let out = grid.show(ui, |ui, i, rect, tile, r, cell| {
                    let (row, base) = &list[i];
                    let base = base.to_lowercase();
                    let picture = loader.portrait_cell(ui.ctx(), &base);
                    mg_uikit::gallery::GalleryCell::new("", picture)
                        .said(&format!("po_{base}"))
                        .lines(0)
                        .radius(3)
                        .kind(mg_uikit::Icon::Creature)
                        .selected(w.portrait == Some(*row))
                        .paint(ui, rect, tile, cell, r);
                    if r.clone().on_hover_text(&base).clicked() {
                        w.portrait = Some(*row);
                    }
                });
                for i in out.drawn.end..out.ahead.end {
                    loader.portrait_cell(ui.ctx(), &list[i].1.to_lowercase());
                }
                // (The keys choose as they move: a picker's.)
                if let Some(i) = out.moved.or(out.activated) {
                    w.portrait = Some(list[i].0);
                }
                out
            });
            // (Opened at the portrait chosen, once each time the page
            // or the list is shown, as Select Portrait opens.)
            let placed_id = id.with("placed");
            let pass = ui.ctx().cumulative_pass_nr();
            let (last, placed) =
                ui.data(|d| d.get_temp::<(u64, bool)>(placed_id)).unwrap_or((0, false));
            let placed = (placed && last + 1 >= pass)
                || match at {
                    Some(at) => crate::widgets::open_gallery_at(ui.ctx(), &shown, &shown.inner, at),
                    None => true,
                };
            ui.data_mut(|d| d.insert_temp(placed_id, (pass, placed)));
            ui.vertical(|ui| {
                ui.set_width(preview);
                let chosen = w.portrait.and_then(|p| list.iter().find(|(r, _)| *r == p));
                if let Some((_, base)) = chosen {
                    let base = base.to_lowercase();
                    let large = loader.picture(ui.ctx(), &format!("po_{base}l")).is_some();
                    let size = if large { 'l' } else { 'm' };
                    // (As large as the room under it holds, its name
                    // too.)
                    let name = ui.text_style_height(&egui::TextStyle::Body) + 8.0;
                    let left = if short { high } else { ui.available_height() };
                    let fits = (left - name) * 64.0 / 100.0;
                    let wide = fits.clamp(48.0, 128.0);
                    loader.portrait(ui, &base, size, wide, egui::Sense::hover());
                    ui.label(base);
                }
            });
        });
    }
}

fn page(
    ui: &mut Ui,
    loader: &mut Loader<'_>,
    load_order: u64,
    w: &mut CreatureWizard,
    summary: &str,
    looks: &mut Looks,
    names: &Names,
) {
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
                mg_uikit::list_rows(ui);
                for c in races(game).iter() {
                    if ui.selectable_label(w.race == Some(c.row as u32), &c.text).clicked() {
                        w.choose_race(game, c.row as u32);
                    }
                }
            });
        }
        2 => {
            ui.label("Please choose 1 to 8 classes for this creature and select the level for each class.");
            let names =
                crate::widgets::table_choices(Some(game), "classes", Some("Name"), Some("Label"));
            let name = |k: u32| {
                names.iter().find(|c| c.row == k as usize).map_or(String::new(), |c| c.text.clone())
            };
            class_page(ui, w, &names, &name);
        }
        3 => {
            // (Where the page is short of the room its wells take, the
            // least window, the page scrolls, its wells a row of pictures
            // high.)
            let room = ui.available_height();
            if room < LOOK_ROOM {
                egui::ScrollArea::vertical()
                    .id_salt("cw-look-page")
                    .max_height(room)
                    .auto_shrink([false, false])
                    .show(ui, |ui| look_page(ui, loader, w, looks, true));
            } else {
                look_page(ui, loader, w, looks, false);
            }
        }
        4 => {
            ui.label("Choose a Faction for this creature.");
            ui.label(
                "The Faction determines how this creature reacts to players and other creatures.",
            );
            mg_uikit::with_rows(ui, |ui| {
                for (i, name) in mg_module::factions::STANDARD.iter().enumerate().skip(1) {
                    if ui.selectable_label(usize::from(w.faction) == i, *name).clicked() {
                        w.faction = i as u16;
                    }
                }
            });
        }
        5 => {
            // A random name to begin with, as Aurora gives (from the race's
            // letter tables; none for races without).
            let race = w.race.unwrap_or(6);
            let mut rng = fastrand::Rng::new();
            w.seed_name_once(game, race, &mut rng);
            ui.label("Please enter a name for this new Creature.");
            egui::Grid::new("cw-name").num_columns(2).show(ui, |ui| {
                crate::widgets::field_label(ui, "Name");
                ui.end_row();
                let field = ui.add(mg_uikit::text_field(&mut w.first_name));
                crate::widgets::autofocus(ui, &field);
                if ui.button("Random").clicked()
                    && let Some(n) = game.random_name(race, w.gender, false, &mut rng)
                {
                    w.first_name = n;
                }
                ui.end_row();
                crate::widgets::field_label(ui, "Last Name:");
                ui.end_row();
                ui.add(mg_uikit::text_field(&mut w.last_name));
                if ui.button("Random").clicked()
                    && let Some(n) = game.random_name(race, w.gender, true, &mut rng)
                {
                    w.last_name = n;
                }
                ui.end_row();
            });
            // The ResRef and Tag it gets: made from the name, as in Aurora,
            // unless others are typed here.
            ui.add_space(6.0);
            let name = w.first_name.trim().to_string();
            egui::Grid::new("cw-names").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "ResRef");
                ui.add(
                    crate::widgets::resref_edit(&mut ResRefText(&mut w.resref))
                        .hint_text(
                            blueprints::resref_named(&name, &names.naming, |r| {
                                names.taken.contains(r)
                            })
                            .to_string(),
                        )
                        .desired_width(180.0),
                )
                .on_hover_text(crate::blueprint_wizard::made_from_the_name(&names.naming));
                ui.end_row();
                crate::widgets::field_label(ui, "Tag");
                ui.add(
                    mg_uikit::text_field(&mut w.tag)
                        .hint_text(blueprints::tag(&name))
                        .char_limit(mg_module::tags::MAX_TAG)
                        .desired_width(180.0),
                )
                .on_hover_text("Left empty: made from the name");
                ui.end_row();
            });
            let own = w.resref.trim();
            if !own.is_empty()
                && let Err(e) = ResRef::from_str(own)
            {
                ui.colored_label(ui.visuals().warn_fg_color, e.to_string());
            }
            crate::blueprint_wizard::code_button(ui, &mut w.resref, &names.naming);
        }
        6 => {
            ui.label("Please select the category in the palette that you wish this blueprint to appear under.");
            let module = loader.ws.map(|ws| &ws.module);
            let mark = (load_order, loader.ws.map(|ws| (ws.identity(), ws.revision())));
            let nodes = crate::blueprint::kept_for_key(
                ui.ctx(),
                egui::Id::new("creature-wizard-categories"),
                &mark,
                || {
                    std::sync::Arc::new(crate::blueprint_wizard::categories(
                        game,
                        module,
                        BlueprintKind::Creature,
                    ))
                },
            );
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                mg_uikit::list_rows(ui);
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
            ui.check_box(&mut w.launch, "Launch Creature Properties");
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
        // The ResRef typed, unless the module has it (then the next free
        // one from it, as for one made from the name: with the builder's
        // code).
        let own = ResRef::from_str(w.resref.trim()).ok().filter(|r| !r.is_empty());
        let naming = crate::blueprint_wizard::naming(app, ResType::UTC);
        let resref = match own {
            Some(r) if !taken(&r) => r,
            Some(r) => blueprints::resref_named(&r.to_string(), &naming, taken),
            None => blueprints::resref_named(w.first_name.trim(), &naming, taken),
        };
        let spec = w.spec(resref);
        let item = item_reader(app);
        let mut g = blueprints::creature(game, &spec, &item);
        let tag = match w.tag.trim() {
            "" => blueprints::tag(&spec.first_name),
            own => own.to_string(),
        };
        g.root.set("Tag", mg_gff::Value::String(crate::text::encode(&tag)));
        (ResKey::new(resref, ResType::UTC), g.to_bytes())
    };
    match bytes {
        Ok(data) => {
            app.actions.push(Action::ApplyMade(
                Command::new("New creature", vec![Edit::SetResource { key, data: Some(data) }]),
                crate::plugin_events::Made::resources("wizard", vec![key]),
            ));
            if w.launch {
                app.actions.push(Action::OpenTab(Tab::Blueprint(key)));
            }
        }
        Err(e) => app.log.error(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_portraits_are_aurora_s_wizard_s() {
        // Rows of a human (6) male (0), female (1), of no gender (4), a
        // plot one, a placeable's, one of the module's own (its pictures in
        // the module) and an outsider's (20), all of no gender.
        let table = "2DA V2.0\n\n  BaseResRef Sex Race InanimateType Plot\n\
                     0 hu_m_01_ 0 6 **** 0\n1 hu_f_01_ 1 6 **** 0\n2 kid_ 4 6 **** 0\n\
                     3 hu_m_31_ 0 6 **** 1\n4 door01_ 0 6 1 ****\n5 w1po_ 0 6 **** 0\n\
                     6 hu_m_02_ 0 6 **** 0\n7 imp_ 4 20 **** 0\n";
        let tga = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 32, 8, 9, 9, 9, 255];
        let mut mem = mg_resman::MemContainer::new();
        mem.insert(ResKey::parse("portraits", ResType::TWODA).unwrap(), table.as_bytes().to_vec());
        for base in ["hu_m_01_", "hu_f_01_", "kid_", "hu_m_31_", "door01_", "hu_m_02_", "imp_"] {
            mem.insert(ResKey::parse(&format!("po_{base}m"), ResType::TGA).unwrap(), tga.clone());
        }
        let mut rm = mg_resman::ResMan::new();
        rm.add(mg_resman::priority::KEY, "game", mg_resman::LayerClass::Key, mem);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse("po_w1po_m", ResType::TGA).unwrap(), tga.clone());
        let ws = mg_edit::Workspace::new(m);
        let names = |list: Vec<(u16, String)>| list.into_iter().map(|(_, b)| b).collect::<Vec<_>>();
        // The gender's alone, in the table's order, the module's own too.
        let male = names(portraits(&game, Some(&ws), 6, 0, false));
        assert_eq!(male, ["hu_m_01_", "w1po_", "hu_m_02_"]);
        assert_eq!(names(portraits(&game, None, 6, 0, false)), ["hu_m_01_", "hu_m_02_"]);
        // A race with none of the gender: those of no gender.
        assert_eq!(names(portraits(&game, Some(&ws), 20, 0, false)), ["imp_"]);
        // All: every creature's.
        let all = names(portraits(&game, Some(&ws), 6, 0, true));
        assert_eq!(all, ["hu_m_01_", "hu_f_01_", "kid_", "hu_m_31_", "w1po_", "hu_m_02_", "imp_"]);
    }

    #[test]
    fn clearing_first_name_does_not_reseed_until_race_changes() {
        let mut mem = mg_resman::MemContainer::new();
        mem.insert(
            ResKey::parse("racialtypes", ResType::TWODA).unwrap(),
            b"2DA V2.0\n\n  Label Appearance ToolsetDefaultClass NameGenTableFirstNameMale NameGenTableLastName\n\
              0 Dwarf 0 4 testfn testln\n6 Human 6 4 testfn testln\n"
                .to_vec(),
        );
        mem.insert(ResKey::parse("testfn", ResType::LTR).unwrap(), vec![0; 4]);
        let mut rm = mg_resman::ResMan::new();
        rm.add(mg_resman::priority::KEY, "game", mg_resman::LayerClass::Key, mem);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut w = CreatureWizard { race: Some(6), ..Default::default() };
        let mut rng = fastrand::Rng::with_seed(1);
        w.first_name = "Alice".into();
        w.seed_name_once(&game, 6, &mut rng);
        assert!(w.name_seeded);
        assert_eq!(w.first_name, "Alice");
        w.first_name.clear();
        w.seed_name_once(&game, 6, &mut rng);
        assert_eq!(w.first_name, "", "clearing First Name stays empty across frames");
        w.choose_race(&game, 0);
        assert!(!w.name_seeded, "changing race allows re-seeding");
    }
}
