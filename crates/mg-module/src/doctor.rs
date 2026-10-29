//! The content doctor: problems in a module's custom content that crash
//! Aurora (most with an access violation that names nothing) or break the
//! game, each found where it is: the hak or folder, the file, the 2DA row and
//! column, the tileset section, the object. Collected from nwn.wiki's
//! "Common Errors and Their Causes" and Beamdog's issue tracker
//! (`docs/research/community_pain_points.md`).
//!
//! Only custom content is examined: the module, haks from the user's `hak`
//! folder, `override`, `development` and NWSync content; the game's own data
//! and the haks it ships are taken as they are.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::{Duration, Instant};

use mg_2da::{TwoDa, overlong_rows};
use mg_core::{Codepage, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::{Layer, LayerClass, ResKey, ResMan, priority};
use mg_set::Tileset;
pub use mg_shader::ProgramCache;
use mg_shader::{Assembled, Preamble, Stage, assemble, program_stage};
use rayon::prelude::*;

use crate::Module;
use crate::verify_cache::{Facts, Learned, Memo, Noted};

/// How bad a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Crashes Aurora or breaks the game.
    Error,
    /// Likely wrong; may misbehave.
    Warning,
}

/// The doctor's checks: each has an id that its findings carry (and `mg
/// verify --json` prints), and what it holds to be true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Check {
    CustomTlk,
    ErfSize,
    SetRead,
    SetLineEnds,
    SetCount,
    SetHole,
    SetModel,
    SetDoor,
    SetGroup,
    SetTransition,
    SetData,
    TileFaces,
    MdlRead,
    MdlText,
    MtrName,
    ShaderCompile,
    ShaderPair,
    TwoDaRow,
    TwoDaRead,
    TwoDaLimit,
    TwoDaShadow,
    TwoDaStrRef,
    TwoDaRef,
    TwoDaDescribed,
    ObjectRow,
    PcFaction,
    StaticTransform,
    AreaVariables,
    TagUnique,
    NuiSources,
    VarType,
    VarValue,
    VarObject,
    VarName,
    VarRuntime,
    VarRequires,
    VarScript,
    ClaimsMerged,
    ClaimsTag,
}

impl Check {
    pub const ALL: [Check; 39] = [
        Check::CustomTlk,
        Check::ErfSize,
        Check::SetRead,
        Check::SetLineEnds,
        Check::SetCount,
        Check::SetHole,
        Check::SetModel,
        Check::SetDoor,
        Check::SetGroup,
        Check::SetTransition,
        Check::SetData,
        Check::TileFaces,
        Check::MdlRead,
        Check::MdlText,
        Check::MtrName,
        Check::ShaderCompile,
        Check::ShaderPair,
        Check::TwoDaRow,
        Check::TwoDaRead,
        Check::TwoDaLimit,
        Check::TwoDaShadow,
        Check::TwoDaStrRef,
        Check::TwoDaRef,
        Check::TwoDaDescribed,
        Check::ObjectRow,
        Check::PcFaction,
        Check::StaticTransform,
        Check::AreaVariables,
        Check::TagUnique,
        Check::NuiSources,
        Check::VarType,
        Check::VarValue,
        Check::VarObject,
        Check::VarName,
        Check::VarRuntime,
        Check::VarRequires,
        Check::VarScript,
        Check::ClaimsMerged,
        Check::ClaimsTag,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Check::CustomTlk => "custom-tlk",
            Check::ErfSize => "erf-size",
            Check::SetRead => "set-read",
            Check::SetLineEnds => "set-line-ends",
            Check::SetCount => "set-count",
            Check::SetHole => "set-hole",
            Check::SetModel => "set-model",
            Check::SetDoor => "set-door",
            Check::SetGroup => "set-group",
            Check::SetTransition => "set-transition",
            Check::SetData => "set-data",
            Check::TileFaces => "tile-faces",
            Check::MdlRead => "mdl-read",
            Check::MdlText => "mdl-text",
            Check::MtrName => "mtr-name",
            Check::ShaderCompile => "shader-compile",
            Check::ShaderPair => "shader-pair",
            Check::TwoDaRow => "2da-row",
            Check::TwoDaRead => "2da-read",
            Check::TwoDaLimit => "2da-limit",
            Check::TwoDaShadow => "2da-shadow",
            Check::TwoDaStrRef => "2da-strref",
            Check::TwoDaRef => "2da-ref",
            Check::TwoDaDescribed => "2da-described",
            Check::ObjectRow => "object-row",
            Check::PcFaction => "pc-faction",
            Check::StaticTransform => "static-transform",
            Check::AreaVariables => "area-variables",
            Check::TagUnique => "tag-unique",
            Check::NuiSources => "nui-sources",
            Check::VarType => "var-type",
            Check::VarValue => "var-value",
            Check::VarObject => "var-object",
            Check::VarName => "var-name",
            Check::VarRuntime => "var-runtime",
            Check::VarRequires => "var-requires",
            Check::VarScript => "var-script",
            Check::ClaimsMerged => "claims-merged",
            Check::ClaimsTag => "claims-tag",
        }
    }

    pub fn from_id(id: &str) -> Option<Check> {
        Check::ALL.into_iter().find(|c| c.id() == id)
    }

    /// What the check holds to be true (a finding says where it is not).
    pub fn about(self) -> &'static str {
        match self {
            Check::CustomTlk => "the module's custom talk table is where the game looks for it",
            Check::ErfSize => "an archive's resources start within the 2 GiB the game can read",
            Check::SetRead => "a tileset can be read",
            Check::SetLineEnds => {
                "a tileset's lines end in CRLF (with a bare LF the game's server never finishes \
                 loading a module that uses it)"
            }
            Check::SetCount => "a tileset's tile and group counts match its sections",
            Check::SetHole => {
                "every tile, terrain, crosser and rule a tileset counts has its section, and every \
                 tile a Model= (the game draws a hole as nothing, with no walkmesh)"
            }
            Check::SetModel => "each tile's model is in the haks or the game",
            Check::SetDoor => "a tile's door types are doortypes.2da rows with a model",
            Check::SetGroup => {
                "a tileset's groups have a size, a first tile of their own and tiles that exist"
            }
            Check::SetTransition => "a tileset with groups has a height transition",
            Check::SetData => {
                "a tileset's tiles, terrains, crossers and rules agree with each other"
            }
            Check::TileFaces => "a tile model has no more faces than Aurora can paint",
            Check::MdlRead => "a tile's model can be read",
            Check::MdlText => {
                "a model kept as text has nothing the game refuses, misreads or crashes on"
            }
            Check::MtrName => "a material's texture names fit a resource name",
            Check::ShaderCompile => {
                "a shader program of the module's content compiles (the game draws one that \
                 doesn't magenta; Moonglow draws with its own shader instead)"
            }
            Check::ShaderPair => {
                "a material that names a custom shader names both its programs \
                 (`customshaderVS` and `customshaderFS`: the game takes the two or neither)"
            }
            Check::TwoDaRow => "a 2DA row has no more cells than the table has columns",
            Check::TwoDaRead => "a 2DA can be read",
            Check::TwoDaLimit => "a 2DA has no more rows than the game or Aurora takes",
            Check::TwoDaShadow => "a 2DA does not hide a longer copy in another hak",
            Check::TwoDaStrRef => "a 2DA's changed rows name talk-table strings that exist",
            Check::TwoDaRef => {
                "a 2DA's changed rows name rows of other 2DAs that exist and aren't blank, and \
                 2DAs that are there"
            }
            Check::TwoDaDescribed => {
                "a table of the module's that has a description (name.2da.toml) holds what it says"
            }
            Check::ObjectRow => "objects use 2DA rows that exist and have what they need",
            Check::PcFaction => "no object belongs to the PC faction, the players' alone",
            Check::StaticTransform => {
                "a static placeable has no visual transform: the game draws it without one"
            }
            Check::AreaVariables => "an area's variables are in its GIT, where the game reads them",
            Check::TagUnique => {
                "a tag the project says must be unique (moonglow.toml's [tags] unique) is on one \
                 object only, or on one of each area (unique_in_area)"
            }
            Check::NuiSources => {
                "a nasher project's sources take in its NUI windows' files (.jui and .txt), which \
                 nasher otherwise doesn't pack and Moonglow doesn't read back"
            }
            Check::VarType => {
                "a variable the project describes (variables.toml) is of the type it says: one \
                 of another type reads as that type's empty value (an Int read as a String is \"\")"
            }
            Check::VarValue => {
                "a described variable's value is one of those it lists, within its range"
            }
            Check::VarObject => "a described variable is on the kind of object it says",
            Check::VarName => {
                "a variable named as the project's are (variables.toml's undescribed) is \
                 described, and a described one is named in the case it is described in"
            }
            Check::VarRuntime => {
                "a variable the project describes as the scripts' alone (variables.toml's \
                 runtime = true: state or a test's hook, set while the server runs) is kept in no \
                 file of the module"
            }
            Check::VarRequires => {
                "a variable the project says needs others beside it (variables.toml's requires, \
                 and its [[rules]]: one needed where another has a value) has them on the same \
                 object"
            }
            Check::VarScript => {
                "a variable kept in a file is of a type the module's scripts read it as (by its \
                 name written out: GetLocalInt(o, \"tdod_lit\")): one of another type reads as \
                 that type's empty value"
            }
            Check::ClaimsMerged => {
                "an area the project's claims table says is merged (in the game) is in the module \
                 under its ResRef, which keeps players' first visits"
            }
            Check::ClaimsTag => {
                "a merged area's Tag is the one the claims table records, which keeps players' \
                 positions"
            }
        }
    }

    /// The group it is reported in.
    pub fn group(self) -> Group {
        match self {
            Check::SetRead
            | Check::SetLineEnds
            | Check::SetHole
            | Check::SetCount
            | Check::SetModel
            | Check::SetDoor
            | Check::SetGroup
            | Check::SetTransition
            | Check::SetData
            | Check::TileFaces
            | Check::MdlRead => Group::Tilesets,
            Check::MdlText | Check::MtrName | Check::ShaderCompile | Check::ShaderPair => {
                Group::Models
            }
            Check::TwoDaRow
            | Check::TwoDaRead
            | Check::TwoDaLimit
            | Check::TwoDaShadow
            | Check::TwoDaStrRef
            | Check::TwoDaRef
            | Check::TwoDaDescribed => Group::Tables,
            Check::ObjectRow
            | Check::PcFaction
            | Check::StaticTransform
            | Check::AreaVariables
            | Check::TagUnique
            | Check::VarType
            | Check::VarValue
            | Check::VarObject
            | Check::VarName
            | Check::VarRuntime
            | Check::VarRequires
            | Check::VarScript => Group::Objects,
            Check::CustomTlk
            | Check::ErfSize
            | Check::NuiSources
            | Check::ClaimsMerged
            | Check::ClaimsTag => Group::Module,
        }
    }
}

/// What Verify reports, in groups a builder can leave out (a tileset of a
/// hak that has errors nobody is going to fix hides the rest): the missing
/// references ([`crate::verify::Missing`]), the doctor's checks by what
/// they examine, and the plugins' checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    References,
    Tilesets,
    Models,
    Tables,
    Objects,
    Module,
    Plugins,
}

impl Group {
    pub const ALL: [Group; 7] = [
        Group::References,
        Group::Tilesets,
        Group::Models,
        Group::Tables,
        Group::Objects,
        Group::Module,
        Group::Plugins,
    ];

    /// Its id, as the settings keep it and `mg verify --skip` takes it.
    pub fn id(self) -> &'static str {
        match self {
            Group::References => "references",
            Group::Tilesets => "tilesets",
            Group::Models => "models",
            Group::Tables => "2das",
            Group::Objects => "objects",
            Group::Module => "module",
            Group::Plugins => "plugins",
        }
    }

    /// Its name, as a checkbox has it.
    pub fn title(self) -> &'static str {
        match self {
            Group::References => "Missing references",
            Group::Tilesets => "Tilesets",
            Group::Models => "Models and materials",
            Group::Tables => "2DAs",
            Group::Objects => "Objects' and areas' 2DA rows and variables",
            Group::Module => "Talk table and archives",
            Group::Plugins => "Plugins' checks",
        }
    }

    pub fn from_id(id: &str) -> Option<Group> {
        Group::ALL.into_iter().find(|g| g.id() == id)
    }

    /// The group of a finding: its check's, or the plugins' for a check
    /// that isn't the doctor's.
    pub fn of(f: &Finding) -> Group {
        Check::ALL.into_iter().find(|c| c.id() == f.check).map_or(Group::Plugins, Check::group)
    }
}

/// What Verify leaves out (`mg verify --skip`, Options › Verify), each
/// said by a word: a whole [`Group`] (`tilesets`), one check (the
/// doctor's, `set-hole`, as `mg checks` lists them, or a plugin's,
/// `example.tags/tag-case`), the missing references of one category
/// (`references:creatures`: Build Module's Missing Resources' categories),
/// what a hak or folder holds (`hak:tdod_tiles01` or `tdod_tiles01.hak`,
/// `override`, `development`) and one resource (a tileset, `ttr01.set`,
/// with its tiles' models; a 2DA; a model). What is left out is not
/// checked where that can be told beforehand (a group's checks, a check's
/// costly part, a hak's or a resource's own checks: [`examine_groups`],
/// and Verify's missing references and plugins), so nothing is counted of
/// it; [`Skip::leave_out`] takes what was found of it anyway out of the
/// report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Skip {
    groups: BTreeSet<Group>,
    checks: BTreeSet<String>,
    categories: BTreeSet<crate::verify::Category>,
    /// Layers' labels, in lower case: `hak:tdod_tiles01`, `override`.
    sources: BTreeSet<String>,
    resources: BTreeSet<ResKey>,
    /// The words, as read, in their order, once each.
    words: Vec<String>,
}

impl Skip {
    /// Nothing left out.
    pub const NONE: Skip = Skip {
        groups: BTreeSet::new(),
        checks: BTreeSet::new(),
        categories: BTreeSet::new(),
        sources: BTreeSet::new(),
        resources: BTreeSet::new(),
        words: Vec::new(),
    };

    /// The words `mg verify --skip` takes beside groups and checks, for its
    /// help and its errors.
    pub const WORDS: &'static str = "a group (references, tilesets, models, 2das, objects, \
         module, plugins), a check's id (`mg checks`), references:CATEGORY (creatures, doors, \
         placeables, items, sounds, triggers, waypoints, stores, conversations, encounters, \
         areas, module, scripts, other), a hak (hak:NAME or NAME.hak), override, development, \
         or a resource (ttr01.set, appearance.2da, a model's name.mdl)";

    /// Whole groups left out.
    pub fn groups(groups: &[Group]) -> Skip {
        let mut skip = Skip::default();
        for g in groups {
            skip.add(g.id()).expect("a group's id");
        }
        skip
    }

    /// Words read; or the first that isn't one.
    pub fn parse<S: AsRef<str>>(words: &[S]) -> Result<Skip, String> {
        let mut skip = Skip::default();
        for w in words {
            skip.add(w.as_ref())?;
        }
        Ok(skip)
    }

    /// Words read, passing over those that aren't (a setting written by a
    /// later version).
    pub fn read<S: AsRef<str>>(words: &[S]) -> Skip {
        let mut skip = Skip::default();
        for w in words {
            let _ = skip.add(w.as_ref());
        }
        skip
    }

    /// One word more left out; an empty one is nothing.
    pub fn add(&mut self, word: &str) -> Result<(), String> {
        let word = word.trim().to_ascii_lowercase();
        if word.is_empty() || self.words.contains(&word) {
            return Ok(());
        }
        if let Some(g) = Group::from_id(&word) {
            self.groups.insert(g);
        } else if Check::from_id(&word).is_some() || word.contains('/') {
            self.checks.insert(word.clone());
        } else if let Some(c) = word.strip_prefix("references:") {
            let c = crate::verify::Category::from_id(c)
                .ok_or_else(|| format!("not a category of missing references: {c}"))?;
            self.categories.insert(c);
        } else if let Some(hak) = word.strip_prefix("hak:").or_else(|| word.strip_suffix(".hak")) {
            if hak.is_empty() {
                return Err(format!("no hak named: {word}"));
            }
            self.sources.insert(format!("hak:{hak}"));
        } else if matches!(word.as_str(), "override" | "development") {
            self.sources.insert(word.clone());
        } else if let Some(key) =
            ResKey::from_filename(&word).filter(|k| k.restype != ResType::INVALID)
        {
            self.resources.insert(key);
        } else {
            return Err(format!("not something Verify can leave out: {word} (one of: {})", {
                Skip::WORDS
            }));
        }
        self.words.push(word);
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// The words, as read: what the report names as left out.
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// Whether a whole group is left out.
    pub fn group(&self, g: Group) -> bool {
        self.groups.contains(&g)
    }

    /// Whether one of the doctor's checks is left out: it, or its group.
    pub fn check(&self, c: Check) -> bool {
        self.group(c.group()) || self.checks.contains(c.id())
    }

    /// Whether all of these checks are (a part of Verify that makes their
    /// findings alone isn't run).
    pub fn all(&self, checks: &[Check]) -> bool {
        checks.iter().all(|c| self.check(*c))
    }

    /// Whether a plugin's check is left out, by its id as its findings
    /// carry it ([`Finding::check`]): it, or the plugins' group.
    pub fn plugin_check(&self, id: &str) -> bool {
        self.group(Group::Plugins) || self.checks.contains(&id.to_ascii_lowercase())
    }

    /// Whether what a layer holds (`hak:tdod_tiles01`) is left out.
    pub fn source(&self, label: &str) -> bool {
        !self.sources.is_empty() && self.sources.contains(&label.to_ascii_lowercase())
    }

    pub fn resource(&self, key: &ResKey) -> bool {
        self.resources.contains(key)
    }

    /// Whether a finding is of something left out.
    pub fn finding(&self, f: &Finding) -> bool {
        self.group(Group::of(f))
            || self.checks.contains(&f.check.to_ascii_lowercase())
            || self.source(&f.source)
            || self.resource(&f.resource)
    }

    /// Whether a missing reference is: the group, its category, or the
    /// resource that makes it.
    pub fn missing(&self, m: &crate::verify::Missing) -> bool {
        self.group(Group::References)
            || self.categories.contains(&m.category)
            || self.resource(&m.reference.from)
    }

    /// Takes what was found of what is left out out of Verify's report.
    pub fn leave_out(
        &self,
        missing: &mut Vec<crate::verify::Missing>,
        findings: &mut Vec<Finding>,
    ) {
        missing.retain(|m| !self.missing(m));
        findings.retain(|f| !self.finding(f));
    }

    /// What the report's summary names: a group by its title, the rest by
    /// its word.
    pub fn named(&self) -> Vec<String> {
        (self.words.iter())
            .map(|w| Group::from_id(w).map_or_else(|| w.clone(), |g| g.title().to_string()))
            .collect()
    }
}

/// How long the parts of a Verify took (`mg verify --timings`): each
/// part's name and its time, in the order they ran.
#[derive(Debug, Clone, Default)]
pub struct Timings {
    pub parts: Vec<(String, Duration)>,
}

impl Timings {
    pub fn add(&mut self, name: impl Into<String>, took: Duration) {
        self.parts.push((name.into(), took));
    }

    /// Runs `f`, timed as `name`.
    pub fn time<T>(&mut self, name: impl Into<String>, f: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let made = f();
        self.add(name, started.elapsed());
        made
    }
}

/// One problem, where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    /// The check's id: one of the doctor's ([`Check::id`]: `set-model`,
    /// `2da-row`, ...), or another's that adds its findings to these.
    pub check: Cow<'static, str>,
    /// Where the resource comes from: `module`, a hak's name, `override`.
    pub source: String,
    pub resource: ResKey,
    /// Where in it: `row 12, ModelName`, `[TILE40] Model`, an object.
    pub at: String,
    pub message: String,
}

/// What the doctor needs to know beyond the resources: how many strings the
/// talk tables have.
#[derive(Debug, Clone, Copy, Default)]
pub struct TalkTables {
    /// `dialog.tlk`'s count.
    pub base: usize,
    /// The module's custom talk table's count, if it names one and it was
    /// found where the game looks ([`crate::talk::find`]).
    pub custom: Option<usize>,
}

impl TalkTables {
    /// The count of a table not read, whose strings are not counted (a
    /// plugin's test with no game and no talk table of its own): `base`
    /// 0, `custom` `Some(UNCOUNTED)`. None of its strings is said to be
    /// missing, nor the table.
    pub const UNCOUNTED: usize = usize::MAX;
}

/// Where Verify keeps the shader programs that compiled: the user's cache
/// folder's `shaders`, which the window's views keep theirs in too
/// ([`crate::build::cache_dir`]).
pub fn shader_cache_dir() -> Option<std::path::PathBuf> {
    crate::build::cache_dir().map(|d| d.join("shaders"))
}

/// What Verify keeps between runs, in a cache folder (the user's,
/// [`crate::build::cache_dir`]): the shader programs that compiled
/// (`shaders`, the views' too) and what it found of each model of an
/// archive that is as it was (`verify`: a model's text's findings, a
/// tile model's faces), so that neither is done again.
#[derive(Debug, Clone)]
pub struct Cache {
    shaders: ProgramCache,
    models: std::path::PathBuf,
}

impl Cache {
    /// The cache in `root`: its `shaders` and `verify` folders.
    pub fn new(root: impl Into<std::path::PathBuf>) -> Cache {
        let root = root.into();
        Cache { shaders: ProgramCache::new(root.join("shaders")), models: root.join("verify") }
    }

    /// The user's ([`crate::build::cache_dir`]).
    pub fn user() -> Option<Cache> {
        crate::build::cache_dir().map(Cache::new)
    }

    pub fn shaders(&self) -> &ProgramCache {
        &self.shaders
    }
}

/// Whether a layer holds custom content (the user's or the module's).
fn is_custom(layer: &Layer) -> bool {
    matches!(
        layer.priority,
        priority::OVERRIDE
            | priority::MODULE
            | priority::HAK_USER
            | priority::NWSYNC
            | priority::DEVELOPMENT
            | priority::DEVELOPMENT_USER
    )
}

struct Doctor<'a> {
    rm: &'a ResMan,
    tlk: TalkTables,
    /// Where shader programs that compiled are kept between runs.
    shaders: Option<&'a ProgramCache>,
    /// What earlier runs found of the archives' models...
    memo: Option<&'a Memo>,
    /// ...and what this one learned of them, for the next: by layer.
    learned: Learned,
    /// What Verify leaves out: never pushed.
    skip: &'a Skip,
    out: Vec<Finding>,
    tables: HashMap<String, Option<TwoDa>>,
}

/// Nothing left out.
static NOTHING: Skip = Skip::NONE;

