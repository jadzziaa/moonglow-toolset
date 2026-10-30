//! Add to Palette (the area viewer's context menu): a placed object made
//! into a new custom blueprint, as Aurora does it (captured:
//! `aurora_palette_add.rs`).
//!
//! - The new resref is the object's blueprint's without a leading `nw_`,
//!   numbered as [`new_resref`] says (`nw_wswls001` → `wswls003`).
//! - The blueprint is the object's fields less where it stands (position,
//!   orientation, visual transforms, outlines, spawn points), with the
//!   palette category and comment of the blueprint it was placed from, and
//!   its portrait's row when that blueprint named a portrait resref.
//! - Each item it holds (inventory, equipment, store pages) becomes a new
//!   item blueprint the same way; the new blueprint holds them by resref.
//! - The placed object, and the items it holds, then name their new
//!   blueprints.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;
use mg_rules::{GameData, ObjectKind};

use crate::project_file::Naming;

/// What Add to Palette makes.
#[derive(Debug, Clone, PartialEq)]
pub struct Added {
    /// The new blueprints: the object's first, then the items it holds.
    pub blueprints: Vec<(ResKey, Gff)>,
    /// The placed object, now naming its new blueprint (and its items
    /// theirs).
    pub instance: Struct,
}

/// A new blueprint resref for a copy of `template`, as Aurora names it:
/// the template without a leading `nw_` and its trailing number (within 13
/// characters); from that number (0 without one), the first not in use
/// (with or without the `nw_`), plus one (`nw_wswls001`, with no
/// `nw_wswls002`: `wswls003`; `mgp_utp`: `mgp_utp001`), or the next free
/// after it.
pub fn new_resref(template: &str, taken: &dyn Fn(&str) -> bool) -> Option<ResRef> {
    new_resref_named(template, &Naming::default(), taken)
}

/// [`new_resref`] by a project's naming: the builder's code in the team's
/// pattern around the stem (`ab_wswls003`), the stem cut to fit.
pub fn new_resref_named(
    template: &str,
    naming: &Naming,
    taken: &dyn Fn(&str) -> bool,
) -> Option<ResRef> {
    let lower = template.to_ascii_lowercase();
    let (prefix, base) = match lower.strip_prefix("nw_") {
        Some(rest) => ("nw_", rest),
        None => ("", lower.as_str()),
    };
    let base = naming.strip(base);
    let stem = base.trim_end_matches(|c: char| c.is_ascii_digit());
    let mut n: u32 = base[stem.len()..].parse().unwrap_or(0);
    let stem = &stem[..stem.len().min(13)];
    let stem = if stem.is_empty() { "blueprint" } else { stem };
    let named = |n: u32| naming.fit(stem, &format!("{n:03}"));
    let used = |n: u32| {
        taken(&format!("{prefix}{stem}{n:03}"))
            || taken(&format!("{stem}{n:03}"))
            || taken(&named(n))
    };
    while n < 999 && used(n) {
        n += 1;
    }
    naming.numbered(stem, n + 1, taken)
}

/// A resref for a blueprint's Edit Copy, as Aurora's offers one (captured
/// 2026-10-29: `nw_w1copy007` in the module gave `w1copy008`): the
/// template without a leading `nw_` and its trailing number (within 13
/// characters), numbered from the template's number plus one (1 without
/// one), the first free. (Add to Palette goes past the numbers in use
/// first: [`new_resref`].)
pub fn copy_resref_named(
    template: &str,
    naming: &Naming,
    taken: &dyn Fn(&str) -> bool,
) -> Option<ResRef> {
    let lower = template.to_ascii_lowercase();
    let base = naming.strip(lower.strip_prefix("nw_").unwrap_or(&lower));
    let stem = base.trim_end_matches(|c: char| c.is_ascii_digit());
    let n: u32 = base[stem.len()..].parse().unwrap_or(0);
    let stem = &stem[..stem.len().min(13)];
    let stem = if stem.is_empty() { "blueprint" } else { stem };
    naming.numbered(stem, n + 1, taken)
}

