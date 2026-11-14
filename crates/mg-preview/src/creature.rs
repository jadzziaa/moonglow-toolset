//! Creatures: part-based (appearance MODELTYPE P) bodies built from a
//! skeleton, body or armour parts, a robe, a head or helmet and a cloak;
//! single-model ones (S, F, L); wings and tails; weapons and shields.

use mg_core::ResRef;
use mg_gff::{Gff, Struct, Value};
use mg_rules::GameData;
use mg_rules::models::{self, BODY_PARTS};

use crate::{
    HAIR, Lookup, Part, Preview, PreviewError, SKIN, TATTOO1, TATTOO2, cell, cell_f32, cell_int,
    env_map, item_colors,
};

/// The robe, among the armor's parts.
const ROBE: usize = 18;

/// The visual-transform scopes of a creature's parts
/// (`OBJECT_VISUAL_TRANSFORM_DATA_SCOPE_CREATURE_*`).
const HEAD_SCOPE: i64 = 254;
const TAIL_SCOPE: i64 = 253;
const WINGS_SCOPE: i64 = 252;
const CLOAK_SCOPE: i64 = 243;

/// `colors` (by PLT layer) with armor part `part`'s own colors over them.
fn part_colors(armor: &Struct, part: usize, mut colors: [u8; 10]) -> [u8; 10] {
    for channel in mg_rules::items::ArmorChannel::ALL {
        if let Some(c) = mg_rules::items::armor_part_color(armor, part, channel) {
            colors[channel.plt_layer()] = c;
        }
    }
    colors
}

/// Equipment slots (`Equip_ItemList` struct ids).
const HEAD: u32 = 0x1;
const CHEST: u32 = 0x2;
const RIGHT_HAND: u32 = 0x10;
const LEFT_HAND: u32 = 0x20;
const CLOAK: u32 = 0x40;

/// The item in an equipment slot: a blueprint's `EquippedRes`, loaded with
/// `item`, or the item struct itself (area instances and characters).
fn equipped(utc: &Struct, slot: u32, item: &dyn Fn(ResRef) -> Option<Gff>) -> Option<Struct> {
    let s = utc.list("Equip_ItemList")?.iter().find(|s| s.id == slot)?;
    match s.resref("EquippedRes") {
        Some(r) if !r.is_empty() => item(r).map(|g| g.root),
        _ => Some(s.clone()),
    }
}

/// A part number field (its EE twin where there is one).
fn number(s: &Struct, label: &str) -> i64 {
    mg_rules::items::part_number(s, label).unwrap_or(0)
}

/// A creature's looks without a blueprint: an appearance row and the body
/// a blueprint would choose, wearing nothing (for browsing appearances).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CreatureLook {
    /// appearance.2da row.
    pub appearance: u16,
    /// gender.2da row (0 male, 1 female).
    pub gender: u8,
    /// phenotype.2da row.
    pub phenotype: u8,
    /// The head's model number (part-based creatures).
    pub head: u8,
    /// The body parts' model number: every part but the belt and the
    /// shoulders, which a bare body lacks.
    pub body: u8,
    /// PLT colours: skin, hair, tattoo 1 and 2.
    pub colors: [u8; 4],
    /// wingmodel.2da and tailmodel.2da rows (0: none).
    pub wings: u32,
    pub tail: u32,
}

impl CreatureLook {
    /// An appearance with the first head and body parts, colour 0.
    pub fn new(appearance: u16) -> CreatureLook {
        CreatureLook {
            appearance,
            gender: 0,
            phenotype: 0,
            head: 1,
            body: 1,
            colors: [0; 4],
            wings: 0,
            tail: 0,
        }
    }

    /// The blueprint fields [`creature`] reads for this look.
    pub fn to_utc(&self) -> Struct {
        let mut utc = Struct::new(0);
        utc.set("Appearance_Type", Value::Word(self.appearance));
        utc.set("Gender", Value::Byte(self.gender));
        utc.set("Phenotype", Value::Int(i32::from(self.phenotype)));
        utc.set("Appearance_Head", Value::Byte(self.head));
        for (mdlname, field, _) in BODY_PARTS {
            let bare = matches!(mdlname, "belt" | "shor" | "shol");
            utc.set(field, Value::Byte(if bare { 0 } else { self.body }));
        }
        for (field, v) in ["Color_Skin", "Color_Hair", "Color_Tattoo1", "Color_Tattoo2"]
            .into_iter()
            .zip(self.colors)
        {
            utc.set(field, Value::Byte(v));
        }
        utc.set("Wings_New", Value::Dword(self.wings));
        utc.set("Tail_New", Value::Dword(self.tail));
        utc
    }
}

