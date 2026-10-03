//! The Script Wizard window, opened from the Conversation Editor's Text
//! Appears When… and Actions Taken tabs as in Aurora: tick what to test (or
//! do), fill a page for each, name the script. Finish writes the source,
//! compiles it and sets it on the line, in one undoable command.

use std::sync::Arc;

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Value;
use mg_module::script_wizard::{
    ABILITIES, ActionScript, Alignment, ClassLevel, Compare, Condition, Difficulty, Entry,
    GOOD_EVIL, LAW_CHAOS, Lists, LocalCheck, LocalSet, Operand, Perform, VarType, action_script,
    condition_script, default_name,
};
use mg_resman::ResKey;

use crate::{Moonglow, Tab};

/// The condition pages: the first page's checkbox and the page's question.
const CONDITION_PAGES: [(&str, &str); 11] = [
    ("Abilities", "What are the ability requirements?"),
    ("Class", "What are the class and level restrictions?"),
    ("Gender", "Which gender(s)?"),
    ("Race", "What are the race restrictions?"),
    ("Alignment", "Allow what alignments?"),
    ("Feats", "What are the required feats?"),
    ("Skills", "What are the required skills?"),
    ("Skill Check", "What skill checks must be done?"),
    ("Item In Inventory", "Which items must be in the inventory?"),
    ("Local Variable", "What local variables have to be set?"),
    ("Random", "How random is this?"),
];

const ACTION_PAGES: [(&str, &str); 4] = [
    ("Give rewards", "Give what rewards?"),
    ("Take from the player", "Take what?"),
    ("Set local variables", "Set what local variables?"),
    ("Perform an action", "What actions would you like to perform?"),
];

/// Where the wizard is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Choosing the pages.
    Choose,
    Page(usize),
    /// Naming the script.
    Name,
}

/// A local variable being entered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalDraft {
    pub ty: VarType,
    pub name: String,
    pub compare: Compare,
    /// The value is another variable (else a constant).
    pub variable: bool,
    pub value: String,
}

impl LocalDraft {
    fn operand(&self) -> Operand {
        if self.variable {
            Operand::Variable(self.value.trim().to_string())
        } else {
            Operand::Constant(self.value.clone())
        }
    }
}

/// The wizard's choices.
#[derive(Debug, Clone)]
pub struct ScriptWizard {
    /// The conversation, and the field the script goes in.
    pub dialog: ResKey,
    pub path: GffPath,
    pub label: String,
    /// A condition (else an action).
    pub condition: bool,
    pub lists: Arc<Lists>,
    pub step: Step,
    /// The pages ticked on the first page.
    pub pages: Vec<bool>,
    pub abilities: [(bool, Compare, i32); 6],
    /// The class to add (an index into the class list; `None` for Any),
    /// at any level or at least `level`.
    pub class: Option<usize>,
    pub specific_level: bool,
    pub level: u32,
    pub classes: Vec<ClassLevel>,
    pub all_classes: bool,
    pub genders: Vec<usize>,
    /// Accepted races (rows).
    pub races: Vec<usize>,
    pub alignment: Alignment,
    pub feats: Vec<usize>,
    pub skills: Vec<usize>,
    pub difficulty: Difficulty,
    pub checks: Vec<(Difficulty, usize)>,
    pub items: Vec<String>,
    pub local: LocalDraft,
    pub locals: Vec<LocalCheck>,
    pub random: (u32, u32),
    pub give_gold: String,
    pub gold_to_party: bool,
    pub give_xp: String,
    pub xp_to_party: bool,
    pub give_items: Vec<String>,
    pub take_gold: String,
    pub destroy_gold: bool,
    pub take_xp: String,
    pub take_items: Vec<String>,
    pub destroy_items: bool,
    pub sets: Vec<LocalSet>,
    pub perform: Perform,
    pub faction: i32,
    /// Text being typed into a page's "add" field.
    pub entry: String,
    /// The selected row of the page's lists: (list, index).
    pub selected: Option<(u8, usize)>,
    /// The feat list's filter.
    pub filter: String,
    pub name: String,
    pub open_editor: bool,
}

