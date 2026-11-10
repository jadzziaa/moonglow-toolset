//! What a module uses of its haks (`mg haks --usage`, and Module
//! Properties' Check for Conflicts): which of the resources in a module's
//! haks the module reaches, hak by hak, and which none of it does: for
//! cutting a release (or a download) to what is used. The feature `usage`
//! (it reads modules, with mg-module): off for those that only draw
//! previews.
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
//!   it (the call the string is passed to is not read), its compiled
//!   script's as its source's (a hak's script without its source);
//! - the rows of tables a compiled script names by a number, where the
//!   number is a constant ([`mg_script::ncs`]): a visual effect's files
//!   (visualeffects.2da and the progfx.2da rows it names), a spell's
//!   (spells.2da), an item property's spell (iprp_spells.2da) or weapon
//!   glow (iprp_visualfx.2da, on every item class), an appearance's look
//!   (appearance.2da); a number worked out as the script runs is counted
//!   ([`ByNumber`]), not followed;
//! - what an area is drawn with: its tileset (its `.set` and palette) and,
//!   of the tiles, those the area places: each one's model, walkmesh
//!   (`.wok`), map image and replacement texture (replacetexture.2da, where
//!   its model has a `replace_tex`); the tileset's grass texture and
//!   environment map, and its edge tiles' models (`<tileset>_edge.2da`,
//!   every row's: the tile border past the area's edge);
//!   its skybox's models (skyboxes.2da), its loading screen
//!   (loadscreens.2da), its ambient sounds (ambientsound.2da) and music
//!   (ambientmusic.2da and its stingers);
//! - what an object looks like, as the area view draws it
//!   ([`crate`]): a placeable's or door's model and its sounds
//!   (placeableobjsnds.2da), a creature's body (heads, body parts, wings,
//!   tails, the armor it wears), its portrait and its sound set (the
//!   `.ssf` and its sounds), its footsteps (footstepsounds.2da) and combat
//!   sounds (appearancesndset.2da: its falls, its natural weapons' hits
//!   and misses, weaponsounds.2da's sounds of each weapon striking its
//!   skin), an item's models (with every race's, gender's and phenotype's
//!   armor part of the numbers it has, as anyone may wear it), their
//!   inventory icons (an armor's its parts' layers, `ipm_chest001`, a
//!   man's and a woman's), a weapon's hits (weaponsounds.2da) and the
//!   model of its visual effect property (`<ItemClass>_fx<suffix>`,
//!   iprp_visualfx.2da);
//! - a model's own files ([`crate::files`]): its walkmeshes, textures
//!   (as the game picks a DDS or TGA, and a PLT), materials (`.mtr`) and
//!   TXIs and what they name, its emitters' models and its supermodels;
//! - the scripts the game runs from its tables as the module sees them
//!   ([`mg_module::verify::table_scripts`]: spells' impact scripts, areas
//!   of effect's, states', traps', diseases' and poisons'), reached from
//!   the table;
//! - the 2DAs the game reads by itself (a hak's replacement of one the
//!   game has), each reached from itself, and the tables a 2DA's cells
//!   name (classes.2da a class's feat table);
//! - what the game shows or plays of a table's rows by itself, with no
//!   script naming the row ([`SHOWN_BY_THE_GAME`]): a feat's, skill's,
//!   class's, domain's, race's and effect's icon, a spell's icon, visuals
//!   and sounds, a cast-spell property's icon, of every row of the table
//!   once it is reached;
//! - what players choose at character creation: every head of the player
//!   races and the body parts the creation screen offers them (part 1 of
//!   each, part 2 of those a tattoo covers), for the phenotypes it offers,
//!   their portraits and the players' voice sets.
//!
//! Not followed, and so not judged by it: a row a script works out as it
//! runs, what the game reads by itself
//! other than its 2DAs (a 2DA of the haks' own nothing names, GUI, fonts,
//! shaders a material doesn't name, palettes) and a hak's replacements of
//! the game's own resources other than 2DAs, which the game's own content
//! may use: these are "not judged", never "unused".

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mg_2da::TwoDa;
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_resman::{ErfContainer, GameInstall, ResKey, ResMan};
use mg_rules::GameData;

use crate::files::Gatherer;

/// One hak's resources, by what the module makes of them.
#[derive(Debug, Default)]
pub struct HakUsage {
    pub name: String,
    pub path: Option<PathBuf>,
    /// Each with its size and the module resource whose walk reached it
    /// first.
    pub used: Vec<(ResKey, u64, ResKey)>,
    pub unused: Vec<(ResKey, u64)>,
    /// Never loaded from this hak: another layer (a hak higher in the
    /// list, the module) has it, which is named.
    pub shadowed: Vec<(ResKey, u64, String)>,
    /// Of a kind the walk doesn't judge, or a replacement of the game's own.
    pub not_judged: Vec<(ResKey, u64)>,
}

impl HakUsage {
    pub fn total(&self) -> usize {
        self.used.len() + self.unused.len() + self.shadowed.len() + self.not_judged.len()
    }

    pub fn bytes(&self) -> u64 {
        let sum = |v: &mut dyn Iterator<Item = u64>| v.sum::<u64>();
        sum(&mut self.used.iter().map(|u| u.1))
            + sum(&mut self.unused.iter().map(|u| u.1))
            + sum(&mut self.shadowed.iter().map(|u| u.1))
            + sum(&mut self.not_judged.iter().map(|u| u.1))
    }

    pub fn used_bytes(&self) -> u64 {
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

/// What a walk can follow further when it reaches it (a 2DA: the tables
/// its cells name, as classes.2da names each class's feat table).
fn followed(t: ResType) -> bool {
    t.is_gff() || matches!(t, ResType::NSS | ResType::NCS | ResType::SSF | ResType::TWODA)
}

/// The engine functions that take a row of a table by a number, which
/// argument it is, and the table: what a compiled script reaches by one.
const BY_NUMBER: [(&str, usize, Row); 10] = [
    ("EffectVisualEffect", 0, Row::VisualEffect),
    ("EffectBeam", 0, Row::VisualEffect),
    ("EffectAreaOfEffect", 0, Row::AreaOfEffect),
    ("ActionCastSpellAtObject", 0, Row::Spell),
    ("ActionCastSpellAtLocation", 0, Row::Spell),
    ("ActionCastFakeSpellAtObject", 0, Row::Spell),
    ("ActionCastFakeSpellAtLocation", 0, Row::Spell),
    ("ItemPropertyVisualEffect", 0, Row::ItemVisualEffect),
    ("ItemPropertyCastSpell", 0, Row::ItemSpell),
    ("SetCreatureAppearanceType", 1, Row::Appearance),
];

/// The tables whose rows the game shows or plays by itself, as a player
/// meets them (a feat on the character sheet, a spell cast from the
/// spellbook), and the columns that name a file of the row's: an icon, a
/// model, a sound. No script names such a row, so every row's are
/// reached once the table is. (Not appearance.2da or placeables.2da: a
/// row of those is an object's, reached by the object.)
const SHOWN_BY_THE_GAME: [(&str, &[&str]); 9] = [
    ("feat", &["ICON"]),
    ("masterfeats", &["ICON"]),
    ("skills", &["Icon"]),
    ("classes", &["Icon"]),
    ("domains", &["Icon"]),
    ("racialtypes", &["Icon"]),
    (
        "spells",
        &[
            "IconResRef",
            "ConjHeadVisual",
            "ConjHandVisual",
            "ConjGrndVisual",
            "ConjSoundVFX",
            "ConjSoundMale",
            "ConjSoundFemale",
            "CastHeadVisual",
            "CastHandVisual",
            "CastGrndVisual",
            "CastSound",
            "ProjModel",
            "ProjSound",
        ],
    ),
    ("iprp_spells", &["Icon"]),
    ("effecticons", &["Icon"]),
];

/// A table a script names a row of by a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Row {
    /// visualeffects.2da (and the progfx.2da rows it names).
    VisualEffect,
    /// vfx_persistent.2da (and the visualeffects.2da row it names).
    AreaOfEffect,
    /// spells.2da.
    Spell,
    /// iprp_visualfx.2da: a weapon's glow, on any weapon.
    ItemVisualEffect,
    /// iprp_spells.2da (and the spells.2da row it names).
    ItemSpell,
    /// appearance.2da: the creature's look.
    Appearance,
}

/// What the walk made of the rows scripts name by a number.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ByNumber {
    /// Calls whose row was a constant, followed.
    pub followed: usize,
    /// Calls whose row the script works out as it runs (a variable, a
    /// sum, another function's result): each script and function, not
    /// followed.
    pub not_followed: Vec<(ResKey, String)>,
}