/// The fields a placed object has and its blueprint does not: where it
/// stands ([`crate::placement::fields_of`]), and a sound's
/// `GeneratedType`, an encounter's spawn points and the others' visual
/// transform.
fn placement_fields(t: ResType) -> Vec<&'static str> {
    let Some(kind) = ObjectKind::from_restype(t) else { return Vec::new() };
    let mut out = crate::placement::fields_of(kind);
    out.push(match kind {
        ObjectKind::Sound => "GeneratedType",
        ObjectKind::Encounter => "SpawnPointList",
        ObjectKind::Trigger => return out,
        _ => "VisTransformList",
    });
    out
}

/// Where a held item sits (kept in the holder's list entry).
const HELD: [&str; 5] = ["Repos_PosX", "Repos_Posy", "Dropable", "Pickpocketable", "Infinite"];

/// Makes `instance` (a placed object of blueprint type `restype`) into a
/// new custom blueprint, with its items. `original` finds the blueprint a
/// resref names (the module's, else the game's); `taken` says whether a
/// resource name is in use; `naming` how new resrefs are made (by the
/// builder's code, in a project with one).
pub fn add_to_palette(
    game: &GameData,
    restype: ResType,
    instance: &Struct,
    original: &dyn Fn(ResKey) -> Option<Struct>,
    taken: &dyn Fn(&ResKey) -> bool,
    naming: &Naming,
) -> Option<Added> {
    let mut blueprints: Vec<(ResKey, Gff)> = Vec::new();
    let mut new_instance = instance.clone();
    let template_field = ObjectKind::resref_field_of(restype);
    let resref = {
        let used = |name: &str, blueprints: &[(ResKey, Gff)]| {
            let key = ResKey::parse(name, restype);
            key.is_some_and(|k| taken(&k) || blueprints.iter().any(|(b, _)| *b == k))
        };
        let template = instance.resref(template_field).map(|r| r.to_string()).unwrap_or_default();
        new_resref_named(&template, naming, &|n| used(n, &blueprints))?
    };
    let from = instance
        .resref(template_field)
        .filter(|r| !r.is_empty())
        .and_then(|r| original(ResKey::new(r, restype)));
    let mut bp = blueprint_fields(game, restype, instance, from.as_ref(), resref);

    // The items it holds, each a new item blueprint.
    let item_blueprint = |item: &Struct, blueprints: &mut Vec<(ResKey, Gff)>| -> ResRef {
        let template = item.resref("TemplateResRef").map(|r| r.to_string()).unwrap_or_default();
        // (The object's own new name is taken too: an item may hold one
        // of its own blueprint.)
        let own = ResKey::new(resref, restype);
        let used = |name: &str| {
            let key = ResKey::parse(name, ResType::UTI);
            key.is_some_and(|k| k == own || taken(&k) || blueprints.iter().any(|(b, _)| *b == k))
        };
        let r = new_resref_named(&template, naming, &used).unwrap_or(ResRef::EMPTY);
        let from = item
            .resref("TemplateResRef")
            .filter(|t| !t.is_empty())
            .and_then(|t| original(ResKey::new(t, ResType::UTI)));
        let mut fields = blueprint_fields(game, ResType::UTI, item, from.as_ref(), r);
        for label in HELD {
            fields.remove(label);
        }
        let mut g = Gff::new(*b"UTI ");
        g.root = fields;
        blueprints.push((ResKey::new(r, ResType::UTI), g));
        r
    };
    let mut items = Vec::new();
    // Inventories: the object's own, and each store page's.
    let holders: Vec<(Option<usize>, &str)> = match restype {
        ResType::UTM => (0..instance.list("StoreList").map_or(0, <[_]>::len))
            .map(|i| (Some(i), "ItemList"))
            .collect(),
        _ => vec![(None, "ItemList")],
    };
    for (page, list) in holders {
        let held = match page {
            Some(p) => instance.list("StoreList").and_then(|l| l.get(p)).and_then(|s| s.list(list)),
            None => instance.list(list),
        };
        let Some(held) = held.map(<[Struct]>::to_vec) else { continue };
        let mut entries = Vec::new();
        let mut renamed = Vec::new();
        for (i, item) in held.iter().enumerate() {
            let r = item_blueprint(item, &mut items);
            let mut entry = Struct::new(i as u32);
            entry.set("InventoryRes", Value::resref(r));
            for label in HELD {
                if let Some(v) = item.get(label) {
                    entry.set(label, v.clone());
                }
            }
            entries.push(entry);
            let mut now = item.clone();
            now.set("TemplateResRef", Value::resref(r));
            renamed.push(now);
        }
        match page {
            Some(p) => {
                if let Some(pages) = bp.list_mut("StoreList") {
                    pages[p].set(list, Value::List(entries));
                }
                if let Some(pages) = new_instance.list_mut("StoreList") {
                    pages[p].set(list, Value::List(renamed));
                }
            }
            None => {
                bp.set(list, Value::List(entries));
                new_instance.set(list, Value::List(renamed));
            }
        }
    }
    // Equipment: held by resref in each slot, with what is said of it there
    // (`Dropable`, `Pickpocketable`), as the backpack's.
    if let Some(equipped) = instance.list("Equip_ItemList").map(<[Struct]>::to_vec) {
        let mut entries = Vec::new();
        let mut renamed = Vec::new();
        for item in &equipped {
            let r = item_blueprint(item, &mut items);
            let mut entry = Struct::new(item.id);
            entry.set("EquippedRes", Value::resref(r));
            for label in ["Dropable", "Pickpocketable"] {
                if let Some(v) = item.get(label) {
                    entry.set(label, v.clone());
                }
            }
            entries.push(entry);
            let mut now = item.clone();
            now.set("TemplateResRef", Value::resref(r));
            renamed.push(now);
        }
        bp.set("Equip_ItemList", Value::List(entries));
        new_instance.set("Equip_ItemList", Value::List(renamed));
    }
    // The palette's fields go last, as Aurora writes them.
    let (palette_field, comment) =
        if restype == ResType::UTM { ("ID", "Comment") } else { ("PaletteID", "Comment") };
    for label in [palette_field, comment] {
        if let Some(v) = bp.remove(label) {
            bp.set(label, v);
        }
    }
    new_instance.set(template_field, Value::resref(resref));
    let mut g = Gff::new(file_type(restype));
    g.root = bp;
    blueprints.push((ResKey::new(resref, restype), g));
    blueprints.rotate_right(1);
    blueprints.extend(items);
    Some(Added { blueprints, instance: new_instance })
}

