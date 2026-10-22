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
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use mg_2da::{TwoDa, overlong_rows};
use mg_core::{Codepage, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::{Layer, LayerClass, ResKey, ResMan, priority};
use mg_set::Tileset;
use mg_shader::{Preamble, Stage, assemble, compile, compile_assembled, program_stage};
use rayon::prelude::*;

use crate::Module;

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
    SetCount,
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
    AreaVariables,
    TagUnique,
    NuiSources,
    VarType,
    VarValue,
    VarObject,
    VarName,
    VarRuntime,
    ClaimsMerged,
    ClaimsTag,
}

impl Check {
    pub const ALL: [Check; 34] = [
        Check::CustomTlk,
        Check::ErfSize,
        Check::SetRead,
        Check::SetCount,
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
        Check::AreaVariables,
        Check::TagUnique,
        Check::NuiSources,
        Check::VarType,
        Check::VarValue,
        Check::VarObject,
        Check::VarName,
        Check::VarRuntime,
        Check::ClaimsMerged,
        Check::ClaimsTag,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Check::CustomTlk => "custom-tlk",
            Check::ErfSize => "erf-size",
            Check::SetRead => "set-read",
            Check::SetCount => "set-count",
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
            Check::AreaVariables => "area-variables",
            Check::TagUnique => "tag-unique",
            Check::NuiSources => "nui-sources",
            Check::VarType => "var-type",
            Check::VarValue => "var-value",
            Check::VarObject => "var-object",
            Check::VarName => "var-name",
            Check::VarRuntime => "var-runtime",
            Check::ClaimsMerged => "claims-merged",
            Check::ClaimsTag => "claims-tag",
        }
    }

    /// What the check holds to be true (a finding says where it is not).
    pub fn about(self) -> &'static str {
        match self {
            Check::CustomTlk => "the module's custom talk table is where the game looks for it",
            Check::ErfSize => "an archive's resources start within the 2 GiB the game can read",
            Check::SetRead => "a tileset can be read",
            Check::SetCount => "a tileset's tile and group counts match its sections",
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
            Check::TwoDaRef => "a 2DA's changed rows name rows of other 2DAs that exist",
            Check::TwoDaDescribed => {
                "a table of the module's that has a description (name.2da.toml) holds what it says"
            }
            Check::ObjectRow => "objects use 2DA rows that exist and have what they need",
            Check::PcFaction => "no object belongs to the PC faction, the players' alone",
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
            | Check::AreaVariables
            | Check::TagUnique
            | Check::VarType
            | Check::VarValue
            | Check::VarObject
            | Check::VarName
            | Check::VarRuntime => Group::Objects,
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

/// Takes what the groups in `skip` found out of Verify's report, and says
/// which they are, in [`Group::ALL`]'s order, for the report to name what
/// it left out. A group left out is not checked at all
/// ([`examine_groups`] skips its checks, Verify its missing references
/// and plugins), so nothing is counted of it: this only takes out what
/// was found of it anyway.
pub fn leave_out(
    missing: &mut Vec<crate::verify::Missing>,
    findings: &mut Vec<Finding>,
    skip: &[Group],
) -> Vec<Group> {
    let left: std::collections::BTreeSet<Group> = skip.iter().copied().collect();
    if left.contains(&Group::References) {
        missing.clear();
    }
    findings.retain(|f| !left.contains(&Group::of(f)));
    left.into_iter().collect()
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
    out: Vec<Finding>,
    tables: HashMap<String, Option<TwoDa>>,
}

impl<'a> Doctor<'a> {
    fn push(
        &mut self,
        severity: Severity,
        check: Check,
        source: &str,
        resource: ResKey,
        at: impl Into<String>,
        message: impl Into<String>,
    ) {
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
    examine_groups(module, resman, tlk, &[], &mut Timings::default())
}

/// [`examine`], leaving out the checks of the groups in `skip` (they are
/// not run: a builder's skipped tilesets and models are most of the time
/// a Verify of a module with many haks takes), and timing its parts.
///
/// The checks of each custom tileset, 2DA, material and model run in
/// parallel; the findings are in the order one after another would make.
pub fn examine_groups(
    module: &Module,
    resman: &ResMan,
    tlk: TalkTables,
    skip: &[Group],
    timings: &mut Timings,
) -> Vec<Finding> {
    let runs = |g: Group| !skip.contains(&g);
    let mut d = Doctor { rm: resman, tlk, out: Vec::new(), tables: HashMap::new() };
    // The winning copy of every custom resource, by type.
    let layers = resman.layers();
    // (Looked up in parallel: a module with a hundred haks has hundreds of
    // thousands, each looked for in the layers above its own.)
    let custom = timings.time("the custom resources", || {
        let mut custom: BTreeMap<ResKey, usize> = BTreeMap::new();
        for (i, layer) in layers.iter().enumerate() {
            if !is_custom(layer) {
                continue;
            }
            let keys: Vec<ResKey> = layer.container.keys().collect();
            let won: Vec<ResKey> =
                keys.into_par_iter().filter(|key| resman.find(key) == Some(i)).collect();
            custom.extend(won.into_iter().map(|key| (key, i)));
        }
        custom
    });
    if runs(Group::Module) {
        timings.time("archives' sizes", || {
            for layer in layers.iter().filter(|l| is_custom(l)) {
                past_read_limit(&mut d, layer);
            }
        });
    }
    // Each resource's own checks, by its type's group.
    let kind = |t: ResType| match t {
        ResType::SET => Some(("tilesets", Group::Tilesets)),
        ResType::TWODA => Some(("2das", Group::Tables)),
        ResType::MTR => Some(("materials", Group::Models)),
        ResType::MDL => Some(("models", Group::Models)),
        _ => None,
    };
    let each: Vec<(&ResKey, usize)> = (custom.iter())
        .filter(|(k, _)| kind(k.restype).is_some_and(|(_, g)| runs(g)))
        .map(|(k, l)| (k, *l))
        .collect();
    let started = Instant::now();
    // (A worker's 2DAs are read once for the resources it takes.)
    let found: Vec<(ResKey, Vec<Finding>, Duration)> = each
        .par_iter()
        .map_init(HashMap::new, |tables, &(key, layer)| {
            let started = Instant::now();
            let mut d = Doctor { rm: resman, tlk, out: Vec::new(), tables: std::mem::take(tables) };
            let source = &layers[layer].label;
            match key.restype {
                ResType::SET => tileset(&mut d, source, *key),
                ResType::TWODA => two_da(&mut d, source, *key, layer),
                ResType::MTR => material(&mut d, source, *key),
                ResType::MDL => model_text(&mut d, source, *key),
                _ => {}
            }
            *tables = d.tables;
            (*key, d.out, started.elapsed())
        })
        .collect();
    timings.add("each tileset, 2DA, material and model (in parallel)", started.elapsed());
    // Of each type: how many, their time added up, the longest.
    let mut work: BTreeMap<&str, (usize, Duration, Duration, ResKey)> = BTreeMap::new();
    for (key, out, took) in found {
        d.out.extend(out);
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
    if runs(Group::Models) {
        timings.time("shaders", || shaders(&mut d, &custom));
    }
    if runs(Group::Objects) {
        timings.time("objects' rows", || objects(&mut d, module));
        timings.time("the PC faction", || pc_faction(&mut d, module));
        timings.time("areas' variables", || area_variables(&mut d, module));
        timings.time("unique tags", || unique_tags(&mut d, module));
    }
    if runs(Group::Module) {
        timings.time("merged areas", || merged_areas(&mut d, module));
    }
    if runs(Group::Objects) {
        timings.time("described variables", || described_variables(&mut d, module));
    }
    if runs(Group::Tables) {
        timings.time("described 2DAs", || described_tables(&mut d, module));
    }
    if runs(Group::Module) {
        custom_tlk(&mut d, module);
        timings.time("NUI sources", || nui_sources(&mut d, module));
    }
    d.out.sort_by(|a, b| {
        (a.severity, &a.source, a.resource).cmp(&(b.severity, &b.source, b.resource))
    });
    d.out
}

/// A custom talk table the module names and the game can't find: the game
/// then won't load the module (`tests/engine_tlk.rs`).
fn custom_tlk(d: &mut Doctor, module: &Module) {
    let Some(name) = module.custom_tlk().ok().flatten().filter(|n| !n.trim().is_empty()) else {
        return;
    };
    if d.tlk.custom.is_some() {
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
    let mut d =
        Doctor { rm: resman, tlk: TalkTables::default(), out: Vec::new(), tables: HashMap::new() };
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
    // Sections past the counts: Aurora's Range Check Error.
    for (list, item, n) in
        [("TILES", "TILE", set.tiles.len()), ("GROUPS", "GROUP", set.groups.len())]
    {
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
    let exists: Vec<bool> = set.tiles.par_iter().map(|t| model_exists(rm, &t.model)).collect();
    let mut checked_models = HashSet::new();
    let firsts: Vec<(usize, bool)> = (set.tiles.iter().zip(&exists).enumerate())
        .filter(|(_, (tile, exists))| {
            !**exists || checked_models.insert(tile.model.to_ascii_lowercase())
        })
        .map(|(i, (_, exists))| (i, *exists))
        .collect();
    let examined: Vec<Vec<Finding>> = firsts
        .par_iter()
        .map(|&(i, exists)| {
            let mut m = Doctor { rm, tlk: d.tlk, out: Vec::new(), tables: HashMap::new() };
            if exists {
                tile_model(&mut m, &set.tiles[i].model);
            }
            m.out
        })
        .collect();
    for (&(i, exists), found) in firsts.iter().zip(examined) {
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
    // (Groups' tiles past the list are reported above.)
    for w in
        set.warnings.iter().filter(|w| !(w.starts_with("group ") && w.ends_with("does not exist")))
    {
        d.push(Warning, Check::SetData, source, key, "", w.clone());
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
    let Ok(data) = d.rm.get(&key) else { return };
    let model = match mg_mdl::Model::read(&data) {
        Ok(m) => m,
        Err(e) => {
            d.push(Error, Check::MdlRead, &source, key, "", format!("can't be read: {e}"));
            return;
        }
    };
    // (Walkmeshes reaching past the tile were tried and left out: the
    // game's own tiles have them, out to 10 m.)
    let faces: usize =
        model.nodes.iter().filter_map(|n| n.mesh()).map(|m| m.triangles().count()).sum();
    if faces > 10_000 {
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

/// A material: texture names longer than a resource name can be (Aurora's
/// "Pure virtual function called").
/// A custom model kept as text (ASCII), by `mg_mdl::lint`: what the game
/// would refuse or crash on is an error, what it would misread a warning
/// (the first few of a model: a hak of thousands may share one habit;
/// not the keywords it skips).
/// Compiled models have nothing to read so.
fn model_text(d: &mut Doctor, source: &str, key: ResKey) {
    const WARNINGS: usize = 5;
    let Ok(data) = d.rm.get(&key) else { return };
    if mg_mdl::is_binary(&data) {
        return;
    }
    let text = String::from_utf8_lossy(&data);
    let mut warnings = 0;
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
        let at = format!("line {}", found.line + 1);
        d.push(severity, Check::MdlText, source, key, at, found.message);
    }
    if warnings > WARNINGS {
        let more = format!("and {} more warnings of this model's text", warnings - WARNINGS);
        d.push(Severity::Warning, Check::MdlText, source, key, "", more);
    }
}

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
/// it. Compiled as the game at its highest settings has them
/// ([`Preamble::default`]). A program's first error is reported; one found
/// in many programs is one finding.
fn shaders(d: &mut Doctor, custom: &BTreeMap<ResKey, usize>) {
    let layers = d.rm.layers();
    // A program, and where it was asked for: source, resource, field.
    let mut wanted: BTreeMap<(String, Stage), (String, ResKey, &str)> = BTreeMap::new();
    let mut replaces_stock = false;
    // Materials that name one program alone, by source: the field and name.
    let mut lone: BTreeMap<String, Vec<(ResKey, &str, String)>> = BTreeMap::new();
    // (Materials and shader files alone are read: the haks' textures and
    // models are most of their size.)
    for (key, &layer) in
        custom.iter().filter(|(k, _)| matches!(k.restype, ResType::MTR | ResType::SHD))
    {
        let source = &layers[layer].label;
        let Ok(data) = d.rm.get(key) else { continue };
        match key.restype {
            ResType::MTR => {
                let mtr = mg_image::mtr::Mtr::parse(&data);
                let name = |n: Option<String>| {
                    n.map(|n| n.trim().to_ascii_lowercase())
                        .filter(|n| !n.is_empty() && n != "null")
                };
                let named = [
                    (name(mtr.shader_vs), Stage::Vertex, "customshaderVS"),
                    (name(mtr.shader_fs), Stage::Fragment, "customshaderFS"),
                ];
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
            ResType::SHD => {
                let name = key.resref.to_lowercase().to_string();
                match program_stage(&name, &data) {
                    Some(stage) => {
                        wanted.entry((name, stage)).or_insert((source.clone(), *key, ""));
                    }
                    None => {
                        replaces_stock |=
                            layers.iter().any(|l| !is_custom(l) && l.container.contains(key))
                    }
                }
            }
            _ => {}
        }
    }
    let custom_layer = |label: &str| layers.iter().any(|l| l.label == label && is_custom(l));
    let pre = Preamble::default();
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
    for ((name, stage), (source, key, field)) in &wanted {
        if let Err(e) = compile(name, *stage, &pre, d.rm) {
            record(e, format!("{name} ({stage})"), (source, *key, field));
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
        for (name, stage, key) in stock {
            let Ok(a) = assemble(&name, stage, &pre, d.rm) else { continue };
            if !a.uses_origin(custom_layer) {
                continue;
            }
            if let Err(e) = compile_assembled(&a) {
                let source = d.rm.origin(&key).unwrap_or_default().to_string();
                record(e, format!("{name} ({stage})"), (&source, key, ""));
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

/// Columns that hold a talk-table string, by table.
pub(crate) const STRREF_COLUMNS: [(&str, &str); 10] = [
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
/// game's own tables (`row_columns_hold_for_the_game_s_tables`); `****`
/// and a negative number name none. Left out: columns the game's own
/// tables break (`appearance.FOOTSTEPTYPE`); the tables a class, a race
/// or a package names in a column are in [`NAMED_TABLES`].
pub(crate) const ROW_COLUMNS: [(&str, &str, &str); 61] = [
    ("appearance", "SOUNDAPPTYPE", "appearancesndset"),
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

/// Tables that a column of another names, and their column that names a
/// row: the naming table and its column, the usual start of such a
/// table's name, its column, the table whose row that is. (A class's
/// feats are in the table its `FeatsTable` names, `cls_feat_*` in the
/// game's own.)
pub(crate) const NAMED_TABLES: [(&str, &str, &str, &str, &str); 6] = [
    ("classes", "FeatsTable", "cls_feat_", "FeatIndex", "feat"),
    ("classes", "SkillsTable", "cls_skill_", "SkillIndex", "skills"),
    ("racialtypes", "FeatsTable", "race_feat_", "FeatIndex", "feat"),
    ("packages", "FeatPref2DA", "packft", "FeatIndex", "feat"),
    ("packages", "SkillPref2DA", "packsk", "SkillIndex", "skills"),
    ("packages", "SpellPref2DA", "packsp", "SpellIndex", "spells"),
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
}

/// The columns of the table `name` (without `.2da`) that the checks know
/// the kind of: the same tables Verify Module reads them from. (A table
/// that a class, a race or a package names is known here by how the
/// game's own are named, `cls_feat_*` and the like; Verify Module goes
/// by the column that names it too.)
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
        .filter(move |(_, _, start, _, _)| lower.starts_with(start))
        .map(|(_, _, _, c, target)| (*c, ColumnKind::Row(target)));
    of(&STRREF_COLUMNS, ColumnKind::StrRef)
        .chain(of(&MODEL_COLUMNS, ColumnKind::Model))
        .chain(rows)
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

/// [`column_kinds`], and for a table that a class, a race or a package
/// names in a column (`table` reads a table as the game has it), whatever
/// its own name, the columns such a table has.
pub fn column_kinds_as_named(
    name: &str,
    table: &dyn Fn(&str) -> Option<std::sync::Arc<TwoDa>>,
) -> Vec<(&'static str, ColumnKind)> {
    let mut kinds = column_kinds(name);
    for (naming, by, _, column, target) in NAMED_TABLES {
        let named = table(naming).is_some_and(|t| {
            (0..t.len()).any(|r| t.get(r, by).is_some_and(|n| n.eq_ignore_ascii_case(name)))
        });
        if named && !kinds.contains(&(column, ColumnKind::Row(target))) {
            kinds.push((column, ColumnKind::Row(target)));
        }
    }
    kinds
}

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
/// that is not there (`rows_of`: how many rows a table has, as the module
/// sees it; the table itself is asked of itself), a talk-table string
/// that is not there. A table being edited is checked by this as it is
/// typed.
pub fn table_faults(
    name: &str,
    t: &TwoDa,
    kinds: &[(&'static str, ColumnKind)],
    rows_of: &dyn Fn(&str) -> Option<usize>,
    tlk: TalkTables,
) -> Vec<CellFault> {
    let mut out = Vec::new();
    for (column, kind) in kinds {
        let Some(at) = t.column(column) else { continue };
        let length = match kind {
            ColumnKind::Row(target) if target.eq_ignore_ascii_case(name) => Some(t.len()),
            ColumnKind::Row(target) => rows_of(target),
            _ => None,
        };
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
                ColumnKind::Model => None,
                ColumnKind::Row(target) => mg_2da::parse_int(text)
                    .and_then(|v| row_named(name, column, v))
                    .zip(length)
                    .filter(|(named, length)| named >= length)
                    .map(|(named, length)| {
                        format!("row {named} of {target}.2da doesn't exist: it has {length} rows")
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
    for (t, column) in STRREF_COLUMNS {
        if name != t {
            continue;
        }
        for row in 0..table.len() {
            if !changed(row) {
                continue;
            }
            let Some(v) = table.get_int(row, column) else { continue };
            let Ok(v) = u32::try_from(v) else { continue };
            if let Some(p) = strref_problem(v, d.tlk) {
                d.push(
                    Error,
                    Check::TwoDaStrRef,
                    source,
                    key,
                    format!("row {row}, {column}"),
                    format!("string {v} doesn't exist: {p} (\"Bad Strref\")"),
                );
            }
        }
    }
    // Rows of other tables (as the module has them, this hak's and the
    // others' over the game's) that its changed rows name.
    let rows: Vec<usize> = (0..table.len()).filter(|r| changed(*r)).collect();
    // (Its own columns, and those it has as a table that a class, a race
    // or a package names: by that column, or by its name as the game's
    // own are named.)
    let mut columns: Vec<(&str, &str)> =
        ROW_COLUMNS.iter().filter(|(t, _, _)| name == *t).map(|(_, c, to)| (*c, *to)).collect();
    for (naming, by, start, column, target) in NAMED_TABLES {
        let is_named = name.starts_with(start)
            || d.table(naming).is_some_and(|t| {
                (0..t.len()).any(|r| t.get(r, by).is_some_and(|n| n.eq_ignore_ascii_case(&name)))
            });
        if is_named && !columns.contains(&(column, target)) {
            columns.push((column, target));
        }
    }
    for (column, target) in columns {
        let t = name.as_str();
        if table.column(column).is_none() {
            continue;
        }
        let Some(length) = d.table(target).map(TwoDa::len) else { continue };
        for &row in &rows {
            let named = table.get_int(row, column).and_then(|v| row_named(t, column, v));
            if let Some(named) = named.filter(|n| *n >= length) {
                d.push(
                    Warning,
                    Check::TwoDaRef,
                    source,
                    key,
                    format!("row {row}, {column}"),
                    format!("row {named} of {target}.2da doesn't exist: it has {length} rows"),
                );
            }
        }
    }
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
/// with a class that isn't there crashes Aurora and the game).
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
        // blueprint breaks only when placed.
        let severity =
            if key.restype == ResType::GIT { Severity::Error } else { Severity::Warning };
        for (path, message) in found {
            let place = crate::rename::describe(key, Some(&gff), &path);
            d.push(severity, Check::ObjectRow, "module", key, place, message);
        }
    }
}

/// Creatures, placeables, doors and encounters (blueprints and placed) of
/// faction 0, the PC faction: the players' alone (Jasperre's report). The
/// game puts a creature of it in Hostile as it loads; a placeable or door
/// keeps it, and creatures regard it as they regard players
/// (`engine_factions.rs`). Triggers are left out (the owner's decision):
/// Aurora writes 0 for a trigger whose blueprint has no faction, so the
/// warning would fall on ordinary triggers. A field the object lacks is
/// not reported.
fn pc_faction(d: &mut Doctor, module: &Module) {
    use mg_rules::ObjectKind;
    let of_pc = |kind: ObjectKind, s: &Struct| -> Option<&'static str> {
        if kind == ObjectKind::Trigger {
            return None;
        }
        let field = kind.faction_field()?;
        (s.integer(field) == Some(mg_rules::objects::PC_FACTION)).then_some(field)
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
                    if let Some(field) = of_pc(kind, s) {
                        found.push((kind, format!("/{label}[{i}]/{field}")));
                    }
                }
            }
        } else if let Some(kind) = ObjectKind::from_restype(key.restype)
            && let Some(field) = of_pc(kind, &gff.root)
        {
            found.push((kind, format!("/{field}")));
        }
        for (kind, path) in found {
            let place = crate::rename::describe(key, Some(&gff), &path);
            let what = match kind {
                ObjectKind::Creature => "the game puts a creature of it in Hostile",
                ObjectKind::Encounter => "an encounter's own faction",
                _ => "creatures regard it as they regard players",
            };
            let message = format!(
                "of the PC faction (0), which is the players' alone ({what}): give it another \
                 faction"
            );
            d.push(Severity::Warning, Check::PcFaction, "module", key, place, message);
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
    let file = var_desc::FILE_NAME;
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
        for f in var_desc::variables_of(key, &gff) {
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
                let how =
                    if desc.is_pattern() { format!(" ({})", desc.name) } else { String::new() };
                let message = format!(
                    "{}{how} is set by the scripts while the server runs ({file}:{} says runtime \
                     = true), and is never to be kept in a file: one kept here is there from the \
                     start, on every server",
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
        if let Err(e) = at.model(d.table(at.table), rm) {
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
        let left = leave_out(&mut missing, &mut findings, &[Group::Models, Group::Tilesets]);
        assert_eq!(left, [Group::Tilesets, Group::Models]);
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
        rm.add(priority::HAK_USER, "watched.hak", LayerClass::Erf, Watched(hak, read.clone()));
        let run = |skip: &[Group]| {
            read.lock().unwrap().clear();
            let f = examine_groups(
                &Module::new(),
                &rm,
                TalkTables::default(),
                skip,
                &mut Timings::default(),
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
        let (left, read_left) = run(&[Group::Tilesets, Group::Models]);
        assert!(left.iter().all(|c| !c.starts_with("set-") && !c.starts_with("mdl-")));
        assert!(!left.contains(&"mtr-name".to_string()), "{left:?}");
        assert_eq!(read_left, [ResType::TWODA], "only the 2DA is read");
        // The 2DAs' group left out too: nothing of the hak is read.
        let (_, read_none) = run(&[Group::Tilesets, Group::Models, Group::Tables]);
        assert!(read_none.is_empty(), "{read_none:?}");
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
             [\"tdod_*_d20\"]\ntype = \"int\"\nruntime = true\n",
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
        // Those that are right, and the others' variables, are not said.
        assert_eq!(found.len(), 10, "{found:#?}");
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
            &|name| (name == "spells").then_some(3),
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
                ("set-model", "[TILE0] Model".into()),
                ("set-transition", "[GENERAL] Transition".into()),
            ]
        );
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
        m.set_gff(key("box", ResType::UTP), &utp).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut p = Struct::new(9);
        p.set("Appearance", Value::Dword(0));
        p.set("Tag", Value::String(b"CHAIR".to_vec()));
        git.root.set("Placeable List", Value::List(vec![p]));
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &rm, TalkTables::default());
        assert_eq!(
            checks(&f),
            [("mtr-name", "texture0".into()), ("object-row", "box.utp › Appearance".into())]
        );
    }

    /// An object of the PC faction is warned of, blueprint or placed; one
    /// of another faction, with no faction field, or a trigger (Aurora
    /// writes 0 for one whose blueprint has none), is not.
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
            Value::List(vec![placed("Faction", Value::Dword(2), "DOOR"), Struct::new(8)]),
        );
        git.root.set("TriggerList", Value::List(vec![placed("Faction", Value::Dword(0), "TRAP")]));
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<_> = f.iter().filter(|f| f.check == "pc-faction").collect();
        let at: Vec<&str> = found.iter().map(|f| f.at.as_str()).collect();
        assert_eq!(at, ["area › placeable BOX › Faction", "guard.utc › FactionID"]);
        assert!(found.iter().all(|f| f.severity == Severity::Warning));
        assert!(found[0].message.contains("regard players"), "{found:?}");
        assert!(found[1].message.contains("puts a creature of it in Hostile"), "{found:?}");
        assert_eq!(Check::PcFaction.group(), Group::Objects);
    }
}
