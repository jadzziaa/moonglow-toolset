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
use mg_rules::GameData;

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
    let lower = template.to_ascii_lowercase();
    let (prefix, base) = match lower.strip_prefix("nw_") {
        Some(rest) => ("nw_", rest),
        None => ("", lower.as_str()),
    };
    let stem = base.trim_end_matches(|c: char| c.is_ascii_digit());
    let mut n: u32 = base[stem.len()..].parse().unwrap_or(0);
    let stem = &stem[..stem.len().min(13)];
    let stem = if stem.is_empty() { "blueprint" } else { stem };
    let used = |n: u32| taken(&format!("{prefix}{stem}{n:03}")) || taken(&format!("{stem}{n:03}"));
    while n < 999 && used(n) {
        n += 1;
    }
    (n + 1..1000)
        .map(|n| format!("{stem}{n:03}"))
        .find(|name| !taken(name))
        .and_then(|name| ResRef::from_str(&name).ok())
}

/// The fields a placed object has and its blueprint does not.
fn placement_fields(t: ResType) -> &'static [&'static str] {
    match t {
        ResType::UTP | ResType::UTD => &["X", "Y", "Z", "Bearing", "VisTransformList"],
        ResType::UTS => &["XPosition", "YPosition", "ZPosition", "GeneratedType"],
        ResType::UTT => &[
            "XPosition",
            "YPosition",
            "ZPosition",
            "XOrientation",
            "YOrientation",
            "ZOrientation",
            "Geometry",
        ],
        ResType::UTE => &["XPosition", "YPosition", "ZPosition", "Geometry", "SpawnPointList"],
        _ => &[
            "XPosition",
            "YPosition",
            "ZPosition",
            "XOrientation",
            "YOrientation",
            "VisTransformList",
        ],
    }
}

/// Where a held item sits (kept in the holder's list entry).
const HELD: [&str; 5] = ["Repos_PosX", "Repos_Posy", "Dropable", "Pickpocketable", "Infinite"];

/// Makes `instance` (a placed object of blueprint type `restype`) into a
/// new custom blueprint, with its items. `original` finds the blueprint a
/// resref names (the module's, else the game's); `taken` says whether a
/// resource name is in use.
pub fn add_to_palette(
    game: &GameData,
    restype: ResType,
    instance: &Struct,
    original: &dyn Fn(ResKey) -> Option<Struct>,
    taken: &dyn Fn(&ResKey) -> bool,
) -> Option<Added> {
    let mut blueprints: Vec<(ResKey, Gff)> = Vec::new();
    let mut new_instance = instance.clone();
    let template_field = if restype == ResType::UTM { "ResRef" } else { "TemplateResRef" };
    let resref = {
        let used = |name: &str, blueprints: &[(ResKey, Gff)]| {
            let key = ResKey::parse(name, restype);
            key.is_some_and(|k| taken(&k) || blueprints.iter().any(|(b, _)| *b == k))
        };
        let template = instance.resref(template_field).map(|r| r.to_string()).unwrap_or_default();
        new_resref(&template, &|n| used(n, &blueprints))?
    };
    let from = instance
        .resref(template_field)
        .filter(|r| !r.is_empty())
        .and_then(|r| original(ResKey::new(r, restype)));
    let mut bp = blueprint_fields(game, restype, instance, from.as_ref(), resref);

    // The items it holds, each a new item blueprint.
    let item_blueprint = |item: &Struct, blueprints: &mut Vec<(ResKey, Gff)>| -> ResRef {
        let template = item.resref("TemplateResRef").map(|r| r.to_string()).unwrap_or_default();
        let used = |name: &str| {
            let key = ResKey::parse(name, ResType::UTI);
            key.is_some_and(|k| taken(&k) || blueprints.iter().any(|(b, _)| *b == k))
        };
        let r = new_resref(&template, &used).unwrap_or(ResRef::EMPTY);
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
    // Equipment: held by resref in each slot.
    if let Some(equipped) = instance.list("Equip_ItemList").map(<[Struct]>::to_vec) {
        let mut entries = Vec::new();
        let mut renamed = Vec::new();
        for item in &equipped {
            let r = item_blueprint(item, &mut items);
            let mut entry = Struct::new(item.id);
            entry.set("EquippedRes", Value::resref(r));
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
    let template_field = if restype == ResType::UTM { "ResRef" } else { "TemplateResRef" };
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
