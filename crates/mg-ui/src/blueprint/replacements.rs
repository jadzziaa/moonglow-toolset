//! The Visuals page's texture and animation replacements (`TextureReplace`,
//! `AnimationReplace`: EE's `ReplaceObjectTexture` and
//! `ReplaceObjectAnimation`), picked from what the object's models have:
//! the textures their meshes use, each with its picture and Select Texture
//! (a gallery of the game's and the haks' textures) to draw it with
//! another; the animations of its model and supermodels. Entries the
//! models lack are kept and shown, and any name can be added as text.
//!
//! The lists are written as the game writes them: an entry edited in
//! place, a new one at the end (struct 0, the old name, then the new).

use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_preview::replaceable::ModelTexture;
use mg_rules::Choice;

use super::Form;
use super::visuals::{ANIMATION_REPLACE_ID, TEXTURE_REPLACE_ID};

/// A replacement list's labels.
pub(super) struct Pairs {
    what: &'static str,
    field: &'static str,
    id: u32,
    list: &'static str,
    old: &'static str,
    new: &'static str,
    titles: (&'static str, &'static str),
}

impl Pairs {
    const TEXTURES: Pairs = Pairs {
        what: "Texture replacement",
        field: "TextureReplace",
        id: TEXTURE_REPLACE_ID,
        list: "TextureReplaceLi",
        old: "OldTexture",
        new: "NewTexture",
        titles: ("Texture", "Drawn as"),
    };
    const ANIMATIONS: Pairs = Pairs {
        what: "Animation replacement",
        field: "AnimationReplace",
        id: ANIMATION_REPLACE_ID,
        list: "AnimationReplace",
        old: "OldAnimation",
        new: "NewAnimation",
        titles: ("Animation", "Played as"),
    };

    fn items(&self, f: &Form<'_>) -> Vec<Struct> {
        f.nested_list(self.field, self.list)
    }

    fn set(&self, f: &mut Form<'_>, items: Vec<Struct>) {
        f.set_nested_list(self.what, self.field, self.id, self.list, items);
    }

    /// A new entry: `old` drawn or played as `new`.
    fn entry(&self, old: ResRef, new: ResRef) -> Struct {
        let mut e = Struct::new(0);
        e.set(self.old, Value::resref(old));
        e.set(self.new, Value::resref(new));
        e
    }

    /// `old` replaced by `new`: its entry (`entry`) given the new name,
    /// else a new entry at the end.
    fn replace(&self, f: &mut Form<'_>, entry: Option<usize>, old: &str, new: ResRef) {
        let Ok(old_ref) = ResRef::from_str(old) else { return };
        let mut items = self.items(f);
        match entry.filter(|&i| i < items.len()) {
            Some(i) => {
                if resref_text(&items[i], self.new) == new.to_string() {
                    return;
                }
                items[i].set(self.new, Value::resref(new));
            }
            None if new.is_empty() => return,
            None => items.push(self.entry(old_ref, new)),
        }
        self.set(f, items);
    }
}

/// The chest's slot in a creature's `Equip_ItemList` (its struct id): the
/// armor, whose parts are the body's models.
const CHEST_SLOT: u32 = 0x2;

/// Why a PLT texture has no replacement to pick.
const PLT_WHY: &str = "A PLT texture: the game draws it in the object's colors and doesn't \
    replace it (ReplaceObjectTexture draws with simple textures, DDS or TGA, not PLTs)";

/// Said of an entry whose texture or animation the object's models don't have.
const NOT_IN_THE_MODEL: &str = "not in the model";

/// The side of a texture's picture in a row, points.
const ROW_PICTURE: f32 = 32.0;
/// The size the pictures are made at, pixels.
const ROW_THUMB: u32 = 64;
const GALLERY_THUMB: u32 = 128;
const PREVIEW_THUMB: u32 = 256;
/// How long a frame may spend making the gallery's pictures.
const PICTURE_BUDGET: Duration = Duration::from_millis(6);

