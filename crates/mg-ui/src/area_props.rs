//! Area Properties (`TdlgAreaProperties`): Basic, Visual (lighting schemes,
//! and the environment Aurora edits in its Customize Environment dialog),
//! Grass (Moonglow's: `crate::area_grass`), Audio (the GIT's
//! `AreaProperties`), Events, Advanced and Comments.
//! Every change is an undoable command, as in the blueprint editors.

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{FieldType, Value};
use mg_resman::ResKey;
use mg_uikit::Controls as _;

use crate::blueprint::Form;
use crate::{Action, Moonglow};
use mg_module::areas::Has;
use mg_module::lighting::TileColors;

pub(crate) const PAGES: [&str; 8] =
    ["Basic", "Visual", "Grass", "Audio", "Events", "Advanced", "Variables", "Comments"];

/// ARE `Flags` bits.
const INTERIOR: u32 = 0x1;
const UNDERGROUND: u32 = 0x2;
const NATURAL: u32 = 0x4;

/// The pages for several areas edited together: what is an area's own
/// (its name, tag and comments) is edited one area at a time.
/// The name of the row of Area Properties' pages.
pub(crate) const PAGES_ROW: &str = "Area Properties pages";

const SHARED_PAGES: [&str; 6] = ["Visual", "Grass", "Audio", "Events", "Advanced", "Variables"];

/// Area Properties of `area`; with `others`, of them all (Edit Areas
/// Together): shown as `area`'s, and what is changed is set on each;
/// under its header with OK and Cancel at its right end
/// (`blueprint::ok_cancel_header`).
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, area: ResRef, others: &[ResRef]) {
    let areas = || std::iter::once(area).chain(others.iter().copied());
    let targets: Vec<(ResKey, GffPath)> = areas()
        .flat_map(|a| [ResType::ARE, ResType::GIT].map(|t| (ResKey::new(a, t), GffPath::root())))
        .collect();
    // (An area another builder holds, in a project that keeps those
    // read-only: its Properties are looked at, gray.)
    let held = areas().find_map(|a| crate::claims::read_only(app, a));
    let viewed = held.is_some();
    let pressed = match app.ws.is_some() {
        true => {
            let by_name = app.settings.area_names;
            let name = match &app.ws {
                Some(ws) => app.area_names.label(ws, app.game.as_deref(), area, by_name),
                None => area.to_string(),
            };
            let sub = match others {
                [] => area.to_string(),
                _ => format!("{area} and {} more", others.len()),
            };
            let title = crate::blueprint::Title {
                header: "Area Properties header".into(),
                icon: mg_uikit::Icon::AreaProperties,
                name: &name,
                sub: if name == sub { "" } else { &sub },
            };
            crate::blueprint::ok_cancel_header(ui, &title, viewed, crate::blueprint::OK_CANCEL_TIPS)
        }
        false => None,
    };
    let pending = app.actions.len();
    crate::claims::banner(app, ui, area);
    let refused_id = egui::Id::new(("area-refused", area));
    if let Some(why) = &held {
        let now = ui.input(|i| i.time);
        let refused = ui
            .data(|d| d.get_temp::<f64>(refused_id))
            .is_some_and(|at| now - at < crate::blueprint::VIEW_REFUSED_SHOWN);
        let text = if refused {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            egui::RichText::new(format!(
                "Not changed: {why}, and the project keeps others' areas read-only."
            ))
            .strong()
        } else {
            egui::RichText::new(format!("{why}: read-only here, shown to look at."))
        };
        ui.colored_label(ui.visuals().warn_fg_color, text);
    }
    if viewed {
        crate::blueprint::typing_kept_out(ui, ui.available_rect_before_wrap());
    }
    let shown = pages(app, ui, area, others, viewed);
    if shown.is_some() && viewed {
        let keys: Vec<ResKey> = targets.iter().map(|(k, _)| *k).collect();
        if crate::blueprint::look_only(app, pending, &|k| keys.contains(k)) {
            let now = ui.input(|i| i.time);
            ui.data_mut(|d| d.insert_temp(refused_id, now));
            ui.ctx().request_repaint();
        }
    }
    let tab = match others {
        [] => crate::Tab::AreaProperties(area),
        _ => crate::Tab::AreasProperties(areas().collect()),
    };
    crate::blueprint::window_drawn(app, pending, &tab, &targets, pressed);
}

/// The window's pages (only looked at, `viewed`): where they were drawn.
fn pages(
    app: &mut Moonglow,
    ui: &mut Ui,
    area: ResRef,
    others: &[ResRef],
    viewed: bool,
) -> Option<egui::Rect> {
    let are = ResKey::new(area, ResType::ARE);
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return None;
    };
    // (The area's fields without its tiles, which no page shows: copied
    // each frame, a large area's thousand tiles were most of the copy.)
    let root = match ws.doc(&are) {
        Ok(g) => mg_gff::Struct {
            id: g.root.id,
            fields: (g.root.fields.iter())
                .filter(|f| f.label.as_bytes() != b"Tile_List")
                .cloned()
                .collect(),
        },
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return None;
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
    // (Several areas keep a page of their own, each set of them its own: a
    // path no document has.)
    let set: Vec<ResRef> = std::iter::once(area).chain(others.iter().copied()).collect();
    let page_key = match several {
        true => (are, several_page(&set)),
        false => (are, GffPath::root()),
    };
    let tab = match several {
        true => crate::Tab::AreasProperties([&[area], others].concat()),
        false => crate::Tab::AreaProperties(area),
    };
    let asked = app.blueprint_pages.get(&page_key).copied();
    let page = asked.or_else(|| app.open_tabs.page(&tab, pages)).unwrap_or(pages[0]);
    let mut top = pages.iter().position(|p| *p == page).unwrap_or(0);
    // (The pages one after another on one scroll, the list at their left,
    // named for a screen reader and the tests: "Audio" is also the area
    // header's popover. `blueprint_pages` keeps the page at the top.)
    // (Kept by the whole set of areas: another set has its own place.)
    let id = egui::Id::new(("area-properties-pages", area, others));
    let mut listed = crate::form_pages::list(ui, id, PAGES_ROW, pages, &mut top);
    let ui = &mut listed.body;
    let mut names = Vec::new();
    if several {
        let by_name = app.settings.area_names;
        names = std::iter::once(&area)
            .chain(others)
            .map(|a| match &app.ws {
                Some(ws) => app.area_names.label(ws, app.game.as_deref(), *a, by_name),
                None => a.to_string(),
            })
            .collect();
        let cue = crate::blueprint::mixed_cue(others.len());
        ui.weak(format!(
            "{} areas: shown as the first ({}); what you change is set on each ({cue})",
            names.len(),
            names[0]
        ))
        .on_hover_text(names.join("\n"));
        ui.weak("Names, tags and comments are edited one area at a time.");
    }
    let also: crate::widgets::Also = std::sync::Arc::new(
        others.iter().map(|a| (ResKey::new(*a, ResType::ARE), GffPath::root())).collect(),
    );
    // (Sideways too: the window fits its pane, `TabViewer::scroll_bars`.)
    let narrow = crate::form_pages::narrow(ui.available_width());
    let scroll = crate::form_pages::scroll(
        id,
        pages,
        !narrow,
        listed.chosen,
        crate::form_pages::page_columns(ui.available_width()),
    );
    let first = names.first().cloned().unwrap_or_default();
    let shown = scroll.show(ui, &mut top, |ui, i| {
        let on = Shared {
            area,
            others,
            also: &also,
            audio: None,
            grass: None,
            root: &root,
            first: &first,
            viewed,
        };
        page_body(app, ui, pages[i], on);
    });
    // (Unless a page asked for another while it was drawn: brought to the
    // top in the next frame.)
    match app.blueprint_pages.get(&page_key).copied() == asked {
        true => _ = app.blueprint_pages.insert(page_key, pages[top]),
        false => ui.ctx().request_repaint(),
    }
    Some(shown.viewport)
}

/// What a page of Area Properties is drawn for ([`page_body`]): the area
/// (the first of several), the others edited with it and their ARE as the
/// form sets them (`also`), the GITs of those of them with audio settings
/// where they were worked out before (`audio`, Edit Areas Together's,
/// kept with its areas, as their grass settings are, `grass`), the first area's fields without its tiles, its
/// name as the window says it, and whether it is only looked at.
#[derive(Clone, Copy)]
struct Shared<'a> {
    area: ResRef,
    others: &'a [ResRef],
    also: &'a crate::widgets::Also,
    audio: Option<&'a crate::widgets::Also>,
    grass: Option<&'a std::sync::OnceLock<Vec<(ResKey, mg_gff::Struct)>>>,
    root: &'a mg_gff::Struct,
    first: &'a str,
    viewed: bool,
}

/// A page of Area Properties, `page`: the one code path of the window's
/// pages (Area Properties of one area or several, a tab) and of Edit Areas
/// Together's section's (M4's U29), which shows the shared ones one at a
/// time on its rail. Of several areas, it starts with Copy This Page to
/// Every Area.
fn page_body(app: &mut Moonglow, ui: &mut Ui, page: &str, on: Shared<'_>) {
    let Shared { area, others, also, audio: audio_also, grass, root, first, viewed } = on;
    if !others.is_empty() {
        // What the page shows is the first area's, and only what is
        // changed is set on the others: this gives them the rest of it
        // too.
        if ui
            .add_enabled(
                !viewed,
                // (Short enough for one line at the Right Dock's default
                // width, at 1280 and 1600: the page's name was in it, and it
                // wrapped there. M4's U29 review.)
                egui::Button::new("Copy This Page to Every Area"),
            )
            .on_hover_text(format!(
                "Sets every other area's {page} settings to what this page shows ({first}'s)"
            ))
            .clicked()
        {
            give_page(app, area, others, page, true);
        }
        ui.add_space(6.0);
    }
    crate::blueprint::read_only_if(ui, viewed, |ui| {
        if page == "Audio" {
            audio(app, ui, area, others, audio_also);
            return;
        }
        if page == "Grass" {
            let cached = match grass {
                Some(cell) => {
                    cell.get_or_init(|| {
                        app.tools.walks += 1;
                        let areas: Vec<ResRef> =
                            std::iter::once(area).chain(others.iter().copied()).collect();
                        crate::area_grass::properties(app, &areas)
                    });
                    None
                }
                None => {
                    type GrassCache = (
                        (Option<(u64, u64)>, u64, ResRef, Vec<ResRef>),
                        std::sync::Arc<Vec<(ResKey, mg_gff::Struct)>>,
                    );
                    let ws_key = app.ws.as_ref().map(|ws| (ws.identity(), ws.revision()));
                    let load_order = app.load_order;
                    let id = egui::Id::new("area-props-grass-cache");
                    let hit = ui.data(|d| d.get_temp::<GrassCache>(id)).and_then(|(k, v)| {
                        (k.0 == ws_key
                            && k.1 == load_order
                            && k.2 == area
                            && k.3.as_slice() == others)
                            .then_some(v)
                    });
                    Some(hit.unwrap_or_else(|| {
                        app.tools.walks += 1;
                        let areas: Vec<ResRef> =
                            std::iter::once(area).chain(others.iter().copied()).collect();
                        let props = std::sync::Arc::new(crate::area_grass::properties(app, &areas));
                        ui.data_mut(|d| {
                            d.insert_temp::<GrassCache>(
                                id,
                                ((ws_key, load_order, area, others.to_vec()), props.clone()),
                            );
                        });
                        props
                    }))
                }
            };
            let kept = grass
                .and_then(|c| c.get().map(Vec::as_slice))
                .or(cached.as_deref().map(Vec::as_slice));
            crate::area_grass::page(app, ui, area, others, kept);
            return;
        }
        let mut f = Form {
            app,
            key: ResKey::new(area, ResType::ARE),
            path: GffPath::root(),
            also: also.clone(),
            root: root.clone(),
            together: Default::default(),
            own_tab: false,
        };
        match page {
            "Basic" => basic(&mut f, ui),
            "Visual" => visual(&mut f, ui),
            "Events" => events(&mut f, ui),
            "Advanced" => advanced(&mut f, ui),
            "Variables" => variables_page(&mut f, ui),
            _ => f.memo(ui, "Comments", "Comments"),
        }
    })
}

