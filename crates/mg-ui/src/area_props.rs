//! Area Properties (`TdlgAreaProperties`): Basic, Visual (lighting schemes,
//! and the environment Aurora edits in its Customize Environment dialog),
//! Audio (the GIT's `AreaProperties`), Events, Advanced and Comments.
//! Every change is an undoable command, as in the blueprint editors.

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{FieldType, Value};
use mg_resman::ResKey;
use mg_rules::{Choice, ChoiceColumns};

use crate::blueprint::Form;
use crate::{Action, Moonglow};

pub(crate) const PAGES: [&str; 6] = ["Basic", "Visual", "Audio", "Events", "Advanced", "Comments"];

/// ARE `Flags` bits.
const INTERIOR: u32 = 0x1;
const UNDERGROUND: u32 = 0x2;
const NATURAL: u32 = 0x4;

fn choices(app: &Moonglow, table: &str, name: &str, label: &str) -> Vec<Choice> {
    app.game
        .as_ref()
        .and_then(|g| g.choices(table, ChoiceColumns { name: Some(name), label: Some(label) }).ok())
        .unwrap_or_default()
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, area: ResRef) {
    let are = ResKey::new(area, ResType::ARE);
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return;
    };
    let root = match ws.doc(&are) {
        Ok(g) => g.root.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    let page_key = (are, GffPath::root());
    let mut page = app.blueprint_pages.get(&page_key).copied().unwrap_or(PAGES[0]);
    ui.horizontal_wrapped(|ui| {
        for p in PAGES {
            ui.selectable_value(&mut page, p, p);
        }
    });
    app.blueprint_pages.insert(page_key, page);
    ui.separator();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        if page == "Audio" {
            audio(app, ui, area);
            return;
        }
        let mut f = Form { app, key: are, path: GffPath::root(), also: Vec::new(), root };
        match page {
            "Basic" => basic(&mut f, ui),
            "Visual" => visual(&mut f, ui),
            "Events" => events(&mut f, ui),
            "Advanced" => advanced(&mut f, ui),
            _ => f.memo(ui, "Comments", "Comments"),
        }
    });
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let tileset = f.root.resref("Tileset").unwrap_or(ResRef::EMPTY);
    let tileset_name = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            let set = mg_area::tileset(g, tileset).ok()?;
            g.string(set.general.display_name).or(set.general.unlocalized_name)
        })
        .unwrap_or_else(|| tileset.to_string());
    let (w, h) = (f.int("Width"), f.int("Height"));
    egui::Grid::new(("are-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Name");
        f.locstring(ui, "Area name", "Name");
        ui.end_row();
        ui.label("Tileset");
        ui.label(format!("{tileset_name} ({tileset})"));
        ui.end_row();
        ui.label("Size");
        ui.label(format!("{w} by {h} tiles")).on_hover_text("Resize Area changes it");
        ui.end_row();
    });
}