impl<'a> Doctor<'a> {
    /// A doctor for one resource's checks, beside this one.
    fn beside(&self) -> Doctor<'a> {
        Doctor {
            rm: self.rm,
            tlk: self.tlk,
            shaders: self.shaders,
            memo: self.memo,
            learned: Vec::new(),
            skip: self.skip,
            out: Vec::new(),
            tables: HashMap::new(),
        }
    }

    /// What earlier runs found of a layer's model.
    fn known(&self, layer: usize, key: &ResKey) -> Option<&'a Facts> {
        self.memo?.get(layer, key)
    }

    /// Notes what was found of a layer's model, for the next run, where
    /// its layer is kept.
    fn learn(&mut self, layer: usize, key: ResKey, facts: Facts) {
        if self.memo.is_some_and(|m| m.keeps(layer)) {
            self.learned.push((layer, key, facts));
        }
    }

    fn push(
        &mut self,
        severity: Severity,
        check: Check,
        source: &str,
        resource: ResKey,
        at: impl Into<String>,
        message: impl Into<String>,
    ) {
        if self.skip.check(check) || self.skip.source(source) || self.skip.resource(&resource) {
            return;
        }
        self.out.push(Finding {
            severity,
            check: Cow::Borrowed(check.id()),
            source: source.to_string(),
            resource,
            at: at.into(),
            message: message.into(),
        });
    }

    /// A 2DA as the game resolves it.
    fn table(&mut self, name: &str) -> Option<&TwoDa> {
        if !self.tables.contains_key(name) {
            let t = ResRef::from_str(name)
                .ok()
                .and_then(|r| self.rm.get(&ResKey::new(r, ResType::TWODA)).ok())
                .and_then(|d| TwoDa::parse(&d, Codepage::default()).ok());
            self.tables.insert(name.to_string(), t);
        }
        self.tables[name].as_ref()
    }

    /// Whether a 2DA row exists: within the table and with something in the
    /// given column.
    fn row_has(&mut self, table: &str, row: i64, column: &str) -> bool {
        let Some(t) = self.table(table) else { return true };
        usize::try_from(row).ok().and_then(|r| t.get(r, column)).is_some()
    }
}

/// Examines the module's custom content. `resman` has the module and its
/// haks layered in, as Verify and the game see them.
pub fn examine(module: &Module, resman: &ResMan, tlk: TalkTables) -> Vec<Finding> {
    examine_groups(module, resman, tlk, &Skip::NONE, &mut Timings::default(), None)
}

/// [`examine`], leaving out what `skip` says (not run where that can be
/// told beforehand: a builder's skipped tilesets and models are most of
/// the time a Verify of a module with many haks takes), and timing its
/// parts.
///
/// The checks of each custom tileset, 2DA, material and model run in
/// parallel; the findings are in the order one after another would make.
/// With a `cache` (the user's: `mg verify`'s and the window's,
/// [`Cache::user`]) the shader programs that compiled are kept, and not
/// compiled again by the next run, and so is what was found of each model
/// of an archive that is as it was (its text's findings, a tile model's
/// faces): its models are not read again.
pub fn examine_groups(
    module: &Module,
    resman: &ResMan,
    tlk: TalkTables,
    skip: &Skip,
    timings: &mut Timings,
    cache: Option<&Cache>,
) -> Vec<Finding> {
    use Check::*;
    // (A part whose checks are all left out isn't run.)
    let runs = |checks: &[Check]| !skip.all(checks);
    let shaders = cache.map(Cache::shaders);
    // The winning copy of every custom resource, by type.
    let layers = resman.layers();
    let memo = cache.map(|c| {
        let custom = |i: usize| is_custom(&layers[i]) && !skip.source(&layers[i].label);
        timings.time("what earlier runs found", || Memo::load(&c.models, resman, custom))
    });
    let mut d = Doctor {
        rm: resman,
        tlk,
        shaders,
        memo: memo.as_ref(),
        learned: Vec::new(),
        skip,
        out: Vec::new(),
        tables: HashMap::new(),
    };
    // (Looked up in parallel: a module with a hundred haks has hundreds of
    // thousands, each looked for in the layers above its own. A hak left
    // out, or a resource, isn't examined.)
    let custom = timings.time("the custom resources", || {
        let mut custom: BTreeMap<ResKey, usize> = BTreeMap::new();
        for (i, layer) in layers.iter().enumerate() {
            if !is_custom(layer) || skip.source(&layer.label) {
                continue;
            }
            let keys: Vec<ResKey> = layer.container.keys().collect();
            let won: Vec<ResKey> = (keys.into_par_iter())
                .filter(|key| !skip.resource(key) && resman.find(key) == Some(i))
                .collect();
            custom.extend(won.into_iter().map(|key| (key, i)));
        }
        custom
    });
    if runs(&[ErfSize]) {
        timings.time("archives' sizes", || {
            for layer in layers.iter().filter(|l| is_custom(l)) {
                past_read_limit(&mut d, layer);
            }
        });
    }
    // Each resource's own checks, by its type's (those that make the
    // findings of a type: a tileset's are its group's).
    let kind = |t: ResType| match t {
        ResType::SET => Some(("tilesets", Group::Tilesets)),
        ResType::TWODA => Some(("2das", Group::Tables)),
        ResType::MTR => Some(("materials", Group::Models)),
        ResType::MDL => Some(("models", Group::Models)),
        _ => None,
    };
    let checked = |t: ResType| match t {
        ResType::SET => !(Check::ALL.into_iter())
            .filter(|c| c.group() == Group::Tilesets)
            .all(|c| skip.check(c)),
        ResType::TWODA => {
            runs(&[TwoDaRow, TwoDaRead, TwoDaLimit, TwoDaShadow, TwoDaStrRef, TwoDaRef])
        }
        ResType::MTR => runs(&[MtrName]),
        ResType::MDL => runs(&[MdlText]),
        _ => false,
    };
    let each: Vec<(&ResKey, usize)> =
        (custom.iter()).filter(|(k, _)| checked(k.restype)).map(|(k, l)| (k, *l)).collect();
    let started = Instant::now();
    // (A worker's 2DAs are read once for the resources it takes.)
    let found: Vec<(ResKey, Vec<Finding>, Learned, Duration)> = each
        .par_iter()
        .map_init(HashMap::new, |tables, &(key, layer)| {
            let started = Instant::now();
            let mut d = Doctor {
                rm: resman,
                tlk,
                shaders,
                memo: memo.as_ref(),
                learned: Vec::new(),
                skip,
                out: Vec::new(),
                tables: std::mem::take(tables),
            };
            let source = &layers[layer].label;
            match key.restype {
                ResType::SET => tileset(&mut d, source, *key),
                ResType::TWODA => two_da(&mut d, source, *key, layer),
                ResType::MTR => material(&mut d, source, *key),
                ResType::MDL => model_text(&mut d, source, *key, layer),
                _ => {}
            }
            *tables = d.tables;
            (*key, d.out, d.learned, started.elapsed())
        })
        .collect();
    timings.add("each tileset, 2DA, material and model (in parallel)", started.elapsed());
    // Of each type: how many, their time added up, the longest.
    let mut work: BTreeMap<&str, (usize, Duration, Duration, ResKey)> = BTreeMap::new();
    for (key, out, learned, took) in found {
        d.out.extend(out);
        d.learned.extend(learned);
        if let Some((name, _)) = kind(key.restype) {
            let w = work.entry(name).or_insert((0, Duration::ZERO, Duration::ZERO, key));
            w.0 += 1;
            w.1 += took;
            if took > w.2 {
                (w.2, w.3) = (took, key);
            }
        }
    }
    for (name, (n, took, longest, key)) in work {
        let longest = longest.as_secs_f64();
        timings.add(format!("  {name}: {n}, added up (the longest {key}, {longest:.3} s)"), took);
    }
    if runs(&[ShaderCompile, ShaderPair]) {
        timings.time("shaders", || self::shaders(&mut d, &custom));
    }
    if runs(&[ObjectRow]) {
        timings.time("objects' rows", || objects(&mut d, module));
    }
    if runs(&[PcFaction]) {
        timings.time("the PC faction", || pc_faction(&mut d, module));
    }
    if runs(&[StaticTransform]) {
        timings.time("static placeables' transforms", || static_transforms(&mut d, module));
    }
    if runs(&[AreaVariables]) {
        timings.time("areas' variables", || area_variables(&mut d, module));
    }
    if runs(&[TagUnique]) {
        timings.time("unique tags", || unique_tags(&mut d, module));
    }
    if runs(&[ClaimsMerged, ClaimsTag]) {
        timings.time("merged areas", || merged_areas(&mut d, module));
    }
    if runs(&[VarType, VarValue, VarObject, VarName, VarRuntime, VarRequires]) {
        timings.time("described variables", || described_variables(&mut d, module));
    }
    if runs(&[VarScript]) {
        timings.time("variables the scripts read", || variables_read(&mut d, module));
    }
    if runs(&[TwoDaDescribed]) {
        timings.time("described 2DAs", || described_tables(&mut d, module));
    }
    if runs(&[CustomTlk]) {
        custom_tlk(&mut d, module);
    }
    if runs(&[NuiSources]) {
        timings.time("NUI sources", || nui_sources(&mut d, module));
    }
    d.out.sort_by(|a, b| {
        (a.severity, &a.source, a.resource).cmp(&(b.severity, &b.source, b.resource))
    });
    let Doctor { out, learned, .. } = d;
    if let Some(memo) = memo {
        timings.time("what this run found, kept", || memo.save(learned, resman));
    }
    out
}

/// A custom talk table the module names and the game can't find: the game
/// then won't load the module (`tests/engine_tlk.rs`). One found first in
/// the player's own content (a hak userpatch.ini names, the user's
/// `override/`, a Workshop item: the game reads it from there too) is a
/// warning, a player without the same not loading the module.
fn custom_tlk(d: &mut Doctor, module: &Module) {
    let Some(name) = module.custom_tlk().ok().flatten().filter(|n| !n.trim().is_empty()) else {
        return;
    };
    if d.tlk.custom.is_some() {
        if let Some(key) = ResKey::parse(name.trim(), ResType::TLK)
            && let Some(local) = crate::verify::only_local(module, d.rm, &key)
        {
            d.push(
                Severity::Warning,
                Check::CustomTlk,
                "module",
                Module::info_key(),
                "Custom Tlk",
                format!(
                    "the talk table {name:?} is found first in {}: players without it there, in \
                     the module's haks or in their tlk folder can't load the module",
                    local.place()
                ),
            );
        }
        return;
    }
    let hint = crate::talk::name_hint(&name).map(|h| format!(" ({h})")).unwrap_or_default();
    d.push(
        Severity::Error,
        Check::CustomTlk,
        "module",
        Module::info_key(),
        "Custom Tlk",
        format!(
            "the talk table {name:?} isn't in the module's haks, the module or the tlk folder{hint}: \
             the game won't load the module"
        ),
    );
}

/// A NUI window's files that a nasher project's include leaves out (one
/// made with `src/**/*.{nss,json}`): on disk where nasher won't pack them
/// and Moonglow doesn't read them, or the module's, to be written there on
/// the next save. Each is said with the line that takes them in.
fn nui_sources(d: &mut Doctor, module: &Module) {
    let Some(p) = module.project.as_deref() else { return };
    let window = |k: &ResKey| {
        k.restype == ResType::JUI
            || (k.restype == ResType::TXT && module.contains(&ResKey::new(k.resref, ResType::JUI)))
    };
    let held: Vec<ResKey> = module.keys().filter(|k| window(k)).copied().collect();
    let mut files: Vec<std::path::PathBuf> = (p.left_out.iter())
        .filter(|f| {
            let name = f.file_name().unwrap_or_default().to_string_lossy();
            ResKey::from_filename(&name).is_some_and(|k| {
                k.restype == ResType::JUI
                    || (k.restype == ResType::TXT && f.with_extension("jui").is_file())
            })
        })
        .cloned()
        .collect();
    files.extend(p.uncovered(&held));
    files.sort();
    files.dedup();
    let Some(line) = p.include_for(&files) else { return };
    let root = p.root();
    for f in &files {
        let name = f.file_name().unwrap_or_default().to_string_lossy().to_string();
        let Some(key) = ResKey::from_filename(&name) else { continue };
        let shown = f.strip_prefix(root).unwrap_or(f).display().to_string().replace('\\', "/");
        d.push(
            Severity::Warning,
            Check::NuiSources,
            "nasher.cfg",
            key,
            shown,
            format!(
                "nasher.cfg's include ({}) leaves this NUI window's file out: nasher won't pack \
                 it and Moonglow doesn't read it back. Add the line {line} to the target's \
                 sources (Build › Add Left-Out Files to nasher.cfg does)",
                p.target().includes.join(", ")
            ),
        );
    }
}

/// An archive's resources that start past 2 GiB: the game finds them and
/// can't read them (an empty 2DA, a script that doesn't run, a missing
/// model), whatever the archives below hold. One finding per archive.
fn past_read_limit(d: &mut Doctor, layer: &Layer) {
    let keys = layer.container.unreadable();
    let Some(first) = keys.first() else { return };
    let mut names: Vec<String> = keys.iter().take(5).map(ResKey::to_string).collect();
    if keys.len() > names.len() {
        names.push(format!("and {} more", keys.len() - names.len()));
    }
    d.push(
        Severity::Error,
        Check::ErfSize,
        &layer.label,
        *first,
        "",
        format!(
            "{} resource(s) start past 2 GiB into the archive, which the game can't read: {}. \
             Move them to another hak.",
            keys.len(),
            names.join(", ")
        ),
    );
}

/// Tilesets: section counts, models, groups, doors (Aurora's "Range Check
/// Error" and access violations when opening or painting an area).
fn tileset(d: &mut Doctor, source: &str, key: ResKey) {
    let Ok(data) = d.rm.get(&key) else { return };
    let data = data.into_owned();
    tileset_data(d, source, key, &data);
}

/// A tileset being made (in the tileset editor, not yet in a hak): its
/// problems, its models looked for in `resman`.
pub fn examine_tileset(resman: &ResMan, key: ResKey, data: &[u8]) -> Vec<Finding> {
    let mut d = Doctor {
        rm: resman,
        tlk: TalkTables::default(),
        shaders: None,
        memo: None,
        learned: Vec::new(),
        skip: &NOTHING,
        out: Vec::new(),
        tables: HashMap::new(),
    };
    tileset_data(&mut d, "tileset", key, data);
    d.out
}

fn tileset_data(d: &mut Doctor, source: &str, key: ResKey, data: &[u8]) {
    use Severity::*;
    let set = match Tileset::parse(data, Codepage::default()) {
        Ok(s) => s,
        Err(e) => {
            d.push(Error, Check::SetRead, source, key, "", format!("can't be read: {e}"));
            return;
        }
    };
    // Lines that end in a bare LF: the game's server never finishes
    // loading the module (`engine_tileset_holes`).
    if data.contains(&b'\n') && !data.windows(2).any(|w| w == b"\r\n") {
        d.push(
            Error,
            Check::SetLineEnds,
            source,
            key,
            "",
            "its lines end in LF, not CRLF: the game's server never finishes loading a module \
             that uses it (save it with Windows line ends)",
        );
    }
    let declared = |list: &str| {
        set.ini.section(list).and_then(|s| s.int("Count")).unwrap_or(0).max(0) as usize
    };
    // Holes: what the counts call for and the file lacks. The game reads
    // the rest (a tile without its section or a model is drawn as
    // nothing, with no walkmesh; a terrain, crosser or rule is left out).
    const NAMED: usize = 50;
    for name in set.missing_sections.iter().take(NAMED) {
        let what = if name.starts_with("TILE") {
            "the game draws the tile as nothing, with no walkmesh"
        } else {
            "the game leaves it out"
        };
        d.push(
            Warning,
            Check::SetHole,
            source,
            key,
            format!("[{name}]"),
            format!("no [{name}] section: {what}"),
        );
    }
    let modelless: Vec<usize> = (set.tiles.iter().enumerate())
        .filter(|(i, t)| t.is_hole() && set.ini.section(&format!("TILE{i}")).is_some())
        .map(|(i, _)| i)
        .collect();
    for i in modelless.iter().take(NAMED) {
        d.push(
            Warning,
            Check::SetHole,
            source,
            key,
            format!("[TILE{i}] Model"),
            "the tile names no model: the game draws it as nothing, with no walkmesh",
        );
    }
    let more =
        set.missing_sections.len().saturating_sub(NAMED) + modelless.len().saturating_sub(NAMED);
    if more > 0 {
        d.push(Warning, Check::SetHole, source, key, "", format!("and {more} more holes"));
    }
    // Sections past the counts: Aurora's Range Check Error.
    for (list, item) in [("TILES", "TILE"), ("GROUPS", "GROUP")] {
        let n = declared(list);
        let extra: Vec<usize> = set
            .ini
            .sections
            .iter()
            .filter_map(|s| {
                let upper = s.name.to_ascii_uppercase();
                upper.strip_prefix(item)?.parse::<usize>().ok()
            })
            .filter(|i| *i >= n)
            .collect();
        if let Some(first) = extra.iter().min() {
            d.push(
                Error,
                Check::SetCount,
                source,
                key,
                format!("[{list}] Count"),
                format!(
                    "Count is {n} but there are {} more [{item}n] sections from [{item}{first}]; \
                     Aurora fails with a Range Check Error",
                    extra.len()
                ),
            );
        }
    }
    // Each tile's model: none, or one to examine at its first tile. (The
    // models are looked for and examined in parallel, a tileset may have
    // thousands; their findings are in the order of the tiles.)
    let rm = d.rm;
    // (A hole is not a model missing: it is named above.)
    let exists: Vec<bool> =
        set.tiles.par_iter().map(|t| t.is_hole() || model_exists(rm, &t.model)).collect();
    let mut checked_models = HashSet::new();
    let firsts: Vec<(usize, bool)> = (set.tiles.iter().zip(&exists).enumerate())
        .filter(|(_, (tile, exists))| {
            !**exists || checked_models.insert(tile.model.to_ascii_lowercase())
        })
        .map(|(i, (_, exists))| (i, *exists))
        .collect();
    // (Not read when their checks are left out.)
    let models = !d.skip.all(&[Check::TileFaces, Check::MdlRead]);
    let beside = d.beside();
    let examined: Vec<(Vec<Finding>, Learned)> = firsts
        .par_iter()
        .map(|&(i, exists)| {
            let mut m = beside.beside();
            if exists && models {
                tile_model(&mut m, &set.tiles[i].model);
            }
            (m.out, m.learned)
        })
        .collect();
    for (&(i, exists), (found, learned)) in firsts.iter().zip(examined) {
        d.learned.extend(learned);
        if exists {
            d.out.extend(found);
        } else {
            d.push(
                Error,
                Check::SetModel,
                source,
                key,
                format!("[TILE{i}] Model"),
                format!(
                    "model {} isn't in the haks or the game; painting the tile crashes Aurora",
                    set.tiles[i].model
                ),
            );
        }
    }
    let mut door_tiles: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (i, tile) in set.tiles.iter().enumerate() {
        for door in &tile.doors {
            if door.door_type > 0 {
                let tiles = door_tiles.entry(door.door_type).or_default();
                if tiles.last() != Some(&i) {
                    tiles.push(i);
                }
            }
        }
    }
    for (door_type, tiles) in door_tiles {
        if !d.row_has("doortypes", i64::from(door_type), "Model") {
            let mut list: Vec<String> = tiles.iter().take(6).map(|t| format!("TILE{t}")).collect();
            if tiles.len() > 6 {
                list.push(format!("{} more", tiles.len() - 6));
            }
            d.push(
                Warning,
                Check::SetDoor,
                source,
                key,
                format!("door Type={door_type}"),
                format!(
                    "door type {door_type} isn't a doortypes.2da row with a model, so the doors of {} \
                     ({}) show no appearance",
                    if tiles.len() == 1 { "1 tile".to_string() } else { format!("{} tiles", tiles.len()) },
                    list.join(", ")
                ),
            );
        }
    }
    let mut first_tiles: HashMap<u32, (usize, String)> = HashMap::new();
    for (g, group) in set.groups.iter().enumerate() {
        let at = format!("[GROUP{g}]");
        if group.tiles.is_empty() {
            d.push(
                Error,
                Check::SetGroup,
                source,
                key,
                &at,
                format!("group {:?} has no rows or columns", group.name),
            );
            continue;
        }
        match group.tiles[0] {
            None => d.push(
                Error,
                Check::SetGroup,
                source,
                key,
                format!("{at} Tile0"),
                format!(
                    "group {:?}: the first tile can't be -1 (it names the group in the palette)",
                    group.name
                ),
            ),
            Some(t) => {
                if let Some((other, name)) = first_tiles.insert(t, (g, group.name.clone())) {
                    d.push(
                        Error,
                        Check::SetGroup,
                        source,
                        key,
                        format!("{at} Tile0"),
                        format!(
                            "group {:?} starts with tile {t}, as [GROUP{other}] ({name:?}) does; the first tile must be unique",
                            group.name
                        ),
                    );
                }
            }
        }
        for (cell, t) in group.tiles.iter().enumerate() {
            if let Some(t) = t
                && *t as usize >= set.tiles.len()
            {
                d.push(
                    Error,
                    Check::SetGroup,
                    source,
                    key,
                    format!("{at} Tile{cell}"),
                    format!(
                        "group {:?} uses tile {t}, past the {} tiles; Aurora crashes",
                        group.name,
                        set.tiles.len()
                    ),
                );
            }
        }
    }
    if set.general.transition == 0.0 && !set.groups.is_empty() {
        d.push(
            Warning,
            Check::SetTransition,
            source,
            key,
            "[GENERAL] Transition",
            "Transition is 0: groups with raised corners can't be placed",
        );
    }
    // (Groups' tiles past the list and the holes are reported above; a
    // count past a list's last section is a count's.)
    for w in set.warnings.iter() {
        let reported = (w.starts_with("group ") && w.ends_with("does not exist"))
            || w.starts_with("no [")
            || w.starts_with("no Model=");
        if reported {
            continue;
        }
        match w.split_once("] Count=").filter(|(list, _)| list.starts_with('[')) {
            Some((list, _)) => {
                let at = format!("{list}] Count");
                d.push(Warning, Check::SetCount, source, key, at, format!("{w}: none past it"));
            }
            None => d.push(Warning, Check::SetData, source, key, "", w.clone()),
        }
    }
}

fn model_exists(rm: &ResMan, name: &str) -> bool {
    ResRef::from_str(name).is_ok_and(|r| rm.contains(&ResKey::new(r, ResType::MDL)))
}