fn resref_text(s: &Struct, label: &str) -> String {
    s.resref(label).map(|r| r.to_string()).unwrap_or_default()
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// What the object's models offer its replacements.
#[derive(Clone, Default)]
struct Offered {
    textures: Arc<Vec<ModelTexture>>,
    animations: Arc<Vec<String>>,
    /// Why there is nothing: the object has no model to read.
    problem: Option<String>,
}

/// The textures and animations of the object's models, read again only
/// when the module or the game data changes (reading a creature's part
/// models each frame would show in its frame time). A creature is its own
/// body: its appearance's model, or, for a body of parts, the parts its
/// armor or its own fields name (whose textures are PLTs), its head, wings
/// and tail; not its weapons, helmet or cloak, which are items' models.
fn offered(f: &Form<'_>, ui: &Ui) -> Offered {
    let Some(game) = f.app.game.clone() else { return Offered::default() };
    let id = f.id("replacements-offered");
    let stamp: Stamp = (f.app.ws.as_ref().map(|w| w.revision()), Arc::as_ptr(&game) as usize);
    if let Some((was, o)) = ui.data(|d| d.get_temp::<(Stamp, Offered)>(id))
        && was == stamp
    {
        return o;
    }
    let preview = match f.restype() {
        ResType::UTP => mg_preview::placeable(&game, &f.root),
        ResType::UTD => mg_preview::door(&game, &f.root),
        ResType::UTC => {
            // What it wears in its chest slot alone.
            let mut body = f.root.clone();
            if let Some(Value::List(slots)) = body.get_mut("Equip_ItemList") {
                slots.retain(|s| s.id == CHEST_SLOT);
            }
            let armor = |r: ResRef| -> Option<mg_gff::Gff> {
                let key = mg_resman::ResKey::new(r, ResType::UTI);
                let module = f.app.ws.as_ref().and_then(|w| w.module.get(&key).map(<[u8]>::to_vec));
                let data = module.or_else(|| game.resman.get(&key).ok().map(|d| d.into_owned()))?;
                mg_gff::Gff::read(&data).ok()
            };
            mg_preview::creature(&game, &body, &armor)
        }
        ResType::UTI => mg_preview::item(&game, &f.root),
        t => Err(mg_preview::PreviewError::NoModel(format!("{t:?}"))),
    };
    let o = match preview {
        Ok(p) => Offered {
            textures: Arc::new(mg_preview::replaceable::textures(&game, &p)),
            animations: Arc::new(mg_preview::replaceable::animations(&game, &p)),
            problem: None,
        },
        Err(e) => Offered { problem: Some(e.to_string()), ..Default::default() },
    };
    ui.data_mut(|d| d.insert_temp(id, (stamp, o.clone())));
    o
}

/// What [`offered`] was read for: the module's revision and the game data.
type Stamp = (Option<u64>, usize);

/// A row of the texture table: a texture of the models, or an entry whose
/// texture they don't have.
struct Row<'a> {
    name: String,
    of_model: Option<&'a ModelTexture>,
    /// The entry that replaces it.
    entry: Option<usize>,
}

/// A texture's small picture, `side` points square (a PLT's in the
/// object's colors); a blank where there is none.
fn small_picture(f: &mut Form<'_>, ui: &mut Ui, name: &str, plt: Option<[u8; 10]>, side: f32) {
    let picture = f.app.loader().and_then(|mut l| match plt {
        Some(colors) => l.plt_picture(ui.ctx(), name, colors),
        None => l.thumbnail(ui.ctx(), name, ROW_THUMB),
    });
    let size = egui::vec2(side, side);
    match picture {
        Some(p) => {
            ui.add(egui::Image::new(&p.texture).fit_to_exact_size(size).alt_text(name));
        }
        None => {
            ui.allocate_exact_size(size, egui::Sense::hover());
        }
    }
}