impl ScriptWizard {
    pub fn new(
        dialog: ResKey,
        path: GffPath,
        label: &str,
        condition: bool,
        lists: Arc<Lists>,
        name: String,
    ) -> ScriptWizard {
        let n = if condition { CONDITION_PAGES.len() } else { ACTION_PAGES.len() };
        ScriptWizard {
            dialog,
            path,
            label: label.to_string(),
            condition,
            lists,
            step: Step::Choose,
            pages: vec![false; n],
            abilities: [(false, Compare::Equal, 8); 6],
            class: None,
            specific_level: false,
            level: 1,
            classes: Vec::new(),
            all_classes: false,
            genders: Vec::new(),
            races: Vec::new(),
            alignment: Alignment::default(),
            feats: Vec::new(),
            skills: Vec::new(),
            difficulty: Difficulty::Easy,
            checks: Vec::new(),
            items: Vec::new(),
            local: LocalDraft::default(),
            locals: Vec::new(),
            random: (1, 100),
            give_gold: String::new(),
            gold_to_party: false,
            give_xp: String::new(),
            xp_to_party: false,
            give_items: Vec::new(),
            take_gold: String::new(),
            destroy_gold: true,
            take_xp: String::new(),
            take_items: Vec::new(),
            destroy_items: true,
            sets: Vec::new(),
            perform: Perform::Nothing,
            faction: 0,
            entry: String::new(),
            selected: None,
            filter: String::new(),
            name,
            open_editor: false,
        }
    }

    fn page_list(&self) -> &'static [(&'static str, &'static str)] {
        if self.condition { &CONDITION_PAGES } else { &ACTION_PAGES }
    }

    fn on(&self, page: usize) -> bool {
        self.pages.get(page).copied().unwrap_or(false)
    }

    fn next(&self) -> Step {
        let from = match self.step {
            Step::Choose => 0,
            Step::Page(i) => i + 1,
            Step::Name => return Step::Name,
        };
        (from..self.pages.len()).find(|&i| self.pages[i]).map_or(Step::Name, Step::Page)
    }

    fn back(&self) -> Step {
        let to = match self.step {
            Step::Choose => return Step::Choose,
            Step::Page(i) => i,
            Step::Name => self.pages.len(),
        };
        (0..to).rev().find(|&i| self.pages[i]).map_or(Step::Choose, Step::Page)
    }

    /// The condition from the pages ticked.
    pub fn to_condition(&self) -> Condition {
        let on = |i| self.on(i);
        Condition {
            abilities: if on(0) {
                (0..6)
                    .filter(|&i| self.abilities[i].0)
                    .map(|i| (i, self.abilities[i].1, self.abilities[i].2))
                    .collect()
            } else {
                Vec::new()
            },
            classes: if on(1) { self.classes.clone() } else { Vec::new() },
            all_classes: self.all_classes,
            genders: on(2).then(|| self.genders.clone()),
            races: on(3).then(|| self.races.clone()),
            alignment: on(4).then_some(self.alignment),
            feats: if on(5) { self.feats.clone() } else { Vec::new() },
            skills: if on(6) { self.skills.clone() } else { Vec::new() },
            skill_checks: if on(7) { self.checks.clone() } else { Vec::new() },
            items: if on(8) { self.items.clone() } else { Vec::new() },
            locals: if on(9) { self.locals.clone() } else { Vec::new() },
            random: on(10).then_some(self.random),
        }
    }

    /// The action from the pages ticked.
    pub fn to_action(&self) -> ActionScript {
        let on = |i| self.on(i);
        let amount = |s: &str| s.trim().parse::<u32>().ok().filter(|&n| n > 0);
        let rewards = on(0);
        let take = on(1);
        ActionScript {
            give_gold: amount(&self.give_gold).filter(|_| rewards).map(|n| (n, self.gold_to_party)),
            give_xp: amount(&self.give_xp).filter(|_| rewards).map(|n| (n, self.xp_to_party)),
            give_items: if rewards { self.give_items.clone() } else { Vec::new() },
            take_gold: amount(&self.take_gold).filter(|_| take).map(|n| (n, self.destroy_gold)),
            take_xp: amount(&self.take_xp).filter(|_| take),
            take_items: if take { self.take_items.clone() } else { Vec::new() },
            destroy_items: self.destroy_items,
            locals: if on(2) { self.sets.clone() } else { Vec::new() },
            perform: if on(3) { self.perform.clone() } else { Perform::Nothing },
            faction: if on(3) { self.faction } else { 0 },
        }
    }

    /// The script's source.
    pub fn source(&self, date: &str) -> String {
        let name = self.name.trim();
        if self.condition {
            condition_script(name, date, &self.to_condition(), &self.lists)
        } else {
            action_script(name, date, &self.to_action())
        }
    }
}

