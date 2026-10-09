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

/// The pages for several areas edited together: what is an area's own
/// (its name, tag and comments) is edited one area at a time.
const SHARED_PAGES: [&str; 4] = ["Visual", "Audio", "Events", "Advanced"];

/// Area Properties of `area`; with `others`, of them all (Edit Areas
/// Together): shown as `area`'s, and what is changed is set on each.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, area: ResRef, others: &[ResRef]) {
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
    // (Areas deleted since are left out.)
    let others: Vec<ResRef> = others
        .iter()
        .copied()
        .filter(|a| ws.module.contains(&ResKey::new(*a, ResType::ARE)))
        .collect();
    let others = others.as_slice();
    let several = !others.is_empty();
    let pages: &[&'static str] = if several { &SHARED_PAGES } else { &PAGES };
    // (Several areas keep a page of their own: a path no document has.)
    let page_key = (are, if several { GffPath::root().field("several") } else { GffPath::root() });
    let mut page = app.blueprint_pages.get(&page_key).copied().unwrap_or(pages[0]);
    if !pages.contains(&page) {
        page = pages[0];
    }
    ui.horizontal_wrapped(|ui| {
        for p in pages {
            ui.selectable_value(&mut page, *p, *p);
        }
    });
    app.blueprint_pages.insert(page_key, page);
    ui.separator();
    if several {
        let by_name = app.settings.area_names;
        let names: Vec<String> = std::iter::once(&area)
            .chain(others)
            .map(|a| match &app.ws {
                Some(ws) => app.area_names.label(ws, app.game.as_deref(), *a, by_name),
                None => a.to_string(),
            })
            .collect();
        ui.weak(format!(
            "{} areas: shown as the first ({}); what you change is set on each (≠: they differ there)",
            names.len(),
            names[0]
        ))
        .on_hover_text(names.join("\n"));
        ui.weak("Names, tags and comments are edited one area at a time.");
        // What the page shows is the first area's, and only what is changed
        // is set on the others: this gives them the rest of it too.
        if ui
            .button(format!("Give Every Area This Page's {page} Settings"))
            .on_hover_text(format!(
                "Sets what this page shows ({}'s) on each of the other areas, changed here or not",
                names[0]
            ))
            .clicked()
        {
            give_page(app, area, others, page);
        }
    }
    let also: Vec<(ResKey, GffPath)> =
        others.iter().map(|a| (ResKey::new(*a, ResType::ARE), GffPath::root())).collect();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        if page == "Audio" {
            audio(app, ui, area, others);
            return;
        }
        let mut f = Form { app, key: are, path: GffPath::root(), also, root };
        match page {
            "Basic" => basic(&mut f, ui),
            "Visual" => visual(&mut f, ui),
            "Events" => events(&mut f, ui),
            "Advanced" => advanced(&mut f, ui),
            _ => f.memo(ui, "Comments", "Comments"),
        }
    });
}

/// The fields of a page shared by several areas: in the area (`false`),
/// or in its GIT's `AreaProperties` (`true`).
fn page_fields(page: &str) -> (bool, &'static [&'static str]) {
    match page {
        "Visual" => (
            false,
            &[
                "ChanceLightning",
                "ChanceRain",
                "ChanceSnow",
                "DayNightCycle",
                "IsNight",
                "LightingScheme",
                "MoonAmbientColor",
                "MoonDiffuseColor",
                "MoonFogAmount",
                "MoonFogColor",
                "MoonShadows",
                "SunAmbientColor",
                "SunDiffuseColor",
                "SunFogAmount",
                "SunFogColor",
                "SunShadows",
                "ShadowOpacity",
                "SkyBox",
                "WindPower",
                "FogClipDist",
            ],
        ),
        "Audio" => (
            true,
            &[
                "AmbientSndDay",
                "AmbientSndDayVol",
                "AmbientSndNight",
                "AmbientSndNitVol",
                "EnvAudio",
                "MusicBattle",
                "MusicDay",
                "MusicDelay",
                "MusicNight",
            ],
        ),
        "Events" => (false, &["OnEnter", "OnExit", "OnHeartbeat", "OnUserDefined"]),
        "Advanced" => (
            false,
            &[
                "Flags",
                "ModListenCheck",
                "ModSpotCheck",
                "NoRest",
                "PlayerVsPlayer",
                "LoadScreenID",
            ],
        ),
        _ => (false, &[]),
    }
}

