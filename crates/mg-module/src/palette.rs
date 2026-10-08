//! Blueprint palettes (ITP): trees of categories with blueprints in them.
//! The game ships a skeleton (`<type>pal.itp`, the categories) and a
//! standard palette (`<type>palstd.itp`) per blueprint type; a module keeps a
//! custom one (`<type>palcus.itp`), which Aurora rebuilds from the module's
//! blueprints: each in the category its `PaletteID` (stores: `ID`) names,
//! under its name, creatures with their challenge rating and faction.

use mg_core::{Codepage, ResRef, ResType, StrRef};
use mg_gff::{Gff, Struct, Value};
use mg_rules::GameData;

use crate::Module;
use crate::new::word_sort_key;

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

    /// The field that holds a blueprint's own resref (stores: `ResRef`).
    pub fn resref_field(self) -> &'static str {
        if self == BlueprintKind::Store { "ResRef" } else { "TemplateResRef" }
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

    /// The palette skeleton (`<type>pal.itp`): the categories blueprints
    /// of the type go in. The game has one; a module (or a hak) may have
    /// its own, with categories of its own.
    pub fn skeleton_key(self) -> mg_resman::ResKey {
        mg_resman::ResKey::parse(&format!("{}pal", self.name()), ResType::ITP).expect("valid")
    }
}

/// The palette skeleton of `kind` the module's blueprints are sorted by:
/// the module's own if it has one, else the one the game's load order has
/// (a hak's, or the game's).
pub fn skeleton(module: &Module, game: &GameData, kind: BlueprintKind) -> Result<Gff, String> {
    let key = kind.skeleton_key();
    if let Some(own) = module.gff(&key) {
        return own.map_err(|e| format!("{key}: {e}"));
    }
    let data = game.resman.get(&key).map_err(|e| e.to_string())?;
    Gff::read(&data).map_err(|e| e.to_string())
}

/// A place in a skeleton: the node's position at each level, from `MAIN`
/// down through the `LIST`s.
pub type NodePath = Vec<usize>;

/// The most categories a skeleton has: a blueprint's category is a byte,
/// and 255 hides it.
pub const CATEGORIES_MAX: u32 = 255;

/// The skeleton's "assign to new category" placeholder (`TYPE` 0), which
/// is no category.
pub fn is_placeholder(node: &Struct) -> bool {
    node.integer("TYPE") == Some(0) && node.integer("ID").is_none()
}

fn skeleton_node<'a>(skeleton: &'a mut Gff, path: &[usize]) -> Option<&'a mut Struct> {
    let (first, rest) = path.split_first()?;
    let mut node = skeleton.root.list_mut("MAIN")?.get_mut(*first)?;
    for i in rest {
        node = node.list_mut("LIST")?.get_mut(*i)?;
    }
    Some(node)
}

/// The list a new node goes in: `MAIN`, or the `LIST` of the node at
/// `parent` (made if it has none).
fn skeleton_list<'a>(skeleton: &'a mut Gff, parent: &[usize]) -> Option<&'a mut Vec<Struct>> {
    if parent.is_empty() {
        if skeleton.root.list("MAIN").is_none() {
            skeleton.root.set("MAIN", Value::List(Vec::new()));
        }
        return skeleton.root.list_mut("MAIN");
    }
    let node = skeleton_node(skeleton, parent)?;
    if node.list("LIST").is_none() {
        node.set("LIST", Value::List(Vec::new()));
    }
    node.list_mut("LIST")
}

/// A name written out: `NAME` (what EE reads), and `DELETE_ME`, BioWare's
/// older field of the same use, in the game's bytes for the text
/// (`codepage`: the game's DM client reads a palette's names as it reads
/// all its text, so UTF-8 here shows there as "HipÃ³lito"). A talk-table
/// name goes.
fn set_name(node: &mut Struct, name: &str, codepage: Codepage) -> Result<(), String> {
    let bytes = codepage.encode(name).ok_or_else(|| {
        format!("the game's text ({codepage:?}) has no way to write some of \"{name}\"")
    })?;
    node.remove("STRREF");
    node.set("NAME", Value::String(bytes.to_vec()));
    node.set("DELETE_ME", Value::String(bytes.into_owned()));
    Ok(())
}

