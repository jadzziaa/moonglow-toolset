//! `mg haks --usage`: which of the resources in a module's haks the
//! module reaches, hak by hak, and which none of it does: for cutting a
//! release (or a download) to what is used.
//!
//! The walk starts at every resource of the module and follows, wherever
//! in the load order what it names is (a hak's blueprint, a game's
//! script…), and what that names in turn:
//!
//! - the references Verify follows ([`mg_module::refs`]): scripts (a
//!   script's compiled and source file both), conversations, blueprints,
//!   sounds, tilesets, areas, portraits (every size of a portrait), and a
//!   script's `#include`s;
//! - a script's string literals: a string that is the name of a script,
//!   blueprint, conversation, sound, music, 2DA, model or texture reaches
//!   it (the call the string is passed to is not read);
//! - what an area is drawn with: its tileset (its `.set` and palette) and,
//!   of the tiles, those the area places: each one's model, walkmesh
//!   (`.wok`) and map image; the tileset's grass texture and environment
//!   map; its skybox's models (skyboxes.2da), its loading screen
//!   (loadscreens.2da), its ambient sounds (ambientsound.2da) and music
//!   (ambientmusic.2da and its stingers);
//! - what an object looks like, as the area view draws it
//!   ([`mg_preview`]): a placeable's or door's model and its sounds
//!   (placeableobjsnds.2da), a creature's body (heads, body parts, wings,
//!   tails, the armor it wears), its portrait and its sound set (the
//!   `.ssf` and its sounds), an item's models (with every race's, gender's
//!   and phenotype's armor part of the numbers it has, as anyone may wear
//!   it) and their inventory icons;
//! - a model's own files ([`mg_preview::files`]): its walkmeshes, textures
//!   (as the game picks a DDS or TGA, and a PLT), materials (`.mtr`) and
//!   TXIs and what they name, its emitters' models and its supermodels;
//! - the scripts the game runs from its tables as the module sees them
//!   ([`mg_module::verify::table_scripts`]: spells' impact scripts, areas
//!   of effect's, states', traps', diseases' and poisons'), reached from
//!   the table.
//!
//! Not followed, and so not judged by it: what scripts reach by a number
//! (visual effects, spells, item properties, appearances set by a
//! script), what players choose (heads, portraits, sound sets and parts at
//! character creation), what the game reads by itself (2DAs other than
//! those the walk reads, GUI, fonts, shaders a material doesn't name,
//! palettes) and a hak's replacements of the game's own resources, which
//! the game's own content may use: these are "not judged", never "unused".

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use mg_2da::TwoDa;
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_preview::files::Gatherer;
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_rules::GameData;

/// One hak's resources, by what the module makes of them.
#[derive(Debug, Default)]
pub(crate) struct HakUsage {
    pub(crate) name: String,
    pub(crate) path: Option<PathBuf>,
    /// Each with its size and the module resource whose walk reached it
    /// first.
    pub(crate) used: Vec<(ResKey, u64, ResKey)>,
    pub(crate) unused: Vec<(ResKey, u64)>,
    /// Never loaded from this hak: another layer (a hak higher in the
    /// list, the module) has it, which is named.
    pub(crate) shadowed: Vec<(ResKey, u64, String)>,
    /// Of a kind the walk doesn't judge, or a replacement of the game's own.
    pub(crate) not_judged: Vec<(ResKey, u64)>,
}

impl HakUsage {
    pub(crate) fn total(&self) -> usize {
        self.used.len() + self.unused.len() + self.shadowed.len() + self.not_judged.len()
    }

    pub(crate) fn bytes(&self) -> u64 {
        let sum = |v: &mut dyn Iterator<Item = u64>| v.sum::<u64>();
        sum(&mut self.used.iter().map(|u| u.1))
            + sum(&mut self.unused.iter().map(|u| u.1))
            + sum(&mut self.shadowed.iter().map(|u| u.1))
            + sum(&mut self.not_judged.iter().map(|u| u.1))
    }

    pub(crate) fn used_bytes(&self) -> u64 {
        self.used.iter().map(|u| u.1).sum()
    }
}