/// Texture replacements: the models' textures, each with what it is drawn
/// as, Select Texture… and Reset; then the entries for textures the models
/// lack, and a row to add any name.
pub(super) fn textures(f: &mut Form<'_>, ui: &mut Ui) {
    let p = Pairs::TEXTURES;
    let offered = offered(f, ui);
    let items = p.items(f);
    let mut claimed = vec![false; items.len()];
    let mut rows: Vec<Row<'_>> = offered
        .textures
        .iter()
        .map(|t| {
            let entry = items.iter().position(|e| same(&resref_text(e, p.old), &t.name));
            if let Some(i) = entry {
                claimed[i] = true;
            }
            Row { name: t.name.clone(), of_model: Some(t), entry }
        })
        .collect();
    for (i, e) in items.iter().enumerate().filter(|(i, _)| !claimed[*i]) {
        let name = resref_text(e, p.old);
        let of_model = offered.textures.iter().find(|t| same(&t.name, &name));
        rows.push(Row { name, of_model, entry: Some(i) });
    }
    if let Some(problem) = &offered.problem {
        ui.weak(format!("No model to list the textures of: {problem}"));
    }
    let mut changed: Option<Vec<Struct>> = None;
    let mut pick: Option<(Option<usize>, String, String)> = None;
    let mut set_new: Option<(Option<usize>, String, ResRef)> = None;
    egui::Grid::new(f.id(p.field)).num_columns(5).spacing([10.0, 4.0]).show(ui, |ui| {
        if !rows.is_empty() {
            ui.label("");
            crate::widgets::field_label(ui, p.titles.0);
            ui.label("");
            crate::widgets::field_label(ui, p.titles.1);
            ui.end_row();
        }
        for row in &rows {
            let plt = row.of_model.filter(|t| t.plt).map(|t| t.colors.unwrap_or_default());
            small_picture(f, ui, &row.name, plt, ROW_PICTURE);
            ui.horizontal(|ui| {
                let name = ui.label(&row.name);
                if let Some(t) = row.of_model {
                    name.on_hover_text(format!("On {}", t.models.join(", ")));
                }
                if row.of_model.is_none() {
                    ui.weak(format!("({NOT_IN_THE_MODEL})")).on_hover_text(
                        "A replacement of a texture the object's models don't have: kept as \
                         it is",
                    );
                } else if plt.is_some() {
                    ui.weak("PLT").on_hover_text(PLT_WHY);
                }
            });
            let new = row.entry.map(|i| resref_text(&items[i], p.new)).unwrap_or_default();
            if new.is_empty() {
                ui.allocate_exact_size(egui::vec2(ROW_PICTURE, ROW_PICTURE), egui::Sense::hover());
            } else {
                small_picture(f, ui, &new, None, ROW_PICTURE);
            }
            ui.horizontal(|ui| {
                let id = f.id(&format!("{}#{}#{:?}#new", p.field, row.name, row.entry));
                if plt.is_some() && row.entry.is_none() {
                    ui.add_enabled(false, egui::Button::new("Select Texture…"))
                        .on_disabled_hover_text(PLT_WHY);
                } else {
                    if let Some(v) = crate::widgets::commit_text_in(
                        &mut f.app.buffers,
                        ui,
                        id,
                        &new,
                        None,
                        140.0,
                        Some(16),
                    ) && let Ok(r) = ResRef::from_str(v.trim())
                    {
                        set_new = Some((row.entry, row.name.clone(), r));
                    }
                    let button =
                        ui.add_enabled(plt.is_none(), egui::Button::new("Select Texture…"));
                    if button
                        .on_hover_text("Pick the texture to draw it with")
                        .on_disabled_hover_text(PLT_WHY)
                        .clicked()
                    {
                        pick = Some((row.entry, row.name.clone(), new.clone()));
                    }
                }
                if let Some(i) = row.entry {
                    let reset = ui
                        .small_button("Reset")
                        .on_hover_text("Remove the replacement: the model's own texture");
                    if reset.clicked() {
                        let mut all = items.clone();
                        all.remove(i);
                        changed = Some(all);
                    }
                }
            });
            ui.end_row();
        }
    });
    if let Some((entry, old, new)) = pick {
        open_gallery(f, ui, entry, old, new);
    }
    if let Some(picked) = gallery(f, ui) {
        set_new = Some(picked);
    }
    if let Some((entry, old, new)) = set_new {
        p.replace(f, entry, &old, new);
    } else if let Some(all) = changed {
        p.set(f, all);
    }
    add_row(f, ui, &p, &items);
}

/// The row to add an entry of any names, typed.
fn add_row(f: &mut Form<'_>, ui: &mut Ui, p: &Pairs, items: &[Struct]) {
    let ids = [p.old, p.new].map(|l| f.id(&format!("{}#new#{l}", p.field)));
    ui.horizontal(|ui| {
        ui.weak("Any name:");
        for (id, hint) in ids.into_iter().zip([p.titles.0, p.titles.1]) {
            let buf = f.app.buffers.entry(id).or_default();
            ui.add(
                egui::TextEdit::singleline(buf).char_limit(16).desired_width(140.0).hint_text(hint),
            );
        }
        let text = |f: &Form<'_>, id| {
            f.app.buffers.get(&id).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        let (old, new) = (text(f, ids[0]), text(f, ids[1]));
        let valid =
            ResRef::from_str(&old).is_ok_and(|r| !r.is_empty()) && ResRef::from_str(&new).is_ok();
        if ui.add_enabled(valid, egui::Button::new("Add").small()).clicked() {
            let e = p.entry(
                ResRef::from_str(&old).expect("checked"),
                ResRef::from_str(&new).expect("checked"),
            );
            let mut all = items.to_vec();
            all.push(e);
            p.set(f, all);
            for id in ids {
                f.app.buffers.remove(&id);
            }
        }
    });
}