fn checked_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() { Err("a category needs a name".into()) } else { Ok(name) }
}

/// Adds a category named `name` to the skeleton, in the group at `parent`
/// (empty: at the top), with the next id the skeleton has free. Returns
/// where it is and its id.
pub fn add_category(
    skeleton: &mut Gff,
    parent: &[usize],
    name: &str,
    codepage: Codepage,
) -> Result<(NodePath, u8), String> {
    let name = checked_name(name)?;
    // The id after every one in use, and after the skeleton's own count.
    let used = Palette::read(skeleton).ids().into_iter().max().map_or(0, |m| u32::from(m) + 1);
    let next = skeleton.root.integer("NEXT_USEABLE_ID").unwrap_or(0).max(0) as u32;
    let id = used.max(next);
    if id >= CATEGORIES_MAX {
        return Err(format!("a palette has at most {CATEGORIES_MAX} categories"));
    }
    let mut node = Struct::new(1);
    set_name(&mut node, name, codepage)?;
    let list = skeleton_list(skeleton, parent).ok_or("there is no such group")?;
    node.set("ID", Value::Byte(id as u8));
    list.push(node);
    let mut path = parent.to_vec();
    path.push(list.len() - 1);
    skeleton.root.set("NEXT_USEABLE_ID", Value::Byte((id + 1) as u8));
    Ok((path, id as u8))
}

/// Adds a group (a branch that holds categories, and is none itself) named
/// `name`, in the group at `parent` (empty: at the top).
pub fn add_group(
    skeleton: &mut Gff,
    parent: &[usize],
    name: &str,
    codepage: Codepage,
) -> Result<NodePath, String> {
    let name = checked_name(name)?;
    let mut node = Struct::new(1);
    set_name(&mut node, name, codepage)?;
    let list = skeleton_list(skeleton, parent).ok_or("there is no such group")?;
    node.set("LIST", Value::List(Vec::new()));
    list.push(node);
    let mut path = parent.to_vec();
    path.push(list.len() - 1);
    Ok(path)
}

/// Renames the group or category at `path`.
pub fn rename_category(
    skeleton: &mut Gff,
    path: &[usize],
    name: &str,
    codepage: Codepage,
) -> Result<(), String> {
    let name = checked_name(name)?;
    let node = skeleton_node(skeleton, path).ok_or("there is no such category")?;
    set_name(node, name, codepage)
}

/// Removes the group or category at `path`, with what is in it. Returns
/// the ids of the categories removed: blueprints in them are in no
/// category of the skeleton any more.
pub fn remove_category(skeleton: &mut Gff, path: &[usize]) -> Result<Vec<u8>, String> {
    let (last, parent) = path.split_last().ok_or("there is no such category")?;
    let list = if parent.is_empty() {
        skeleton.root.list_mut("MAIN")
    } else {
        skeleton_node(skeleton, parent).and_then(|n| n.list_mut("LIST"))
    };
    let list = list.filter(|l| *last < l.len()).ok_or("there is no such category")?;
    let removed = list.remove(*last);
    let mut holder = Gff::new(*b"ITP ");
    holder.root.set("MAIN", Value::List(vec![removed]));
    Ok(Palette::read(&holder).ids())
}