/// The kinds of resource the game reads by itself (or the toolset does):
/// not judged unless the walk reaches one.
fn judged(t: ResType) -> bool {
    !matches!(
        t,
        ResType::TWODA
            | ResType::GUI
            | ResType::INI
            | ResType::TXT
            | ResType::IDS
            | ResType::LTR
            | ResType::TTF
            | ResType::FNT
            | ResType::JUI
            | ResType::SHD
            | ResType::ITP
            | ResType::BIK
            | ResType::WBM
            | ResType::TLK
            | ResType::TML
            | ResType::SQL
            | ResType::SQ3
            | ResType::LUA
            | ResType::PNG
            | ResType::JPG
            | ResType::BMP
            | ResType::RES
    )
}

/// What a walk can follow further when it reaches it.
fn followed(t: ResType) -> bool {
    t.is_gff() || t == ResType::NSS || t == ResType::SSF
}

/// The types a script's string may name (models and textures are tried
/// apart).
const NAMED_IN_SCRIPTS: [ResType; 15] = [
    ResType::NCS,
    ResType::NSS,
    ResType::UTC,
    ResType::UTI,
    ResType::UTP,
    ResType::UTD,
    ResType::UTE,
    ResType::UTM,
    ResType::UTS,
    ResType::UTT,
    ResType::UTW,
    ResType::DLG,
    ResType::WAV,
    ResType::BMU,
    ResType::TWODA,
];

pub(crate) struct Walk<'a> {
    game: &'a GameData,
    gather: Gatherer<'a>,
    /// Every resource reached, with the module resource the walk began at.
    pub(crate) from: HashMap<ResKey, ResKey>,
    queue: Vec<ResKey>,
    root: ResKey,
    /// How much of what `gather` found is in `from` already.
    gathered: usize,
    tilesets: HashMap<ResRef, Option<Arc<mg_set::Tileset>>>,
    /// Part-based races' letters and the phenotypes, for armor parts
    /// anyone may wear.
    wearers: Option<(Vec<char>, Vec<usize>)>,
    /// Armor parts and icons done.
    parts: std::collections::HashSet<String>,
}

fn text(s: &Struct, label: &str) -> Option<String> {
    match s.get(label)? {
        Value::String(b) | Value::ResRef(b) if !b.is_empty() => {
            Some(String::from_utf8_lossy(b).to_ascii_lowercase())
        }
        _ => None,
    }
}

fn cell(t: &TwoDa, row: i64, column: &str) -> Option<String> {
    let v = t.get(usize::try_from(row).ok()?, column)?.trim();
    (!v.is_empty() && v != "****").then(|| v.to_ascii_lowercase())
}

impl<'a> Walk<'a> {
    pub(crate) fn new(game: &'a GameData) -> Walk<'a> {
        Walk {
            game,
            gather: Gatherer::new(&game.resman),
            from: HashMap::new(),
            queue: Vec::new(),
            root: ResKey::new(ResRef::EMPTY, ResType::RES),
            gathered: 0,
            tilesets: HashMap::new(),
            wearers: None,
            parts: Default::default(),
        }
    }

    fn rm(&self) -> &'a ResMan {
        &self.game.resman
    }

    /// Walks from every resource of the module.
    pub(crate) fn module(&mut self, m: &Module) {
        let mut keys: Vec<ResKey> = m.keys().copied().collect();
        keys.sort();
        for k in keys {
            self.root = k;
            self.reach(k);
            while let Some(next) = self.queue.pop() {
                self.follow(next);
                self.settle();
            }
        }
        // And the scripts the game runs from its tables, as the module
        // sees them (a hak's spell scripts its spells.2da names), reached
        // from the table.
        let read = |name: &str| self.game.table(name).ok();
        let run = mg_module::verify::table_scripts(&read);
        for r in run {
            self.root = r.from;
            for c in r.candidates() {
                self.reach(c);
            }
            while let Some(next) = self.queue.pop() {
                self.follow(next);
                self.settle();
            }
        }
    }

    /// Notes a resource the load order has as reached (to follow further,
    /// where it names more); whether it has it.
    fn reach(&mut self, key: ResKey) -> bool {
        if !self.rm().contains(&key) {
            return false;
        }
        if !self.from.contains_key(&key) {
            self.from.insert(key, self.root);
            if followed(key.restype) {
                self.queue.push(key);
            }
        }
        true
    }

    fn reach_named(&mut self, name: &str, t: ResType) -> bool {
        ResKey::parse(name, t).is_some_and(|k| self.reach(k))
    }

    /// What the model walk found since, as reached.
    fn settle(&mut self) {
        for f in &self.gather.files()[self.gathered..] {
            self.from.entry(f.key).or_insert(self.root);
        }
        self.gathered = self.gather.files().len();
    }