/// The GITs of `others` that have audio settings (their AreaProperties),
/// as the Audio page's form sets them.
fn audio_also(app: &mut Moonglow, others: &[ResRef]) -> crate::widgets::Also {
    app.tools.walks += 1;
    let path = GffPath::root().field("AreaProperties");
    let also = others
        .iter()
        .map(|a| ResKey::new(*a, ResType::GIT))
        .filter(|g| {
            let doc = app.ws.as_mut().and_then(|ws| ws.doc(g).ok());
            doc.is_some_and(|d| path.get(&d.root).is_some())
        })
        .map(|g| (g, path.clone()))
        .collect();
    std::sync::Arc::new(also)
}

/// Where the page at the top of Area Properties of several areas `set`
/// is kept (with the first area's ARE): a path no document has, its own
/// for each set.
pub(crate) fn several_page(set: &[ResRef]) -> GffPath {
    let names: Vec<String> = set.iter().map(ResRef::to_string).collect();
    GffPath::root().field(&format!("several {}", names.join(" ")))
}

/// The Visual page's fields ([`mg_module::lighting::VISUAL_FIELDS`]).
pub(crate) use mg_module::lighting::VISUAL_FIELDS;

/// The fields of a page shared by several areas: in the area (`false`),
/// or in its GIT's `AreaProperties` (`true`).
fn page_fields(page: &str) -> (bool, &'static [&'static str]) {
    match page {
        "Visual" => (false, VISUAL_FIELDS),
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
        "Grass" => (true, &[mg_area::grass::DISABLED, mg_area::grass::OVERRIDES]),
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
fn give_page(app: &mut Moonglow, area: ResRef, others: &[ResRef], page: &str, relight: bool) {
    let (in_git, labels) = page_fields(page);
    let (restype, path) = if in_git {
        (ResType::GIT, GffPath::root().field("AreaProperties"))
    } else {
        (ResType::ARE, GffPath::root())
    };
    let Some(ws) = app.ws.as_mut() else { return };
    // (The grass the first area lacks, the others lose: what is not there
    // is what grows.)
    let lacking = |l: &'static str| (page == "Grass").then_some((l, None));
    type Read = (Vec<(&'static str, Option<Value>)>, Option<i64>);
    let (values, scheme_row): Read = match ws.doc(&ResKey::new(area, restype)) {
        Ok(doc) => match path.get(&doc.root) {
            Some(from) => (
                labels
                    .iter()
                    .filter_map(|l| {
                        from.get(l).map(|v| (*l, Some(v.clone()))).or_else(|| lacking(l))
                    })
                    .collect(),
                from.integer("LightingScheme"),
            ),
            None => return,
        },
        Err(_) => return,
    };
    // The Visual page: an area of another lighting scheme gets its tiles'
    // lights from the first's, as choosing the scheme gives them (one of
    // the same scheme keeps its own). Without the game's data, none.
    // Copied from an area (not `relight`): every tile's lights are left as
    // they are (the owner's word, 2026-10-29: "It should leave the lights
    // alone").
    let scheme: Option<TileColors> =
        (relight && page == "Visual").then_some(scheme_row).flatten().and_then(|row| {
            let g = app.game.as_deref()?;
            Some(mg_module::new::Scheme::read(g, row as usize).ok()?.tile_colors())
        });
    let mut rng = fastrand::Rng::new();
    let mut edits = Vec::new();
    for other in others {
        let key = ResKey::new(*other, restype);
        let Some(there) = ws.doc(&key).ok().and_then(|d| path.get(&d.root).cloned()) else {
            continue;
        };
        edits.extend(values.iter().map(|(label, value)| Edit::SetField {
            key,
            path: path.clone(),
            label: (*label).into(),
            value: value.clone(),
        }));
        if let Some(scheme) =
            scheme.as_ref().filter(|_| there.integer("LightingScheme") != scheme_row)
        {
            let tiles = there.list("Tile_List").map_or(0, <[_]>::len);
            edits.extend(tile_light_edits(scheme, key, tiles, &mut rng));
        }
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
    let set = app.tileset(tileset);
    let name = (app.game.as_ref().zip(set))
        .and_then(|(g, set)| {
            g.string(set.general.display_name).or(set.general.unlocalized_name.clone())
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
    let schemes = crate::widgets::table_choices(
        f.app.game.as_deref(),
        "environment",
        Some("STRREF"),
        Some("LABEL"),
    );
    let current = f.int("LightingScheme");
    crate::widgets::section_heading(ui, "Lighting Scheme");
    ui.weak(
        "Choosing a scheme replaces the area's lighting and weather settings and its tiles' \
         custom lighting.",
    );
    // In as many columns as the page is wide, each filled downwards, all of
    // them shown (Heed's ask: one column, scrolled, left the width unused).
    let mut picked = None;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let widest_key = (
        f.app.game.is_some(),
        f.app.load_order,
        schemes.len(),
        ui.ctx().pixels_per_point().to_bits(),
        font.size.to_bits(),
    );
    let widest_id = egui::Id::new("area-props-scheme-widest");
    let widest = if let Some((was, cached)) =
        ui.data(|d| d.get_temp::<((bool, u64, usize, u32, u32), f32)>(widest_id))
        && was == widest_key
    {
        cached
    } else {
        let w = (schemes.iter())
            .map(|c| {
                ui.painter().layout_no_wrap(c.text.clone(), font.clone(), egui::Color32::WHITE)
            })
            .map(|g| g.size().x)
            .fold(0.0, f32::max);
        ui.data_mut(|d| d.insert_temp(widest_id, (widest_key, w)));
        w
    };
    let column = widest + 2.0 * ui.spacing().button_padding.x + 16.0;
    let columns = ((ui.available_width() / column) as usize).clamp(1, schemes.len().max(1));
    let rows = schemes.len().div_ceil(columns);
    // The arrow keys move the choice while a scheme has the keys, as in a
    // list of choices: Up and Down down the columns, Left and Right to the
    // column beside (the one chosen so takes the keys in the next frame).
    let focus_id = egui::Id::new(("schemes-focus", f.key));
    let refocus: Option<usize> = ui.data_mut(|d| d.remove_temp(focus_id));
    let mut stepped = None;
    mg_uikit::with_rows(ui, |ui| {
        egui::Grid::new(("schemes", f.key)).spacing([16.0, 0.0]).min_col_width(widest).show(
            ui,
            |ui| {
                for row in 0..rows {
                    for col in 0..columns {
                        let at = col * rows + row;
                        let Some(c) = schemes.get(at) else { continue };
                        let r = ui.selectable_label(c.row as i64 == current, &c.text);
                        if refocus == Some(at) {
                            r.request_focus();
                        }
                        if r.clicked() {
                            picked = Some(c.row);
                        }
                        stepped = stepped.or(crate::widgets::grid_arrow_pick(
                            ui,
                            &r,
                            at,
                            schemes.len(),
                            rows,
                        ));
                    }
                    ui.end_row();
                }
            },
        )
    });
    if let Some(at) = stepped {
        picked = Some(schemes[at].row);
        ui.data_mut(|d| d.insert_temp(focus_id, at));
    }
    if let Some(row) = picked {
        apply_scheme(f, row);
    }
    ui.add_space(4.0);
    lighting_buttons(f, ui);
    ui.add_space(crate::widgets::SECTION_GAP);
    crate::widgets::section_heading(ui, "Environment");
    let skyboxes = crate::widgets::table_choices(
        f.app.game.as_deref(),
        "skyboxes",
        Some("STRING_REF"),
        Some("LABEL"),
    );
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
        f.spin(ui, "Sun fog amount", "SunFogAmount", 0..=200);
        f.spin(ui, "Moon fog amount", "MoonFogAmount", 0..=200);
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
    // (Wrapped in a narrow room: Edit Areas Together's section in the
    // Right Dock, M4's U29.)
    ui.horizontal_wrapped(|ui| {
        f.mark_mixed_inline(ui, &["DayNightCycle", "IsNight"]);
        for (m, text) in [(0, "Cycle Day and Night"), (1, "Always Bright"), (2, "Always Dark")] {
            if ui.radio_button(mode == m, text).clicked() && mode != m {
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
    // (Narrower than a label's column and the three winds: each label over
    // its field.)
    fields(ui, ("are-env2", f.key), 400.0, |ui, grid| {
        crate::widgets::field_label(ui, "Shadow Opacity");
        f.spin(ui, "Shadow opacity", "ShadowOpacity", 0..=100);
        end_row(ui, grid);
        crate::widgets::field_label(ui, "Fog Clip Distance (m)");
        f.spin_float(ui, "Fog clip distance", "FogClipDist", 30.0..=9999.0, 1.0, 1.0);
        end_row(ui, grid);
        crate::widgets::field_label(ui, "Sky Box");
        f.choice(ui, "Sky box", "SkyBox", &skyboxes, FieldType::Byte);
        end_row(ui, grid);
        crate::widgets::field_label(ui, "% Rain");
        f.spin(ui, "Chance of rain", "ChanceRain", 0..=100);
        end_row(ui, grid);
        crate::widgets::field_label(ui, "% Snow");
        f.spin(ui, "Chance of snow", "ChanceSnow", 0..=100);
        end_row(ui, grid);
        crate::widgets::field_label(ui, "% Lightning");
        f.spin(ui, "Chance of lightning", "ChanceLightning", 0..=100);
        end_row(ui, grid);
        crate::widgets::field_label(ui, "Wind Power");
        let winds = ["None", "Weak", "Strong"];
        let wind = f.int("WindPower").clamp(0, 2);
        ui.horizontal(|ui| {
            f.mark_mixed_inline(ui, &["WindPower"]);
            for (i, w) in winds.iter().enumerate() {
                if ui.radio_button(wind == i as i64, *w).clicked() && wind != i as i64 {
                    f.set_int("Wind power", "WindPower", i as i64, FieldType::Int);
                }
            }
        });
        end_row(ui, grid);
    });
}

/// Under the schemes: another area's lighting given to this one (and to
/// those edited with it), and lighting kept under a name, saved from here
/// or given from it (dansen9191's ask: the colors of one area in another,
/// and schemes of one's own).
fn lighting_buttons(f: &mut Form<'_>, ui: &mut Ui) {
    let targets: Vec<ResRef> =
        std::iter::once(f.key.resref).chain(f.also.iter().map(|(k, _)| k.resref)).collect();
    let by_name = f.app.settings.area_names;
    let dir = f.app.lighting_dir.clone();
    // The lighting kept, by name: read from its folder when the menu
    // opens and after a Save, Rename or Delete here, not every frame the
    // page shows (kept in between, for every Area Properties window).
    let saved_id = egui::Id::new("lighting-saved");
    let menu_id = egui::Id::new("lighting-saved-open");
    let kept: Option<std::sync::Arc<Vec<String>>> = ui.data(|d| d.get_temp(saved_id));
    let mut saved = match kept {
        Some(kept) => kept,
        None => {
            let read = std::sync::Arc::new(crate::lighting::list(dir.as_deref()));
            ui.data_mut(|d| d.insert_temp(saved_id, read.clone()));
            read
        }
    };
    let was_open: bool = ui.data(|d| d.get_temp(menu_id)).unwrap_or(false);
    let (mut is_open, mut changed) = (false, false);
    // A name being typed: for Save This Lighting (`None`) or to rename the
    // one named (`Some`).
    let naming = ui.id().with(("lighting-name", f.key.resref));
    // The one deleted last, which Undo Delete puts back (its file is
    // Moonglow's, not the module's: Edit › Undo doesn't know it).
    let deleted_id = egui::Id::new("lighting-deleted");
    let deleted: Option<(String, String)> = ui.data(|d| d.get_temp(deleted_id));
    // Whether saved lighting gives the tiles' lights too: asked each time
    // the window is open, off until ticked (it changes every tile).
    let relight_id = egui::Id::new("lighting-tiles-too");
    let mut relight: bool = ui.data(|d| d.get_temp(relight_id)).unwrap_or(false);
    let mut scheme_of = None;
    ui.horizontal(|ui| {
        mg_uikit::menu_button(ui, "Copy From Area", |ui| {
            mg_uikit::menu_rows(ui);
            // (The module's other areas, made while the menu is open.)
            let mut areas: Vec<(ResRef, String)> = match &f.app.ws {
                Some(ws) => (ws.module.keys_of(ResType::ARE).map(|k| k.resref))
                    .filter(|a| !targets.contains(a))
                    .map(|a| (a, f.app.area_names.label(ws, f.app.game.as_deref(), a, by_name)))
                    .collect(),
                None => Vec::new(),
            };
            areas.sort_by_cached_key(|(_, name)| name.to_lowercase());
            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for (area, name) in &areas {
                    if ui.button(name).clicked() {
                        give_page(f.app, *area, &targets, "Visual", false);
                        ui.close();
                    }
                }
            });
        })
        .response
        .on_hover_text(
            "Takes the lighting, fog, sky and weather from another area. Tiles' lights stay.",
        );
        mg_uikit::menu_button(ui, "Saved Lighting", |ui| {
            mg_uikit::menu_rows(ui);
            is_open = true;
            if !was_open {
                saved = std::sync::Arc::new(crate::lighting::list(dir.as_deref()));
                ui.data_mut(|d| d.insert_temp(saved_id, saved.clone()));
            }
            let saved = saved.as_slice();
            if let Some((name, text)) = &deleted
                && ui.button(format!("Undo Delete '{name}'")).clicked()
            {
                changed = true;
                match crate::lighting::restore(dir.as_deref(), name, text) {
                    Ok(()) => f.app.log.info(format!("Lighting '{name}' put back")),
                    Err(e) => f.app.log.error(format!("Undo Delete: {e}")),
                }
                ui.data_mut(|d| d.remove::<(String, String)>(deleted_id));
                ui.close();
            }
            if saved.is_empty() {
                mg_uikit::menu_note(ui, "None saved yet");
            }
            for name in saved {
                if ui.button(name).clicked() {
                    give_saved(f, name, relight);
                    ui.close();
                }
            }
            if !saved.is_empty() {
                mg_uikit::menu_check(ui, &mut relight, "Tiles' Lights Too").on_hover_text(
                    "Also picks each tile's lights anew from the colors saved with the lighting",
                );
            }
            mg_uikit::menu_separator(ui);
            if ui.button("Save This Lighting…").clicked() {
                ui.data_mut(|d| d.insert_temp(naming, (None::<String>, String::new())));
                ui.close();
            }
            if saved.is_empty() {
                return;
            }
            mg_uikit::menu_button(ui, "Rename", |ui| {
                for name in saved {
                    if ui.button(name).clicked() {
                        ui.data_mut(|d| d.insert_temp(naming, (Some(name.clone()), name.clone())));
                        ui.close();
                    }
                }
            });
            mg_uikit::menu_button(ui, "Delete", |ui| {
                for name in saved {
                    if ui
                        .button(name)
                        .on_hover_text("Undo Delete, in this menu, puts it back")
                        .clicked()
                    {
                        changed = true;
                        match crate::lighting::delete(dir.as_deref(), name) {
                            Ok(text) => {
                                f.app.log.info(format!("Lighting '{name}' deleted"));
                                ui.data_mut(|d| d.insert_temp(deleted_id, (name.clone(), text)));
                            }
                            Err(e) => f.app.log.error(format!("Delete lighting: {e}")),
                        }
                        ui.close();
                    }
                }
            });
            mg_uikit::menu_button(ui, "Make a Lighting Scheme", |ui| {
                mg_uikit::menu_heading(ui, "A row of environment.2da: the table to add it to");
                let key =
                    ResKey::new(ResRef::from_str("environment").expect("a name"), ResType::TWODA);
                for name in saved {
                    mg_uikit::menu_button(ui, name, |ui| {
                        if let Some(home) = crate::browser::table_homes(f.app, ui, key) {
                            scheme_of = Some((name.clone(), home));
                            ui.close();
                        }
                    });
                }
            })
            .response
            .on_hover_text(
                "Keeps the lighting as a row of the module's environment.2da, for \
                 SetAreaLightingScheme",
            );
        })
        .response
        .on_hover_text("Lighting kept under a name, for any area of any module");
    });
    ui.data_mut(|d| {
        d.insert_temp(relight_id, relight);
        d.insert_temp(menu_id, is_open);
        if changed {
            d.remove::<std::sync::Arc<Vec<String>>>(saved_id);
        }
    });
    if let Some((name, home)) = scheme_of {
        make_scheme(f.app, &name, home);
    }
    // Save This Lighting's name, or Rename's.
    let Some((from, mut name)) = ui.data(|d| d.get_temp::<(Option<String>, String)>(naming)) else {
        return;
    };
    let mut done = false;
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, if from.is_some() { "Rename to" } else { "Name" });
        let r = ui.add(mg_uikit::text_field(&mut name).desired_width(200.0));
        crate::widgets::autofocus(ui, &r);
        let other = |s: &&String| from.as_ref() != Some(*s) && s.eq_ignore_ascii_case(name.trim());
        let taken = saved.iter().any(|s| other(&s));
        let ok = crate::saved::valid_name(&name) && !(from.is_some() && taken);
        let entered = r.lost_focus()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let button = if from.is_some() { "Rename" } else { "Save" };
        if ui.add_enabled(ok, egui::Button::new(button)).clicked() || (ok && entered) {
            match &from {
                // (The area itself: the form's copy has no tiles.)
                None => match (f.app.ws.as_mut().map(|ws| ws.doc(&f.key)))
                    .and_then(Result::ok)
                    .ok_or_else(|| format!("{} can't be read", f.key))
                    .and_then(|are| crate::lighting::save(dir.as_deref(), &name, &are.root))
                {
                    Ok(p) => f.app.log.info(format!("Lighting saved: {}", p.display())),
                    Err(e) => f.app.log.error(format!("Save This Lighting: {e}")),
                },
                Some(from) => match crate::lighting::rename(dir.as_deref(), from, &name) {
                    Ok(()) => {
                        f.app.log.info(format!("Lighting '{from}' renamed to '{}'", name.trim()))
                    }
                    Err(e) => f.app.log.error(format!("Rename lighting: {e}")),
                },
            }
            done = true;
            ui.data_mut(|d| d.remove::<std::sync::Arc<Vec<String>>>(saved_id));
        }
        if ui.button("Cancel").clicked() {
            done = true;
        }
        if taken {
            ui.weak(match from {
                None => "replaces the one of that name",
                Some(_) => "another lighting has that name",
            });
        }
    });
    if done {
        ui.data_mut(|d| d.remove::<(Option<String>, String)>(naming));
    } else {
        ui.data_mut(|d| d.insert_temp(naming, (from, name)));
    }
}

/// The lighting kept as `name` given to the area (and those edited with
/// it), as one step: the Visual page's fields, and (`relight`, ticked in
/// the menu) each tile's lights picked from the colors kept with them.
fn give_saved(f: &mut Form<'_>, name: &str, relight: bool) {
    let saved = match crate::lighting::read(f.app.lighting_dir.as_deref(), name) {
        Ok(saved) => saved,
        Err(e) => {
            f.app.log.error(format!("Saved Lighting: {e}"));
            return;
        }
    };
    let Some(ws) = f.app.ws.as_mut() else { return };
    let mut rng = fastrand::Rng::new();
    let mut edits: Vec<Edit> = Vec::new();
    for key in std::iter::once(f.key).chain(f.also.iter().map(|(k, _)| *k)) {
        let Ok(are) = ws.doc(&key) else { continue };
        let fields = match saved.fields_for(&are.root) {
            Ok(fields) => fields,
            Err(e) => {
                f.app.log.error(format!("Saved Lighting: {name}: {e}"));
                return;
            }
        };
        let tiles = are.root.list("Tile_List").map_or(0, <[_]>::len);
        edits.extend(fields.into_iter().map(|(label, value)| Edit::SetField {
            key,
            path: GffPath::root(),
            label: label.into(),
            value: Some(value),
        }));
        if let Some(colors) = saved.tiles().filter(|_| relight) {
            edits.extend(tile_light_edits(colors, key, tiles, &mut rng));
        }
    }
    f.app.actions.push(Action::Apply(Command::new(format!("Lighting {name}"), edits)));
}

/// Make a Lighting Scheme: the lighting kept as `name` added as a row of
/// environment.2da, in the 2DA editor where `home` says, to be saved
/// there (a hak's Save writes the hak; Tools › Reload Resources then
/// lists the scheme).
fn make_scheme(app: &mut Moonglow, name: &str, home: crate::browser::EditTable) {
    let saved = match crate::lighting::read(app.lighting_dir.as_deref(), name) {
        Ok(saved) => saved,
        Err(e) => {
            app.log.error(format!("Make a Lighting Scheme: {e}"));
            return;
        }
    };
    let key = ResKey::new(ResRef::from_str("environment").expect("a name"), ResType::TWODA);
    let Some(id) = crate::browser::open_table(app, key, home) else { return };
    let cells = saved.environment_row(&mg_module::lighting::row_label(name));
    let label = format!("Lighting Scheme {name}");
    if let Some(row) = crate::table_view::add_row_of(app, id, &label, &cells) {
        app.log.info(format!(
            "Lighting '{name}' is row {row} of environment.2da: save the table where it is (a \
             hak's own Save then writes the hak), and Tools › Reload Resources lists it among \
             the lighting schemes"
        ));
    }
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
        edits.extend(scheme.applied_fields().into_iter().map(|(label, value)| Edit::SetField {
            key,
            path: GffPath::root(),
            label: label.into(),
            value: Some(value),
        }));
        edits.extend(tile_light_edits(&scheme.tile_colors(), key, tiles, &mut rng));
    }
    f.app.actions.push(Action::Apply(Command::new("Lighting Scheme", edits)));
}

/// The edits that give the `tiles` tiles of area `key` (its ARE) the
/// lights a lighting scheme picks for them.
fn tile_light_edits(
    colors: &TileColors,
    key: ResKey,
    tiles: usize,
    rng: &mut fastrand::Rng,
) -> Vec<Edit> {
    let mut edits = Vec::new();
    for i in 0..tiles {
        for (label, value) in mg_module::lighting::pick_tile_fields(colors, rng) {
            edits.push(Edit::SetField {
                key,
                path: GffPath::root().item("Tile_List", i),
                label: label.into(),
                value: Some(value),
            });
        }
    }
    edits
}

/// The Audio page of `area` (and `others`, where several are edited at
/// once; `kept`, those of their GITs with audio settings where they were
/// worked out before).
fn audio(
    app: &mut Moonglow,
    ui: &mut Ui,
    area: ResRef,
    others: &[ResRef],
    kept: Option<&crate::widgets::Also>,
) {
    let git = ResKey::new(area, ResType::GIT);
    let path = GffPath::root().field("AreaProperties");
    let also = match kept {
        Some(kept) => kept.clone(),
        None => {
            type AudioAlsoCache = ((Option<(u64, u64)>, u64, Vec<ResRef>), crate::widgets::Also);
            let ws_key = app.ws.as_ref().map(|ws| (ws.identity(), ws.revision()));
            let load_order = app.load_order;
            let id = egui::Id::new("area-props-audio-also-cache");
            let hit = ui.data(|d| d.get_temp::<AudioAlsoCache>(id)).and_then(|(k, v)| {
                (k.0 == ws_key && k.1 == load_order && k.2.as_slice() == others).then_some(v)
            });
            hit.unwrap_or_else(|| {
                let also = audio_also(app, others);
                ui.data_mut(|d| {
                    d.insert_temp::<AudioAlsoCache>(
                        id,
                        ((ws_key, load_order, others.to_vec()), also.clone()),
                    );
                });
                also
            })
        }
    };
    let Some(root) =
        app.ws.as_mut().and_then(|ws| ws.doc(&git).ok()).and_then(|g| path.get(&g.root)).cloned()
    else {
        ui.label("This area has no audio settings (its GIT lacks AreaProperties).");
        return;
    };
    let sounds = crate::widgets::table_choices(
        app.game.as_deref(),
        "ambientsound",
        Some("Description"),
        Some("Resource"),
    );
    let music = crate::widgets::table_choices(
        app.game.as_deref(),
        "ambientmusic",
        Some("Description"),
        Some("Resource"),
    );
    let eax = crate::widgets::table_choices(
        app.game.as_deref(),
        "soundeax",
        Some("Description"),
        Some("Label"),
    );
    let mut f =
        Form { app, key: git, path, also, root, together: Default::default(), own_tab: false };
    // (Narrower than its longest label and a choice: each label over its
    // field.)
    fields(ui, ("are-audio", git), 440.0, |ui, grid| {
        for (text, label, list) in [
            ("Ambient Sound, Day", "AmbientSndDay", &sounds),
            ("Ambient Sound, Night", "AmbientSndNight", &sounds),
        ] {
            crate::widgets::field_label(ui, text);
            f.choice(ui, text, label, list, FieldType::Int);
            end_row(ui, grid);
        }
        crate::widgets::field_label(ui, "Ambient Sound, Day Volume");
        f.slider(ui, "Day volume", "AmbientSndDayVol", 0..=127);
        end_row(ui, grid);
        crate::widgets::field_label(ui, "Ambient Sound, Night Volume");
        f.slider(ui, "Night volume", "AmbientSndNitVol", 0..=127);
        end_row(ui, grid);
        for (text, label) in [
            ("Music, Day", "MusicDay"),
            ("Music, Night", "MusicNight"),
            ("Music, Battle", "MusicBattle"),
        ] {
            crate::widgets::field_label(ui, text);
            f.choice(ui, text, label, &music, FieldType::Int);
            end_row(ui, grid);
        }
        crate::widgets::field_label(ui, "Music, Playing Delay");
        f.spin(ui, "Music delay", "MusicDelay", -99..=99);
        end_row(ui, grid);
        crate::widgets::field_label(ui, "Environmental Audio Effects");
        f.choice(ui, "Environmental audio", "EnvAudio", &eax, FieldType::Int);
        end_row(ui, grid);
    });
}

fn events(f: &mut Form<'_>, ui: &mut Ui) {
    let events = ["OnEnter", "OnExit", "OnHeartbeat", "OnUserDefined"].map(|l| (l, l));
    fields(ui, ("are-events", f.key), 400.0, |ui, grid| {
        for (label, field) in events {
            crate::widgets::field_label(ui, label);
            f.script(ui, label, field);
            end_row(ui, grid);
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

/// A page's fields as the form's grid of two columns, or, where the room
/// is narrower than `two_columns`, each label over its field: Edit Areas
/// Together's section in the Right Dock (M4's U29), where the grid, which
/// sizes a column before a wrap, ran a field past the dock's edge. `add`
/// is told whether it draws into the grid (for [`end_row`]).
fn fields(ui: &mut Ui, id: impl egui::AsIdSalt, two_columns: f32, add: impl FnOnce(&mut Ui, bool)) {
    if ui.available_width() < two_columns {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            add(ui, false);
        });
    } else {
        egui::Grid::new(id).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| add(ui, true));
    }
}

/// The end of a row of [`fields`] drawn into the grid.
fn end_row(ui: &mut Ui, grid: bool) {
    if grid {
        ui.end_row();
    }
}

/// The module's tilesets with their names (the Areas page's Tileset).
type Tilesets = std::sync::Arc<Vec<(ResRef, String)>>;

/// The width under which the Advanced page goes one field under the
/// other: a label's column and "Exterior" beside "Interior (no weather
/// effects)".
const ADVANCED_TWO_COLUMNS: f32 = 480.0;

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let id = ("are-advanced", f.key);
    fields(ui, id, ADVANCED_TWO_COLUMNS, |ui, grid| advanced_fields(f, ui, grid));
}

/// The Advanced page's fields: a label and its field a row of the form's
/// grid where `grid`, else the label over the field.
fn advanced_fields(f: &mut Form<'_>, ui: &mut Ui, grid: bool) {
    let pvp = crate::widgets::table_choices(
        f.app.game.as_deref(),
        "pvpsettings",
        Some("strref"),
        Some("label"),
    );
    let screens = crate::widgets::table_choices(
        f.app.game.as_deref(),
        "loadscreens",
        Some("StrRef"),
        Some("Label"),
    );
    let flags = f.int("Flags") as u32;
    if f.also.is_empty() {
        crate::widgets::field_label(ui, "Tag");
        f.tag(ui, "Tag", "Tag");
        end_row(ui, grid);
        crate::widgets::field_label(ui, "ResRef");
        ui.label(f.key.resref.to_string());
        end_row(ui, grid);
    }
    crate::widgets::field_label(ui, "Check Modifier - Listen");
    f.spin(ui, "Listen check modifier", "ModListenCheck", -99..=99);
    end_row(ui, grid);
    crate::widgets::field_label(ui, "Check Modifier - Spot");
    f.spin(ui, "Spot check modifier", "ModSpotCheck", -99..=99);
    end_row(ui, grid);
    crate::widgets::field_label(ui, "Player Vs. Player");
    f.choice(ui, "Player vs. player", "PlayerVsPlayer", &pvp, FieldType::Byte);
    end_row(ui, grid);
    crate::widgets::field_label(ui, "Loading Screen");
    f.load_screen(ui, &screens);
    end_row(ui, grid);
    if grid {
        crate::widgets::field_label(ui, "");
    }
    f.check(ui, "No Rest", "NoRest");
    end_row(ui, grid);
    for (bit, off, on) in [
        (INTERIOR, "Exterior", "Interior (no weather effects)"),
        (NATURAL, "Artificial", "Natural"),
        (UNDERGROUND, "Above ground", "Underground"),
    ] {
        let both = |ui: &mut Ui| {
            // (One field holds these and the shader flags: each bit marked,
            // before its pair of choices, the whole pair's.)
            f.mark_mixed_bit_inline(ui, "Flags", bit);
            let set = flags & bit != 0;
            for (want, text) in [(false, off), (true, on)] {
                if ui.radio_button(set == want, text).clicked() && set != want {
                    set_flag(f, text, bit, want);
                }
            }
        };
        if grid {
            ui.label("");
            ui.horizontal(both);
        } else {
            ui.horizontal_wrapped(both);
        }
        end_row(ui, grid);
    }
    // The flags past the three, for custom shaders (their `areaFlags`
    // uniform): kept by the game (`engine_ee_fields.rs`).
    crate::widgets::field_label(ui, "Shader Flags").on_hover_text(
        "Area flags for custom shaders, read from their areaFlags uniform. Stock shaders \
         ignore them.",
    );
    // Rows of their own (a wrapped row would run over the next one: the
    // form's grid sizes its rows before the wrap).
    let per_row = if grid { 7 } else { 4 };
    ui.vertical(|ui| {
        egui::Grid::new("area-shader-flags").num_columns(per_row).show(ui, |ui| {
            for (k, bit) in (3..16).map(|b| 1u32 << b).enumerate() {
                let mut on = flags & bit != 0;
                f.mark_mixed_bit(ui, "Flags", bit);
                if ui.check_box(&mut on, bit.to_string()).changed() {
                    set_flag(f, "Shader flags", bit, on);
                }
                if k % per_row == per_row - 1 {
                    ui.end_row();
                }
            }
        });
        let higher = flags & !0xFFFF;
        if higher != 0 {
            ui.weak(format!("and {higher:#x}"));
        }
    });
    end_row(ui, grid);
    crate::widgets::field_label(ui, "Variables");
    area_variables(f, ui);
    end_row(ui, grid);
}

/// The areas' variables (this one's, and those edited with it): the game
/// reads an area's from its GIT's root, none from its ARE
/// (`engine_area_variables`). Those Moonglow wrote into the ARE before
/// 1.34.3 are moved to the GIT, one step to undo, when the window opens
/// (one of the same name in the GIT keeps its value there).
fn area_variables(f: &mut Form<'_>, ui: &mut Ui) {
    let cached = variables_moved(f, ui);
    let Some((target, moved)) = cached.1.as_ref() else { return };
    if !crate::claims::variables_button(f.app, ui, &f.key, moved[0].1.len()) {
        return;
    }
    let target = target.clone();
    let moved = moved.clone();
    drop(cached);
    move_variables(f, &moved);
    f.app.var_edit = Some(crate::widgets::VarTableEdit::new(target, &moved[0].1));
}

#[allow(clippy::type_complexity)]
type VarsMovedValue =
    Option<(crate::widgets::FieldTarget, Vec<(ResRef, Vec<mg_gff::Struct>, bool)>)>;
#[allow(clippy::type_complexity)]
type VarsMovedCache = ((Option<(u64, u64)>, u64, ResKey, crate::widgets::Also), VarsMovedValue);

/// Each area's variables (this one's, and those edited with it) as they
/// are once moved from the ARE to the GIT (the GIT's, then the ARE's it
/// lacks, as Verify's fix moves them), and whether any were in the ARE;
/// and their GIT roots' VarTable, the first's with the others'.
fn variables_moved(f: &mut Form<'_>, ui: &Ui) -> std::sync::Arc<VarsMovedCache> {
    let ws_key = f.app.ws.as_ref().map(|ws| (ws.identity(), ws.revision()));
    let load_order = f.app.load_order;
    let id = egui::Id::new("area-props-variables-moved-cache");
    if let Some(cached) = ui.data(|d| d.get_temp::<std::sync::Arc<VarsMovedCache>>(id))
        && cached.0.0 == ws_key
        && cached.0.1 == load_order
        && cached.0.2 == f.key
        && cached.0.3 == f.also
    {
        return cached;
    }
    let areas: Vec<ResRef> =
        std::iter::once(f.key.resref).chain(f.also.iter().map(|(k, _)| k.resref)).collect();
    let computed = match f.app.ws.as_mut() {
        Some(ws) => {
            let mut lists = Vec::new();
            for &area in &areas {
                let read = |t: ResType, ws: &mut mg_edit::Workspace| -> Vec<mg_gff::Struct> {
                    ws.doc(&ResKey::new(area, t))
                        .ok()
                        .and_then(|d| d.root.list("VarTable").map(<[_]>::to_vec))
                        .unwrap_or_default()
                };
                lists.push((area, read(ResType::GIT, ws), read(ResType::ARE, ws)));
            }
            let moved = lists
                .into_iter()
                .map(|(area, git, are)| {
                    (area, mg_edit::fix::variables_moved(&git, &are), !are.is_empty())
                })
                .collect();
            let git = |a: ResRef| (ResKey::new(a, ResType::GIT), GffPath::root());
            let (key, path) = git(areas[0]);
            let mut target = crate::widgets::FieldTarget::new(key, path, "VarTable");
            target.also = std::sync::Arc::new(areas[1..].iter().map(|&a| git(a)).collect());
            Some((target, moved))
        }
        None => None,
    };
    let entry = std::sync::Arc::new(((ws_key, load_order, f.key, f.also.clone()), computed));
    ui.data_mut(|d| d.insert_temp(id, entry.clone()));
    entry
}

/// Moves the variables still in an area's ARE to its GIT (`moved`, as
/// [`variables_moved`] reads them), one step to undo: whether there were
/// any.
fn move_variables(f: &mut Form<'_>, moved: &[(ResRef, Vec<mg_gff::Struct>, bool)]) -> bool {
    let mut edits = Vec::new();
    for (area, list, had) in moved {
        if !had {
            continue;
        }
        let set = |t: ResType, value: Option<Value>| Edit::SetField {
            key: ResKey::new(*area, t),
            path: GffPath::root(),
            label: "VarTable".into(),
            value,
        };
        edits.push(set(ResType::GIT, Some(Value::List(list.clone()))));
        edits.push(set(ResType::ARE, None));
    }
    if edits.is_empty() {
        return false;
    }
    f.app.log.info(if edits.len() == 2 {
        "The area's variables moved from its ARE to its GIT, where the game reads them".into()
    } else {
        format!(
            "{} areas' variables moved from their ARE to their GIT, where the game reads them",
            edits.len() / 2
        )
    });
    f.app.actions.push(Action::Apply(Command::new("Move Area Variables", edits)));
    true
}

/// The Variables page: the areas' variables in place
/// ([`crate::widgets::variables_page`]), those still in an ARE moved to
/// the GIT first, as the Variables… button moves them.
fn variables_page(f: &mut Form<'_>, ui: &mut Ui) {
    let cached = variables_moved(f, ui);
    let Some((target, moved)) = cached.1.as_ref() else { return };
    if move_variables(f, moved) {
        ui.weak("Moving the variables from the ARE to the GIT…");
        return;
    }
    let locked = crate::claims::read_only(f.app, f.key.resref);
    crate::widgets::variables_page(f.app, ui, target.clone(), &moved[0].1, locked);
}

/// What [`AreaChooser::run`]'s length was worked out for: the step on top
/// of the history, its depth, the steps kept, whether they are the ticks'.
type RunKey = (Option<u64>, usize, usize, bool);

/// Edit Areas Together (its Right Dock section, M4's U29): the areas
/// ticked, and the filters that narrow the list they are ticked in.
#[derive(Debug, Clone, Default)]
pub(crate) struct AreaChooser {
    /// Text in an area's name or ResRef.
    pub filter: String,
    pub tileset: Option<ResRef>,
    /// Interior, underground, natural: must be so, must not, or either.
    pub flags: [Option<bool>; 3],
    /// What the areas must hold: music, a variable, a field's value.
    pub hold: Hold,
    /// A variable's name, a field's label (`*` as in tags).
    pub hold_name: String,
    /// Its value (`*` as in tags; empty: any).
    pub hold_value: String,
    /// The music: an ambientmusic.2da row.
    pub music: Option<usize>,
    /// The areas that hold what is asked for, and what it was asked for
    /// (the condition, the workspace's revision).
    pub(crate) held: Option<(Option<Has>, u64, std::sync::Arc<std::collections::BTreeSet<ResRef>>)>,
    /// The name Save Set… keeps the ticked areas under.
    pub set_name: String,
    /// The sets kept for this module, read once the window opens and
    /// again after Save Set and Delete Set (not each frame).
    pub(crate) sets: Option<Vec<String>>,
    pub chosen: std::collections::BTreeSet<ResRef>,
    /// The module's areas, each with what the filters ask of it and its
    /// label, in the list's order: made once for a revision of the
    /// module and a way of naming areas, not every frame.
    pub(crate) listed: Option<((u64, bool), Listed)>,
    /// The section's page on its rail ([`TOGETHER_PAGES`]: 0, Areas).
    pub(crate) page: usize,
    /// The row of the checklist the keys chose.
    pub(crate) cursor: Option<ResRef>,
    /// Whether the checklist had rows as it was last drawn: its keys and
    /// their hints are there only then ([`AreaChooser::list_keys`]).
    pub(crate) rows: bool,
    /// The checklist's scroll and height as last drawn.
    list_seen: (f32, f32),
    /// Counted up each time the areas ticked change ([`AreaChooser::ticked`]).
    chosen_at: u64,
    /// The rows the filters show (places in the list), and what for.
    shown: Option<(ShownFor, std::sync::Arc<Vec<usize>>)>,
    /// The module's tilesets with their names, for the list made at the
    /// stamp kept.
    tilesets: Option<((u64, bool), Tilesets)>,
    /// The areas ticked as their pages take them, and what for (the
    /// module's revision, `chosen_at`, whether areas are said by name, how
    /// often the project's files were read).
    set: Option<(SetKey, std::sync::Arc<TogetherSet>)>,
    /// How often the rows shown, and the set, were worked out (for tests:
    /// never every frame).
    pub(crate) made: (u64, u64),
    /// The steps of the history its shared pages made since the areas were
    /// ticked (`steps_at`: the `chosen_at` they were made at), each with
    /// the depth of the history it was made at: Cancel All's
    /// ([`AreaChooser::run`]).
    steps: Vec<(u64, usize)>,
    steps_at: u64,
    /// Whether a field of its pages held text typed as it was last drawn
    /// (Cancel All takes it back too: the field is left as it is pressed).
    typing: bool,
    /// The field of its pages with the keyboard and its text as it took
    /// them: typed in only once the text differs.
    typed_from: Option<(egui::Id, String)>,
    /// [`AreaChooser::run`]'s length and what it was worked out for (the
    /// step on top and the history's depth, the steps kept and whether
    /// they are the ticks'): again only when one changes.
    run_seen: Option<(RunKey, usize)>,
    /// How often it was worked out (for tests: never every frame).
    pub(crate) runs: u64,
}

/// What [`AreaChooser`]'s rows shown were worked out for: the list's
/// stamp, the filters, what the areas hold and its revision.
type ShownFor = ((u64, bool), String, Option<ResRef>, [Option<bool>; 3], Option<Has>, Option<u64>);

/// [`AreaChooser`]'s areas, as its list has them.
type Listed = std::sync::Arc<Vec<(ResRef, crate::tree::AreaInfo, String)>>;

/// What Edit Areas Together's Holding asks of an area.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Hold {
    #[default]
    Anything,
    Music,
    Variable,
    Field,
}

impl Hold {
    const ALL: [Hold; 4] = [Hold::Anything, Hold::Music, Hold::Variable, Hold::Field];

    fn label(self) -> &'static str {
        match self {
            Hold::Anything => "Anything",
            Hold::Music => "Music",
            Hold::Variable => "A variable",
            Hold::Field => "A field's value",
        }
    }
}

/// The flags the chooser filters by, as its list reads.
const FLAG_FILTERS: [(u32, &str, &str, &str); 3] = [
    (INTERIOR, "Interior or exterior", "Interior", "Exterior"),
    (UNDERGROUND, "Above or under ground", "Underground", "Above ground"),
    (NATURAL, "Natural or artificial", "Natural", "Artificial"),
];

impl AreaChooser {
    /// A step its shared pages made, `depth` the history's after it.
    pub(crate) fn made_step(&mut self, step: u64, depth: usize) {
        if self.steps_at != self.chosen_at {
            self.steps.clear();
            self.steps_at = self.chosen_at;
        }
        self.steps.push((step, depth));
    }

    /// Where in [`AreaChooser::steps`] the run of its own steps on top of
    /// the history is, last first: the step on top its, made at that depth,
    /// then each made one below the other (a step undone by hand, never
    /// made again, is passed over). An edit made elsewhere between two
    /// ends it; empty where the step on top is not its, or the areas were
    /// ticked again since.
    pub(crate) fn run(&self, ws: &mg_edit::Workspace) -> Vec<usize> {
        let (Some(top), depth) = (ws.last_step(), ws.undo_depth()) else { return Vec::new() };
        if self.steps_at != self.chosen_at {
            return Vec::new();
        }
        let Some(at) = self.steps.iter().rposition(|s| *s == (top, depth)) else {
            return Vec::new();
        };
        let mut run = vec![at];
        let mut want = depth;
        for (i, &(step, d)) in self.steps[..at].iter().enumerate().rev() {
            if !ws.step_done(step) {
                continue;
            }
            if d + 1 != want {
                break;
            }
            run.push(i);
            want = d;
        }
        run
    }

    /// How many steps [`AreaChooser::run`] holds, kept while the history's
    /// top and depth and the steps are as they were (not every frame).
    pub(crate) fn run_len(&mut self, ws: &mg_edit::Workspace) -> usize {
        let key =
            (ws.last_step(), ws.undo_depth(), self.steps.len(), self.steps_at == self.chosen_at);
        if let Some((seen, n)) = self.run_seen
            && seen == key
        {
            return n;
        }
        self.runs += 1;
        let n = self.run(ws).len();
        self.run_seen = Some((key, n));
        n
    }

    /// The run ([`AreaChooser::run`]) made one step of the history: kept as
    /// that one, at the depth of its first, and the steps after it (undone,
    /// to be redone) each that much lower.
    pub(crate) fn joined(&mut self, run: &[usize], ws: &mg_edit::Workspace) {
        let (Some(&last), Some(&first)) = (run.first(), run.last()) else { return };
        let fewer = run.len() - 1;
        let depth = self.steps[first].1;
        for s in &mut self.steps[last + 1..] {
            s.1 = s.1.saturating_sub(fewer);
        }
        let step = ws.last_step().unwrap_or(self.steps[last].0);
        self.steps[last] = (step, depth);
        for &i in &run[1..] {
            self.steps.remove(i);
        }
    }

    /// Whether Enter is Open in Tab: on the Areas page alone, two areas or
    /// more ticked (the section's key and its hint ask the same).
    pub(crate) fn enter_opens(&self) -> bool {
        self.page == 0 && self.chosen.len() >= 2
    }

    /// Whether the checklist's keys (Up, Down, Space...) are there to say:
    /// on the Areas page, with rows to move in (its hints ask the same).
    pub(crate) fn list_keys(&self) -> bool {
        self.page == 0 && self.rows
    }

    /// Starting from `areas`, ticked.
    pub(crate) fn of(areas: impl IntoIterator<Item = ResRef>) -> AreaChooser {
        AreaChooser { chosen: areas.into_iter().collect(), ..Default::default() }
    }

    /// Whether an area passes the filters (but for what it holds:
    /// [`AreaChooser::has`]).
    /// Whether the filters show an area: its name, tag or ResRef, its
    /// tileset and kind, as `mg areas` chooses (`areas::Filter`).
    #[cfg(test)]
    pub(crate) fn shows(&self, area: ResRef, info: &crate::tree::AreaInfo) -> bool {
        self.filters().matches(&crate::tree::AreaInfo { resref: area, ..info.clone() })
    }

    /// The filters, as `mg areas` has them (`areas::Filter`): made once
    /// for a frame's list, not for each area of it.
    fn filters(&self) -> mg_module::areas::Filter {
        let [interior, underground, natural] = self.flags;
        mg_module::areas::Filter {
            text: Some(self.filter.clone()).filter(|t| !t.trim().is_empty()),
            tileset: self.tileset,
            interior,
            underground,
            natural,
            ..Default::default()
        }
    }

    /// The module's areas as the list has them, in its order (the module
    /// tree's: by name when it lists names); made again when the module
    /// changes, and the areas that went away are then no longer chosen.
    fn listed(&mut self, app: &mut Moonglow) -> Listed {
        let Some(ws) = &app.ws else { return Default::default() };
        let by_name = app.settings.area_names;
        let stamp = (ws.revision(), by_name);
        if let Some((at, listed)) = &self.listed
            && *at == stamp
        {
            return listed.clone();
        }
        let mut areas: Vec<(ResRef, crate::tree::AreaInfo, String)> = (ws.module)
            .keys_of(ResType::ARE)
            .map(|k| {
                let a = k.resref;
                let i = app.area_names.info(ws, app.game.as_deref(), a);
                // (A space the name begins with is kept, so it sorts as
                // the Module panel lists it, #54.)
                let label = match (i.name.trim_end(), by_name) {
                    ("", _) => a.to_string(),
                    (name, true) => format!("{name} ({a})"),
                    (name, false) => format!("{a} ({name})"),
                };
                (a, crate::tree::AreaInfo { resref: a, ..i }, label)
            })
            .collect();
        areas.sort_by_cached_key(|(a, _, label)| (label.to_lowercase(), *a));
        let there: std::collections::HashSet<ResRef> = areas.iter().map(|(a, ..)| *a).collect();
        let before = self.chosen.len();
        self.chosen.retain(|a| there.contains(a));
        if self.chosen.len() != before {
            self.ticked();
        }
        let listed = Listed::new(areas);
        self.listed = Some((stamp, listed.clone()));
        listed
    }

    /// What the areas must hold, as Holding asks it (none: anything, or
    /// what it asks isn't typed yet).
    pub(crate) fn has(&self) -> Option<Has> {
        let name = self.hold_name.trim().to_string();
        let value = Some(self.hold_value.trim().to_string()).filter(|v| !v.is_empty());
        match self.hold {
            Hold::Anything => None,
            Hold::Music => self.music.map(|r| Has::Music(r as i64)),
            Hold::Variable => (!name.is_empty()).then_some(Has::Var { name, value }),
            Hold::Field => (!name.is_empty()).then_some(Has::Field { label: name, value }),
        }
    }

    /// The areas that hold what Holding asks (all where it asks nothing),
    /// read again once it asks something else or the module changes.
    fn holding(
        &mut self,
        app: &mut Moonglow,
    ) -> Option<&std::sync::Arc<std::collections::BTreeSet<ResRef>>> {
        let has = self.has()?;
        let ws = app.ws.as_mut()?;
        let revision = ws.revision();
        let asked = |h: &(Option<Has>, u64, _)| h.0.as_ref() == Some(&has) && h.1 == revision;
        if !self.held.as_ref().is_some_and(asked) {
            // (The documents open, flushed: what they show is what is
            // read.)
            if let Err(e) = ws.flush() {
                app.log.error(e.to_string());
                return None;
            }
            let cp = app.game.as_deref().map_or(mg_core::Codepage::WINDOWS_1252, |g| g.codepage());
            let found = (ws.module.keys_of(ResType::ARE).map(|k| k.resref))
                .filter(|&a| app.area_holds.of(&ws.module, a, cp).is_some_and(|h| has.matches(h)))
                .collect();
            let found = std::sync::Arc::new(found);
            self.held = Some((Some(has), revision, found));
        }
        self.held.as_ref().map(|h| &h.2)
    }

    /// The areas ticked changed: what is worked out of them is again.
    pub(crate) fn ticked(&mut self) {
        self.chosen_at += 1;
    }

    /// The places in `areas` of the rows the filters show (and Holding,
    /// `holding`), worked out once for what they ask and the list as it is.
    fn shown(
        &mut self,
        areas: &Listed,
        holding: Option<&std::collections::BTreeSet<ResRef>>,
    ) -> std::sync::Arc<Vec<usize>> {
        let stamp = self.listed.as_ref().map_or((0, false), |(s, _)| *s);
        let held_at = self.held.as_ref().filter(|_| holding.is_some()).map(|h| h.1);
        let wanted: ShownFor =
            (stamp, self.filter.clone(), self.tileset, self.flags, self.has(), held_at);
        if let Some((at, shown)) = &self.shown
            && *at == wanted
        {
            return shown.clone();
        }
        let filters = self.filters();
        let compiled = filters.compiled();
        let holds = |a: &ResRef| holding.is_none_or(|h| h.contains(a));
        let shown: Vec<usize> = (areas.iter().enumerate())
            .filter(|(_, (a, i, _))| compiled.matches(i) && holds(a))
            .map(|(n, _)| n)
            .collect();
        let shown = std::sync::Arc::new(shown);
        self.shown = Some((wanted, shown.clone()));
        self.made.0 += 1;
        shown
    }

    /// The module's tilesets, each with its name, as the Tileset filter
    /// lists them: once for the list made.
    fn tilesets(&mut self, app: &mut Moonglow, areas: &Listed) -> Tilesets {
        let stamp = self.listed.as_ref().map_or((0, false), |(s, _)| *s);
        if let Some((at, kept)) = &self.tilesets
            && *at == stamp
        {
            return kept.clone();
        }
        let mut sets: Vec<ResRef> = areas.iter().map(|(_, i, _)| i.tileset).collect();
        sets.sort();
        sets.dedup();
        let named = std::sync::Arc::new(
            sets.into_iter().map(|t| (t, tileset_name(app, t))).collect::<Vec<_>>(),
        );
        self.tilesets = Some((stamp, named.clone()));
        named
    }

    /// The areas ticked as their shared pages take them ([`TogetherSet`]):
    /// worked out again when the module or the areas ticked change.
    fn set(&mut self, app: &mut Moonglow) -> Option<std::sync::Arc<TogetherSet>> {
        let ws = app.ws.as_ref()?;
        // (An area ticked renamed or deleted since: no longer ticked, the
        // count said and the pages' areas as the module has them. M4's U29
        // review.)
        let before = self.chosen.len();
        self.chosen.retain(|a| ws.module.contains(&ResKey::new(*a, ResType::ARE)));
        if self.chosen.len() != before {
            self.ticked();
        }
        // (Again where the names are said otherwise, and where the project's
        // files were read again: who may edit which area. M4's U29 review.)
        let wanted =
            (ws.revision(), self.chosen_at, app.settings.area_names, ws.module.project_reads());
        if let Some((at, set)) = &self.set
            && *at == wanted
        {
            return Some(set.clone());
        }
        let mut areas = (self.chosen.iter().copied())
            .filter(|a| ws.module.contains(&ResKey::new(*a, ResType::ARE)));
        let area = areas.next()?;
        let others: Vec<ResRef> = areas.collect();
        let also = std::sync::Arc::new(
            others.iter().map(|a| (ResKey::new(*a, ResType::ARE), GffPath::root())).collect(),
        );
        let by_name = app.settings.area_names;
        let mut name = |a: ResRef| app.area_names.label(ws, app.game.as_deref(), a, by_name);
        let first = name(area);
        // (The tip names the first hundred: a world's thousands would not
        // fit on the screen.)
        let mut names: Vec<String> =
            std::iter::once(area).chain(others.iter().copied()).take(100).map(name).collect();
        if others.len() + 1 > names.len() {
            names.push(format!("and {} more", others.len() + 1 - names.len()));
        }
        let names = names.join("\n");
        let audio = audio_also(app, &others);
        let grass = Default::default();
        // (Asked of each area only in a project that keeps others' areas
        // read-only: else none is, and a world's thousands go unasked.)
        let held = crate::claims::kept(app)
            .then(|| {
                app.tools.walks += 1;
                std::iter::once(area)
                    .chain(others.iter().copied())
                    .find_map(|a| crate::claims::read_only(app, a))
            })
            .flatten();
        let set = std::sync::Arc::new(TogetherSet {
            area,
            others,
            also,
            audio,
            grass,
            first,
            names,
            held,
        });
        self.set = Some((wanted, set.clone()));
        self.made.1 += 1;
        Some(set)
    }
}

/// Edit Areas Together's section's pages (M4's U29): Areas, the chooser,
/// and the pages Area Properties of several areas shares.
pub(crate) const TOGETHER_PAGES: [&str; 7] =
    ["Areas", "Visual", "Grass", "Audio", "Events", "Advanced", "Variables"];

/// Its Open in Tab's tip.
const TOGETHER_OPEN_TIP: &str =
    "Open the checked areas' Area Properties in a tab or window of their own, with OK and Cancel";

/// Its Cancel All's tip: the rule (ruling 66).
const CANCEL_ALL_TIP: &str =
    "Take back the edits made here since the areas were checked, up to one made elsewhere";

/// Why Cancel All is disabled: nothing made here.
const CANCEL_ALL_NONE: &str = "Nothing has been edited here since the areas were checked";

/// Why Cancel All is disabled: what was made here is under another step.
const CANCEL_ALL_UNDER: &str = "An edit made elsewhere or Undo came after the edits made here";

/// What the ticked areas' set was worked out for ([`AreaChooser`]'s
/// `set`): the module's revision, `chosen_at`, whether areas are said by
/// name, how often the project's files were read.
type SetKey = (u64, u64, bool, u64);

/// Edit › Edit Areas Together…, the Right Dock's section (M4's U29; a
/// window before, and its Edit N Together the tab of Area Properties of
/// several areas): Open in Tab and Close at its top, then a rail of its
/// pages, Areas (the checklist of the module's areas, narrowed by name,
/// tileset, kind and what they hold, kept as sets by name) and the shared
/// pages of the areas ticked, Visual to Variables, as Properties' pages are
/// shown (one at a time, applied at once, "≠" where they differ, as
/// Properties' several of one type: `blueprint::Form::mark_mixed`). Open in
/// Tab opens them as the tab of today, with OK and Cancel, and closes the
/// section. What the areas have and are shown as is worked out when the
/// module or the areas ticked change, never every frame.
pub(crate) fn together_panel(
    app: &mut Moonglow,
    ui: &mut Ui,
    detail: Option<&str>,
) -> Option<crate::tool_sections::Answer> {
    use crate::tool_sections::Answer;
    let mut c = app.area_chooser.take()?;
    // (No module: the section closes with it.)
    app.ws.as_ref()?;
    let n = c.chosen.len();
    let keys = crate::palette_view::Keys::AreasTogether;
    let main = Some(("Open in Tab", n >= 2, TOGETHER_OPEN_TIP));
    let leave = ("Close", "Close Edit Areas Together");
    // (What the last frame's pages set is applied by now.)
    app.tools.marks.clear();
    // (Cancel All between them, the dialog pattern's order: ruling 66.)
    let run = app.ws.as_ref().map_or(0, |ws| c.run_len(ws));
    let can = run > 0 || c.typing;
    let tip = match (can, c.steps_at == c.chosen_at && !c.steps.is_empty()) {
        (true, _) => CANCEL_ALL_TIP,
        (false, false) => CANCEL_ALL_NONE,
        (false, true) => CANCEL_ALL_UNDER,
    };
    let other = Some(("Cancel All", can, tip));
    let (mut answer, cancel) =
        crate::tool_sections::actions_with(ui, keys, main, other, leave, detail, None);
    if cancel {
        app.tools.cancel_all = true;
        ui.ctx().request_repaint();
    }
    c.typing = false;
    let whole = ui.available_rect_before_wrap();
    let mut rail = mg_uikit::pages::PageRail::new("areas-together-pages");
    for p in TOGETHER_PAGES {
        rail = rail.page(crate::properties_panel::page_icon(p), p);
    }
    let mut left = ui.new_child(egui::UiBuilder::new().max_rect(whole));
    let mut at = c.page.min(TOGETHER_PAGES.len() - 1);
    rail.show(&mut left, &mut at);
    c.page = at;
    let room = whole.with_min_x(whole.left() + mg_uikit::pages::PAGE_RAIL + 1.0);
    let mut body = ui.new_child(egui::UiBuilder::new().max_rect(room).id_salt("page"));
    body.set_clip_rect(room.intersect(ui.clip_rect()));
    match TOGETHER_PAGES[at] {
        // (On a scroll of its own where the room is too low for the
        // filters and a few rows of the list, the least window's: the list
        // then a few rows high below them.)
        "Areas" => {
            let view = body.available_height();
            crate::tool_sections::scrolled_in(
                &mut body,
                "areas-together-areas",
                Default::default(),
                |ui| chooser_ui(app, ui, &mut c, view),
            )
        }
        page => together_page(app, &mut body, &mut c, page),
    }
    ui.advance_cursor_after_rect(whole);
    // (Enter is Open in Tab on the Areas page alone: on a shared page it
    // is its fields'. M4's U29 review.)
    let enter_opens = c.enter_opens();
    answer = answer.or_else(|| crate::tool_sections::enter_after(ui, keys, enter_opens, false));
    if answer == Some(Answer::Main) {
        let areas = c.chosen.iter().copied().collect();
        app.actions.push(Action::OpenTab(crate::Tab::AreasProperties(areas)));
        // (Open there: the section has done its work.)
        answer = Some(Answer::Leave);
    }
    if answer != Some(Answer::Leave) {
        app.area_chooser = Some(c);
    }
    answer
}

/// The areas ticked, as their shared pages take them: kept while the
/// module and the areas ticked are as they were ([`AreaChooser::set_at`]).
#[derive(Debug)]
pub(crate) struct TogetherSet {
    /// The first area, whose values the pages show.
    area: ResRef,
    /// The others, those still in the module.
    others: Vec<ResRef>,
    /// Their ARE, as the forms set them.
    also: crate::widgets::Also,
    /// The GITs of those with audio settings, as the Audio page sets them.
    audio: crate::widgets::Also,
    /// Every area's grass settings, as the Grass page sets them: read when
    /// it is first shown.
    grass: std::sync::OnceLock<Vec<(ResKey, mg_gff::Struct)>>,
    /// The first area's name, as the module tree says it.
    first: String,
    /// Every area's name, for the tip of the line that counts them.
    names: String,
    /// Why they are read-only here, where one of them is held by another
    /// builder: asked as the set is made, not each frame.
    held: Option<String>,
}

/// A shared page of the areas ticked (Visual to Variables) in Edit Areas
/// Together's section: said for fewer than two; said where their Area
/// Properties are open in a tab or window (Show It brings it); else the
/// page ([`page_body`], the window's own code path), read-only where any
/// of them is held by another builder.
fn together_page(app: &mut Moonglow, ui: &mut Ui, c: &mut AreaChooser, page: &str) {
    let padded = |ui: &mut Ui, add: &mut dyn FnMut(&mut Ui)| {
        let margin = egui::Margin { left: 8, right: 8, top: 8, bottom: 4 };
        egui::Frame::new().inner_margin(margin).show(ui, |ui| add(ui));
    };
    if c.chosen.len() < 2 {
        padded(ui, &mut |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Check two areas or more on the Areas page to edit them together here.",
                    )
                    .weak(),
                )
                .wrap(),
            );
        });
        return;
    }
    let Some(set) = c.set(app) else { return };
    // (Area Properties of any of them open as a tab or window: the fields
    // are there, not twice, and their OK and Cancel would set back what
    // was changed here.)
    let chosen = &c.chosen;
    let theirs = |t: &crate::Tab| match t {
        crate::Tab::AreaProperties(a) => chosen.contains(a),
        crate::Tab::AreasProperties(v) => v.iter().any(|a| chosen.contains(a)),
        _ => false,
    };
    let docked = app.dock.iter_all_tabs().map(|(_, t)| t);
    if let Some(tab) = docked.chain(app.panes_tabs.iter().flatten()).find(|t| theirs(t)).cloned() {
        let mut show = false;
        padded(ui, &mut |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Area Properties of these areas are open in a tab or window of their own.",
                    )
                    .weak(),
                )
                .wrap(),
            );
            show = ui
                .button("Show It")
                .on_hover_text("Bring that tab or window to the front")
                .clicked();
        });
        if show {
            app.actions.push(Action::OpenTab(tab));
        }
        return;
    }
    let are = ResKey::new(set.area, ResType::ARE);
    let Some(root) = app.ws.as_mut().and_then(|ws| ws.doc(&are).ok()).map(|g| mg_gff::Struct {
        id: g.root.id,
        fields: (g.root.fields.iter())
            .filter(|f| f.label.as_bytes() != b"Tile_List")
            .cloned()
            .collect(),
    }) else {
        return;
    };
    let held = set.held.clone();
    let viewed = held.is_some();
    let pending = app.actions.len();
    let refused_id = egui::Id::new("areas-together-refused");
    let pass = ui.ctx().cumulative_pass_nr();
    app.panel_form = true;
    crate::tool_sections::scrolled(ui, &format!("areas-together-{page}"), |ui| {
        // (How many are ticked is the section's detail, at its top.)
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!(
                    "Shown as the first, {}; what you change is set on each (≠: they differ \
                     there).",
                    set.first
                ))
                .weak(),
            )
            .wrap(),
        )
        .on_hover_text(&set.names);
        if let Some(why) = &held {
            let now = ui.input(|i| i.time);
            let refused = ui
                .data(|d| d.get_temp::<f64>(refused_id))
                .is_some_and(|at| now - at < crate::blueprint::VIEW_REFUSED_SHOWN);
            let text = if refused {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                egui::RichText::new(format!(
                    "Not changed: {why}, and the project keeps others' areas read-only."
                ))
                .strong()
            } else {
                egui::RichText::new(format!("{why}: read-only here, shown to look at."))
            };
            ui.colored_label(ui.visuals().warn_fg_color, text);
            crate::blueprint::typing_kept_out(ui, ui.available_rect_before_wrap());
        }
        ui.add_space(4.0);
        let on = Shared {
            area: set.area,
            others: &set.others,
            also: &set.also,
            audio: Some(&set.audio),
            grass: Some(&set.grass),
            root: &root,
            first: &set.first,
            viewed,
        };
        page_body(app, ui, page, on);
    });
    app.panel_form = false;
    // (A text field of the page with the keyboard, its text changed since
    // it took the keys: what is typed in it is set as the press on Cancel
    // All leaves it, and taken back with the rest, so Cancel All is
    // enabled. Focused and unchanged, there is nothing to take.)
    let room = ui.clip_rect();
    let in_room =
        |id: egui::Id| ui.ctx().read_response(id).is_some_and(|r| room.contains_rect(r.rect));
    let focused = ui.memory(|m| m.focused());
    let field = focused.filter(|id| in_room(*id)).and_then(|id| Some((id, app.buffers.get(&id)?)));
    c.typing = match (field, &c.typed_from) {
        (Some((id, now)), Some((from_id, from))) if id == *from_id => now != from,
        (Some((id, now)), _) => {
            c.typed_from = Some((id, now.clone()));
            false
        }
        (None, _) => {
            c.typed_from = None;
            false
        }
    };
    // (A field typed in here is the section's: no window drawn after it
    // this frame claims it for its Cancel.)
    if let Some(held) = app.held_text.as_mut().filter(|h| h.window.is_none() && h.drawn == pass) {
        held.panel = true;
        // (Its Cancel All's, should the field be gone before it is left.)
        c.typing |= held.command.is_some();
        app.tools.held_mark = held.command.clone();
    }
    if viewed {
        let ours = |k: &ResKey| {
            matches!(k.restype, ResType::ARE | ResType::GIT)
                && (k.resref == set.area || set.others.contains(&k.resref))
        };
        if crate::blueprint::look_only(app, pending, &ours) {
            let now = ui.input(|i| i.time);
            ui.data_mut(|d| d.insert_temp(refused_id, now));
            ui.ctx().request_repaint();
        }
    }
    // (What the page set is the section's, its Cancel All's: ruling 66.)
    for a in app.actions.iter().skip(pending) {
        if let Action::Apply(cmd) | Action::ApplyTyped(cmd) | Action::ApplyMade(cmd, _) = a {
            app.tools.marks.push(cmd.clone());
        }
    }
}