/// A creature by its looks alone ([`CreatureLook`]).
pub fn creature_look(game: &GameData, look: &CreatureLook) -> Result<Preview, PreviewError> {
    creature(game, &look.to_utc(), &|_| None)
}

/// A creature blueprint (UTC fields; equipped items through `item`),
/// without what the game takes off it as it loads it (an item it may not
/// wear: [`GameData::wear_problem`]).
pub fn creature(
    game: &GameData,
    utc: &Struct,
    item: &dyn Fn(ResRef) -> Option<Gff>,
) -> Result<Preview, PreviewError> {
    dressed(game, utc, item, true, None)
}

/// [`creature`], what it may not wear left off where `as_the_game` (an
/// armor's preview is a body that wears it whatever); `remembered`: what
/// a walk over many creatures has read of their parts' models already.
pub(crate) fn dressed(
    game: &GameData,
    utc: &Struct,
    item: &dyn Fn(ResRef) -> Option<Gff>,
    as_the_game: bool,
    remembered: Option<&crate::Remembered>,
) -> Result<Preview, PreviewError> {
    let worn = |slot| {
        equipped(utc, slot, item).filter(|it| !as_the_game || game.wear_problem(utc, it).is_none())
    };
    let lk = Lookup { game, remembered };
    let appearance = game.table("appearance")?;
    let row = number(utc, "Appearance_Type");
    // (A model of its own, where the row names one, that is there.)
    crate::appearance(game, mg_rules::ObjectKind::Creature, utc)?;
    let r = usize::try_from(row).map_err(|_| PreviewError::NoRow { table: "appearance", row })?;
    let race =
        cell(&appearance, r, "RACE").ok_or(PreviewError::NoRow { table: "appearance", row })?;
    let model_type = cell(&appearance, r, "MODELTYPE").unwrap_or("S").to_ascii_uppercase();
    let env = env_map(cell(&appearance, r, "ENVMAP"));

    // Colours: skin, hair and tattoos from the creature, the rest from its
    // armour.
    let mut colors = [0u8; 10];
    for (layer, field) in [
        (SKIN, "Color_Skin"),
        (HAIR, "Color_Hair"),
        (TATTOO1, "Color_Tattoo1"),
        (TATTOO2, "Color_Tattoo2"),
    ] {
        colors[layer] = number(utc, field).clamp(0, 255) as u8;
    }
    let armor = worn(CHEST);
    let body_colors = armor.as_ref().map_or(colors, |a| item_colors(a, colors));

    let mut preview = Preview {
        base: Part::new(race),
        parts: Vec::new(),
        idle: Some(if model_type.starts_with('P') || model_type.starts_with('F') {
            "pause1".into()
        } else {
            "cpause1".into()
        }),
        lights: Vec::new(),
    };
    preview.base.env_map = env.clone();
    preview.base.colors = Some(colors);
    let mut hide_shoulders = [false, false];
    let (mut hide_wings, mut hide_tail) = (false, false);

    if model_type.starts_with('P') {
        let genders = game.table("gender")?;
        let phenotypes = game.table("phenotype")?;
        // Its phenotype's skeleton, then the phenotype's default's: by the
        // one rule Verify and the blueprint's lists go by.
        let [skeleton, fallback] =
            models::skeletons(utc, &appearance, Some(&genders), Some(&phenotypes))
                .ok_or(PreviewError::NoRow { table: "appearance", row })?;
        let female = skeleton.chars().nth(1) == Some('f');
        let skel = if lk.has_model(&skeleton) { skeleton.clone() } else { fallback.clone() };
        preview.base = Part::new(&skel);
        let part_model = |name: &str, n: i64| -> Option<String> {
            [&skeleton, &fallback]
                .into_iter()
                .map(|s| match name {
                    "cloak" => models::body_cloak_model(s, n),
                    _ => models::body_part_model(s, name, n),
                })
                .find(|m| lk.has_model(m))
        };
        // `part`: the armor part (an index of `ARMOR_PARTS`), which may
        // have colors of its own.
        let with = |model: String, attach: Option<&str>, animated: bool, part: Option<usize>| {
            let mut p = Part::new(model);
            p.attach = attach.map(str::to_ascii_lowercase);
            p.colors = Some(match (&armor, part) {
                (Some(a), Some(part)) => part_colors(a, part, body_colors),
                _ => body_colors,
            });
            p.env_map = env.clone();
            p.animated = animated;
            p.textures = plt_fallbacks(&lk, &p.model);
            p
        };

        // A robe hides the parts its parts_robe row says, but the head: the
        // client draws the head whatever `HIDEHEAD` says (a robe row of the
        // game's set to 1, `creatures_look` with `MG_PARTS=Robe=3`; issue
        // #42, a Vault hak's robes that hid the head in Moonglow alone).
        let robe = armor.as_ref().map_or(0, |a| number(a, "ArmorPart_Robe"));
        let robes = game.table("parts_robe")?;
        let hidden = |mdlname: &str| {
            robe > 0
                && cell(&robes, robe as usize, &format!("HIDE{}", mdlname.to_ascii_uppercase()))
                    == Some("1")
        };
        if robe > 0
            && let Some(m) = part_model("robe", robe)
        {
            preview.parts.push(with(m, None, true, Some(ROBE)));
        }

        let capart = game.table("capart")?;
        let cloak = worn(CLOAK);
        if let Some(c) = &cloak {
            let cloaks = game.table("cloakmodel")?;
            let row = number(c, "ModelPart1").max(0) as usize;
            let flag = |col| cell(&cloaks, row, col) == Some("1");
            hide_shoulders = [flag("HIDESHOL"), flag("HIDESHOR")];
            hide_wings = flag("HIDEWING");
            hide_tail = flag("HIDETAIL");
            if let Some(model) =
                cell_int(&cloaks, row, "MODEL").and_then(|m| part_model("cloak", m))
            {
                let mut p = with(model, None, true, None);
                p.local = crate::scope_matrix(utc, CLOAK_SCOPE);
                let texture =
                    format!("cloak_{:03}", cell_int(&cloaks, row, "TEXTURE").unwrap_or(1));
                p.textures =
                    lk.bitmaps(&p.model).into_iter().map(|b| (b, texture.clone())).collect();
                p.colors = Some(item_colors(c, colors));
                p.env_map = env_map(cell(&cloaks, row, "ENVMAP"));
                p.item = true;
                preview.parts.push(p);
            }
        }

        for row in 0..capart.len() {
            let Some(mdlname) = cell(&capart, row, "MDLNAME").map(str::to_ascii_lowercase) else {
                continue;
            };
            let Some(part) = BODY_PARTS.iter().position(|(n, _, _)| *n == mdlname) else {
                continue;
            };
            let (_, body_field, armor_field) = BODY_PARTS[part];
            if hidden(&mdlname)
                || (mdlname == "shol" && hide_shoulders[0])
                || (mdlname == "shor" && hide_shoulders[1])
            {
                continue;
            }
            let worn = armor.as_ref().map_or(0, |a| number(a, armor_field));
            // The armor's part covers the creature's own unless it is
            // bare skin (part 1) or nothing: there the creature's shows (a
            // pale master's arm; a jackalwere's legs and paws under armor
            // whose parts are all 1), and where the creature has none (no
            // field), none shows: a commoner made a human, his clothes'
            // arms 1, has no arms in the client (`reading_part_scopes`).
            let own = number(utc, body_field);
            let n = if worn > 1 { worn } else { own };
            if n <= 0 {
                continue;
            }
            let node = cell(&capart, row, "NODENAME").unwrap_or("rootdummy");
            if let Some(m) = part_model(&mdlname, n) {
                preview.parts.push(with(m, Some(node), false, Some(part)));
            }
        }

        // A helmet takes the head's place.
        let helmet = worn(HEAD);
        match &helmet {
            Some(h) => {
                // (A helmet whose model isn't there is drawn as its base
                // item's `DefaultModel`, the bag, in the head's place, as
                // the client draws it: `reading_missing_helmet_model`.)
                let model = Some(models::helmet_model(number(h, "ModelPart1")))
                    .filter(|m| lk.has_model(m))
                    .or_else(|| {
                        let baseitems = game.table("baseitems").ok()?;
                        let row = usize::try_from(number(h, "BaseItem")).ok()?;
                        cell(&baseitems, row, "DefaultModel")
                            .map(str::to_ascii_lowercase)
                            .filter(|m| lk.has_model(m))
                    });
                if let Some(model) = model {
                    let mut p = Part::new(model);
                    p.attach = Some("head_g".into());
                    let scale_col = if female { "HELMET_SCALE_F" } else { "HELMET_SCALE_M" };
                    p.scale = cell_f32(&appearance, r, scale_col).unwrap_or(1.0);
                    p.colors = Some(item_colors(h, colors));
                    p.env_map = Some("chrome1".into());
                    p.textures = helmet_textures(&lk, &p.model);
                    // (In the head's place, as the head's.)
                    p.local = crate::scope_matrix(utc, HEAD_SCOPE);
                    p.item = true;
                    preview.parts.push(p);
                }
            }
            // Head 0, or no field, is no head: the client draws the neck
            // alone (`client_render.rs`'s `heads_look`).
            None => {
                let head = number(utc, "Appearance_Head");
                if head > 0
                    && let Some(m) = part_model("head", head)
                {
                    let mut p = with(m, Some("head_g"), false, None);
                    p.local = crate::scope_matrix(utc, HEAD_SCOPE);
                    preview.parts.push(p);
                }
            }
        }
    }

    // Wings and tails at their dummies.
    let scale = cell_f32(&appearance, r, "WING_TAIL_SCALE").unwrap_or(1.0);
    for (table, fields, node, hide, scope) in [
        ("wingmodel", ["Wings_New", "Wings"], "wings", hide_wings, WINGS_SCOPE),
        ("tailmodel", ["Tail_New", "Tail"], "tail", hide_tail, TAIL_SCOPE),
    ] {
        let n = fields.iter().find_map(|f| utc.integer(f)).unwrap_or(0);
        if n <= 0 || hide {
            continue;
        }
        let t = game.table(table)?;
        if let Some(model) = cell(&t, n as usize, "MODEL").filter(|m| lk.has_model(m)) {
            let mut p = Part::new(model);
            p.attach = Some(node.into());
            p.scale = scale;
            p.colors = Some(colors);
            p.env_map = env_map(cell(&t, n as usize, "ENVMAP"));
            p.animated = true;
            p.local = crate::scope_matrix(utc, scope);
            preview.parts.push(p);
        }
    }

    // Weapons and shields, except on simple (S) creatures.
    if !model_type.starts_with('S') {
        let weapon_scale = cell_f32(&appearance, r, "WEAPONSCALE").unwrap_or(1.0);
        let baseitems = game.table("baseitems")?;
        for slot in [RIGHT_HAND, LEFT_HAND] {
            let Some(it) = worn(slot) else { continue };
            let shield =
                cell_int(&baseitems, number(&it, "BaseItem").max(0) as usize, "WeaponWield")
                    == Some(7);
            let node = match slot {
                RIGHT_HAND => "rhand",
                _ if shield => "lforearm",
                _ => "lhand",
            };
            // (One that names no model, nor a default one, is left off, as
            // a helmet or a cloak whose model is nowhere is: the creature
            // is drawn without it.)
            let Ok(parts) = crate::item::item_parts(game, &it, colors, false) else { continue };
            for mut p in parts {
                p.attach = Some(node.into());
                p.scale = weapon_scale;
                p.item = true;
                preview.parts.push(p);
            }
        }
    }
    Ok(preview)
}

