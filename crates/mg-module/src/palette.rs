//! Blueprint palettes (ITP): trees of categories with blueprints in them.
//! The game ships a skeleton (`<type>pal.itp`, the categories) and a
//! standard palette (`<type>palstd.itp`) per blueprint type; a module keeps a
//! custom one (`<type>palcus.itp`), which Aurora rebuilds from the module's
//! blueprints: each in the category its `PaletteID` (stores: `ID`) names,
//! under its name, creatures with their challenge rating and faction.

use mg_core::{ResRef, ResType, StrRef};
use mg_gff::{Gff, Struct, Value};
use mg_rules::GameData;

use crate::Module;
use crate::new::{custom_palette, word_sort_key};

/// A blueprint type with a palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlueprintKind {
    Creature,
    Door,
    Encounter,
    Item,
    Placeable,
    Sound,
    Store,
    Trigger,
    Waypoint,
}

impl BlueprintKind {
    pub const ALL: [BlueprintKind; 9] = [
        BlueprintKind::Creature,
        BlueprintKind::Door,
        BlueprintKind::Encounter,
        BlueprintKind::Item,
        BlueprintKind::Placeable,
        BlueprintKind::Sound,
        BlueprintKind::Store,
        BlueprintKind::Trigger,
        BlueprintKind::Waypoint,
    ];

    /// As in the palette file names (`<name>pal.itp`).
    pub fn name(self) -> &'static str {
        match self {
            BlueprintKind::Creature => "creature",
            BlueprintKind::Door => "door",
            BlueprintKind::Encounter => "encounter",
            BlueprintKind::Item => "item",
            BlueprintKind::Placeable => "placeable",
            BlueprintKind::Sound => "sound",
            BlueprintKind::Store => "store",
            BlueprintKind::Trigger => "trigger",
            BlueprintKind::Waypoint => "waypoint",
        }
    }

    /// As Aurora's palette tabs say it.
    pub fn label(self) -> &'static str {
        match self {
            BlueprintKind::Creature => "Creatures",
            BlueprintKind::Door => "Doors",
            BlueprintKind::Encounter => "Encounters",
            BlueprintKind::Item => "Items",
            BlueprintKind::Placeable => "Placeables",
            BlueprintKind::Sound => "Sounds",
            BlueprintKind::Store => "Stores",
            BlueprintKind::Trigger => "Triggers",
            BlueprintKind::Waypoint => "Waypoints",
        }
    }

    /// The blueprint resource type.
    pub fn restype(self) -> ResType {
        match self {
            BlueprintKind::Creature => ResType::UTC,
            BlueprintKind::Door => ResType::UTD,
            BlueprintKind::Encounter => ResType::UTE,
            BlueprintKind::Item => ResType::UTI,
            BlueprintKind::Placeable => ResType::UTP,
            BlueprintKind::Sound => ResType::UTS,
            BlueprintKind::Store => ResType::UTM,
            BlueprintKind::Trigger => ResType::UTT,
            BlueprintKind::Waypoint => ResType::UTW,
        }
    }

    pub fn from_restype(t: ResType) -> Option<BlueprintKind> {
        BlueprintKind::ALL.into_iter().find(|k| k.restype() == t)
    }

    /// The field that holds a blueprint's category (stores call it `ID`).
    pub fn palette_field(self) -> &'static str {
        if self == BlueprintKind::Store { "ID" } else { "PaletteID" }
    }

    /// The field that holds a blueprint's name (creatures: first name).
    pub fn name_field(self) -> &'static str {
        match self {
            BlueprintKind::Creature => "FirstName",
            BlueprintKind::Encounter
            | BlueprintKind::Item
            | BlueprintKind::Trigger
            | BlueprintKind::Waypoint => "LocalizedName",
            _ => "LocName",
        }
    }

    /// The module's custom palette resource.
    pub fn custom_key(self) -> mg_resman::ResKey {
        mg_resman::ResKey::parse(&format!("{}palcus", self.name()), ResType::ITP).expect("valid")
    }
}