/// Edit Areas Together's Areas page: the filters and sets over the
/// checklist of the areas shown, which holds the keys (Up, Down, Page Up,
/// Page Down, Home and End move the row chosen, Space ticks it) while the
/// section has them.
fn chooser_ui(app: &mut Moonglow, ui: &mut Ui, c: &mut AreaChooser, view: f32) {
    let module =
        crate::area_sets::module_name(app.ws.as_ref().and_then(|w| w.module.location.as_ref()));
    let sets = (c
        .sets
        .get_or_insert_with(|| crate::area_sets::list_for(app.area_set_dir.as_deref(), &module)))
    .clone();
    let music = areas_music_choices(app);
    let areas = c.listed(app);
    let tilesets = c.tilesets(app, &areas);
    let tileset_label = |t: ResRef| {
        let name = tilesets.iter().find(|(r, _)| *r == t).map_or("", |(_, n)| n.as_str());
        if name.is_empty() || name == t.to_string() {
            t.to_string()
        } else {
            format!("{name} ({t})")
        }
    };
    let margin = egui::Margin { left: 8, right: 8, top: 8, bottom: 4 };
    let width = (ui.available_width() - 16.0).max(80.0);
    // (Properties' grid, M4's U8 to U10: its label column, the fields the
    // rest of the room. M4's U29 review.)
    let field =
        (width - mg_uikit::metrics::LABEL_WIDTH.min((width * 0.5).floor()) - 12.0).max(80.0);
    let form = egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_max_width(width);
        // (One short line, the rest its tip: three lines took the list's
        // rows in the dock, M4's U29 review.)
        ui.add(
            egui::Label::new(egui::RichText::new("Check the areas to edit together.").weak())
                .wrap(),
        )
        .on_hover_text(
            "The shared pages on the rail set what you change (lighting, fog, weather, music, \
                 scripts, variables…) on each area checked.",
        );
        // (A choice's words cut to its width rather than widening the grid
        // past the room: the least Right Dock's, where the list below needs
        // the height a field under each label would take.)
        let grid =
            crate::blueprint::form_grid(ui, "area-chooser-filters").labels().spacing([12.0, 6.0]);
        grid.show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            // (The labels whole: their column is as wide as the widest.)
            let whole_label = |ui: &mut Ui, text: &str| {
                ui.scope(|ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    crate::widgets::field_label(ui, text)
                })
                .inner
            };
            whole_label(ui, "Filter");
            crate::widgets::SearchField::new(egui::Id::new("area-chooser-filter"), &mut c.filter)
                .hint("Name or ResRef")
                .width(field)
                .find_over()
                .show(ui);
            ui.end_row();
            whole_label(ui, "Tileset");
            let options: Vec<_> =
                std::iter::once(None).chain(tilesets.iter().map(|(t, _)| Some(*t))).collect();
            crate::widgets::dropdown("area-chooser-tileset")
                .width(field)
                .selected_text(c.tileset.map_or("Any".to_string(), tileset_label))
                .show(ui, &mut c.tileset, &options, |t| t.map_or("Any".to_string(), tileset_label));
            ui.end_row();
            for (k, (_, either, yes, no)) in FLAG_FILTERS.iter().enumerate() {
                whole_label(ui, yes);
                let text = |v: Option<bool>| match v {
                    None => *either,
                    Some(true) => *yes,
                    Some(false) => *no,
                };
                let options = [None, Some(true), Some(false)];
                crate::widgets::dropdown(("area-chooser-flag", k))
                    .width(field)
                    .selected_text(text(c.flags[k]))
                    .show(ui, &mut c.flags[k], &options, |v| text(*v));
                ui.end_row();
            }
            whole_label(ui, "Holding")
                .on_hover_text("Areas with this music, variable or field value (* as in tags)");
            crate::widgets::dropdown("area-chooser-hold")
                .width(field)
                .selected_text(c.hold.label())
                .show(ui, &mut c.hold, &Hold::ALL, |h| h.label());
            ui.end_row();
            match c.hold {
                Hold::Anything => {}
                Hold::Music => {
                    ui.label("");
                    crate::search::row_picker(ui, "area-chooser-music", &music, &mut c.music, None);
                    ui.end_row();
                }
                Hold::Variable | Hold::Field => {
                    let (name, hint) = match c.hold {
                        Hold::Variable => ("Name", "tdod_rental, kz_*"),
                        _ => ("Label", "MusicDay, Fog*, OnEnter"),
                    };
                    whole_label(ui, name);
                    ui.add(
                        mg_uikit::text_field(&mut c.hold_name).hint_text(hint).desired_width(field),
                    );
                    ui.end_row();
                    whole_label(ui, "Value");
                    ui.add(
                        mg_uikit::text_field(&mut c.hold_value)
                            .hint_text("any")
                            .desired_width(field),
                    )
                    .on_hover_text("Its value (* as in tags). Empty: any.");
                    ui.end_row();
                }
            }
        });
        // The sets kept for this module, and the ticked kept as one.
        ui.horizontal_wrapped(|ui| {
            let mut picked: Option<String> = None;
            let options: Vec<Option<String>> = sets.iter().cloned().map(Some).collect();
            crate::widgets::dropdown("area-chooser-sets")
                .selected_text(if sets.is_empty() { "No sets kept" } else { "Check a Set…" })
                .width(140.0)
                .show(ui, &mut picked, &options, |s| s.clone().unwrap_or_default())
                .on_hover_text("Checks the areas of a set kept for this module, as they are");
            if let Some(name) = picked {
                match crate::area_sets::read(app.area_set_dir.as_deref(), &name) {
                    Ok(set) => {
                        let there: std::collections::HashSet<ResRef> =
                            areas.iter().map(|(a, ..)| *a).collect();
                        c.chosen = set.into_iter().filter(|a| there.contains(a)).collect();
                        c.ticked();
                    }
                    Err(e) => app.log.error(format!("The set {name}: {e}")),
                }
            }
            ui.add(
                mg_uikit::text_field(&mut c.set_name).hint_text("set name").desired_width(120.0),
            );
            let can = !c.chosen.is_empty() && crate::saved::valid_name(&c.set_name);
            if ui
                .add_enabled(can, egui::Button::new("Save Set"))
                .on_hover_text("Keeps the checked areas as a set under this name, for this module")
                .clicked()
            {
                let name = c.set_name.trim().to_string();
                let dir = app.area_set_dir.as_deref();
                match crate::area_sets::save(dir, &name, &module, c.chosen.iter().copied()) {
                    Ok(_) => {
                        app.log.info(format!("Kept {} areas as the set {name}", c.chosen.len()))
                    }
                    Err(e) => app.log.error(format!("Save Set: {e}")),
                }
                c.sets = None;
            }
            if sets.iter().any(|s| s.eq_ignore_ascii_case(c.set_name.trim()))
                && ui.button("Delete Set").on_hover_text("Deletes the set of this name").clicked()
            {
                let name = c.set_name.trim().to_string();
                match crate::area_sets::delete(app.area_set_dir.as_deref(), &name) {
                    Ok(()) => app.log.info(format!("Deleted the set {name}")),
                    Err(e) => app.log.error(format!("Delete Set: {e}")),
                }
                c.sets = None;
            }
        });
        // (What Holding asks as its fields say it now, above: asked before
        // them, the rows shown were kept for the value typed with the areas
        // of the value before.)
        let holding = c.holding(app).cloned();
        let shown = c.shown(&areas, holding.as_deref());
        ui.horizontal_wrapped(|ui| {
            if ui.button("Check Shown").clicked() {
                c.chosen.extend(shown.iter().map(|&i| areas[i].0));
                c.ticked();
            }
            if ui.button("Uncheck Shown").clicked() {
                for &i in shown.iter() {
                    c.chosen.remove(&areas[i].0);
                }
                c.ticked();
            }
            if ui.button("Uncheck All").clicked() {
                c.chosen.clear();
                c.ticked();
            }
            ui.weak(format!("{} of {} areas shown", shown.len(), areas.len()));
        });
        shown
    });
    let shown = form.inner;
    let t = mg_uikit::tokens(ui.ctx());
    let line = ui.min_rect().bottom() + 2.0;
    ui.painter().hline(ui.max_rect().x_range(), line, egui::Stroke::new(1.0, t.line));
    ui.add_space(4.0);
    // (What is left of the room in sight, and six rows at the least.)
    let used = ui.min_rect().height();
    let high = (view - used).max(6.0 * mg_uikit::row_pitch(ui));
    let size = egui::vec2(ui.available_width(), high);
    ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_min_height(high);
        checklist(ui, c, &areas, &shown);
    });
}