/// Gives the other areas the first one's values of a page's fields (those
/// it has), in one command.
fn give_page(app: &mut Moonglow, area: ResRef, others: &[ResRef], page: &str) {
    let (in_git, labels) = page_fields(page);
    let (restype, path) = if in_git {
        (ResType::GIT, GffPath::root().field("AreaProperties"))
    } else {
        (ResType::ARE, GffPath::root())
    };
    let Some(ws) = app.ws.as_mut() else { return };
    let values: Vec<(&str, Value)> = match ws.doc(&ResKey::new(area, restype)) {
        Ok(doc) => match path.get(&doc.root) {
            Some(from) => labels.iter().filter_map(|l| Some((*l, from.get(l)?.clone()))).collect(),
            None => return,
        },
        Err(_) => return,
    };
    let mut edits = Vec::new();
    for other in others {
        let key = ResKey::new(*other, restype);
        let there = ws.doc(&key).ok().is_some_and(|d| path.get(&d.root).is_some());
        if !there {
            continue;
        }
        edits.extend(values.iter().map(|(label, value)| Edit::SetField {
            key,
            path: path.clone(),
            label: (*label).into(),
            value: Some(value.clone()),
        }));
    }
    if !edits.is_empty() {
        let what = format!("{page} settings to {} areas", others.len());
        app.actions.push(Action::Apply(Command::new(what, edits)));
    }
}

/// A tileset's name (read once), else its ResRef.
fn tileset_name(app: &mut Moonglow, tileset: ResRef) -> String {
    if let Some(name) = app.palette.tileset_names.get(&tileset) {
        return name.clone();
    }
    let name = app
        .game
        .as_ref()
        .and_then(|g| {
            let set = mg_area::tileset(g, tileset).ok()?;
            g.string(set.general.display_name).or(set.general.unlocalized_name)
        })
        .unwrap_or_else(|| tileset.to_string());
    app.palette.tileset_names.insert(tileset, name.clone());
    name
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let tileset = f.root.resref("Tileset").unwrap_or(ResRef::EMPTY);
    let tileset_name = tileset_name(f.app, tileset);
    let (w, h) = (f.int("Width"), f.int("Height"));
    egui::Grid::new(("are-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Name");
        f.locstring(ui, "Area name", "Name");
        ui.end_row();
        crate::widgets::field_label(ui, "Tileset");
        ui.label(format!("{tileset_name} ({tileset})"));
        ui.end_row();
        crate::widgets::field_label(ui, "Size");
        ui.label(format!("{w} by {h} tiles")).on_hover_text("Resize Area changes it");
        ui.end_row();
    });
}

