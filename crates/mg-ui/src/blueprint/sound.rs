//! Sound Properties (`TdlgSoundEdit`): Basic (sounds, volume), Positioning,
//! Advanced (timing, play style and order, interval, variations), Comments.

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;

use super::Form;
use crate::Action;
use crate::audio::Channel;

pub(super) const PAGES: [&str; 4] = ["Basic", "Positioning", "Advanced", "Comments"];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Positioning" => positioning(f, ui),
        "Advanced" => advanced(f, ui),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    egui::Grid::new(("uts-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Name");
        f.locstring(ui, "Name", "LocName");
        ui.end_row();
        crate::widgets::field_label(ui, "Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Sound);
        ui.end_row();
        crate::widgets::field_label(ui, "Volume");
        f.slider(ui, "Volume", "Volume", 0..=127);
        ui.end_row();
    });
    ui.separator();
    crate::widgets::field_label(ui, "List of Sounds to Play");
    sound_list(f, ui);
}

/// The Sounds list: add (picker), remove, move up and down.
fn sound_list(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let sounds: Vec<ResRef> = f
        .root
        .list("Sounds")
        .unwrap_or(&[])
        .iter()
        .map(|s| s.resref("Sound").unwrap_or(ResRef::EMPTY))
        .collect();
    let sel_id = egui::Id::new(("uts-selected", f.key));
    let mut selected: Option<usize> =
        ui.data(|d| d.get_temp(sel_id)).filter(|&i: &usize| i < sounds.len());
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_height(120.0);
        ui.set_min_width(260.0);
        for (i, s) in sounds.iter().enumerate() {
            if ui.selectable_label(selected == Some(i), s.to_string()).clicked() {
                selected = Some(i);
            }
        }
    });
    let key = f.key;
    let edit = |what: &str, edits: Vec<Edit>| Action::Apply(Command::new(what, edits));
    let item = |r: ResRef| {
        let mut s = Struct::new(0);
        s.set("Sound", Value::resref(r));
        s
    };
    ui.horizontal(|ui| {
        let pick_id = egui::Id::new(("uts-add", key));
        if let Some(r) = f.app.take_pick(pick_id).filter(|r| !r.is_empty()) {
            f.app.actions.push(edit(
                "Add sound",
                vec![Edit::InsertItem {
                    key,
                    path: base.clone(),
                    list: "Sounds".into(),
                    index: sounds.len(),
                    item: item(r),
                }],
            ));
        }
        if ui.button("Add Sounds…").clicked() {
            f.app.open_picker(pick_id, "Add a sound", &[ResType::WAV]);
        }
        let remove = |i: usize| Edit::RemoveItem {
            key,
            path: base.clone(),
            list: "Sounds".into(),
            index: i,
        };
        let insert = |i: usize, r: ResRef| Edit::InsertItem {
            key,
            path: base.clone(),
            list: "Sounds".into(),
            index: i,
            item: item(r),
        };
        if ui.add_enabled(selected.is_some(), egui::Button::new("Remove")).clicked()
            && let Some(i) = selected
        {
            f.app.actions.push(edit("Remove sound", vec![remove(i)]));
            selected = None;
        }
        let up = selected.is_some_and(|i| i > 0);
        if ui.add_enabled(up, egui::Button::new("Move Up")).clicked()
            && let Some(i) = selected
        {
            f.app.actions.push(edit("Move sound up", vec![remove(i), insert(i - 1, sounds[i])]));
            selected = Some(i - 1);
        }
        // Play the selected sound (else the first) at the blueprint's
        // volume; Stop.
        let volume = f.root.integer("Volume").unwrap_or(127).clamp(0, 127) as f32 / 127.0;
        let play = selected.or((!sounds.is_empty()).then_some(0));
        if ui
            .add_enabled(play.is_some(), egui::Button::new("Play"))
            .on_hover_text("Play the selected sound")
            .clicked()
            && let Some(i) = play
        {
            f.app.play_sound(Channel::Preview, sounds[i], volume, false);
        }
        if ui.button("Stop").clicked() {
            f.app.stop_sound(Channel::Preview);
        }
        let down = selected.is_some_and(|i| i + 1 < sounds.len());
        if ui.add_enabled(down, egui::Button::new("Move Down")).clicked()
            && let Some(i) = selected
        {
            f.app.actions.push(edit("Move sound down", vec![remove(i), insert(i + 1, sounds[i])]));
            selected = Some(i + 1);
        }
    });
    ui.data_mut(|d| match selected {
        Some(i) => {
            d.insert_temp(sel_id, i);
        }
        None => d.remove::<usize>(sel_id),
    });
}