/// Today's date, `YYYY-MM-DD` (UTC).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

impl Moonglow {
    /// Opens the Script Wizard for a conversation's condition (`Active` on
    /// a link) or action (`Script` on a line).
    pub(crate) fn open_script_wizard(
        &mut self,
        dialog: ResKey,
        path: GffPath,
        label: &str,
        condition: bool,
    ) {
        let Some(game) = &self.game else {
            self.log.error("The Script Wizard needs the game data (Tools > Options)");
            return;
        };
        let lists = Arc::new(mg_module::script_wizard::Lists::load(game));
        let module = self.ws.as_ref().map(|w| &w.module);
        let name = default_name(condition, |n| {
            module.is_some_and(|m| {
                [ResType::NSS, ResType::NCS]
                    .iter()
                    .any(|&t| ResKey::parse(n, t).is_some_and(|k| m.contains(&k)))
            })
        });
        self.script_wizard = Some(ScriptWizard::new(dialog, path, label, condition, lists, name));
    }
}

/// A list of rows to pick from; returns the row double-clicked.
fn pick_list(
    ui: &mut Ui,
    id: &str,
    rows: &[(usize, &str)],
    list: u8,
    selected: &mut Option<(u8, usize)>,
    size: [f32; 2],
) -> Option<usize> {
    let mut chosen = None;
    egui::Frame::group(ui.style()).inner_margin(2.0).show(ui, |ui| {
        ui.set_min_size(size.into());
        ui.set_max_width(size[0]);
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::vertical()
                .id_salt(id)
                .max_height(size[1])
                .auto_shrink(false)
                .show_rows(
                    ui,
                    ui.text_style_height(&egui::TextStyle::Body),
                    rows.len(),
                    |ui, range| {
                        for &(row, text) in &rows[range] {
                            let r = ui.selectable_label(*selected == Some((list, row)), text);
                            if r.clicked() {
                                *selected = Some((list, row));
                            }
                            if r.double_clicked() {
                                chosen = Some(row);
                            }
                        }
                    },
                );
        });
    });
    chosen
}

/// Two lists with -> and <- between them (Feats, Skills, Race): moves rows
/// between `chosen` and the rest of `all`.
#[allow(clippy::too_many_arguments)]
fn move_lists(
    ui: &mut Ui,
    id: &str,
    all: &[Entry],
    chosen: &mut Vec<usize>,
    titles: [&str; 2],
    selected: &mut Option<(u8, usize)>,
    lists: [u8; 2],
    filter: Option<&str>,
    height: f32,
) {
    let filter = filter.map(str::to_lowercase).unwrap_or_default();
    let left: Vec<(usize, &str)> = all
        .iter()
        .filter(|e| !chosen.contains(&e.row))
        .filter(|e| filter.is_empty() || e.name.to_lowercase().contains(&filter))
        .map(|e| (e.row, e.name.as_str()))
        .collect();
    let right: Vec<(usize, &str)> =
        all.iter().filter(|e| chosen.contains(&e.row)).map(|e| (e.row, e.name.as_str())).collect();
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(titles[0]);
            if let Some(row) =
                pick_list(ui, &format!("{id}-l"), &left, lists[0], selected, [200.0, height])
            {
                chosen.push(row);
            }
        });
        ui.vertical(|ui| {
            ui.add_space(40.0);
            if ui.button("->").on_hover_text(format!("Move to {}", titles[1])).clicked()
                && let Some((l, row)) = *selected
                && l == lists[0]
                && !chosen.contains(&row)
            {
                chosen.push(row);
            }
            if ui.button("<-").on_hover_text(format!("Move to {}", titles[0])).clicked()
                && let Some((l, row)) = *selected
                && l == lists[1]
            {
                chosen.retain(|&r| r != row);
            }
        });
        ui.vertical(|ui| {
            ui.label(titles[1]);
            if let Some(row) =
                pick_list(ui, &format!("{id}-r"), &right, lists[1], selected, [200.0, height])
            {
                chosen.retain(|&r| r != row);
            }
        });
    });
}