fn visual(f: &mut Form<'_>, ui: &mut Ui) {
    let schemes = choices(f.app, "environment", "STRREF", "LABEL");
    let current = f.int("LightingScheme");
    crate::widgets::section_heading(ui, "Lighting Scheme");
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
    ui.add_space(crate::widgets::SECTION_GAP);
    crate::widgets::section_heading(ui, "Environment");
    let skyboxes = choices(f.app, "skyboxes", "STRING_REF", "LABEL");
    egui::Grid::new(("are-env", f.key)).num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "");
        ui.strong("Sun");
        ui.strong("Moon");
        ui.end_row();
        for (what, sun, moon) in [
            ("Ambient Color", "SunAmbientColor", "MoonAmbientColor"),
            ("Diffuse Color", "SunDiffuseColor", "MoonDiffuseColor"),
            ("Fog Color", "SunFogColor", "MoonFogColor"),
        ] {
            crate::widgets::field_label(ui, what);
            f.color(ui, &format!("Sun {what}"), sun);
            f.color(ui, &format!("Moon {what}"), moon);
            ui.end_row();
        }
        crate::widgets::field_label(ui, "Fog Amount");
        f.number(ui, "Sun fog amount", "SunFogAmount", 0..=200);
        f.number(ui, "Moon fog amount", "MoonFogAmount", 0..=200);
        ui.end_row();
        crate::widgets::field_label(ui, "Shadows");
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
        crate::widgets::field_label(ui, "Shadow Opacity");
        f.number(ui, "Shadow opacity", "ShadowOpacity", 0..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "Fog Clip Distance (m)");
        f.float(ui, "Fog clip distance", "FogClipDist", 25.0..=999.0, 1.0);
        ui.end_row();
        crate::widgets::field_label(ui, "Sky Box");
        f.choice(ui, "Sky box", "SkyBox", &skyboxes, FieldType::Byte);
        ui.end_row();
        crate::widgets::field_label(ui, "% Rain");
        f.number(ui, "Chance of rain", "ChanceRain", 0..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "% Snow");
        f.number(ui, "Chance of snow", "ChanceSnow", 0..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "% Lightning");
        f.number(ui, "Chance of lightning", "ChanceLightning", 0..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "Wind Power");
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
    let Some(game) = f.app.game.as_deref() else { return };
    let scheme = match mg_module::new::Scheme::read(game, row) {
        Ok(s) => s,
        Err(e) => {
            f.app.log.error(e.to_string());
            return;
        }
    };
    // Each area edited (one, or several together): its fields and its own
    // tiles.
    let keys: Vec<ResKey> = std::iter::once(f.key).chain(f.also.iter().map(|(k, _)| *k)).collect();
    let mut rng = fastrand::Rng::new();
    let mut edits: Vec<Edit> = Vec::new();
    for key in keys {
        let tiles = match f.app.ws.as_mut().map(|ws| ws.doc(&key)) {
            Some(Ok(are)) => are.root.list("Tile_List").map_or(0, <[_]>::len),
            _ => continue,
        };
        edits.extend(scheme.fields().into_iter().map(|(label, value)| Edit::SetField {
            key,
            path: GffPath::root(),
            label: label.into(),
            value: Some(value),
        }));
        for i in 0..tiles {
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
    }
    f.app.actions.push(Action::Apply(Command::new("Lighting scheme", edits)));
}

fn audio(app: &mut Moonglow, ui: &mut Ui, area: ResRef, others: &[ResRef]) {
    let git = ResKey::new(area, ResType::GIT);
    let path = GffPath::root().field("AreaProperties");
    // The other areas that have audio settings.
    let also: Vec<(ResKey, GffPath)> = others
        .iter()
        .map(|a| ResKey::new(*a, ResType::GIT))
        .filter(|g| {
            let doc = app.ws.as_mut().and_then(|ws| ws.doc(g).ok());
            doc.is_some_and(|d| path.get(&d.root).is_some())
        })
        .map(|g| (g, path.clone()))
        .collect();
    let Some(root) =
        app.ws.as_mut().and_then(|ws| ws.doc(&git).ok()).and_then(|g| path.get(&g.root)).cloned()
    else {
        ui.label("This area has no audio settings (its GIT lacks AreaProperties).");
        return;
    };
    let sounds = choices(app, "ambientsound", "Description", "Resource");
    let music = choices(app, "ambientmusic", "Description", "Resource");
    let eax = choices(app, "soundeax", "Description", "Label");
    let mut f = Form { app, key: git, path, also, root };
    egui::Grid::new(("are-audio", git)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for (text, label, list) in [
            ("Ambient Sound, Day", "AmbientSndDay", &sounds),
            ("Ambient Sound, Night", "AmbientSndNight", &sounds),
        ] {
            crate::widgets::field_label(ui, text);
            f.choice(ui, text, label, list, FieldType::Int);
            ui.end_row();
        }
        crate::widgets::field_label(ui, "Ambient Sound, Day Volume");
        f.slider(ui, "Day volume", "AmbientSndDayVol", 0..=127);
        ui.end_row();
        crate::widgets::field_label(ui, "Ambient Sound, Night Volume");
        f.slider(ui, "Night volume", "AmbientSndNitVol", 0..=127);
        ui.end_row();
        for (text, label) in [
            ("Music, Day", "MusicDay"),
            ("Music, Night", "MusicNight"),
            ("Music, Battle", "MusicBattle"),
        ] {
            crate::widgets::field_label(ui, text);
            f.choice(ui, text, label, &music, FieldType::Int);
            ui.end_row();
        }
        crate::widgets::field_label(ui, "Music, Playing Delay");
        f.number(ui, "Music delay", "MusicDelay", -99..=99);
        ui.end_row();
        crate::widgets::field_label(ui, "Environmental Audio Effects");
        f.choice(ui, "Environmental audio", "EnvAudio", &eax, FieldType::Int);
        ui.end_row();
    });
}