    /// A table the walk reads, which it reaches.
    fn table(&mut self, name: &str) -> Option<Arc<TwoDa>> {
        self.reach_named(name, ResType::TWODA);
        self.game.table(name).ok()
    }

    fn follow(&mut self, key: ResKey) {
        let Ok(data) = self.rm().get(&key) else { return };
        let data = data.into_owned();
        if key.restype == ResType::NSS {
            for r in mg_module::refs::script_includes(key, &data) {
                self.reach(ResKey::new(r.target, ResType::NSS));
            }
            self.strings(&data);
            return;
        }
        if key.restype == ResType::SSF {
            if let Ok(ssf) = mg_ssf::Ssf::read(&data) {
                for e in &ssf.entries {
                    self.reach_named(&String::from_utf8_lossy(&e.sound), ResType::WAV);
                }
            }
            return;
        }
        let Ok(gff) = Gff::read(&data) else { return };
        for r in mg_module::refs::gff_references(key, &gff) {
            match r.kind {
                mg_module::refs::RefKind::Portrait => self.portrait(&r.target.to_string()),
                _ => {
                    for c in r.candidates() {
                        self.reach(c);
                    }
                }
            }
        }
        let root = &gff.root;
        match key.restype {
            ResType::ARE => self.area(root),
            ResType::GIT => self.placed(root),
            ResType::UTC => self.creature(root),
            ResType::UTP => self.placeable(root),
            ResType::UTD => self.door(root),
            ResType::UTI => self.item(root, false),
            _ => {}
        }
        if matches!(key.restype, ResType::UTC | ResType::UTP | ResType::UTM) {
            self.items_in(root);
        }
    }

    /// A script's string literals that name something.
    fn strings(&mut self, src: &[u8]) {
        for r in mg_module::refs::script_strings(src) {
            let name = r.to_string().to_ascii_lowercase();
            for ty in NAMED_IN_SCRIPTS {
                self.reach(ResKey::new(r, ty));
            }
            if self.rm().contains(&ResKey::new(r, ResType::MDL)) {
                self.gather.add_model(&name);
            }
            if self.rm().texture_type(r).is_some()
                || self.rm().contains(&ResKey::new(r, ResType::PLT))
            {
                self.gather.add_texture(&name);
            }
        }
    }

    /// A portrait's every size (`po_name` + h, l, m, s, t, i).
    fn portrait(&mut self, base: &str) {
        for size in ["h", "l", "m", "s", "t", "i"] {
            self.gather.add_texture(&format!("{base}{size}"));
        }
    }

    fn portrait_id(&mut self, s: &Struct) {
        let Some(id) = s.integer("PortraitId").filter(|&i| i > 0 && i < 0xFFFF) else { return };
        if let Some(t) = self.table("portraits")
            && let Some(base) = cell(&t, id, "BaseResRef")
        {
            self.portrait(&format!("po_{base}"));
        }
    }

    fn preview(&mut self, p: Result<mg_preview::Preview, mg_preview::PreviewError>, s: &Struct) {
        if let Ok(p) = p {
            self.gather.preview(&mg_preview::replaced(p, s));
        }
    }

    fn area(&mut self, are: &Struct) {
        let mut set = None;
        if let Some(name) = text(are, "Tileset")
            && let Ok(r) = ResRef::from_str(&name)
        {
            self.reach_named(&format!("{name}palstd"), ResType::ITP);
            set = self.tileset(r);
        }
        // (The tables its skybox and loading screen are read from.)
        self.table("skyboxes");
        if are.integer("LoadScreenID").is_some_and(|r| r > 0) {
            self.table("loadscreens");
        }
        // Its tiles' models, walkmeshes and minimap pictures, the
        // tileset's maps, the skybox and the loading screen, as Export
        // Files gathers them.
        self.gather.area(self.game, are, set.as_deref(), None);
    }

    fn tileset(&mut self, r: ResRef) -> Option<Arc<mg_set::Tileset>> {
        if let Some(t) = self.tilesets.get(&r) {
            return t.clone();
        }
        let key = ResKey::new(r, ResType::SET);
        self.reach(key);
        let cp = self.game.codepage();
        let set = (self.rm().get(&key).ok())
            .and_then(|d| mg_set::Tileset::parse(&d, cp).ok())
            .map(Arc::new);
        self.tilesets.insert(r, set.clone());
        set
    }