/// The textures a part model is drawn with in place of those its meshes
/// name. Where the part has a PLT of its own name
/// (`p<g><r><pheno>_<part>NNN`, or the first of its fallbacks: phenotype
/// 0 → human of the same gender → human male; a head's stop at its
/// race's phenotype 0), the game draws every mesh with that name, whatever
/// each names; otherwise each keeps what it names, and one that is missing
/// takes the first texture of the part's own names, then of its own.
pub(crate) fn plt_fallbacks(
    lk: &Lookup<'_>,
    model: &str,
) -> std::collections::BTreeMap<String, String> {
    let model = model.to_ascii_lowercase();
    let Some(kept) = lk.remembered else { return read_plt_fallbacks(lk, &model) };
    if let Some(found) = kept.fallbacks.borrow().get(&model) {
        return found.clone();
    }
    let found = read_plt_fallbacks(lk, &model);
    kept.fallbacks.borrow_mut().insert(model, found.clone());
    found
}

/// The textures a helmet's model (a layered item's: `helm_033`) is drawn
/// with in place of those its meshes name: where it has a PLT of its own
/// name, the game draws every mesh with it, whatever each names, as a
/// body part's ([`plt_fallbacks`]); otherwise each keeps what it names.
/// The game's hood, `helm_033`, names `helm`, a texture that is nowhere,
/// and was drawn untextured (GitHub issue 45); nine of the game's helmets
/// name another helmet's PLT (`helm_006` names `helm_003`'s). Seen in the
/// client (2026-11-13): a helmet's own PLT on a mesh naming a texture
/// that is nowhere, another helmet's PLT, a TGA of no part's name or no
/// texture (`bitmap NULL`); a mesh naming a material with a `texture0`
/// keeps it, and a TGA of the helmet's name alone replaces nothing
/// (`part_textures_look`'s rows W, X, Y, Z, 0, 1, 2 and the game's hood,
/// 3, in mg-corpus-tests' `client_render.rs`).
pub(crate) fn helmet_textures(
    lk: &Lookup<'_>,
    model: &str,
) -> std::collections::BTreeMap<String, String> {
    let model = model.to_ascii_lowercase();
    if !lk.has_plt(&model) {
        return std::collections::BTreeMap::new();
    }
    let read = || every_mesh_in(lk, lk.model(&model).as_deref(), &model);
    let Some(kept) = lk.remembered else { return read() };
    if let Some(found) = kept.helmets.borrow().get(&model) {
        return found.clone();
    }
    let found = read();
    kept.helmets.borrow_mut().insert(model, found.clone());
    found
}