/// The checklist of the areas shown (Edit Areas Together's Areas page): the
/// rows in sight alone laid out (a world has thousands of areas), every row
/// reached by the keys while the section has them, the row they choose
/// brought into sight and outlined.
fn checklist(ui: &mut Ui, c: &mut AreaChooser, areas: &Listed, shown: &[usize]) {
    let keys = crate::palette_view::Keys::AreasTogether;
    let pitch = mg_uikit::row_pitch(ui);
    let room = ui.available_rect_before_wrap();
    let lines = ((room.height() / pitch).floor() as usize).max(1);
    let at = c.cursor.and_then(|a| shown.iter().position(|&i| areas[i].0 == a));
    let mut moved = None;
    c.rows = !shown.is_empty();
    // (Not under a list a box of the filters dropped down, which egui
    // draws: `holds_keys`.)
    if crate::tool_sections::holds_keys(ui.ctx(), keys)
        && crate::report_table::has_keys(ui.ctx(), keys)
        && !shown.is_empty()
    {
        use egui::Key;
        let last = shown.len() - 1;
        let now = at.unwrap_or(0);
        let to = ui.input(|i| {
            let page = lines.saturating_sub(1).max(1);
            if i.key_pressed(Key::ArrowDown) {
                Some(if at.is_none() { 0 } else { (now + 1).min(last) })
            } else if i.key_pressed(Key::ArrowUp) {
                Some(now.saturating_sub(1))
            } else if i.key_pressed(Key::PageDown) {
                Some((now + page).min(last))
            } else if i.key_pressed(Key::PageUp) {
                Some(now.saturating_sub(page))
            } else if i.key_pressed(Key::Home) {
                Some(0)
            } else if i.key_pressed(Key::End) {
                Some(last)
            } else {
                None
            }
        });
        if let Some(to) = to {
            c.cursor = Some(areas[shown[to]].0);
            moved = Some(to);
        }
        let space = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Space));
        if space && let Some(a) = c.cursor.filter(|_| at.is_some() || moved.is_some()) {
            if !c.chosen.remove(&a) {
                c.chosen.insert(a);
            }
            c.ticked();
        }
    }
    let mut area = egui::ScrollArea::vertical()
        .id_salt("areas-together-list")
        .auto_shrink([false, false])
        .min_scrolled_height(0.0);
    // (The row the keys moved to, whole in sight.)
    if let Some(to) = moved {
        let (offset, high) = c.list_seen;
        let top = to as f32 * pitch;
        let want = if top < offset {
            Some(top)
        } else if top + pitch > offset + high {
            Some(top + pitch - high)
        } else {
            None
        };
        if let Some(want) = want {
            area = area.vertical_scroll_offset(want.max(0.0));
        }
    }
    let cursor = c.cursor;
    let mut toggled: Option<ResRef> = None;
    let out = area.show_rows(ui, pitch - ui.spacing().item_spacing.y, shown.len(), |ui, range| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let margin = egui::Margin { left: 8, right: 8, top: 0, bottom: 0 };
        egui::Frame::new().inner_margin(margin).show(ui, |ui| {
            for &i in &shown[range] {
                let (a, _, label) = &areas[i];
                let mut on = c.chosen.contains(a);
                let r = ui.check_box(&mut on, label.as_str());
                if r.changed() {
                    toggled = Some(*a);
                }
                if r.clicked() || r.changed() {
                    crate::palette_view::give_keys(ui.ctx(), keys);
                }
                if cursor == Some(*a) {
                    let row =
                        egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), r.rect.y_range());
                    let t = mg_uikit::tokens(ui.ctx());
                    ui.painter().rect_stroke(
                        row.expand(1.0),
                        2.0,
                        egui::Stroke::new(1.0, t.accent),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        });
    });
    c.list_seen = (out.state.offset.y, out.inner_rect.height());
    if let Some(a) = toggled {
        if !c.chosen.remove(&a) {
            c.chosen.insert(a);
        }
        c.cursor = Some(a);
        c.ticked();
    }
}

