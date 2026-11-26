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

use mg_uikit::Controls as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_preview::replaceable::{ModelTexture, Source};
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
        if let Some(items) = self.replaced(self.items(f), entry, old, new) {
            self.set(f, items);
        }
    }

    /// `items` with `old` replaced by `new` ([`replace`](Self::replace));
    /// `None` where nothing changes. `entry` is where `old`'s entry was
    /// when the pick was begun (Select Texture's window doesn't block):
    /// where another's is there by now (one before it reset behind the
    /// window), `old`'s is found by its name, and where it is gone a new
    /// one is made.
    fn replaced(
        &self,
        mut items: Vec<Struct>,
        entry: Option<usize>,
        old: &str,
        new: ResRef,
    ) -> Option<Vec<Struct>> {
        let old_ref = ResRef::from_str(old).ok()?;
        let of_old = |s: &Struct| same(&resref_text(s, self.old), old);
        let at = match entry {
            Some(i) if items.get(i).is_some_and(of_old) => Some(i),
            Some(_) => items.iter().position(of_old),
            None => None,
        };
        match at {
            Some(i) => {
                if resref_text(&items[i], self.new) == new.to_string() {
                    return None;
                }
                items[i].set(self.new, Value::resref(new));
            }
            None if new.is_empty() => return None,
            None => items.push(self.entry(old_ref, new)),
        }
        Some(items)
    }
}

/// The chest's slot in a creature's `Equip_ItemList` (its struct id): the
/// armor, whose parts are the body's models.
const CHEST_SLOT: u32 = 0x2;

/// Why a PLT texture has no replacement to pick.
const PLT_WHY: &str = "A PLT, drawn in the object's colors. The replacement draws its texture as \
                       it is.";