/// Select Texture's state.
#[derive(Clone, Default)]
struct TexturePick {
    /// The model's texture it picks a replacement for, and its entry.
    old: String,
    entry: Option<usize>,
    filter: String,
    chosen: Option<String>,
    /// The textures to pick from (lower case, by name).
    names: Arc<Vec<String>>,
    /// Scrolled to the one chosen, once.
    placed: bool,
    /// The pictures it made (their keys in the app's pictures), oldest
    /// first: let go of when it closes, and the oldest out of sight past
    /// [`PICTURES_KEPT`] (a scroll through every texture kept gigabytes).
    made: Vec<String>,
}

/// How many of its pictures Select Texture keeps while open (64 KB each).
const PICTURES_KEPT: usize = 1500;

/// A picture's key in the app's pictures ([`crate::images::Loader::thumbnail`]).
fn thumb_key(name: &str, side: u32) -> String {
    format!("{}#thumb{side}", name.to_ascii_lowercase())
}

/// The textures a replacement can draw with: the game's, the haks' and the
/// module's DDS and TGA textures, each name once.
fn texture_names(app: &crate::Moonglow) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    if let Some(g) = &app.game {
        for t in [ResType::DDS, ResType::TGA] {
            names.extend(g.resman.list(t).into_iter().map(|r| r.to_lowercase().to_string()));
        }
    }
    if let Some(ws) = &app.ws {
        names.extend(
            ws.module
                .keys()
                .filter(|k| matches!(k.restype, ResType::DDS | ResType::TGA))
                .map(|k| k.resref.to_lowercase().to_string()),
        );
    }
    names.sort_unstable();
    names.dedup();
    names
}

fn open_gallery(f: &mut Form<'_>, ui: &Ui, entry: Option<usize>, old: String, current: String) {
    let names = Arc::new(texture_names(f.app));
    let chosen = (!current.is_empty()).then(|| current.to_ascii_lowercase());
    let state = TexturePick { old, entry, chosen, names, ..Default::default() };
    ui.data_mut(|d| d.insert_temp(f.id("texture-gallery"), state));
}