fn events(f: &mut Form<'_>, ui: &mut Ui) {
    let events = ["OnEnter", "OnExit", "OnHeartbeat", "OnUserDefined"].map(|l| (l, l));
    egui::Grid::new(("are-events", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for (label, field) in events {
            crate::widgets::field_label(ui, label);
            f.script(ui, label, field);
            ui.end_row();
        }
    });
    f.script_set_buttons(ui, &events);
}

/// Sets or clears one bit of `Flags` on each area edited, each keeping its
/// other flags (one command).
fn set_flag(f: &mut Form<'_>, what: &str, bit: u32, on: bool) {
    let keys: Vec<ResKey> = std::iter::once(f.key).chain(f.also.iter().map(|(k, _)| *k)).collect();
    let mut edits = Vec::new();
    for key in keys {
        let Some(Ok(are)) = f.app.ws.as_mut().map(|ws| ws.doc(&key)) else { continue };
        let flags = are.root.integer("Flags").unwrap_or(0) as u32;
        let changed = flag_set(flags, bit, on);
        if changed != flags {
            edits.push(Edit::SetField {
                key,
                path: GffPath::root(),
                label: "Flags".into(),
                value: Some(Value::Dword(changed)),
            });
        }
    }
    if !edits.is_empty() {
        f.app.actions.push(Action::Apply(Command::new(what, edits)));
    }
}

/// `flags` with `bit` set or cleared.
fn flag_set(flags: u32, bit: u32, on: bool) -> u32 {
    if on { flags | bit } else { flags & !bit }
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let pvp = choices(f.app, "pvpsettings", "strref", "label");
    let screens = choices(f.app, "loadscreens", "StrRef", "Label");
    let flags = f.int("Flags") as u32;
    egui::Grid::new(("are-advanced", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        if f.also.is_empty() {
            crate::widgets::field_label(ui, "Tag");
            f.text(ui, "Tag", "Tag", 32);
            ui.end_row();
            crate::widgets::field_label(ui, "ResRef");
            ui.label(f.key.resref.to_string());
            ui.end_row();
        }
        crate::widgets::field_label(ui, "Check Modifier - Listen");
        f.number(ui, "Listen check modifier", "ModListenCheck", -99..=99);
        ui.end_row();
        crate::widgets::field_label(ui, "Check Modifier - Spot");
        f.number(ui, "Spot check modifier", "ModSpotCheck", -99..=99);
        ui.end_row();
        crate::widgets::field_label(ui, "Player Vs. Player");
        f.choice(ui, "Player vs. player", "PlayerVsPlayer", &pvp, FieldType::Byte);
        ui.end_row();
        crate::widgets::field_label(ui, "Loading Screen");
        f.load_screen(ui, &screens);
        ui.end_row();
        crate::widgets::field_label(ui, "");
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
                        set_flag(f, text, bit, want);
                    }
                }
            });
            ui.end_row();
        }
        // The flags past the three, for custom shaders (their `areaFlags`
        // uniform): kept by the game (`engine_ee_fields.rs`).
        crate::widgets::field_label(ui, "Shader Flags").on_hover_text(
            "Area flags past the three above, for custom shaders: a shader reads them \
             from its areaFlags uniform",
        );
        // Rows of their own (a wrapped row would run over the next one: the
        // form's grid sizes its rows before the wrap).
        ui.vertical(|ui| {
            egui::Grid::new("area-shader-flags").num_columns(7).show(ui, |ui| {
                for (k, bit) in (3..16).map(|b| 1u32 << b).enumerate() {
                    let mut on = flags & bit != 0;
                    if ui.checkbox(&mut on, bit.to_string()).changed() {
                        set_flag(f, "Shader flags", bit, on);
                    }
                    if k % 7 == 6 {
                        ui.end_row();
                    }
                }
            });
            let higher = flags & !0xFFFF;
            if higher != 0 {
                ui.weak(format!("and {higher:#x}"));
            }
        });
        ui.end_row();
        crate::widgets::field_label(ui, "Variables");
        f.variables(ui);
        ui.end_row();
    });
}

