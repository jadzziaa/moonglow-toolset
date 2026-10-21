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

use mg_2da::{TwoDa, overlong_rows};
use mg_core::{Codepage, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::{Layer, LayerClass, ResKey, ResMan, priority};
use mg_set::Tileset;
use mg_shader::{Preamble, Stage, assemble, compile, compile_assembled, program_stage};

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
    TwoDaRow,
    TwoDaRead,
    TwoDaLimit,
    TwoDaShadow,
    TwoDaStrRef,
    TwoDaRef,
    TwoDaDescribed,
    ObjectRow,
    AreaVariables,
}

impl Check {
    pub const ALL: [Check; 23] = [
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
        Check::TwoDaRow,
        Check::TwoDaRead,
        Check::TwoDaLimit,
        Check::TwoDaShadow,
        Check::TwoDaStrRef,
        Check::TwoDaRef,
        Check::TwoDaDescribed,
        Check::ObjectRow,
        Check::AreaVariables,
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
            Check::TwoDaRow => "2da-row",
            Check::TwoDaRead => "2da-read",
            Check::TwoDaLimit => "2da-limit",
            Check::TwoDaShadow => "2da-shadow",
            Check::TwoDaStrRef => "2da-strref",
            Check::TwoDaRef => "2da-ref",
            Check::TwoDaDescribed => "2da-described",
            Check::ObjectRow => "object-row",
            Check::AreaVariables => "area-variables",
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
            Check::AreaVariables => "an area's variables are in its GIT, where the game reads them",
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
            Check::MdlText | Check::MtrName | Check::ShaderCompile => Group::Models,
            Check::TwoDaRow
            | Check::TwoDaRead
            | Check::TwoDaLimit
            | Check::TwoDaShadow
            | Check::TwoDaStrRef
            | Check::TwoDaRef
            | Check::TwoDaDescribed => Group::Tables,
            Check::ObjectRow | Check::AreaVariables => Group::Objects,
            Check::CustomTlk | Check::ErfSize => Group::Module,
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

/// Takes what the groups in `skip` found out of Verify's report: each of
/// those groups, in [`Group::ALL`]'s order, with how many it had (none
/// too), for the report to say what it leaves out.
pub fn leave_out(
    missing: &mut Vec<crate::verify::Missing>,
    findings: &mut Vec<Finding>,
    skip: &[Group],
) -> Vec<(Group, usize)> {
    let mut left: BTreeMap<Group, usize> = skip.iter().map(|g| (*g, 0)).collect();
    if let Some(n) = left.get_mut(&Group::References) {
        *n = std::mem::take(missing).len();
    }
    findings.retain(|f| match left.get_mut(&Group::of(f)) {
        Some(n) => {
            *n += 1;
            false
        }
        None => true,
    });
    left.into_iter().collect()
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
            | priority::USERPATCH
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

    fn model_exists(&self, name: &str) -> bool {
        ResRef::from_str(name).is_ok_and(|r| self.rm.contains(&ResKey::new(r, ResType::MDL)))
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
    let mut d = Doctor { rm: resman, tlk, out: Vec::new(), tables: HashMap::new() };
    // The winning copy of every custom resource, by type.
    let layers = resman.layers();
    let mut custom: BTreeMap<ResKey, usize> = BTreeMap::new();
    for (i, layer) in layers.iter().enumerate() {
        if !is_custom(layer) {
            continue;
        }
        for key in layer.container.keys() {
            if resman.find(&key) == Some(i) {
                custom.insert(key, i);
            }
        }
    }
    for layer in layers.iter().filter(|l| is_custom(l)) {
        past_read_limit(&mut d, layer);
    }
    for (key, &layer) in &custom {
        let source = layers[layer].label.clone();
        match key.restype {
            ResType::SET => tileset(&mut d, &source, *key),
            ResType::TWODA => two_da(&mut d, &source, *key, layer),
            ResType::MTR => material(&mut d, &source, *key),
            ResType::MDL => model_text(&mut d, &source, *key),
            _ => {}
        }
    }
    shaders(&mut d, &custom);
    objects(&mut d, module);
    area_variables(&mut d, module);
    described_tables(&mut d, module);
    custom_tlk(&mut d, module);
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
    let mut checked_models = HashSet::new();
    let mut door_tiles: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (i, tile) in set.tiles.iter().enumerate() {
        if !d.model_exists(&tile.model) {
            d.push(
                Error,
                Check::SetModel,
                source,
                key,
                format!("[TILE{i}] Model"),
                format!(
                    "model {} isn't in the haks or the game; painting the tile crashes Aurora",
                    tile.model
                ),
            );
        } else if checked_models.insert(tile.model.to_ascii_lowercase()) {
            tile_model(d, &tile.model);
        }
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
    for (key, &layer) in custom {
        let source = &layers[layer].label;
        let Ok(data) = d.rm.get(key) else { continue };
        match key.restype {
            ResType::MTR => {
                let mtr = mg_image::mtr::Mtr::parse(&data);
                let named = [
                    (mtr.shader_vs, Stage::Vertex, "customshaderVS"),
                    (mtr.shader_fs, Stage::Fragment, "customshaderFS"),
                ];
                for (name, stage, field) in named {
                    if let Some(name) = name {
                        wanted.entry((name, stage)).or_insert((source.clone(), *key, field));
                    }
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
        assert_eq!(checks(&f), [("shader-compile", "customshaderFS".to_string())]);
        assert!(f[0].message.contains("fs_gone"), "{}", f[0].message);
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
        // Left out: counted, the rest kept in order.
        let mut findings = vec![finding("set-model"), finding("2da-row"), finding("set-door")];
        let mut missing = Vec::new();
        let left = leave_out(&mut missing, &mut findings, &[Group::Tilesets, Group::Models]);
        assert_eq!(left, [(Group::Tilesets, 2), (Group::Models, 0)]);
        assert_eq!(checks(&findings), [("2da-row", String::new())]);
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
}