/// A placed object's (or held item's) fields as a blueprint named `resref`
/// (the palette's fields from `from`, the blueprint it was placed from).
fn blueprint_fields(
    game: &GameData,
    restype: ResType,
    placed: &Struct,
    from: Option<&Struct>,
    resref: ResRef,
) -> Struct {
    let mut s = placed.clone();
    s.id = 0;
    for label in placement_fields(restype) {
        s.remove(label);
    }
    let template_field = ObjectKind::resref_field_of(restype);
    s.set(template_field, Value::resref(resref));
    // A portrait the old blueprint named by resref: its row.
    if s.integer("PortraitId") == Some(0)
        && let Some(portrait) = from.and_then(|f| f.resref("Portrait")).filter(|p| !p.is_empty())
        && let Some(row) = portrait_row(game, &portrait.to_string())
    {
        s.set("PortraitId", Value::Word(row));
    }
    let (palette_field, palette_default) = if restype == ResType::UTM {
        ("ID", Value::Byte(0))
    } else {
        ("PaletteID", Value::Byte(0))
    };
    let palette = from.and_then(|f| f.get(palette_field)).cloned().unwrap_or(palette_default);
    s.set(palette_field, palette);
    let comment = from.and_then(|f| f.get("Comment")).cloned();
    s.set("Comment", comment.unwrap_or(Value::String(Vec::new())));
    s
}

/// portraits.2da's row for a portrait resref (`po_` and its base).
fn portrait_row(game: &GameData, portrait: &str) -> Option<u16> {
    let t = game.table("portraits").ok()?;
    let base = portrait.to_ascii_lowercase();
    let base = base.strip_prefix("po_").unwrap_or(&base);
    (0..t.len())
        .find(|&r| t.get(r, "BaseResRef").is_some_and(|b| b.trim().eq_ignore_ascii_case(base)))
        .and_then(|r| u16::try_from(r).ok())
}