/// A category's or blueprint's name in a palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteName {
    StrRef(u32),
    Text(String),
}

impl PaletteName {
    /// The text, looking talk-table strings up in `game`.
    pub fn text(&self, game: &GameData) -> String {
        match self {
            PaletteName::StrRef(s) => game.string(StrRef(*s)).unwrap_or_default(),
            PaletteName::Text(t) => t.clone(),
        }
    }
}

/// A blueprint in a palette.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteBlueprint {
    pub resref: ResRef,
    pub name: PaletteName,
    /// Creatures: challenge rating and faction name.
    pub cr: Option<f32>,
    pub faction: Option<String>,
}

/// A palette branch or category.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteNode {
    pub name: PaletteName,
    /// The category id blueprints use; `None` for branches.
    pub id: Option<u8>,
    pub children: Vec<PaletteNode>,
    pub blueprints: Vec<PaletteBlueprint>,
}

/// A palette tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Palette {
    pub nodes: Vec<PaletteNode>,
}

fn name_of(s: &Struct) -> PaletteName {
    match (s.string("NAME"), s.integer("STRREF")) {
        (Some(t), _) => PaletteName::Text(String::from_utf8_lossy(t).into_owned()),
        (None, Some(r)) => PaletteName::StrRef(u32::try_from(r).unwrap_or(u32::MAX)),
        (None, None) => PaletteName::Text(String::new()),
    }
}

impl Palette {
    /// Any palette file: skeleton, standard or custom.
    pub fn read(g: &Gff) -> Palette {
        fn nodes(list: &[Struct]) -> Vec<PaletteNode> {
            let mut out = Vec::new();
            for s in list {
                if s.resref("RESREF").is_some() {
                    continue;
                }
                let children = s.list("LIST").unwrap_or(&[]);
                out.push(PaletteNode {
                    name: name_of(s),
                    id: s.integer("ID").and_then(|v| u8::try_from(v).ok()),
                    children: nodes(children),
                    blueprints: children
                        .iter()
                        .filter_map(|c| {
                            Some(PaletteBlueprint {
                                resref: c.resref("RESREF")?,
                                name: name_of(c),
                                cr: c.float("CR"),
                                faction: c
                                    .string("FACTION")
                                    .map(|f| String::from_utf8_lossy(f).into_owned()),
                            })
                        })
                        .collect(),
                });
            }
            out
        }
        Palette { nodes: nodes(g.root.list("MAIN").unwrap_or(&[])) }
    }