/// `own` on every mesh of `model` (a part or a helmet with a PLT of that
/// name), under each texture a mesh names and, for a mesh that names
/// none, the empty name (`mg_render::Instance::textures`); but a mesh
/// whose `bitmap` names a material (MTR) with a `texture0` keeps the
/// material, as one whose `materialname` does (the renderer draws that
/// over the bitmap).
fn every_mesh_in(
    lk: &Lookup<'_>,
    model: Option<&mg_mdl::Model>,
    own: &str,
) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let bitmaps = model.map_or_else(Vec::new, crate::bitmaps);
    for b in bitmaps.into_iter().filter(|b| b != own && !lk.has_material_texture(b)) {
        out.insert(b, own.to_string());
    }
    if model.is_some_and(crate::unnamed_mesh) {
        out.insert(String::new(), own.to_string());
    }
    out
}

/// [`plt_fallbacks`] of `model` (lower case), read from the load order.
fn read_plt_fallbacks(lk: &Lookup<'_>, model: &str) -> std::collections::BTreeMap<String, String> {
    // A part's name, and its fallbacks: a head's no other race's or
    // gender's (an elf's or a woman's head drawn in the human man's PLT of
    // its name keeps what its mesh names, and one whose mesh names a
    // texture that is missing is drawn without).
    let named = |name: &str| -> Vec<String> {
        let Some(part) = models::part_name(name) else { return Vec::new() };
        let (g, r, rest) = (part.gender, part.race, part.part);
        let mut names = vec![name.to_string(), format!("p{g}{r}0_{rest}")];
        if !rest.to_ascii_lowercase().starts_with("head") {
            names.extend([format!("p{g}h0_{rest}"), format!("pmh0_{rest}")]);
        }
        names
    };
    let mut out = std::collections::BTreeMap::new();
    // (The model read once, for its textures and for a mesh without one.)
    let parsed = lk.model(model);
    let bitmaps = parsed.as_deref().map_or_else(Vec::new, crate::bitmaps);
    // The part's own PLT, where it has one, is what the game draws on every
    // mesh of it, whatever the mesh names: another part's texture (the
    // game's own thigh 3 names thigh 2's, chest 32 chest 13's), a texture
    // of no part's name, a PLT or a TGA, one mesh of two (a head's hair,
    // a gem). A race's part without a PLT of its name takes the human's
    // (the elf's chest 3 names the human's chest 4 and is drawn in the
    // human's chest 3). A texture of the part's own name that is only a
    // DDS or TGA replaces nothing. Which of a DDS or TGA and a PLT of the
    // name is drawn is the renderer's (`mg_render::Assets::texture_kept`):
    // the DDS or true-color TGA. Seen in the client: `part_textures_look`
    // in mg-corpus-tests' `client_render.rs`. A mesh whose `bitmap` names a
    // material (MTR) with a `texture0` keeps the material, as one whose
    // `materialname` does (the renderer draws that over the bitmap).
    if let Some(own) = named(model).into_iter().find(|c| lk.has_plt(c)) {
        return every_mesh_in(lk, parsed.as_deref(), &own);
    }
    for b in bitmaps {
        if lk.has_texture(&b) {
            continue;
        }
        let candidates = named(model).into_iter().chain(named(&b));
        if let Some(c) = candidates.into_iter().find(|c| *c != b && lk.has_texture(c)) {
            out.insert(b, c);
        }
    }
    // A mesh that names no texture takes the part's own too (several of
    // CEP's heads: `bitmap NULL`, with a PLT of the head's name). Under the
    // empty name (`mg_render::Instance::textures`).
    if parsed.as_deref().is_some_and(crate::unnamed_mesh)
        && let Some(c) = named(model).into_iter().find(|c| lk.has_texture(c))
    {
        out.insert(String::new(), c);
    }
    out
}