/// What the walk made of a module and its haks.
#[derive(Debug, Default)]
pub struct Walked {
    pub haks: Vec<HakUsage>,
    /// How many resources it reached.
    pub reached: usize,
    pub by_number: ByNumber,
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
    /// The names of the 2DAs in the load order, once (a table's cells
    /// that are one reach it).
    tables: Option<std::collections::HashSet<ResRef>>,
    /// The language spec (`nwscript.nss`), once: a compiled script's
    /// functions by number.
    spec: Option<Option<Arc<mg_script::spec::Spec>>>,
    /// The rows followed, once each.
    rows: std::collections::HashSet<(Row, i64)>,
    /// The names scripts and tables' cells gave, each tried once.
    names: std::collections::HashSet<ResRef>,
    /// What the previews of the objects walked have read of their parts'
    /// models.
    remembered: crate::Remembered,
    pub(crate) by_number: ByNumber,
}

fn text(s: &Struct, label: &str) -> Option<String> {
    match s.get(label)? {
        Value::String(b) | Value::ResRef(b) if !b.is_empty() => {
            Some(String::from_utf8_lossy(b).to_ascii_lowercase())
        }
        _ => None,
    }
}

/// The body parts the creation screen offers a player (its Tattoo
/// choices, the same for every race, gender and phenotype; read from the
/// client's `CCharacterAppearancePanel::PostAttachmentInitialize`, 89.8193.37,
/// and borne out by the game's pregenerated characters): the parts a
/// tattoo covers, part 1 or 2 (the chest by the pattern's first bit, the
/// biceps by its second, the shins by its third, the forearms by its
/// fourth); the belt, shoulders and robe none; every other part 1.
/// parts_*.2da is not read.
const CHARGEN_BODY_PARTS: [(&str, &[i64]); 15] = [
    ("footr", &[1]),
    ("footl", &[1]),
    ("shinr", &[1, 2]),
    ("shinl", &[1, 2]),
    ("legl", &[1]),
    ("legr", &[1]),
    ("pelvis", &[1]),
    ("chest", &[1, 2]),
    ("neck", &[1]),
    ("forer", &[1, 2]),
    ("forel", &[1, 2]),
    ("bicepr", &[1, 2]),
    ("bicepl", &[1, 2]),
    ("handr", &[1]),
    ("handl", &[1]),
];

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
            tables: None,
            spec: None,
            rows: Default::default(),
            names: Default::default(),
            remembered: Default::default(),
            by_number: ByNumber::default(),
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
        if key.restype == ResType::TWODA {
            self.table_names(key.resref);
            self.shown_by_the_game(key.resref);
            return;
        }
        let Ok(data) = self.rm().get(&key) else { return };
        let data = data.into_owned();
        if key.restype == ResType::NSS {
            for r in mg_module::refs::script_includes(key, &data) {
                self.reach(ResKey::new(r.target, ResType::NSS));
            }
            self.strings(&data);
            return;
        }
        if key.restype == ResType::NCS {
            self.compiled(key, &data);
            return;
        }
        if key.restype == ResType::SSF {
            if let Ok(ssf) = mg_ssf::Ssf::read(&data) {
                for e in &ssf.entries {
                    self.reach_named(&String::from_utf8_lossy(e.sound_name()), ResType::WAV);
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
            self.named(r);
        }
    }

    /// A name a script or a table's cell gives: whatever of the kinds a
    /// script may name it is (a script, blueprint, conversation, sound,
    /// 2DA, model or texture).
    fn named(&mut self, r: ResRef) {
        // (Once: a name is tried against 15 types, a model and a texture,
        // each a look through every layer, and scripts give the same
        // names again and again, in their source and compiled. What a
        // name reaches the first time is reached.)
        if !self.names.insert(r) {
            return;
        }
        let name = r.to_string().to_ascii_lowercase();
        for ty in NAMED_IN_SCRIPTS {
            self.reach(ResKey::new(r, ty));
        }
        if self.rm().contains(&ResKey::new(r, ResType::MDL)) {
            self.gather.add_model(&name);
        }
        if self.rm().texture_type(r).is_some() || self.rm().contains(&ResKey::new(r, ResType::PLT))
        {
            self.gather.add_texture(&name);
        }
    }

    /// A compiled script: the strings it pushes, as a source's literals,
    /// and the rows of tables it names by a number where the number is a
    /// constant (`mg_script::ncs`: a script with no source, a hak's, is
    /// read the same way); a row it works out as it runs is counted, not
    /// followed.
    fn compiled(&mut self, key: ResKey, data: &[u8]) {
        let Ok(code) = mg_script::ncs::decode(data) else { return };
        let names: Vec<ResRef> = mg_script::ncs::strings(&code)
            .filter_map(|s| std::str::from_utf8(s).ok().and_then(|s| ResRef::from_str(s).ok()))
            .filter(|r| !r.is_empty())
            .collect();
        for r in names {
            self.named(r);
        }
        let spec = self.spec.get_or_insert_with(|| {
            let src = self.game.resman.get(&ResKey::parse("nwscript", ResType::NSS)?).ok()?;
            Some(Arc::new(mg_script::spec::Spec::parse(&src)))
        });
        let Some(spec) = spec.clone() else { return };
        for call in mg_script::ncs::calls(&code, &spec) {
            let name = &spec.functions[usize::from(call.routine)].name;
            let Some(&(f, arg, row)) = BY_NUMBER.iter().find(|(f, ..)| f == name) else {
                continue;
            };
            match call.args.get(arg) {
                Some(Some(mg_script::ncs::Const::Int(n))) => {
                    self.by_number.followed += 1;
                    self.row(row, i64::from(*n));
                }
                // (Fewer arguments than the function has: an old script's,
                // compiled before the parameter was added.)
                None => {}
                Some(_) => self.by_number.not_followed.push((key, f.to_string())),
            }
        }
    }

    /// A row of a table a script names by a number, followed once.
    fn row(&mut self, row: Row, n: i64) {
        if n < 0 || !self.rows.insert((row, n)) {
            return;
        }
        match row {
            Row::VisualEffect => {
                let Some(t) = self.table("visualeffects") else { return };
                self.row_files(&t, n);
                // Its programmed effects (progfx.2da), by number.
                let progfx: Vec<i64> = (t.columns().iter())
                    .filter(|c| c.to_ascii_lowercase().starts_with("progfx"))
                    .filter_map(|c| cell(&t, n, c)?.parse().ok())
                    .collect();
                if !progfx.is_empty()
                    && let Some(p) = self.table("progfx")
                {
                    for r in progfx {
                        self.row_files(&p, r);
                    }
                }
            }
            Row::AreaOfEffect => {
                let Some(t) = self.table("vfx_persistent") else { return };
                // (Its scripts, models and sounds; and its one large
                // effect, a row of visualeffects.2da.)
                self.row_files(&t, n);
                if let Some(fx) = cell(&t, n, "DurationVFX").and_then(|s| s.parse().ok()) {
                    self.row(Row::VisualEffect, fx);
                }
            }
            Row::Spell => {
                let Some(t) = self.table("spells") else { return };
                self.row_files(&t, n);
            }
            Row::ItemSpell => {
                let Some(t) = self.table("iprp_spells") else { return };
                self.row_files(&t, n);
                if let Some(spell) = cell(&t, n, "SpellIndex").and_then(|s| s.parse().ok()) {
                    self.row(Row::Spell, spell);
                }
            }
            Row::ItemVisualEffect => {
                let Some(fx) = self.table("iprp_visualfx") else { return };
                let Some(suffix) = cell(&fx, n, "ModelSuffix") else { return };
                // (On whatever weapon it is put: every item class's.)
                let Some(items) = self.table("baseitems") else { return };
                let classes: std::collections::BTreeSet<String> = (0..items.len() as i64)
                    .filter_map(|r| cell(&items, r, "ItemClass"))
                    .map(|c| c.to_ascii_lowercase())
                    .collect();
                for class in classes {
                    let name = format!("{class}_fx{suffix}");
                    if ResKey::parse(&name, ResType::MDL).is_some_and(|k| self.rm().contains(&k)) {
                        self.gather.add_model(&name);
                    }
                }
            }
            Row::Appearance => {
                let Ok(n) = u16::try_from(n) else { return };
                self.creature(&crate::CreatureLook::new(n).to_utc());
            }
        }
    }

    /// What a row of a table names, cell by cell (but its label): a
    /// model, texture, sound, script or table, as a script's string would.
    fn row_files(&mut self, t: &TwoDa, row: i64) {
        let Ok(r) = usize::try_from(row) else { return };
        for c in 0..t.columns().len() {
            if t.columns()[c].eq_ignore_ascii_case("Label") {
                continue;
            }
            let Some(v) = t.cell(r, c).map(str::trim) else { continue };
            if v.is_empty() || v == "****" || v.parse::<f64>().is_ok() {
                continue;
            }
            if let Ok(name) = ResRef::from_str(v) {
                self.named(name);
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

    /// What an object of `kind` is drawn with (an item as it lies).
    fn preview(&mut self, kind: mg_rules::ObjectKind, s: &Struct) {
        let rm = self.rm();
        let items = |r: ResRef| -> Option<Gff> {
            Gff::read(&rm.get(&ResKey::new(r, ResType::UTI)).ok()?).ok()
        };
        let shown = crate::Shown::Placed;
        let kept = Some(&self.remembered);
        if let Some(Ok(p)) = crate::of_object_with(self.game, kind, s, &items, shown, kept) {
            self.gather.preview(&p);
        }
    }

    fn area(&mut self, are: &Struct) {
        let mut set = None;
        if let Some(name) = text(are, "Tileset")
            && let Ok(r) = ResRef::from_str(&name)
        {
            self.reach_named(&format!("{name}palstd"), ResType::ITP);
            set = self.tileset(r);
            // (The tables its tile border's models and its tiles'
            // replacement textures are read from.)
            self.table(&mg_rules::rows::edge_table(&name));
            self.table("replacetexture");
        }
        // (The tables its skybox and loading screen are read from.)
        self.table("skyboxes");
        if are.integer("LoadScreenID").is_some_and(|r| r > 0) {
            self.table("loadscreens");
        }
        // Its tiles' models, walkmeshes, minimap pictures and replacement
        // textures, the tileset's maps and edge tiles, the skybox and the
        // loading screen, as Export Files gathers them.
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
                        self.preview(K::Item, s);
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
        self.preview(mg_rules::ObjectKind::Creature, s);
        self.portrait_id(s);
        if let Some(id) = s.integer("SoundSetFile").filter(|&i| i < 0xFFFF)
            && let Some(t) = self.table("soundset")
            && let Some(ssf) = cell(&t, id, "RESREF")
        {
            self.reach_named(&ssf, ResType::SSF);
        }
        // Its footsteps (footstepsounds.2da) and its combat sounds
        // (appearancesndset.2da: falling, its natural weapons' hits and
        // misses, its skin's when hit), by its appearance.2da row.
        let row = s.integer("Appearance_Type").unwrap_or(-1);
        let Some(t) = self.table("appearance") else { return };
        if let Some(steps) = cell(&t, row, "FOOTSTEPTYPE").and_then(|r| r.parse().ok())
            && let Some(f) = self.table("footstepsounds")
        {
            self.row_sounds(&f, steps);
        }
        let Some(set) = cell(&t, row, "SOUNDAPPTYPE").and_then(|r| r.parse::<i64>().ok()) else {
            return;
        };
        let Some(sounds) = self.table("appearancesndset") else { return };
        for c in ["Looping", "FallFwd", "FallBck"] {
            if let Some(w) = cell(&sounds, set, c) {
                self.reach_named(&w, ResType::WAV);
            }
        }
        let Some(weapons) = self.table("weaponsounds") else { return };
        let natural = ["WeapTypeL", "WeapTypeR", "WeapTypeS", "WeapTypeClsLw", "WeapTypeClsH"];
        for c in natural.into_iter().chain(["WeapTypeRch", "MissIndex"]) {
            if let Some(r) = cell(&sounds, set, c).and_then(|r| r.parse().ok()) {
                self.row_sounds(&weapons, r);
            }
        }
        // (Struck: each weapon's sound against its skin, `Leather0`…)
        if let Some(skin) = cell(&sounds, set, "ArmorType") {
            for n in 0..2 {
                let column = format!("{skin}{n}");
                for r in 0..weapons.len() {
                    if let Some(w) = cell(&weapons, r as i64, &column) {
                        self.reach_named(&w, ResType::WAV);
                    }
                }
            }
        }
    }

    /// Every sound a row of a table of sounds names (each column but its
    /// label).
    fn row_sounds(&mut self, t: &TwoDa, row: i64) {
        let Ok(r) = usize::try_from(row) else { return };
        for c in 0..t.columns().len() {
            if t.columns()[c].eq_ignore_ascii_case("Label") {
                continue;
            }
            if let Some(w) = t.cell(r, c).map(str::trim).filter(|w| !w.is_empty() && *w != "****") {
                self.reach_named(w, ResType::WAV);
            }
        }
    }

    /// The tables a 2DA's cells name (classes.2da a class's feat, skill and
    /// spell tables; itempropdef.2da its subtypes' tables): each a 2DA of
    /// the load order, reached.
    fn table_names(&mut self, table: ResRef) {
        let Ok(t) = self.game.table(&table.to_string()) else { return };
        let tables = self.tables.get_or_insert_with(|| {
            let rm = &self.game.resman;
            (rm.layers().iter())
                .flat_map(|l| l.container.keys())
                .filter(|k| k.restype == ResType::TWODA)
                .map(|k| k.resref)
                .collect()
        });
        let mut named = Vec::new();
        for r in 0..t.len() {
            for c in 0..t.columns().len() {
                let Some(v) = t.cell(r, c) else { continue };
                if v.len() > 16 || v.starts_with(|c: char| c.is_ascii_digit() || c == '*') {
                    continue;
                }
                if let Ok(name) = ResRef::from_str(&v.to_ascii_lowercase())
                    && name != table
                    && tables.contains(&name)
                {
                    named.push(name);
                }
            }
        }
        for name in named {
            self.reach(ResKey::new(name, ResType::TWODA));
        }
    }

    /// What the game shows or plays of `table`'s rows by itself
    /// ([`SHOWN_BY_THE_GAME`]): every row's icon, models and sounds.
    fn shown_by_the_game(&mut self, table: ResRef) {
        let name = table.to_string();
        let Some((_, columns)) =
            SHOWN_BY_THE_GAME.iter().find(|(t, _)| t.eq_ignore_ascii_case(&name))
        else {
            return;
        };
        let Ok(t) = self.game.table(&name) else { return };
        for column in columns.iter().filter_map(|c| t.column(c)) {
            for row in 0..t.len() {
                let Some(v) = t.cell(row, column).map(str::trim) else { continue };
                if v.is_empty() || v == "****" {
                    continue;
                }
                if let Ok(name) = ResRef::from_str(v) {
                    self.named(name);
                }
            }
        }
    }

    /// The phenotypes the creation screen's Body offers: the bits of
    /// ruleset.2da's `CHARGEN_ENABLED_PHENOTYPES` (5, phenotypes 0 and 2,
    /// where the table has none), as the client's
    /// `CCharacterAppearancePanel::HandleBodyNextButton` reads them.
    fn chargen_phenotypes(&mut self) -> Vec<i64> {
        let bits = self
            .table("ruleset")
            .and_then(|t| {
                (0..t.len()).find_map(|r| {
                    (cell(&t, r as i64, "Label").as_deref() == Some("chargen_enabled_phenotypes"))
                        .then(|| cell(&t, r as i64, "Value"))
                        .flatten()
                })
            })
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(5);
        (0..32).filter(|ph| bits >> ph & 1 == 1).collect()
    }

    /// What players choose at character creation, as their game offers it,
    /// reached from the table that offers it: for the player races
    /// (racialtypes.2da `PlayerRace`), each gender and the phenotypes the
    /// creation screen offers ([`Walk::chargen_phenotypes`]), every head
    /// and the body parts of [`CHARGEN_BODY_PARTS`]; the portraits of those
    /// races (portraits.2da) and the players' sound sets (soundset.2da
    /// `TYPE` 0).
    pub(crate) fn players_choose(&mut self) {
        let Some(races) = self.table("racialtypes") else { return };
        let player = |r: usize| cell(&races, r as i64, "PlayerRace").as_deref() == Some("1");
        let rows: Vec<usize> = (0..races.len()).filter(|r| player(*r)).collect();
        self.root = ResKey::new(ResRef::from_str("racialtypes").expect("a name"), ResType::TWODA);
        let mut letters = Vec::new();
        if let Some(looks) = self.table("appearance") {
            for r in &rows {
                if let Some(look) =
                    cell(&races, *r as i64, "Appearance").and_then(|a| a.parse().ok())
                    && let Some(letter) = cell(&looks, look, "RACE").filter(|l| l.len() == 1)
                    && !letters.contains(&letter)
                {
                    letters.push(letter);
                }
            }
        }
        let phenotypes = self.chargen_phenotypes();
        // (The body parts the creation screen offers, by
        // `mg_rules::models::BODY_PARTS`' names.)
        let mut parts: Vec<(&str, i64)> = Vec::new();
        for (part, offered) in CHARGEN_BODY_PARTS {
            parts.extend(offered.iter().map(|n| (part, *n)));
        }
        for letter in &letters {
            for g in ['m', 'f'] {
                for ph in &phenotypes {
                    let skeleton =
                        mg_rules::models::skeleton(&g.to_string(), &letter.to_string(), *ph);
                    // (A creature's head is a byte.)
                    let heads = (1..=255).map(|n| ("head", n));
                    for (part, n) in heads.chain(parts.iter().copied()) {
                        let name = mg_rules::models::body_part_model(&skeleton, part, n);
                        if ResKey::parse(&name, ResType::MDL)
                            .is_some_and(|k| self.rm().contains(&k))
                        {
                            self.body_part(&name);
                        }
                    }
                }
            }
        }
        self.settle();
        if let Some(portraits) = self.table("portraits") {
            self.root = ResKey::new(ResRef::from_str("portraits").expect("a name"), ResType::TWODA);
            for r in 0..portraits.len() {
                let race = cell(&portraits, r as i64, "Race").and_then(|v| v.parse::<usize>().ok());
                if race.is_some_and(|race| rows.contains(&race))
                    && let Some(base) = cell(&portraits, r as i64, "BaseResRef")
                {
                    self.portrait(&format!("po_{base}"));
                }
            }
            self.settle();
        }
        if let Some(sets) = self.table("soundset") {
            self.root = ResKey::new(ResRef::from_str("soundset").expect("a name"), ResType::TWODA);
            for r in 0..sets.len() {
                if cell(&sets, r as i64, "TYPE").as_deref() == Some("0")
                    && let Some(ssf) = cell(&sets, r as i64, "RESREF")
                {
                    self.reach_named(&ssf, ResType::SSF);
                }
            }
            self.drain();
        }
    }

    /// The 2DAs the game reads by itself (`tables`: those it has of its
    /// own, which a hak replaces), each reached from itself, and what
    /// their cells name.
    pub(crate) fn game_reads(&mut self, tables: impl IntoIterator<Item = ResKey>) {
        for key in tables {
            self.root = key;
            self.reach(key);
            self.drain();
        }
    }

    /// Follows what is queued, to the end.
    fn drain(&mut self) {
        while let Some(next) = self.queue.pop() {
            self.follow(next);
            self.settle();
        }
    }

    fn placeable(&mut self, s: &Struct) {
        self.preview(mg_rules::ObjectKind::Placeable, s);
        self.portrait_id(s);
        let row = s.integer("Appearance").unwrap_or(-1);
        if let Some(t) = self.table("placeables")
            && let Some(sounds) = cell(&t, row, "SoundAppType")
        {
            self.object_sounds(&sounds);
        }
    }

    fn door(&mut self, s: &Struct) {
        self.preview(mg_rules::ObjectKind::Door, s);
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

    /// An item's models (`placed`: it lies in an area, and what it lies as
    /// is gathered already) and their icons; armor's parts and a cloak as
    /// anyone may wear them.
    fn item(&mut self, s: &Struct, placed: bool) {
        let base = s.integer("BaseItem").unwrap_or(-1);
        let mut armor = false;
        if let Some(t) = self.table("baseitems") {
            if let Some(icon) = cell(&t, base, "DefaultIcon") {
                self.gather.add_texture(&icon);
            }
            armor = cell(&t, base, "ModelType").as_deref() == Some("3");
            // Its icon's layers, a man's and a woman's (an armor's are its
            // parts' `ipm_chest001`…, PLTs), as the inventory draws it.
            for female in [false, true] {
                for icon in mg_rules::items::icon_names(self.game, s, female) {
                    self.gather.add_texture(&icon);
                }
            }
            // A weapon's sounds as it hits (weaponsounds.2da).
            if let Some(r) = cell(&t, base, "WeaponMatType").and_then(|r| r.parse().ok())
                && let Some(w) = self.table("weaponsounds")
            {
                self.row_sounds(&w, r);
            }
            // A visual effect property's model, `<ItemClass>_fx<suffix>`
            // (iprp_visualfx.2da's ModelSuffix).
            if let Some(class) = cell(&t, base, "ItemClass") {
                self.visual_effects(s, &class);
            }
        }
        if armor {
            // (Its preview is a body wearing it: the body is no part of it.)
            let parts = mg_rules::models::BODY_PARTS.iter().map(|(p, _, a)| (*p, *a));
            for (mdl, field) in parts.chain([mg_rules::models::ROBE]) {
                let n = mg_rules::items::part_number(s, field).unwrap_or(0);
                if n > 0 {
                    self.worn(&format!("{mdl}{n:03}"));
                }
            }
            return;
        }
        let Ok(p) = crate::item(self.game, s) else { return };
        let p = crate::replaced(p, s);
        // (One that lies as something else, a cloak folded, is drawn as
        // this once it is picked up and worn: its cloakmodel.2da texture.)
        let lies_as_another = || {
            (std::iter::once(&p.base).chain(&p.parts))
                .any(|part| mg_rules::models::part_name(&part.model).is_some())
        };
        if !placed || lies_as_another() {
            self.gather.preview(&p);
        }
        for part in std::iter::once(&p.base).chain(&p.parts) {
            let name = part.model.to_ascii_lowercase();
            match mg_rules::models::part_name(&name) {
                Some(worn) => self.worn(worn.part),
                None if self.parts.insert(name.clone()) => {
                    self.gather.add_texture(&format!("i{name}"))
                }
                None => {}
            }
        }
    }

    /// The models of an item's visual effect properties (itempropdef.2da's
    /// row with the subtypes of iprp_visualfx.2da): `<class>_fx<suffix>`.
    fn visual_effects(&mut self, s: &Struct, class: &str) {
        let Some(defs) = self.table("itempropdef") else { return };
        let mut suffixes = Vec::new();
        for p in s.list("PropertiesList").unwrap_or_default() {
            let Some(row) = p.integer("PropertyName") else { continue };
            let table = cell(&defs, row, "SubTypeResRef");
            if table.as_deref() != Some("iprp_visualfx") {
                continue;
            }
            suffixes.push(p.integer("Subtype").unwrap_or(-1));
        }
        if suffixes.is_empty() {
            return;
        }
        let Some(fx) = self.table("iprp_visualfx") else { return };
        for row in suffixes {
            if let Some(suffix) = cell(&fx, row, "ModelSuffix") {
                let name = format!("{class}_fx{suffix}");
                if ResKey::parse(&name, ResType::MDL).is_some_and(|k| self.rm().contains(&k)) {
                    self.gather.add_model(&name);
                }
            }
        }
    }

    /// A part worn (`chest001`, `cloak_001`): its model for every gender,
    /// part-based race and phenotype that has one, with the textures the
    /// game draws on it, once.
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
                        self.body_part(&variant);
                    }
                }
            }
        }
    }

    /// A body part's model (a head, an armor's part) and what the game
    /// draws on it, as a creature's preview has it: the textures its
    /// meshes name and, whatever they name, the part's own PLT (or its
    /// fallbacks': phenotype 0's, the human's), which a mesh with `bitmap
    /// NULL` takes too ([`crate::creature::plt_fallbacks`]).
    fn body_part(&mut self, model: &str) {
        self.gather.add_model(model);
        let lk = crate::Lookup { game: self.game, remembered: Some(&self.remembered) };
        for texture in crate::creature::plt_fallbacks(&lk, model).values() {
            self.gather.add_texture(mg_image::plt::split_colors(texture).0);
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

/// The game a player has, with the module's haks and the module
/// ([`Module::player_game_data`]): the user's override and development
/// folders left out (a player's game doesn't have them), the haks found
/// where the game finds them. A hak that can't be opened is an error.
fn player_game(gi: &GameInstall, m: &Module) -> Result<GameData, String> {
    let found = m.player_game_data(gi).map_err(|e| e.to_string())?;
    match found.wrong_haks {
        Some(e) => Err(format!("the module's haks: {e}")),
        None => Ok(found.game),
    }
}

/// Each of the module's haks' resources, by what the module makes of
/// them; and how many resources the walk reached in all.
pub fn hak_usage(
    gi: &GameInstall,
    m: &Module,
    report: &mg_module::haks::HakReport,
) -> Result<Walked, String> {
    let game = player_game(gi, m)?;
    let mut walk = Walk::new(&game);
    walk.module(m);
    walk.settle();
    // The game's own 2DAs, which a hak replaces: the game reads them.
    let tables = report.overrides.keys().filter(|k| k.restype == ResType::TWODA).copied();
    walk.game_reads(tables.collect::<Vec<_>>());
    walk.players_choose();
    walk.settle();
    let mut out = Vec::new();
    for (name, path) in &report.haks {
        let index = match path {
            Some(p) => ErfContainer::open(p).map_err(|e| e.to_string())?.index().to_vec(),
            None => Vec::new(),
        };
        let in_base = |k: &ResKey| report.overrides.contains_key(k);
        let mut u = attribute(&game.resman, &walk.from, name, &index, &in_base);
        u.path.clone_from(path);
        out.push(u);
    }
    Ok(Walked { haks: out, reached: walk.from.len(), by_number: walk.by_number })
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
pub fn size(bytes: u64) -> String {
    let b = bytes as f64;
    match bytes {
        0..1_000 => format!("{bytes} B"),
        1_000..1_000_000 => format!("{:.1} KB", b / 1e3),
        1_000_000..1_000_000_000 => format!("{:.1} MB", b / 1e6),
        _ => format!("{:.2} GB", b / 1e9),
    }
}

/// A hak's summary line.
pub fn line(u: &HakUsage) -> String {
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

/// The line of every hak together: what is used of all, and how many
/// haks nothing reaches.
pub fn total_line(haks: &[HakUsage]) -> String {
    let (used, all): (usize, usize) =
        haks.iter().fold((0, 0), |(a, b), u| (a + u.used.len(), b + u.total()));
    let (used_bytes, bytes) =
        haks.iter().fold((0, 0), |(a, b), u| (a + u.used_bytes(), b + u.bytes()));
    let none: Vec<&HakUsage> =
        haks.iter().filter(|u| u.path.is_some() && u.used.is_empty()).collect();
    let kept = none.iter().filter(|u| !u.not_judged.is_empty()).count();
    format!(
        "All\t{used} of {all} used ({} of {}); nothing reaches {} of {} haks ({kept} of them \
         hold what isn't judged, which the game may read itself)",
        size(used_bytes),
        size(bytes),
        none.len(),
        haks.len()
    )
}

/// What the walk doesn't follow, said with its results.
pub const NOT_FOLLOWED: &str = "not followed: a row of a table a script works out as it runs (a \
    visual effect, spell or appearance in a variable), and what the game reads itself other than \
    its 2DAs (\"not judged\")";

/// The line of what scripts name by a number: how many calls were
/// followed, and how many weren't, in how many scripts.
pub fn by_number_line(b: &ByNumber) -> String {
    let scripts: std::collections::BTreeSet<&ResKey> =
        b.not_followed.iter().map(|(k, _)| k).collect();
    format!(
        "scripts' numbers: {} calls naming a visual effect, spell or appearance followed; {} \
         working it out as the script runs not followed (in {} scripts)",
        b.followed,
        b.not_followed.len(),
        scripts.len()
    )
}

#[cfg(test)]
mod tests {
    use mg_resman::{LayerClass, MemContainer, priority};

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
    }

    /// A walk of a module of `resources` (each in the module's layer)
    /// over a hak `content` and the game's `stock`: the game, and what
    /// the walk reached with what it was reached from.
    fn walked(
        resources: Vec<(&str, Vec<u8>)>,
        content: MemContainer,
        stock: MemContainer,
    ) -> (GameData, HashMap<ResKey, ResKey>) {
        let mut module = MemContainer::new();
        let mut m = Module::new();
        for (name, data) in resources {
            module.insert(key(name), data.clone());
            m.set(key(name), data);
        }
        let mut rm = ResMan::new();
        rm.add(priority::MODULE, "module", LayerClass::Erf, module);
        rm.add(priority::HAK_USER, "hak:mg_content", LayerClass::Erf, content);
        rm.add(priority::KEY, "nwn_base.key", LayerClass::Key, stock);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let from = {
            let mut walk = Walk::new(&game);
            walk.module(&m);
            walk.settle();
            walk.players_choose();
            walk.settle();
            walk.from
        };
        (game, from)
    }

    /// What the area view draws past an area's edge and on a tile whose
    /// texture is replaced is used: the models of the tileset's edge
    /// table (named nowhere else) with their textures, and the
    /// replacetexture.2da texture of a tile whose model has a
    /// `replace_tex`, by the tile's `Tile_ReplaceTex` (row 0 without the
    /// field); not that of a row no tile takes. They were listed unused,
    /// and a release cut by the report lost them (the review's M4).
    #[test]
    fn an_area_s_tile_border_and_replacement_textures_are_used() {
        let table = |text: &str| format!("2DA V2.0\n\n{text}").into_bytes();
        let mut stock = MemContainer::new();
        let set = "[GENERAL]\nName=mgt01\n[TILES]\nCount=2\n[TILE0]\nModel=mgt01_a01\n\
                   [TILE1]\nModel=mgt01_a02\n";
        stock.insert(key("mgt01.set"), set.as_bytes());
        let mut content = MemContainer::new();
        content.insert(
            key("mgt01_edge.2da"),
            table(" Corner1 Edge Corner2 Model Height\n0 Grass **** Grass MGT01_Z01 0\n1 Grass **** **** **** 0\n"),
        );
        content.insert(key("mgt01_z01.mdl"), model("mgt01_z01", "mg_edge"));
        content.insert(key("mg_edge.tga"), &b"tga"[..]);
        content.insert(
            key("replacetexture.2da"),
            table(" TEXTURENAME\n0 MG_Flag0\n1 mg_flag1\n2 mg_flag2\n"),
        );
        for n in ["mg_flag0.tga", "mg_flag1.tga", "mg_flag2.tga", "mg_plain.tga"] {
            content.insert(key(n), &b"tga"[..]);
        }
        content.insert(key("mgt01_a01.mdl"), model("mgt01_a01", "Replace_Tex"));
        content.insert(key("mgt01_a02.mdl"), model("mgt01_a02", "mg_plain"));
        let mut are = Gff::new(*b"ARE ");
        are.root.set("Tileset", Value::resref(rr("mgt01")));
        // A flag tile without the field (row 0), one of row 1 (value 2),
        // and a plain tile of row 2 (value 3), which has nothing replaced.
        let tiles = [(0, None), (0, Some(2)), (1, Some(3))].map(|(id, replace)| {
            let mut tile = Struct::new(1);
            tile.set("Tile_ID", Value::Int(id));
            if let Some(v) = replace {
                tile.set("Tile_ReplaceTex", Value::Byte(v));
            }
            tile
        });
        are.root.set("Tile_List", Value::List(tiles.to_vec()));
        let (_, from) = walked(vec![("a1.are", are.to_bytes().unwrap())], content, stock);
        for n in [
            "mgt01_edge.2da",
            "mgt01_z01.mdl",
            "mg_edge.tga",
            "replacetexture.2da",
            "mg_flag0.tga",
            "mg_flag1.tga",
            "mg_plain.tga",
        ] {
            assert_eq!(from.get(&key(n)).map(|f| f.to_string()).as_deref(), Some("a1.are"), "{n}");
        }
        assert_eq!(from.get(&key("mg_flag2.tga")), None);
    }

    /// A worn part and a creation-screen head are used with the texture
    /// the game draws on them, not only the one their meshes name: a head
    /// whose mesh has `bitmap NULL` with the PLT of its name, an elf's
    /// chest whose mesh names the human's with the elf's own PLT, and a
    /// cloak lying in an area with its cloakmodel.2da texture. They were
    /// listed unused beside their models (the review's M6).
    #[test]
    fn a_part_s_own_texture_is_used_with_its_model() {
        let table = |text: &str| format!("2DA V2.0\n\n{text}").into_bytes();
        let mut stock = MemContainer::new();
        stock.insert(
            key("appearance.2da"),
            table("   LABEL MODELTYPE RACE\n0 Human P H\n1 Elf P E\n"),
        );
        stock.insert(key("racialtypes.2da"), table(" Label Appearance PlayerRace\n0 Human 0 1\n"));
        stock.insert(
            key("baseitems.2da"),
            table(
                " label ItemClass ModelType DefaultModel\n0 sword WSwSs 2 ****\n\
                 1 armor AArmor 3 ****\n2 cloak Cloak 1 ****\n",
            ),
        );
        stock
            .insert(key("cloakmodel.2da"), table(" LABEL MODEL TEXTURE\n0 none 1 1\n1 Mine 3 7\n"));
        // (A server's own phenotype of two digits, 15.)
        let phenotypes: String = (0..16).map(|row| format!("{row} P{row} 0\n")).collect();
        stock
            .insert(key("phenotype.2da"), table(&format!(" Label DefaultPhenoType\n{phenotypes}")));
        let mut content = MemContainer::new();
        content.insert(key("pmh15_chest009.mdl"), model("pmh15_chest009", "NULL"));
        content.insert(key("pmh15_chest009.plt"), &b"plt"[..]);
        content.insert(key("pmh0_head123.mdl"), model("pmh0_head123", "NULL"));
        content.insert(key("pme0_chest009.mdl"), model("pme0_chest009", "pmh0_chest009"));
        content.insert(key("pmh0_cloak_003.mdl"), model("pmh0_cloak_003", "cloak_001"));
        let textures = [
            "pmh0_head123.plt",
            "pme0_chest009.plt",
            "pmh0_chest009.plt",
            "cloak_007.plt",
            "pmh0_head124.plt",
            "cloak_008.plt",
        ];
        for n in textures {
            content.insert(key(n), &b"plt"[..]);
        }
        // A store with the armor, and a cloak lying in an area.
        let mut armor = Struct::new(0);
        armor.set("BaseItem", Value::Int(1));
        armor.set("ArmorPart_Torso", Value::Byte(9));
        let mut page = Struct::new(0);
        page.set("ItemList", Value::List(vec![armor]));
        let mut shop = Gff::new(*b"UTM ");
        shop.root.set("StoreList", Value::List(vec![page]));
        let mut cloak = Struct::new(0);
        cloak.set("BaseItem", Value::Int(2));
        cloak.set("ModelPart1", Value::Byte(1));
        let mut git = Gff::new(*b"GIT ");
        git.root.set(mg_rules::ObjectKind::Item.list(), Value::List(vec![cloak]));
        let resources =
            vec![("shop.utm", shop.to_bytes().unwrap()), ("a1.git", git.to_bytes().unwrap())];
        let (_, from) = walked(resources, content, stock);
        let reached = |n: &str| from.get(&key(n)).map(|f| f.to_string());
        assert_eq!(reached("pmh0_head123.mdl").as_deref(), Some("racialtypes.2da"));
        assert_eq!(reached("pmh0_head123.plt").as_deref(), Some("racialtypes.2da"));
        for n in ["pme0_chest009.mdl", "pme0_chest009.plt", "pmh0_chest009.plt"] {
            assert_eq!(reached(n).as_deref(), Some("shop.utm"), "{n}");
        }
        // A part of a phenotype of two digits is a part like any other
        // (its name was read as no part's, and it got no texture: the
        // review's M10).
        for n in ["pmh15_chest009.mdl", "pmh15_chest009.plt"] {
            assert_eq!(reached(n).as_deref(), Some("shop.utm"), "{n}");
        }
        for n in ["pmh0_cloak_003.mdl", "cloak_007.plt"] {
            assert_eq!(reached(n).as_deref(), Some("a1.git"), "{n}");
        }
        // (A head and a cloak row nothing has.)
        assert_eq!(reached("pmh0_head124.plt"), None);
        assert_eq!(reached("cloak_008.plt"), None);
    }

    /// A container that counts how often each of its resources is read
    /// and asked after.
    #[derive(Debug)]
    struct Counting {
        files: MemContainer,
        asked: Arc<std::sync::Mutex<HashMap<ResKey, (usize, usize)>>>,
    }

    impl mg_resman::Container for Counting {
        fn contains(&self, key: &ResKey) -> bool {
            self.asked.lock().unwrap().entry(*key).or_default().1 += 1;
            self.files.contains(key)
        }
        fn read(&self, key: &ResKey) -> Result<std::borrow::Cow<'_, [u8]>, mg_resman::ResError> {
            self.asked.lock().unwrap().entry(*key).or_default().0 += 1;
            self.files.read(key)
        }
        fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
            self.files.keys()
        }
        fn len(&self) -> usize {
            self.files.len()
        }
    }

    /// The walk looks a thing up once, however often it meets it: a body
    /// part's model is read once for its textures' fallbacks (and once to
    /// gather it), not once more for every creature placed that has it; a
    /// name a script gives fifty times is tried once; a texture every
    /// model names has its material looked for once. And what it reaches
    /// is the same as with one of each (the review's M17).
    #[test]
    fn the_walk_looks_a_thing_up_once_however_often_it_meets_it() {
        let table = |text: &str| format!("2DA V2.0\n\n{text}").into_bytes();
        let walk_of = |copies: usize| {
            let mut stock = MemContainer::new();
            stock.insert(key("appearance.2da"), table("   LABEL MODELTYPE RACE\n0 Human P H\n"));
            stock.insert(key("gender.2da"), table(" NAME GENDER\n0 Male M\n"));
            stock.insert(key("phenotype.2da"), table(" Label DefaultPhenoType\n0 Normal 0\n"));
            stock.insert(key("parts_robe.2da"), table(" COSTMODIFIER\n0 0\n"));
            stock.insert(
                key("capart.2da"),
                table(" NAME MDLNAME NODENAME\n0 Torso chest torso_g\n1 Pelvis pelvis pelvis_g\n"),
            );
            stock.insert(
                key("baseitems.2da"),
                table(" label ItemClass ModelType\n0 sword WSwSs 2\n"),
            );
            let mut content = MemContainer::new();
            content.insert(key("pmh0.mdl"), model("pmh0", "NULL"));
            for part in ["pmh0_chest001", "pmh0_pelvis001", "pmh0_head001"] {
                content.insert(key(&format!("{part}.mdl")), model(part, part));
                content.insert(key(&format!("{part}.plt")), &b"plt"[..]);
            }
            content.insert(key("mg_named.utp"), {
                let mut g = Gff::new(*b"UTP ");
                g.root.set("Appearance", Value::Dword(0));
                g.to_bytes().unwrap()
            });
            let mut man = Struct::new(4);
            man.set("Appearance_Type", Value::Word(0));
            man.set("Appearance_Head", Value::Byte(1));
            man.set("BodyPart_Torso", Value::Byte(1));
            man.set("BodyPart_Pelvis", Value::Byte(1));
            let mut git = Gff::new(*b"GIT ");
            git.root.set("Creature List", Value::List(vec![man; copies]));
            let script = format!("void main() {{ {} }}", "f(\"mg_named\"); ".repeat(copies));
            let asked = Arc::new(std::sync::Mutex::new(HashMap::new()));
            let content = Counting { files: content, asked: asked.clone() };
            let mut module = MemContainer::new();
            let mut m = Module::new();
            for (name, data) in
                [("a1.git", git.to_bytes().unwrap()), ("s.nss", script.into_bytes())]
            {
                module.insert(key(name), data.clone());
                m.set(key(name), data);
            }
            let mut rm = ResMan::new();
            rm.add(priority::MODULE, "module", LayerClass::Erf, module);
            rm.add(priority::HAK_USER, "hak:mg_content", LayerClass::Erf, content);
            rm.add(priority::KEY, "nwn_base.key", LayerClass::Key, stock);
            let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
            let mut walk = Walk::new(&game);
            walk.module(&m);
            walk.settle();
            let mut reached: Vec<(ResKey, ResKey)> = walk.from.into_iter().collect();
            reached.sort();
            let asked = asked.lock().unwrap().clone();
            (reached, asked)
        };
        let (one, _) = walk_of(1);
        let (many, asked) = walk_of(50);
        assert_eq!(many, one, "the same reached, from the same");
        for n in ["pmh0_chest001.mdl", "pmh0_head001.plt", "mg_named.utp"] {
            assert!(one.iter().any(|(k, _)| *k == key(n)), "{n} is reached: {one:?}");
        }
        let of = |n: &str| asked.get(&key(n)).copied().unwrap_or_default();
        for part in ["pmh0_chest001.mdl", "pmh0_pelvis001.mdl", "pmh0_head001.mdl"] {
            assert!(of(part).0 <= 2, "{part} was read {} times", of(part).0);
        }
        // (Asked after: the blueprint a script names, a model's texture's
        // material and its image.)
        for (n, most) in [("mg_named.utp", 4), ("pmh0_chest001.mtr", 4), ("pmh0_chest001.plt", 12)]
        {
            assert!(of(n).1 <= most, "{n} was asked after {} times", of(n).1);
        }
    }

    /// What the walk followed only since the deferred sprint: a creature's
    /// footsteps and combat sounds, a weapon's hits and its visual effect
    /// property's model, an armor's icon layers, the tables a 2DA names,
    /// the game's own 2DAs a hak replaces, and what players choose at
    /// character creation (heads, portraits, voice sets).
    #[test]
    fn sounds_icons_effects_tables_and_what_players_choose() {
        let table = |text: &str| format!("2DA V2.0\n\n{text}").into_bytes();
        let mut stock = MemContainer::new();
        stock.insert(
            key("appearance.2da"),
            table(
                "   LABEL MODELTYPE RACE FOOTSTEPTYPE SOUNDAPPTYPE\n0 Human P H 0 0\n\
                 1 Wolf F **** 1 1\n",
            ),
        );
        stock.insert(
            key("footstepsounds.2da"),
            table(" Label Dirt0 Stone0\n0 normal fs_a fs_b\n1 paw fs_paw ****\n"),
        );
        stock.insert(
            key("appearancesndset.2da"),
            table(
                " Label ArmorType WeapTypeL WeapTypeR WeapTypeS WeapTypeClsLw WeapTypeClsH \
                 WeapTypeRch MissIndex Looping FallFwd FallBck\n0 Human **** **** **** **** **** \
                 **** **** **** **** bf_h bf_h\n1 Wolf leather 1 1 1 1 1 1 1 **** bf_wolf bf_wolf\n",
            ),
        );
        stock.insert(
            key("weaponsounds.2da"),
            table(" Label Leather0 Leather1 Chain0 Miss0\n0 Blade cb_bl_le cb_bl_le2 cb_bl_ch cb_miss\n1 Claw cb_cl_le **** cb_cl_ch cb_cl_miss\n"),
        );
        stock.insert(
            key("baseitems.2da"),
            table(
                " label ItemClass ModelType WeaponMatType DefaultIcon\n0 shortsword WSwSs 2 0 iw_ss\n\
                 1 armor AArmor 3 **** ia_armor\n",
            ),
        );
        stock.insert(
            key("itempropdef.2da"),
            table(" Label SubTypeResRef\n0 Ability IPRP_ABILITIES\n1 VisualEffect IPRP_VISUALFX\n"),
        );
        stock.insert(key("iprp_visualfx.2da"), table(" Label ModelSuffix\n0 Acid acid\n"));
        stock.insert(key("racialtypes.2da"), table(" Label Appearance PlayerRace FeatsTable\n0 Human 0 1 race_feat_human\n1 Wolf 1 0 ****\n"));
        stock.insert(key("race_feat_human.2da"), table(" FeatIndex\n0 1\n"));
        stock.insert(key("portraits.2da"), table(" BaseResRef Race\n0 hu_m_01 0\n1 wolf_ 1\n"));
        stock.insert(key("soundset.2da"), table(" RESREF TYPE\n0 vs_player 0\n1 vs_npc 1\n"));
        stock.insert(key("classes.2da"), table(" Label FeatsTable\n0 Mine cls_feat_mine\n"));
        let mut haks = MemContainer::new();
        for (n, d) in [
            ("fs_paw.wav", &b"w"[..]),
            ("fs_a.wav", b"w"),
            ("bf_wolf.wav", b"w"),
            ("cb_cl_le.wav", b"w"),
            ("cb_cl_miss.wav", b"w"),
            ("cb_bl_le.wav", b"w"),
            ("cb_bl_ch.wav", b"w"),
            ("cb_miss.wav", b"w"),
            ("ipm_chest005.plt", b"p"),
            ("ipf_chest005.plt", b"p"),
            ("ipm_chest006.plt", b"p"),
            ("cls_feat_mine.2da", b"2DA V2.0\n\n FeatIndex\n"),
            ("cls_feat_none.2da", b"2DA V2.0\n\n FeatIndex\n"),
            ("vs_player.ssf", b"s"),
            ("vs_npc.ssf", b"s"),
            ("po_hu_m_01h.tga", b"t"),
            ("po_wolf_h.tga", b"t"),
        ] {
            haks.insert(key(n), d);
        }
        haks.insert(key("classes.2da"), table(" Label FeatsTable\n0 Mine cls_feat_mine\n"));
        stock.insert(key("feat.2da"), table(" LABEL ICON\n0 Alertness ife_alertness\n"));
        stock.insert(key("spells.2da"), table(" Label IconResRef\n0 Acid_Fog is_acidfog\n"));
        haks.insert(
            key("feat.2da"),
            table(
                " LABEL ICON Constant\n0 Alertness ife_alertness ****\n1 Mine IFE_Mine ife_other\n",
            ),
        );
        haks.insert(
            key("spells.2da"),
            table(
                " Label IconResRef ConjHeadVisual ConjSoundVFX\n0 Acid_Fog is_acidfog **** ****\n\
                 1 Mine is_mine vco_mine sco_mine\n",
            ),
        );
        for n in ["ife_mine.tga", "ife_other.tga", "is_mine.tga", "fx_mine.tga"] {
            haks.insert(key(n), &b"t"[..]);
        }
        haks.insert(key("sco_mine.wav"), &b"w"[..]);
        haks.insert(key("vco_mine.mdl"), model("vco_mine", "fx_mine"));
        haks.insert(key("wswss_fxacid.mdl"), model("wswss_fxacid", "fx_acid"));
        haks.insert(key("wswss_fxfire.mdl"), model("wswss_fxfire", "fx_fire"));
        haks.insert(key("pmh0_head007.mdl"), model("pmh0_head007", "pmh0_head007"));
        haks.insert(key("pmh0_head007.tga"), &b"t"[..]);
        haks.insert(key("pfh0_head300.mdl"), model("pfh0_head300", "x"));
        // Body parts: the creation screen's (part 1 of each, part 2 of a
        // tattoo's parts), and others: a third chest, a second belt, a
        // phenotype it doesn't offer.
        for n in [
            "pmh0_chest002",
            "pfh2_shinl002",
            "pmh0_handl001",
            "pmh0_chest003",
            "pmh0_belt002",
            "pmh0_legr002",
            "pmh4_chest001",
            "pmh4_head001",
        ] {
            haks.insert(key(&format!("{n}.mdl")), model(n, n));
        }
        let mut module = MemContainer::new();
        let mut git = Gff::new(*b"GIT ");
        let mut wolf = Struct::new(4);
        wolf.set("Appearance_Type", Value::Word(1));
        let mut sword = Struct::new(0);
        sword.set("BaseItem", Value::Int(0));
        let mut fx = Struct::new(0);
        fx.set("PropertyName", Value::Word(1));
        fx.set("Subtype", Value::Word(0));
        sword.set("PropertiesList", Value::List(vec![fx]));
        let mut armor = Struct::new(0);
        armor.set("BaseItem", Value::Int(1));
        armor.set("ArmorPart_Torso", Value::Byte(5));
        wolf.set("ItemList", Value::List(vec![sword, armor]));
        git.root.set("Creature List", Value::List(vec![wolf]));
        module.insert(key("a1.git"), git.to_bytes().unwrap());

        let mut rm = ResMan::new();
        rm.add(priority::MODULE, "module", LayerClass::Erf, module);
        rm.add(priority::HAK_USER, "hak:mine", LayerClass::Erf, haks);
        rm.add(priority::KEY, "nwn_base.key", LayerClass::Key, stock);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut m = Module::new();
        m.set(key("a1.git"), game.resman.get(&key("a1.git")).unwrap().into_owned());
        let mut walk = Walk::new(&game);
        walk.module(&m);
        walk.settle();
        let reached = |w: &Walk<'_>, n: &str| w.from.get(&key(n)).map(|f| f.to_string());
        // The wolf's paws, its fall, its claws and its hide struck by a
        // blade; the sword's hits and its acid; the armor's icon.
        for n in [
            "fs_paw.wav",
            "bf_wolf.wav",
            "cb_cl_le.wav",
            "cb_cl_miss.wav",
            "cb_bl_le.wav",
            "cb_bl_ch.wav",
            "cb_miss.wav",
            "wswss_fxacid.mdl",
            "ipm_chest005.plt",
            "ipf_chest005.plt",
        ] {
            assert_eq!(reached(&walk, n).as_deref(), Some("a1.git"), "{n}");
        }
        for n in ["fs_a.wav", "wswss_fxfire.mdl", "ipm_chest006.plt", "cls_feat_mine.2da"] {
            assert_eq!(reached(&walk, n), None, "{n}");
        }
        // The game's classes.2da, which the hak replaces: read by the game,
        // with the feat table it names.
        walk.game_reads([key("classes.2da")]);
        assert_eq!(reached(&walk, "classes.2da").as_deref(), Some("classes.2da"));
        assert_eq!(reached(&walk, "cls_feat_mine.2da").as_deref(), Some("classes.2da"));
        assert_eq!(reached(&walk, "cls_feat_none.2da"), None);
        // What the game shows of a table's rows by itself, with no script
        // naming the row: a hak's feat.2da over the game's brings its
        // feats' icons, its spells.2da its spells' icons, visuals and
        // sounds (they were listed unused: the review's M5); not what a
        // column the game shows nothing of names.
        for n in ["ife_mine.tga", "is_mine.tga", "vco_mine.mdl", "fx_mine.tga", "sco_mine.wav"] {
            assert_eq!(reached(&walk, n), None, "{n}");
        }
        walk.game_reads([key("feat.2da"), key("spells.2da")]);
        assert_eq!(reached(&walk, "ife_mine.tga").as_deref(), Some("feat.2da"));
        for n in ["is_mine.tga", "vco_mine.mdl", "fx_mine.tga", "sco_mine.wav"] {
            assert_eq!(reached(&walk, n).as_deref(), Some("spells.2da"), "{n}");
        }
        assert_eq!(reached(&walk, "ife_other.tga"), None);
        // A player's choices: a human's heads (any number a byte holds),
        // portraits and voice sets; not a wolf's portrait, nor an NPC's.
        walk.players_choose();
        walk.settle();
        for n in ["pmh0_head007.mdl", "pmh0_head007.tga", "po_hu_m_01h.tga", "vs_player.ssf"] {
            assert!(reached(&walk, n).is_some(), "{n}");
        }
        for n in ["pfh0_head300.mdl", "po_wolf_h.tga", "vs_npc.ssf"] {
            assert_eq!(reached(&walk, n), None, "{n}");
        }
        // The creation screen's body parts, for phenotypes 0 and 2 alone
        // (ruleset.2da's CHARGEN_ENABLED_PHENOTYPES, 5 where there is none).
        for n in ["pmh0_chest002.mdl", "pfh2_shinl002.mdl", "pmh0_handl001.mdl"] {
            assert_eq!(reached(&walk, n).as_deref(), Some("racialtypes.2da"), "{n}");
        }
        for n in [
            "pmh0_chest003.mdl",
            "pmh0_belt002.mdl",
            "pmh0_legr002.mdl",
            "pmh4_chest001.mdl",
            "pmh4_head001.mdl",
        ] {
            assert_eq!(reached(&walk, n), None, "{n}");
        }
    }

    /// The phenotypes of what players choose follow ruleset.2da's
    /// `CHARGEN_ENABLED_PHENOTYPES` where a hak changes it: phenotype 4
    /// offered (bits 0, 2 and 4), its heads and body parts are used.
    #[test]
    fn a_hak_s_ruleset_offers_another_phenotype() {
        let table = |text: &str| format!("2DA V2.0\n\n{text}").into_bytes();
        let mut stock = MemContainer::new();
        stock.insert(key("appearance.2da"), table("   LABEL MODELTYPE RACE\n0 Human P H\n"));
        stock.insert(key("racialtypes.2da"), table(" Label Appearance PlayerRace\n0 Human 0 1\n"));
        stock.insert(
            key("ruleset.2da"),
            table(" Label Value\n0 CHARGEN_ENABLED_PHENOTYPES 5\n1 OTHER 1\n"),
        );
        let mut haks = MemContainer::new();
        haks.insert(
            key("ruleset.2da"),
            table(" Label Value\n0 OTHER 1\n1 CHARGEN_ENABLED_PHENOTYPES 21\n"),
        );
        for n in ["pmh4_chest002", "pmh4_head009", "pmh2_neck001", "pmh1_neck001"] {
            haks.insert(key(&format!("{n}.mdl")), model(n, n));
        }
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:mine", LayerClass::Erf, haks);
        rm.add(priority::KEY, "nwn_base.key", LayerClass::Key, stock);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut walk = Walk::new(&game);
        walk.players_choose();
        walk.settle();
        let reached = |n: &str| walk.from.get(&key(n)).map(|f| f.to_string());
        for n in ["pmh4_chest002.mdl", "pmh4_head009.mdl", "pmh2_neck001.mdl"] {
            assert!(reached(n).is_some(), "{n}");
        }
        assert_eq!(reached("pmh1_neck001.mdl"), None);
    }

    /// A compiled script's rows named by a number, where the number is a
    /// constant: a visual effect's models, sounds and programmed effect, an
    /// area of effect's scripts, models, sounds and visual effect, a
    /// spell's icon, script, visuals and sounds, an item's spell (its icon
    /// and its spell), an appearance's model; and its strings. A number
    /// worked out as it runs (a variable's) is counted, not followed.
    #[test]
    fn a_compiled_script_s_numbers_and_strings() {
        let table = |text: &str| format!("2DA V2.0\n\n{text}").into_bytes();
        let mut stock = MemContainer::new();
        stock.insert(
            key("nwscript.nss"),
            &b"effect EffectVisualEffect(int nVisualEffectId, int nMissEffect=FALSE);\n\
               void ActionCastSpellAtObject(int nSpell, object oTarget);\n\
               void SetCreatureAppearanceType(object oCreature, int nAppearanceType);\n\
               itemproperty ItemPropertyCastSpell(int nSpell, int nNumUses);\n\
               effect EffectAreaOfEffect(int nAreaEffectId, string sOnEnterScript=\"\");\n"[..],
        );
        stock.insert(
            key("visualeffects.2da"),
            table(
                " Label Imp_Impact_Node SoundImpact ProgFX_Impact\n0 A vfx_none **** ****\n\
                 1 B **** **** ****\n2 Heal vfx_heal snd_heal 1\n3 Fog vfx_fog **** ****\n",
            ),
        );
        stock.insert(
            key("vfx_persistent.2da"),
            table(
                " LABEL SHAPE RADIUS ONENTER DurationVFX MODEL01 SoundDuration\n\
                 0 VFX_PER_A C 5 mg_other_a **** vps_other ****\n\
                 1 VFX_PER_MG C 5 mg_aoe_a 3 vps_mg sps_mg_loop\n",
            ),
        );
        stock.insert(key("progfx.2da"), table(" Label Param1\n0 X ****\n1 Glow fx_glowtex\n"));
        stock.insert(
            key("spells.2da"),
            table(
                " Label IconResRef ImpactScript ConjHeadVisual CastSound\n0 A **** **** **** ****\n\
                 1 Fire is_fire nw_s0_fire vco_fire sco_fire\n2 B **** **** **** ****\n\
                 3 Ice is_ice **** **** ****\n",
            ),
        );
        stock
            .insert(key("iprp_spells.2da"), table(" Label SpellIndex Icon\n0 IceWand 3 is_wand\n"));
        stock.insert(
            key("appearance.2da"),
            table(" LABEL MODELTYPE RACE\n0 Human P H\n1 Wolf F c_wolf\n"),
        );
        stock.insert(key("baseitems.2da"), table(" label ItemClass\n0 shortsword WSwSs\n"));
        let mut haks = MemContainer::new();
        for n in [
            "snd_heal.wav",
            "fx_glowtex.tga",
            "is_fire.tga",
            "sco_fire.wav",
            "is_ice.tga",
            "is_wand.tga",
            "mg_ding.wav",
            "sps_mg_loop.wav",
        ] {
            haks.insert(key(n), &b"x"[..]);
        }
        for n in ["vfx_heal", "vfx_none", "vco_fire", "c_wolf", "vfx_fog", "vps_mg", "vps_other"] {
            haks.insert(key(&format!("{n}.mdl")), model(n, "x"));
        }
        // (A compiled script: the header, then the code.)
        let ncs = |code: &[&[u8]]| {
            let code = code.concat();
            let mut d = b"NCS V1.0\x42".to_vec();
            d.extend_from_slice(&(13 + code.len() as u32).to_be_bytes());
            d.extend_from_slice(&code);
            d
        };
        let int = |n: i32| [&[0x04, 0x03][..], &n.to_be_bytes()].concat();
        let object = [&[0x04, 0x06][..], &0u32.to_be_bytes()].concat();
        let call = |id: u8, argc: u8| vec![0x05, 0x00, 0x00, id, argc];
        haks.insert(key("nw_s0_fire.ncs"), ncs(&[&[0x20, 0x00]]));
        haks.insert(key("mg_aoe_a.ncs"), ncs(&[&[0x20, 0x00]]));
        let mut module = MemContainer::new();
        module.insert(
            key("mg_fx.ncs"),
            ncs(&[
                // EffectVisualEffect(2, FALSE)
                &int(0),
                &int(2),
                &call(0, 2),
                // ActionCastSpellAtObject(1, OBJECT_SELF)
                &object,
                &int(1),
                &call(1, 2),
                // SetCreatureAppearanceType(OBJECT_SELF, 1)
                &int(1),
                &object,
                &call(2, 2),
                // ItemPropertyCastSpell(0, 1)
                &int(1),
                &int(0),
                &call(3, 2),
                // EffectAreaOfEffect(1)
                &int(1),
                &call(4, 1),
                // EffectVisualEffect(nVis): a variable's copy.
                &[0x03, 0x01, 0xff, 0xff, 0xff, 0xfc, 0x00, 0x04],
                &call(0, 1),
                // "mg_ding"
                &[0x04, 0x05, 0x00, 0x07],
                b"mg_ding",
            ]),
        );
        let mut rm = ResMan::new();
        rm.add(priority::MODULE, "module", LayerClass::Erf, module);
        rm.add(priority::HAK_USER, "hak:mine", LayerClass::Erf, haks);
        rm.add(priority::KEY, "nwn_base.key", LayerClass::Key, stock);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut m = Module::new();
        m.set(key("mg_fx.ncs"), game.resman.get(&key("mg_fx.ncs")).unwrap().into_owned());
        let mut walk = Walk::new(&game);
        walk.module(&m);
        walk.settle();
        let reached = |w: &Walk<'_>, n: &str| w.from.get(&key(n)).map(|f| f.to_string());
        for n in [
            "vfx_heal.mdl",
            "snd_heal.wav",
            "fx_glowtex.tga",
            "is_fire.tga",
            "nw_s0_fire.ncs",
            "vco_fire.mdl",
            "sco_fire.wav",
            "is_ice.tga",
            "is_wand.tga",
            "c_wolf.mdl",
            "mg_ding.wav",
            "mg_aoe_a.ncs",
            "vps_mg.mdl",
            "sps_mg_loop.wav",
            "vfx_fog.mdl",
        ] {
            assert_eq!(reached(&walk, n).as_deref(), Some("mg_fx.ncs"), "{n}");
        }
        assert_eq!(reached(&walk, "vps_other.mdl"), None, "an area of effect nothing names");
        assert_eq!(reached(&walk, "vfx_none.mdl"), None, "row 0, which nothing names");
        assert_eq!(walk.by_number.followed, 5);
        assert_eq!(
            walk.by_number.not_followed,
            vec![(key("mg_fx.ncs"), "EffectVisualEffect".to_string())]
        );
    }
}