    /// Every blueprint with its category id.
    pub fn blueprints(&self) -> Vec<(Option<u8>, &PaletteBlueprint)> {
        fn walk<'a>(n: &'a [PaletteNode], out: &mut Vec<(Option<u8>, &'a PaletteBlueprint)>) {
            for node in n {
                out.extend(node.blueprints.iter().map(|b| (node.id, b)));
                walk(&node.children, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.nodes, &mut out);
        out
    }

    /// The categories blueprints can go in: id and path ("Special / Custom
    /// 1"), in tree order.
    pub fn categories(&self, game: &GameData) -> Vec<(u8, String)> {
        fn walk(n: &[PaletteNode], path: &str, game: &GameData, out: &mut Vec<(u8, String)>) {
            for node in n {
                let name = node.name.text(game);
                let here = if path.is_empty() { name } else { format!("{path} / {name}") };
                if let Some(id) = node.id {
                    out.push((id, here.clone()));
                }
                walk(&node.children, &here, game, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.nodes, "", game, &mut out);
        out
    }
}

/// The name a blueprint shows in palettes, in the game's language: its name
/// field (creatures: the first name only, as the shipped modules' palettes
/// have it).
pub fn blueprint_name(kind: BlueprintKind, s: &Struct, game: &GameData) -> String {
    s.locstring(kind.name_field()).and_then(|l| game.locstring(l)).unwrap_or_default()
}

/// The module's custom palette of `kind`, rebuilt from its blueprints: the
/// skeleton's categories (as [`custom_palette`]), each blueprint in the
/// category its palette field names (255 hides it, an unknown id drops it)
/// under its name, sorted by name; creatures with `CR` and `FACTION` (the
/// module's faction name).
pub fn rebuild_custom_palette(
    module: &Module,
    game: &GameData,
    kind: BlueprintKind,
) -> Result<Gff, String> {
    let skeleton_name = format!("{}pal", kind.name());
    let data = game.resman.get_named(&skeleton_name, ResType::ITP).map_err(|e| e.to_string())?;
    let skeleton = Gff::read(&data).map_err(|e| e.to_string())?;
    let mut palette = custom_palette(&skeleton, |s| game.string(StrRef(s)).unwrap_or_default());
    let factions: Vec<String> = module
        .gff(&mg_resman::ResKey::parse("repute", ResType::FAC).expect("valid"))
        .and_then(Result::ok)
        .and_then(|f| {
            f.root.list("FactionList").map(|l| {
                l.iter()
                    .map(|s| {
                        String::from_utf8_lossy(s.string("FactionName").unwrap_or_default())
                            .into_owned()
                    })
                    .collect()
            })
        })
        .unwrap_or_default();
    // Blueprints by category.
    let mut by_category: std::collections::BTreeMap<u8, Vec<(String, Struct)>> = Default::default();
    for key in module.keys_of(kind.restype()).copied().collect::<Vec<_>>() {
        let Some(Ok(g)) = module.gff(&key) else { continue };
        let Some(id) = g.root.integer(kind.palette_field()).and_then(|v| u8::try_from(v).ok())
        else {
            continue;
        };
        if id == 255 {
            continue;
        }
        let name = blueprint_name(kind, &g.root, game);
        let mut leaf = Struct::new(0);
        leaf.set("NAME", Value::String(name.clone().into_bytes()));
        leaf.set("RESREF", Value::resref(key.resref));
        if kind == BlueprintKind::Creature {
            leaf.set("CR", Value::Float(g.root.float("ChallengeRating").unwrap_or(0.0)));
            let faction = g
                .root
                .integer("FactionID")
                .and_then(|f| factions.get(usize::try_from(f).ok()?).cloned())
                .unwrap_or_default();
            leaf.set("FACTION", Value::String(faction.into_bytes()));
        }
        by_category.entry(id).or_default().push((name, leaf));
    }
    fn fill(
        list: &mut [Struct],
        by_category: &mut std::collections::BTreeMap<u8, Vec<(String, Struct)>>,
    ) {
        for node in list {
            if let Some(id) = node.integer("ID").and_then(|v| u8::try_from(v).ok())
                && let Some(mut leaves) = by_category.remove(&id)
            {
                // By name, then (for equal names) by resref.
                leaves.sort_by(|a, b| {
                    word_sort_key(&a.0).cmp(&word_sort_key(&b.0)).then_with(|| {
                        a.1.resref("RESREF")
                            .map(|r| r.to_string())
                            .cmp(&b.1.resref("RESREF").map(|r| r.to_string()))
                    })
                });
                let mut children = match node.get("LIST") {
                    Some(Value::List(l)) => l.clone(),
                    _ => Vec::new(),
                };
                children.extend(leaves.into_iter().map(|(_, s)| s));
                node.set("LIST", Value::List(children));
            }
            if let Some(Value::List(children)) = node.get_mut("LIST") {
                fill(children, by_category);
            }
        }
    }
    if let Some(Value::List(main)) = palette.root.get_mut("MAIN") {
        fill(main, &mut by_category);
    }
    Ok(palette)
}

/// Rebuilds every custom palette of the module from its blueprints
/// ([`rebuild_custom_palette`]); how many changed.
pub fn rebuild_custom_palettes(module: &mut Module, game: &GameData) -> Result<usize, String> {
    let mut changed = 0;
    for kind in BlueprintKind::ALL {
        let g = rebuild_custom_palette(module, game, kind)?;
        let bytes = g.to_bytes().map_err(|e| e.to_string())?;
        if module.get(&kind.custom_key()) != Some(&bytes[..]) {
            module.set(kind.custom_key(), bytes);
            changed += 1;
        }
    }
    Ok(changed)
}