    /// An area's placed objects, and its sounds and music.
    fn placed(&mut self, git: &Struct) {
        if let Some(p) = git.child("AreaProperties") {
            if let Some(t) = self.table("ambientsound") {
                for f in ["AmbientSndDay", "AmbientSndNight"] {
                    if let Some(w) = p.integer(f).and_then(|r| cell(&t, r, "Resource")) {
                        self.reach_named(&w, ResType::WAV);
                    }
                }
            }
            if let Some(t) = self.table("ambientmusic") {
                for f in ["MusicDay", "MusicNight", "MusicBattle"] {
                    for c in ["Resource", "Stinger1", "Stinger2", "Stinger3"] {
                        if let Some(m) = p.integer(f).and_then(|r| cell(&t, r, c)) {
                            self.reach_named(&m, ResType::BMU);
                            self.reach_named(&m, ResType::WAV);
                        }
                    }
                }
            }
        }
        use mg_rules::ObjectKind as K;
        for kind in K::ALL {
            for s in git.list(kind.list()).unwrap_or_default() {
                match kind {
                    K::Creature => self.creature(s),
                    K::Placeable => self.placeable(s),
                    K::Door => self.door(s),
                    K::Item => {
                        let p = mg_preview::item_placed(self.game, s);
                        self.preview(p, s);
                        self.item(s, true);
                    }
                    _ => {}
                }
                self.items_in(s);
            }
        }
    }

    /// Items held (inventories, what is worn, a store's goods by its
    /// categories) as the objects they are, as a placed object keeps them.
    fn items_in(&mut self, s: &Struct) {
        for list in ["ItemList", "Equip_ItemList", "StoreList"] {
            for i in s.list(list).unwrap_or_default() {
                if i.get("BaseItem").is_some() {
                    self.item(i, false);
                }
                self.items_in(i);
            }
        }
    }

    fn creature(&mut self, s: &Struct) {
        let rm = self.rm();
        let items = |r: ResRef| -> Option<Gff> {
            Gff::read(&rm.get(&ResKey::new(r, ResType::UTI)).ok()?).ok()
        };
        let p = mg_preview::creature(self.game, s, &items);
        self.preview(p, s);
        self.portrait_id(s);
        if let Some(id) = s.integer("SoundSetFile").filter(|&i| i < 0xFFFF)
            && let Some(t) = self.table("soundset")
            && let Some(ssf) = cell(&t, id, "RESREF")
        {
            self.reach_named(&ssf, ResType::SSF);
        }
    }

    fn placeable(&mut self, s: &Struct) {
        let p = mg_preview::placeable(self.game, s);
        self.preview(p, s);
        self.portrait_id(s);
        let row = s.integer("Appearance").unwrap_or(-1);
        if let Some(t) = self.table("placeables")
            && let Some(sounds) = cell(&t, row, "SoundAppType")
        {
            self.object_sounds(&sounds);
        }
    }

    fn door(&mut self, s: &Struct) {
        let p = mg_preview::door(self.game, s);
        self.preview(p, s);
        self.portrait_id(s);
        let (table, row) = match s.integer("Appearance").unwrap_or(0) {
            0 => ("genericdoors", s.integer("GenericType_New").or(s.integer("GenericType"))),
            a => ("doortypes", Some(a)),
        };
        if let (Some(t), Some(row)) = (self.table(table), row)
            && let Some(sounds) = cell(&t, row, "SoundAppType")
        {
            self.object_sounds(&sounds);
        }
    }

    /// placeableobjsnds.2da's sounds of a row.
    fn object_sounds(&mut self, row: &str) {
        let Ok(row) = row.parse::<i64>() else { return };
        let Some(t) = self.table("placeableobjsnds") else { return };
        for c in ["Opened", "Closed", "Destroyed", "Used", "Locked"] {
            if let Some(w) = cell(&t, row, c) {
                self.reach_named(&w, ResType::WAV);
            }
        }
    }