/// A custom tile model: its size.
fn tile_model(d: &mut Doctor, name: &str) {
    use Severity::*;
    let Ok(r) = ResRef::from_str(name) else { return };
    let key = ResKey::new(r, ResType::MDL);
    // The game's own tiles are taken as they are.
    let Some(layer) = d.rm.find(&key) else { return };
    if !is_custom(&d.rm.layers()[layer]) {
        return;
    }
    let source = d.rm.layers()[layer].label.clone();
    // (One whose hak is left out, or itself, isn't read.)
    if d.skip.source(&source) || d.skip.resource(&key) {
        return;
    }
    // (Walkmeshes reaching past the tile were tried and left out: the
    // game's own tiles have them, out to 10 m. A model kept as text with
    // fewer face rows than that is not read: `faces_over`; nor one an
    // earlier run read, of an archive as it was.)
    let faces = match d.known(layer, &key).and_then(|f| f.faces.clone()) {
        Some(faces) => faces,
        None => {
            let Ok(data) = d.rm.get(&key) else { return };
            let faces = mg_mdl::faces_over(&data, 10_000).map_err(|e| e.to_string());
            d.learn(layer, key, Facts { faces: Some(faces.clone()), ..Facts::default() });
            faces
        }
    };
    let faces = match faces {
        Ok(faces) => faces,
        Err(e) => {
            d.push(Error, Check::MdlRead, &source, key, "", format!("can't be read: {e}"));
            return;
        }
    };
    if let Some(faces) = faces {
        d.push(
            Warning,
            Check::TileFaces,
            &source,
            key,
            "",
            format!("{faces} faces; over 10,000 can crash Aurora when painting the tile"),
        );
    }
}

/// A custom model kept as text (ASCII), by `mg_mdl::lint`: what the game
/// would refuse or crash on is an error, what it would misread a warning
/// (the first few of a model: a hak of thousands may share one habit;
/// not the keywords it skips).
/// Compiled models have nothing to read so.
fn model_text(d: &mut Doctor, source: &str, key: ResKey, layer: usize) {
    // (What an earlier run found, of an archive as it was: not read.)
    let found = match d.known(layer, &key).and_then(|f| f.text.clone()) {
        Some(found) => found,
        None => {
            let found = model_text_found(d.rm, &key);
            if let Some(found) = &found {
                d.learn(layer, key, Facts { text: Some(found.clone()), ..Facts::default() });
            }
            found.flatten()
        }
    };
    for (severity, at, message) in found.into_iter().flatten() {
        d.push(severity, Check::MdlText, source, key, at, message);
    }
}

/// [`model_text`]'s findings of a model: `Some(None)` for a compiled one,
/// `None` for one that can't be read.
fn model_text_found(rm: &ResMan, key: &ResKey) -> Option<Option<Vec<Noted>>> {
    const WARNINGS: usize = 5;
    // (Told by its first bytes: the haks' compiled models are gigabytes.)
    let start = rm.get_start(key, 12).ok()?;
    if mg_mdl::is_binary(&start) {
        return Some(None);
    }
    let data = rm.get(key).ok()?;
    let text = String::from_utf8_lossy(&data);
    let mut warnings = 0;
    let mut out = Vec::new();
    for found in mg_mdl::lint::check(&text) {
        let severity = match found.severity {
            mg_mdl::lint::Severity::Error => Severity::Error,
            mg_mdl::lint::Severity::Warning => Severity::Warning,
            mg_mdl::lint::Severity::Info => continue,
        };
        // (A keyword the game skips does no harm, and old content is
        // full of them: BioWare's own premium modules have a thousand.)
        if severity == Severity::Warning && found.message.starts_with("unknown keyword") {
            continue;
        }
        if severity == Severity::Warning {
            warnings += 1;
            if warnings > WARNINGS {
                continue;
            }
        }
        out.push((severity, format!("line {}", found.line + 1), found.message));
    }
    if warnings > WARNINGS {
        let more = format!("and {} more warnings of this model's text", warnings - WARNINGS);
        out.push((Severity::Warning, String::new(), more));
    }
    Some(Some(out))
}

/// A material: texture names longer than a resource name can be (Aurora's
/// "Pure virtual function called").
fn material(d: &mut Doctor, source: &str, key: ResKey) {
    let Ok(data) = d.rm.get(&key) else { return };
    let mtr = mg_image::mtr::Mtr::parse(&data);
    for (slot, t) in mtr.textures.iter().enumerate() {
        if let Some(t) = t
            && t.len() > 16
        {
            d.push(
                Severity::Error,
                Check::MtrName,
                source,
                key,
                format!("texture{slot}"),
                format!(
                    "texture {t} is {} characters, over the 16 a resource name can have; \
                     Aurora fails with \"Pure virtual function called\"",
                    t.len()
                ),
            );
        }
    }
}