/// The Edit Areas Together window: the areas ticked, and the filters that
/// narrow the list they are ticked in.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AreaChooser {
    /// Text in an area's name or ResRef.
    pub filter: String,
    pub tileset: Option<ResRef>,
    /// Interior, underground, natural: must be so, must not, or either.
    pub flags: [Option<bool>; 3],
    pub chosen: std::collections::BTreeSet<ResRef>,
}

/// The flags the chooser filters by, as its list reads.
const FLAG_FILTERS: [(u32, &str, &str, &str); 3] = [
    (INTERIOR, "Interior or exterior", "Interior", "Exterior"),
    (UNDERGROUND, "Above or under ground", "Underground", "Above ground"),
    (NATURAL, "Natural or artificial", "Natural", "Artificial"),
];

impl AreaChooser {
    /// Starting from `area`, ticked.
    pub(crate) fn with(area: ResRef) -> AreaChooser {
        AreaChooser { chosen: [area].into(), ..Default::default() }
    }

    /// Whether an area passes the filters.
    pub(crate) fn shows(&self, area: ResRef, info: &crate::tree::AreaInfo) -> bool {
        let text = self.filter.trim().to_lowercase();
        (text.is_empty()
            || area.to_string().contains(&text)
            || info.name.to_lowercase().contains(&text))
            && self.tileset.is_none_or(|t| t == info.tileset)
            && FLAG_FILTERS
                .iter()
                .zip(self.flags)
                .all(|((bit, ..), want)| want.is_none_or(|w| (info.flags & bit != 0) == w))
    }
}