    /// An item's models (`placed`: as it lies, drawn already) and their
    /// icons; armor's parts and a cloak as anyone may wear them.
    fn item(&mut self, s: &Struct, placed: bool) {
        let base = s.integer("BaseItem").unwrap_or(-1);
        let mut armor = false;
        if let Some(t) = self.table("baseitems") {
            if let Some(icon) = cell(&t, base, "DefaultIcon") {
                self.gather.add_texture(&icon);
            }
            armor = cell(&t, base, "ModelType").as_deref() == Some("3");
        }
        if armor {
            // (Its preview is a body wearing it: the body is no part of it.)
            for (mdl, field) in ARMOR_PARTS {
                let n = mg_rules::items::part_number(s, field).unwrap_or(0);
                if n > 0 {
                    self.worn(&format!("{mdl}{n:03}"));
                }
            }
            return;
        }
        let Ok(p) = mg_preview::item(self.game, s) else { return };
        let p = mg_preview::replaced(p, s);
        if !placed {
            self.gather.preview(&p);
        }
        for part in std::iter::once(&p.base).chain(&p.parts) {
            let name = part.model.to_ascii_lowercase();
            match armor_part(&name) {
                Some((_, _, _, rest)) => self.worn(rest),
                None if self.parts.insert(name.clone()) => {
                    self.gather.add_texture(&format!("i{name}"))
                }
                None => {}
            }
        }
    }

    /// A part worn (`chest001`, `cloak_001`): its model for every gender,
    /// part-based race and phenotype that has one, once.
    fn worn(&mut self, part: &str) {
        if !self.parts.insert(part.to_string()) {
            return;
        }
        self.find_wearers();
        let (races, phenotypes) = self.wearers.clone().unwrap_or_default();
        let rm = self.rm();
        for g in ['m', 'f'] {
            for r in &races {
                for ph in &phenotypes {
                    let variant = format!("p{g}{r}{ph}_{part}");
                    if ResKey::parse(&variant, ResType::MDL).is_some_and(|k| rm.contains(&k)) {
                        self.gather.add_model(&variant);
                    }
                }
            }
        }
    }

    /// The part-based races' letters (appearance.2da) and the phenotypes
    /// (phenotype.2da), once.
    fn find_wearers(&mut self) {
        if self.wearers.is_some() {
            return;
        }
        let mut races: Vec<char> = Vec::new();
        if let Some(t) = self.table("appearance") {
            for row in 0..t.len() {
                let part_based = cell(&t, row as i64, "MODELTYPE").is_some_and(|m| m.contains('p'));
                if let Some(r) = cell(&t, row as i64, "RACE").filter(|_| part_based)
                    && r.len() == 1
                    && let Some(c) = r.chars().next()
                    && !races.contains(&c)
                {
                    races.push(c);
                }
            }
        }
        let mut phenotypes = Vec::new();
        if let Some(t) = self.table("phenotype") {
            for row in 0..t.len() {
                if cell(&t, row as i64, "Label").is_some() {
                    phenotypes.push(row);
                }
            }
        }
        if phenotypes.is_empty() {
            phenotypes = vec![0, 2];
        }
        self.wearers = Some((races, phenotypes));
    }
}

/// An armor's parts: each one's model name (capart.2da `MDLNAME`) and
/// the item's field with its number.
const ARMOR_PARTS: [(&str, &str); 19] = [
    ("footr", "ArmorPart_RFoot"),
    ("footl", "ArmorPart_LFoot"),
    ("shinr", "ArmorPart_RShin"),
    ("shinl", "ArmorPart_LShin"),
    ("legl", "ArmorPart_LThigh"),
    ("legr", "ArmorPart_RThigh"),
    ("pelvis", "ArmorPart_Pelvis"),
    ("chest", "ArmorPart_Torso"),
    ("belt", "ArmorPart_Belt"),
    ("neck", "ArmorPart_Neck"),
    ("forer", "ArmorPart_RFArm"),
    ("forel", "ArmorPart_LFArm"),
    ("bicepr", "ArmorPart_RBicep"),
    ("bicepl", "ArmorPart_LBicep"),
    ("shor", "ArmorPart_RShoul"),
    ("shol", "ArmorPart_LShoul"),
    ("handr", "ArmorPart_RHand"),
    ("handl", "ArmorPart_LHand"),
    ("robe", "ArmorPart_Robe"),
];

/// An armor part's model name, `p` + gender + race + phenotype + `_` +
/// the part and its number (`pmh0_chest001`), in its pieces.
fn armor_part(name: &str) -> Option<(char, char, &str, &str)> {
    let rest = name.strip_prefix('p')?;
    let mut chars = rest.chars();
    let g = chars.next().filter(|c| matches!(c, 'm' | 'f'))?;
    let r = chars.next().filter(char::is_ascii_lowercase)?;
    let tail = &rest[2..];
    let (ph, part) = tail.split_once('_')?;
    (!ph.is_empty() && ph.bytes().all(|b| b.is_ascii_digit()) && !part.is_empty())
        .then_some((g, r, ph, part))
}