/// The music Holding chooses from: ambientmusic.2da's rows, as the game
/// names them (none without game data: a number).
fn areas_music_choices(app: &Moonglow) -> std::sync::Arc<Vec<mg_rules::Choice>> {
    let table = mg_module::areas::row_table("MusicDay").expect("music is a row");
    crate::search::listed(app.game.as_deref(), &table)
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
            tag: "WOLF_DEN".into(),
            tileset: r("tdc01"),
            flags: INTERIOR | UNDERGROUND | NATURAL,
            ..Default::default()
        };
        let inn = AreaInfo {
            name: "Apple Inn".into(),
            tileset: r("tin01"),
            flags: INTERIOR,
            ..Default::default()
        };
        let all = AreaChooser::default();
        assert!(all.shows(r("area467"), &cave) && all.shows(r("area002"), &inn));
        // A name or a ResRef, whatever its case.
        let text = |t: &str| AreaChooser { filter: t.into(), ..Default::default() };
        assert!(text("wolf").shows(r("area467"), &cave) && !text("wolf").shows(r("area002"), &inn));
        assert!(text(" A467 ").shows(r("area467"), &cave));
        // Or its tag, as `mg areas` finds it.
        assert!(text("den").shows(r("area467"), &cave) && !text("den").shows(r("area002"), &inn));
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