/// A text field with Add (and "…" for the item picker), and the list it
/// adds to with Remove. Returns whether "…" was clicked.
fn add_list(
    ui: &mut Ui,
    id: &str,
    entry: &mut String,
    items: &mut Vec<String>,
    sorted: bool,
    selected: &mut Option<(u8, usize)>,
    picker: bool,
) -> bool {
    let mut pick = false;
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(entry).desired_width(200.0).id_salt(id));
        if picker {
            pick = ui.small_button("…").on_hover_text("Select an item blueprint").clicked();
        }
        let tag = entry.trim().to_string();
        if ui.add_enabled(!tag.is_empty(), egui::Button::new("Add")).clicked()
            && !items.contains(&tag)
        {
            items.push(tag);
            if sorted {
                items.sort_by_cached_key(|s| (s.to_lowercase(), s.clone()));
            }
        }
    });
    let rows: Vec<(usize, &str)> = items.iter().enumerate().map(|(i, s)| (i, s.as_str())).collect();
    let mut remove = None;
    ui.horizontal_top(|ui| {
        pick_list(ui, id, &rows, 0, selected, [200.0, 120.0]);
        if ui.add_enabled(selected.is_some(), egui::Button::new("Remove")).clicked()
            && let Some((0, i)) = *selected
        {
            remove = Some(i);
        }
    });
    if let Some(i) = remove.filter(|&i| i < items.len()) {
        items.remove(i);
        *selected = None;
    }
    pick
}

fn combo<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: &str,
    value: &mut T,
    options: &[T],
    label: impl Fn(T) -> String,
) {
    egui::ComboBox::from_id_salt(id).selected_text(label(*value)).show_ui(ui, |ui| {
        for &o in options {
            ui.selectable_value(value, o, label(o));
        }
    });
}

fn local_draft(ui: &mut Ui, w: &mut ScriptWizard, condition: bool) -> Option<(String, Operand)> {
    let d = &mut w.local;
    egui::Grid::new("wiz-local").num_columns(2).show(ui, |ui| {
        combo(ui, "wiz-local-type", &mut d.ty, &VarType::ALL, |t| t.label().to_string());
        ui.add(egui::TextEdit::singleline(&mut d.name).hint_text("variable name"));
        ui.end_row();
        if condition {
            if !d.ty.comparisons().contains(&d.compare) {
                d.compare = Compare::Equal;
            }
            combo(ui, "wiz-local-cmp", &mut d.compare, d.ty.comparisons(), |c| {
                c.label().to_string()
            });
            ui.end_row();
        }
        let ty = d.ty.label();
        combo(ui, "wiz-local-src", &mut d.variable, &[false, true], |v| {
            format!("{} {ty}", if v { "variable" } else { "constant" })
        });
        ui.add(egui::TextEdit::singleline(&mut d.value).hint_text(if d.variable {
            "variable name"
        } else {
            "value"
        }));
        ui.end_row();
    });
    let name = d.name.trim().to_string();
    let operand = d.operand();
    let valid = !name.is_empty()
        && d.ty.value(&operand).is_some()
        && !matches!(&operand, Operand::Variable(v) if v.is_empty());
    valid.then_some((name, operand))
}