/// The game a player has, with the module's haks and the module: the
/// user's override and development folders left out (a player's game
/// doesn't have them), the haks found where the game finds them.
fn player_game(gi: &GameInstall, m: &Module) -> Result<GameData> {
    let player = GameInstall { user_dir: None, workshop: false, ..gi.clone() };
    let mut game = GameData::open(&player)?;
    game.resman.add_haks(gi, &m.haks()?)?;
    game.resman.add(priority::MODULE, "module", LayerClass::Erf, m.container());
    game.invalidate();
    Ok(game)
}

/// Each of the module's haks' resources, by what the module makes of
/// them; and how many resources the walk reached in all.
pub(crate) fn hak_usage(
    gi: &GameInstall,
    m: &Module,
    report: &mg_module::haks::HakReport,
) -> Result<(Vec<HakUsage>, usize)> {
    let game = player_game(gi, m)?;
    let mut walk = Walk::new(&game);
    walk.module(m);
    walk.settle();
    let mut out = Vec::new();
    for (name, path) in &report.haks {
        let index = match path {
            Some(p) => ErfContainer::open(p)?.index().to_vec(),
            None => Vec::new(),
        };
        let in_base = |k: &ResKey| report.overrides.contains_key(k);
        let mut u = attribute(&game.resman, &walk.from, name, &index, &in_base);
        u.path.clone_from(path);
        out.push(u);
    }
    Ok((out, walk.from.len()))
}

/// A hak's resources (`index`, with their sizes) by what the walk made
/// of them: used where it reached them and the hak's copy is the one the
/// load order gives, shadowed where another layer's is.
fn attribute(
    rm: &ResMan,
    from: &HashMap<ResKey, ResKey>,
    name: &str,
    index: &[(ResKey, u64)],
    in_base: &dyn Fn(&ResKey) -> bool,
) -> HakUsage {
    let mut u = HakUsage { name: name.to_string(), ..Default::default() };
    let label = format!("hak:{name}");
    for &(key, size) in index {
        match rm.origin(&key) {
            Some(o) if o != label => u.shadowed.push((key, size, o.to_string())),
            _ => match from.get(&key) {
                Some(f) => u.used.push((key, size, *f)),
                None if !judged(key.restype) || in_base(&key) => u.not_judged.push((key, size)),
                None => u.unused.push((key, size)),
            },
        }
    }
    u
}

/// Bytes for people: `412.3 MB`.
pub(crate) fn size(bytes: u64) -> String {
    let b = bytes as f64;
    match bytes {
        0..1_000 => format!("{bytes} B"),
        1_000..1_000_000 => format!("{:.1} KB", b / 1e3),
        1_000_000..1_000_000_000 => format!("{:.1} MB", b / 1e6),
        _ => format!("{:.2} GB", b / 1e9),
    }
}

/// The JSON of `mg haks --usage`, hak by hak.
pub(crate) fn json(usage: &[HakUsage]) -> serde_json::Value {
    use serde_json::json;
    let names =
        |v: &mut dyn Iterator<Item = &ResKey>| v.map(ToString::to_string).collect::<Vec<_>>();
    serde_json::Value::Array(
        usage
            .iter()
            .map(|u| {
                let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
                for (k, _, _) in &u.used {
                    *by_kind.entry(k.restype.extension().unwrap_or("?")).or_default() += 1;
                }
                json!({
                    "name": u.name,
                    "path": u.path.as_ref().map(|p| p.display().to_string()),
                    "resources": u.total(),
                    "bytes": u.bytes(),
                    "used": u.used.len(),
                    "used_bytes": u.used_bytes(),
                    "unused": u.unused.len(),
                    "shadowed": u.shadowed.len(),
                    "not_judged": u.not_judged.len(),
                    "used_by_type": by_kind,
                    "used_resources": u.used.iter().map(|(k, _, from)| json!({
                        "resource": k.to_string(), "from": from.to_string(),
                    })).collect::<Vec<_>>(),
                    "unused_resources": names(&mut u.unused.iter().map(|u| &u.0)),
                    "shadowed_resources": u.shadowed.iter().map(|(k, _, by)| json!({
                        "resource": k.to_string(), "by": by,
                    })).collect::<Vec<_>>(),
                    "not_judged_resources": names(&mut u.not_judged.iter().map(|u| &u.0)),
                })
            })
            .collect(),
    )
}