/// Select Texture, while it is open: Find, the textures as pictures (made
/// as they come into sight, a few milliseconds' worth a frame), the one
/// chosen larger; OK gives (the model's texture, the one picked).
fn gallery(f: &mut Form<'_>, ui: &mut Ui) -> Option<(Option<usize>, String, ResRef)> {
    let id = f.id("texture-gallery");
    let mut state: TexturePick = ui.data(|d| d.get_temp(id))?;
    let needle = mg_core::text::Needle::new(&state.filter);
    let names = state.names.clone();
    let shown: Vec<&String> = names.iter().filter(|n| needle.finds(n)).collect();
    let (mut ok, mut cancel, mut wanted, mut in_sight) = (false, false, Vec::new(), Vec::new());
    let wide = (ui.ctx().content_rect().width() * 0.8).clamp(480.0, 960.0);
    let tall = (ui.ctx().content_rect().height() * 0.6).clamp(300.0, 700.0);
    let side = crate::appearance_gallery::tile_side(f.app);
    let modal = egui::Modal::new(id.with("modal")).show(ui.ctx(), |ui| {
        ui.set_width(wide);
        ui.heading("Select Texture");
        ui.label(format!("To draw {} with", state.old));
        ui.horizontal(|ui| {
            let field = id.with("filter");
            crate::widgets::pane_filter_keys(ui, field, &mut state.filter);
            ui.add(
                egui::TextEdit::singleline(&mut state.filter)
                    .id(field)
                    .hint_text("Find")
                    .desired_width(180.0),
            );
            crate::appearance_gallery::size_slider(f.app, ui);
            ui.weak(format!("{} of {} textures", shown.len(), state.names.len())).on_hover_text(
                "DDS and TGA textures of the game, the haks and the module (no \
                    PLTs: the game replaces textures with simple ones alone)",
            );
        });
        ui.separator();
        let preview = 200.0;
        ui.horizontal_top(|ui| {
            let gap = 4.0;
            let room = wide - preview - 3.0 * ui.spacing().item_spacing.x;
            let room_inside = room - ui.spacing().scroll.bar_width - 10.0;
            let (per_row, side) = crate::appearance_gallery::fitted(room_inside, side, gap);
            let height = side + crate::widgets::tile_label_height(ui);
            let layout = egui::Layout::top_down(egui::Align::Min);
            ui.allocate_ui_with_layout(egui::vec2(room, tall), layout, |ui| {
                ui.set_width(room);
                let mut area = egui::ScrollArea::vertical()
                    .id_salt(id.with("grid"))
                    .max_height(tall)
                    .auto_shrink([false, false]);
                if !state.placed {
                    state.placed = true;
                    if let Some(at) =
                        state.chosen.as_ref().and_then(|c| shown.iter().position(|n| *n == c))
                    {
                        area = area.vertical_scroll_offset(
                            (at / per_row) as f32 * (height + ui.spacing().item_spacing.y),
                        );
                    }
                }
                area.show_rows(ui, height, shown.len().div_ceil(per_row), |ui, lines| {
                    ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                    for line in lines {
                        ui.horizontal(|ui| {
                            for name in shown.iter().skip(line * per_row).take(per_row) {
                                let made = picture_made(f, name);
                                let chosen = state.chosen.as_deref() == Some(name.as_str());
                                let (r, seen) =
                                    crate::widgets::picture_tile(ui, side, name, chosen, made);
                                if seen {
                                    in_sight.push(thumb_key(name, GALLERY_THUMB));
                                }
                                if seen && made.is_none() {
                                    wanted.push((*name).clone());
                                }
                                if r.clicked() {
                                    state.chosen = Some((*name).clone());
                                }
                                if r.double_clicked() {
                                    state.chosen = Some((*name).clone());
                                    ok = true;
                                }
                            }
                        });
                    }
                });
            });
            ui.vertical(|ui| {
                ui.set_width(preview);
                if let Some(c) = state.chosen.clone() {
                    let key = thumb_key(&c, PREVIEW_THUMB);
                    if !state.made.contains(&key) {
                        state.made.push(key);
                    }
                    let picture =
                        f.app.loader().and_then(|mut l| l.thumbnail(ui.ctx(), &c, PREVIEW_THUMB));
                    if let Some(p) = picture {
                        let size = egui::vec2(preview, preview);
                        ui.add(egui::Image::new(&p.texture).fit_to_exact_size(size).alt_text(&c));
                    }
                    ui.label(&c);
                }
            });
        });
        ui.horizontal(|ui| {
            let any = state.chosen.is_some();
            ok |= ui
                .add_enabled(any, egui::Button::new("OK"))
                .on_hover_text("Accept changes")
                .clicked()
                || (any && crate::widgets::enter(ui));
            cancel = crate::widgets::cancel_discard(ui);
        });
    });
    // The pictures in sight not made yet: as many as a few milliseconds
    // make, the rest in the frames after.
    let started = Instant::now();
    let mut left = wanted.len();
    if let Some(mut l) = f.app.loader() {
        for name in &wanted {
            if started.elapsed() > PICTURE_BUDGET {
                break;
            }
            l.thumbnail(ui.ctx(), name, GALLERY_THUMB);
            state.made.push(thumb_key(name, GALLERY_THUMB));
            left -= 1;
        }
    }
    if left > 0 {
        ui.ctx().request_repaint_after(Duration::from_millis(10));
    }
    if state.made.len() > PICTURES_KEPT {
        let mut over = state.made.len() - PICTURES_KEPT;
        state.made.retain(|k| {
            let gone = over > 0 && !in_sight.contains(k);
            if gone {
                f.app.pictures.remove(k);
                over -= 1;
            }
            !gone
        });
    }
    let picked = state
        .chosen
        .as_deref()
        .filter(|_| ok)
        .and_then(|c| ResRef::from_str(c).ok())
        .map(|r| (state.entry, state.old.clone(), r));
    if ok || cancel || modal.should_close() {
        for k in &state.made {
            f.app.pictures.remove(k);
        }
        ui.data_mut(|d| d.remove::<TexturePick>(id));
    } else {
        ui.data_mut(|d| d.insert_temp(id, state));
    }
    picked
}