fn condition_page(ui: &mut Ui, w: &mut ScriptWizard, page: usize) {
    let lists = w.lists.clone();
    match page {
        0 => {
            egui::Grid::new("wiz-abilities").num_columns(3).show(ui, |ui| {
                for (i, (name, _)) in ABILITIES.iter().enumerate() {
                    let (on, cmp, value) = &mut w.abilities[i];
                    ui.checkbox(on, *name);
                    ui.add_enabled_ui(*on, |ui| {
                        combo(ui, &format!("wiz-ab-{i}"), cmp, &Compare::ALL, |c| {
                            c.label().to_string()
                        });
                    });
                    ui.add_enabled(*on, egui::DragValue::new(value).range(0..=255));
                    ui.end_row();
                }
            });
        }
        1 => {
            ui.horizontal(|ui| {
                let name = |c: Option<usize>| {
                    c.and_then(|i| lists.classes.get(i)).map_or("Any".into(), |e| e.name.clone())
                };
                let options: Vec<Option<usize>> =
                    std::iter::once(None).chain((0..lists.classes.len()).map(Some)).collect();
                combo(ui, "wiz-class", &mut w.class, &options, name);
                ui.vertical(|ui| {
                    ui.radio_value(&mut w.specific_level, false, "Any level");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut w.specific_level, true, "Specific level");
                        ui.add_enabled(
                            w.specific_level,
                            egui::DragValue::new(&mut w.level).range(1..=60),
                        );
                    });
                });
                if ui.button("Add").clicked() {
                    w.classes.push(ClassLevel {
                        class: w.class.and_then(|i| lists.classes.get(i)).map(|e| e.row),
                        level: w.specific_level.then_some(w.level),
                    });
                }
            });
            let class_name = |c: &ClassLevel| {
                c.class
                    .and_then(|r| lists.classes.iter().find(|e| e.row == r))
                    .map_or("Any".into(), |e| e.name.clone())
            };
            let mut remove = None;
            ui.horizontal_top(|ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_min_size([280.0, 100.0].into());
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        egui::Grid::new("wiz-classes").num_columns(2).striped(true).show(
                            ui,
                            |ui| {
                                ui.strong("Class");
                                ui.strong("Min Level");
                                ui.end_row();
                                for (i, c) in w.classes.iter().enumerate() {
                                    let on = w.selected == Some((0, i));
                                    if ui.selectable_label(on, class_name(c)).clicked() {
                                        w.selected = Some((0, i));
                                    }
                                    ui.label(c.level.map_or("Any".into(), |l| l.to_string()));
                                    ui.end_row();
                                }
                            },
                        );
                    });
                });
                if ui.add_enabled(w.selected.is_some(), egui::Button::new("Remove")).clicked()
                    && let Some((0, i)) = w.selected
                {
                    remove = Some(i);
                }
            });
            if let Some(i) = remove.filter(|&i| i < w.classes.len()) {
                w.classes.remove(i);
                w.selected = None;
            }
            ui.add_enabled_ui(w.classes.len() > 1, |ui| {
                ui.radio_value(
                    &mut w.all_classes,
                    false,
                    "The player needs to meet only one of the restrictions",
                );
                ui.radio_value(
                    &mut w.all_classes,
                    true,
                    "The player needs to meet all of the restrictions",
                );
            });
        }
        2 => {
            ui.label("Select required genders");
            egui::Frame::group(ui.style()).show(ui, |ui| {
                for e in &lists.genders {
                    let on = w.genders.contains(&e.row);
                    if ui.selectable_label(on, &e.name).clicked() {
                        if on {
                            w.genders.retain(|&r| r != e.row);
                        } else {
                            w.genders.push(e.row);
                        }
                    }
                }
            });
        }
        3 => {
            for (list, title, ids) in
                [(&lists.player_races, "Player", [0u8, 1]), (&lists.other_races, "Other", [2, 3])]
            {
                crate::widgets::field_label(ui, title);
                let titles = ["Rejected", "Accepted"];
                move_lists(
                    ui,
                    &format!("wiz-race-{title}"),
                    list,
                    &mut w.races,
                    titles,
                    &mut w.selected,
                    ids,
                    None,
                    95.0,
                );
            }
        }
        4 => {
            for (i, (name, _)) in GOOD_EVIL.iter().enumerate() {
                ui.checkbox(&mut w.alignment.good_evil[i], *name);
            }
            ui.add_space(8.0);
            for (i, (name, _)) in LAW_CHAOS.iter().enumerate() {
                ui.checkbox(&mut w.alignment.law_chaos[i], *name);
            }
        }
        5 => {
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Filter");
                ui.text_edit_singleline(&mut w.filter);
            });
            move_lists(
                ui,
                "wiz-feats",
                &lists.feats,
                &mut w.feats,
                ["All Feats", "Required Feats"],
                &mut w.selected,
                [0, 1],
                Some(&w.filter),
                200.0,
            );
        }
        6 => {
            move_lists(
                ui,
                "wiz-skills",
                &lists.skills_by_name(),
                &mut w.skills,
                ["All Skills", "Required Skills"],
                &mut w.selected,
                [0, 1],
                None,
                220.0,
            );
        }
        7 => {
            let skills: Vec<(usize, &str)> =
                lists.skills.iter().map(|e| (e.row, e.name.as_str())).collect();
            let mut remove = None;
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    crate::widgets::field_label(ui, "Available Skills");
                    pick_list(ui, "wiz-check-skills", &skills, 0, &mut w.selected, [180.0, 200.0]);
                });
                ui.vertical(|ui| {
                    crate::widgets::field_label(ui, "Difficulty");
                    ui.horizontal(|ui| {
                        for d in Difficulty::ALL {
                            ui.radio_value(&mut w.difficulty, d, d.label());
                        }
                    });
                    let skill = w.selected.filter(|s| s.0 == 0).map(|s| s.1);
                    if ui.add_enabled(skill.is_some(), egui::Button::new("Add")).clicked()
                        && let Some(row) = skill
                    {
                        w.checks.push((w.difficulty, row));
                    }
                    crate::widgets::field_label(ui, "Checks");
                    let checks: Vec<String> = w
                        .checks
                        .iter()
                        .map(|(d, row)| {
                            let c = lists.skills.iter().find(|e| e.row == *row);
                            format!(
                                "AutoDC({}, {}, GetPCSpeaker())",
                                d.constant(),
                                c.map_or_else(|| row.to_string(), |e| e.constant.clone())
                            )
                        })
                        .collect();
                    let rows: Vec<(usize, &str)> =
                        checks.iter().enumerate().map(|(i, s)| (i, s.as_str())).collect();
                    ui.horizontal_top(|ui| {
                        pick_list(ui, "wiz-checks", &rows, 1, &mut w.selected, [260.0, 100.0]);
                        if ui
                            .add_enabled(
                                w.selected.is_some_and(|s| s.0 == 1),
                                egui::Button::new("Remove"),
                            )
                            .clicked()
                            && let Some((1, i)) = w.selected
                        {
                            remove = Some(i);
                        }
                    });
                });
            });
            if let Some(i) = remove.filter(|&i| i < w.checks.len()) {
                w.checks.remove(i);
                w.selected = None;
            }
        }
        8 => {
            ui.label("Enter a new tag");
            add_list(ui, "wiz-items", &mut w.entry, &mut w.items, true, &mut w.selected, false);
        }
        9 => {
            let draft = local_draft(ui, w, true);
            if ui.add_enabled(draft.is_some(), egui::Button::new("Add")).clicked()
                && let Some((name, value)) = draft
            {
                let d = &w.local;
                w.locals.push(LocalCheck { ty: d.ty, name, compare: d.compare, value });
            }
            crate::widgets::field_label(ui, "Local Expressions");
            let mut exprs: Vec<(usize, String)> = w
                .locals
                .iter()
                .enumerate()
                .filter_map(|(i, l)| l.expression().map(|e| (i, e)))
                .collect();
            exprs.sort_by_cached_key(|(_, e)| e.to_lowercase());
            local_list(ui, &mut w.locals, &exprs, &mut w.selected);
        }
        _ => {
            ui.label("Has the chance of appearing of");
            ui.add(egui::DragValue::new(&mut w.random.0).range(1..=w.random.1.max(1)));
            ui.label("in");
            ui.add(egui::DragValue::new(&mut w.random.1).range(1..=10_000));
            w.random.0 = w.random.0.min(w.random.1);
        }
    }
}