/// A blueprint type's GFF file type.
fn file_type(t: ResType) -> [u8; 4] {
    let ext = t.extension().unwrap_or_default().to_ascii_uppercase();
    let b = ext.as_bytes();
    [
        b.first().copied().unwrap_or(b' '),
        b.get(1).copied().unwrap_or(b' '),
        b.get(2).copied().unwrap_or(b' '),
        b' ',
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A placed creature made a blueprint: its items' `Dropable` and
    /// `Pickpocketable` go into the blueprint's entries, equipped as in
    /// the backpack.
    #[test]
    fn a_creature_s_items_keep_their_flags_in_its_new_blueprint() {
        let game =
            GameData::new(mg_resman::ResMan::new(), mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let item = |id: u32, template: &str, flag: &str| {
            let mut s = Struct::new(id);
            s.set("TemplateResRef", Value::resref(ResRef::from_str(template).unwrap()));
            s.set("BaseItem", Value::Int(1));
            s.set(flag, Value::Byte(1));
            s
        };
        let mut creature = Struct::new(4);
        creature.set("TemplateResRef", Value::resref(ResRef::from_str("bandit").unwrap()));
        creature.set("Equip_ItemList", Value::List(vec![item(16, "sword", "Dropable")]));
        let mut pack = item(0, "torch", "Pickpocketable");
        pack.set("Repos_PosX", Value::Word(0));
        pack.set("Repos_Posy", Value::Word(0));
        creature.set("ItemList", Value::List(vec![pack]));
        let naming = Naming::new("ab", crate::project_file::DEFAULT_PATTERN);
        let added =
            add_to_palette(&game, ResType::UTC, &creature, &|_| None, &|_| false, &naming).unwrap();
        let bp = &added.blueprints[0].1.root;
        let hand = &bp.list("Equip_ItemList").unwrap()[0];
        assert_eq!(hand.id, 16);
        assert_eq!(hand.get("Dropable"), Some(&Value::Byte(1)));
        assert!(hand.get("Pickpocketable").is_none(), "only what it had");
        let held = &bp.list("ItemList").unwrap()[0];
        assert_eq!(held.get("Pickpocketable"), Some(&Value::Byte(1)));
    }

    #[test]
    fn an_edit_copy_is_numbered_from_the_template_s_number() {
        let copy = |t: &str, taken: &dyn Fn(&str) -> bool| {
            copy_resref_named(t, &Naming::default(), taken).unwrap().to_string()
        };
        // Aurora's (captured): the template in the module, the next number.
        assert_eq!(copy("nw_w1copy007", &|n| n == "nw_w1copy007"), "w1copy008");
        assert_eq!(copy("nw_wp_tavern", &|_| false), "wp_tavern001");
        assert_eq!(copy("w1copy007", &|n| n == "w1copy008"), "w1copy009", "never one in use");
    }

    #[test]
    fn a_builder_s_code_begins_the_new_resref() {
        let ab = Naming::new("ab", crate::project_file::DEFAULT_PATTERN);
        let game = |n: &str| n == "nw_wswls001";
        let new = |t: &str, taken: &dyn Fn(&str) -> bool| {
            new_resref_named(t, &ab, taken).unwrap().to_string()
        };
        assert_eq!(new("nw_wswls001", &game), "ab_wswls003", "past the number, as Aurora");
        assert_eq!(new("ab_wswls002", &|n| n == "ab_wswls002"), "ab_wswls004");
        assert_eq!(new("averyveryverylongname", &|_| false), "ab_averyveryv001");
    }

    #[test]
    fn new_resrefs_as_aurora_names_them() {
        // Aurora's choices (captured): past the template's own number and
        // those after it in use, one more.
        let game = |n: &str| {
            n == "nw_wswls001"
                || (1..=23).any(|i| n == format!("nw_it_mpotion{i:03}"))
                || n == "mgp_utp001x"
        };
        assert_eq!(new_resref("nw_wswls001", &game).unwrap().to_string(), "wswls003");
        assert_eq!(new_resref("nw_it_mpotion001", &game).unwrap().to_string(), "it_mpotion025");
        assert_eq!(new_resref("mgp_utp", &game).unwrap().to_string(), "mgp_utp001");
        // Never a name in use.
        let taken = |n: &str| n == "mgp_utp001" || n == "mgp_utp002";
        assert_eq!(new_resref("mgp_utp", &taken).unwrap().to_string(), "mgp_utp003");
        let long = new_resref("averyveryverylongname", &|_| false).unwrap();
        assert_eq!(long.to_string(), "averyveryvery001");
    }
}