/// Moves the group or category at `from` into the group at `parent`
/// (empty: the top), before what is at `index` there now (past the end:
/// last). Returns where it is afterwards. A group can't go into itself.
pub fn move_category(
    skeleton: &mut Gff,
    from: &[usize],
    parent: &[usize],
    index: usize,
) -> Result<NodePath, String> {
    let (&at, old_parent) = from.split_last().ok_or("there is no such category")?;
    if parent.starts_with(from) {
        return Err("a group can't be moved into itself".into());
    }
    if skeleton_node(skeleton, from).is_none() {
        return Err("there is no such category".into());
    }
    if !parent.is_empty() && skeleton_node(skeleton, parent).is_none() {
        return Err("there is no such group".into());
    }
    let old = if old_parent.is_empty() {
        skeleton.root.list_mut("MAIN")
    } else {
        skeleton_node(skeleton, old_parent).and_then(|n| n.list_mut("LIST"))
    };
    let node = old.ok_or("there is no such category")?.remove(at);
    // Taken out, what came after it in its list is one place up: the new
    // place too, if it is in that list or under one of those.
    let mut parent = parent.to_vec();
    let mut index = index;
    if parent == old_parent {
        if index > at {
            index -= 1;
        }
    } else if parent.starts_with(old_parent) && parent[old_parent.len()] > at {
        parent[old_parent.len()] -= 1;
    }
    let list = skeleton_list(skeleton, &parent).ok_or("there is no such group")?;
    let index = index.min(list.len());
    list.insert(index, node);
    parent.push(index);
    Ok(parent)
}

/// Puts every level of the skeleton in the order of its names (Windows'
/// word sort, as Aurora lists them), the placeholder first: how the
/// game's categories show before a module arranges its own. `name` looks
/// up a StrRef.
pub fn sort_by_name(skeleton: &mut Gff, name: &dyn Fn(u32) -> String) {
    fn sort(list: &mut [Struct], name: &dyn Fn(u32) -> String) {
        list.sort_by_cached_key(|n| {
            let shown = match (n.string("NAME"), n.integer("STRREF")) {
                (Some(text), _) => written(text),
                (None, Some(s)) => u32::try_from(s).map(name).unwrap_or_default(),
                (None, None) => n.string("DELETE_ME").map(written).unwrap_or_default(),
            };
            (!is_placeholder(n), word_sort_key(&shown))
        });
        for node in list {
            if let Some(children) = node.list_mut("LIST") {
                sort(children, name);
            }
        }
    }
    if let Some(main) = skeleton.root.list_mut("MAIN") {
        sort(main, name);
    }
}

/// How many of the module's blueprints of `kind` are in each of the
/// categories `ids`.
pub fn blueprints_in(module: &Module, kind: BlueprintKind, ids: &[u8]) -> usize {
    module
        .keys_of(kind.restype())
        .filter_map(|key| module.gff(key)?.ok())
        .filter_map(|g| g.root.integer(kind.palette_field()).and_then(|v| u8::try_from(v).ok()))
        .filter(|id| ids.contains(id))
        .count()
}

/// A category's or blueprint's name in a palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteName {
    StrRef(u32),
    /// A name written out, as the file has it: the game's bytes.
    Text(Vec<u8>),
}

impl PaletteName {
    /// The text, looking talk-table strings up in `game` and reading a
    /// name written out as the game reads text in its language.
    pub fn text(&self, game: &GameData) -> String {
        match self {
            PaletteName::StrRef(s) => game.string(StrRef(*s)).unwrap_or_default(),
            PaletteName::Text(t) => text_of(t, game.codepage()),
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

/// A name written out in a palette (`NAME`, `DELETE_ME`), as text: the
/// game's bytes, in `codepage` (a skeleton's "Compañeros" in
/// Windows-1252, a Polish module's names in Windows-1250). Names that are
/// UTF-8 are read as that: Moonglow wrote them so until 1.19.3 (which
/// the game showed as "HipÃ³lito"); they are written right when the
/// palettes are next made.
pub fn text_of(bytes: &[u8], codepage: Codepage) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => codepage.decode(bytes).into_owned(),
    }
}

/// [`text_of`] where the language is not known (to put names in order):
/// as Windows-1252.
pub fn written(bytes: &[u8]) -> String {
    text_of(bytes, Codepage::WINDOWS_1252)
}

/// A name's bytes as a palette is to carry them: a name left as UTF-8 by
/// Moonglow before 1.19.4 (anything but plain ASCII that reads as UTF-8)
/// in the game's bytes for it, where the codepage has them; any other as
/// it is.
fn game_bytes(bytes: &[u8], codepage: Codepage) -> Vec<u8> {
    if bytes.is_ascii() {
        return bytes.to_vec();
    }
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| codepage.encode(text))
        .map_or_else(|| bytes.to_vec(), |b| b.into_owned())
}