/// A hak's summary line.
pub(crate) fn line(u: &HakUsage) -> String {
    if u.path.is_none() {
        return format!("{}\tNOT FOUND", u.name);
    }
    format!(
        "{}\t{} of {} used ({} of {}); {} unused, {} shadowed, {} not judged",
        u.name,
        u.used.len(),
        u.total(),
        size(u.used_bytes()),
        size(u.bytes()),
        u.unused.len(),
        u.shadowed.len(),
        u.not_judged.len()
    )
}

#[cfg(test)]
mod tests {
    use mg_resman::MemContainer;

    use super::*;

    const MODEL: &str = "newmodel NAME\nsetsupermodel NAME NULL\nclassification Character\n\
        setanimationscale 1\nbeginmodelgeom NAME\nnode dummy NAME\n  parent NULL\nendnode\n\
        node trimesh box\n  parent NAME\n  bitmap TEX\n  verts 3\n    0 0 0\n    1 0 0\n    \
        0 1 0\n  faces 1\n    0 1 2 1 0 0 0 0\nendnode\nendmodelgeom NAME\ndonemodel NAME\n";

    fn model(name: &str, texture: &str) -> Vec<u8> {
        MODEL.replace("NAME", name).replace("TEX", texture).into_bytes()
    }

    fn key(name: &str) -> ResKey {
        ResKey::from_filename(name).unwrap()
    }

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    /// A module's area, its placeable, a script and what they name reach
    /// into two haks; what nothing reaches is unused, what a higher hak
    /// has is shadowed, a hak's 2DA is not judged.
    #[test]
    fn a_module_reaches_into_its_haks() {
        // The game: the placeables table, a tileset of two tiles.
        let mut stock = MemContainer::new();
        stock.insert(
            key("placeables.2da"),
            &b"2DA V2.0\n\n   Label ModelName SoundAppType\n0  Chest plc_mg01 0\n1  Box mg_box ****\n"[..],
        );
        stock.insert(
            key("placeableobjsnds.2da"),
            &b"2DA V2.0\n\n   Label Opened Closed Destroyed Used Locked\n0  wood mg_open **** **** **** ****\n"[..],
        );
        let set = "[GENERAL]\nName=mgt01\nEnvMap=mg_env\n[TILES]\nCount=2\n[TILE0]\nModel=mgt01_a01\n\
                   ImageMap2D=mi_mgt01_a01\n[TILE1]\nModel=mgt01_a02\n";
        stock.insert(key("mgt01.set"), set.as_bytes());
        // A hak of content, and one above it with a copy of one file.
        let mut content = MemContainer::new();
        content.insert(key("plc_mg01.mdl"), model("plc_mg01", "mg_wood"));
        content.insert(key("mg_wood.tga"), &b"tga"[..]);
        content.insert(key("mg_open.wav"), &b"wav"[..]);
        content.insert(key("mgt01_a01.mdl"), model("mgt01_a01", "mg_tile"));
        content.insert(key("mgt01_a01.wok"), &b"wok"[..]);
        content.insert(key("mg_tile.dds"), &b"dds"[..]);
        content.insert(key("mi_mgt01_a01.tga"), &b"tga"[..]);
        content.insert(key("mg_env.tga"), &b"tga"[..]);
        content.insert(key("mgt01_a02.mdl"), model("mgt01_a02", "mg_tile2"));
        content.insert(key("mg_named.utp"), {
            let mut g = Gff::new(*b"UTP ");
            g.root.set("Appearance", Value::Byte(1));
            g.root.set("OnUsed", Value::resref(rr("mg_hak_used")));
            g.to_bytes().unwrap()
        });
        content.insert(key("mg_box.mdl"), model("mg_box", "mg_box"));
        content.insert(key("mg_hak_used.ncs"), &b"NCS"[..]);
        content.insert(key("mg_unused.mdl"), model("mg_unused", "mg_wood"));
        content.insert(key("mg_rules.2da"), &b"2DA V2.0\n"[..]);
        let mut top = MemContainer::new();
        top.insert(key("mg_wood.tga"), &b"tga2"[..]);
        // The module: an area with one of the two tiles and a placeable,
        // a script naming a hak blueprint.
        let mut module = MemContainer::new();
        let mut are = Gff::new(*b"ARE ");
        are.root.set("Tileset", Value::resref(rr("mgt01")));
        let mut tile = Struct::new(1);
        tile.set("Tile_ID", Value::Int(0));
        are.root.set("Tile_List", Value::List(vec![tile]));
        module.insert(key("a1.are"), are.to_bytes().unwrap());
        let mut git = Gff::new(*b"GIT ");
        let mut p = Struct::new(9);
        p.set("Appearance", Value::Dword(0));
        git.root.set("Placeable List", Value::List(vec![p]));
        module.insert(key("a1.git"), git.to_bytes().unwrap());
        module.insert(key("spawn.nss"), &b"void main() { CreateObject(64, \"MG_Named\", l); }"[..]);

        let mut rm = ResMan::new();
        rm.add(priority::MODULE, "module", LayerClass::Erf, module);
        rm.add(priority::HAK_USER, "hak:mg_top", LayerClass::Erf, top);
        rm.add(priority::HAK_USER, "hak:mg_content", LayerClass::Erf, content);
        rm.add(priority::KEY, "nwn_base.key", LayerClass::Key, stock);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut m = Module::new();
        for k in ["a1.are", "a1.git", "spawn.nss"] {
            m.set(key(k), game.resman.get(&key(k)).unwrap().into_owned());
        }
        let mut walk = Walk::new(&game);
        walk.module(&m);
        walk.settle();
        let reached = |n: &str| walk.from.get(&key(n)).map(|f| f.to_string());
        // The area's tile (not the other), its walkmesh, texture and map,
        // the tileset's environment map.
        for n in [
            "mgt01.set",
            "mgt01_a01.mdl",
            "mgt01_a01.wok",
            "mg_tile.dds",
            "mi_mgt01_a01.tga",
            "mg_env.tga",
        ] {
            assert_eq!(reached(n).as_deref(), Some("a1.are"), "{n}");
        }
        assert_eq!(reached("mgt01_a02.mdl"), None);
        // The placeable's model, texture and sound.
        for n in ["plc_mg01.mdl", "mg_wood.tga", "mg_open.wav", "placeables.2da"] {
            assert_eq!(reached(n).as_deref(), Some("a1.git"), "{n}");
        }
        // The script's string: a hak blueprint, its model and its script.
        for n in ["mg_named.utp", "mg_box.mdl", "mg_hak_used.ncs"] {
            assert_eq!(reached(n).as_deref(), Some("spawn.nss"), "{n}");
        }
        assert_eq!(reached("mg_unused.mdl"), None);

        // By hak.
        let index = |c: &dyn mg_resman::Container| -> Vec<(ResKey, u64)> {
            let mut keys: Vec<ResKey> = c.keys().collect();
            keys.sort();
            keys.into_iter().map(|k| (k, 10)).collect()
        };
        let layer = |label: &str| {
            let l = game.resman.layers().iter().find(|l| l.label == label).unwrap();
            index(&*l.container)
        };
        let none = |_: &ResKey| false;
        let mut u =
            attribute(&game.resman, &walk.from, "mg_content", &layer("hak:mg_content"), &none);
        u.path = Some("mg_content.hak".into());
        let names = |v: Vec<String>| v.join(" ");
        assert_eq!(
            names(u.unused.iter().map(|k| k.0.to_string()).collect()),
            "mg_unused.mdl mgt01_a02.mdl"
        );
        assert_eq!(
            names(u.shadowed.iter().map(|k| format!("{} {}", k.0, k.2)).collect()),
            "mg_wood.tga hak:mg_top"
        );
        assert_eq!(names(u.not_judged.iter().map(|k| k.0.to_string()).collect()), "mg_rules.2da");
        assert_eq!((u.used.len(), u.total(), u.used_bytes(), u.bytes()), (10, 14, 100, 140));
        assert!(
            line(&u).contains("10 of 14 used (100 B of 140 B); 2 unused, 1 shadowed, 1 not judged"),
            "{}",
            line(&u)
        );
        let top = attribute(&game.resman, &walk.from, "mg_top", &layer("hak:mg_top"), &none);
        assert_eq!(top.used.len(), 1);
        // A replacement of the game's own, reached by nothing: not judged.
        let base = |k: &ResKey| *k == key("mg_unused.mdl");
        let u = attribute(&game.resman, &walk.from, "mg_content", &layer("hak:mg_content"), &base);
        assert_eq!(u.unused.len(), 1);
        assert_eq!(armor_part("pmh0_chest001"), Some(('m', 'h', "0", "chest001")));
        assert_eq!(armor_part("plc_a01"), None);
    }
}