/// Edit › Edit Areas Together…: a checklist of the module's areas, narrowed
/// by name, tileset and kind; the areas ticked open in one Area Properties.
pub(crate) fn chooser_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut c) = app.area_chooser.take() else { return };
    let Some(ws) = &app.ws else { return };
    let mut areas: Vec<(ResRef, crate::tree::AreaInfo)> = ws
        .module
        .keys_of(ResType::ARE)
        .map(|k| (k.resref, app.area_names.info(ws, app.game.as_deref(), k.resref)))
        .collect();
    // In the module tree's order: by name when it lists names.
    let by_name = app.settings.area_names;
    let label = |a: ResRef, i: &crate::tree::AreaInfo| match (i.name.trim(), by_name) {
        ("", _) => a.to_string(),
        (name, true) => format!("{name} ({a})"),
        (name, false) => format!("{a} ({name})"),
    };
    areas.sort_by_cached_key(|(a, i)| (label(*a, i).to_lowercase(), *a));
    // Areas that went away are no longer chosen.
    c.chosen.retain(|a| areas.iter().any(|(b, _)| a == b));
    let mut tilesets: Vec<ResRef> = areas.iter().map(|(_, i)| i.tileset).collect();
    tilesets.sort();
    tilesets.dedup();
    let tilesets: Vec<(ResRef, String)> =
        tilesets.into_iter().map(|t| (t, tileset_name(app, t))).collect();
    let tileset_label = |t: ResRef| {
        let name = tilesets.iter().find(|(r, _)| *r == t).map_or("", |(_, n)| n.as_str());
        if name.is_empty() || name == t.to_string() {
            t.to_string()
        } else {
            format!("{name} ({t})")
        }
    };
    let (mut open, mut edit, mut cancel) = (true, false, false);
    egui::Window::new("Edit Areas Together")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_size([460.0, 520.0])
        .collapsible(false)
        .open(crate::widgets::open_unless_escape(ctx, "Edit Areas Together", &mut open))
        .show(ctx, |ui| {
            ui.label(
                "Tick the areas to edit; their Area Properties open as one, and what you \
                 change there (lighting, fog, weather, music, scripts, variables…) is set on \
                 each.",
            );
            egui::Grid::new("area-chooser-filters").num_columns(2).spacing([12.0, 6.0]).show(
                ui,
                |ui| {
                    crate::widgets::field_label(ui, "Filter");
                    ui.add(
                        egui::TextEdit::singleline(&mut c.filter)
                            .hint_text("Name or ResRef")
                            .desired_width(260.0),
                    );
                    ui.end_row();
                    crate::widgets::field_label(ui, "Tileset");
                    egui::ComboBox::from_id_salt("area-chooser-tileset")
                        .width(260.0)
                        .selected_text(c.tileset.map_or("Any".to_string(), tileset_label))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut c.tileset, None, "Any");
                            for (t, _) in &tilesets {
                                ui.selectable_value(&mut c.tileset, Some(*t), tileset_label(*t));
                            }
                        });
                    ui.end_row();
                    for (k, (_, either, yes, no)) in FLAG_FILTERS.iter().enumerate() {
                        crate::widgets::field_label(ui, *yes);
                        let text = |v: Option<bool>| match v {
                            None => *either,
                            Some(true) => *yes,
                            Some(false) => *no,
                        };
                        egui::ComboBox::from_id_salt(("area-chooser-flag", k))
                            .width(260.0)
                            .selected_text(text(c.flags[k]))
                            .show_ui(ui, |ui| {
                                for v in [None, Some(true), Some(false)] {
                                    ui.selectable_value(&mut c.flags[k], v, text(v));
                                }
                            });
                        ui.end_row();
                    }
                },
            );
            let shown: Vec<&(ResRef, crate::tree::AreaInfo)> =
                areas.iter().filter(|(a, i)| c.shows(*a, i)).collect();
            ui.horizontal(|ui| {
                if ui.button("Tick Shown").clicked() {
                    c.chosen.extend(shown.iter().map(|(a, _)| *a));
                }
                if ui.button("Untick Shown").clicked() {
                    for (a, _) in &shown {
                        c.chosen.remove(a);
                    }
                }
                if ui.button("Untick All").clicked() {
                    c.chosen.clear();
                }
                ui.weak(format!("{} of {} areas shown", shown.len(), areas.len()));
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height(320.0).auto_shrink([false, false]).show(
                ui,
                |ui| {
                    for (a, info) in shown {
                        let mut on = c.chosen.contains(a);
                        if ui.checkbox(&mut on, label(*a, info)).changed() {
                            if on {
                                c.chosen.insert(*a);
                            } else {
                                c.chosen.remove(a);
                            }
                        }
                    }
                },
            );
            ui.separator();
            ui.horizontal(|ui| {
                let n = c.chosen.len();
                let button = egui::Button::new(format!("Edit {n} Together"));
                edit = ui
                    .add_enabled(n >= 2, button)
                    .on_disabled_hover_text("Tick two areas or more")
                    .clicked();
                cancel = crate::widgets::cancel(ui);
            });
        });
    if edit {
        let areas = c.chosen.iter().copied().collect();
        app.actions.push(Action::OpenTab(crate::Tab::AreasProperties(areas)));
    } else if open && !cancel {
        app.area_chooser = Some(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::AreaInfo;

    #[test]
    fn the_chooser_filters_by_text_tileset_and_kind() {
        let r = |s: &str| ResRef::from_str(s).unwrap();
        let cave = AreaInfo {
            name: "Wolf Cave".into(),
            tileset: r("tdc01"),
            flags: INTERIOR | UNDERGROUND | NATURAL,
        };
        let inn = AreaInfo { name: "Apple Inn".into(), tileset: r("tin01"), flags: INTERIOR };
        let all = AreaChooser::default();
        assert!(all.shows(r("area467"), &cave) && all.shows(r("area002"), &inn));
        // A name or a ResRef, whatever its case.
        let text = |t: &str| AreaChooser { filter: t.into(), ..Default::default() };
        assert!(text("wolf").shows(r("area467"), &cave) && !text("wolf").shows(r("area002"), &inn));
        assert!(text(" A467 ").shows(r("area467"), &cave));
        // Underground areas; areas above ground.
        let under = |v| AreaChooser { flags: [None, Some(v), None], ..Default::default() };
        assert!(under(true).shows(r("a"), &cave) && !under(true).shows(r("b"), &inn));
        assert!(!under(false).shows(r("a"), &cave) && under(false).shows(r("b"), &inn));
        // A tileset, with the rest.
        let set = AreaChooser { tileset: Some(r("tin01")), ..under(false) };
        assert!(set.shows(r("b"), &inn) && !set.shows(r("a"), &cave));
    }

    #[test]
    fn a_flag_changes_alone() {
        assert_eq!(flag_set(INTERIOR | 0x100, UNDERGROUND, true), INTERIOR | UNDERGROUND | 0x100);
        assert_eq!(flag_set(INTERIOR | UNDERGROUND, INTERIOR, false), UNDERGROUND);
        assert_eq!(flag_set(NATURAL, NATURAL, true), NATURAL);
    }
}