fn name_of(s: &Struct) -> PaletteName {
    match (s.string("NAME"), s.integer("STRREF")) {
        (Some(t), _) => PaletteName::Text(t.to_vec()),
        (None, Some(r)) => PaletteName::StrRef(u32::try_from(r).unwrap_or(u32::MAX)),
        // (A skeleton's node with neither: BioWare's older name field.)
        (None, None) => PaletteName::Text(s.string("DELETE_ME").unwrap_or_default().to_vec()),
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
                                faction: c.string("FACTION").map(written),
                            })
                        })
                        .collect(),
                });
            }
            out
        }
        Palette { nodes: nodes(g.root.list("MAIN").unwrap_or(&[])) }
    }

    /// Every category id of the palette.
    pub fn ids(&self) -> Vec<u8> {
        fn walk(n: &[PaletteNode], out: &mut Vec<u8>) {
            for node in n {
                out.extend(node.id);
                walk(&node.children, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.nodes, &mut out);
        out
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

/// [`blueprint_name`] as the bytes the name has in the blueprint (or the
/// talk table): what a palette carries, so that the game's DM client,
/// which reads a palette's names as it reads the blueprints', shows the
/// same (an accent, a color token's bytes).
fn blueprint_name_bytes(kind: BlueprintKind, s: &Struct, game: &GameData) -> Vec<u8> {
    s.locstring(kind.name_field()).and_then(|l| game.locstring_bytes(l)).unwrap_or_default()
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
    // (The module's own categories, if it has a skeleton of its own: in
    // the order it has them. The game's are by name, as Aurora lists them.)
    let own = module.contains(&kind.skeleton_key());
    let skeleton = skeleton(module, game, kind)?;
    let name = |s: u32| game.string(StrRef(s)).unwrap_or_default();
    let mut palette = crate::new::custom_palette_in(&skeleton, name, !own);
    // (Names as the files have them: bytes, not text read and written
    // again.)
    let factions: Vec<Vec<u8>> = module
        .gff(&mg_resman::ResKey::parse("repute", ResType::FAC).expect("valid"))
        .and_then(Result::ok)
        .and_then(|f| {
            f.root.list("FactionList").map(|l| {
                l.iter().map(|s| s.string("FactionName").unwrap_or_default().to_vec()).collect()
            })
        })
        .unwrap_or_default();
    // (Categories a module named in Moonglow before 1.19.4 are UTF-8 in
    // its skeleton: in the game's bytes here.)
    if let Some(Value::List(main)) = palette.root.get_mut("MAIN") {
        names_in_game_bytes(main, game.codepage());
    }
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
        leaf.set("NAME", Value::String(blueprint_name_bytes(kind, &g.root, game)));
        leaf.set("RESREF", Value::resref(key.resref));
        if kind == BlueprintKind::Creature {
            leaf.set("CR", Value::Float(g.root.float("ChallengeRating").unwrap_or(0.0)));
            let faction = g
                .root
                .integer("FactionID")
                .and_then(|f| factions.get(usize::try_from(f).ok()?).cloned())
                .unwrap_or_default();
            leaf.set("FACTION", Value::String(faction));
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

/// Puts the names of a palette's nodes that were left in UTF-8 (by
/// Moonglow before 1.19.4) in the game's bytes ([`game_bytes`]); whether
/// any changed.
fn names_in_game_bytes(list: &mut [Struct], codepage: Codepage) -> bool {
    let mut changed = false;
    for node in list {
        for label in ["NAME", "DELETE_ME"] {
            let Some(was) = node.string(label).map(<[u8]>::to_vec) else { continue };
            let now = game_bytes(&was, codepage);
            if now != was {
                node.set(label, Value::String(now));
                changed = true;
            }
        }
        if let Some(Value::List(children)) = node.get_mut("LIST") {
            changed |= names_in_game_bytes(children, codepage);
        }
    }
    changed
}

/// Rebuilds every custom palette of the module from its blueprints
/// ([`rebuild_custom_palette`]); how many changed. The module's own
/// categories (its skeletons) named in UTF-8 before 1.19.4 are put in
/// the game's bytes too, for Aurora and the game to read.
pub fn rebuild_custom_palettes(module: &mut Module, game: &GameData) -> Result<usize, String> {
    let mut changed = 0;
    for kind in BlueprintKind::ALL {
        let key = kind.skeleton_key();
        if module.contains(&key)
            && let Some(Ok(mut own)) = module.gff(&key)
            && let Some(Value::List(main)) = own.root.get_mut("MAIN")
            && names_in_game_bytes(main, game.codepage())
        {
            let bytes = own.to_bytes().map_err(|e| e.to_string())?;
            module.set(key, bytes);
        }
        let g = rebuild_custom_palette(module, game, kind)?;
        let bytes = g.to_bytes().map_err(|e| e.to_string())?;
        if module.get(&kind.custom_key()) != Some(&bytes[..]) {
            module.set(kind.custom_key(), bytes);
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A skeleton whose categories are named in `DELETE_ME` alone (no
    /// StrRef, no `NAME`: a builder's palettes) keeps their names in the
    /// palette built from it, in the game's code page.
    #[test]
    fn categories_named_in_the_older_field_alone_keep_their_names() {
        let node = |name: &[u8], id: Option<u8>, children: Vec<Struct>| {
            let mut s = Struct::new(0);
            s.set("DELETE_ME", Value::String(name.to_vec()));
            if let Some(id) = id {
                s.set("ID", Value::Byte(id));
            }
            if !children.is_empty() {
                s.set("LIST", Value::List(children));
            }
            s
        };
        let mut skeleton = Gff::new(*b"ITP ");
        let branch = node(b"Aliados", None, vec![node(b"Compa\xF1eros", Some(3), Vec::new())]);
        skeleton.root.set("MAIN", Value::List(vec![branch]));
        let built = crate::new::custom_palette(&skeleton, |_| String::new());
        let main = built.root.list("MAIN").unwrap();
        assert_eq!(main[0].string("NAME"), Some(&b"Aliados"[..]));
        let leaf = &main[0].list("LIST").unwrap()[0];
        assert_eq!(leaf.string("NAME"), Some(&b"Compa\xF1eros"[..]));
        assert_eq!(written(leaf.string("NAME").unwrap()), "Compañeros");
        assert!(leaf.get("DELETE_ME").is_none(), "the palette has the name once");
        // And read back as a palette: named.
        assert_eq!(name_of(leaf), PaletteName::Text(b"Compa\xf1eros".to_vec()));
        // A name in UTF-8 (a blueprint's, as Moonglow writes it) is read
        // as that.
        assert_eq!(written("Kryształowa czaszka".as_bytes()), "Kryształowa czaszka");
    }

    /// A custom palette carries its blueprints' names, its factions' and
    /// its categories' as the game reads them: the bytes they have in the
    /// module, not UTF-8 (which the game's DM client showed as
    /// "HipÃ³lito", and a color token's bytes as others). A category left
    /// in UTF-8 by Moonglow before 1.19.4 is put right.
    #[test]
    fn a_custom_palette_carries_names_in_the_game_s_bytes() {
        use mg_core::{Gender, Language, LocString};
        let game = GameData::new(mg_resman::ResMan::new(), mg_tlk::Tlk::new(Language::ENGLISH));
        let mut module = Module::new();
        let key = |name: &str, t: ResType| mg_resman::ResKey::parse(name, t).unwrap();
        let category = |name: &[u8], id: u8| {
            let mut s = Struct::new(1);
            s.set("NAME", Value::String(name.to_vec()));
            s.set("ID", Value::Byte(id));
            s
        };
        for kind in BlueprintKind::ALL {
            let mut skeleton = Gff::new(*b"ITP ");
            // (The second as Moonglow wrote "Compañía" before: UTF-8.)
            let main = vec![category(b"Libros", 0), category("Compañía".as_bytes(), 1)];
            skeleton.root.set("MAIN", Value::List(main));
            module.set_gff(kind.skeleton_key(), &skeleton).unwrap();
        }
        let blueprint = |kind: BlueprintKind, name: &[u8], category: u8| {
            let mut g = Gff::new(*b"UTI ");
            let name = LocString::from_text(Language::ENGLISH, Gender::Male, name.to_vec());
            g.root.set(kind.name_field(), Value::LocString(name));
            g.root.set(kind.palette_field(), Value::Byte(category));
            g
        };
        let book = blueprint(BlueprintKind::Item, b"Hip\xf3lito", 0);
        module.set_gff(key("book", ResType::UTI), &book).unwrap();
        // (A color token's bytes are any: one that Windows-1252 has no
        // letter for among them.)
        let forge = blueprint(BlueprintKind::Item, b"<c5\xa9\x81>Forja</c>", 1);
        module.set_gff(key("forge", ResType::UTI), &forge).unwrap();
        let mut guard = blueprint(BlueprintKind::Creature, b"Guardi\xe1n", 0);
        guard.root.set("FactionID", Value::Word(1));
        module.set_gff(key("guard", ResType::UTC), &guard).unwrap();
        let mut repute = Gff::new(*b"FAC ");
        let faction = |name: &[u8]| {
            let mut s = Struct::new(0);
            s.set("FactionName", Value::String(name.to_vec()));
            s
        };
        let factions = vec![faction(b"PC"), faction(b"Compa\xf1\xeda")];
        repute.root.set("FactionList", Value::List(factions));
        module.set_gff(key("repute", ResType::FAC), &repute).unwrap();

        let items = rebuild_custom_palette(&module, &game, BlueprintKind::Item).unwrap();
        let main = items.root.list("MAIN").unwrap();
        let named = |node: &Struct| node.string("NAME").unwrap().to_vec();
        let leaf = |category: usize| main[category].list("LIST").unwrap()[0].clone();
        assert_eq!(named(&leaf(0)), b"Hip\xf3lito");
        assert_eq!(named(&leaf(1)), b"<c5\xa9\x81>Forja</c>");
        assert_eq!(named(&main[0]), b"Libros");
        assert_eq!(named(&main[1]), b"Compa\xf1\xeda", "a name left in UTF-8 is put right");
        // Made with the rest at a save: the module's own categories too.
        let mut saved = module.clone();
        rebuild_custom_palettes(&mut saved, &game).unwrap();
        let own = saved.gff(&BlueprintKind::Item.skeleton_key()).unwrap().unwrap();
        assert_eq!(named(&own.root.list("MAIN").unwrap()[1]), b"Compa\xf1\xeda");
        let creatures = rebuild_custom_palette(&module, &game, BlueprintKind::Creature).unwrap();
        let guard = &creatures.root.list("MAIN").unwrap()[0].list("LIST").unwrap()[0];
        assert_eq!(named(guard), b"Guardi\xe1n");
        assert_eq!(guard.string("FACTION"), Some(&b"Compa\xf1\xeda"[..]));
        // Read back, they show as they are written (and as they were when
        // they were UTF-8).
        let read = Palette::read(&items);
        assert_eq!(read.nodes[0].blueprints[0].name.text(&game), "Hipólito");
        assert_eq!(read.nodes[1].name.text(&game), "Compañía");
        assert_eq!(text_of("Compañía".as_bytes(), Codepage::WINDOWS_1252), "Compañía");
        assert_eq!(
            Palette::read(&creatures).nodes[0].blueprints[0].faction.as_deref(),
            Some("Compañía")
        );
        // A category named in Moonglow is written in the game's bytes; a
        // name the game's text cannot hold is refused, not written as "?".
        let mut skeleton = module.gff(&BlueprintKind::Item.skeleton_key()).unwrap().unwrap();
        let (path, _) = add_category(&mut skeleton, &[], "Año", Codepage::WINDOWS_1252).unwrap();
        let added = skeleton_node(&mut skeleton, &path).unwrap();
        assert_eq!(added.string("NAME"), Some(&b"A\xf1o"[..]));
        assert!(add_category(&mut skeleton, &[], "Żółw", Codepage::WINDOWS_1252).is_err());
        assert!(add_category(&mut skeleton, &[], "Żółw", Codepage::WINDOWS_1250).is_ok());
    }

    /// A skeleton as the game's: the placeholder, a group of two
    /// categories, a category.
    fn a_skeleton() -> Gff {
        let node =
            |strref: u32, id: Option<u8>, kind: Option<u8>, children: Option<Vec<Struct>>| {
                let mut s = Struct::new(1);
                s.set("STRREF", Value::Dword(strref));
                if let Some(id) = id {
                    s.set("ID", Value::Byte(id));
                }
                if let Some(kind) = kind {
                    s.set("TYPE", Value::Byte(kind));
                }
                if let Some(children) = children {
                    s.set("LIST", Value::List(children));
                }
                s
            };
        let special = vec![node(6688, Some(0), None, None), node(6689, Some(1), None, None)];
        let mut g = Gff::new(*b"ITP ");
        g.root.set(
            "MAIN",
            Value::List(vec![
                node(6733, None, Some(0), None),
                node(6687, None, Some(2), Some(special)),
                node(6782, Some(6), None, None),
            ]),
        );
        g.root.set("NEXT_USEABLE_ID", Value::Byte(7));
        g
    }

    #[test]
    fn categories_are_added_renamed_and_removed() {
        let mut g = a_skeleton();
        assert!(is_placeholder(&g.root.list("MAIN").unwrap()[0]));
        assert!(!is_placeholder(&g.root.list("MAIN").unwrap()[1]));
        // A category at the top: the next id, and the count moves on.
        let (path, id) = add_category(&mut g, &[], " Ruins ", Codepage::WINDOWS_1252).unwrap();
        assert_eq!((path.as_slice(), id), (&[3][..], 7));
        assert_eq!(g.root.integer("NEXT_USEABLE_ID"), Some(8));
        let ruins = &g.root.list("MAIN").unwrap()[3];
        assert_eq!(ruins.string("NAME"), Some(&b"Ruins"[..]));
        assert_eq!(ruins.string("DELETE_ME"), Some(&b"Ruins"[..]), "BioWare's older field too");
        assert_eq!(ruins.integer("ID"), Some(7));
        // A group, and a category in it.
        let group = add_group(&mut g, &[], "Planar", Codepage::WINDOWS_1252).unwrap();
        assert_eq!(group, [4]);
        let (path, id) = add_category(&mut g, &group, "Abyss", Codepage::WINDOWS_1252).unwrap();
        assert_eq!((path.as_slice(), id), (&[4, 0][..], 8));
        // One in a group of the game's.
        let (path, id) = add_category(&mut g, &[1], "Custom 3", Codepage::WINDOWS_1252).unwrap();
        assert_eq!((path.as_slice(), id), (&[1, 2][..], 9));
        assert_eq!(Palette::read(&g).ids(), [0, 1, 9, 6, 7, 8]);
        // Renamed: its talk-table name goes.
        rename_category(&mut g, &[2], "Chests", Codepage::WINDOWS_1252).unwrap();
        let chests = &g.root.list("MAIN").unwrap()[2];
        assert_eq!((chests.string("NAME"), chests.get("STRREF")), (Some(&b"Chests"[..]), None));
        assert_eq!(chests.integer("ID"), Some(6), "its id stays: blueprints keep their category");
        // Removed, a group takes its categories with it.
        assert_eq!(remove_category(&mut g, &[4]), Ok(vec![8]));
        assert_eq!(remove_category(&mut g, &[1]), Ok(vec![0, 1, 9]));
        assert_eq!(Palette::read(&g).ids(), [6, 7]);
        // An id is never given twice, whatever was removed.
        assert_eq!(add_category(&mut g, &[], "New", Codepage::WINDOWS_1252).unwrap().1, 10);

        // What can't be done changes nothing.
        let before = g.clone();
        assert!(add_category(&mut g, &[], "  ", Codepage::WINDOWS_1252).is_err());
        assert!(add_category(&mut g, &[9], "Nowhere", Codepage::WINDOWS_1252).is_err());
        assert!(rename_category(&mut g, &[9], "Nothing", Codepage::WINDOWS_1252).is_err());
        assert!(remove_category(&mut g, &[]).is_err());
        assert!(remove_category(&mut g, &[9]).is_err());
        assert_eq!(g, before);
        g.root.set("NEXT_USEABLE_ID", Value::Byte(255));
        assert!(add_category(&mut g, &[], "One too many", Codepage::WINDOWS_1252).is_err());
    }

    #[test]
    fn categories_are_moved_and_put_in_order() {
        let ids = |g: &Gff| Palette::read(g).ids();
        let mut g = a_skeleton();
        assert_eq!(ids(&g), [0, 1, 6]);
        // Within the top: the last before the group.
        assert_eq!(move_category(&mut g, &[2], &[], 1), Ok(vec![1]));
        assert_eq!(ids(&g), [6, 0, 1]);
        // Down its own list: the place counted as it was before the move.
        assert_eq!(move_category(&mut g, &[1], &[], 3), Ok(vec![2]));
        assert_eq!(ids(&g), [0, 1, 6]);
        // Into a group that comes after it, whose place moves up by one.
        assert_eq!(move_category(&mut g, &[2], &[], 1), Ok(vec![1]));
        assert_eq!(move_category(&mut g, &[1], &[2], 1), Ok(vec![1, 1]));
        assert_eq!(ids(&g), [0, 6, 1]);
        // Out again, to the end.
        assert_eq!(move_category(&mut g, &[1, 1], &[], 9), Ok(vec![2]));
        assert_eq!(ids(&g), [0, 1, 6]);
        // Not into itself, nor from or to nowhere.
        let before = g.clone();
        assert!(move_category(&mut g, &[1], &[1], 0).is_err());
        assert!(move_category(&mut g, &[7], &[], 0).is_err());
        assert!(move_category(&mut g, &[2], &[7], 0).is_err());
        assert_eq!(g, before);

        // By name: the placeholder stays first.
        let name = |s: u32| match s {
            6687 => "Special".into(),
            6688 => "Zeta".into(),
            6689 => "Alpha".into(),
            6782 => "Containers".into(),
            _ => "Assign".into(),
        };
        sort_by_name(&mut g, &name);
        assert!(is_placeholder(&g.root.list("MAIN").unwrap()[0]));
        assert_eq!(ids(&g), [6, 1, 0], "Containers, then Special's Alpha and Zeta");
    }
}