fn visual(f: &mut Form<'_>, ui: &mut Ui) {
    let schemes = choices(f.app, "environment", "STRREF", "LABEL");
    let current = f.int("LightingScheme");
    ui.strong("Lighting Scheme");
    ui.weak(
        "Choosing a scheme replaces the area's lighting and weather settings and its tiles' \
         custom lighting.",
    );
    let mut picked = None;
    egui::ScrollArea::vertical().id_salt("schemes").max_height(160.0).show(ui, |ui| {
        for c in &schemes {
            if ui.selectable_label(c.row as i64 == current, &c.text).clicked() {
                picked = Some(c.row);
            }
        }
    });
    if let Some(row) = picked {
        apply_scheme(f, row);
    }
    ui.separator();
    ui.strong("Environment");
    let skyboxes = choices(f.app, "skyboxes", "STRING_REF", "LABEL");
    egui::Grid::new(("are-env", f.key)).num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("");
        ui.strong("Sun");
        ui.strong("Moon");
        ui.end_row();
        for (what, sun, moon) in [
            ("Ambient Color", "SunAmbientColor", "MoonAmbientColor"),
            ("Diffuse Color", "SunDiffuseColor", "MoonDiffuseColor"),
            ("Fog Color", "SunFogColor", "MoonFogColor"),
        ] {
            ui.label(what);
            f.color(ui, &format!("Sun {what}"), sun);
            f.color(ui, &format!("Moon {what}"), moon);
            ui.end_row();
        }
        ui.label("Fog Amount");
        f.number(ui, "Sun fog amount", "SunFogAmount", 0..=200);
        f.number(ui, "Moon fog amount", "MoonFogAmount", 0..=200);
        ui.end_row();
        ui.label("Shadows");
        f.check(ui, "", "SunShadows");
        f.check(ui, "", "MoonShadows");
        ui.end_row();
    });
    ui.add_space(6.0);
    // Day and night: cycling, always bright, always dark.
    let cycle = f.int("DayNightCycle") != 0;
    let night = f.int("IsNight") != 0;
    let mode = if cycle {
        0
    } else if night {
        2
    } else {
        1
    };
    ui.horizontal(|ui| {
        for (m, text) in [(0, "Cycle Day and Night"), (1, "Always Bright"), (2, "Always Dark")] {
            if ui.radio(mode == m, text).clicked() && mode != m {
                let (c, n) = match m {
                    0 => (1, i64::from(night)),
                    1 => (0, 0),
                    _ => (0, 1),
                };
                f.set_many(
                    "Day and night",
                    &[("DayNightCycle", c, FieldType::Byte), ("IsNight", n, FieldType::Byte)],
                );
            }
        }
    });
    egui::Grid::new(("are-env2", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Shadow Opacity");
        f.number(ui, "Shadow opacity", "ShadowOpacity", 0..=100);
        ui.end_row();
        ui.label("Fog Clip Distance (m)");
        f.float(ui, "Fog clip distance", "FogClipDist", 25.0..=999.0, 1.0);
        ui.end_row();
        ui.label("Sky Box");
        f.choice(ui, "Sky box", "SkyBox", &skyboxes, FieldType::Byte);
        ui.end_row();
        ui.label("% Rain");
        f.number(ui, "Chance of rain", "ChanceRain", 0..=100);
        ui.end_row();
        ui.label("% Snow");
        f.number(ui, "Chance of snow", "ChanceSnow", 0..=100);
        ui.end_row();
        ui.label("% Lightning");
        f.number(ui, "Chance of lightning", "ChanceLightning", 0..=100);
        ui.end_row();
        ui.label("Wind Power");
        let winds = ["None", "Weak", "Strong"];
        let wind = f.int("WindPower").clamp(0, 2);
        ui.horizontal(|ui| {
            for (i, w) in winds.iter().enumerate() {
                if ui.radio(wind == i as i64, *w).clicked() && wind != i as i64 {
                    f.set_int("Wind power", "WindPower", i as i64, FieldType::Int);
                }
            }
        });
        ui.end_row();
    });
}

/// Applies an environment.2da scheme: the area's lighting and weather, and
/// each tile's lights picked anew from the scheme's colours (one command).
fn apply_scheme(f: &mut Form<'_>, row: usize) {
    let Some(game) = f.app.game.as_ref() else { return };
    let scheme = match mg_module::new::Scheme::read(game, row) {
        Ok(s) => s,
        Err(e) => {
            f.app.log.error(e.to_string());
            return;
        }
    };
    let key = f.key;
    let mut edits: Vec<Edit> = scheme
        .fields()
        .into_iter()
        .map(|(label, value)| Edit::SetField {
            key,
            path: GffPath::root(),
            label: label.into(),
            value: Some(value),
        })
        .collect();
    let mut rng = fastrand::Rng::new();
    for i in 0..f.root.list("Tile_List").map_or(0, <[_]>::len) {
        let [main1, main2, source] = scheme.tile_lights(&mut rng);
        for (label, v) in [
            ("Tile_MainLight1", main1),
            ("Tile_MainLight2", main2),
            ("Tile_SrcLight1", source),
            ("Tile_SrcLight2", source),
        ] {
            edits.push(Edit::SetField {
                key,
                path: GffPath::root().item("Tile_List", i),
                label: label.into(),
                value: Some(Value::Byte(v)),
            });
        }
    }
    f.app.actions.push(Action::Apply(Command::new("Lighting scheme", edits)));
}