/// The custom content's shader programs that don't compile: those its
/// materials name (`customshaderVS`, `customshaderFS`), its `.shd` files
/// that are programs, and, where it replaces a stock file (an include:
/// `tdod_shaders.hak`'s `inc_lighting`), the stock programs that include
/// it. Compiled as the game's players have them ([`settings`]): at the
/// game's highest settings first, then at each setting that changes what
/// the program is given; a program is reported at the first it fails at,
/// with its first error, and one error found in many programs is one
/// finding. glslang alone judges, as the game's driver does
/// ([`mg_shader::check_assembled`]); with a cache, what compiled before is
/// not compiled again ([`ProgramCache::check`]).
fn shaders(d: &mut Doctor, custom: &BTreeMap<ResKey, usize>) {
    let layers = d.rm.layers();
    // A program, and where it was asked for: source, resource, field.
    let mut wanted: BTreeMap<(String, Stage), (String, ResKey, &str)> = BTreeMap::new();
    let mut replaces_stock = false;
    // Materials that name one program alone, by source: the field and name.
    let mut lone: BTreeMap<String, Vec<(ResKey, &str, String)>> = BTreeMap::new();
    // (Materials and shader files alone are read: the haks' textures and
    // models are most of their size.)
    // (Each read and parsed in parallel, then taken in key order.)
    enum Asks {
        Programs([(Option<String>, Stage, &'static str); 2]),
        Shader(Option<Stage>),
    }
    let rm = d.rm;
    let files: Vec<(&ResKey, &usize)> =
        (custom.iter()).filter(|(k, _)| matches!(k.restype, ResType::MTR | ResType::SHD)).collect();
    let asks: Vec<Option<Asks>> = (files.par_iter())
        .map(|&(key, _)| {
            let data = rm.get(key).ok()?;
            match key.restype {
                ResType::MTR => {
                    let mtr = mg_image::mtr::Mtr::parse(&data);
                    let name = |n: Option<String>| {
                        n.map(|n| n.trim().to_ascii_lowercase())
                            .filter(|n| !n.is_empty() && n != "null")
                    };
                    Some(Asks::Programs([
                        (name(mtr.shader_vs), Stage::Vertex, "customshaderVS"),
                        (name(mtr.shader_fs), Stage::Fragment, "customshaderFS"),
                    ]))
                }
                ResType::SHD => {
                    let name = key.resref.to_lowercase().to_string();
                    Some(Asks::Shader(program_stage(&name, &data)))
                }
                _ => None,
            }
        })
        .collect();
    for ((key, &layer), asks) in files.into_iter().zip(asks) {
        let source = &layers[layer].label;
        let Some(asks) = asks else { continue };
        match asks {
            Asks::Programs(named) => {
                match named {
                    // A pair: the game compiles both (one that doesn't
                    // compile, or isn't there, is drawn magenta).
                    [(Some(_), ..), (Some(_), ..)] => {
                        for (name, stage, field) in named {
                            let name = name.expect("named");
                            wanted.entry((name, stage)).or_insert((source.clone(), *key, field));
                        }
                    }
                    // One alone: the game draws the mesh with its stock
                    // programs (Moonglow likewise; docs/shaders-proposal.md M2).
                    [(Some(n), _, field), (None, ..)] | [(None, ..), (Some(n), _, field)] => {
                        lone.entry(source.clone()).or_default().push((*key, field, n))
                    }
                    _ => {}
                }
            }
            Asks::Shader(Some(stage)) => {
                let name = key.resref.to_lowercase().to_string();
                wanted.entry((name, stage)).or_insert((source.clone(), *key, ""));
            }
            Asks::Shader(None) => {
                replaces_stock |= layers.iter().any(|l| !is_custom(l) && l.container.contains(key))
            }
        }
    }
    let custom_layer = |label: &str| layers.iter().any(|l| l.label == label && is_custom(l));
    // (source, resource, at, message) -> the programs it was found in.
    let mut found: BTreeMap<(String, ResKey, String, String), Vec<String>> = BTreeMap::new();
    // A program's first error: glslang's later ones mostly follow from it.
    let mut record = |e: mg_shader::ShaderError, program: String, asked: (&str, ResKey, &str)| {
        if let Some(dg) = e.diagnostics().into_iter().next() {
            let place = dg.at.as_ref().and_then(|at| {
                let key = ResKey::parse(&at.file.name, ResType::SHD)?;
                custom_layer(&at.file.origin)
                    .then(|| (at.file.origin.clone(), key, format!("line {}", at.line)))
            });
            let (source, resource, at) = place.unwrap_or_else(|| {
                let at = dg.at.as_ref().map_or(asked.2.to_string(), ToString::to_string);
                (asked.0.to_string(), asked.1, at)
            });
            found.entry((source, resource, at, dg.message)).or_default().push(program.clone());
        }
    };
    // (Compiled in parallel, recorded in the programs' order; not at all
    // when shader-compile is left out.)
    let (rm, cache) = (d.rm, d.shaders);
    if d.skip.check(Check::ShaderCompile) {
        wanted.clear();
        replaces_stock = false;
    }
    let compiled: Vec<Option<(mg_shader::ShaderError, &str)>> = (wanted.par_iter())
        .map(|((name, stage), _)| first_failure(cache, |pre| assemble(name, *stage, pre, rm)))
        .collect();
    for (((name, stage), (source, key, field)), error) in wanted.iter().zip(compiled) {
        if let Some((e, setting)) = error {
            record(e, program_label(name, *stage, setting), (source, *key, field));
        }
    }
    if replaces_stock {
        let mut stock: Vec<(String, Stage, ResKey)> = Vec::new();
        for l in layers.iter().filter(|l| l.class == LayerClass::Key) {
            for k in l.container.keys().filter(|k| k.restype == ResType::SHD) {
                let name = k.resref.to_lowercase().to_string();
                let Ok(data) = l.container.read(&k) else { continue };
                if let Some(stage) = program_stage(&name, &data)
                    && !wanted.contains_key(&(name.clone(), stage))
                {
                    stock.push((name, stage, k));
                }
            }
        }
        stock.sort();
        stock.dedup();
        let stock: Vec<_> = stock
            .into_iter()
            .filter(|(name, stage, _)| {
                assemble(name, *stage, &Preamble::default(), rm)
                    .is_ok_and(|a| a.uses_origin(custom_layer))
            })
            .collect();
        let compiled: Vec<_> = (stock.par_iter())
            .map(|(name, stage, _)| first_failure(cache, |pre| assemble(name, *stage, pre, rm)))
            .collect();
        for ((name, stage, key), error) in stock.into_iter().zip(compiled) {
            if let Some((e, setting)) = error {
                let source = d.rm.origin(&key).unwrap_or_default().to_string();
                record(e, program_label(&name, stage, setting), (&source, key, ""));
            }
        }
    }
    for (source, materials) in lone {
        let Some((key, field, name)) = materials.first() else { continue };
        let more = match materials.len() {
            1 => String::new(),
            n => format!(" (and {} more materials)", n - 1),
        };
        d.push(
            Severity::Warning,
            Check::ShaderPair,
            &source,
            *key,
            (*field).to_string(),
            format!(
                "names {field} {name} alone{more}: the game takes a custom shader's two \
                 programs or neither, and draws these with its stock ones"
            ),
        );
    }
    for ((source, resource, at, message), programs) in found {
        let which = match programs.as_slice() {
            [one] => format!("{one} doesn't compile (the game draws it magenta)"),
            [a, b] => format!("{a} and {b} don't compile (the game draws them magenta)"),
            [a, b, rest @ ..] => format!(
                "{a}, {b} and {} more programs don't compile (the game draws them magenta)",
                rest.len()
            ),
            [] => continue,
        };
        d.push(
            Severity::Error,
            Check::ShaderCompile,
            &source,
            resource,
            at,
            format!("{message}: {which}"),
        );
    }
}

/// The settings Verify compiles a program at, with how a finding names
/// each: the game at its highest settings (enhanced lighting, High
/// Quality, 32 lights, keyholing on, a material that may discard:
/// [`Preamble::default`]), then one setting changed at a time, each
/// changing what the program is given (`notes_shaders.md` §A.5, §D.7):
/// Performance, the game's own default quality; Low, where the lighting is
/// per vertex; enhanced lighting off; an opaque material (`NO_DISCARD 1`,
/// `discard` defined away); and the fewest lights the client takes (3: an
/// array indexed past them fails).
pub(crate) fn settings() -> [(Preamble, &'static str); 6] {
    let high = Preamble::default();
    [
        (high.clone(), ""),
        (Preamble { quality: 1, ..high.clone() }, "at Performance shader quality"),
        (
            Preamble { quality: 0, fragment_lighting: false, ..high.clone() },
            "at Low shader quality",
        ),
        (
            Preamble { gamma_correction: false, fragment_lighting: false, ..high.clone() },
            "with enhanced lighting off",
        ),
        (Preamble { no_discard: true, ..high.clone() }, "for an opaque material"),
        (Preamble { max_lights: 3, ..high }, "with 3 lights"),
    ]
}

/// A program at each of [`settings`] in turn, assembled by `assemble`:
/// the first error and the setting it was found at, or `None` where it
/// compiles at all of them. (A missing include fails at the first.)
fn first_failure(
    cache: Option<&ProgramCache>,
    assemble: impl Fn(&Preamble) -> Result<Assembled, mg_shader::ShaderError>,
) -> Option<(mg_shader::ShaderError, &'static str)> {
    for (pre, setting) in settings() {
        let checked = assemble(&pre).and_then(|a| match cache {
            Some(c) => c.check(&a).map(|_| ()),
            None => mg_shader::check_assembled(&a),
        });
        if let Err(e) = checked {
            return Some((e, setting));
        }
    }
    None
}

/// How a finding names a program: `fs_mine (fragment)`, `fs_mine
/// (fragment, at Low shader quality)`.
fn program_label(name: &str, stage: Stage, setting: &str) -> String {
    if setting.is_empty() {
        format!("{name} ({stage})")
    } else {
        format!("{name} ({stage}, {setting})")
    }
}

/// Columns that hold a talk-table string, by table.
pub(crate) const STRREF_COLUMNS: [(&str, &str); 15] = [
    ("appearance", "STRING_REF"),
    ("baseitems", "Name"),
    ("classes", "Name"),
    ("feat", "FEAT"),
    ("genericdoors", "StrRef"),
    ("placeables", "StrRef"),
    ("racialtypes", "Name"),
    ("skills", "Name"),
    ("spells", "Name"),
    ("doortypes", "StringRefGame"),
    ("itempropdef", "Name"),
    ("itempropdef", "GameStrRef"),
    ("itempropdef", "Description"),
    ("itemprops", "StringRef"),
    ("iprp_paramtable", "Name"),
];

/// Columns that name a script the engine runs, by table: a spell's
/// (`spells.2da` ImpactScript), an area of effect's three
/// (`vfx_persistent.2da`), the scripts an effect's state runs on its
/// creature (`statescripts.2da`: charmed, confused, dominated...), a
/// trap's (`traps.2da`: one made by a script, a DM or a player), and a
/// disease's and a poison's (`disease.2da`, `poison.2da`). Each is in
/// nwn.wiki's page of its table as a script's ResRef, and its cells name
/// the game's own scripts (`script_columns_name_the_game_s_scripts`).
/// `areaeffects.2da` names scripts too, none of which the game has: the
/// engine doesn't read it (nwn.wiki's 2da Files: "Unused, see
/// vfx_persistent.2da instead").
pub const SCRIPT_COLUMNS: [(&str, &str); 10] = [
    ("spells", "ImpactScript"),
    ("vfx_persistent", "ONENTER"),
    ("vfx_persistent", "ONEXIT"),
    ("vfx_persistent", "HEARTBEAT"),
    ("statescripts", "SCRIPTNAME"),
    ("traps", "TrapScript"),
    ("disease", "End_Incu_Script"),
    ("disease", "24_Hour_Script"),
    ("poison", "Script_1"),
    ("poison", "Script_2"),
];

/// Columns that name a model, by table: the ones an object's appearance
/// is checked through.
const APPEARANCE_MODEL: (&str, &str) = ("appearance", "RACE");
const PLACEABLE_MODEL: (&str, &str) = ("placeables", "ModelName");
const GENERIC_DOOR_MODEL: (&str, &str) = ("genericdoors", "ModelName");
const DOOR_MODEL: (&str, &str) = ("doortypes", "Model");
const MODEL_COLUMNS: [(&str, &str); 4] =
    [APPEARANCE_MODEL, PLACEABLE_MODEL, GENERIC_DOOR_MODEL, DOOR_MODEL];

/// Columns that name a row of another table (or of their own): the
/// table, the column, the table whose row it is. Each is in nwn.wiki's
/// page of its table as a line reference, and holds for every row of the
/// game's own tables (`row_columns_hold_for_the_game_s_tables`) but
/// `appearance.FOOTSTEPTYPE`, which the game's rows 562 to 568 break
/// (footstep 30 of footstepsounds.2da's 18); `****` and a negative number
/// name none. The tables a class, a race, a package or an item property
/// names in a column are in [`NAMED_TABLES`].
pub(crate) const ROW_COLUMNS: [(&str, &str, &str); 64] = [
    ("appearance", "SOUNDAPPTYPE", "appearancesndset"),
    ("appearance", "FOOTSTEPTYPE", "footstepsounds"),
    ("baseitems", "ReqFeat0", "feat"),
    ("baseitems", "ReqFeat1", "feat"),
    ("baseitems", "ReqFeat2", "feat"),
    ("baseitems", "ReqFeat3", "feat"),
    ("baseitems", "ReqFeat4", "feat"),
    ("baseitems", "WeaponFocusFeat", "feat"),
    ("baseitems", "EpicWeaponFocusFeat", "feat"),
    ("baseitems", "WeaponSpecializationFeat", "feat"),
    ("baseitems", "EpicWeaponSpecializationFeat", "feat"),
    ("baseitems", "WeaponImprovedCriticalFeat", "feat"),
    ("baseitems", "EpicWeaponOverwhelmingCriticalFeat", "feat"),
    ("baseitems", "EpicWeaponDevastatingCriticalFeat", "feat"),
    ("baseitems", "WeaponOfChoiceFeat", "feat"),
    ("classes", "Package", "packages"),
    ("domains", "Level_1", "spells"),
    ("domains", "Level_2", "spells"),
    ("domains", "Level_3", "spells"),
    ("domains", "Level_4", "spells"),
    ("domains", "Level_5", "spells"),
    ("domains", "Level_6", "spells"),
    ("domains", "Level_7", "spells"),
    ("domains", "Level_8", "spells"),
    ("domains", "Level_9", "spells"),
    ("domains", "GrantedFeat", "feat"),
    ("feat", "PREREQFEAT1", "feat"),
    ("feat", "PREREQFEAT2", "feat"),
    ("feat", "OrReqFeat0", "feat"),
    ("feat", "OrReqFeat1", "feat"),
    ("feat", "OrReqFeat2", "feat"),
    ("feat", "OrReqFeat3", "feat"),
    ("feat", "OrReqFeat4", "feat"),
    ("feat", "SUCCESSOR", "feat"),
    ("feat", "MASTERFEAT", "masterfeats"),
    ("feat", "SPELLID", "spells"),
    ("feat", "REQSKILL", "skills"),
    ("feat", "REQSKILL2", "skills"),
    ("feat", "MinLevelClass", "classes"),
    ("genericdoors", "SoundAppType", "placeableobjsnds"),
    ("itempropdef", "CostTableResRef", "iprp_costtable"),
    ("itempropdef", "Param1ResRef", "iprp_paramtable"),
    ("packages", "ClassID", "classes"),
    ("packages", "Domain1", "domains"),
    ("packages", "Domain2", "domains"),
    ("packages", "School", "spellschools"),
    ("placeables", "SoundAppType", "placeableobjsnds"),
    ("placeables", "LightColor", "lightcolor"),
    ("polymorph", "AppearanceType", "appearance"),
    ("polymorph", "RacialType", "racialtypes"),
    ("polymorph", "SPELL1", "spells"),
    ("polymorph", "SPELL2", "spells"),
    ("polymorph", "SPELL3", "spells"),
    ("racialtypes", "Appearance", "appearance"),
    ("racialtypes", "ToolsetDefaultClass", "classes"),
    ("racialtypes", "Favored", "classes"),
    ("racialtypes", "FavoredEnemyFeat", "feat"),
    ("spells", "Master", "spells"),
    ("spells", "SubRadSpell1", "spells"),
    ("spells", "SubRadSpell2", "spells"),
    ("spells", "SubRadSpell3", "spells"),
    ("spells", "SubRadSpell4", "spells"),
    ("spells", "SubRadSpell5", "spells"),
    ("spells", "FeatID", "feat"),
];

/// The tables each `ReqType` of a class's requirements (`cls_pres_*`)
/// names a row of in its `ReqParam1` (nwn.wiki's cls_pres_xxx.2da; held by
/// every row of the game's own: `row_columns_hold_for_the_game_s_tables`):
/// the others take a number, a name or nothing.
pub(crate) const REQUIREMENT_ROWS: [(&str, &str); 6] = [
    ("FEAT", "feat"),
    ("FEATOR", "feat"),
    ("SKILL", "skills"),
    ("RACE", "racialtypes"),
    ("CLASSOR", "classes"),
    ("CLASSNOT", "classes"),
];

/// Tables that a column of another names, and what a column of theirs
/// holds: the naming table and its column, the usual start of such a
/// table's name (none: known only by the column that names it), its
/// column and what it holds. (A class's feats are in the table its
/// `FeatsTable` names, `cls_feat_*` in the game's own; an item
/// property's subtypes, costs and parameters in the tables
/// `itempropdef.2da`, `iprp_costtable.2da` and `iprp_paramtable.2da` name,
/// each with its `Name`, a talk-table string.)
pub(crate) const NAMED_TABLES: [(&str, &str, &str, &str, ColumnKind); 10] = [
    ("classes", "FeatsTable", "cls_feat_", "FeatIndex", ColumnKind::Row("feat")),
    ("classes", "SkillsTable", "cls_skill_", "SkillIndex", ColumnKind::Row("skills")),
    (
        "classes",
        "PreReqTable",
        "cls_pres_",
        "ReqParam1",
        ColumnKind::RowBy { by: "ReqType", tables: &REQUIREMENT_ROWS },
    ),
    ("racialtypes", "FeatsTable", "race_feat_", "FeatIndex", ColumnKind::Row("feat")),
    ("packages", "FeatPref2DA", "packft", "FeatIndex", ColumnKind::Row("feat")),
    ("packages", "SkillPref2DA", "packsk", "SkillIndex", ColumnKind::Row("skills")),
    ("packages", "SpellPref2DA", "packsp", "SpellIndex", ColumnKind::Row("spells")),
    ("itempropdef", "SubTypeResRef", "", "Name", ColumnKind::StrRef),
    ("iprp_costtable", "Name", "", "Name", ColumnKind::StrRef),
    ("iprp_paramtable", "TableResRef", "", "Name", ColumnKind::StrRef),
];

/// Columns that name another table (without `.2da`), by table: a class's
/// lists, a race's, a package's and an item property's. (The tables a
/// class's `BonusFeatsTable`, `SpellGainTable`, `SpellKnownTable`,
/// `SavingThrowTable`, `AttackBonusTable` and `StatGainTable` name hold
/// numbers alone: nothing in them is a row, a string or a model.)
pub(crate) const TABLE_COLUMNS: [(&str, &str); 17] = [
    ("classes", "FeatsTable"),
    ("classes", "SkillsTable"),
    ("classes", "SavingThrowTable"),
    ("classes", "SpellGainTable"),
    ("classes", "SpellKnownTable"),
    ("classes", "PreReqTable"),
    ("classes", "BonusFeatsTable"),
    ("classes", "AttackBonusTable"),
    ("classes", "StatGainTable"),
    ("racialtypes", "FeatsTable"),
    ("packages", "FeatPref2DA"),
    ("packages", "SkillPref2DA"),
    ("packages", "SpellPref2DA"),
    ("packages", "Equip2DA"),
    ("itempropdef", "SubTypeResRef"),
    ("iprp_costtable", "Name"),
    ("iprp_paramtable", "TableResRef"),
];

/// The row a cell of one of [`ROW_COLUMNS`] names; `None`: none. (A
/// spell's `FeatID` carries a radial spell's place above its low 16
/// bits.)
pub fn row_named(table: &str, column: &str, value: i32) -> Option<usize> {
    let row = usize::try_from(value).ok()?;
    let radial = table.eq_ignore_ascii_case("spells") && column.eq_ignore_ascii_case("FeatID");
    Some(if radial { row & 0xFFFF } else { row })
}

/// What a column of a 2DA holds, where the checks know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    /// A talk-table string's number.
    StrRef,
    /// A model's name.
    Model,
    /// A row's number in the table named (without `.2da`).
    Row(&'static str),
    /// A row's number in the table another column's cell chooses: the
    /// column (`by`), and the table each of its values names (`ReqType`'s
    /// `FEAT`, `feat`); another value names no row. A class's requirement's
    /// `ReqParam1`.
    RowBy { by: &'static str, tables: &'static [(&'static str, &'static str)] },
    /// The name of another table, without `.2da` (a class's `FeatsTable`).
    Table,
    /// A script's name, which the engine runs (a spell's `ImpactScript`):
    /// [`SCRIPT_COLUMNS`].
    Script,
}

impl ColumnKind {
    /// The table whose row a cell of this kind names in row `row` of `t`
    /// (for [`ColumnKind::RowBy`], by the row's other cell); `None` for
    /// one that names no row.
    pub fn row_table(self, t: &TwoDa, row: usize) -> Option<&'static str> {
        match self {
            ColumnKind::Row(target) => Some(target),
            ColumnKind::RowBy { by, tables } => {
                let chosen = t.get(row, by)?.trim();
                tables.iter().find(|(v, _)| v.eq_ignore_ascii_case(chosen)).map(|(_, t)| *t)
            }
            _ => None,
        }
    }
}

/// Why a row of `table` (`target`) is not one to name: it isn't there, or
/// it is blank (a padding row: every cell empty). `None`: it is fine.
fn row_fault(table: &TwoDa, target: &str, row: usize) -> Option<String> {
    let length = table.len();
    if row >= length {
        Some(format!("row {row} of {target}.2da doesn't exist: it has {length} rows"))
    } else if (0..table.columns().len()).all(|c| table.cell(row, c).is_none()) {
        Some(format!("row {row} of {target}.2da is blank (a padding row)"))
    } else {
        None
    }
}

/// The columns of the table `name` (without `.2da`) that the checks know
/// the kind of: the same tables Verify Module reads them from. (A table
/// that a class, a race or a package names is known here by how the
/// game's own are named, `cls_feat_*` and the like; Verify Module goes
/// by the column that names it too, [`column_kinds_as_named`], which knows
/// the tables an item property names by it alone.)
pub fn column_kinds(name: &str) -> Vec<(&'static str, ColumnKind)> {
    let of = |list: &'static [(&'static str, &'static str)], kind: ColumnKind| {
        list.iter().filter(|(t, _)| t.eq_ignore_ascii_case(name)).map(move |(_, c)| (*c, kind))
    };
    let rows = ROW_COLUMNS
        .iter()
        .filter(|(t, _, _)| t.eq_ignore_ascii_case(name))
        .map(|(_, c, target)| (*c, ColumnKind::Row(target)));
    let lower = name.to_ascii_lowercase();
    let named = NAMED_TABLES
        .iter()
        .filter(move |(_, _, start, _, _)| !start.is_empty() && lower.starts_with(start))
        .map(|(_, _, _, c, kind)| (*c, *kind));
    of(&STRREF_COLUMNS, ColumnKind::StrRef)
        .chain(of(&MODEL_COLUMNS, ColumnKind::Model))
        .chain(rows)
        .chain(of(&TABLE_COLUMNS, ColumnKind::Table))
        .chain(of(&SCRIPT_COLUMNS, ColumnKind::Script))
        .chain(named)
        .collect()
}

/// Why a talk-table string's number names no string, if it names none.
pub(crate) fn strref_problem(v: u32, tlk: TalkTables) -> Option<String> {
    if v >= 0x0100_0000 {
        match tlk.custom {
            None => Some("the module has no custom talk table".to_string()),
            Some(n) if (v - 0x0100_0000) as usize >= n => {
                Some(format!("the custom talk table has {n} strings"))
            }
            Some(_) => None,
        }
    } else if tlk.base > 0 && v as usize >= tlk.base {
        Some(format!("dialog.tlk has {} strings", tlk.base))
    } else {
        None
    }
}

/// [`column_kinds`], and for a table that a class, a race, a package or
/// an item property names in a column (`table` reads a table as the game
/// has it), whatever its own name, the columns such a table has.
pub fn column_kinds_as_named(
    name: &str,
    table: &dyn Fn(&str) -> Option<std::sync::Arc<TwoDa>>,
) -> Vec<(&'static str, ColumnKind)> {
    let mut kinds = column_kinds(name);
    for (naming, by, _, column, kind) in NAMED_TABLES {
        if kinds.contains(&(column, kind)) {
            continue;
        }
        let named = table(naming).is_some_and(|t| {
            (0..t.len()).any(|r| t.get(r, by).is_some_and(|n| n.eq_ignore_ascii_case(name)))
        });
        if named {
            kinds.push((column, kind));
        }
    }
    kinds
}

/// What reads a table by its name (without `.2da`), as the module sees it.
pub type TableOf<'a> = dyn Fn(&str) -> Option<std::sync::Arc<TwoDa>> + 'a;

/// A cell of a table that is wrong as it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellFault {
    pub row: usize,
    /// The column's place in the table.
    pub column: usize,
    pub message: String,
}

/// What is wrong in the cells of the table `name` (without `.2da`), every
/// row of it, by what its columns hold (`kinds`): a row of another table
/// that is not there or is blank, a table that is not there (`tables`:
/// the tables as the module sees them, none where there is no game; the
/// table itself is asked of itself), a talk-table string that is not
/// there. A table being edited is checked by this as it is typed.
pub fn table_faults(
    name: &str,
    t: &TwoDa,
    kinds: &[(&'static str, ColumnKind)],
    tables: Option<&TableOf>,
    tlk: TalkTables,
) -> Vec<CellFault> {
    let mut out = Vec::new();
    for (column, kind) in kinds {
        let Some(at) = t.column(column) else { continue };
        for row in 0..t.len() {
            let Some(text) = t.cell(row, at) else { continue };
            let message = match kind {
                ColumnKind::StrRef => mg_2da::parse_int(text)
                    .and_then(|v| u32::try_from(v).ok())
                    .and_then(|v| strref_problem(v, tlk).map(|p| (v, p)))
                    .map(|(v, p)| format!("string {v} doesn't exist: {p}")),
                // (Not by the row: the game's own tables name models it
                // never shipped, and an appearance's is a letter for a
                // body of parts. A row's model matters when something
                // uses the row, which Verify Module looks at.)
                // (Nor a script: the game's own spells.2da names 26 it
                // doesn't have, which run nothing.)
                ColumnKind::Model | ColumnKind::Script => None,
                ColumnKind::Row(_) | ColumnKind::RowBy { .. } => {
                    let named = mg_2da::parse_int(text).and_then(|v| row_named(name, column, v));
                    match (kind.row_table(t, row), named) {
                        (Some(target), Some(named)) if target.eq_ignore_ascii_case(name) => {
                            row_fault(t, target, named)
                        }
                        (Some(target), Some(named)) => tables
                            .and_then(|read| read(target))
                            .and_then(|other| row_fault(&other, target, named)),
                        _ => None,
                    }
                }
                ColumnKind::Table => tables.and_then(|read| {
                    let there = text.eq_ignore_ascii_case(name) || read(text.trim()).is_some();
                    (!there).then(|| format!("{text}.2da, which it names, isn't there"))
                }),
            };
            if let Some(message) = message {
                out.push(CellFault { row, column: at, message });
            }
        }
    }
    out.sort_by_key(|f| (f.row, f.column));
    out
}

/// Rows past which Aurora fails, by table.
const AURORA_ROW_LIMITS: [(&str, usize, &str); 2] = [
    ("baseitems", 256, "Aurora fails with an access violation"),
    ("lightcolor", 32, "Tile Properties crashes Aurora"),
];

/// A custom 2DA: malformed rows, rows past Aurora's limits, strings that
/// aren't there, and other haks' copies it hides.
fn two_da(d: &mut Doctor, source: &str, key: ResKey, layer: usize) {
    use Severity::*;
    let Ok(data) = d.rm.get(&key) else { return };
    let data = data.into_owned();
    let name = key.resref.to_string().to_ascii_lowercase();
    // Tables the game has are the ones it and Aurora read; others are read
    // only by scripts, which may keep marker lines in them.
    let game_table = d.rm.layers().iter().any(|l| !is_custom(l) && l.container.contains(&key));
    for (row, cells) in overlong_rows(&data, Codepage::default()).into_iter().filter(|_| game_table)
    {
        d.push(
            Error,
            Check::TwoDaRow,
            source,
            key,
            format!("row {row}"),
            format!("has {cells} cells, more than the table's columns: two rows run together?"),
        );
    }
    let Ok(table) = TwoDa::parse(&data, Codepage::default()) else {
        d.push(Error, Check::TwoDaRead, source, key, "", "can't be read");
        return;
    };
    for (t, limit, why) in AURORA_ROW_LIMITS {
        if name == t && table.len() > limit {
            d.push(
                Warning,
                Check::TwoDaLimit,
                source,
                key,
                format!("row {limit}"),
                format!("{} rows; more than {limit} and {why} (the game is fine)", table.len()),
            );
        }
    }
    // The copies it hides (lower haks, the game's): only rows it adds or
    // changes are checked against them, and a shorter copy hides rows.
    let layers = d.rm.layers();
    let below: Vec<(String, bool, TwoDa)> = layers
        .iter()
        .enumerate()
        .skip(layer + 1)
        .filter_map(|(_, l)| {
            let data = l.container.read(&key).ok()?;
            Some((l.label.clone(), is_custom(l), TwoDa::parse(&data, Codepage::default()).ok()?))
        })
        .collect();
    // Hiding the game's longer copy is common (haks made before EE carry
    // whole old tables) and only matters for rows something uses, which the
    // object checks catch; hiding another hak's is a mistake in hak order.
    if let Some((label, _, longer)) =
        below.iter().find(|(_, custom, t)| *custom && t.len() > table.len())
    {
        d.push(
            Warning,
            Check::TwoDaShadow,
            source,
            key,
            "",
            format!(
                "{} rows, but the copy in {label} it hides has {}: rows {} on are lost (an older copy?)",
                table.len(),
                longer.len(),
                table.len()
            ),
        );
    }
    let base = below.last().map(|(_, _, t)| t);
    let changed = |row: usize| -> bool {
        let Some(base) = base else { return true };
        let mine: Vec<Option<&str>> = table.columns().iter().map(|c| table.get(row, c)).collect();
        let theirs: Vec<Option<&str>> = table.columns().iter().map(|c| base.get(row, c)).collect();
        row >= base.len() || mine != theirs
    };
    // What its columns hold: its own, and those it has as a table that a
    // class, a race, a package or an item property names (by that column,
    // or by its name as the game's own are named).
    let mut kinds = column_kinds(&name);
    for (naming, by, _, column, kind) in NAMED_TABLES {
        if kinds.contains(&(column, kind)) {
            continue;
        }
        let named = d.table(naming).is_some_and(|t| {
            (0..t.len()).any(|r| t.get(r, by).is_some_and(|n| n.eq_ignore_ascii_case(&name)))
        });
        if named {
            kinds.push((column, kind));
        }
    }
    for (column, kind) in kinds {
        if table.column(column).is_none() {
            continue;
        }
        for row in (0..table.len()).filter(|r| changed(*r)) {
            let at = format!("row {row}, {column}");
            match kind {
                ColumnKind::StrRef => {
                    let Some(v) = table.get_int(row, column) else { continue };
                    let Ok(v) = u32::try_from(v) else { continue };
                    if let Some(p) = strref_problem(v, d.tlk) {
                        let message = format!("string {v} doesn't exist: {p} (\"Bad Strref\")");
                        d.push(Error, Check::TwoDaStrRef, source, key, at, message);
                    }
                }
                ColumnKind::Model | ColumnKind::Script => {}
                // Rows of other tables (as the module has them, this
                // hak's and the others' over the game's).
                ColumnKind::Row(_) | ColumnKind::RowBy { .. } => {
                    let Some(target) = kind.row_table(&table, row) else { continue };
                    let Some(named) =
                        table.get_int(row, column).and_then(|v| row_named(&name, column, v))
                    else {
                        continue;
                    };
                    let Some(other) = d.table(target) else { continue };
                    let Some(message) = row_fault(other, target, named) else { continue };
                    // (A cell as the copy below has it, wrong in the game's
                    // own tables too, is the game's: appearance's
                    // FOOTSTEPTYPE 30.)
                    let kept = base.is_some_and(|b| b.get(row, column) == table.get(row, column));
                    if kept
                        && game_copy(d.rm, target)
                            .is_some_and(|g| row_fault(&g, target, named).is_some())
                    {
                        continue;
                    }
                    d.push(Warning, Check::TwoDaRef, source, key, at, message);
                }
                // A table that is not there.
                ColumnKind::Table => {
                    let Some(named) = table.get(row, column) else { continue };
                    let there = ResKey::parse(named.trim(), ResType::TWODA)
                        .is_some_and(|k| d.rm.contains(&k));
                    if !there {
                        let message = format!("{named}.2da, which it names, isn't there");
                        d.push(Warning, Check::TwoDaRef, source, key, at, message);
                    }
                }
            }
        }
    }
}

/// The game's own copy of a table (the lowest, below every custom layer).
pub fn game_copy(rm: &ResMan, name: &str) -> Option<TwoDa> {
    let key = ResKey::parse(name, ResType::TWODA)?;
    let data = rm
        .layers()
        .iter()
        .rev()
        .filter(|l| !is_custom(l))
        .find_map(|l| l.container.read(&key).ok())?;
    TwoDa::parse(&data, Codepage::default()).ok()
}

/// The module's own tables that have a description beside them
/// ([`crate::table_desc`]): each cell that is not what its column is said
/// to hold, and a description that cannot be read.
fn described_tables(d: &mut Doctor, module: &Module) {
    use crate::table_desc::{Beside, Description, path_in_module};
    let mut keys: Vec<ResKey> = module.keys_of(ResType::TWODA).copied().collect();
    keys.sort();
    // (The folders are walked once for the tables of one.)
    let mut walked: Option<(std::path::PathBuf, Beside)> = None;
    for key in keys {
        let Some(path) = path_in_module(module, &key).filter(|p| p.is_file()) else { continue };
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let Some(table) =
            module.get(&key).and_then(|data| TwoDa::parse(data, Codepage::default()).ok())
        else {
            continue;
        };
        let root = Beside::root_of(&path.with_extension(""));
        if walked.as_ref().is_none_or(|(r, _)| *r != root) {
            walked = Some((root.clone(), Beside::of(&root)));
        }
        let files = &walked.as_ref().expect("walked").1;
        let rm = d.rm;
        let from_game = |name: &str, t: ResType| {
            let key = ResKey::parse(name, t)?;
            rm.get(&key).ok().map(|d| d.into_owned())
        };
        let script = |name: &str| files.script(name).or_else(|| from_game(name, ResType::NSS));
        let rows_of = |name: &str| {
            files.rows_of(name, Codepage::default()).or_else(|| {
                let data = from_game(name, ResType::TWODA)?;
                TwoDa::parse(&data, Codepage::default()).ok().map(|t| t.len())
            })
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                let message = format!("its description ({file}) can't be read: {e}");
                d.push(Severity::Warning, Check::TwoDaDescribed, "module", key, "", message);
                continue;
            }
        };
        match Description::parse(&text, &script) {
            Err(e) => {
                let message = format!("its description ({file}) can't be read: {e}");
                d.push(Severity::Warning, Check::TwoDaDescribed, "module", key, "", message);
            }
            Ok(description) => {
                for f in description.faults(&table, &rows_of, d.tlk) {
                    let at = format!("row {}, {}", f.row, table.columns()[f.column]);
                    d.push(Severity::Warning, Check::TwoDaDescribed, "module", key, at, f.message);
                }
            }
        }
    }
}

/// The module's objects: 2DA rows they name that don't exist (a creature
/// with a class that isn't there crashes Aurora and the game). A
/// waypoint's (its flag, which the toolset alone draws) is a warning
/// wherever it is.
fn objects(d: &mut Doctor, module: &Module) {
    let mut keys: Vec<ResKey> = module
        .keys()
        .filter(|k| {
            matches!(
                k.restype,
                ResType::UTC
                    | ResType::UTP
                    | ResType::UTI
                    | ResType::UTD
                    | ResType::UTM
                    | ResType::UTW
                    | ResType::GIT
            )
        })
        .copied()
        .collect();
    keys.sort();
    for key in keys {
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        let mut found = Vec::new();
        let kind = match key.restype {
            ResType::GIT => None,
            t => Some(t),
        };
        walk_objects(d, &gff.root, kind, "", &mut found);
        // Placed, it breaks the area (Aurora's area view crashes on it); a
        // blueprint breaks only when placed, and a waypoint's flag nothing
        // the game shows.
        for (path, message) in found {
            let waypoint = path.starts_with("/WaypointList[") || key.restype == ResType::UTW;
            let severity = match key.restype == ResType::GIT && !waypoint {
                true => Severity::Error,
                false => Severity::Warning,
            };
            let place = crate::rename::describe(key, Some(&gff), &path);
            d.push(severity, Check::ObjectRow, "module", key, place, message);
        }
    }
}

/// Creatures, placeables, doors and encounters (blueprints and placed) of
/// faction 0, the PC faction: the players' alone (Jasperre's report). The
/// game puts a creature of it in Hostile as it loads; a placeable or door
/// keeps it, and creatures regard it as they regard players
/// (`engine_factions.rs`). An object with no faction field is of it too
/// (the game reads none as 0), but for an encounter, whose reading was not
/// measured. Triggers are left out (the owner's decision): Aurora writes
/// 0 for a trigger whose blueprint has no faction, so the warning would
/// fall on ordinary triggers. The rule is `mg_rules::objects::pc_faction`.
fn pc_faction(d: &mut Doctor, module: &Module) {
    use mg_rules::ObjectKind;
    let of_pc = |kind: ObjectKind, s: &Struct| {
        mg_rules::objects::pc_faction(kind, s).map(|field| (field, s.get(field).is_none()))
    };
    let mut keys: Vec<ResKey> = module
        .keys()
        .filter(|k| {
            matches!(
                k.restype,
                ResType::UTC | ResType::UTP | ResType::UTD | ResType::UTE | ResType::GIT
            )
        })
        .copied()
        .collect();
    keys.sort();
    for key in keys {
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        let mut found = Vec::new();
        if key.restype == ResType::GIT {
            for f in &gff.root.fields {
                let label = f.label.to_string_lossy();
                let (Value::List(objects), Some(kind)) = (&f.value, ObjectKind::from_list(&label))
                else {
                    continue;
                };
                for (i, s) in objects.iter().enumerate() {
                    if let Some((field, none)) = of_pc(kind, s) {
                        found.push((kind, format!("/{label}[{i}]/{field}"), none));
                    }
                }
            }
        } else if let Some(kind) = ObjectKind::from_restype(key.restype)
            && let Some((field, none)) = of_pc(kind, &gff.root)
        {
            found.push((kind, format!("/{field}"), none));
        }
        for (kind, path, none) in found {
            let place = crate::rename::describe(key, Some(&gff), &path);
            let what = match kind {
                ObjectKind::Creature => "the game puts a creature of it in Hostile",
                ObjectKind::Encounter => "an encounter's own faction",
                _ => "creatures regard it as they regard players",
            };
            let of = match none {
                true => "has no faction, which the game reads as the PC faction (0)",
                false => "of the PC faction (0)",
            };
            let message =
                format!("{of}, which is the players' alone ({what}): give it another faction");
            d.push(Severity::Warning, Check::PcFaction, "module", key, place, message);
        }
    }
}

/// Static placeables with a visual transform that changes their model:
/// the server keeps it (a script reads it back), but the game client
/// draws a static placeable without it, merged into the area's tiles
/// (`visual_transforms_look` and `placeables_look` in `client_render.rs`).
/// Aurora takes a placeable's transform off when Static is ticked, and so
/// do Moonglow's Properties and Make Static: such a file was made
/// elsewhere (another tool, by hand, or an older Moonglow).
fn static_transforms(d: &mut Doctor, module: &Module) {
    let mut keys: Vec<ResKey> =
        module.keys().filter(|k| k.restype == ResType::GIT).copied().collect();
    keys.sort();
    for key in keys {
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        let list = gff.root.list("Placeable List").unwrap_or(&[]);
        for (i, s) in list.iter().enumerate() {
            if s.integer("Static").unwrap_or(0) == 0 || !crate::areas::transforms(s) {
                continue;
            }
            let place =
                crate::rename::describe(key, Some(&gff), &format!("/Placeable List[{i}]/Static"));
            let message = "static, so the game draws it without its visual transform (its scale, \
                           rotation and translation): untick Static (Properties) to have it drawn \
                           transformed (ticking it again takes the transform off, as Aurora does)"
                .to_string();
            d.push(Severity::Warning, Check::StaticTransform, "module", key, place, message);
        }
    }
}

/// Areas with variables in their ARE: the game reads an area's from its
/// GIT's root and none from the ARE (`engine_area_variables`), where
/// Moonglow wrote them before 1.34.3, so a script read 0 for them.
fn area_variables(d: &mut Doctor, module: &Module) {
    let mut keys: Vec<ResKey> =
        module.keys().filter(|k| k.restype == ResType::ARE).copied().collect();
    keys.sort();
    for key in keys {
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        let Some(vars) = gff.root.list("VarTable").filter(|v| !v.is_empty()) else { continue };
        let names: Vec<String> = vars
            .iter()
            .filter_map(|v| v.string("Name"))
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .collect();
        let message = format!(
            "{} variable{} in the area's ARE, which the game does not read ({}): open the \
             area's Variables (Area Properties, Advanced) to move them to its GIT",
            names.len(),
            if names.len() == 1 { "" } else { "s" },
            names.join(", ")
        );
        d.push(Severity::Error, Check::AreaVariables, "module", key, "VarTable", message);
    }
}

/// Tags the project says must be unique (`[tags] unique` in its
/// `moonglow.toml`) that are on more than one object: `GetObjectByTag`
/// gives a script only one of them (the one placed later, of two in an
/// area: `engine_static_placeables`).
fn unique_tags(d: &mut Doctor, module: &Module) {
    let rules = &module.project_file.tags;
    if !rules.any_unique() {
        return;
    }
    let index = crate::tags::TagIndex::of_module(module);
    for b in index.breaches(rules) {
        let tag = String::from_utf8_lossy(b.tag);
        let (n, pattern) = (b.places.len(), b.pattern);
        let message = match b.scope {
            crate::project_file::Scope::Module => format!(
                "{tag} is on {n} objects, and the project's moonglow.toml wants {pattern} \
                 unique: a script finds only one of them by its tag"
            ),
            crate::project_file::Scope::Area => format!(
                "{tag} is on {n} objects in {}, and the project's moonglow.toml wants {pattern} \
                 unique in each area: a script that looks for it there finds only one of them",
                b.places[0].area()
            ),
        };
        let at: Vec<String> = b.places.iter().map(|p| p.describe()).collect();
        d.push(
            Severity::Warning,
            Check::TagUnique,
            "module",
            b.places[0].key(),
            at.join(", "),
            message,
        );
    }
}

/// The areas the project's claims table says are merged (in the game):
/// still in the module under their ResRef (the game keeps players' first
/// visits by it), and with the Tag the table records (their positions).
fn merged_areas(d: &mut Doctor, module: &Module) {
    let pf = &module.project_file;
    let Some(table) = &pf.claim_table else { return };
    let file = pf.claims.file.as_deref().unwrap_or("the claims table");
    for c in table.rows.iter().filter(|c| c.merged()) {
        let key = ResKey::new(c.area, ResType::ARE);
        let at = format!("{file}, row {}", c.row);
        match module.gff(&key) {
            None => d.push(
                Severity::Error,
                Check::ClaimsMerged,
                "module",
                key,
                at,
                format!(
                    "{} is merged ({file}), and the module has no area of that ResRef: the game \
                     keeps players' first visits by it, and an area renamed is another area to it",
                    c.area
                ),
            ),
            Some(Ok(are)) => {
                let now = String::from_utf8_lossy(are.root.string("Tag").unwrap_or_default());
                if let Some(want) = c.tag.as_deref().filter(|w| *w != now) {
                    d.push(
                        Severity::Error,
                        Check::ClaimsTag,
                        "module",
                        key,
                        at,
                        format!(
                            "{}'s Tag is {now}, and it was merged as {want} ({file}): a \
                             character's position is kept by the area's Tag",
                            c.area
                        ),
                    );
                }
            }
            Some(Err(_)) => {}
        }
    }
}

/// The module's variables held to the project's descriptions
/// (`variables.toml`): the type each is of (a variable read as another
/// type is that type's empty value, `engine_local_variables`), its value,
/// what it is on, and names that look like a described one's but are
/// not (in another case, which the game tells apart, or named as the
/// project's are and described nowhere).
fn described_variables(d: &mut Doctor, module: &Module) {
    use crate::var_desc::{self, Choices, ModuleLookup, Unknown, type_label};
    let vars = &module.project_file.variables;
    if vars.is_empty() {
        return;
    }
    let file = vars.file();
    let lookup = ModuleLookup::new(module, Some(d.rm));
    let mut choices: HashMap<String, Choices> = HashMap::new();
    let mut unread: HashSet<String> = HashSet::new();
    let mut keys: Vec<ResKey> = module
        .keys()
        .filter(|k| var_desc::Owner::of_restype(k.restype).is_some())
        .copied()
        .collect();
    keys.sort();
    let empty = |t: crate::vars::VarType| match t {
        crate::vars::VarType::Int => "0",
        crate::vars::VarType::Float => "0.0",
        crate::vars::VarType::String => "\"\"",
        crate::vars::VarType::Object => "OBJECT_INVALID",
        crate::vars::VarType::Location => "an invalid location",
    };
    for key in keys {
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        let found = var_desc::variables_of(key, &gff);
        // What each object (by its path) has without what one of its
        // variables needs beside it.
        let mut objects: Vec<&str> = found.iter().map(|f| f.path.as_str()).collect();
        objects.dedup();
        for path in objects {
            let here: Vec<(&str, &str)> = (found.iter())
                .filter(|f| f.path == path)
                .map(|f| (f.name.as_str(), f.value.as_str()))
                .collect();
            for n in vars.missing(&here) {
                let place = crate::rename::describe(key, Some(&gff), &format!("{path}/VarTable"));
                let why = match &n.is {
                    Some(is) => format!("where {} is {is}", n.by),
                    None => "beside it".to_string(),
                };
                let message = format!(
                    "{} is set here and {} isn't: {file}:{} says it is needed {why}",
                    n.by, n.require, n.line
                );
                let at = format!("{place} › {}", n.by);
                d.push(Severity::Warning, Check::VarRequires, "module", key, at, message);
            }
        }
        for f in found {
            let place = crate::rename::describe(key, Some(&gff), &format!("{}/VarTable", f.path));
            let at = format!("{place} › {}", f.name);
            let Some(desc) = vars.describe(&f.name) else {
                let message = match vars.unknown(&f.name) {
                    Some(Unknown::OtherCase(o)) => format!(
                        "{} is described as {} ({file}:{}): the game tells names apart by \
                         their case, and a script that reads {} finds nothing here",
                        f.name, o.name, o.line, o.name
                    ),
                    Some(Unknown::Undescribed { pattern, nearest }) => {
                        let near = nearest.map(|n| format!(": {n}?")).unwrap_or_default();
                        format!(
                            "{} is named as the project's variables are ({pattern}), and {file} \
                             describes no such variable{near}",
                            f.name
                        )
                    }
                    None => continue,
                };
                d.push(Severity::Warning, Check::VarName, "module", key, at, message);
                continue;
            };
            // One the scripts alone set, kept here: it is there from the
            // start, on every server (a test's hook fixing every roll).
            if desc.runtime {
                let how = if desc.is_pattern() {
                    format!(" ({})", desc.quoted_for(&f.name))
                } else {
                    String::new()
                };
                // (A typo of a name of its own entry, a builder's, that
                // one of the scripts' patterns takes: said as var-name
                // says one.)
                let near = vars
                    .runtime_typo(&f.name)
                    .map(|n| format!(": {}? ({file}:{})", n.name, n.line))
                    .unwrap_or_default();
                let message = format!(
                    "{}{how} is set by the scripts while the server runs ({file}:{} says runtime \
                     = true), and is never to be kept in a file: one kept here is there from the \
                     start, on every server{near}",
                    f.name, desc.line
                );
                d.push(Severity::Error, Check::VarRuntime, "module", key, at, message);
                continue;
            }
            if let (Some(want), Some(have)) = (desc.var_type, f.var_type)
                && want != have
            {
                let message = format!(
                    "{} is {} {} here, and {file}:{} says it is {} {}: the scripts read {}",
                    f.name,
                    if have == crate::vars::VarType::Int { "an" } else { "a" },
                    type_label(have),
                    desc.line,
                    if want == crate::vars::VarType::Int { "an" } else { "a" },
                    type_label(want),
                    empty(want)
                );
                d.push(Severity::Error, Check::VarType, "module", key, at, message);
                continue;
            }
            if !desc.belongs_on(f.owner) {
                let on: Vec<&str> = desc.on.iter().map(|o| o.name()).collect();
                let message = format!(
                    "{} is on {} {}, and {file}:{} says it goes on {}",
                    f.name,
                    if f.owner.name().starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" },
                    f.owner.name(),
                    desc.line,
                    on.join(", ")
                );
                d.push(Severity::Warning, Check::VarObject, "module", key, at.clone(), message);
            }
            let typed = matches!(
                f.var_type,
                Some(
                    crate::vars::VarType::Int
                        | crate::vars::VarType::Float
                        | crate::vars::VarType::String
                )
            );
            if !typed || !(desc.has_list() || desc.min.is_some() || desc.max.is_some()) {
                continue;
            }
            let c = choices.entry(desc.name.clone()).or_insert_with(|| desc.choices(&lookup));
            if let Some(problem) = &c.problem
                && unread.insert(desc.name.clone())
            {
                let message = format!(
                    "{}'s values ({file}:{}) can't be listed: {problem}",
                    desc.name, desc.line
                );
                d.push(Severity::Warning, Check::VarValue, "module", key, at.clone(), message);
            }
            if let Some(problem) = desc.value_problem(&f.value, c) {
                let message = format!("{}: {problem} ({file}:{})", f.name, desc.line);
                d.push(Severity::Warning, Check::VarValue, "module", key, at, message);
            }
        }
    }
}

/// Variables kept in the module's files as another type than every one of
/// its scripts reads (or sets) them as, by their names written out: the
/// script reads that type's empty value (`engine_local_variables`). A
/// variable the project describes with a type is held to that
/// (`var-type`) instead; one no script names by its name is left.
fn variables_read(d: &mut Doctor, module: &Module) {
    use crate::var_desc::{self, type_label};
    let reads = var_desc::script_reads(module);
    if reads.is_empty() {
        return;
    }
    let vars = &module.project_file.variables;
    let empty = |t: crate::vars::VarType| match t {
        crate::vars::VarType::Int => "0",
        crate::vars::VarType::Float => "0.0",
        crate::vars::VarType::String => "\"\"",
        crate::vars::VarType::Object => "OBJECT_INVALID",
        crate::vars::VarType::Location => "an invalid location",
    };
    let article = |t: crate::vars::VarType| match t {
        crate::vars::VarType::Int => "an",
        _ => "a",
    };
    let mut keys: Vec<ResKey> = module
        .keys()
        .filter(|k| var_desc::Owner::of_restype(k.restype).is_some())
        .copied()
        .collect();
    keys.sort();
    for key in keys {
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        for f in var_desc::variables_of(key, &gff) {
            let (Some(have), Some(read)) = (f.var_type, reads.get(&f.name)) else { continue };
            if read.iter().any(|r| r.var_type == have)
                || vars.describe(&f.name).is_some_and(|x| x.var_type.is_some() || x.runtime)
            {
                continue;
            }
            let first = &read[0];
            let more = match read.len() {
                1 => String::new(),
                n => format!(" and {} more", n - 1),
            };
            let place = crate::rename::describe(key, Some(&gff), &format!("{}/VarTable", f.path));
            let message = format!(
                "{} is {} {} here, and the scripts read it as {} {} ({} in {}.nss line {}{more}): \
                 they read {}",
                f.name,
                article(have),
                type_label(have),
                article(first.var_type),
                type_label(first.var_type),
                first.function,
                first.script.resref,
                first.line,
                empty(first.var_type)
            );
            let at = format!("{place} › {}", f.name);
            d.push(Severity::Warning, Check::VarScript, "module", key, at, message);
        }
    }
}

/// The object type a GIT list or inventory holds (of these,
/// [`walk_objects`] checks creatures, doors, placeables and items, and
/// the items stores and the others hold).
fn list_type(list: &str) -> Option<ResType> {
    Some(match list {
        "ItemList" | "Equip_ItemList" => ResType::UTI,
        l => mg_rules::ObjectKind::from_list(l)?.restype(),
    })
}

fn walk_objects(
    d: &mut Doctor,
    s: &Struct,
    kind: Option<ResType>,
    path: &str,
    out: &mut Vec<(String, String)>,
) {
    // The row the object's appearance names must exist, and the model its
    // column names too (a missing one crashes Aurora's area view), as the
    // previews and the Properties' warnings judge it
    // (`mg_rules::appearance`).
    let object = kind.and_then(mg_rules::ObjectKind::from_restype);
    if let Some(at) = object.and_then(|k| mg_rules::appearance::AppearanceRow::of(k, s)) {
        let rm = d.rm;
        let table = d.table(at.table);
        // (A waypoint's flag is the toolset's alone: judged only where
        // there is a waypoint.2da to judge it by, a game.)
        let unjudged = object == Some(mg_rules::ObjectKind::Waypoint) && table.is_none();
        if let Err(e) = at.model(table, rm)
            && !unjudged
        {
            out.push((format!("{path}/{}", at.field), e.to_string()));
        }
    }
    if kind == Some(ResType::UTC)
        && let Some(Value::List(classes)) = s.get("ClassList")
    {
        for (i, c) in classes.iter().enumerate() {
            if let Some(Value::Int(v)) = c.get("Class")
                && !d.row_has("classes", i64::from(*v), "Label")
            {
                out.push((
                    format!("{path}/ClassList[{i}]/Class"),
                    format!("class {v} isn't a classes.2da row; Aurora and the game crash"),
                ));
            }
        }
    }
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        if let Value::List(items) = &f.value
            && let Some(t) = list_type(&label)
        {
            for (i, item) in items.iter().enumerate() {
                walk_objects(d, item, Some(t), &format!("{path}/{label}[{i}]"), out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mg_gff::Gff;
    use mg_resman::{LayerClass, MemContainer};

    use super::*;

    fn key(n: &str, t: ResType) -> ResKey {
        ResKey::new(ResRef::from_str(n).unwrap(), t)
    }

    /// A game layer with a placeables.2da and a model, and a hak over it.
    fn resman(hak: &[(&str, ResType, &[u8])]) -> ResMan {
        let mut game = MemContainer::new();
        game.insert(
            key("placeables", ResType::TWODA),
            b"2DA V2.0\n\n   Label ModelName StrRef\n0  Chair plc_chair 5\n".to_vec(),
        );
        game.insert(key("plc_chair", ResType::MDL), b"x".to_vec());
        game.insert(
            key("doortypes", ResType::TWODA),
            b"2DA V2.0\n\n   Label Model\n0 None ****\n1 Door d1\n".to_vec(),
        );
        let mut h = MemContainer::new();
        for (n, t, data) in hak {
            h.insert(key(n, *t), data.to_vec());
        }
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, game);
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, h);
        rm
    }

    fn checks(f: &[Finding]) -> Vec<(&str, String)> {
        f.iter().map(|f| (f.check.as_ref(), f.at.clone())).collect()
    }

    /// A program that compiles at the game's highest settings and not at
    /// another its players may have is reported at that one, once (the
    /// first it fails at); one that fails at all of them is reported as
    /// before. With a cache, a second run compiles only what failed, and
    /// says the same.
    #[test]
    fn shader_programs_are_compiled_at_the_players_settings() {
        let shd = |t: &str| t.as_bytes().to_vec();
        let mut hak = MemContainer::new();
        // Each a fragment program, broken under one setting.
        let programs = [
            ("fs_low", "#if SHADER_QUALITY_MODE == 0\nx = 1;\n#endif"),
            ("fs_perf", "#if SHADER_QUALITY_MODE == 1\nx = 1;\n#endif"),
            ("fs_vertex", "#if GAMMA_CORRECTION == 0\nx = 1;\n#endif"),
            ("fs_opaque", "#if NO_DISCARD\nx = 1;\n#endif"),
            ("fs_lights", "uniform vec4 l[MAX_NUM_LIGHTS];\nvec4 L() { return l[4]; }"),
            ("fs_always", "x = 1;"),
            ("fs_fine", ""),
        ];
        for (name, body) in programs {
            hak.insert(
                key(name, ResType::SHD),
                shd(&format!(
                    "#define SHADER_TYPE 2\n{body}\nvoid main() {{ gl_FragColor = vec4(1.0); }}\n"
                )),
            );
        }
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let dir = std::env::temp_dir().join(format!("mg-doctor-shaders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = Cache::new(&dir);
        let run = |cache: Option<&Cache>| {
            let f = examine_groups(
                &Module::new(),
                &rm,
                TalkTables::default(),
                &Skip::NONE,
                &mut Timings::default(),
                cache,
            );
            let mut got: Vec<(String, String)> = (f.iter())
                .filter(|f| f.check == "shader-compile")
                .map(|f| (f.resource.to_string(), f.message.clone()))
                .collect();
            got.sort();
            got
        };
        let got = run(None);
        let says = |name: &str| {
            got.iter()
                .find(|(r, _)| *r == format!("{name}.shd"))
                .map(|(_, m)| m.as_str())
                .unwrap_or_else(|| panic!("{name}: {got:#?}"))
        };
        assert_eq!(got.len(), 6, "{got:#?}");
        assert!(says("fs_low").contains("fs_low (fragment, at Low shader quality) doesn't"));
        assert!(says("fs_perf").contains("(fragment, at Performance shader quality)"));
        assert!(says("fs_vertex").contains("(fragment, with enhanced lighting off)"));
        assert!(says("fs_opaque").contains("(fragment, for an opaque material)"));
        assert!(says("fs_lights").contains("(fragment, with 3 lights)"), "{}", says("fs_lights"));
        assert!(
            says("fs_always").contains("fs_always (fragment) doesn't"),
            "{}",
            says("fs_always")
        );
        // Kept: each that compiled at a setting; a second run says the same.
        assert_eq!(run(Some(&cache)), got);
        let kept = std::fs::read_dir(dir.join("shaders")).unwrap().count();
        // (fs_fine at all six, fs_always at none, each other at the
        // settings before the one it fails at.)
        assert_eq!(kept, 6 + 2 + 1 + 3 + 4 + 5, "{kept}");
        assert_eq!(run(Some(&cache)), got);
        assert_eq!(std::fs::read_dir(dir.join("shaders")).unwrap().count(), kept);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn shader_programs_that_do_not_compile() {
        let mut game = MemContainer::new();
        let shd = |t: &str| t.as_bytes().to_vec();
        game.insert(key("inc_x", ResType::SHD), shd("float F() { return 1.0; }\n"));
        game.insert(
            key("fs_stock", ResType::SHD),
            shd("#define SHADER_TYPE 2\n#include \"inc_x\"\nvoid main() { gl_FragColor = vec4(F()); }\n"),
        );
        game.insert(
            key("fs_other", ResType::SHD),
            shd("#define SHADER_TYPE 2\nvoid main() { gl_FragColor = vec4(1.0); }\n"),
        );
        let mut hak = MemContainer::new();
        hak.insert(
            key("my_mat", ResType::MTR),
            b"customshaderVS vs_ok\ncustomshaderFS fs_mine\n".to_vec(),
        );
        hak.insert(
            key("fs_mine", ResType::SHD),
            shd("#define SHADER_TYPE 2\nvoid main()\n{ gl_FragColor = vec4(nope); }\n"),
        );
        hak.insert(
            key("vs_ok", ResType::SHD),
            shd("#define SHADER_TYPE 1\nattribute vec4 vPos;\nvoid main() { gl_Position = vPos; }\n"),
        );
        // Replaces the game's include: the stock program that has it breaks.
        hak.insert(key("inc_x", ResType::SHD), shd("\nfloat F() { return missing; }\n"));
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, game);
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let f: Vec<_> = f.iter().filter(|f| f.check == "shader-compile").collect();
        let got: Vec<_> =
            f.iter().map(|f| (f.source.as_str(), f.resource.to_string(), f.at.as_str())).collect();
        assert_eq!(
            got,
            [
                ("my.hak", "fs_mine.shd".to_string(), "line 3"),
                ("my.hak", "inc_x.shd".into(), "line 2")
            ],
            "{f:#?}"
        );
        assert!(f[0].message.contains("'nope'") && f[0].message.contains("fs_mine (fragment)"));
        assert!(f[1].message.contains("fs_stock (fragment) doesn't compile"), "{}", f[1].message);
        // A material naming a shader that isn't there.
        let mut hak = MemContainer::new();
        hak.insert(key("lost", ResType::MTR), b"customshaderFS fs_gone\n".to_vec());
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        assert_eq!(checks(&f), [("shader-pair", "customshaderFS".to_string())]);
        assert!(f[0].message.contains("fs_gone alone"), "{}", f[0].message);
        // A pair naming one that isn't there: magenta in the game.
        let mut hak = MemContainer::new();
        hak.insert(
            key("lost", ResType::MTR),
            b"customshaderVS vs_gone\ncustomshaderFS fs_gone\n".to_vec(),
        );
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let got = checks(&f);
        assert!(got.iter().all(|(c, _)| *c == "shader-compile") && got.len() == 2, "{got:?}");
        // Materials that name a program alone (The Domains of Dread's
        // `customshadervs fslit_nm`): one warning for the hak, not compiled
        // for the stage named.
        let mut hak = MemContainer::new();
        for n in ["a_feet", "a_shin", "a_hand"] {
            hak.insert(key(n, ResType::MTR), b"customshadervs fslit_nm\n".to_vec());
        }
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        assert_eq!(checks(&f), [("shader-pair", "customshaderVS".to_string())]);
        assert!(f[0].message.contains("fslit_nm alone (and 2 more materials)"), "{}", f[0].message);
        assert_eq!(f[0].severity, Severity::Warning);
    }

    /// A nasher project made with `src/**/*.{nss,json}` (The Domains of
    /// Dread's was): a NUI window's files there are left out, and one the
    /// module holds would be written where they are; each is a warning with
    /// the line that takes them in. Once the line is there, nothing.
    #[test]
    fn a_window_s_files_a_nasher_include_leaves_out() {
        let root = std::env::temp_dir().join(format!("mg-doctor-nui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        let cfg = "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
                   [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n";
        std::fs::write(root.join("nasher.cfg"), cfg).unwrap();
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        m.save_as(&crate::ModuleLocation::Project { root: root.clone(), target: "m".into() })
            .unwrap();
        std::fs::write(root.join("src/crier.jui"), b"{}").unwrap();
        std::fs::write(root.join("src/crier.txt"), b"{}").unwrap();
        std::fs::write(root.join("src/notes.txt"), b"mine").unwrap();
        let mut m = Module::open(&root).unwrap();
        m.set(key("shop", ResType::JUI), b"{}".to_vec());
        let found: Vec<(String, String)> = examine(&m, &ResMan::new(), TalkTables::default())
            .into_iter()
            .filter(|f| f.check == "nui-sources")
            .map(|f| (f.at, f.message))
            .collect();
        let mut at: Vec<&str> = found.iter().map(|(at, _)| at.as_str()).collect();
        at.sort();
        assert_eq!(at, ["src/crier.jui", "src/crier.txt", "src/shop.jui"]);
        assert!(found[0].1.contains("include = \"src/**/*.{jui,txt}\""), "{}", found[0].1);
        // Taken in: nothing to say.
        std::fs::write(root.join("nasher.cfg"), cfg.replace("{nss,json}", "{nss,json,jui,txt}"))
            .unwrap();
        let m = Module::open(&root).unwrap();
        let f = examine(&m, &ResMan::new(), TalkTables::default());
        assert!(f.iter().all(|f| f.check != "nui-sources"), "{f:?}");
        assert_eq!(Group::of(&found_finding()), Group::Module);
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn found_finding() -> Finding {
        Finding {
            severity: Severity::Warning,
            check: Cow::Borrowed(Check::NuiSources.id()),
            source: "nasher.cfg".into(),
            resource: key("crier", ResType::JUI),
            at: String::new(),
            message: String::new(),
        }
    }

    #[test]
    fn each_check_has_an_id_of_its_own() {
        let ids: HashSet<&str> = Check::ALL.iter().map(|c| c.id()).collect();
        assert_eq!(ids.len(), Check::ALL.len());
        assert!(Check::ALL.iter().all(|c| !c.about().is_empty()));
    }

    /// The groups Verify can leave out: kept by id in the settings, so the
    /// ids are these for good; a finding of a check that isn't the
    /// doctor's is a plugin's.
    #[test]
    fn findings_fall_into_groups_kept_by_id() {
        let ids: Vec<&str> = Group::ALL.iter().map(|g| g.id()).collect();
        assert_eq!(
            ids,
            ["references", "tilesets", "models", "2das", "objects", "module", "plugins"]
        );
        assert!(Group::ALL.iter().all(|g| Group::from_id(g.id()) == Some(*g)));
        let finding = |check: &'static str| Finding {
            severity: Severity::Error,
            check: check.into(),
            source: "hak".into(),
            resource: key("tdm01", ResType::SET),
            at: String::new(),
            message: String::new(),
        };
        assert_eq!(Group::of(&finding("set-model")), Group::Tilesets);
        assert_eq!(Group::of(&finding("2da-row")), Group::Tables);
        assert_eq!(Group::of(&finding("my-plugin-check")), Group::Plugins);
        // Every check is in one of the doctor's groups.
        let theirs = [Group::References, Group::Plugins];
        assert!(Check::ALL.iter().all(|c| !theirs.contains(&c.group())));
        // Left out: named, in their order, the rest kept in order.
        let mut findings = vec![finding("set-model"), finding("2da-row"), finding("set-door")];
        let mut missing = Vec::new();
        let skip = Skip::groups(&[Group::Models, Group::Tilesets]);
        skip.leave_out(&mut missing, &mut findings);
        assert_eq!(skip.named(), ["Models and materials", "Tilesets"]);
        assert_eq!(checks(&findings), [("2da-row", String::new())]);
    }

    /// A hak that says what was read of it.
    #[derive(Debug)]
    struct Watched(MemContainer, std::sync::Arc<std::sync::Mutex<Vec<ResKey>>>);

    impl mg_resman::Container for Watched {
        fn contains(&self, key: &ResKey) -> bool {
            self.0.contains(key)
        }
        fn read(&self, key: &ResKey) -> Result<std::borrow::Cow<'_, [u8]>, mg_resman::ResError> {
            self.1.lock().unwrap().push(*key);
            self.0.read(key)
        }
        fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
            self.0.keys()
        }
        fn len(&self) -> usize {
            self.0.len()
        }
    }

    /// A group left out isn't checked: nothing it would examine is read
    /// (the skipped tilesets and models were a minute of the Domains of
    /// Dread's Verify, with 144 haks), and the other groups find what they
    /// would. A texture is never read.
    #[test]
    fn a_group_left_out_is_not_checked() {
        let set = b"[GENERAL]\nName=TST\nTransition=1\n[TILES]\nCount=1\n[TILE0]\nModel=tst_a01\n\
[GROUPS]\nCount=0\n";
        let mut hak = MemContainer::new();
        hak.insert(key("tst", ResType::SET), set.to_vec());
        hak.insert(key("tst_a01", ResType::MDL), b"not a model".to_vec());
        let emitter = "newmodel fx\nsetsupermodel fx NULL\nbeginmodelgeom fx\n\
node dummy fx\n  parent NULL\nendnode\nnode emitter spark\n  parent fx\n  p2p 1\nendnode\n\
endmodelgeom fx\ndonemodel fx\n";
        hak.insert(key("fx", ResType::MDL), emitter.as_bytes().to_vec());
        hak.insert(key("my_mat", ResType::MTR), b"texture0 a_texture_name_too_long\n".to_vec());
        hak.insert(key("my_tex", ResType::TGA), vec![0; 64]);
        hak.insert(key("placeables", ResType::TWODA), b"2DA V2.0\n\n   Label\n".to_vec());
        let read = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut rm = resman(&[]);
        rm.add(priority::HAK_USER, "hak:watched", LayerClass::Erf, Watched(hak, read.clone()));
        let run = |skip: &[&str]| {
            read.lock().unwrap().clear();
            let f = examine_groups(
                &Module::new(),
                &rm,
                TalkTables::default(),
                &Skip::parse(skip).unwrap(),
                &mut Timings::default(),
                None,
            );
            let mut types: Vec<ResType> = read.lock().unwrap().iter().map(|k| k.restype).collect();
            types.sort();
            types.dedup();
            (f.iter().map(|f| f.check.to_string()).collect::<Vec<_>>(), types)
        };
        let (all, read_all) = run(&[]);
        assert!(
            ["mdl-read", "mtr-name", "mdl-text"].iter().all(|c| all.contains(&c.to_string())),
            "{all:?}"
        );
        assert!(!read_all.contains(&ResType::TGA), "{read_all:?}");
        let (left, read_left) = run(&["tilesets", "models"]);
        assert!(left.iter().all(|c| !c.starts_with("set-") && !c.starts_with("mdl-")));
        assert!(!left.contains(&"mtr-name".to_string()), "{left:?}");
        assert_eq!(read_left, [ResType::TWODA], "only the 2DA is read");
        // The 2DAs' group left out too: nothing of the hak is read.
        let (_, read_none) = run(&["tilesets", "models", "2das"]);
        assert!(read_none.is_empty(), "{read_none:?}");
        // One check left out, not its group: its costly part isn't run (a
        // tile's model isn't read), the group's other checks are.
        let (left, read_left) = run(&["mdl-read", "tile-faces", "mdl-text"]);
        assert!(!left.iter().any(|c| c == "mdl-read" || c == "mdl-text"), "{left:?}");
        assert!(left.contains(&"mtr-name".to_string()), "{left:?}");
        assert!(!read_left.contains(&ResType::MDL), "{read_left:?}");
        // A hak left out: nothing of it is read.
        let (left, read_left) = run(&["watched.hak"]);
        assert!(left.is_empty() && read_left.is_empty(), "{left:?} {read_left:?}");
        let (_, read_left) = run(&["hak:watched"]);
        assert!(read_left.is_empty(), "{read_left:?}");
        // A tileset left out: it and its tiles' models aren't read; the
        // rest is checked.
        let (left, read_left) = run(&["tst.set"]);
        assert!(!left.iter().any(|c| c.starts_with("set-") || c == "mdl-read"), "{left:?}");
        assert!(left.contains(&"mtr-name".to_string()), "{left:?}");
        assert!(!read_left.contains(&ResType::SET), "{read_left:?}");
    }

    /// A hak's models are read once: what was found of each (a text's
    /// findings, a tile model's faces) is kept in the cache and a second
    /// Verify reads none of them while the hak is as it was, finding the
    /// same; a hak changed (another time or size) is read again. A folder
    /// is always read.
    #[test]
    fn an_archive_s_models_are_read_once_while_it_is_as_it_was() {
        #[derive(Debug)]
        struct Stamped(Watched, std::sync::Arc<std::sync::atomic::AtomicU64>);
        impl mg_resman::Container for Stamped {
            fn contains(&self, key: &ResKey) -> bool {
                self.0.contains(key)
            }
            fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, mg_resman::ResError> {
                self.0.read(key)
            }
            fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
                self.0.keys()
            }
            fn len(&self) -> usize {
                self.0.len()
            }
            fn fingerprint(&self) -> u64 {
                self.1.load(std::sync::atomic::Ordering::Relaxed)
            }
        }
        let set = b"[GENERAL]\nName=TST\nTransition=1\n[TILES]\nCount=2\n[TILE0]\nModel=tst_a01\n\
[TILE1]\nModel=tst_b01\n[GROUPS]\nCount=0\n";
        let mut hak = MemContainer::new();
        hak.insert(key("tst", ResType::SET), set.to_vec());
        hak.insert(key("tst_a01", ResType::MDL), b"not a model".to_vec());
        let mut text = String::from(
            "newmodel tst_b01\nsetsupermodel tst_b01 NULL\nbeginmodelgeom tst_b01\n\
node dummy tst_b01\n  parent NULL\nendnode\nnode trimesh m\n  parent tst_b01\n",
        );
        text.push_str("  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n  faces 10001\n");
        for _ in 0..10_001 {
            text.push_str("    0 1 2 1 0 0 0 0\n");
        }
        text.push_str("endnode\nendmodelgeom tst_b01\ndonemodel tst_b01\n");
        let emitter = "newmodel fx\nsetsupermodel fx NULL\nbeginmodelgeom fx\n\
node dummy fx\n  parent NULL\nendnode\nnode emitter spark\n  parent fx\n  p2p 1\nendnode\n\
endmodelgeom fx\ndonemodel fx\n";
        hak.insert(key("fx", ResType::MDL), emitter.as_bytes().to_vec());
        hak.insert(key("tst_b01", ResType::MDL), text.into_bytes());
        let mut folder = MemContainer::new();
        folder.insert(key("loose", ResType::MDL), b"newmodel loose\nparent\n".to_vec());
        let read = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let stamp = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(7));
        let mut rm = resman(&[]);
        rm.add(
            priority::HAK_USER,
            "hak:stamped",
            LayerClass::Erf,
            Stamped(Watched(hak, read.clone()), stamp.clone()),
        );
        rm.add(
            priority::OVERRIDE,
            "override",
            LayerClass::Directory,
            Watched(folder, read.clone()),
        );
        let dir = mg_testkit::scratch_dir("doctor-model-cache");
        let cache = Cache::new(&dir);
        let run = |rm: &ResMan, cache: Option<&Cache>| {
            read.lock().unwrap().clear();
            let f = examine_groups(
                &Module::new(),
                rm,
                TalkTables::default(),
                &Skip::NONE,
                &mut Timings::default(),
                cache,
            );
            let mut models: Vec<String> = (read.lock().unwrap().iter())
                .filter(|k| k.restype == ResType::MDL)
                .map(ToString::to_string)
                .collect();
            models.sort();
            models.dedup();
            (f, models)
        };
        let (plain, _) = run(&rm, None);
        for c in ["mdl-read", "tile-faces", "mdl-text"] {
            assert!(plain.iter().any(|f| f.check == c), "{c}: {plain:#?}");
        }
        let (first, read_first) = run(&rm, Some(&cache));
        assert_eq!(first, plain);
        assert_eq!(read_first, ["fx.mdl", "loose.mdl", "tst_a01.mdl", "tst_b01.mdl"]);
        // As it was: only the folder's model is read, the findings the same.
        let (again, read_again) = run(&rm, Some(&cache));
        assert_eq!(again, plain);
        assert_eq!(read_again, ["loose.mdl"]);
        // Changed: read again, and the earlier version's file goes.
        stamp.store(8, std::sync::atomic::Ordering::Relaxed);
        rm.rescan("hak:stamped").unwrap();
        let (changed, read_changed) = run(&rm, Some(&cache));
        assert_eq!(changed, plain);
        assert_eq!(read_changed, ["fx.mdl", "loose.mdl", "tst_a01.mdl", "tst_b01.mdl"]);
        let files: Vec<String> = std::fs::read_dir(dir.join("verify"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(files, ["hak_stamped-0000000000000008.json"]);
    }

    /// What `mg verify --skip` and Options › Verify take: groups, checks
    /// (the doctor's and plugins'), missing references' categories, haks
    /// and folders, resources; anything else is refused, with what is
    /// taken.
    #[test]
    fn verify_leaves_out_groups_checks_haks_and_resources() {
        let skip = Skip::parse(&[
            "Tilesets",
            "set-hole",
            "example.tags/tag-case",
            "references:creatures",
            "hak:TDoD_Tiles01",
            "tdod_tiles02.hak",
            "override",
            "ttr01.set",
            " ",
            "set-hole",
        ])
        .unwrap();
        assert_eq!(
            skip.words(),
            [
                "tilesets",
                "set-hole",
                "example.tags/tag-case",
                "references:creatures",
                "hak:tdod_tiles01",
                "tdod_tiles02.hak",
                "override",
                "ttr01.set"
            ]
        );
        assert!(skip.check(Check::SetModel) && skip.check(Check::SetHole));
        assert!(!skip.check(Check::TwoDaRow));
        assert!(skip.plugin_check("example.tags/tag-case") && !skip.plugin_check("example.tags/x"));
        assert!(skip.source("hak:tdod_tiles01") && skip.source("hak:tdod_tiles02"));
        assert!(skip.source("override") && !skip.source("module") && !skip.source("hak:other"));
        assert!(skip.resource(&key("ttr01", ResType::SET)));
        assert_eq!(skip.named()[0], "Tilesets");
        let finding = |check: &'static str, source: &str, resource: ResKey| Finding {
            severity: Severity::Error,
            check: check.into(),
            source: source.into(),
            resource,
            at: String::new(),
            message: String::new(),
        };
        let mut findings = vec![
            finding("2da-row", "hak:tdod_tiles01", key("a", ResType::TWODA)),
            finding("2da-row", "hak:keep", key("ttr01", ResType::SET)),
            finding("example.tags/tag-case", "module", key("module", ResType::IFO)),
            finding("2da-row", "module", key("b", ResType::TWODA)),
        ];
        let missing = |t: ResType| {
            let reference = crate::refs::Reference {
                from: key("x", t),
                path: "Script".into(),
                kind: crate::refs::RefKind::Script,
                target: ResRef::from_str("gone").unwrap(),
            };
            crate::verify::Missing {
                category: crate::verify::Category::of(&reference),
                reference,
                uncompiled: false,
                only_local: None,
            }
        };
        let mut gone = vec![missing(ResType::UTC), missing(ResType::UTD)];
        skip.leave_out(&mut gone, &mut findings);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].resource, key("b", ResType::TWODA));
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].category, crate::verify::Category::Doors);
        // Anything else is refused, saying what is taken.
        let e = Skip::parse(&["tilesetz"]).unwrap_err();
        assert!(e.contains("tilesetz") && e.contains("references:CATEGORY"), "{e}");
        assert!(Skip::parse(&["references:dragons"]).is_err());
        assert!(Skip::parse(&["hak:"]).is_err());
        // The settings' words pass over what isn't one.
        assert_eq!(Skip::read(&["tilesetz", "2das"]).words(), ["2das"]);
    }

    /// The resources are examined in parallel, and the findings are as one
    /// after another make them: a tileset's tiles' in the order of its
    /// tiles, its tile models' at the first tile of each, the same each
    /// time.
    #[test]
    fn findings_examined_in_parallel_keep_their_order() {
        let mut hak: Vec<(String, ResType, Vec<u8>)> = Vec::new();
        for s in 0..40 {
            let mut set = format!("[GENERAL]\nName=T{s}\nTransition=1\n[TILES]\nCount=60\n");
            for t in 0..60 {
                // Every third tile's model is nowhere; the others share
                // five models, unreadable, of the tileset's own.
                let model =
                    if t % 3 == 0 { format!("gone{s}_{t}") } else { format!("m{s}_{}", t % 5) };
                set.push_str(&format!("[TILE{t}]\nModel={model}\n"));
            }
            set.push_str("[GROUPS]\nCount=0\n");
            hak.push((format!("set{s:02}"), ResType::SET, set.into_bytes()));
            for m in 0..5 {
                hak.push((format!("m{s}_{m}"), ResType::MDL, b"not a model".to_vec()));
            }
        }
        let hak: Vec<(&str, ResType, &[u8])> =
            hak.iter().map(|(n, t, d)| (n.as_str(), *t, d.as_slice())).collect();
        let rm = resman(&hak);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let models: Vec<&Finding> = f.iter().filter(|f| f.check == "set-model").collect();
        assert_eq!(models.len(), 40 * 20);
        for s in 0..40 {
            let at: Vec<&str> = (models.iter())
                .filter(|f| f.resource == key(&format!("set{s:02}"), ResType::SET))
                .map(|f| f.at.as_str())
                .collect();
            let tiles: Vec<String> =
                (0..60).step_by(3).map(|t| format!("[TILE{t}] Model")).collect();
            assert_eq!(at, tiles, "tileset {s}");
        }
        assert_eq!(f.iter().filter(|f| f.check == "mdl-read").count(), 40 * 5);
        for _ in 0..3 {
            assert_eq!(examine(&Module::new(), &rm, TalkTables::default()), f);
        }
    }

    /// A module kept as a folder: its table with a description beside it
    /// is held to the description; one without is not looked at.
    #[test]
    fn a_described_table_is_held_to_its_description() {
        let dir = mg_testkit::scratch_dir("doctor-described");
        let table = b"2DA V2.0\n\n  Label Chance Kinds\n0 a 50 0x01\n1 b 500 0x04\n";
        let mut m = Module::new();
        m.set(ResKey::parse("loot", ResType::TWODA).unwrap(), table.to_vec());
        m.set(ResKey::parse("other", ResType::TWODA).unwrap(), table.to_vec());
        m.save_as(&crate::ModuleLocation::Folder(dir.clone())).unwrap();
        std::fs::write(dir.join("loot_inc.nss"), "const int KIND_GOLD = 0x01;\n").unwrap();
        std::fs::write(
            dir.join("loot.2da.toml"),
            "[columns.Chance]\nkind = \"int\"\nmax = 100\n\n[columns.Kinds]\nkind = \
             \"flags\"\nconstants = \"loot_inc\"\nprefix = \"KIND_\"\n",
        )
        .unwrap();
        let m = Module::open(&dir).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        assert_eq!(
            checks(&f),
            [("2da-described", "row 1, Chance".into()), ("2da-described", "row 1, Kinds".into())]
        );
        assert!(f[0].message.contains("500 is above") && f[0].source == "module", "{f:?}");
        // A description that cannot be read is said once, for its table.
        std::fs::write(dir.join("loot.2da.toml"), "[columns.Chance]\nkind = \"odd\"\n").unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        assert_eq!(checks(&f), [("2da-described", String::new())]);
        assert!(f[0].message.contains("loot.2da.toml") && f[0].message.contains("no kind"));
    }

    /// A tag the project's moonglow.toml wants unique, on two objects: a
    /// warning naming both. Without the file, nothing.
    /// A variable set without one it needs beside it on the same object
    /// (`requires`, a `[[rules]]` rule where the other has its value) is
    /// said, once per object; one that has it, or another value, is not.
    #[test]
    fn a_variable_without_what_it_needs_beside_it_is_said() {
        let dir = mg_testkit::scratch_dir("doctor-variables-required");
        let s = |t: &str| Value::String(t.as_bytes().to_vec());
        let var = |name: &str, value: &str| {
            let mut v = Struct::new(0);
            v.set("Name", s(name));
            v.set("Type", Value::Dword(3));
            v.set("Value", s(value));
            v
        };
        let object = |tag: &str, vars: Vec<Struct>| {
            let mut o = Struct::new(4);
            o.set("Tag", s(tag));
            o.set("VarTable", Value::List(vars));
            o
        };
        let mut m = Module::new();
        let mut git = Gff::new(*b"GIT ");
        let creatures = vec![
            object("LONE", vec![var("tdod_rental", "smithy")]),
            object("BOTH", vec![var("tdod_rental", "inn"), var("tdod_rental_door", "D_INN")]),
            object("TRADER", vec![var("tdod_kind", "merchant")]),
            object("SMITH", vec![var("tdod_kind", "smith")]),
        ];
        git.root.set("Creature List", Value::List(creatures));
        m.set_gff(key("krezk", ResType::GIT), &git).unwrap();
        m.save_as(&crate::ModuleLocation::Folder(dir.clone())).unwrap();
        std::fs::write(
            dir.join("variables.toml"),
            "[tdod_rental]\ntype = \"string\"\nrequires = [\"tdod_rental_door\"]\n\n\
             [tdod_rental_door]\ntype = \"string\"\n\n[tdod_kind]\ntype = \"string\"\n\n\
             [[rules]]\nvariable = \"tdod_kind\"\nis = \"merchant\"\nrequire = \"tdod_store\"\n",
        )
        .unwrap();
        let m = Module::open(&dir).unwrap();
        assert_eq!(m.project_file.warnings, Vec::<String>::new());
        let found: Vec<(String, String)> = examine(&m, &resman(&[]), TalkTables::default())
            .into_iter()
            .filter(|f| f.check == "var-requires")
            .map(|f| (f.at, f.message))
            .collect();
        assert_eq!(found.len(), 2, "{found:#?}");
        assert!(found[0].0.contains("LONE") && found[0].0.ends_with("tdod_rental"), "{found:#?}");
        assert_eq!(
            found[0].1,
            "tdod_rental is set here and tdod_rental_door isn't: variables.toml:1 says it is \
             needed beside it"
        );
        assert!(found[1].0.contains("TRADER"), "{found:#?}");
        assert_eq!(
            found[1].1,
            "tdod_kind is set here and tdod_store isn't: variables.toml:11 says it is needed where \
             tdod_kind is merchant"
        );
        // A variable of another type than the scripts read it as, not
        // described with a type: said, with the script and line.
        let mut m = Module::open(&dir).unwrap();
        m.set(
            key("tdod_trade", ResType::NSS),
            b"void main()\n{\n    int n = GetLocalInt(OBJECT_SELF, \"tdod_rental_door\");\n    \
              string k = GetLocalString(OBJECT_SELF, \"tdod_kind\");\n}\n"
                .to_vec(),
        );
        std::fs::write(
            dir.join("variables.toml"),
            "[tdod_kind]\ntype = \"string\"\n\n[tdod_rental_door]\nhelp = \"no type\"\n",
        )
        .unwrap();
        m.reread_project_file();
        let read: Vec<(String, String)> = examine(&m, &resman(&[]), TalkTables::default())
            .into_iter()
            .filter(|f| f.check == "var-script")
            .map(|f| (f.at, f.message))
            .collect();
        assert_eq!(read.len(), 1, "{read:#?}");
        assert!(read[0].0.contains("BOTH") && read[0].0.ends_with("tdod_rental_door"), "{read:#?}");
        assert_eq!(
            read[0].1,
            "tdod_rental_door is a String here, and the scripts read it as an Int (GetLocalInt in \
             tdod_trade.nss line 3): they read 0"
        );
        // Left out with its check.
        let skip = Skip::parse(&["var-requires"]).unwrap();
        let f = examine_groups(
            &m,
            &resman(&[]),
            TalkTables::default(),
            &skip,
            &mut Timings::default(),
            None,
        );
        assert!(!f.iter().any(|f| f.check == "var-requires"));
    }

    #[test]
    fn variables_are_held_to_the_project_s_descriptions() {
        let dir = mg_testkit::scratch_dir("doctor-described-variables");
        let var = |name: &str, value: Value| {
            let t = match value {
                Value::Int(_) => 1,
                Value::Float(_) => 2,
                _ => 3,
            };
            let mut v = Struct::new(0);
            v.set("Name", Value::String(name.as_bytes().to_vec()));
            v.set("Type", Value::Dword(t));
            v.set("Value", value);
            v
        };
        let s = |t: &str| Value::String(t.as_bytes().to_vec());
        let object = |id: u32, tag: &str, vars: Vec<Struct>| {
            let mut o = Struct::new(id);
            o.set("Tag", s(tag));
            o.set("VarTable", Value::List(vars));
            o
        };
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.set("VarTable", Value::List(vec![var("tdod_lamplit", Value::Int(1))]));
        m.set_gff(key("module", ResType::IFO), &ifo).unwrap();
        let mut git = Gff::new(*b"GIT ");
        git.root.set("VarTable", Value::List(vec![var("tdod_lamplit", Value::Int(1))]));
        let bench = object(
            9,
            "BENCH",
            vec![
                var("tdod_workbench", s("Forge")),
                var("tdod_lit", s("1")),
                var("TDOD_light", Value::Int(1)),
                var("tdod_workbnch", s("forge")),
                var("tdod_harvest_dc", Value::Int(99)),
                var("tdod_craft", Value::Int(1)),
                var("tdod_craft2", Value::Int(7)),
                var("someone_elses", s("x")),
                var("tdod_trodden", Value::Int(1)),
                var("tdod_save_d20", Value::Int(20)),
                var("tdod_lore_txt", s("A worn plaque")),
                var("tdod_lore_seen", Value::Int(1)),
            ],
        );
        let mut good = object(9, "GOOD", vec![var("tdod_workbench", s("forge"))]);
        good.set("Tag", s("GOOD"));
        let mut keeper = object(4, "KEEPER", vec![var("tdod_kind", s("monster"))]);
        let item = object(0, "", vec![var("tdod_kind", s("beast"))]);
        keeper.set("ItemList", Value::List(vec![item]));
        git.root.set("Placeable List", Value::List(vec![bench, good]));
        git.root.set("Creature List", Value::List(vec![keeper]));
        m.set_gff(key("krezk", ResType::GIT), &git).unwrap();
        m.set(
            key("tdod_kind_inc", ResType::NSS),
            b"const string TDOD_KIND_BEAST = \"beast\";\nconst string TDOD_KIND_UNDEAD = \"undead\";\n"
                .to_vec(),
        );
        m.save_as(&crate::ModuleLocation::Folder(dir.clone())).unwrap();
        let rm = resman(&[(
            "crafts",
            ResType::TWODA,
            b"2DA V2.0\n\n   Label\n0  Smithing\n1  Tailoring\n",
        )]);
        let m = Module::open(&dir).unwrap();
        let ours = |f: &[Finding]| -> Vec<(String, String, String)> {
            f.iter()
                .filter(|x| x.check.starts_with("var-"))
                .map(|x| (x.check.to_string(), x.at.clone(), x.message.clone()))
                .collect()
        };
        assert_eq!(ours(&examine(&m, &rm, TalkTables::default())), [], "no description");
        std::fs::write(
            dir.join("variables.toml"),
            "undescribed = [\"tdod_*\"]\n\n\
             [tdod_workbench]\ntype = \"string\"\non = [\"placeable\"]\n\
             values = { forge = \"Smithing\", tanner = \"Leatherworking\" }\n\n\
             [tdod_lit]\ntype = \"int\"\nvalues = [1]\n\n\
             [tdod_light]\ntype = \"int\"\n\n\
             [tdod_harvest_dc]\ntype = \"int\"\nmin = 1\nmax = 60\n\n\
             [tdod_lamplit]\ntype = \"int\"\non = [\"area\"]\n\n\
             [tdod_kind]\ntype = \"string\"\nconstants = \"tdod_kind_inc\"\nprefix = \"TDOD_KIND_\"\n\n\
             [\"tdod_craft*\"]\ntype = \"int\"\ntable = \"crafts\"\n\n\
             [tdod_trodden]\nruntime = true\nhelp = \"The scripts' own\"\n\n\
             [\"tdod_*_d20\"]\ntype = \"int\"\nruntime = true\n\n\
             [tdod_lore_text]\ntype = \"string\"\n\n\
             [\"tdod_lore_*\"]\nruntime = true\n",
        )
        .unwrap();
        let m = Module::open(&dir).unwrap();
        assert!(m.project_file.variables_found, "{:?}", m.project_file.warnings);
        assert_eq!(m.project_file.warnings, Vec::<String>::new());
        let found = ours(&examine(&m, &rm, TalkTables::default()));
        let has = |check: &str, at: &str, message: &str| {
            assert!(
                found.iter().any(|(c, a, m)| c == check && a.ends_with(at) && m == message),
                "no {check} at …{at}: {message}\nin {found:#?}"
            );
        };
        has(
            "var-type",
            "tdod_lit",
            "tdod_lit is a String here, and variables.toml:8 says it is an Int: the scripts read 0",
        );
        has(
            "var-value",
            "tdod_workbench",
            "tdod_workbench: \"Forge\" is none of its values: \"forge\" (the scripts compare it as \
             it is written, case and all) (variables.toml:3)",
        );
        has(
            "var-value",
            "tdod_harvest_dc",
            "tdod_harvest_dc: 99 is more than 60, the most it takes (variables.toml:15)",
        );
        has(
            "var-value",
            "tdod_kind",
            "tdod_kind: \"monster\" is none of its values (\"beast\", \"undead\") \
             (variables.toml:24)",
        );
        has(
            "var-value",
            "tdod_craft2",
            "tdod_craft2: 7 is no row of crafts.2da (2 rows) (variables.toml:29)",
        );
        has(
            "var-name",
            "TDOD_light",
            "TDOD_light is described as tdod_light (variables.toml:12): the game tells names \
             apart by their case, and a script that reads tdod_light finds nothing here",
        );
        has(
            "var-name",
            "tdod_workbnch",
            "tdod_workbnch is named as the project's variables are (tdod_*), and variables.toml \
             describes no such variable: tdod_workbench?",
        );
        has(
            "var-object",
            "Module › VarTable › tdod_lamplit",
            "tdod_lamplit is on a module, and variables.toml:20 says it goes on area",
        );
        // The scripts' own, kept in a file: an error, whatever its type.
        has(
            "var-runtime",
            "tdod_trodden",
            "tdod_trodden is set by the scripts while the server runs (variables.toml:33 says \
             runtime = true), and is never to be kept in a file: one kept here is there from the \
             start, on every server",
        );
        has(
            "var-runtime",
            "tdod_save_d20",
            "tdod_save_d20 (tdod_*_d20) is set by the scripts while the server runs \
             (variables.toml:37 says runtime = true), and is never to be kept in a file: one kept \
             here is there from the start, on every server",
        );
        // A typo of a builder's name that the scripts' own pattern takes
        // (The Domains of Dread's tdod_lore_txt): said with the name it
        // is near, as var-name says one; one near none, without.
        has(
            "var-runtime",
            "tdod_lore_txt",
            "tdod_lore_txt (tdod_lore_*) is set by the scripts while the server runs \
             (variables.toml:44 says runtime = true), and is never to be kept in a file: one kept \
             here is there from the start, on every server: tdod_lore_text? (variables.toml:41)",
        );
        has(
            "var-runtime",
            "tdod_lore_seen",
            "tdod_lore_seen (tdod_lore_*) is set by the scripts while the server runs \
             (variables.toml:44 says runtime = true), and is never to be kept in a file: one kept \
             here is there from the start, on every server",
        );
        // Those that are right, and the others' variables, are not said.
        assert_eq!(found.len(), 12, "{found:#?}");
        let severity = examine(&m, &rm, TalkTables::default())
            .into_iter()
            .filter(|f| f.check.starts_with("var-"))
            .map(|f| (f.check.to_string(), f.severity))
            .collect::<Vec<_>>();
        assert!(severity.contains(&("var-type".into(), Severity::Error)));
        assert!(severity.contains(&("var-runtime".into(), Severity::Error)));
        assert!(
            (severity.iter())
                .filter(|(c, _)| c != "var-type" && c != "var-runtime")
                .all(|(_, s)| *s == Severity::Warning)
        );
    }

    #[test]
    fn a_tag_the_project_wants_unique_is_found_twice() {
        let dir = mg_testkit::scratch_dir("doctor-unique-tags");
        let mut m = Module::new();
        let waypoint = |tag: &str| {
            let mut w = Struct::new(5);
            w.set("Tag", Value::String(tag.as_bytes().to_vec()));
            w
        };
        for (area, tags) in [
            ("dev_staging", &["TDOD_START_KREZK", "LAMP"][..]),
            ("krezk", &["TDOD_START_KREZK", "LAMP"]),
        ] {
            let mut git = Gff::new(*b"GIT ");
            git.root.set("WaypointList", Value::List(tags.iter().map(|t| waypoint(t)).collect()));
            m.set_gff(key(area, ResType::GIT), &git).unwrap();
        }
        m.save_as(&crate::ModuleLocation::Folder(dir.clone())).unwrap();
        let m = Module::open(&dir).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        assert!(f.iter().all(|x| x.check != "tag-unique"), "no project file: {f:?}");
        std::fs::write(dir.join("moonglow.toml"), "[tags]\nunique = [\"TDOD_START_*\"]\n").unwrap();
        let m = Module::open(&dir).unwrap();
        assert!(m.project_file.found);
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<&Finding> = f.iter().filter(|x| x.check == "tag-unique").collect();
        assert_eq!(found.len(), 1, "LAMP may be on two: {f:?}");
        assert_eq!(found[0].at, "waypoint 0 in dev_staging, waypoint 0 in krezk");
        assert_eq!(found[0].resource, key("dev_staging", ResType::GIT));
        assert_eq!(found[0].severity, Severity::Warning);
        assert!(
            found[0].message.starts_with("TDOD_START_KREZK is on 2 objects"),
            "{}",
            found[0].message
        );
        assert_eq!(Group::of(found[0]), Group::Objects);
        // Unique in each area: two areas with one each are as meant, and
        // a second in one area is said, with that area's places alone.
        std::fs::write(
            dir.join("moonglow.toml"),
            "[tags]\nunique = [\"TDOD_START_*\", \"*\"]\nunique_in_area = [\"LAMP\"]\n",
        )
        .unwrap();
        let m = Module::open(&dir).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<&Finding> = f.iter().filter(|x| x.check == "tag-unique").collect();
        assert_eq!(found.len(), 1, "LAMP once in each area: {found:?}");
        let mut m = Module::open(&dir).unwrap();
        let git = key("krezk", ResType::GIT);
        let mut g = m.gff(&git).unwrap().unwrap();
        g.root.set(
            "WaypointList",
            Value::List(["TDOD_START_KREZK", "LAMP", "LAMP"].iter().map(|t| waypoint(t)).collect()),
        );
        m.set_gff(git, &g).unwrap();
        m.save().unwrap();
        let m = Module::open(&dir).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<&Finding> = f.iter().filter(|x| x.check == "tag-unique").collect();
        assert_eq!(found.len(), 2, "{found:?}");
        let lamp = found.iter().find(|x| x.message.starts_with("LAMP")).unwrap();
        assert_eq!(lamp.at, "waypoint 1 in krezk, waypoint 2 in krezk");
        assert_eq!(lamp.resource, git);
        assert_eq!(
            lamp.message,
            "LAMP is on 2 objects in krezk, and the project's moonglow.toml wants LAMP unique in \
             each area: a script that looks for it there finds only one of them"
        );
    }

    /// The Domains of Dread's builder proposal, C7: a merged area keeps its
    /// ResRef and the Tag the claims table records.
    #[test]
    fn a_merged_area_renamed_or_retagged_is_found() {
        let dir = mg_testkit::scratch_dir("doctor-merged-areas");
        let mut m = Module::new();
        for (area, tag) in [("kz_square", "KZ_SQUARE"), ("kz_gate", "KZ_GATE_2")] {
            let mut are = Gff::new(*b"ARE ");
            are.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
            m.set_gff(key(area, ResType::ARE), &are).unwrap();
        }
        m.save_as(&crate::ModuleLocation::Folder(dir.clone())).unwrap();
        std::fs::write(
            dir.join("claims.2da"),
            "2DA V2.0\n\n Area Builder Status Date Tag\n0 kz_square ab merged x KZ_SQUARE\n\
             1 kz_gate **** merged x KZ_GATE\n2 kz_bridge cd merged x KZ_BRIDGE\n\
             3 kz_mill cd claimed x ****\n",
        )
        .unwrap();
        let m = Module::open(&dir).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        assert!(f.iter().all(|x| !x.check.starts_with("claims-")), "no project file: {f:?}");
        std::fs::write(dir.join("moonglow.toml"), "[claims]\nfile = \"claims.2da\"\n").unwrap();
        let m = Module::open(&dir).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<(&str, &str, String)> = f
            .iter()
            .filter(|x| x.check.starts_with("claims-"))
            .map(|x| (&*x.check, &*x.at, x.resource.to_string()))
            .collect();
        assert_eq!(
            found,
            [
                ("claims-merged", "claims.2da, row 2", "kz_bridge.are".into()),
                ("claims-tag", "claims.2da, row 1", "kz_gate.are".into()),
            ]
        );
        let tag = f.iter().find(|x| x.check == "claims-tag").unwrap();
        assert_eq!(
            tag.message,
            "kz_gate's Tag is KZ_GATE_2, and it was merged as KZ_GATE (claims.2da): a character's \
             position is kept by the area's Tag"
        );
        assert!(
            f.iter()
                .filter(|x| x.check.starts_with("claims-"))
                .all(|x| x.severity == Severity::Error)
        );
        assert_eq!(Group::of(tag), Group::Module);
    }

    #[test]
    fn two_das_name_rows_and_columns() {
        let table = b"2DA V2.0\n\n   Label ModelName StrRef\n0  Chair plc_chair 5\n1  Stool plc_stool 16777217\n2  Box plc_box 3 Crate plc_crate 4\n";
        let rm = resman(&[("placeables", ResType::TWODA, table), ("plc_box", ResType::MDL, b"newmodel plc_box\nbeginmodelgeom plc_box\nnode dummy plc_box\n  parent NULL\nendnode\nendmodelgeom plc_box\ndonemodel plc_box\n")]);
        let f = examine(&Module::new(), &rm, TalkTables { base: 100, custom: Some(1) });
        assert_eq!(
            checks(&f),
            [("2da-row", "row 2".into()), ("2da-strref", "row 1, StrRef".into())]
        );
        assert!(f.iter().all(|x| x.source == "my.hak"));
        // Row 0 is the game's, unchanged: not checked again. A row's missing
        // model matters when something uses it.
        let mut m = Module::new();
        let mut utp = Gff::new(*b"UTP ");
        utp.root.set("Appearance", Value::Dword(1));
        m.set_gff(key("stool", ResType::UTP), &utp).unwrap();
        let f = examine(&m, &rm, TalkTables { base: 100, custom: Some(1) });
        let object = f.iter().find(|x| x.check == "object-row").unwrap();
        assert!(
            object.message.contains("No model: plc_stool (placeables.2da row 1, ModelName)"),
            "{}",
            object.message
        );
        // A generic door names its row in EE's GenericType_New (Moonglow's
        // doors have no other): Verify read the older GenericType alone,
        // and found nothing.
        let doors = b"2DA V2.0\n\n   Label ModelName\n0  Wood door_wood\n";
        let rm = resman(&[("genericdoors", ResType::TWODA, doors)]);
        let mut m = Module::new();
        let mut utd = Gff::new(*b"UTD ");
        utd.root.set("Appearance", Value::Dword(0));
        utd.root.set("GenericType_New", Value::Dword(0));
        m.set_gff(key("gate", ResType::UTD), &utd).unwrap();
        let f = examine(&m, &rm, TalkTables::default());
        let door = f.iter().find(|x| x.check == "object-row").expect("the door's model");
        assert_eq!(door.at, "gate.utd › GenericType_New");
        assert!(
            door.message.contains("No model: door_wood (genericdoors.2da row 0"),
            "{}",
            door.message
        );
    }

    #[test]
    fn a_2da_s_changed_rows_name_rows_that_exist() {
        let feat = |rows: &str| format!("2DA V2.0\n\n   LABEL PREREQFEAT1 SPELLID\n{rows}");
        let mut game = MemContainer::new();
        game.insert(
            key("feat", ResType::TWODA),
            feat("0 Alertness **** ****\n1 Old 9 ****\n").into_bytes(),
        );
        game.insert(
            key("spells", ResType::TWODA),
            b"2DA V2.0\n\n   Label FeatID\n0 Acid ****\n1 Radial 131075\n".to_vec(),
        );
        // Row 1 is the game's, as it was (its own bad row is not this
        // hak's); row 2 names a feat that is there, row 3 one that is not
        // and a spell that is not; row 4 names none.
        let mut hak = MemContainer::new();
        hak.insert(
            key("feat", ResType::TWODA),
            feat("0 Alertness **** ****\n1 Old 9 ****\n2 New 4 1\n3 Bad 5 7\n4 None -1 ****\n")
                .into_bytes(),
        );
        // (A spell's FeatID with a radial's place over it: feat 3.)
        hak.insert(
            key("spells", ResType::TWODA),
            b"2DA V2.0\n\n   Label FeatID\n0 Acid ****\n1 Radial 131075\n2 New 131077\n".to_vec(),
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, game);
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine(&Module::new(), &rm, TalkTables { base: 100, custom: None });
        assert_eq!(
            checks(&f),
            [
                ("2da-ref", "row 3, PREREQFEAT1".into()),
                ("2da-ref", "row 3, SPELLID".into()),
                ("2da-ref", "row 2, FeatID".into()),
            ]
        );
        assert_eq!(f[0].message, "row 5 of feat.2da doesn't exist: it has 5 rows");
        assert_eq!(f[1].message, "row 7 of spells.2da doesn't exist: it has 3 rows");
        assert_eq!(f[2].message, "row 5 of feat.2da doesn't exist: it has 5 rows");
        assert!(f.iter().all(|x| x.severity == Severity::Warning && x.source == "my.hak"));
        // A table a class names, whatever its name; and one named as the
        // game's own are.
        let mut game = MemContainer::new();
        game.insert(key("feat", ResType::TWODA), feat("0 Alertness **** ****\n").into_bytes());
        let mut hak = MemContainer::new();
        hak.insert(
            key("classes", ResType::TWODA),
            b"2DA V2.0\n\n   Label FeatsTable\n0 Knight my_knight_feats\n".to_vec(),
        );
        let list = b"2DA V2.0\n\n   FeatLabel FeatIndex\n0 Alertness 0\n1 Gone 12\n".to_vec();
        hak.insert(key("my_knight_feats", ResType::TWODA), list.clone());
        hak.insert(key("cls_feat_orphan", ResType::TWODA), list.clone());
        hak.insert(key("other_list", ResType::TWODA), list);
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, game);
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine(&Module::new(), &rm, TalkTables { base: 100, custom: None });
        let mut found: Vec<(String, String)> =
            f.iter().map(|x| (x.resource.to_string(), x.at.clone())).collect();
        found.sort();
        assert_eq!(
            found,
            [
                ("cls_feat_orphan.2da".to_string(), "row 1, FeatIndex".to_string()),
                ("my_knight_feats.2da".to_string(), "row 1, FeatIndex".to_string()),
            ]
        );
        assert_eq!(column_kinds("CLS_FEAT_WIZ"), [("FeatIndex", ColumnKind::Row("feat"))]);
        // The same, of a table as it is edited: every row of it, its own
        // rows counted as they are now.
        let t =
            TwoDa::parse(feat("0 a 1 ****\n1 b 7 9\n2 c -1 0\n").as_bytes(), Codepage::default())
                .unwrap();
        let read = |name: &str| {
            let key = ResKey::parse(name, ResType::TWODA)?;
            let t = TwoDa::parse(&rm.get(&key).ok()?, Codepage::default()).ok()?;
            Some(std::sync::Arc::new(t))
        };
        assert_eq!(
            column_kinds_as_named("my_knight_feats", &read),
            [("FeatIndex", ColumnKind::Row("feat"))]
        );
        let found = table_faults(
            "feat",
            &t,
            &column_kinds("feat"),
            Some(&|name: &str| {
                let spells = "2DA V2.0\n\n   Label\n0 a\n1 b\n2 c\n";
                (name == "spells").then(|| {
                    std::sync::Arc::new(
                        TwoDa::parse(spells.as_bytes(), Codepage::default()).unwrap(),
                    )
                })
            }),
            TalkTables { base: 100, custom: None },
        );
        let at: Vec<(usize, usize)> = found.iter().map(|f| (f.row, f.column)).collect();
        assert_eq!(at, [(1, 1), (1, 2)], "{found:?}");
        assert_eq!(found[0].message, "row 7 of feat.2da doesn't exist: it has 3 rows");
        // What a table's columns hold, for those who ask.
        let kinds = column_kinds("Feat");
        assert!(kinds.contains(&("FEAT", ColumnKind::StrRef)));
        assert!(kinds.contains(&("MASTERFEAT", ColumnKind::Row("masterfeats"))));
        assert!(column_kinds("nothing").is_empty());
    }

    /// A class's other tables, the item properties' and the tables they
    /// name (the notes on the tables for plugins): a requirement's row by
    /// its kind, a table that isn't there, a row that is blank, the
    /// strings of a table an item property names.
    #[test]
    fn a_class_s_requirements_item_properties_and_padding_rows_are_checked() {
        let feat = |rows: &str| format!("2DA V2.0\n\n   LABEL PREREQFEAT1 SPELLID\n{rows}");
        let mut game = MemContainer::new();
        game.insert(
            key("feat", ResType::TWODA),
            feat("0 Alertness **** ****\n1 **** **** ****\n2 Dodge **** ****\n").into_bytes(),
        );
        game.insert(key("skills", ResType::TWODA), b"2DA V2.0\n\n   Label\n0 Hide\n".to_vec());
        let mut hak = MemContainer::new();
        hak.insert(
            key("classes", ResType::TWODA),
            b"2DA V2.0\n\n   Label PreReqTable BonusFeatsTable\n\
              0 Knight my_knight_pres cls_bfeat_gone\n"
                .to_vec(),
        );
        // A requirement of each kind: a feat, a blank feat, a skill past
        // the table's end, a variable (no row) and a feat past the end.
        hak.insert(
            key("my_knight_pres", ResType::TWODA),
            b"2DA V2.0\n\n   LABEL ReqType ReqParam1 ReqParam2\n\
              0 Dodge FEAT 2 ****\n1 Pad FEATOR 1 ****\n2 Hide SKILL 4 8\n\
              3 Var VAR X1_Allow 0\n4 Gone FEAT 9 ****\n"
                .to_vec(),
        );
        hak.insert(
            key("itempropdef", ResType::TWODA),
            b"2DA V2.0\n\n   Name Label SubTypeResRef CostTableResRef\n\
              0 649 Ability my_abilities 3\n"
                .to_vec(),
        );
        hak.insert(
            key("iprp_costtable", ResType::TWODA),
            b"2DA V2.0\n\n   Name Label\n0 IPRP_BASE1 Base\n".to_vec(),
        );
        hak.insert(
            key("my_abilities", ResType::TWODA),
            b"2DA V2.0\n\n   Name Label\n0 135 Str\n1 900 Dex\n".to_vec(),
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, game);
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine(&Module::new(), &rm, TalkTables { base: 800, custom: None });
        let mut found: Vec<(String, String, String)> =
            f.iter().map(|x| (x.resource.to_string(), x.at.clone(), x.message.clone())).collect();
        found.sort();
        let row = |r: &str, a: &str, m: &str| (r.to_string(), a.to_string(), m.to_string());
        assert_eq!(
            found,
            [
                row(
                    "classes.2da",
                    "row 0, BonusFeatsTable",
                    "cls_bfeat_gone.2da, which it names, isn't there"
                ),
                row(
                    "iprp_costtable.2da",
                    "row 0, Name",
                    "IPRP_BASE1.2da, which it names, isn't there"
                ),
                row(
                    "itempropdef.2da",
                    "row 0, CostTableResRef",
                    "row 3 of iprp_costtable.2da doesn't exist: it has 1 rows"
                ),
                row(
                    "my_abilities.2da",
                    "row 1, Name",
                    "string 900 doesn't exist: dialog.tlk has 800 strings (\"Bad Strref\")"
                ),
                row(
                    "my_knight_pres.2da",
                    "row 1, ReqParam1",
                    "row 1 of feat.2da is blank (a padding row)"
                ),
                row(
                    "my_knight_pres.2da",
                    "row 2, ReqParam1",
                    "row 4 of skills.2da doesn't exist: it has 1 rows"
                ),
                row(
                    "my_knight_pres.2da",
                    "row 4, ReqParam1",
                    "row 9 of feat.2da doesn't exist: it has 3 rows"
                ),
            ]
        );
        // What the columns hold: by the table's name, and by the column
        // that names it.
        let requirement = ColumnKind::RowBy { by: "ReqType", tables: &REQUIREMENT_ROWS };
        assert_eq!(column_kinds("cls_pres_wm"), [("ReqParam1", requirement)]);
        let read = |name: &str| {
            let key = ResKey::parse(name, ResType::TWODA)?;
            let t = TwoDa::parse(&rm.get(&key).ok()?, Codepage::default()).ok()?;
            Some(std::sync::Arc::new(t))
        };
        assert_eq!(column_kinds_as_named("my_knight_pres", &read), [("ReqParam1", requirement)]);
        assert_eq!(column_kinds_as_named("my_abilities", &read), [("Name", ColumnKind::StrRef)]);
        assert!(column_kinds("my_abilities").is_empty(), "known only as named");
        let kinds = column_kinds("classes");
        assert!(kinds.contains(&("SpellGainTable", ColumnKind::Table)));
        let pres = TwoDa::parse(
            &rm.get(&key("my_knight_pres", ResType::TWODA)).unwrap(),
            Codepage::default(),
        )
        .unwrap();
        assert_eq!(requirement.row_table(&pres, 2), Some("skills"));
        assert_eq!(requirement.row_table(&pres, 3), None);
        // As the 2DA editor checks a table, every row of it: and with no
        // game, no table is known not to be there.
        let classes =
            TwoDa::parse(&rm.get(&key("classes", ResType::TWODA)).unwrap(), Codepage::default())
                .unwrap();
        let tlk = TalkTables { base: 800, custom: None };
        let found = table_faults("classes", &classes, &kinds, Some(&read), tlk);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(table_faults("classes", &classes, &kinds, None, tlk).is_empty());
        let found =
            table_faults("my_knight_pres", &pres, &[("ReqParam1", requirement)], Some(&read), tlk);
        let at: Vec<usize> = found.iter().map(|f| f.row).collect();
        assert_eq!(at, [1, 2, 4], "{found:?}");
        // A cell a hak keeps as the game has it, wrong in the game too, is
        // the game's: said only where the hak changes it.
        let mut game = MemContainer::new();
        let appearance = |rows: &str| format!("2DA V2.0\n\n   LABEL RACE FOOTSTEPTYPE\n{rows}");
        game.insert(key("appearance", ResType::TWODA), appearance("0 Imp a 30\n").into_bytes());
        game.insert(
            key("footstepsounds", ResType::TWODA),
            b"2DA V2.0\n\n   Label\n0 Normal\n".to_vec(),
        );
        let hak = |rows: &str| {
            let mut hak = MemContainer::new();
            hak.insert(key("appearance", ResType::TWODA), appearance(rows).into_bytes());
            let mut rm = ResMan::new();
            rm.add(priority::KEY, "game", LayerClass::Key, game.clone());
            rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
            examine(&Module::new(), &rm, TalkTables { base: 800, custom: None })
        };
        assert!(hak("0 Imp b 30\n").is_empty(), "the game's 30, kept");
        let f = hak("0 Imp b 31\n");
        assert_eq!(checks(&f), [("2da-ref", "row 0, FOOTSTEPTYPE".into())]);
    }

    #[test]
    fn a_custom_talk_table_the_game_cant_find() {
        let rm = resman(&[]);
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.set("Mod_CustomTlk", Value::String(b"mine.tlk".to_vec()));
        m.set_gff(ResKey::parse("module", ResType::IFO).unwrap(), &ifo).unwrap();
        let f = examine(&m, &rm, TalkTables { base: 100, custom: None });
        assert_eq!(checks(&f), [("custom-tlk", "Custom Tlk".into())]);
        assert!(f[0].message.contains("without .tlk"), "{}", f[0].message);
        assert!(examine(&m, &rm, TalkTables { base: 100, custom: Some(3) }).is_empty());
        // A table not read (a plugin's test with no game): not said, nor
        // its strings.
        let uncounted = TalkTables { base: 0, custom: Some(TalkTables::UNCOUNTED) };
        assert!(examine(&m, &rm, uncounted).is_empty());
        assert_eq!(strref_problem(0x0100_0000 + 5000, uncounted), None);
        assert_eq!(strref_problem(5000, uncounted), None);
    }

    /// A custom talk table found first in the player's own content (the
    /// game reads it from a userpatch hak or override, `tests/engine_tlk.rs`)
    /// is a warning; in a hak of the module's, nothing.
    #[test]
    fn a_custom_talk_table_only_the_player_has() {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.set("Mod_CustomTlk", Value::String(b"mine".to_vec()));
        m.set_gff(ResKey::parse("module", ResType::IFO).unwrap(), &ifo).unwrap();
        let found = TalkTables { base: 100, custom: Some(3) };
        let table = || {
            let mut c = MemContainer::new();
            c.insert(ResKey::parse("mine", ResType::TLK).unwrap(), b"TLK V3.0".to_vec());
            c
        };
        for (p, place) in
            [(priority::USERPATCH, "userpatch.ini's haks"), (priority::OVERRIDE, "your override")]
        {
            let mut rm = resman(&[]);
            rm.add(p, "theirs", LayerClass::Directory, table());
            let f = examine(&m, &rm, found);
            assert_eq!(checks(&f), [("custom-tlk", "Custom Tlk".into())]);
            assert_eq!(f[0].severity, Severity::Warning);
            assert!(f[0].message.contains(place), "{}", f[0].message);
            // With the module's hak too: the module's own.
            rm.add(priority::HAK_USER, "mine.hak", LayerClass::Erf, table());
            assert!(examine(&m, &rm, found).is_empty());
        }
    }

    /// An archive whose last resources sit past 2 GiB.
    #[derive(Debug)]
    struct Big(MemContainer, Vec<ResKey>);

    impl mg_resman::Container for Big {
        fn contains(&self, key: &ResKey) -> bool {
            self.0.contains(key)
        }
        fn read(&self, key: &ResKey) -> Result<std::borrow::Cow<'_, [u8]>, mg_resman::ResError> {
            self.0.read(key)
        }
        fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
            self.0.keys()
        }
        fn len(&self) -> usize {
            self.0.len()
        }
        fn unreadable(&self) -> Vec<ResKey> {
            self.1.clone()
        }
    }

    #[test]
    fn resources_past_2_gib_are_named() {
        let mut rm = resman(&[]);
        let far: Vec<ResKey> = (0..7).map(|i| key(&format!("tex{i}"), ResType::DDS)).collect();
        rm.add(priority::HAK_USER, "big.hak", LayerClass::Erf, Big(MemContainer::new(), far));
        let f = examine(&Module::new(), &rm, TalkTables::default());
        assert_eq!(checks(&f), [("erf-size", String::new())]);
        assert_eq!((f[0].severity, f[0].source.as_str()), (Severity::Error, "big.hak"));
        assert!(
            f[0].message.starts_with("7 resource(s) start past 2 GiB")
                && f[0].message.contains("tex4.dds, and 2 more"),
            "{}",
            f[0].message
        );
    }

    #[test]
    fn a_shorter_copy_hides_another_haks_rows() {
        let short = b"2DA V2.0\n\n   Label ModelName StrRef\n";
        let rm = resman(&[("placeables", ResType::TWODA, short)]);
        // Hiding the game's longer copy is common and not reported.
        assert!(examine(&Module::new(), &rm, TalkTables::default()).is_empty());
        let mut rm = rm;
        let mut lower = MemContainer::new();
        lower.insert(
            key("placeables", ResType::TWODA),
            b"2DA V2.0\n\n   Label ModelName StrRef\n0  Chair plc_chair 5\n".to_vec(),
        );
        rm.add(priority::HAK_USER, "older.hak", LayerClass::Erf, lower);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        assert_eq!(checks(&f), [("2da-shadow", String::new())]);
        assert!(f[0].message.contains("the copy in older.hak it hides has 1"), "{}", f[0].message);
    }

    #[test]
    fn tilesets_name_sections() {
        let set = b"[GENERAL]\nName=TST\nTransition=0\n[TERRAIN TYPES]\nCount=1\n[TERRAIN0]\nName=Grass\n\
[TILES]\nCount=1\n[TILE0]\nModel=tst_a01\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\nDoors=1\n\
[TILE0DOOR0]\nType=7\n[TILE1]\nModel=tst_a02\n\
[GROUPS]\nCount=2\n[GROUP0]\nName=Hut\nRows=1\nColumns=2\nTile0=0\nTile1=5\n[GROUP1]\nName=Hut2\nRows=1\nColumns=1\nTile0=0\n";
        let rm = resman(&[("tst", ResType::SET, set)]);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let mut got = checks(&f);
        got.sort();
        assert_eq!(
            got,
            [
                ("set-count", "[TILES] Count".into()),
                ("set-door", "door Type=7".into()),
                ("set-group", "[GROUP0] Tile1".into()),
                ("set-group", "[GROUP1] Tile0".into()),
                ("set-line-ends", String::new()),
                ("set-model", "[TILE0] Model".into()),
                ("set-transition", "[GENERAL] Transition".into()),
            ]
        );
    }

    /// Holes, as old tilesets merged by hand have: each is named, and the
    /// rest of the tileset is checked as ever (a builder's tileset drew no
    /// tiles at all, though the game draws them: `engine_tileset_holes`).
    /// A tile without its section or a model is no model missing.
    #[test]
    fn a_tileset_s_holes_are_named() {
        let set = "[GENERAL]\nName=TST\nTransition=1\n\
[TERRAIN TYPES]\nCount=3\n[TERRAIN0]\nName=Grass\n[TERRAIN2]\nName=Water\n\
[TILES]\nCount=40\n[TILE0]\nModel=plc_chair\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\n\
[TILE2]\nWalkMesh=msb01\n\
[TILE3]\nModel=PLC_CHAIR\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\n\
[GROUPS]\nCount=0\n"
            .replace('\n', "\r\n");
        let rm = resman(&[("tst", ResType::SET, set.as_bytes())]);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let mut got = checks(&f);
        got.sort();
        assert_eq!(
            got,
            [
                ("set-count", "[TILES] Count".into()),
                ("set-hole", "[TERRAIN1]".into()),
                ("set-hole", "[TILE1]".into()),
                ("set-hole", "[TILE2] Model".into()),
            ],
            "{f:#?}"
        );
        let said = |at: &str| f.iter().find(|f| f.at == at).unwrap().message.clone();
        assert!(said("[TILE1]").contains("no walkmesh"), "{}", said("[TILE1]"));
        assert!(said("[TILES] Count").contains("the last [TILEn] is [TILE3]: none past it"));
        // With a bare LF at each line's end: the module wouldn't load.
        let rm = resman(&[("tst", ResType::SET, set.replace("\r\n", "\n").as_bytes())]);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let lf = f.iter().find(|f| f.check == Check::SetLineEnds.id()).expect("line ends named");
        assert_eq!(lf.severity, Severity::Error);
    }

    /// A model kept as text is read as the game would: an emitter that
    /// crashes it is an error, a keyword it skips a warning (the first
    /// few), and a compiled model is left alone.
    #[test]
    fn a_model_kept_as_text_is_checked() {
        let text = "newmodel fx\nsetsupermodel fx NULL\nbeginmodelgeom fx\n\
node dummy fx\n  parent NULL\nendnode\n\
node emitter spark\n  parent fx\n  p2p 1\n  wibble 1\nendnode\n\
node dummy twin\n  parent fx\nendnode\n\
node dummy twin\n  parent fx\nendnode\n\
node dummy child\n  parent twin\nendnode\n\
endmodelgeom fx\ndonemodel fx\n";
        let compiled = mg_mdl::compile::compile(
            text.replace("p2p 1", "p2p 0").as_bytes(),
            &Default::default(),
        )
        .unwrap()
        .binary;
        let rm =
            resman(&[("fx", ResType::MDL, text.as_bytes()), ("fx_done", ResType::MDL, &compiled)]);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let found: Vec<_> = f.iter().filter(|f| f.check == "mdl-text").collect();
        assert!(found.iter().all(|f| f.resource == key("fx", ResType::MDL)), "{found:?}");
        let errors: Vec<_> = found.iter().filter(|f| f.severity == Severity::Error).collect();
        assert_eq!(errors.len(), 1, "{found:?}");
        assert!(errors[0].message.contains("reference node") && errors[0].at == "line 7");
        let warned = found.iter().filter(|f| f.severity == Severity::Warning).count();
        assert_eq!(warned, 1, "the second twin, not the keyword the game skips: {found:?}");
    }

    #[test]
    fn materials_and_objects() {
        let rm =
            resman(&[("my_mat", ResType::MTR, b"texture0 a_texture_name_too_long\ntexture1 ok\n")]);
        let mut m = Module::new();
        let mut utp = Gff::new(*b"UTP ");
        utp.root.set("Appearance", Value::Dword(9));
        utp.root.set("Faction", Value::Dword(1));
        m.set_gff(key("box", ResType::UTP), &utp).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut p = Struct::new(9);
        p.set("Appearance", Value::Dword(0));
        p.set("Faction", Value::Dword(1));
        p.set("Tag", Value::String(b"CHAIR".to_vec()));
        git.root.set("Placeable List", Value::List(vec![p]));
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &rm, TalkTables::default());
        assert_eq!(
            checks(&f),
            [("mtr-name", "texture0".into()), ("object-row", "box.utp › Appearance".into())]
        );
    }

    /// A waypoint's flag is a waypoint.2da row with a model, as the
    /// previews have it: one that isn't is warned of, a blueprint's or a
    /// placed one's (the game draws no waypoint: only the toolset's flag
    /// is missing); one without the field is not (BioWare's blueprints).
    #[test]
    fn a_waypoint_s_flag_is_a_row_with_a_model() {
        let table =
            b"2DA V2.0\n\n   LABEL RESREF STRREF\n0 **** **** ****\n1 blue gi_waypoint01 5\n";
        let rm =
            resman(&[("waypoint", ResType::TWODA, table), ("gi_waypoint01", ResType::MDL, b"x")]);
        let mut m = Module::new();
        let mut utw = Gff::new(*b"UTW ");
        utw.root.set("Appearance", Value::Byte(1));
        m.set_gff(key("post", ResType::UTW), &utw).unwrap();
        m.set_gff(key("bare", ResType::UTW), &Gff::new(*b"UTW ")).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let placed = |row: u8, tag: &str| {
            let mut s = Struct::new(5);
            s.set("Appearance", Value::Byte(row));
            s.set("Tag", Value::String(tag.as_bytes().to_vec()));
            s
        };
        git.root.set("WaypointList", Value::List(vec![placed(1, "WP_OK"), placed(7, "WP_BAD")]));
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &rm, TalkTables::default());
        let rows: Vec<_> = f.iter().filter(|f| f.check == "object-row").collect();
        let at: Vec<&str> = rows.iter().map(|f| f.at.as_str()).collect();
        assert_eq!(at, ["area › waypoint WP_BAD › Appearance"], "{rows:?}");
        assert!(rows.iter().all(|f| f.severity == Severity::Warning), "{rows:?}");
        assert!(rows[0].message.contains("waypoint.2da has no row 7"), "{rows:?}");
    }

    /// An object of the PC faction is warned of, blueprint or placed, and
    /// one with no faction field (which the game reads as 0); one of
    /// another faction, or a trigger (Aurora writes 0 for one whose
    /// blueprint has none), is not.
    #[test]
    fn objects_of_the_pc_faction_are_warned_of() {
        let mut m = Module::new();
        let mut utc = Gff::new(*b"UTC ");
        utc.root.set("FactionID", Value::Word(0));
        m.set_gff(key("guard", ResType::UTC), &utc).unwrap();
        let mut utt = Gff::new(*b"UTT ");
        utt.root.set("Faction", Value::Dword(0));
        m.set_gff(key("trap", ResType::UTT), &utt).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let placed = |field: &str, v: Value, tag: &str| {
            let mut s = Struct::new(0);
            s.set(field, v);
            s.set("Tag", Value::String(tag.as_bytes().to_vec()));
            s
        };
        git.root
            .set("Placeable List", Value::List(vec![placed("Faction", Value::Dword(0), "BOX")]));
        git.root.set(
            "Door List",
            Value::List(vec![
                placed("Faction", Value::Dword(2), "DOOR"),
                placed("Unused", Value::Dword(0), "GATE"),
            ]),
        );
        git.root.set("TriggerList", Value::List(vec![placed("Faction", Value::Dword(0), "TRAP")]));
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<_> = f.iter().filter(|f| f.check == "pc-faction").collect();
        let at: Vec<&str> = found.iter().map(|f| f.at.as_str()).collect();
        assert_eq!(
            at,
            [
                "area › placeable BOX › Faction",
                "area › door GATE › Faction",
                "guard.utc › FactionID"
            ]
        );
        assert!(found.iter().all(|f| f.severity == Severity::Warning));
        assert!(found[0].message.contains("regard players"), "{found:?}");
        assert!(found[1].message.starts_with("has no faction"), "{found:?}");
        assert!(found[2].message.contains("puts a creature of it in Hostile"), "{found:?}");
        assert_eq!(Check::PcFaction.group(), Group::Objects);
    }

    /// A static placeable whose visual transform changes its model is
    /// warned of (the game client draws it without); a dynamic one, or a
    /// static one whose transform changes nothing, is not.
    #[test]
    fn static_placeables_with_a_visual_transform_are_warned_of() {
        let axis = |v: f32| {
            let mut s = Struct::new(0);
            s.set("ValueTo", Value::Float(v));
            Value::Struct(s)
        };
        let placed = |tag: &str, is_static: u8, scale: f32| {
            let mut s = Struct::new(9);
            s.set("Tag", Value::String(tag.as_bytes().to_vec()));
            s.set("Static", Value::Byte(is_static));
            let mut entry = Struct::new(6);
            entry.set("Scope", Value::Int(0));
            for a in ["ScaleX", "ScaleY", "ScaleZ"] {
                entry.set(a, axis(scale));
            }
            s.set("VisTransformList", Value::List(vec![entry]));
            s
        };
        let mut git = Gff::new(*b"GIT ");
        git.root.set(
            "Placeable List",
            Value::List(vec![
                placed("MIRROR", 1, 0.46),
                placed("DYNAMIC", 0, 0.46),
                placed("PLAIN", 1, 1.0),
            ]),
        );
        let mut m = Module::new();
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<_> = f.iter().filter(|f| f.check == "static-transform").collect();
        let at: Vec<&str> = found.iter().map(|f| f.at.as_str()).collect();
        assert_eq!(at, ["area › placeable MIRROR › Static"]);
        assert_eq!(found[0].severity, Severity::Warning);
        assert!(found[0].message.contains("untick Static"), "{found:?}");
        assert_eq!(Check::StaticTransform.group(), Group::Objects);
    }
}