/// A gallery picture: made (`Some(None)`: there is none), or not yet.
fn picture_made(f: &mut Form<'_>, name: &str) -> Option<Option<egui::TextureId>> {
    f.app.pictures.get(&thumb_key(name, GALLERY_THUMB)).map(|p| p.as_ref().map(|p| p.texture.id()))
}

/// Animation replacements: each entry's two animations, picked from those
/// of the object's model and supermodels (an entry's own kept, marked, if
/// they have it not), with Reset; a row to pick a new one, and a row to
/// add any names.
pub(super) fn animations(f: &mut Form<'_>, ui: &mut Ui) {
    let p = Pairs::ANIMATIONS;
    let offered = offered(f, ui);
    let items = p.items(f);
    // The choices: the models' animations, then the entries' names they
    // lack.
    let mut names: Vec<String> = offered.animations.to_vec();
    let known = names.len();
    for e in &items {
        for label in [p.old, p.new] {
            let n = resref_text(e, label).to_ascii_lowercase();
            if !n.is_empty() && !names.contains(&n) {
                names.push(n);
            }
        }
    }
    let choices: Vec<Choice> = names
        .iter()
        .enumerate()
        .map(|(row, n)| {
            let text = if row < known { n.clone() } else { format!("{n} ({NOT_IN_THE_MODEL})") };
            Choice { row, text }
        })
        .collect();
    let row_of = |n: &str| names.iter().position(|m| same(m, n)).map(|r| r as i64);
    if offered.animations.is_empty() {
        ui.weak(match &offered.problem {
            Some(problem) => format!("No model to list the animations of: {problem}"),
            None => "The object's model has no animations.".to_string(),
        });
    }
    let mut changed: Option<Vec<Struct>> = None;
    let pending_id = f.id("AnimationReplace#pending");
    let mut pending: (Option<i64>, Option<i64>) =
        ui.data(|d| d.get_temp(pending_id)).unwrap_or_default();
    egui::Grid::new(f.id(p.field)).num_columns(3).spacing([12.0, 4.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, p.titles.0);
        crate::widgets::field_label(ui, p.titles.1);
        ui.end_row();
        for (i, e) in items.iter().enumerate() {
            for label in [p.old, p.new] {
                let now = resref_text(e, label);
                let id = f.id(&format!("{}#{i}#{label}", p.field));
                let list = crate::widgets::row_choice(id, &choices, row_of(&now)).width(180.0);
                let list = if now.is_empty() { list.unset("(the original)") } else { list };
                if let Some(r) = list.show(ui)
                    && let Some(r) = names.get(r as usize).and_then(|n| ResRef::from_str(n).ok())
                {
                    let mut all = items.clone();
                    all[i].set(label, Value::resref(r));
                    changed = Some(all);
                }
            }
            let reset = ui
                .small_button("Reset")
                .on_hover_text("Remove the replacement: the model's own animation");
            if reset.clicked() {
                let mut all = items.clone();
                all.remove(i);
                changed = Some(all);
            }
            ui.end_row();
        }
        // A new one, picked.
        for (k, slot) in [&mut pending.0, &mut pending.1].into_iter().enumerate() {
            let id = f.id(&format!("{}#pick#{k}", p.field));
            let unset = if k == 0 { "Choose an animation" } else { "Choose the one to play" };
            let list = crate::widgets::row_choice(id, &choices, *slot).width(180.0).unset(unset);
            if let Some(r) = list.show(ui) {
                *slot = Some(r);
            }
        }
        let name = |r: Option<i64>| {
            r.and_then(|r| names.get(r as usize)).and_then(|n| ResRef::from_str(n).ok())
        };
        let (old, new) = (name(pending.0), name(pending.1));
        if ui
            .add_enabled(old.is_some() && new.is_some(), egui::Button::new("Add").small())
            .clicked()
            && let (Some(old), Some(new)) = (old, new)
        {
            let mut all = items.clone();
            all.push(p.entry(old, new));
            changed = Some(all);
            pending = (None, None);
        }
        ui.end_row();
    });
    ui.data_mut(|d| d.insert_temp(pending_id, pending));
    if let Some(all) = changed {
        p.set(f, all);
    }
    add_row(f, ui, &p, &items);
}