fn audio(app: &mut Moonglow, ui: &mut Ui, area: ResRef) {
    let git = ResKey::new(area, ResType::GIT);
    let path = GffPath::root().field("AreaProperties");
    let Some(root) =
        app.ws.as_mut().and_then(|ws| ws.doc(&git).ok()).and_then(|g| path.get(&g.root)).cloned()
    else {
        ui.label("This area has no audio settings (its GIT lacks AreaProperties).");
        return;
    };
    let sounds = choices(app, "ambientsound", "Description", "Resource");
    let music = choices(app, "ambientmusic", "Description", "Resource");
    let eax = choices(app, "soundeax", "Description", "Label");
    let mut f = Form { app, key: git, path, also: Vec::new(), root };
    egui::Grid::new(("are-audio", git)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for (text, label, list) in [
            ("Ambient Sound, Day", "AmbientSndDay", &sounds),
            ("Ambient Sound, Night", "AmbientSndNight", &sounds),
        ] {
            ui.label(text);
            f.choice(ui, text, label, list, FieldType::Int);
            ui.end_row();
        }
        ui.label("Ambient Sound, Day Volume");
        f.slider(ui, "Day volume", "AmbientSndDayVol", 0..=127);
        ui.end_row();
        ui.label("Ambient Sound, Night Volume");
        f.slider(ui, "Night volume", "AmbientSndNitVol", 0..=127);
        ui.end_row();
        for (text, label) in [
            ("Music, Day", "MusicDay"),
            ("Music, Night", "MusicNight"),
            ("Music, Battle", "MusicBattle"),
        ] {
            ui.label(text);
            f.choice(ui, text, label, &music, FieldType::Int);
            ui.end_row();
        }
        ui.label("Music, Playing Delay");
        f.number(ui, "Music delay", "MusicDelay", -99..=99);
        ui.end_row();
        ui.label("Environmental Audio Effects");
        f.choice(ui, "Environmental audio", "EnvAudio", &eax, FieldType::Int);
        ui.end_row();
    });
}

fn events(f: &mut Form<'_>, ui: &mut Ui) {
    egui::Grid::new(("are-events", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for label in ["OnEnter", "OnExit", "OnHeartbeat", "OnUserDefined"] {
            ui.label(label);
            f.script(ui, label, label);
            ui.end_row();
        }
    });
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let pvp = choices(f.app, "pvpsettings", "strref", "label");
    let screens = choices(f.app, "loadscreens", "StrRef", "Label");
    let flags = f.int("Flags") as u32;
    egui::Grid::new(("are-advanced", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        ui.label("ResRef");
        ui.label(f.key.resref.to_string());
        ui.end_row();
        ui.label("Check Modifier - Listen");
        f.number(ui, "Listen check modifier", "ModListenCheck", -99..=99);
        ui.end_row();
        ui.label("Check Modifier - Spot");
        f.number(ui, "Spot check modifier", "ModSpotCheck", -99..=99);
        ui.end_row();
        ui.label("Player Vs. Player");
        f.choice(ui, "Player vs. player", "PlayerVsPlayer", &pvp, FieldType::Byte);
        ui.end_row();
        ui.label("Loading Screen");
        f.choice(ui, "Loading screen", "LoadScreenID", &screens, FieldType::Word);
        ui.end_row();
        ui.label("");
        f.check(ui, "No Rest", "NoRest");
        ui.end_row();
        for (bit, off, on) in [
            (INTERIOR, "Exterior", "Interior (no weather effects)"),
            (NATURAL, "Artificial", "Natural"),
            (UNDERGROUND, "Above ground", "Underground"),
        ] {
            ui.label("");
            ui.horizontal(|ui| {
                let set = flags & bit != 0;
                for (want, text) in [(false, off), (true, on)] {
                    if ui.radio(set == want, text).clicked() && set != want {
                        let v = if want { flags | bit } else { flags & !bit };
                        f.set_int(text, "Flags", i64::from(v), FieldType::Dword);
                    }
                }
            });
            ui.end_row();
        }
        ui.label("Variables");
        f.variables(ui);
        ui.end_row();
    });
}