fn positioning(f: &mut Form<'_>, ui: &mut Ui) {
    let positional = f.int("Positional") != 0;
    let random = f.int("RandomPosition") != 0;
    // (label, Positional, RandomPosition)
    let choices = [
        ("Plays everywhere in area", false, false),
        ("Plays from a random position each time it is played", true, true),
        ("Plays from a specific position", true, false),
    ];
    let looping = f.int("Looping") != 0;
    for (text, p, r) in choices {
        let on = positional == p && (!p || random == r);
        if ui.radio(on, text).clicked() && !on {
            // Aurora sets the priority group with it.
            let priority = i64::from(mg_module::blueprints::sound_priority(looping, p));
            f.set_many(
                "Positioning",
                &[
                    ("Positional", i64::from(p), FieldType::Byte),
                    ("RandomPosition", i64::from(r), FieldType::Byte),
                    ("Priority", priority, FieldType::Byte),
                ],
            );
        }
    }
    ui.separator();
    egui::Grid::new(("uts-pos", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.add_enabled_ui(positional, |ui| crate::widgets::field_label(ui, "Cutoff distance (m)"));
        ui.add_enabled_ui(positional, |ui| {
            f.float(ui, "Cutoff distance", "MaxDistance", 0.0..=32000.0, 0.5)
        });
        ui.end_row();
        ui.add_enabled_ui(positional, |ui| {
            crate::widgets::field_label(ui, "Max Volume Distance (m)")
        });
        ui.add_enabled_ui(positional, |ui| {
            f.float(ui, "Max volume distance", "MinDistance", 0.0..=32000.0, 0.5)
        });
        ui.end_row();
        let rand = positional && random;
        ui.add_enabled_ui(rand, |ui| crate::widgets::field_label(ui, "West-East Random Range (m)"));
        ui.add_enabled_ui(rand, |ui| {
            f.float(ui, "West-east random range", "RandomRangeX", 0.0..=32000.0, 0.5)
        });
        ui.end_row();
        ui.add_enabled_ui(rand, |ui| {
            crate::widgets::field_label(ui, "North-South Random Range (m)")
        });
        ui.add_enabled_ui(rand, |ui| {
            f.float(ui, "North-south random range", "RandomRangeY", 0.0..=32000.0, 0.5)
        });
        ui.end_row();
        crate::widgets::field_label(ui, "Height (m)");
        f.float(ui, "Height", "Elevation", -10.0..=10.0, 0.1);
        ui.end_row();
    });
}