/// The added local variables (as their expressions) with Remove.
fn local_list<T>(
    ui: &mut Ui,
    items: &mut Vec<T>,
    shown: &[(usize, String)],
    selected: &mut Option<(u8, usize)>,
) {
    let rows: Vec<(usize, &str)> = shown.iter().map(|(i, s)| (*i, s.as_str())).collect();
    let mut remove = None;
    ui.horizontal_top(|ui| {
        pick_list(ui, "wiz-locals", &rows, 5, selected, [380.0, 100.0]);
        if ui.add_enabled(selected.is_some(), egui::Button::new("Remove")).clicked()
            && let Some((5, i)) = *selected
        {
            remove = Some(i);
        }
    });
    if let Some(i) = remove.filter(|&i| i < items.len()) {
        items.remove(i);
        *selected = None;
    }
}

fn action_page(app: &mut Moonglow, ui: &mut Ui, w: &mut ScriptWizard, page: usize) {
    let number = |ui: &mut Ui, s: &mut String, id: &str| {
        ui.add(egui::TextEdit::singleline(s).desired_width(70.0).id_salt(id));
        s.retain(|c| c.is_ascii_digit());
    };
    match page {
        0 => {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    crate::widgets::field_label(ui, "Give gold");
                    number(ui, &mut w.give_gold, "wiz-give-gold");
                    ui.checkbox(&mut w.gold_to_party, "To Party");
                });
                ui.vertical(|ui| {
                    crate::widgets::field_label(ui, "Give XP");
                    number(ui, &mut w.give_xp, "wiz-give-xp");
                    ui.checkbox(&mut w.xp_to_party, "To Party");
                });
                ui.vertical(|ui| {
                    let id = egui::Id::new("wiz-give-item-pick");
                    if let Some(r) = app.take_pick(id) {
                        w.entry = r.to_string();
                    }
                    crate::widgets::field_label(ui, "Give item (by resref)");
                    if add_list(
                        ui,
                        "wiz-give-items",
                        &mut w.entry,
                        &mut w.give_items,
                        false,
                        &mut w.selected,
                        true,
                    ) {
                        app.open_picker(id, "Select an item blueprint", &[ResType::UTI]);
                    }
                });
            });
        }
        1 => {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    crate::widgets::field_label(ui, "Take gold");
                    number(ui, &mut w.take_gold, "wiz-take-gold");
                    ui.radio_value(&mut w.destroy_gold, true, "Destroy");
                    ui.radio_value(&mut w.destroy_gold, false, "Keep");
                    ui.add_space(12.0);
                    crate::widgets::field_label(ui, "Take XP");
                    number(ui, &mut w.take_xp, "wiz-take-xp");
                });
                ui.vertical(|ui| {
                    crate::widgets::field_label(ui, "Take item (by Tag)");
                    add_list(
                        ui,
                        "wiz-take-items",
                        &mut w.entry,
                        &mut w.take_items,
                        false,
                        &mut w.selected,
                        false,
                    );
                    ui.radio_value(&mut w.destroy_items, true, "Destroy");
                    ui.radio_value(&mut w.destroy_items, false, "Keep");
                });
            });
        }
        2 => {
            crate::widgets::field_label(ui, "Set local variable");
            let draft = local_draft(ui, w, false);
            if ui.add_enabled(draft.is_some(), egui::Button::new("Add")).clicked()
                && let Some((name, value)) = draft
            {
                w.sets.push(LocalSet { ty: w.local.ty, name, value });
            }
            crate::widgets::field_label(ui, "Local Expressions");
            let mut shown: Vec<(usize, String)> = w
                .sets
                .iter()
                .enumerate()
                .filter_map(|(i, s)| s.statement().map(|e| (i, e)))
                .collect();
            shown.sort_by_cached_key(|(_, e)| e.to_lowercase());
            local_list(ui, &mut w.sets, &shown, &mut w.selected);
        }
        _ => {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                let store = matches!(w.perform, Perform::Store { .. });
                if ui.radio(w.perform == Perform::Nothing, "No Action").clicked() {
                    w.perform = Perform::Nothing;
                }
                if ui.radio(w.perform == Perform::Attack, "Attack").clicked() {
                    w.perform = Perform::Attack;
                }
                if ui.radio(store, "Start a Merchant").clicked() && !store {
                    w.perform = Perform::Store { tag: String::new(), appraise: true };
                }
                let (mut tag, mut appraise) = match &w.perform {
                    Perform::Store { tag, appraise } => (tag.clone(), *appraise),
                    _ => (String::new(), true),
                };
                ui.add_enabled_ui(store, |ui| {
                    ui.indent("wiz-store", |ui| {
                        ui.checkbox(&mut appraise, "Use appraise checks");
                        crate::widgets::field_label(ui, "Script Tag");
                        ui.add(egui::TextEdit::singleline(&mut tag).id_salt("wiz-store-tag"));
                    });
                });
                if store {
                    w.perform = Perform::Store { tag, appraise };
                }
            });
            ui.add_space(8.0);
            let attack = w.perform == Perform::Attack;
            let mut faction = if attack { -100 } else { w.faction };
            ui.add_enabled_ui(!attack, |ui| {
                ui.horizontal(|ui| {
                    crate::widgets::field_label(ui, "Modify Faction");
                    ui.spacing_mut().slider_width = 320.0;
                    ui.add(egui::Slider::new(&mut faction, -100..=100).step_by(10.0));
                });
            });
            if !attack {
                w.faction = faction;
            }
        }
    }
}