/// Why a texture with a name longer than a resref has no replacement.
const TOO_LONG: &str = "The model names it with more than 16 characters, which a replacement can't \
                        name";

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
/// when the model/appearance fields or load order change (not on unrelated edits).
fn offered(f: &mut Form<'_>, ui: &Ui) -> Offered {
    let Some(game) = f.app.game.clone() else { return Offered::default() };
    let id = f.id("replacements-offered");
    let sig = model_sig(f);
    if let Some((was, o)) = ui.data(|d| d.get_temp::<(u64, Offered)>(id))
        && was == sig
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
            let ws_armor: Vec<(ResRef, mg_gff::Gff)> = body
                .list("Equip_ItemList")
                .into_iter()
                .flatten()
                .filter_map(|s| s.resref("EquippedRes"))
                .filter(|r| !r.is_empty())
                .filter_map(|r| {
                    let key = mg_resman::ResKey::new(r, ResType::UTI);
                    let doc = f.app.ws.as_mut()?.doc(&key).ok()?;
                    Some((r, doc.clone()))
                })
                .collect();
            let armor = |r: ResRef| -> Option<mg_gff::Gff> {
                if let Some((_, doc)) = ws_armor.iter().find(|(cr, _)| *cr == r) {
                    return Some(doc.clone());
                }
                let key = mg_resman::ResKey::new(r, ResType::UTI);
                let data = game.resman.get(&key).ok()?;
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
    ui.data_mut(|d| d.insert_temp(id, (sig, o.clone())));
    o
}

fn model_sig(f: &mut Form<'_>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    f.app.load_order.hash(&mut h);
    f.restype().0.hash(&mut h);
    match f.restype() {
        ResType::UTP | ResType::UTD => {
            f.root.integer("Appearance").hash(&mut h);
            f.root.integer("GenericType_New").hash(&mut h);
            f.root.integer("GenericType").hash(&mut h);
        }
        ResType::UTI => {
            format!("{:?}", mg_preview::ItemLook::from_uti(&f.root)).hash(&mut h);
        }
        ResType::UTC => {
            format!("{:?}", mg_preview::DressedLook::from_utc(&f.root, &|_| None)).hash(&mut h);
            let chest =
                f.root.list("Equip_ItemList").and_then(|l| l.iter().find(|s| s.id == CHEST_SLOT));
            if let Some(slot) = chest
                && slot.integer("BaseItem").is_none()
                && let Some(r) = slot.resref("EquippedRes").filter(|r| !r.is_empty())
            {
                r.hash(&mut h);
                let key = mg_resman::ResKey::new(r, ResType::UTI);
                if let Some(ws) = f.app.ws.as_mut()
                    && let Ok(doc) = ws.doc(&key)
                {
                    true.hash(&mut h);
                    format!("{:?}", mg_preview::ItemLook::from_uti(&doc.root)).hash(&mut h);
                } else {
                    false.hash(&mut h);
                }
            }
        }
        _ => {}
    }
    h.finish()
}

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
    // (In a narrow room, the Properties panel's, or a column of pages side
    // by side (M4's U10): each texture on a line, what draws it on the line
    // under.)
    let narrow = super::narrow(ui) || ui.available_width() < TEXTURE_ROW;
    mg_uikit::with_table_rows(ui, |ui| {
        egui::Grid::new(f.id(p.field).with(narrow))
            .num_columns(if narrow { 2 } else { 5 })
            .spacing([10.0, mg_uikit::metrics::TABLE_RULE])
            .show(ui, |ui| {
                if !rows.is_empty() && !narrow {
                    ui.label("");
                    crate::widgets::field_label(ui, p.titles.0);
                    ui.label("");
                    crate::widgets::field_label(ui, p.titles.1);
                    ui.end_row();
                }
                for row in &rows {
                    let plt = row.of_model.filter(|t| t.plt).map(|t| t.colors.unwrap_or_default());
                    small_picture(f, ui, &row.name, plt, ROW_PICTURE);
                    // (A name the replacement's field can't hold: said, not offered.)
                    let long = row.name.len() > ResRef::MAX_LEN;
                    super::row(ui, |ui| {
                        let name = ui.label(&row.name);
                        if let Some(t) = row.of_model {
                            name.on_hover_text(format!("On {}", t.models.join(", ")));
                        }
                        if row.of_model.is_none() {
                            ui.weak(format!("({NOT_IN_THE_MODEL})")).on_hover_text(
                            "Its models don't have this texture. The replacement is kept as it is.",
                        );
                        } else if plt.is_some() {
                            ui.weak("PLT").on_hover_text(PLT_WHY);
                        }
                        if let Some(t) = row.of_model {
                            match &t.source {
                                Source::Mesh => {}
                                Source::Material(m) => {
                                    ui.weak(format!("(material {m})")).on_hover_text(format!(
                                        "A texture of the material {m}.mtr a mesh is drawn with"
                                    ));
                                }
                                Source::Emitter => {
                                    ui.weak("(emitter)")
                                        .on_hover_text("A particle emitter's texture");
                                }
                                Source::Flare => {
                                    ui.weak("(flare)").on_hover_text("A light's lens flare");
                                }
                                // (Not offered: `named` leaves the TXIs' maps out.)
                                Source::Txi { .. } => {}
                            }
                        }
                        if long {
                            ui.weak("(name too long)").on_hover_text(TOO_LONG);
                        }
                    });
                    if narrow {
                        ui.end_row();
                    }
                    let new = row.entry.map(|i| resref_text(&items[i], p.new)).unwrap_or_default();
                    if new.is_empty() {
                        ui.allocate_exact_size(
                            egui::vec2(ROW_PICTURE, ROW_PICTURE),
                            egui::Sense::hover(),
                        );
                    } else {
                        small_picture(f, ui, &new, None, ROW_PICTURE);
                    }
                    super::row(ui, |ui| {
                        let id = f.id(&format!("{}#{}#{:?}#new", p.field, row.name, row.entry));
                        if long {
                            ui.add_enabled(false, egui::Button::new("Select Texture…"))
                                .on_disabled_hover_text(TOO_LONG);
                        } else {
                            let width = super::fit(ui, 140.0);
                            if let Some(v) = crate::widgets::commit_resref_in(
                                &mut f.app.buffers,
                                ui,
                                id,
                                &new,
                                width,
                            ) && let Ok(r) = ResRef::from_str(v.trim())
                            {
                                set_new = Some((row.entry, row.name.clone(), r));
                            }
                            f.hold_command(ui, id, false, |form, buf| {
                                if buf == new {
                                    return None;
                                }
                                let r = ResRef::from_str(buf.trim()).ok()?;
                                let all = p.replaced(items.clone(), row.entry, &row.name, r)?;
                                Some(form.nested_list_command(p.what, p.field, p.id, p.list, all))
                            });
                            let button = ui.button("Select Texture…");
                            if button.on_hover_text("Pick the texture to draw it with").clicked() {
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
            })
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
    super::row(ui, |ui| {
        ui.weak("Any name:");
        for (id, hint) in ids.into_iter().zip([p.titles.0, p.titles.1]) {
            let mut text = crate::widgets::ResRefText(f.app.buffers.entry(id).or_default());
            let width = super::fit(ui, 140.0);
            ui.add(crate::widgets::resref_edit(&mut text).desired_width(width).hint_text(hint));
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
    /// The textures to pick from.
    textures: Arc<Textures>,
    /// The interface's pictures are listed too (icons, portraits, load
    /// screens, maps and the GUI's own).
    interface: bool,
    /// Scrolled to the one chosen, once.
    placed: bool,
}

#[derive(Clone, Default)]
struct MadePictures {
    queue: std::collections::VecDeque<String>,
    set: std::collections::HashSet<String>,
}

impl MadePictures {
    fn insert(&mut self, key: String) {
        if self.set.insert(key.clone()) {
            self.queue.push_back(key);
        }
    }
}

/// How many of its pictures Select Texture keeps while open (64 KB each).
const PICTURES_KEPT: usize = 1500;

/// A picture's key in the app's pictures ([`crate::images::Loader::thumbnail`]).
fn thumb_key(name: &str, side: u32) -> String {
    format!("{}#thumb{side}", name.to_ascii_lowercase())
}

/// The textures Select Texture offers, by name (lower case), each with
/// whether it is one of the interface's pictures.
#[derive(Default)]
struct Textures {
    names: Vec<String>,
    interface: Vec<bool>,
}

impl Textures {
    fn of(mut names: Vec<String>, icons: &[String]) -> Textures {
        names.sort_unstable();
        names.dedup();
        let interface = names.iter().map(|n| is_interface(n, icons)).collect();
        Textures { names, interface }
    }

    /// How many there are, of the interface's or not.
    fn count(&self, interface: bool) -> usize {
        if interface { self.names.len() } else { self.interface.iter().filter(|i| !**i).count() }
    }
}

/// The beginnings of the names of the interface's pictures, which no
/// model is drawn with: the GUI's own (`gui_`, `ctl_`, `pnl_`, the
/// character maker's `cg_`, NUI's, the radial menu's `ir_`, fonts), the
/// icons of feats, spells, effects and misc items, portraits (`po_`),
/// load screens (`ls_`), the area map's (`mi_`) and the credits'.
const INTERFACE_PREFIXES: [&str; 18] = [
    "gui_",
    "ctl_",
    "ct_",
    "cg_",
    "pnl_",
    "fnt_",
    "nui_",
    "ir_",
    "ife_",
    "ief_",
    "is_",
    "iss_",
    "iit_",
    "po_",
    "ls_",
    "mi_",
    "xp1credits",
    "xp2credits",
];

/// Whether a texture is one of the interface's pictures: one whose name
/// begins as theirs do ([`INTERFACE_PREFIXES`]), a tile's picture on the
/// area map (`mi`, the tileset's prefix and `_`: `mitu01_`), or an item's
/// icon (`i`, its base item's `ItemClass` and `_`: `icons` holds those
/// beginnings).
fn is_interface(name: &str, icons: &[String]) -> bool {
    if INTERFACE_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return true;
    }
    let b = name.as_bytes();
    let map_tile = b.len() > 7
        && b.starts_with(b"mi")
        && b[2..4].iter().all(u8::is_ascii_alphabetic)
        && b[4..6].iter().all(u8::is_ascii_digit)
        && b[6] == b'_';
    map_tile || (name.starts_with('i') && icons.iter().any(|p| name.starts_with(p.as_str())))
}

/// The beginnings of item icons' names: `i`, each base item's `ItemClass`
/// (baseitems.2da), `_`.
fn icon_prefixes(game: &mg_rules::GameData) -> Vec<String> {
    let Ok(t) = game.table("baseitems") else { return Vec::new() };
    let mut out: Vec<String> = (0..t.len())
        .filter_map(|r| t.get(r, "ItemClass"))
        .filter(|c| !c.is_empty())
        .map(|c| format!("i{}_", c.to_ascii_lowercase()))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The textures a replacement can draw with: the game's, the haks' and the
/// module's DDS and TGA textures, each name once. The game's are listed
/// once for the load order (29,567 of them took 15 ms a click), the
/// module's each time.
fn texture_names(app: &crate::Moonglow, ui: &Ui) -> Arc<Textures> {
    let Some(game) = app.game.clone() else { return Arc::default() };
    let id = egui::Id::new("select-texture-names");
    let listed = super::kept_for_load_order(ui, app.load_order, id, || {
        // (The layers' names looked through once for both types, each
        // ResRef sorted and made a name once: listed type by type, a
        // world's 135,000 textures took 36 ms.)
        let names = sorted_names(
            (game.resman.layers().iter())
                .flat_map(|l| l.container.keys())
                .filter(|k| k.restype == ResType::DDS || k.restype == ResType::TGA)
                .map(|k| k.resref),
        );
        Arc::new(Textures::of(names, &icon_prefixes(&game)))
    });
    let own_sig: (usize, u64) = match &app.ws {
        Some(ws) => {
            use std::hash::{Hash, Hasher};
            let mut count = 0usize;
            let mut xor = 0u64;
            for t in [ResType::DDS, ResType::TGA] {
                for k in ws.module.keys_of(t) {
                    count += 1;
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    k.hash(&mut h);
                    xor ^= h.finish();
                }
            }
            (count, xor)
        }
        None => (0, 0),
    };
    if own_sig.0 == 0 {
        return listed;
    }
    super::kept_for_key(
        ui.ctx(),
        egui::Id::new("select-texture-names-merged"),
        &(app.load_order, own_sig),
        || {
            let Some(ws) = &app.ws else { return listed.clone() };
            let own: Vec<String> = ws
                .module
                .keys_of(ResType::DDS)
                .chain(ws.module.keys_of(ResType::TGA))
                .map(|k| k.resref.to_lowercase().to_string())
                .filter(|n| listed.names.binary_search(n).is_err())
                .collect();
            if own.is_empty() {
                return listed.clone();
            }
            let mut names = listed.names.clone();
            names.extend(own);
            Arc::new(Textures::of(names, &icon_prefixes(&game)))
        },
    )
}

/// The names of `refs`, lowercase, each once, in order of their bytes.
/// (Each as one number, its lowercase bytes padded with zeros, big end
/// first: sorted as the bytes are, shorter first. By ResRef's own order,
/// which folds both names at each comparison, Select Texture's sort took
/// 19 ms of a 27 ms opening.)
fn sorted_names(refs: impl Iterator<Item = mg_core::ResRef>) -> Vec<String> {
    let mut keys: Vec<u128> = refs
        .map(|r| {
            let mut bytes = [0; mg_core::ResRef::MAX_LEN];
            let name = r.as_bytes();
            bytes[..name.len()].copy_from_slice(name);
            bytes.make_ascii_lowercase();
            u128::from_be_bytes(bytes)
        })
        .collect();
    keys.sort_unstable();
    keys.dedup();
    (keys.iter())
        .map(|k| {
            let bytes = k.to_be_bytes();
            let len = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
            mg_core::ResRef::from_bytes(&bytes[..len]).unwrap_or_default().to_string()
        })
        .collect()
}

fn open_gallery(f: &mut Form<'_>, ui: &Ui, entry: Option<usize>, old: String, current: String) {
    let textures = texture_names(f.app, ui);
    let chosen = (!current.is_empty()).then(|| current.to_ascii_lowercase());
    // (The one it is drawn with now is shown, of the interface's or not.)
    let interface = chosen
        .as_ref()
        .is_some_and(|c| textures.names.binary_search(c).is_ok_and(|i| textures.interface[i]));
    let state = TexturePick { old, entry, chosen, textures, interface, ..Default::default() };
    ui.data_mut(|d| d.insert_temp(f.id("texture-gallery"), state));
}

/// Select Texture, while it is open: Find, the textures as pictures (made
/// as they come into sight, a few milliseconds' worth a frame), the one
/// chosen larger; OK gives (the model's texture, the one picked).
fn gallery(f: &mut Form<'_>, ui: &mut Ui) -> Option<(Option<usize>, String, ResRef)> {
    let id = f.id("texture-gallery");
    let mut state: TexturePick = ui.data(|d| d.get_temp(id))?;
    let needle = mg_core::text::Needle::new(&state.filter);
    let textures = state.textures.clone();
    // (The places of those shown, found once for the list, what is typed
    // and the switch: 60,000 names were searched every frame.)
    let shown_for = (state.filter.clone(), state.interface);
    let shown = super::kept_for(ui.ctx(), id.with("shown"), &textures, &shown_for, || {
        // (The names are lowercase, folded as they are when ASCII: folded
        // again for each, 135,000 took 7 ms a letter typed.)
        let finds = |n: &str| match n.is_ascii() {
            true => needle.is_empty() || needle.finds_folded(n),
            false => needle.finds(n),
        };
        let wanted = |i: u32| {
            let i = i as usize;
            (state.interface || !textures.interface[i]) && finds(&textures.names[i])
        };
        let len = textures.names.len();
        if needle.is_empty() {
            return Arc::new((0..len as u32).filter(|&i| wanted(i)).collect::<Vec<u32>>());
        }
        // (A key typed looks among what the filter before it found, not
        // among every name: 6.4 ms a letter at 135,000 textures.)
        let narrowing_id = id.with("narrowing");
        type Kept = (Arc<Textures>, bool, crate::narrowing::Narrowing);
        let kept = ui.data(|d| d.get_temp::<Kept>(narrowing_id));
        let mut narrowing = match kept {
            Some((of, interface, n))
                if Arc::ptr_eq(&of, &textures) && interface == state.interface =>
            {
                n
            }
            _ => Default::default(),
        };
        let shown = narrowing.find(needle.clone(), len, wanted);
        ui.data_mut(|d| {
            d.insert_temp::<Kept>(narrowing_id, (textures.clone(), state.interface, narrowing))
        });
        shown
    });
    let name_at = |i: u32| &textures.names[i as usize];
    registered(ui, id);
    let mut wanted = Vec::new();
    let mut new_made = Vec::new();
    let mut in_sight_indices = 0..0;
    // (No wider than the body's room: at the least window it ran past both
    // its edges.)
    let wide = (ui.ctx().content_rect().width() * 0.8)
        .clamp(480.0, 960.0)
        .min(crate::widgets::body_room_width(ui.ctx()));
    let tall = (ui.ctx().content_rect().height() * 0.6).clamp(300.0, 700.0);
    let side = crate::appearance_gallery::tile_side(f.app);
    let old = state.old.to_ascii_lowercase();
    let window =
        crate::widgets::modal(ui.ctx(), "Select Texture").id(id.with("modal")).close_as_cancel();
    // (OK as the grid gives it: a double click, Enter on a picture.)
    let mut ok_flag = false;
    let footed = window.show_foot(ui.ctx(), |ui| {
        let ok = &mut ok_flag;
        ui.set_width(wide);
        ui.label(format!("To draw {} with", state.old));
        // (Wrapped in the dialog's one width: a row of its own widened
        // the dialog at its least size past the well and the preview.)
        ui.horizontal_wrapped(|ui| {
            let field = id.with("filter");
            crate::widgets::SearchField::new(field, &mut state.filter)
                .hint("Find")
                .width(180.0)
                .find_over()
                .show(ui);
            crate::appearance_gallery::size_slider(f.app, ui);
            let all = textures.count(state.interface);
            ui.weak(format!("{} of {all} textures", shown.len())).on_hover_text(
                "The game's, the haks' and the module's DDS and TGA textures, not PLTs",
            );
            let interface = ui.check_box(&mut state.interface, "Interface textures too");
            if interface
                .on_hover_text(
                    "Also list icons, portraits, load screens and other interface pictures",
                )
                .changed()
            {
                state.placed = false;
                ui.ctx().request_repaint();
            }
        });
        let preview = 200.0;
        // (The well takes the dialog's width beside the preview: what
        // the dialog is, not what it was asked to be.)
        let room = (ui.available_width() - preview - 16.0).max(120.0);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            {
                // (Where the one chosen is among those shown, found once for
                // them and it: 60,000 names were walked every frame.)
                // (With none chosen, the texture replaced: the grid opens
                // at it, ringed.)
                let at_id = id.with("at");
                let at_name = state.chosen.clone().unwrap_or_else(|| old.clone());
                let current = super::kept_for(ui.ctx(), at_id, &shown, &at_name, || {
                    shown.iter().position(|&i| *name_at(i) == at_name)
                });
                // (Not while a menu or popup is open over it: the dialog is
                // itself what has the keys, so not `keys_blocked`.)
                let keys = !egui::Popup::is_any_open(ui.ctx());
                let grid =
                    mg_uikit::gallery::GalleryGrid::new(id.with("pictures"), shown.len(), side)
                        // (One line of name, cut: rows the board's 10
                        // by 14 apart, not a second line's room more.)
                        .label(mg_uikit::gallery::label_height(false, 1, false))
                        .cursor(current)
                        .keys_allowed(keys)
                        .name("Textures");
                let pad = egui::Margin { left: 10, right: 10, top: 8, bottom: 8 };
                let well = id.with("grid");
                let size = egui::vec2(room, tall);
                let shown_area = crate::widgets::picture_well(ui, well, size, pad, |ui| {
                    let out = grid.show(ui, |ui, i, rect, tile, r, cell| {
                        let name = name_at(shown[i]);
                        let made = picture_made(f, name);
                        let chosen = *name == at_name;
                        // (Names only, in the monospace of names: the
                        // board's shared cell.)
                        mg_uikit::gallery::GalleryCell::new(
                            name,
                            crate::widgets::cell_picture(made),
                        )
                        .model(true)
                        .kind(mg_uikit::Icon::Texture)
                        .selected(chosen)
                        .paint(ui, rect, tile, cell, r);
                        if r.clicked() {
                            state.chosen = Some(name.clone());
                        }
                        if r.double_clicked() {
                            state.chosen = Some(name.clone());
                            *ok = true;
                        }
                    });
                    // (The keys choose as they move: a picker's; Enter on the
                    // grid is OK.)
                    if let Some(i) = out.moved.or(out.activated) {
                        state.chosen = Some(name_at(shown[i]).clone());
                    }
                    if out.activated.is_some() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        *ok = true;
                    }
                    // (Those in sight and a row ahead: made, a few a frame.)
                    in_sight_indices = out.ahead.clone();
                    for i in out.ahead.clone() {
                        let name = name_at(shown[i]);
                        if picture_made(f, name).is_none() {
                            wanted.push(name.clone());
                        }
                    }
                    out
                });
                // (Opened at the one chosen, once, where the grid put it.)
                if !state.placed {
                    state.placed = match current {
                        Some(at) => crate::widgets::open_gallery_at(
                            ui.ctx(),
                            &shown_area,
                            &shown_area.inner,
                            at,
                        ),
                        None => true,
                    };
                }
            }
            ui.vertical(|ui| {
                ui.set_width(preview);
                // (The one chosen, else the texture it replaces, with
                // its name.)
                let shown = state.chosen.clone().or_else(|| (!old.is_empty()).then(|| old.clone()));
                if let Some(c) = shown {
                    let key = thumb_key(&c, PREVIEW_THUMB);
                    new_made.push(key);
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
        let any = state.chosen.is_some();
        ((), crate::widgets::picker_footer(None, any))
    });
    let pressed = footed.map(|f| f.pressed).unwrap_or_default();
    let ok = ok_flag || pressed.main();
    let cancel = pressed.cancelled();
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
            new_made.push(thumb_key(name, GALLERY_THUMB));
            left -= 1;
        }
    }
    if left > 0 {
        ui.ctx().request_repaint_after(Duration::from_millis(10));
    }
    let picked = state
        .chosen
        .as_deref()
        .filter(|_| ok)
        .and_then(|c| ResRef::from_str(c).ok())
        .map(|r| (state.entry, state.old.clone(), r));
    let made_id = id.with("made");
    if ok || cancel {
        if let Some(made) = ui.data_mut(|d| d.remove_temp::<MadePictures>(made_id)) {
            for k in &made.queue {
                f.app.pictures.remove(k);
            }
        }
        for k in new_made {
            f.app.pictures.remove(&k);
        }
        ui.data_mut(|d| d.remove::<TexturePick>(id));
    } else {
        if !new_made.is_empty() {
            ui.data_mut(|d| {
                let made = d.get_temp_mut_or_default::<MadePictures>(made_id);
                for k in new_made {
                    made.insert(k);
                }
                if made.queue.len() > PICTURES_KEPT {
                    let in_sight: std::collections::HashSet<String> = in_sight_indices
                        .clone()
                        .map(|i| thumb_key(name_at(shown[i]), GALLERY_THUMB))
                        .collect();
                    let mut over = made.queue.len() - PICTURES_KEPT;
                    let mut kept = std::collections::VecDeque::with_capacity(made.queue.len());
                    while let Some(k) = made.queue.pop_front() {
                        if over > 0 && !in_sight.contains(&k) {
                            f.app.pictures.remove(&k);
                            made.set.remove(&k);
                            over -= 1;
                        } else {
                            kept.push_back(k);
                        }
                    }
                    made.queue = kept;
                }
            });
        }
        ui.data_mut(|d| d.insert_temp(id, state));
    }
    picked
}

/// The Select Texture windows open, each with the pass it was last drawn
/// in ([`registered`], [`let_go_of_unseen`]).
type Open = Vec<(egui::Id, u64)>;

fn open_id() -> egui::Id {
    egui::Id::new("select-texture-open")
}

/// Notes that the Select Texture window `id` is drawn in this pass.
fn registered(ui: &Ui, id: egui::Id) {
    let pass = ui.ctx().cumulative_pass_nr();
    ui.data_mut(|d| {
        let open = d.get_temp_mut_or_default::<Open>(open_id());
        match open.iter_mut().find(|(i, _)| *i == id) {
            Some(entry) => entry.1 = pass,
            None => open.push((id, pass)),
        }
    });
}

/// After a frame: a Select Texture window that was not drawn (its page
/// left, its Properties window closed) is closed, and the pictures it
/// made let go of. They were let go of by its OK and Cancel alone: left
/// behind, it kept up to [`PICTURES_KEPT`] of them for the session.
pub(crate) fn let_go_of_unseen(app: &mut crate::Moonglow, ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    let gone: Vec<egui::Id> = ctx.data_mut(|d| {
        let Some(open) = d.get_temp::<Open>(open_id()).filter(|o| !o.is_empty()) else {
            return Vec::new();
        };
        let (seen, unseen): (Open, Open) = open.into_iter().partition(|(_, at)| *at >= pass);
        d.insert_temp(open_id(), seen);
        unseen.into_iter().map(|(id, _)| id).collect()
    });
    for id in gone {
        ctx.data_mut(|d| d.remove_temp::<TexturePick>(id));
        if let Some(made) = ctx.data_mut(|d| d.remove_temp::<MadePictures>(id.with("made"))) {
            for k in &made.queue {
                app.pictures.remove(k);
            }
        }
    }
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
    // (In a narrow room: each of a pair on a line of its own.)
    let narrow = super::narrow(ui);
    mg_uikit::with_table_rows(ui, |ui| {
        egui::Grid::new(f.id(p.field).with(narrow))
            .num_columns(if narrow { 1 } else { 3 })
            .spacing([12.0, mg_uikit::metrics::TABLE_RULE])
            .show(ui, |ui| {
                if !narrow {
                    crate::widgets::field_label(ui, p.titles.0);
                    crate::widgets::field_label(ui, p.titles.1);
                    ui.end_row();
                }
                for (i, e) in items.iter().enumerate() {
                    for label in [p.old, p.new] {
                        if narrow && label == p.new {
                            ui.end_row();
                        }
                        let now = resref_text(e, label);
                        let id = f.id(&format!("{}#{i}#{label}", p.field));
                        let list =
                            crate::widgets::row_choice(id, &choices, row_of(&now)).width(180.0);
                        let list = if now.is_empty() { list.unset("(the original)") } else { list };
                        if let Some(r) = list.show(ui)
                            && let Some(r) =
                                names.get(r as usize).and_then(|n| ResRef::from_str(n).ok())
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
                    if narrow && k == 1 {
                        ui.end_row();
                    }
                    let id = f.id(&format!("{}#pick#{k}", p.field));
                    let unset =
                        if k == 0 { "Choose an animation" } else { "Choose the one to play" };
                    let list =
                        crate::widgets::row_choice(id, &choices, *slot).width(180.0).unset(unset);
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
            })
    });
    ui.data_mut(|d| d.insert_temp(pending_id, pending));
    if let Some(all) = changed {
        p.set(f, all);
    }
    add_row(f, ui, &p, &items);
}

/// The width a texture's row takes on one line: its pictures, its name,
/// the field, Select Texture… and Reset.
const TEXTURE_ROW: f32 = 560.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// The textures' names, sorted as numbers, are in the order ResRef's
    /// own sort gives the lowercase names, each once, the shorter of two
    /// alike first, a full 16 letters too.
    #[test]
    fn texture_names_sorted_as_numbers_are_in_the_names_order() {
        let r = |s: &str| ResRef::from_str(s).unwrap();
        let given = ["tno01_wall", "TNO01_Wall", "a", "ab", "a_b", "zzzzzzzzzzzzzzzz", "Ab", "a1"];
        let mut expected: Vec<ResRef> = given.iter().map(|s| r(s).to_lowercase()).collect();
        expected.sort();
        expected.dedup();
        let expected: Vec<String> = expected.iter().map(ToString::to_string).collect();
        assert_eq!(sorted_names(given.iter().map(|s| r(s))), expected);
    }

    /// A pick is given to the entry of the texture it was made for, not to
    /// whatever is at that entry's place by OK (the review's B11: with A
    /// reset behind Select Texture, B's pick drew C).
    #[test]
    fn a_pick_goes_to_its_texture_s_entry_wherever_it_is_by_then() {
        let p = Pairs::TEXTURES;
        let r = |s: &str| ResRef::from_str(s).unwrap();
        let (a, b, c) =
            (p.entry(r("a"), r("a2")), p.entry(r("b"), r("b2")), p.entry(r("c"), r("c2")));
        let news = |items: &[Struct]| -> Vec<String> {
            items.iter().map(|s| resref_text(s, p.new)).collect()
        };
        // As it was: B is the second.
        let all = vec![a.clone(), b.clone(), c.clone()];
        let set = p.replaced(all.clone(), Some(1), "B", r("pick")).unwrap();
        assert_eq!(news(&set), ["a2", "pick", "c2"]);
        assert_eq!(p.replaced(all, Some(1), "b", r("b2")), None, "the same: nothing to set");
        // A reset behind the window: B is the first now.
        let set = p.replaced(vec![b.clone(), c.clone()], Some(1), "b", r("pick")).unwrap();
        assert_eq!(news(&set), ["pick", "c2"]);
        // B itself reset: a new entry, C as it was.
        let set = p.replaced(vec![a.clone(), c.clone()], Some(1), "b", r("pick")).unwrap();
        assert_eq!(news(&set), ["a2", "c2", "pick"]);
        assert_eq!(resref_text(&set[2], p.old), "b");
        // No entry yet: one is made, unless nothing is picked.
        assert_eq!(
            news(&p.replaced(vec![a.clone()], None, "b", r("pick")).unwrap()),
            ["a2", "pick"]
        );
        assert_eq!(p.replaced(vec![a], None, "b", ResRef::EMPTY), None);
    }

    #[test]
    fn the_interface_s_pictures_are_told_by_their_names() {
        let icons = vec!["iwswls_".to_string(), "iit_bag_".to_string()];
        for name in [
            "gui_mp_arwalk",
            "ctl_cg_btn_feats",
            "pnl_quick_bar",
            "ife_x2epicprot",
            "is_fireball",
            "po_hezrou_h",
            "ls_tcn_01",
            "mi_tcm02_a01_01",
            "mitu01_a03",
            "iwswls_b_011",
        ] {
            assert!(is_interface(name, &icons), "{name}");
        }
        // Models' textures: a weapon's part (no `i`), a tile's, a chest's,
        // a name that only begins with `mi`.
        for name in ["wswls_b_011", "tcm02_wall01", "plc_chest1", "mirror01", "mitd_wall"] {
            assert!(!is_interface(name, &icons), "{name}");
        }
        let t = Textures::of(
            ["plc_chest1", "po_hezrou_h", "plc_chest1", "ttr01_bark01"].map(String::from).into(),
            &icons,
        );
        assert_eq!(t.names, ["plc_chest1", "po_hezrou_h", "ttr01_bark01"]);
        assert_eq!((t.count(false), t.count(true)), (2, 3));
    }
}