const HOURS: [&str; 24] = [
    "12 AM", "1 AM", "2 AM", "3 AM", "4 AM", "5 AM", "6 AM", "7 AM", "8 AM", "9 AM", "10 AM",
    "11 AM", "12 PM", "1 PM", "2 PM", "3 PM", "4 PM", "5 PM", "6 PM", "7 PM", "8 PM", "9 PM",
    "10 PM", "11 PM",
];

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    egui::Grid::new(("uts-adv", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Blueprint ResRef");
        f.blueprint_resref(ui);
        ui.end_row();
        crate::widgets::field_label(ui, "Active");
        f.check(ui, "", "Active");
        ui.end_row();
        crate::widgets::field_label(ui, "Pitch Variation (octaves)");
        f.float(ui, "Pitch variation", "PitchVariation", 0.0..=1.0, 0.01);
        ui.end_row();
        crate::widgets::field_label(ui, "Volume Variation");
        f.slider(ui, "Volume variation", "VolumeVrtn", 0..=127);
        ui.end_row();
        crate::widgets::field_label(ui, "Variables");
        f.variables(ui);
        ui.end_row();
    });
    f.update_instances(ui);
    ui.separator();

    crate::widgets::field_label(ui, "When to play");
    let times = f.int("Times");
    ui.horizontal(|ui| {
        for (text, v) in [("Always", 3), ("Day", 1), ("Night", 2), ("Specific Hours", 0)] {
            if ui.radio(times == v, text).clicked() && times != v {
                f.set_int("When to play", "Times", v, FieldType::Byte);
            }
        }
    });
    let hours = f.int("Hours");
    ui.add_enabled_ui(times == 0, |ui| {
        egui::Grid::new(("uts-hours", f.key)).num_columns(6).show(ui, |ui| {
            for (bit, text) in HOURS.iter().enumerate() {
                let mut on = hours & (1 << bit) != 0;
                if ui.checkbox(&mut on, *text).changed() {
                    let v = if on { hours | (1 << bit) } else { hours & !(1 << bit) };
                    f.set_int("Hours", "Hours", v, FieldType::Dword);
                }
                if bit % 6 == 5 {
                    ui.end_row();
                }
            }
        });
    });
    ui.separator();

    let looping = f.int("Looping") != 0;
    let continuous = f.int("Continuous") != 0;
    let positional = f.int("Positional") != 0;
    let single = f.root.list("Sounds").is_some_and(|l| l.len() == 1);
    crate::widgets::field_label(ui, "Play Style");
    ui.horizontal(|ui| {
        // (label, Looping, Continuous), as Aurora writes them; seamless
        // looping takes a single sound, played in order.
        for (text, l, c) in mg_module::blueprints::SOUND_PLAY_STYLES {
            let on = if l { looping } else { !looping && continuous == c };
            let enabled = !l || single || looping;
            if ui
                .add_enabled(enabled, egui::RadioButton::new(on, text))
                .on_disabled_hover_text("Seamless looping plays a single sound")
                .clicked()
                && !on
            {
                let priority = i64::from(mg_module::blueprints::sound_priority(l, positional));
                let mut fields = vec![
                    ("Looping", i64::from(l), FieldType::Byte),
                    ("Continuous", i64::from(c), FieldType::Byte),
                    ("Priority", priority, FieldType::Byte),
                ];
                if l {
                    fields.push(("Random", 0, FieldType::Byte));
                }
                f.set_many("Play style", &fields);
            }
        }
    });
    crate::widgets::field_label(ui, "Play Order");
    let random = f.int("Random") != 0;
    ui.add_enabled_ui(!looping, |ui| {
        ui.horizontal(|ui| {
            for (text, v) in [("Sequential", false), ("Random", true)] {
                if ui.radio(random == v, text).clicked() && random != v {
                    f.set_int("Play order", "Random", i64::from(v), FieldType::Byte);
                }
            }
        });
    });
    ui.add_enabled_ui(!looping, |ui| {
        egui::Grid::new(("uts-interval", f.key)).num_columns(2).spacing([12.0, 6.0]).show(
            ui,
            |ui| {
                crate::widgets::field_label(ui, "Interval between playing sounds (seconds)");
                f.millis(ui, "Interval", "Interval", 0.0..=100.0);
                ui.end_row();
                crate::widgets::field_label(ui, "Interval Variation (seconds)");
                f.millis(ui, "Interval variation", "IntervalVrtn", 0.0..=100.0);
                ui.end_row();
            },
        );
    });
}