/// Draws the Script Wizard, if open.
pub(crate) fn window(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut w) = app.script_wizard.take() else { return };
    let ctx = ui.ctx().clone();
    let mut close = false;
    let mut finish = false;
    egui::Window::new("Script Wizard")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(&ctx, |ui| {
            ui.set_min_width(520.0);
            let pages = w.page_list();
            let question = match w.step {
                Step::Choose if w.condition => "What conditions would you like to test for?",
                Step::Choose => "What actions would you like to perform?",
                Step::Page(i) => pages[i].1,
                Step::Name => "Congratulations!",
            };
            ui.heading(question);
            ui.separator();
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                ui.set_min_height(300.0);
                match w.step {
                    Step::Choose => {
                        ui.columns(2, |cols| {
                            let half = if w.condition { 8 } else { pages.len() };
                            for (i, (label, _)) in pages.iter().enumerate() {
                                let col = usize::from(i >= half);
                                cols[col].checkbox(&mut w.pages[i], *label);
                            }
                        });
                    }
                    Step::Page(i) if w.condition => condition_page(ui, &mut w, i),
                    Step::Page(i) => action_page(app, ui, &mut w, i),
                    Step::Name => {
                        ui.label("Enter the script name");
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut w.name)
                                .char_limit(16)
                                .id_salt("wiz-name"),
                        );
                        crate::widgets::autofocus(ui, &field);
                        let exists = ResRef::from_str(w.name.trim()).ok().is_some_and(|r| {
                            app.ws
                                .as_ref()
                                .is_some_and(|ws| ws.module.contains(&ResKey::new(r, ResType::NSS)))
                        });
                        if exists {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                "The module has a script of this name; it will be replaced.",
                            );
                        }
                        ui.checkbox(&mut w.open_editor, "Start the script editor");
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                let valid_name =
                    ResRef::from_str(w.name.trim()).ok().is_some_and(|r| !r.is_empty());
                if ui.add_enabled(w.step != Step::Choose, egui::Button::new("< Back")).clicked() {
                    w.step = w.back();
                    w.selected = None;
                    w.entry.clear();
                }
                if ui.add_enabled(w.step != Step::Name, egui::Button::new("Next >")).clicked()
                    || (w.step != Step::Name && crate::widgets::enter(ui))
                {
                    w.step = w.next();
                    w.selected = None;
                    w.entry.clear();
                }
                if ui
                    .add_enabled(w.step == Step::Name && valid_name, egui::Button::new("Finish"))
                    .clicked()
                    || (w.step == Step::Name && valid_name && crate::widgets::enter(ui))
                {
                    finish = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    if finish {
        finish_wizard(app, &w);
        close = true;
    }
    if !close {
        app.script_wizard = Some(w);
    }
}

/// Writes, compiles and sets the script.
fn finish_wizard(app: &mut Moonglow, w: &ScriptWizard) {
    let Ok(name) = ResRef::from_str(w.name.trim()) else { return };
    let source = w.source(&today());
    let nss = ResKey::new(name, ResType::NSS);
    let mut edits = vec![
        Edit::SetResource { key: nss, data: Some(crate::text::encode(&source)) },
        Edit::SetField {
            key: w.dialog,
            path: w.path.clone(),
            label: w.label.clone(),
            value: Some(Value::resref(name)),
        },
    ];
    match crate::script_view::compile_source(app, nss, &source) {
        Ok(ncs) => {
            edits.push(Edit::SetResource { key: ResKey::new(name, ResType::NCS), data: Some(ncs) })
        }
        Err(e) => app.log.error(e.message),
    }
    app.actions.push(crate::Action::Apply(Command::new(format!("Script Wizard: {name}"), edits)));
    app.log.info(format!("Script Wizard: wrote {nss}"));
    if w.open_editor {
        app.actions.push(crate::Action::OpenTab(Tab::Script(nss)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_date_is_iso() {
        let d = today();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20"));
        assert_eq!(&d[4..5], "-");
    }

    #[test]
    fn next_and_back_visit_the_ticked_pages() {
        let key = ResKey::parse("d", ResType::DLG).unwrap();
        let mut w = ScriptWizard::new(
            key,
            GffPath::root(),
            "Active",
            true,
            Arc::new(Lists::default()),
            "sc_001".into(),
        );
        w.pages[2] = true;
        w.pages[8] = true;
        assert_eq!(w.next(), Step::Page(2));
        w.step = Step::Page(2);
        assert_eq!(w.next(), Step::Page(8));
        w.step = Step::Page(8);
        assert_eq!(w.next(), Step::Name);
        assert_eq!(w.back(), Step::Page(2));
        w.step = Step::Name;
        assert_eq!(w.back(), Step::Page(8));
    }
}
