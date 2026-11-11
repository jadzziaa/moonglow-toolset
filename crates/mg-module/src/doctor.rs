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
use mg_gff::{GffPath, Step, Struct, Value};
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
    MtrTexture,
    ShaderCompile,
    ShaderPair,
    TwoDaRow,
    TwoDaRead,
    TwoDaLimit,
    TwoDaShadow,
    TwoDaStrRef,
    TwoDaRef,
    TwoDaDescribed,
    TwoDaSpells,
    ObjectRow,
    PcFaction,
    StaticTransform,
    StaticTint,
    TintStored,
    AppearanceRange,
    AreaVariables,
    TileReplace,
    TileSeam,
    SoundSilent,
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
    TalkRange,
    PaletteRules,
    PaletteGroupEmpty,
    PaletteListMissing,
    BakeRules,
    BakeRowPending,
    BakeUnused,
}

impl Check {
    pub const ALL: [Check; 54] = [
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
        Check::MtrTexture,
        Check::ShaderCompile,
        Check::ShaderPair,
        Check::TwoDaRow,
        Check::TwoDaRead,
        Check::TwoDaLimit,
        Check::TwoDaShadow,
        Check::TwoDaStrRef,
        Check::TwoDaRef,
        Check::TwoDaDescribed,
        Check::TwoDaSpells,
        Check::ObjectRow,
        Check::PcFaction,
        Check::StaticTransform,
        Check::StaticTint,
        Check::TintStored,
        Check::AppearanceRange,
        Check::AreaVariables,
        Check::TileReplace,
        Check::TileSeam,
        Check::SoundSilent,
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
        Check::TalkRange,
        Check::PaletteRules,
        Check::PaletteGroupEmpty,
        Check::PaletteListMissing,
        Check::BakeRules,
        Check::BakeRowPending,
        Check::BakeUnused,
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
            Check::MtrTexture => "mtr-texture",
            Check::ShaderCompile => "shader-compile",
            Check::ShaderPair => "shader-pair",
            Check::TwoDaRow => "2da-row",
            Check::TwoDaRead => "2da-read",
            Check::TwoDaLimit => "2da-limit",
            Check::TwoDaShadow => "2da-shadow",
            Check::TwoDaStrRef => "2da-strref",
            Check::TwoDaRef => "2da-ref",
            Check::TwoDaDescribed => "2da-described",
            Check::TwoDaSpells => "2da-spells",
            Check::ObjectRow => "object-row",
            Check::PcFaction => "pc-faction",
            Check::StaticTransform => "static-transform",
            Check::StaticTint => "static-tint",
            Check::TintStored => "tint-stored",
            Check::AppearanceRange => "appearance-range",
            Check::AreaVariables => "area-variables",
            Check::TileReplace => "tile-replace",
            Check::TileSeam => "tile-seam",
            Check::SoundSilent => "sound-silent",
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
            Check::TalkRange => "talk-range",
            Check::PaletteRules => "palette-rules",
            Check::PaletteGroupEmpty => "palette-group-empty",
            Check::PaletteListMissing => "palette-list-missing",
            Check::BakeRules => "bake-rules",
            Check::BakeRowPending => "bake-row-pending",
            Check::BakeUnused => "bake-unused",
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
                "a tileset's lines end in CRLF and have at most 255 characters (with a bare LF \
                 the game's server never finishes loading a module that uses it, nor with more \
                 than 256 bytes after its last CRLF, nor with a longer line)"
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
            Check::MtrTexture => "a material's textures are in the load order",
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
                "a table of the module's or its haks' that has a description (name.2da.toml) \
                 holds what it says"
            }
            Check::TwoDaSpells => {
                "a spellcasting class's spells: its SpellTableColumn is a column of spells.2da, \
                 and the spells its packages prefer are ones it can learn"
            }
            Check::ObjectRow => "objects use 2DA rows that exist and have what they need",
            Check::PcFaction => "no object belongs to the PC faction, the players' alone",
            Check::StaticTransform => {
                "a static placeable has no visual transform: the game draws it without one"
            }
            Check::StaticTint => {
                "a static placeable has no tint (The Domains of Dread's tdod_tint variables): \
                 the game draws it as made"
            }
            Check::TintStored => {
                "a material's tint is set in one place: a tdodTint stored in the object's shader \
                 parameters isn't also given by its tdod_tint variables, which the module's \
                 script sets over it as the module loads"
            }
            Check::AppearanceRange => {
                "a placeable's Appearance is a placeables.2da row the game can carry, 0 to 65535 \
                 (it carries the row as a 16-bit number and draws a placeable of row 65536 as \
                 row 0's: another model, silently)"
            }
            Check::AreaVariables => "an area's variables are in its GIT, where the game reads them",
            Check::TileReplace => {
                "a tile whose model has a replace_tex texture names a replacetexture.2da row \
                 (Tile_ReplaceTex, one past the row: the game crashes as it loads the area on 0 \
                 or a value past the table)"
            }
            Check::TileSeam => {
                "an area's neighboring tiles agree about the corners and edge they share: its \
                 terrain and height, and its crosser (a road, a stream), as Aurora's painting \
                 leaves them"
            }
            Check::SoundSilent => {
                "an area's music and ambient sounds (its MusicDay, MusicNight and MusicBattle \
                 rows of ambientmusic.2da, its AmbientSndDay and AmbientSndNight rows of \
                 ambientsound.2da) and the sounds of its placed sound objects and of sound \
                 blueprints are files the game finds and plays there: music a .bmu holding MP3, \
                 an ambient sound a .wav holding a WAVE or a .bmu holding MP3, a placed sound a \
                 .wav holding a WAVE (mono where the sound is positional) or MP3 behind the BMU \
                 V1.0 header"
            }
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
            Check::TalkRange => {
                "the custom talk table's lines the project gives typed text (moonglow.toml's \
                 [talk]) have room, are not mg tlk-alloc's, and each builder has a part of their \
                 own that no other shares"
            }
            Check::PaletteRules => {
                "the project's palette groupings (moonglow.toml's [palette.*]) and the lists \
                 their from files hold read without a fault (a grouping works without what \
                 isn't understood)"
            }
            Check::PaletteGroupEmpty => {
                "every group of a palette grouping has a blueprint in it, in the standard or the \
                 custom palettes it is offered in (by its words, its ResRef patterns or its list)"
            }
            Check::PaletteListMissing => {
                "a blueprint a palette grouping's from file lists is in the module, its haks or \
                 the game, of a type the group is offered in"
            }
            Check::BakeRules => {
                "the project's [bake] (moonglow.toml) and its rows file read without a fault, \
                 its rows end by 65535 (the last the game carries), and they are rows the game's \
                 placeables.2da doesn't hold already (a bake never takes one)"
            }
            Check::BakeRowPending => {
                "each row of the bake's rows file is in the placeables.2da the module's haks give \
                 the game, with its baked model: until the project's build adds it, the game has \
                 no such row (Moonglow draws it as the build will make it)"
            }
            Check::BakeUnused => {
                "every row of the bake's rows file, and every baked model in its hak, is used by \
                 a placeable of this module (placed or a blueprint); another module or a hak's \
                 blueprint may still use one, so none is taken out unasked"
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
            Check::MdlText
            | Check::MtrName
            | Check::MtrTexture
            | Check::ShaderCompile
            | Check::ShaderPair => Group::Models,
            Check::TwoDaRow
            | Check::TwoDaRead
            | Check::TwoDaLimit
            | Check::TwoDaShadow
            | Check::TwoDaStrRef
            | Check::TwoDaRef
            | Check::TwoDaDescribed
            | Check::TwoDaSpells => Group::Tables,
            Check::ObjectRow
            | Check::PcFaction
            | Check::StaticTransform
            | Check::StaticTint
            | Check::TintStored
            | Check::AppearanceRange
            | Check::AreaVariables
            | Check::TileReplace
            | Check::TileSeam
            | Check::SoundSilent
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
            | Check::ClaimsTag
            | Check::TalkRange
            | Check::PaletteRules
            | Check::PaletteGroupEmpty
            | Check::PaletteListMissing
            | Check::BakeRules
            | Check::BakeRowPending
            | Check::BakeUnused => Group::Module,
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
    /// Missing references found only in the player's own content of these
    /// kinds (`references:override`; `references:local`, all three).
    locals: BTreeSet<crate::verify::Local>,
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
        locals: BTreeSet::new(),
        sources: BTreeSet::new(),
        resources: BTreeSet::new(),
        words: Vec::new(),
    };

    /// The words `mg verify --skip` takes beside groups and checks, for its
    /// help and its errors.
    pub const WORDS: &'static str = "a group (references, tilesets, models, 2das, objects, \
         module, plugins), a check's id (`mg checks`), references:CATEGORY (creatures, doors, \
         placeables, items, sounds, triggers, waypoints, stores, conversations, encounters, \
         areas, module, scripts, other), references:override, references:userpatch, \
         references:workshop or references:local (what only the player's own content has), a hak (hak:NAME or NAME.hak), override, development, \
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
            use crate::verify::Local;
            let locals: &[Local] = match c {
                "local" => &[Local::Override, Local::Userpatch, Local::Workshop],
                "override" => &[Local::Override],
                "userpatch" => &[Local::Userpatch],
                "workshop" => &[Local::Workshop],
                _ => &[],
            };
            if !locals.is_empty() {
                self.locals.extend(locals);
                self.words.push(word);
                return Ok(());
            }
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

    /// Whether a missing reference is: the group, its category, the
    /// player's own content that alone has it, or the resource that makes
    /// it.
    pub fn missing(&self, m: &crate::verify::Missing) -> bool {
        self.group(Group::References)
            || self.categories.contains(&m.category)
            || m.only_local.is_some_and(|l| self.locals.contains(&l))
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

/// The project file's name as findings say it: its own, or
/// `moonglow.toml` where the module has none.
fn project_file_name(pf: &crate::project_file::ProjectFile) -> String {
    pf.path
        .as_ref()
        .and_then(|p| p.file_name())
        .map_or_else(|| crate::project_file::FILE_NAME.to_string(), |n| n.to_string_lossy().into())
}

/// What a script reads of a variable that isn't set: a type's empty value,
/// as findings say it.
fn empty(t: crate::vars::VarType) -> &'static str {
    use crate::vars::VarType;
    match t {
        VarType::Int => "0",
        VarType::Float => "0.0",
        VarType::String => "\"\"",
        VarType::Object => "OBJECT_INVALID",
        VarType::Location => "an invalid location",
    }
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

    /// The user's ([`crate::build::cache_dir`]; its shaders in
    /// [`crate::build::shader_cache_dir`]).
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
    /// The 2DAs this doctor has looked at, in front of [`Doctor::tables_read`].
    tables: HashMap<String, Option<std::sync::Arc<TwoDa>>>,
    /// Every 2DA read in this run, by name: shared by the doctors beside
    /// this one, so that each is parsed once a run (a worker's own were
    /// begun again at each of rayon's splits of the module's resources:
    /// a persistent world's 17,000-row appearance.2da parsed hundreds of
    /// times, ten times the run's whole time).
    tables_read: TablesRead,
    /// Why each model an object names couldn't be read (`None`: it was,
    /// or it is the game's own), read once a run: shared by the doctors
    /// beside this one, which look at the module's objects on other cores.
    models_read: ModelsRead,
}

/// [`Doctor::tables_read`].
type TablesRead = std::sync::Arc<std::sync::Mutex<HashMap<String, Option<std::sync::Arc<TwoDa>>>>>;

/// [`Doctor::models_read`].
type ModelsRead = std::sync::Arc<std::sync::Mutex<HashMap<ResKey, Option<String>>>>;

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
            tables_read: self.tables_read.clone(),
            models_read: self.models_read.clone(),
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
            let read = &self.tables_read;
            let known = read.lock().unwrap_or_else(|e| e.into_inner()).get(name).cloned();
            let t = known.unwrap_or_else(|| {
                // (Parsed with the lock let go: another doctor may read
                // another table meanwhile; the first one in is kept.)
                let t = ResRef::from_str(name)
                    .ok()
                    .and_then(|r| self.rm.get(&ResKey::new(r, ResType::TWODA)).ok())
                    .and_then(|d| TwoDa::parse(&d, Codepage::default()).ok())
                    .map(std::sync::Arc::new);
                let mut kept = read.lock().unwrap_or_else(|e| e.into_inner());
                kept.entry(name.to_string()).or_insert(t).clone()
            });
            self.tables.insert(name.to_string(), t);
        }
        self.tables[name].as_deref()
    }

    /// [`table`](Self::table), to hand on.
    fn table_shared(&mut self, name: &str) -> Option<std::sync::Arc<TwoDa>> {
        self.table(name)?;
        self.tables[name].clone()
    }

    /// Why model `name` (an object's appearance's or one of its parts')
    /// can't be read, where it is the module's or a hak's (the game's own
    /// are taken as they are, as a hak left out is): read whole once, and
    /// not again while its archive is as it was.
    fn unreadable(&mut self, name: &str) -> Option<String> {
        let key = ResKey::parse(name, ResType::MDL)?;
        let known = self.models_read.lock().ok().and_then(|read| read.get(&key).cloned());
        if let Some(known) = known {
            return known;
        }
        let why = self.rm.find(&key).and_then(|layer| {
            let l = &self.rm.layers()[layer];
            if !is_custom(l) || self.skip.source(&l.label) {
                return None;
            }
            let facts = self.known(layer, &key);
            if let Some(read) = facts.and_then(|f| f.read.clone()) {
                return read;
            }
            if let Some(Err(e)) = facts.and_then(|f| f.faces.clone()) {
                return Some(e);
            }
            let data = self.rm.get(&key).ok()?;
            let read = mg_mdl::Model::read(&data).err().map(|e| e.to_string());
            self.learn(layer, key, Facts { read: Some(read.clone()), ..Facts::default() });
            read
        });
        if let Ok(mut read) = self.models_read.lock() {
            read.insert(key, why.clone());
        }
        why
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
/// parallel, and so do those of the module's areas, objects and
/// blueprints, each of which is read once for all of them
/// ([`resources_once`]); the findings are in the order one after another
/// would make.
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
        tables_read: TablesRead::default(),
        models_read: ModelsRead::default(),
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
                // (The copy the game reads: the pending baked rows' layer
                // is over the haks, and is not one.)
                .filter(|key| {
                    !skip.resource(key) && crate::bake::layer_read(resman, key) == Some(i)
                })
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
        ResType::MTR => runs(&[MtrName, MtrTexture]),
        ResType::MDL => runs(&[MdlText]),
        _ => false,
    };
    let each: Vec<(&ResKey, usize)> =
        (custom.iter()).filter(|(k, _)| checked(k.restype)).map(|(k, l)| (k, *l)).collect();
    let started = Instant::now();
    // (A 2DA is read once a run, for every worker.)
    let tables_read = d.tables_read.clone();
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
                tables_read: tables_read.clone(),
                models_read: ModelsRead::default(),
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
    // The module's areas, objects and blueprints: each read once, on
    // every core, and handed to every check that reads its type
    // ([`resources_once`]). What the checks after the unique tags and the
    // merged areas found is said after those, as it was when each check
    // read the module for itself.
    let reads = match runs(&[VarScript]) {
        true => {
            timings.time("the variables the scripts name", || crate::var_desc::script_reads(module))
        }
        false => Default::default(),
    };
    let wanted = Once::ALL.map(|o| {
        runs(o.checks())
            && match o {
                Once::Tags => module.project_file.tags.any_unique(),
                Once::Described => !module.project_file.variables.is_empty(),
                Once::Read => !reads.is_empty(),
                _ => true,
            }
    });
    let once = resources_once(&mut d, module, wanted, &reads, timings);
    if runs(&[TagUnique]) {
        timings.time("unique tags", || unique_tags(&mut d, module, once.tags.as_ref()));
    }
    if runs(&[ClaimsMerged, ClaimsTag]) {
        timings.time("merged areas", || merged_areas(&mut d, module));
    }
    once.say_the_rest(&mut d);
    if runs(&[TwoDaDescribed]) {
        timings.time("described 2DAs", || described_tables(&mut d, module));
    }
    if runs(&[TwoDaSpells]) {
        timings.time("classes' spells", || class_spells(&mut d));
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

/// The custom talk table's lines the project gives typed text
/// (`moonglow.toml`'s `[talk]`, [`crate::talk_range`]), held to the
/// table (`table`: the module's custom table, where it was found): room
/// left (a tenth of the range or less is a warning, none an error), no
/// line `mg tlk-alloc`'s map gives inside the range, builders' parts of
/// their own that no other shares, and a part for this builder where the
/// others have one. Every tool says the same (Verify, `mg verify`).
pub fn talk_range(module: &Module, table: Option<&mg_tlk::Tlk>, skip: &Skip) -> Vec<Finding> {
    use crate::talk_range::{self as range, Allowed};
    let pf = &module.project_file;
    let rules = &pf.talk;
    if !rules.is_set() || skip.check(Check::TalkRange) || skip.source("module") {
        return Vec::new();
    }
    let file = project_file_name(pf);
    let mut out = Vec::new();
    let mut push = |severity, message: String| {
        out.push(Finding {
            severity,
            check: Cow::Borrowed(Check::TalkRange.id()),
            source: "module".into(),
            resource: Module::info_key(),
            at: format!("{file} [talk]"),
            message,
        })
    };
    for p in rules.problems() {
        push(Severity::Warning, p);
    }
    let map = match pf.talk_map() {
        Ok(m) => m,
        Err(e) => {
            push(Severity::Warning, e);
            None
        }
    };
    let map_lines = map.as_ref().map(range::map_lines).unwrap_or_default();
    for lines in rules.reserved() {
        let inside: Vec<u32> = map_lines.range(lines.from..=lines.to).copied().collect();
        if let Some(first) = inside.first() {
            push(
                Severity::Warning,
                format!(
                    "{} of the lines typed text takes ({lines}) are mg tlk-alloc's ({}, line \
                     {first} the first): typed text passes over them, and a key given one \
                     elsewhere would share it; keep the two apart",
                    inside.len(),
                    rules.map.as_deref().unwrap_or("the map")
                ),
            );
        }
    }
    let code = pf.user.builder_code.as_deref();
    if !rules.builders.is_empty() && code.is_none_or(|c| !rules.builders.contains_key(c)) {
        push(
            Severity::Warning,
            format!(
                "the builders {} have lines of their own, and you ({}) have none: the text you \
                 type takes from the rest of the range, which another builder without a part \
                 takes from too",
                rules.builders.keys().cloned().collect::<Vec<_>>().join(", "),
                code.map_or_else(|| "with no builder code".to_string(), |c| format!("code {c}"))
            ),
        );
    }
    let allowed = rules.allowed(code);
    if let (Some(t), Allowed::Range { .. }) = (table, &allowed) {
        let blank = |l: u32| {
            t.entries.get(l as usize).is_none_or(|e| {
                e.flags & (mg_tlk::FLAG_TEXT | mg_tlk::FLAG_SOUND) == 0
                    || (e.text.is_empty() && e.sound.is_empty())
            })
        };
        let len = t.entries.len() as u32;
        if let Some((free, total)) = range::free_lines(&allowed, len, blank, &map_lines) {
            let what = allowed.describe();
            if free == 0 {
                push(
                    Severity::Error,
                    format!(
                        "the {what} typed text takes are all used: text typed now stays in the \
                         file (Module Properties › Custom Content sets other lines)"
                    ),
                );
            } else if u64::from(free) * 10 <= u64::from(total) {
                push(
                    Severity::Warning,
                    format!("{free} of the {total} {what} typed text takes are left"),
                );
            }
        }
    }
    out
}

/// The project's palette groupings (`[palette.*]`,
/// [`crate::palette_groups`]), held to the palettes they are offered in:
/// what of their rules and lists wasn't understood (`palette-rules`), a
/// group no blueprint of the standard or custom palettes falls in
/// (`palette-group-empty`), and blueprints a `from` file lists that are
/// nowhere (`palette-list-missing`). `name`: a palette name as the
/// palette shows it (a talk-table string looked up). Every tool says the
/// same (Verify, `mg verify`).
pub fn palette_groups(
    module: &Module,
    resman: &ResMan,
    name: &dyn Fn(&crate::palette::PaletteName) -> String,
    skip: &Skip,
) -> Vec<Finding> {
    use crate::palette::{BlueprintKind, Palette};
    use crate::palette_groups::entries;
    let pf = &module.project_file;
    let groupings = &pf.palette;
    let mut out = Vec::new();
    if (groupings.list.is_empty() && groupings.problems.is_empty()) || skip.source("module") {
        return out;
    }
    let file = project_file_name(pf);
    let mut push = |check: Check, at: String, message: String| {
        if !skip.check(check) {
            out.push(Finding {
                severity: Severity::Warning,
                check: Cow::Borrowed(check.id()),
                source: "module".into(),
                resource: Module::info_key(),
                at,
                message,
            });
        }
    };
    for p in &groupings.problems {
        push(Check::PaletteRules, format!("{file} [palette]"), p.clone());
    }
    // Each kind's palettes, the standard and the custom, read once: their
    // blueprints and names.
    let mut palettes: HashMap<BlueprintKind, Vec<(ResRef, String)>> = HashMap::new();
    let read = |kind: BlueprintKind| -> Vec<(ResRef, String)> {
        let standard = resman
            .get_named(&format!("{}palstd", kind.name()), ResType::ITP)
            .ok()
            .and_then(|d| mg_gff::Gff::read(&d).ok());
        let custom = module.gff(&kind.custom_key()).and_then(Result::ok);
        let mut all = Vec::new();
        for g in standard.iter().chain(custom.iter()) {
            let p = Palette::read(g);
            all.extend(entries(&p).into_iter().map(|(_, b)| (b.resref, name(&b.name))));
        }
        // (No custom palette file: the module's own blueprints, as the
        // pane's Custom shows them then.)
        if custom.is_none() {
            let own = crate::palette::own_entries(module, kind);
            all.extend(own.into_iter().map(|(r, n)| (r, name(&n))));
        }
        all
    };
    for g in &groupings.list {
        let empty = !skip.check(Check::PaletteGroupEmpty);
        let mut filled = vec![false; g.groups.len()];
        for kind in BlueprintKind::ALL.into_iter().filter(|k| empty && g.offered(*k)) {
            let listed = palettes.entry(kind).or_insert_with(|| read(kind));
            let keyed: Vec<(ResRef, &str)> = listed.iter().map(|(r, n)| (*r, n.as_str())).collect();
            for (i, members) in g.sort(kind, &keyed).groups.iter().enumerate() {
                filled[i] |= !members.is_empty();
            }
        }
        for (group, _) in g.groups.iter().zip(&filled).filter(|(_, f)| empty && !**f) {
            let kinds: Vec<&str> = g.kinds_of(group).iter().map(|k| k.name()).collect();
            let r#where = if kinds.is_empty() {
                "it is offered in no palette (its types are none of its grouping's)".to_string()
            } else {
                format!("no blueprint of the {} palettes falls in it", kinds.join(", "))
            };
            push(
                Check::PaletteGroupEmpty,
                format!("{file} [palette.{}]", g.id),
                format!("the group {:?} of {} is empty: {where}", group.name, g.title),
            );
        }
        // The lists' blueprints, where they are.
        let Some(from) = g.from.as_deref() else { continue };
        for group in &g.groups {
            let nowhere: Vec<String> = group
                .listed
                .iter()
                .filter(|(r, k)| {
                    let kinds = k.map_or_else(|| g.kinds_of(group), |k| vec![k]);
                    !kinds.iter().any(|k| {
                        let key = ResKey::new(*r, k.restype());
                        module.contains(&key) || resman.find(&key).is_some()
                    })
                })
                .map(|(r, k)| match k {
                    Some(k) => format!("{r}.{}", k.restype().extension().unwrap_or_default()),
                    None => r.to_string(),
                })
                .collect();
            if nowhere.is_empty() {
                continue;
            }
            let shown = nowhere.iter().take(10).cloned().collect::<Vec<_>>().join(", ");
            let more = match nowhere.len() {
                n if n > 10 => format!(" and {} more", n - 10),
                _ => String::new(),
            };
            push(
                Check::PaletteListMissing,
                format!("{from} {:?}", group.name),
                format!(
                    "{} blueprint{} the list of {:?} names {} nowhere (the module, its haks, the \
                     game): {shown}{more}",
                    nowhere.len(),
                    if nowhere.len() == 1 { "" } else { "s" },
                    group.name,
                    if nowhere.len() == 1 { "is" } else { "are" },
                ),
            );
        }
    }
    out
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
        tables_read: TablesRead::default(),
        models_read: ModelsRead::default(),
    };
    tileset_data(&mut d, "tileset", key, data);
    d.out
}

/// The longest line of a tileset the game's server loads a module with,
/// in characters, its CR and LF not counted (`engine_tileset_holes`).
const MAX_SET_LINE: usize = 255;

/// The most bytes after a tileset's last CRLF (lines ending in LF) the
/// game's server loads a module with.
const MAX_SET_TAIL: usize = 256;

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
    // loading the module (`engine_tileset_holes`). Some of them, where
    // the rest end in CRLF: it reads them as CRLF ones, unless more than
    // 256 bytes follow the file's last CRLF. And a line of more than 255
    // characters, however it ends, stops it too.
    let lf =
        data.iter().enumerate().filter(|&(i, &b)| b == b'\n' && (i == 0 || data[i - 1] != b'\r'));
    let (bare, first) = (lf.clone().count(), lf.map(|(i, _)| i).next());
    let crlf = data.windows(2).filter(|w| w == b"\r\n").count();
    let long = data
        .split(|&b| b == b'\n')
        .enumerate()
        .map(|(i, l)| (i + 1, l.strip_suffix(b"\r").unwrap_or(l).len()))
        .find(|&(_, n)| n > MAX_SET_LINE);
    if let Some((line, n)) = long {
        let message = format!(
            "line {line} is {n} characters long: the game's server never finishes loading a \
             module that uses it (a line has at most {MAX_SET_LINE})"
        );
        d.push(Error, Check::SetLineEnds, source, key, format!("line {line}"), message);
    }
    let after_crlf = data.windows(2).rposition(|w| w == b"\r\n").map(|at| data.len() - at - 2);
    if bare > 0
        && let Some(after) = after_crlf.filter(|&n| n > MAX_SET_TAIL)
    {
        d.push(
            Error,
            Check::SetLineEnds,
            source,
            key,
            "",
            format!(
                "its last {after} bytes, after its last CRLF, end in LF: the game's server \
                 never finishes loading a module that uses it with more than {MAX_SET_TAIL} \
                 there (save it with Windows line ends)"
            ),
        );
    } else if bare > 0 && crlf == 0 {
        d.push(
            Error,
            Check::SetLineEnds,
            source,
            key,
            "",
            "its lines end in LF, not CRLF: the game's server never finishes loading a module \
             that uses it (save it with Windows line ends)",
        );
    } else if let Some(at) = first {
        let line = mg_core::text::line_of(data, at);
        d.push(
            Warning,
            Check::SetLineEnds,
            source,
            key,
            "",
            format!(
                "{bare} of its lines end in LF, not CRLF (the first is line {line}): the game's \
                 server reads them as CRLF ones, but not more than {MAX_SET_TAIL} bytes of them \
                 at the file's end (save it with Windows line ends throughout, as the game's \
                 own are)"
            ),
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

/// A material (the one the game reads: the load order's): texture names
/// longer than a resource name can be (Aurora's "Pure virtual function
/// called"), and textures no layer holds as a `.dds`, `.tga` or `.plt`
/// (`null` names none), which the game draws without (a builder's report:
/// a hak's 149 materials named textures a build step had renamed, and
/// Verify said nothing).
fn material(d: &mut Doctor, source: &str, key: ResKey) {
    let Ok(data) = d.rm.get(&key) else { return };
    let mtr = mg_image::mtr::Mtr::parse(&data);
    let held = |t: &str| {
        mg_core::ResRef::cut(t).is_some_and(|r| {
            [ResType::DDS, ResType::TGA, ResType::PLT]
                .into_iter()
                .any(|kind| d.rm.contains(&ResKey::new(r, kind)))
        })
    };
    let missing: Vec<(usize, String)> = (mtr.textures.iter().enumerate())
        .filter_map(|(slot, t)| Some((slot, t.as_ref()?.clone())))
        .filter(|(_, t)| !t.eq_ignore_ascii_case("null") && t.len() <= 16 && !held(t))
        .collect();
    if !d.skip.check(Check::MtrTexture) {
        // (Its texture0, the color, an error: the game draws it without
        // any. Another slot's, a normal, specular or other map: drawn
        // without that, a warning.)
        for (slot, t) in missing {
            let (severity, without) = match slot {
                0 => (Severity::Error, "the game draws the material untextured"),
                _ => (Severity::Warning, "the game draws the material without that map"),
            };
            d.push(
                severity,
                Check::MtrTexture,
                source,
                key,
                format!("texture{slot}"),
                format!(
                    "texture {t} is in no layer of the load order (as .dds, .tga or .plt): \
                     {without}"
                ),
            );
        }
    }
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
            found.entry((source, resource, at, dg.message)).or_default().push(program);
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
    // (A table is there where it reads.)
    let there = |n: &str| tables.is_some_and(|read| read(n).is_some());
    let others = tables.map(|read| Others { read, there: &there });
    let mut out: Vec<CellFault> = (cell_faults(name, t, kinds, others.as_ref(), tlk).into_iter())
        .map(|f| CellFault { row: f.row, column: f.column, message: f.message })
        .collect();
    out.sort_by_key(|f| (f.row, f.column));
    out
}

/// How the cells' rule reads the other tables: a table's rows, and
/// whether a table is there at all (Verify asks the game data, which has
/// a table it can't read too).
struct Others<'a> {
    read: &'a TableOf<'a>,
    there: &'a dyn Fn(&str) -> bool,
}

/// A cell that is wrong, with what its column holds: what the one rule of
/// a table's cells finds ([`cell_faults`]), for the 2DA editor
/// ([`table_faults`]) and for Verify ([`two_da`]) to say each in its way.
struct KindFault {
    row: usize,
    /// The column's place in the table, and its name as the checks have it.
    column: usize,
    label: &'static str,
    kind: ColumnKind,
    /// The row of a table that the cell names (a [`ColumnKind::Row`]'s).
    names: Option<(&'static str, usize)>,
    message: String,
}

/// The rule of a table's cells, written once: what is wrong in the cells
/// of the table `name`, by what its columns hold (`kinds`), a column
/// after another and each from its first row down.
fn cell_faults(
    name: &str,
    t: &TwoDa,
    kinds: &[(&'static str, ColumnKind)],
    others: Option<&Others>,
    tlk: TalkTables,
) -> Vec<KindFault> {
    let mut out = Vec::new();
    for (column, kind) in kinds {
        let Some(at) = t.column(column) else { continue };
        for row in 0..t.len() {
            let Some(text) = t.cell(row, at) else { continue };
            let mut names = None;
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
                    names = kind.row_table(t, row).zip(named);
                    match names {
                        Some((target, named)) if target.eq_ignore_ascii_case(name) => {
                            row_fault(t, target, named)
                        }
                        Some((target, named)) => others
                            .and_then(|o| (o.read)(target))
                            .and_then(|other| row_fault(&other, target, named)),
                        None => None,
                    }
                }
                ColumnKind::Table => others.and_then(|o| {
                    let there = text.eq_ignore_ascii_case(name) || (o.there)(text.trim());
                    (!there).then(|| format!("{text}.2da, which it names, isn't there"))
                }),
            };
            if let Some(message) = message {
                out.push(KindFault { row, column: at, label: column, kind: *kind, names, message });
            }
        }
    }
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
    // (Its layer's copy, the one the game reads: for placeables.2da the
    // pending baked rows' layer may be over it.)
    let Ok(data) = d.rm.layers()[layer].container.read(&key) else { return };
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
    // or by its name as the game's own are named). Its cells are held to
    // the rule the 2DA editor holds them to as they are typed
    // ([`cell_faults`]): the other tables as the module has them (this
    // hak's and the others' over the game's), a table there where the
    // game data has it.
    let (rm, tlk) = (d.rm, d.tlk);
    let found = {
        let doctor = std::cell::RefCell::new(&mut *d);
        let read = |n: &str| doctor.borrow_mut().table_shared(n);
        let there = |n: &str| ResKey::parse(n, ResType::TWODA).is_some_and(|k| rm.contains(&k));
        let kinds = column_kinds_as_named(&name, &read);
        let others = Others { read: &read, there: &there };
        cell_faults(&name, &table, &kinds, Some(&others), tlk)
    };
    // (Of the rows it adds or changes alone.)
    for f in found.into_iter().filter(|f| changed(f.row)) {
        let at = format!("row {}, {}", f.row, f.label);
        match f.kind {
            ColumnKind::StrRef => {
                let message = format!("{} (\"Bad Strref\")", f.message);
                d.push(Error, Check::TwoDaStrRef, source, key, at, message);
            }
            ColumnKind::Row(_) | ColumnKind::RowBy { .. } => {
                // (A cell as the copy below has it, wrong in the game's
                // own tables too, is the game's: appearance's
                // FOOTSTEPTYPE 30.)
                let kept = base.is_some_and(|b| b.get(f.row, f.label) == table.get(f.row, f.label));
                let the_game_s = |(target, named): (&str, usize)| {
                    game_copy(rm, target).is_some_and(|g| row_fault(&g, target, named).is_some())
                };
                if kept && f.names.is_some_and(the_game_s) {
                    continue;
                }
                d.push(Warning, Check::TwoDaRef, source, key, at, f.message);
            }
            // A table that is not there.
            ColumnKind::Table => d.push(Warning, Check::TwoDaRef, source, key, at, f.message),
            ColumnKind::Model | ColumnKind::Script => {}
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

/// A class's spells, by the tables the game reads: each spellcasting
/// class's `SpellTableColumn` (the column of spells.2da whose levels are
/// its spells) is one spells.2da has; and each spell its packages prefer
/// (a package's `SpellPref2DA`, its `SpellIndex`) is one the class can
/// learn: spells.2da gives it a level in the class's column (or, for a
/// class that picks domains, a domain grants it). A fault the game's own
/// tables have too is the game's, and not said. Looked at where one of
/// those tables is custom (the module's, a hak's).
fn class_spells(d: &mut Doctor) {
    let custom = |name: &str| {
        let Some(key) = ResKey::parse(name, ResType::TWODA) else { return false };
        d.rm.find(&key).is_some_and(|i| is_custom(&d.rm.layers()[i]))
    };
    let read = |name: &str| -> Option<TwoDa> {
        let key = ResKey::parse(name, ResType::TWODA)?;
        TwoDa::parse(&d.rm.get(&key).ok()?, Codepage::default()).ok()
    };
    let (Some(classes), Some(packages), Some(spells)) =
        (read("classes"), read("packages"), read("spells"))
    else {
        return;
    };
    let pref_tables: Vec<String> = (0..packages.len())
        .filter_map(|r| packages.get(r, "SpellPref2DA").map(str::to_ascii_lowercase))
        .collect();
    if !["classes", "packages", "spells", "domains"].iter().any(|n| custom(n))
        && !pref_tables.iter().any(|n| custom(n))
    {
        return;
    }
    let mut found = spell_faults(&classes, &packages, &spells, read("domains").as_ref(), &read);
    found.sort_by(|a, b| (&a.table, a.row, &a.at).cmp(&(&b.table, b.row, &b.at)));
    // The game's own, with its own tables: those are its, not the builder's.
    let game = |name: &str| game_copy(d.rm, name);
    let theirs = match (game("classes"), game("packages"), game("spells")) {
        (Some(c), Some(p), Some(s)) => spell_faults(&c, &p, &s, game("domains").as_ref(), &game),
        _ => Vec::new(),
    };
    let mut found: Vec<SpellFault> = found.into_iter().filter(|f| !theirs.contains(f)).collect();
    // (A table several packages share is said once a row.)
    found.dedup_by(|a, b| (&a.table, &a.at) == (&b.table, &b.at));
    // A package's table of the game's, whose spells a custom spells.2da
    // or classes.2da took from its class: said once for the table.
    let mut of_the_game: Vec<(String, Vec<SpellFault>)> = Vec::new();
    for f in found {
        let Some(key) = ResKey::parse(&f.table, ResType::TWODA) else { continue };
        let at = d.rm.find(&key);
        if at.is_some_and(|i| is_custom(&d.rm.layers()[i])) || f.at.contains("SpellTableColumn") {
            let source = at.map(|i| d.rm.layers()[i].label.clone()).unwrap_or_default();
            d.push(Severity::Warning, Check::TwoDaSpells, &source, key, f.at, f.message);
            continue;
        }
        match of_the_game.iter_mut().find(|(t, _)| *t == f.table) {
            Some((_, all)) => all.push(f),
            None => of_the_game.push((f.table.clone(), vec![f])),
        }
    }
    for (table, all) in of_the_game {
        let Some(key) = ResKey::parse(&table, ResType::TWODA) else { continue };
        let source = d.rm.find(&key).map(|i| d.rm.layers()[i].label.clone()).unwrap_or_default();
        let rows: Vec<String> = all.iter().map(|f| f.row.to_string()).take(5).collect();
        let more =
            if all.len() > 5 { format!(" and {} more", all.len() - 5) } else { String::new() };
        let message = format!(
            "the game's table, whose package's class can no longer learn {} of the spells it \
             prefers by the spells.2da and classes.2da read now (rows {}{more}; the first: {})",
            all.len(),
            rows.join(", "),
            all[0].message
        );
        d.push(Severity::Warning, Check::TwoDaSpells, &source, key, "", message);
    }
}

/// A fault [`spell_faults`] finds: the table at fault, where in it, and
/// why.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SpellFault {
    table: String,
    row: usize,
    at: String,
    message: String,
}

/// [`class_spells`]' faults of tables as given (`read` reads a package's
/// spell table by its name): the table at fault, where in it, and why.
fn spell_faults(
    classes: &TwoDa,
    packages: &TwoDa,
    spells: &TwoDa,
    domains: Option<&TwoDa>,
    read: &dyn Fn(&str) -> Option<TwoDa>,
) -> Vec<SpellFault> {
    let mut out = Vec::new();
    let label = |t: &TwoDa, r: usize| t.get(r, "Label").unwrap_or("****").to_string();
    // A class's column of spells.2da, where it casts and names one.
    let column_of = |class: usize| -> Option<&str> {
        let caster = classes.get_int(class, "SpellCaster").unwrap_or(0) != 0;
        classes.get(class, "SpellTableColumn").filter(|_| caster)
    };
    for class in 0..classes.len() {
        let Some(column) = column_of(class) else { continue };
        if spells.column(column).is_none() {
            out.push(SpellFault {
                table: "classes".to_string(),
                row: class,
                at: format!("row {class}, SpellTableColumn"),
                message: format!(
                    "{}'s spells are spells.2da's column {column}, which it doesn't have: the \
                     class has no spells",
                    label(classes, class)
                ),
            });
        }
    }
    // The spells domains grant (a class that picks domains learns them).
    let granted: std::collections::BTreeSet<i64> = domains
        .map(|t| {
            (0..t.len())
                .flat_map(|r| (1..=9).filter_map(move |l| t.get_int(r, &format!("Level_{l}"))))
                .map(i64::from)
                .collect()
        })
        .unwrap_or_default();
    let mut read_tables: std::collections::HashMap<String, Option<TwoDa>> = Default::default();
    for package in 0..packages.len() {
        let (Some(table), Some(class)) =
            (packages.get(package, "SpellPref2DA"), packages.get_int(package, "ClassID"))
        else {
            continue;
        };
        let Some(class) = usize::try_from(class).ok().filter(|c| *c < classes.len()) else {
            continue;
        };
        let Some(column) = column_of(class).and_then(|c| spells.column(c)) else { continue };
        let domains = classes.get_int(class, "PickDomains").unwrap_or(0) != 0;
        let name = table.to_ascii_lowercase();
        let prefs = read_tables.entry(name.clone()).or_insert_with(|| read(&name));
        let Some(prefs) = prefs.as_ref() else { continue };
        for row in 0..prefs.len() {
            let Some(spell) = prefs.get_int(row, "SpellIndex") else { continue };
            let Some(s) = usize::try_from(spell).ok().filter(|s| *s < spells.len()) else {
                continue;
            };
            let learns = spells.cell(s, column).is_some()
                || (domains && granted.contains(&i64::from(spell)));
            if !learns {
                out.push(SpellFault {
                    table: name.clone(),
                    row,
                    at: format!("row {row}, SpellIndex"),
                    message: format!(
                        "spell {spell} ({}) is no spell of {}'s, whose package {package} ({}) \
                         prefers it: spells.2da's {} column has no level for it",
                        label(spells, s),
                        label(classes, class),
                        label(packages, package),
                        spells.columns()[column],
                    ),
                });
            }
        }
    }
    out
}

/// The module's own tables that have a description (beside them, or a
/// family's in their folder: [`crate::table_desc`]), and its haks' (the
/// copy the game reads), whose description is beside the hak's file, or
/// beside the one table of the name in the module's project that has one
/// (the folder a hak is built from): each cell that is not what its
/// column is said to hold, and a description that cannot be read.
fn described_tables(d: &mut Doctor, module: &Module) {
    use crate::table_desc::{Beside, Description, description_in_module, find_for};
    let mut keys: Vec<ResKey> = module.keys_of(ResType::TWODA).copied().collect();
    keys.sort();
    // Each table with its source (its layer's label), its description and
    // its bytes: the module's, then the haks'.
    let mut tables: Vec<(String, ResKey, std::path::PathBuf, Vec<u8>)> = Vec::new();
    for key in keys {
        let Some(path) = description_in_module(module, &key) else { continue };
        let Some(data) = module.get(&key) else { continue };
        tables.push(("module".to_string(), key, path, data.to_vec()));
    }
    // (The module's project, walked once, for a hak's table's sources.)
    let project = match &module.location {
        Some(crate::ModuleLocation::Archive(path)) => Some(Beside::root_of(path)),
        Some(crate::ModuleLocation::Folder(dir)) => Some(Beside::root_of(&dir.join("module.ifo"))),
        Some(crate::ModuleLocation::Project { root, .. }) => Some(root.clone()),
        None => None,
    };
    let mut sources: Option<Beside> = None;
    let layers = d.rm.layers();
    for (i, layer) in layers.iter().enumerate() {
        let Some(hak) = layer.container.watched().filter(|_| is_custom(layer)) else { continue };
        if !hak.extension().is_some_and(|e| e.eq_ignore_ascii_case("hak")) {
            continue;
        }
        let mut keys: Vec<ResKey> =
            layer.container.keys().filter(|k| k.restype == ResType::TWODA).collect();
        keys.sort();
        for key in keys {
            // (The copy the game reads alone, as Verify takes a hak's.)
            if d.rm.find(&key) != Some(i) || module.contains(&key) {
                continue;
            }
            let beside = find_for(&hak.with_file_name(key.to_string()));
            let path = beside.or_else(|| {
                let root = project.as_ref()?;
                let files = sources.get_or_insert_with(|| Beside::of(root));
                let mut found = files.described().into_iter().filter(|(t, _)| {
                    t.file_name()
                        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(&key.to_string()))
                });
                // (One of the name: two would be a guess.)
                match (found.next(), found.next()) {
                    (Some((_, description)), None) => Some(description),
                    _ => None,
                }
            });
            let (Some(path), Ok(data)) = (path, layer.container.read(&key)) else { continue };
            tables.push((layer.label.clone(), key, path, data.into_owned()));
        }
    }
    // (The folders are walked once for the tables of one.)
    let mut walked: Option<(std::path::PathBuf, Beside)> = None;
    for (source, key, path, data) in tables {
        let source = source.as_str();
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let Ok(table) = TwoDa::parse(&data, Codepage::default()) else { continue };
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
                d.push(Severity::Warning, Check::TwoDaDescribed, source, key, "", message);
                continue;
            }
        };
        match Description::parse(&text, &script) {
            Err(e) => {
                let message = format!("its description ({file}) can't be read: {e}");
                d.push(Severity::Warning, Check::TwoDaDescribed, source, key, "", message);
            }
            Ok(description) => {
                for f in description.faults(&table, &rows_of, d.tlk) {
                    let at = format!("row {}, {}", f.row, table.columns()[f.column]);
                    d.push(Severity::Warning, Check::TwoDaDescribed, source, key, at, f.message);
                }
            }
        }
    }
}

/// The checks that read the module's areas, objects and blueprints, in
/// the order they are made in: of one resource, each is handed the tree
/// it was read into once ([`resources_once`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Once {
    Objects,
    PcFaction,
    StaticTransforms,
    StaticTints,
    TintsStored,
    AppearanceRange,
    AreaVariables,
    TileReplacements,
    TileSeams,
    AreaSounds,
    /// The tags of the areas and their objects, for `tag-unique`.
    Tags,
    Described,
    Read,
}

impl Once {
    const ALL: [Once; 13] = [
        Once::Objects,
        Once::PcFaction,
        Once::StaticTransforms,
        Once::StaticTints,
        Once::TintsStored,
        Once::AppearanceRange,
        Once::AreaVariables,
        Once::TileReplacements,
        Once::TileSeams,
        Once::AreaSounds,
        Once::Tags,
        Once::Described,
        Once::Read,
    ];

    /// What the timings call it.
    fn name(self) -> &'static str {
        match self {
            Once::Objects => "objects' rows",
            Once::PcFaction => "the PC faction",
            Once::StaticTransforms => "static placeables' transforms",
            Once::StaticTints => "static placeables' tints",
            Once::TintsStored => "tints stored and set",
            Once::AppearanceRange => "placeables' rows past 65535",
            Once::AreaVariables => "areas' variables",
            Once::TileReplacements => "tiles' replacement textures",
            Once::TileSeams => "tiles' seams",
            Once::AreaSounds => "music, ambient and placed sounds",
            Once::Tags => "areas' and objects' tags",
            Once::Described => "described variables",
            Once::Read => "variables the scripts read",
        }
    }

    /// The checks it makes the findings of: left out with all of them.
    fn checks(self) -> &'static [Check] {
        use Check::*;
        match self {
            Once::Objects => &[ObjectRow],
            Once::PcFaction => &[PcFaction],
            Once::StaticTransforms => &[StaticTransform],
            Once::StaticTints => &[StaticTint],
            Once::TintsStored => &[TintStored],
            Once::AppearanceRange => &[AppearanceRange],
            Once::AreaVariables => &[AreaVariables],
            Once::TileReplacements => &[TileReplace],
            Once::TileSeams => &[TileSeam],
            Once::AreaSounds => &[SoundSilent],
            Once::Tags => &[TagUnique],
            Once::Described => &[VarType, VarValue, VarObject, VarName, VarRuntime, VarRequires],
            Once::Read => &[VarScript],
        }
    }

    /// Whether it reads resources of this type.
    fn reads(self, t: ResType) -> bool {
        use ResType as T;
        match self {
            Once::Objects => {
                matches!(t, T::UTC | T::UTP | T::UTI | T::UTD | T::UTM | T::UTW | T::GIT)
            }
            Once::PcFaction => matches!(t, T::UTC | T::UTP | T::UTD | T::UTE | T::GIT),
            Once::StaticTransforms => t == T::GIT,
            Once::AreaSounds => matches!(t, T::GIT | T::UTS),
            Once::StaticTints | Once::TintsStored | Once::AppearanceRange => {
                matches!(t, T::GIT | T::UTP)
            }
            Once::AreaVariables | Once::TileReplacements | Once::TileSeams => t == T::ARE,
            Once::Tags => matches!(t, T::ARE | T::GIT),
            Once::Described | Once::Read => crate::var_desc::Owner::of_restype(t).is_some(),
        }
    }

    /// Whether what it finds is said after the unique tags and the merged
    /// areas (it was made after them when each check read the module for
    /// itself, and findings of one resource and severity keep that order).
    fn said_after(self) -> bool {
        matches!(self, Once::Described | Once::Read)
    }
}

/// A list of a described variable's values that can't be read: said once
/// a run, at the first variable of it in the first resource. Each
/// resource's first is noted (where among its later findings it goes),
/// and the first of them all is said ([`SeenOnce::say_the_rest`]).
struct Unlisted {
    before: usize,
    name: String,
    at: String,
    message: String,
}

/// What [`resources_once`] found of one resource.
struct OfOne {
    key: ResKey,
    /// Of the checks made before the unique tags and the merged areas,
    /// and of those after ([`Once::said_after`]).
    first: Vec<Finding>,
    after: Vec<Finding>,
    unlisted: Vec<Unlisted>,
    tags: Option<crate::tags::TagIndex>,
    learned: Learned,
    /// How long each check took, and the read (the last).
    took: [Duration; Once::ALL.len() + 1],
}

/// What a worker of [`resources_once`] keeps from one resource to the
/// next: the tables it read, the tile models it looked into, and the
/// described variables' lists.
#[derive(Default)]
struct KeptOnce<'a> {
    tables: HashMap<String, Option<std::sync::Arc<TwoDa>>>,
    replaced: HashMap<String, bool>,
    choices: HashMap<String, crate::var_desc::Choices>,
    lookup: Option<crate::var_desc::ModuleLookup<'a>>,
}

/// Each tileset's tiles by the cells they fit, made once a run for the
/// areas of every worker.
type TileIndexes = std::sync::Mutex<HashMap<ResRef, Option<std::sync::Arc<mg_tiles::TileIndex>>>>;

/// What is left of [`resources_once`] for after it.
struct SeenOnce {
    /// Every area's and placed object's tag, where `tag-unique` runs.
    tags: Option<crate::tags::TagIndex>,
    after: Vec<(ResKey, Vec<Finding>, Vec<Unlisted>)>,
}

impl SeenOnce {
    /// Says what the checks after the unique tags and the merged areas
    /// found, a resource after another, and each list that can't be read
    /// once: at the first variable of it.
    fn say_the_rest(self, d: &mut Doctor) {
        let mut unread: HashSet<String> = HashSet::new();
        for (key, said, unlisted) in self.after {
            let mut unlisted = unlisted.into_iter().peekable();
            let mut say = |d: &mut Doctor, u: Unlisted| {
                if unread.insert(u.name) {
                    d.push(Severity::Warning, Check::VarValue, "module", key, u.at, u.message);
                }
            };
            for (i, finding) in said.into_iter().enumerate() {
                while let Some(u) = unlisted.next_if(|u| u.before == i) {
                    say(d, u);
                }
                d.out.push(finding);
            }
            for u in unlisted {
                say(d, u);
            }
        }
    }
}

/// The checks of the module's areas, objects and blueprints (`wanted`: by
/// [`Once::ALL`]'s order, those that run): each resource one of them reads
/// is read once, on every core, and its tree handed to every check that
/// reads its type, where each check read every resource again, one after
/// another (a persistent world's Verify parsed each area's GIT a dozen
/// times on one core). The findings are in the order those made: a
/// resource's in the checks' order, the resources in their keys'.
fn resources_once(
    d: &mut Doctor,
    module: &Module,
    wanted: [bool; Once::ALL.len()],
    reads: &std::collections::BTreeMap<String, Vec<crate::var_desc::Read>>,
    timings: &mut Timings,
) -> SeenOnce {
    use crate::var_desc;
    let asked = |o: Once, t: ResType| wanted[o as usize] && o.reads(t);
    let mut keys: Vec<ResKey> = (module.keys())
        .filter(|k| Once::ALL.iter().any(|o| asked(*o, k.restype)))
        .copied()
        .collect();
    keys.sort();
    let indexes = TileIndexes::default();
    // (What each sound file holds, read once for every area that names it.)
    let judged = crate::sounds::Judged::default();
    let (rm, beside) = (d.rm, &*d);
    let started = Instant::now();
    let found: Vec<Option<OfOne>> = keys
        .par_iter()
        .map_init(KeptOnce::default, |kept, &key| {
            let t = key.restype;
            let mut took = [Duration::ZERO; Once::ALL.len() + 1];
            let reading = Instant::now();
            let Some(Ok(gff)) = module.gff(&key) else { return None };
            took[Once::ALL.len()] = reading.elapsed();
            let mut w = beside.beside();
            w.tables = std::mem::take(&mut kept.tables);
            // (Timed where it runs.)
            let mut run = |o: Once, check: &mut dyn FnMut()| {
                if asked(o, t) {
                    let started = Instant::now();
                    check();
                    took[o as usize] += started.elapsed();
                }
            };
            run(Once::Objects, &mut || objects(&mut w, key, &gff));
            run(Once::PcFaction, &mut || pc_faction(&mut w, key, &gff));
            run(Once::StaticTransforms, &mut || static_transforms(&mut w, key, &gff));
            run(Once::StaticTints, &mut || static_tints(&mut w, key, &gff));
            run(Once::TintsStored, &mut || tints_stored(&mut w, key, &gff));
            run(Once::AppearanceRange, &mut || appearance_range(&mut w, key, &gff));
            run(Once::AreaVariables, &mut || area_variables(&mut w, key, &gff));
            run(Once::TileReplacements, &mut || {
                tile_replacements(&mut w, key, &gff, &mut kept.replaced)
            });
            run(Once::TileSeams, &mut || tile_seams(&mut w, key, &gff, &indexes));
            run(Once::AreaSounds, &mut || area_sounds(&mut w, key, &gff, &judged));
            let mut tags = None;
            run(Once::Tags, &mut || {
                let index = tags.insert(crate::tags::TagIndex::default());
                match t {
                    ResType::ARE => index.add_area(key.resref, &gff.root),
                    _ => index.add_git(key.resref, &gff.root),
                }
            });
            let first = std::mem::take(&mut w.out);
            // (A resource's variables, found once for both checks of them.)
            let variables = (Once::ALL.iter())
                .any(|o| o.said_after() && asked(*o, t))
                .then(|| var_desc::variables_of(key, &gff));
            let variables = variables.as_deref().unwrap_or(&[]);
            let mut unlisted = Vec::new();
            run(Once::Described, &mut || {
                let lookup = (kept.lookup)
                    .get_or_insert_with(|| var_desc::ModuleLookup::new(module, Some(rm)));
                let lists = (lookup as &var_desc::ModuleLookup, &mut kept.choices);
                described_variables(&mut w, module, key, &gff, variables, lists, &mut unlisted);
            });
            run(Once::Read, &mut || variables_read(&mut w, module, key, &gff, variables, reads));
            kept.tables = std::mem::take(&mut w.tables);
            Some(OfOne { key, first, after: w.out, unlisted, tags, learned: w.learned, took })
        })
        .collect();
    let wall = started.elapsed();
    let mut seen = SeenOnce {
        tags: wanted[Once::Tags as usize].then(crate::tags::TagIndex::default),
        after: Vec::new(),
    };
    let mut took = [Duration::ZERO; Once::ALL.len() + 1];
    let mut read = 0;
    for one in found.into_iter().flatten() {
        d.out.extend(one.first);
        d.learned.extend(one.learned);
        if let (Some(all), Some(own)) = (seen.tags.as_mut(), &one.tags) {
            all.absorb(own);
        }
        for (all, own) in took.iter_mut().zip(one.took) {
            *all += own;
        }
        read += 1;
        if !one.after.is_empty() || !one.unlisted.is_empty() {
            seen.after.push((one.key, one.after, one.unlisted));
        }
    }
    timings.add("the module's areas, objects and blueprints, each read once (in parallel)", wall);
    timings.add(format!("  read: {read}, added up"), took[Once::ALL.len()]);
    for o in Once::ALL.into_iter().filter(|o| wanted[*o as usize]) {
        timings.add(format!("  {}, added up", o.name()), took[o as usize]);
    }
    seen
}

/// A module's object's (a blueprint's, or an area's placed ones'): 2DA
/// rows they name that don't exist (a creature with a class that isn't
/// there crashes Aurora and the game). A waypoint's (its flag, which the
/// toolset alone draws) is a warning wherever it is.
fn objects(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff) {
    let mut found = Vec::new();
    let kind = match key.restype {
        ResType::GIT => None,
        t => Some(t),
    };
    walk_objects(d, &gff.root, kind, &GffPath::root(), &mut found);
    // Placed, it breaks the area (Aurora's area view crashes on it); a
    // blueprint breaks only when placed, and a waypoint's flag nothing
    // the game shows. A part with no model (an item's piece, a body's
    // part) leaves the rest drawn: a warning wherever it is.
    for (path, message, part) in found {
        let in_list =
            |list: &str| path.first().and_then(Step::item).is_some_and(|(l, _)| l == list);
        let waypoint = in_list("WaypointList") || key.restype == ResType::UTW;
        let severity = match key.restype == ResType::GIT && !waypoint && !part {
            true => Severity::Error,
            false => Severity::Warning,
        };
        let place = crate::rename::describe(key, Some(gff), &path);
        d.push(severity, Check::ObjectRow, "module", key, place, message);
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
fn pc_faction(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff) {
    use mg_rules::ObjectKind;
    let of_pc = |kind: ObjectKind, s: &Struct| {
        mg_rules::objects::pc_faction(kind, s).map(|field| (field, s.get(field).is_none()))
    };
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
                    found.push((kind, GffPath::root().item(&label, i).field(field), none));
                }
            }
        }
    } else if let Some(kind) = ObjectKind::from_restype(key.restype)
        && let Some((field, none)) = of_pc(kind, &gff.root)
    {
        found.push((kind, GffPath::root().field(field), none));
    }
    for (kind, path, none) in found {
        let place = crate::rename::describe(key, Some(gff), &path);
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

/// Static placeables with a visual transform that changes their model:
/// the server keeps it (a script reads it back), but the game client
/// draws a static placeable without it, merged into the area's tiles
/// (`visual_transforms_look` and `placeables_look` in `client_render.rs`).
/// Aurora takes a placeable's transform off when Static is ticked, and so
/// do Moonglow's Properties and Make Static: such a file was made
/// elsewhere (another tool, by hand, or an older Moonglow).
fn static_transforms(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff) {
    let list = gff.root.list("Placeable List").unwrap_or(&[]);
    for (i, s) in list.iter().enumerate() {
        if s.integer("Static").unwrap_or(0) == 0 || !crate::placement::is_transformed(s) {
            continue;
        }
        let static_at = GffPath::root().item("Placeable List", i).field("Static");
        let place = crate::rename::describe(key, Some(gff), &static_at);
        let message = "static, so the game draws it without its visual transform (its scale, \
                           rotation and translation): untick Static (Properties) to have it drawn \
                           transformed (ticking it again takes the transform off, as Aurora does)"
            .to_string();
        d.push(Severity::Warning, Check::StaticTransform, "module", key, place, message);
    }
}

/// Static placeables with a tint's local (The Domains of Dread's
/// colorways, [`mg_rules::tint`]): the module's script sets the tint on
/// the object, and the game client draws a static placeable merged into
/// the area's tiles as made, untinted (seen in the client, that project's
/// trial of 2026-11-06). Placed ones and blueprints.
fn static_tints(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff) {
    let message = "static, so the game draws it as made, without the tint its tdod_tint \
                   variables give: untick Static (Properties) to have it tinted"
        .to_string();
    let tinted =
        |s: &Struct| !mg_rules::tint::takes_tint(s) && !mg_rules::tint::of_object(s).is_empty();
    if key.restype == ResType::UTP {
        if tinted(&gff.root) {
            let place = crate::rename::describe(key, Some(gff), &GffPath::root().field("Static"));
            d.push(Severity::Warning, Check::StaticTint, "module", key, place, message);
        }
        return;
    }
    let list = gff.root.list("Placeable List").unwrap_or(&[]);
    for (i, _) in list.iter().enumerate().filter(|(_, s)| tinted(s)) {
        let at = GffPath::root().item("Placeable List", i).field("Static");
        let place = crate::rename::describe(key, Some(gff), &at);
        d.push(Severity::Warning, Check::StaticTint, "module", key, place, message.clone());
    }
}

/// Placeables, placed and blueprints, with a `tdodTint` stored in their
/// shader parameters (`Material` › `ShaderParams`, Properties › Visuals)
/// for a material their tint's locals also tint: the module's script
/// sets the locals' over it as the module loads, so the stored one is
/// never seen (The Domains of Dread's scripts: the locals are the
/// source, [`mg_rules::tint`]).
fn tints_stored(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff) {
    let mut check = |key: ResKey, gff: &mg_gff::Gff, s: &Struct, at: GffPath| {
        let tints = mg_rules::tint::of_object(s);
        if tints.is_empty() {
            return;
        }
        let params = s.child("Material").and_then(|m| m.list("ShaderParams")).unwrap_or(&[]);
        for (i, e) in params.iter().enumerate() {
            let text = |l: &str| {
                String::from_utf8_lossy(e.string(l).unwrap_or_default()).trim().to_string()
            };
            let (material, param) = (text("Material"), text("Param"));
            if !param.eq_ignore_ascii_case(mg_rules::tint::PARAMETER)
                || tints.of(&material).is_none()
            {
                continue;
            }
            let path = at.clone().field("Material").item("ShaderParams", i);
            let place = crate::rename::describe(key, Some(gff), &path);
            let message = format!(
                "stores a tdodTint for {material}, which its tdod_tint variables also tint: the \
                 module's script sets theirs over it as the module loads. Take the stored one \
                 out (Properties › Visuals › Shader parameters), or the variable"
            );
            d.push(Severity::Warning, Check::TintStored, "module", key, place, message);
        }
    };
    if key.restype == ResType::UTP {
        check(key, gff, &gff.root, GffPath::root());
        return;
    }
    for (i, s) in gff.root.list("Placeable List").unwrap_or(&[]).iter().enumerate() {
        check(key, gff, s, GffPath::root().item("Placeable List", i));
    }
}

/// Placeables, placed and blueprints, whose `Appearance` is past row
/// 65535: the engine and the network carry the row as a 16-bit number, so
/// the game draws row 65536 as row 0 and 65537 as row 1 (measured by The
/// Domains of Dread on its server; nwn.wiki's 2DA limits say the same).
fn appearance_range(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff) {
    let past = |s: &Struct| {
        s.integer("Appearance").filter(|&a| a > i64::from(crate::bake::LAST_ROW) || a < 0)
    };
    let said = |a: i64| {
        let wraps = a.rem_euclid(65536);
        format!(
            "its Appearance, row {a} of placeables.2da, is past 65535, the last row the game can \
             carry: it draws row {wraps}'s model instead"
        )
    };
    if key.restype == ResType::UTP {
        if let Some(a) = past(&gff.root) {
            let at = "Appearance".to_string();
            d.push(Severity::Error, Check::AppearanceRange, "module", key, at, said(a));
        }
        return;
    }
    let list = gff.root.list("Placeable List").unwrap_or(&[]);
    for (i, s) in list.iter().enumerate() {
        let Some(a) = past(s) else { continue };
        let at = GffPath::root().item("Placeable List", i).field("Appearance");
        let place = crate::rename::describe(key, Some(gff), &at);
        d.push(Severity::Error, Check::AppearanceRange, "module", key, place, said(a));
    }
}

/// The project's walkmesh bake (`[bake]`, [`crate::bake`]): what is
/// wrong with its rules and rows file, which rows its haks' placeables.2da
/// lacks yet, and which rows and baked models no placeable of the module
/// uses. `rm` is the game data with the module's haks (the table is read
/// under the module's own and the rows Moonglow draws as pending).
pub fn bake(module: &Module, rm: &ResMan, codepage: Codepage, skip: &Skip) -> Vec<Finding> {
    use crate::bake::{self as b, RowState};
    let pf = &module.project_file;
    if !pf.bake.given || skip.source("module") {
        return Vec::new();
    }
    let file = project_file_name(pf);
    let rows_name = pf.bake.rows_file.clone().unwrap_or_else(|| b::DEFAULT_ROWS_FILE.into());
    let mut out = Vec::new();
    let mut push = |check: Check, severity, at: String, message: String| {
        if !skip.check(check) {
            out.push(Finding {
                severity,
                check: Cow::Borrowed(check.id()),
                source: "module".into(),
                resource: Module::info_key(),
                at,
                message,
            });
        }
    };
    for p in &pf.bake.problems {
        push(Check::BakeRules, Severity::Warning, format!("{file} [bake]"), p.clone());
    }
    let rows = match pf.baked_rows() {
        Ok(rows) => rows,
        Err(e) => {
            let message = format!("{e}: nothing is baked until it reads");
            push(Check::BakeRules, Severity::Error, rows_name, message);
            return out;
        }
    };
    let Ok((table, from)) = b::table_under(rm, codepage) else { return out };
    if let Some(range) = pf.bake.rows {
        let held = b::rows_held(&range, &table, &rows);
        if let (Some(first), Some(last)) = (held.first(), held.last()) {
            let which = match held.len() {
                1 => format!("row {first} is one placeables.2da has"),
                n => format!("{n} rows ({first} to {last}) are ones placeables.2da has"),
            };
            push(
                Check::BakeRules,
                Severity::Warning,
                format!("{file} [bake]"),
                format!(
                    "of the rows baked models take ({range}), {which} already (from {from}): a \
                     bake passes over them, so fewer are free than the range says; give [bake] \
                     rows the table doesn't use"
                ),
            );
        }
    }
    for r in &rows.rows {
        let at = format!("{rows_name}, row {}", r.row);
        let message = match b::row_state(&table, r) {
            RowState::Added => continue,
            RowState::Pending => format!(
                "placeables.2da (from {from}) has no row {}: the project's build hasn't added \
                 it yet, so the game has no such row and draws nothing for a placeable of it \
                 (Moonglow draws it as the build will make it, {} as row {})",
                r.row, r.model, r.copy_of
            ),
            RowState::Other(model) => format!(
                "placeables.2da (from {from}) has row {} with another model ({}), not {}: the \
                 game draws that one",
                r.row,
                model.as_deref().unwrap_or("none"),
                r.model
            ),
        };
        push(Check::BakeRowPending, Severity::Warning, at, message);
    }
    let hak = pf.bake.hak.clone().unwrap_or_default();
    let names = pf.bake.names.clone().unwrap_or_else(|| b::DEFAULT_NAMES.into());
    let unused = b::unused(module, &rows, &names, &b::hak_models(rm, &hak));
    for r in &unused.rows {
        let message = format!(
            "no placeable of this module uses row {} ({}): Tidy Up takes it and its model out \
             when you save, if another module doesn't use it",
            r.row, r.model
        );
        push(Check::BakeUnused, Severity::Warning, format!("{rows_name}, row {}", r.row), message);
    }
    let in_rows: Vec<&str> = unused.rows.iter().map(|r| r.model.as_str()).collect();
    for m in unused.models.iter().filter(|m| !in_rows.iter().any(|r| r.eq_ignore_ascii_case(m))) {
        let message = format!(
            "{m}.mdl in {hak}.hak is a baked model no row of {rows_name} names any longer: no \
             placeable can use it"
        );
        push(Check::BakeUnused, Severity::Warning, format!("{hak}.hak"), message);
    }
    out
}

/// Areas with variables in their ARE: the game reads an area's from its
/// GIT's root and none from the ARE (`engine_area_variables`), where
/// Moonglow wrote them before 1.34.3, so a script read 0 for them.
fn area_variables(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff) {
    let Some(vars) = gff.root.list("VarTable").filter(|v| !v.is_empty()) else { return };
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

/// Tiles whose `Tile_ReplaceTex` the game crashes on as it loads the
/// area (`CNWCTile::SetReplaceTexture`, seen in the client:
/// `reading_tile_replacement_texture`): 0, or a value past
/// replacetexture.2da ([`mg_rules::rows::replacement_row`]), on a tile
/// whose model has a mesh drawn with `replace_tex` (no other is replaced).
/// Only tiles with the field are looked at, and their models read once
/// each (`replaced`: whether each draws one, kept from area to area; the
/// field is seldom written: the game's own tiles but one have no
/// `replace_tex`).
fn tile_replacements(
    d: &mut Doctor,
    key: ResKey,
    gff: &mg_gff::Gff,
    replaced: &mut HashMap<String, bool>,
) {
    let tiles = gff.root.list("Tile_List").unwrap_or(&[]);
    let rows = d.table("replacetexture").map_or(0, TwoDa::len);
    let crashing: Vec<(usize, i64, i64)> = (tiles.iter().enumerate())
        .filter_map(|(i, t)| {
            let v = t.integer("Tile_ReplaceTex")?;
            mg_rules::rows::replacement_row(Some(v), rows)
                .is_none()
                .then(|| (i, v, t.integer("Tile_ID").unwrap_or(0)))
        })
        .collect();
    if crashing.is_empty() {
        return;
    }
    let Ok(set) = crate::minimap::tileset(d.rm, &gff.root) else { return };
    for (i, value, id) in crashing {
        let Some(model) = usize::try_from(id).ok().and_then(|id| set.tiles.get(id)) else {
            continue;
        };
        let model = model.model.to_ascii_lowercase();
        let rm = d.rm;
        let has = *replaced.entry(model.clone()).or_insert_with(|| {
            (ResKey::parse(&model, ResType::MDL))
                .and_then(|k| rm.get(&k).ok())
                .and_then(|data| mg_mdl::Model::read(&data).ok())
                .is_some_and(|m| m.draws_texture("replace_tex"))
        });
        if !has {
            continue;
        }
        let width = gff.root.integer("Width").unwrap_or(1).max(1) as usize;
        let place = format!("tile ({}, {}), Tile_ReplaceTex", i % width, i / width);
        let why = if value == 0 {
            "0, which names no row (a row's value is one past it)".to_string()
        } else {
            format!("past replacetexture.2da's {rows} rows (row {} by it)", value - 1)
        };
        let message = format!(
            "the tile's Tile_ReplaceTex is {value}, {why}: the game crashes as it loads the \
                 area. Its model, {model}, draws a replace_tex texture: choose a row in Tile \
                 Properties' Replacement Texture"
        );
        d.push(Severity::Error, Check::TileReplace, "module", key, place, message);
    }
}

/// An area's music and ambient sounds, and placed sounds' and sound
/// blueprints' sounds, that the game plays nothing of ([`crate::sounds`]).
///
/// Of an area (its GIT): each row its `AreaProperties` name (by day, by
/// night, in battle) of `ambientmusic.2da` or `ambientsound.2da`, as the
/// load order gives the table (the module's, its haks', the game's), whose
/// `Resource` is found nowhere, or is a file the game does not look for in
/// that role or does not play there (a WAVE as music, MP3 under `.wav` as
/// an ambient sound), or no sound at all; and a row past the table's end.
/// Row 0 (`****` in the game's tables) and a blank `Resource` are no music
/// or no sound, and said nothing of.
///
/// Of a placed sound object (the GIT's `SoundList`) and a sound blueprint
/// (a UTS), each sound its `Sounds` list names: MP3 under `.wav` without
/// the `BMU V1.0` header, a file that is no sound, and, where the object is
/// `Positional`, a stereo WAVE. A sound found nowhere, or only as `.bmu`,
/// is a missing reference's to say ([`crate::verify::Missing`]), not this.
///
/// Warnings: the area loads and plays on, silent, as a missing sound
/// leaves it. One finding a field or a sound; what a file holds is read
/// once a run (`judged`), from its first bytes.
fn area_sounds(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff, judged: &crate::sounds::Judged) {
    let rm = d.rm;
    let read = |k: &ResKey| judged.form(rm, k);
    // A placed sound object's or a blueprint's sounds, at `place`.
    let placed = |d: &mut Doctor, s: &Struct, place: &str| {
        let positional = s.integer("Positional").unwrap_or(0) != 0;
        for (j, entry) in s.list("Sounds").unwrap_or_default().iter().enumerate() {
            let Some(name) = entry.resref("Sound").filter(|n| !n.is_empty()) else { continue };
            let role = mg_audio::Role::Placed;
            let unheard = crate::sounds::unheard_by(rm, role, name, positional, &read);
            use crate::sounds::Unheard;
            let Some(unheard) = unheard.filter(|u| {
                !matches!(
                    u,
                    Unheard::Missing | Unheard::Silent { why: mg_audio::Silent::OnlyBmu, .. }
                )
            }) else {
                continue;
            };
            let at = format!("{place}Sounds[{j}]");
            let message = unheard.message(role, name);
            d.push(Severity::Warning, Check::SoundSilent, "module", key, at, message);
        }
    };
    if key.restype == ResType::UTS {
        placed(d, &gff.root, "");
        return;
    }
    for (i, s) in gff.root.list("SoundList").unwrap_or_default().iter().enumerate() {
        let tag = match s.get("Tag") {
            Some(Value::String(t)) if !t.is_empty() => format!(" ({})", String::from_utf8_lossy(t)),
            _ => String::new(),
        };
        placed(d, s, &format!("SoundList[{i}]{tag}, "));
    }
    let Some(props) = gff.root.child("AreaProperties") else { return };
    for (field, table, role) in crate::sounds::AREA_SOUNDS {
        let Some(row) = props.integer(field).and_then(|r| usize::try_from(r).ok()) else {
            continue;
        };
        let Some(t) = d.table_shared(table) else { continue };
        let at = format!("AreaProperties/{field}, {table}.2da row {row}");
        if row >= t.len() {
            if row > 0 {
                let message =
                    format!("{field} is row {row}, past the end of {table}.2da ({} rows)", t.len());
                d.push(Severity::Warning, Check::SoundSilent, "module", key, at, message);
            }
            continue;
        }
        let Some(cell) = t.get(row, "Resource").map(str::trim).filter(|c| !c.is_empty()) else {
            continue;
        };
        // A name longer than a resource's, 16 characters, is found as no
        // file: a music row's read in the client (Wyvern Crown of Cormyr's
        // row 83, mus_wc_ruralguitar, silent though the game's own
        // data/mus has a file of that name, while mus_wc_ruralg, the same
        // bytes, plays), an ambient row's by the same limit on names.
        let Ok(name) = ResRef::from_str(cell) else {
            let message = crate::sounds::long_name_message(role, cell);
            d.push(Severity::Warning, Check::SoundSilent, "module", key, at, message);
            continue;
        };
        let Some(unheard) = crate::sounds::unheard_by(rm, role, name, false, &read) else {
            continue;
        };
        let message = unheard.message(role, name);
        d.push(Severity::Warning, Check::SoundSilent, "module", key, at, message);
    }
}

/// Neighboring tiles that disagree about a corner or an edge they share
/// ([`mg_tiles::seams`]): its terrain, its height (the tile's corner and
/// its `Tile_Height`) or its crosser, where a tile that agrees could have
/// been chosen (not what a tileset's own group makes). Aurora's painting
/// leaves none; an area written by hand, by a script or by a tool can,
/// and its ground then doesn't meet. Of the game's own areas, nine
/// (`tiles.rs` `shipped_tile_grids_are_consistent`). A tile the tileset
/// lacks, or a hole in it, is the area's problems' to name, not this
/// check's.
fn tile_seams(d: &mut Doctor, key: ResKey, gff: &mg_gff::Gff, indexes: &TileIndexes) {
    let Some(name) = gff.root.resref("Tileset") else { return };
    // (Made outside the lock: two workers may make one at once, and
    // the first kept is the one both use.)
    let known = indexes.lock().ok().and_then(|made| made.get(&name).cloned());
    let index = known.unwrap_or_else(|| {
        let made = (crate::minimap::tileset(d.rm, &gff.root).ok())
            .map(|set| std::sync::Arc::new(mg_tiles::TileIndex::new(&set)));
        match indexes.lock() {
            Ok(mut all) => all.entry(name).or_insert(made).clone(),
            Err(_) => made,
        }
    });
    let Some(index) = index.as_deref() else { return };
    let int = |s: &Struct, label: &str| s.integer(label).unwrap_or(0);
    let width = u32::try_from(int(&gff.root, "Width")).unwrap_or(0).min(1024);
    let height = u32::try_from(int(&gff.root, "Height")).unwrap_or(0).min(1024);
    let tiles: Vec<mg_tiles::Placement> = (gff.root.list("Tile_List").unwrap_or(&[]).iter())
        .map(|t| mg_tiles::Placement {
            tile: u32::try_from(int(t, "Tile_ID")).unwrap_or(u32::MAX),
            orientation: int(t, "Tile_Orientation").rem_euclid(4) as u8,
            height: int(t, "Tile_Height").clamp(-1024, 1024) as i32,
        })
        .collect();
    for seam in mg_tiles::seams(index, width, height, &tiles).area {
        let (place, message) = seam_said(index, &seam);
        d.push(Severity::Warning, Check::TileSeam, "module", key, place, message);
    }
}

/// Where a seam is (`tile (3, 4)`) and what it says.
fn seam_said(index: &mg_tiles::TileIndex, seam: &mg_tiles::Seam) -> (String, String) {
    use mg_tiles::Disagreement;
    const CORNERS: [&str; 4] = ["south-west", "south-east", "north-east", "north-west"];
    const EDGES: [&str; 4] = ["south", "east", "north", "west"];
    let ((x, y), (nx, ny)) = (seam.cell, seam.neighbor);
    let side = match (i64::from(nx) - i64::from(x), i64::from(ny) - i64::from(y)) {
        (1, 0) => "east",
        (0, 1) => "north",
        (1, 1) => "north-east",
        _ => "north-west",
    };
    let corner =
        |c: mg_tiles::Corner| format!("{} at height {}", index.terrain_name(c.terrain), c.height);
    let crosser = |c: Option<mg_tiles::Crosser>| match c {
        Some(c) => format!("a {}", index.crosser_name(c)),
        None => "no crosser".to_string(),
    };
    let what: Vec<String> = (seam.what.iter())
        .map(|w| match *w {
            Disagreement::Corner { corner: k, here, there } => format!(
                "the {} corner is {} here, {} there",
                CORNERS[k],
                corner(here),
                corner(there)
            ),
            Disagreement::Edge { edge, here, there } => format!(
                "the {} edge has {} here, {} there",
                EDGES[edge],
                crosser(here),
                crosser(there)
            ),
        })
        .collect();
    let corners = seam.what.iter().any(|w| matches!(w, Disagreement::Corner { .. }));
    let edge = seam.what.iter().any(|w| matches!(w, Disagreement::Edge { .. }));
    let shows = match (corners, edge) {
        (true, true) => "their ground doesn't meet, and what crosses one tile's edge stops there",
        (true, false) => "their ground doesn't meet",
        _ => "what crosses one tile's edge stops there",
    };
    let message = format!(
        "the tile and its {side} neighbor, tile ({nx}, {ny}), don't fit together: {}. In the \
         tileset's terms {shows}: paint the corner or the crosser again (painting chooses tiles \
         that fit), or choose such tiles yourself",
        what.join("; ")
    );
    (format!("tile ({x}, {y})"), message)
}

/// Tags the project says must be unique (`[tags] unique` in its
/// `moonglow.toml`) that are on more than one object: `GetObjectByTag`
/// gives a script only one of them (the one placed later, of two in an
/// area: `engine_static_placeables`).
fn unique_tags(d: &mut Doctor, module: &Module, index: Option<&crate::tags::TagIndex>) {
    let rules = &module.project_file.tags;
    if !rules.any_unique() {
        return;
    }
    // (The tags as the module's areas were read for the other checks;
    // read for this one where they were not.)
    let read;
    let index = match index {
        Some(index) => index,
        None => {
            read = crate::tags::TagIndex::of_module(module);
            &read
        }
    };
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
///
/// Of one resource: `found` are its variables, `lists` what reads a
/// description's values and those read so far, and `unlisted` takes a
/// list that can't be read, which is said once a run
/// ([`SeenOnce::say_the_rest`]).
fn described_variables(
    d: &mut Doctor,
    module: &Module,
    key: ResKey,
    gff: &mg_gff::Gff,
    found: &[crate::var_desc::Found],
    lists: (&crate::var_desc::ModuleLookup, &mut HashMap<String, crate::var_desc::Choices>),
    unlisted: &mut Vec<Unlisted>,
) {
    use crate::var_desc::{self, Unknown, type_label};
    let vars = &module.project_file.variables;
    if vars.is_empty() {
        return;
    }
    let file = vars.file();
    let (lookup, choices) = lists;
    let mut unread: HashSet<&str> = HashSet::new();
    // What each object (by its path) has without what one of its
    // variables needs beside it. (An object's variables are found
    // together, `variables_of` walking each table once: a run of one
    // path is an object's, with no look through the file's others.)
    for object in found.chunk_by(|a, b| a.path == b.path) {
        let path = &object[0].path;
        let here: Vec<(&str, &str)> =
            object.iter().map(|f| (f.name.as_str(), f.value.as_str())).collect();
        for n in vars.missing(&here) {
            let table = var_desc::table_path(path);
            let place = crate::rename::describe(key, Some(gff), &table);
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
        let place = crate::rename::describe(key, Some(gff), &var_desc::table_path(&f.path));
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
        let c = choices.entry(desc.name.clone()).or_insert_with(|| desc.choices(lookup));
        if let Some(problem) = &c.problem
            && unread.insert(desc.name.as_str())
        {
            let message =
                format!("{}'s values ({file}:{}) can't be listed: {problem}", desc.name, desc.line);
            // (Said once a run, at the first of them: this resource's
            // first is noted, in its place among its findings.)
            let (before, name) = (d.out.len(), desc.name.clone());
            unlisted.push(Unlisted { before, name, at: at.clone(), message });
        }
        if let Some(problem) = desc.value_problem(&f.value, c) {
            let message = format!("{}: {problem} ({file}:{})", f.name, desc.line);
            d.push(Severity::Warning, Check::VarValue, "module", key, at, message);
        }
    }
}

/// Variables kept in the module's files as another type than every one of
/// its scripts reads (or sets) them as, by their names written out: the
/// script reads that type's empty value (`engine_local_variables`). A
/// variable the project describes with a type is held to that
/// (`var-type`) instead; one no script names by its name is left.
///
/// Of one resource: `found` are its variables, `reads` what the module's
/// scripts read or set by name ([`crate::var_desc::script_reads`]).
fn variables_read(
    d: &mut Doctor,
    module: &Module,
    key: ResKey,
    gff: &mg_gff::Gff,
    found: &[crate::var_desc::Found],
    reads: &std::collections::BTreeMap<String, Vec<crate::var_desc::Read>>,
) {
    use crate::var_desc::{self, type_label};
    if reads.is_empty() {
        return;
    }
    let vars = &module.project_file.variables;
    let article = |t: crate::vars::VarType| match t {
        crate::vars::VarType::Int => "an",
        _ => "a",
    };
    for f in found {
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
        let place = crate::rename::describe(key, Some(gff), &var_desc::table_path(&f.path));
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
    path: &GffPath,
    out: &mut Vec<(GffPath, String, bool)>,
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
        match at.model(table, rm) {
            Err(e) if !unjudged => out.push((path.field(at.field), e.to_string(), false)),
            Err(_) => {}
            // Its parts' models (an item's pieces and armor parts, a
            // body's skeleton and parts) must be there too, and what is
            // there be readable (one that isn't is found only when drawn).
            Ok(own) => {
                let names = ["baseitems", "cloakmodel", "appearance", "gender", "phenotype"];
                for name in names {
                    d.table(name);
                }
                let tables = &d.tables;
                let lookup = |name: &str| tables.get(name).and_then(Option::as_deref);
                let parts = at.parts(s, &lookup, rm);
                for p in &parts.problems {
                    out.push((path.field(p.field()), p.to_string(), p.problem.is_part()));
                }
                let models = own.map(|m| (None, m));
                let found = parts.found.into_iter().map(|(field, m)| (Some(field), m));
                for (field, model) in models.into_iter().chain(found) {
                    let Some(why) = d.unreadable(&model) else { continue };
                    let unread = mg_rules::appearance::Unread { field, model, why };
                    let problem = mg_rules::appearance::Problem::Unreadable(Box::new(unread));
                    let p = mg_rules::appearance::ModelProblem { at, problem };
                    out.push((path.field(p.field()), p.to_string(), p.problem.is_part()));
                }
            }
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
                    path.item("ClassList", i).field("Class"),
                    format!("class {v} isn't a classes.2da row; Aurora and the game crash"),
                    false,
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
                walk_objects(d, item, Some(t), &path.item(&label, i), out);
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

    /// An area's music and ambient sounds that the game plays nothing of
    /// (`sound-silent`): each row an area's fields name, of the tables the
    /// load order gives (the hak's over the game's), its `Resource` found
    /// as the role looks for it and judged by what it holds. Row 0 and a
    /// blank `Resource` are no music, and nothing is said of them.
    #[test]
    fn an_area_s_music_and_sounds_the_game_plays_nothing_of_are_said() {
        use crate::sounds::tests::{mp3, wave};
        let bmu = [mg_audio::BMU_HEADER.as_slice(), &mp3(3)].concat();
        let pcm = wave(1, &[0; 64]);
        let music = b"2DA V2.0\n\n   Resource Stinger1\n0 **** ****\n1 mus_ok mus_gone\n\
            2 mus_pcm ****\n3 mus_wav ****\n4 mus_gone ****\n5 mus_in_wave ****\n6 **** ****\n\
            7 mus_noise ****\n8 a_name_far_too_long_x ****\n";
        let ambient = b"2DA V2.0\n\n   Resource\n0 ****\n1 as_ok\n2 as_mp3\n3 as_bmu_ok\n\
            4 as_bmu_pcm\n";
        let noise = vec![0x42u8; 3000];
        let in_wave = wave(0x55, &mp3(3));
        let rm = {
            let mut rm = resman(&[
                ("ambientmusic", ResType::TWODA, music),
                ("ambientsound", ResType::TWODA, ambient),
                ("mus_ok", ResType::BMU, &bmu),
                ("mus_pcm", ResType::BMU, &pcm),
                ("mus_wav", ResType::WAV, &pcm),
                ("mus_in_wave", ResType::BMU, &in_wave),
                ("mus_noise", ResType::BMU, &noise),
                ("as_ok", ResType::WAV, &pcm),
                ("as_mp3", ResType::WAV, &bmu),
                ("as_bmu_ok", ResType::BMU, &bmu),
                ("as_bmu_pcm", ResType::BMU, &pcm),
            ]);
            // (The game's own tables, under the hak's: not read.)
            let mut game = MemContainer::new();
            game.insert(key("ambientmusic", ResType::TWODA), b"2DA V2.0\n\n   Resource\n".to_vec());
            rm.add(priority::KEY - 1, "game2", LayerClass::Key, game);
            rm
        };
        let mut m = Module::new();
        let area = |m: &mut Module, name: &str, fields: &[(&str, i32)]| {
            let mut props = Struct::new(100);
            for (f, v) in fields {
                props.set(f, Value::Int(*v));
            }
            let mut git = Gff::new(*b"GIT ");
            git.root.set("AreaProperties", Value::Struct(props));
            m.set_gff(key(name, ResType::GIT), &git).unwrap();
        };
        let fields = ["MusicDay", "MusicNight", "MusicBattle", "AmbientSndDay", "AmbientSndNight"];
        let named = |rows: [i32; 5]| -> Vec<(&str, i32)> { fields.into_iter().zip(rows).collect() };
        area(&mut m, "area1", &named([1, 2, 3, 1, 2]));
        // (Row 2 again, said again for this area; rows past the end.)
        area(&mut m, "area2", &named([2, 4, 0, 4, 99]));
        area(&mut m, "area3", &named([5, 6, 7, 3, 0]));
        area(&mut m, "area4", &[("MusicDay", 8), ("MusicNight", -1), ("MusicBattle", 900)]);
        // (A name longer than a resource's, row 8; no AreaProperties at all.)
        m.set_gff(key("area5", ResType::GIT), &Gff::new(*b"GIT ")).unwrap();
        // Placed sounds and a blueprint: each of their sounds, judged as
        // PlaySound's, and stereo where the object is positional.
        let sound_object = |positional: u8, tag: &str, names: &[&str]| {
            let mut s = Struct::new(6);
            s.set("Positional", Value::Byte(positional));
            s.set("Tag", Value::String(tag.as_bytes().to_vec()));
            let list = names.iter().map(|n| {
                let mut e = Struct::new(0);
                e.set("Sound", Value::resref(ResRef::from_str(n).unwrap()));
                e
            });
            s.set("Sounds", Value::List(list.collect()));
            s
        };
        let mut git = Gff::new(*b"GIT ");
        git.root.set(
            "SoundList",
            Value::List(vec![
                sound_object(1, "WIND", &["snd_stereo", "snd_mono", "snd_bmu_mp3", "snd_bare"]),
                sound_object(0, "WIDE", &["snd_stereo", "snd_only_bmu", "snd_gone"]),
            ]),
        );
        m.set_gff(key("area6", ResType::GIT), &git).unwrap();
        let mut uts = Gff::new(*b"UTS ");
        uts.root = sound_object(1, "BP", &["snd_stereo", "snd_noise"]);
        m.set_gff(key("snd_bp", ResType::UTS), &uts).unwrap();
        let stereo = crate::sounds::tests::wave_of(1, 2, &[0; 64]);
        let rm = {
            let mut rm = rm;
            let mut more = MemContainer::new();
            for (n, t, data) in [
                ("snd_stereo", ResType::WAV, stereo),
                ("snd_mono", ResType::WAV, pcm.clone()),
                ("snd_bmu_mp3", ResType::WAV, bmu.clone()),
                ("snd_bare", ResType::WAV, mp3(3)),
                ("snd_only_bmu", ResType::BMU, bmu.clone()),
                ("snd_noise", ResType::WAV, vec![7; 900]),
            ] {
                more.insert(key(n, t), data);
            }
            rm.add(priority::HAK_USER + 1, "sounds.hak", LayerClass::Erf, more);
            rm
        };
        let found: Vec<(String, String, String)> = examine(&m, &rm, TalkTables::default())
            .into_iter()
            .filter(|f| f.check == "sound-silent")
            .inspect(|f| assert_eq!((f.severity, f.source.as_str()), (Severity::Warning, "module")))
            .map(|f| (f.resource.to_string(), f.at, f.message))
            .collect();
        let row = |area: &str, field: &str, table: &str, row: u32, message: &str| {
            let at = format!("AreaProperties/{field}, {table}.2da row {row}");
            (format!("{area}.git"), at, message.to_string())
        };
        assert_eq!(
            found,
            [
                row(
                    "area1",
                    "MusicNight",
                    "ambientmusic",
                    2,
                    "Music mus_pcm.bmu is a WAVE, which \
                     the game does not play as music (it must hold MP3)"
                ),
                row(
                    "area1",
                    "MusicBattle",
                    "ambientmusic",
                    3,
                    "Battle music mus_wav exists only \
                     as mus_wav.wav, which the game does not look for (music must be a .bmu \
                     holding MP3)"
                ),
                row(
                    "area1",
                    "AmbientSndNight",
                    "ambientsound",
                    2,
                    "Ambient sound as_mp3.wav \
                     holds MP3, which the game does not play under .wav (it plays a WAVE there, \
                     or MP3 as as_mp3.bmu)"
                ),
                row(
                    "area2",
                    "MusicDay",
                    "ambientmusic",
                    2,
                    "Music mus_pcm.bmu is a WAVE, which \
                     the game does not play as music (it must hold MP3)"
                ),
                row(
                    "area2",
                    "MusicNight",
                    "ambientmusic",
                    4,
                    "Music mus_gone is found nowhere \
                     (the game looks for mus_gone.bmu)"
                ),
                row(
                    "area2",
                    "AmbientSndDay",
                    "ambientsound",
                    4,
                    "Ambient sound as_bmu_pcm.bmu \
                     is a WAVE, which the game does not play under .bmu (it plays MP3 there, or \
                     a WAVE as as_bmu_pcm.wav)"
                ),
                row(
                    "area2",
                    "AmbientSndNight",
                    "ambientsound",
                    99,
                    "AmbientSndNight is row 99, \
                     past the end of ambientsound.2da (5 rows)"
                ),
                row(
                    "area3",
                    "MusicBattle",
                    "ambientmusic",
                    7,
                    "Battle music mus_noise.bmu is \
                     not a sound the game plays (neither WAVE nor MP3)"
                ),
                row(
                    "area4",
                    "MusicDay",
                    "ambientmusic",
                    8,
                    "Music a_name_far_too_long_x is a name longer than 16 characters, which the \
                     game does not find"
                ),
                row(
                    "area4",
                    "MusicBattle",
                    "ambientmusic",
                    900,
                    "MusicBattle is row 900, past \
                     the end of ambientmusic.2da (9 rows)"
                ),
                (
                    "area6.git".into(),
                    "SoundList[0] (WIND), Sounds[0]".into(),
                    "Sound snd_stereo.wav is stereo, which the game does not play from a \
                     positional sound (make it mono, or the sound area-wide)"
                        .into()
                ),
                (
                    "area6.git".into(),
                    "SoundList[0] (WIND), Sounds[3]".into(),
                    "Sound snd_bare.wav is MP3 without the BMU V1.0 header, which the game does \
                     not play (it plays a WAVE there, or MP3 behind the header)"
                        .into()
                ),
                (
                    "snd_bp.uts".into(),
                    "Sounds[0]".into(),
                    "Sound snd_stereo.wav is stereo, which the game does not play from a \
                     positional sound (make it mono, or the sound area-wide)"
                        .into()
                ),
                (
                    "snd_bp.uts".into(),
                    "Sounds[1]".into(),
                    "Sound snd_noise.wav is not a sound the game plays (neither WAVE nor MP3)"
                        .into()
                ),
            ]
        );
        // Run, it is timed; left out, it isn't run: nothing is said, and
        // nothing timed of it.
        let mut timings = Timings::default();
        examine_groups(&m, &rm, TalkTables::default(), &Skip::NONE, &mut timings, None);
        let part = "  music, ambient and placed sounds, added up";
        assert!(timings.parts.iter().any(|(p, _)| p == part), "{:?}", timings.parts);
        let skip = Skip::parse(&["sound-silent"]).unwrap();
        let mut timings = Timings::default();
        let left = examine_groups(&m, &rm, TalkTables::default(), &skip, &mut timings, None);
        assert!(left.iter().all(|f| f.check != "sound-silent"), "{left:#?}");
        assert!(
            timings.parts.iter().all(|(p, _)| !p.contains("placed sounds")),
            "{:?}",
            timings.parts
        );
        let skip = Skip::parse(&["objects"]).unwrap();
        let left = examine_groups(&m, &rm, TalkTables::default(), &skip, &mut timings, None);
        assert!(left.iter().all(|f| f.check != "sound-silent"));
        assert_eq!(Check::from_id("sound-silent").map(Check::group), Some(Group::Objects));
    }

    /// A 2DA is parsed once a run, for every doctor beside the first (the
    /// workers of the module's resources): the table one reads is the
    /// one the others are handed, as the game resolves it; a table that
    /// isn't there is not looked for again either.
    #[test]
    fn a_table_is_read_once_for_the_doctors_beside() {
        let rm = resman(&[]);
        let d = Doctor {
            rm: &rm,
            tlk: TalkTables::default(),
            shaders: None,
            memo: None,
            learned: Vec::new(),
            skip: &NOTHING,
            out: Vec::new(),
            tables: HashMap::new(),
            tables_read: TablesRead::default(),
            models_read: ModelsRead::default(),
        };
        let (mut one, mut two) = (d.beside(), d.beside());
        let first = one.table_shared("placeables").unwrap();
        assert_eq!(first.len(), 1);
        assert!(std::sync::Arc::ptr_eq(&first, &two.table_shared("placeables").unwrap()));
        assert!(one.table("nothere").is_none() && two.table("nothere").is_none());
        let read = d.tables_read.lock().unwrap();
        assert_eq!(read.len(), 2);
        assert!(read["nothere"].is_none());
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
        let dir = mg_testkit::scratch_dir("mg-doctor-shaders");
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
        let root = mg_testkit::scratch_dir("mg-doctor-nui");
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

    /// A material's textures are in the load order (a builder's report: a
    /// hak's materials named textures a build step had renamed, and Verify
    /// said nothing): one held by any layer (the hak's own, the game's) as
    /// `.dds`, `.tga` or `.plt` is found, `null` names none, and one held by
    /// none is an error at its slot. The material the game reads is the
    /// one judged: a hak's own over the game's.
    #[test]
    fn a_material_s_textures_are_in_the_load_order() {
        let mut game = MemContainer::new();
        game.insert(key("game_tex", ResType::DDS), vec![0; 16]);
        game.insert(key("over_mat", ResType::MTR), b"texture0 lost_in_game\n".to_vec());
        let mut hak = MemContainer::new();
        let mtr = "texture0 my_tex\ntexture1 game_tex\ntexture2 null\ntexture3 my_mat_n\n\
                   texture4 my_skin\n";
        hak.insert(key("my_mat", ResType::MTR), mtr.as_bytes().to_vec());
        hak.insert(key("bare_mat", ResType::MTR), b"texture0 bare_d\n".to_vec());
        hak.insert(key("my_tex", ResType::TGA), vec![0; 64]);
        hak.insert(key("my_skin", ResType::PLT), vec![0; 64]);
        hak.insert(key("over_mat", ResType::MTR), b"texture0 my_tex\n".to_vec());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Erf, game);
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine_groups(
            &Module::new(),
            &rm,
            TalkTables::default(),
            &Skip::NONE,
            &mut Timings::default(),
            None,
        );
        let found: Vec<(String, String, String)> = (f.iter())
            .filter(|f| f.check == "mtr-texture")
            .map(|f| (f.resource.to_string(), f.at.clone(), f.message.clone()))
            .collect();
        assert_eq!(found.len(), 2, "{found:#?}");
        let of =
            |m: &str| f.iter().find(|f| f.check == "mtr-texture" && f.resource.to_string() == m);
        let (bare, map) = (of("bare_mat.mtr").unwrap(), of("my_mat.mtr").unwrap());
        // (Its color: an error; a map: a warning.)
        assert_eq!((bare.at.as_str(), bare.severity), ("texture0", Severity::Error));
        assert!(
            bare.message.contains("bare_d is in no layer") && bare.message.contains("untextured")
        );
        assert_eq!((map.at.as_str(), map.severity), ("texture3", Severity::Warning));
        assert!(map.message.contains("my_mat_n is in no layer"), "{}", map.message);
        // Left out, as any check.
        let skipped = examine_groups(
            &Module::new(),
            &rm,
            TalkTables::default(),
            &Skip::parse(&["mtr-texture"]).unwrap(),
            &mut Timings::default(),
            None,
        );
        assert!(skipped.iter().all(|f| f.check != "mtr-texture"));
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
        let kept = || -> Vec<String> {
            (std::fs::read_dir(dir.join("verify")).unwrap())
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        };
        let (first, read_first) = run(&rm, Some(&cache));
        assert_eq!(first, plain);
        assert_eq!(read_first, ["fx.mdl", "loose.mdl", "tst_a01.mdl", "tst_b01.mdl"]);
        // (Its file: its label, no folder, its version.)
        let was = kept();
        assert_eq!(was.len(), 1, "{was:?}");
        assert!(was[0].starts_with("hak_stamped-00000000-") && was[0].ends_with(".json"));
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
        let now = kept();
        assert_eq!(now.len(), 1, "{now:?}");
        assert!(now[0].starts_with("hak_stamped-00000000-") && now != was, "{now:?} {was:?}");
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
                path: crate::refs::Location::Gff(GffPath::root().field("Script")),
                kind: crate::refs::RefKind::Script,
                target: ResRef::from_str("gone").unwrap(),
            };
            crate::verify::Missing {
                category: crate::verify::Category::of(&reference),
                reference,
                uncompiled: false,
                only_local: None,
                found_as: None,
            }
        };
        let mut gone = vec![missing(ResType::UTC), missing(ResType::UTD)];
        // Found only in the player's own override folder or Workshop item,
        // a warning each, left out by its kind (`references:local`, all).
        let only = |t: ResType, l: crate::verify::Local| crate::verify::Missing {
            only_local: Some(l),
            ..missing(t)
        };
        let mut locals = vec![
            only(ResType::UTD, crate::verify::Local::Override),
            only(ResType::UTD, crate::verify::Local::Workshop),
            missing(ResType::UTD),
        ];
        let by = Skip::parse(&["references:override"]).unwrap();
        locals.retain(|m| !by.missing(m));
        assert_eq!(
            locals.iter().map(|m| m.only_local).collect::<Vec<_>>(),
            [Some(crate::verify::Local::Workshop), None]
        );
        let all = Skip::parse(&["References:Local"]).unwrap();
        assert_eq!(all.words(), ["references:local"]);
        locals.retain(|m| !all.missing(m));
        assert_eq!(locals.len(), 1);
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

    /// A hak's tables are held to their descriptions too: one beside the
    /// hak's file, and one beside the table's source in the module's
    /// project (the folder the hak is built from), found as the only
    /// table of its name there that has one; a family's description
    /// (`tables = ["loot_*"]`) is a description of each table it names.
    #[test]
    fn a_hak_s_described_tables_are_held_to_their_descriptions() {
        use mg_resman::ErfContainer;
        let dir = mg_testkit::scratch_dir("doctor-described-haks");
        let project = dir.join("project");
        std::fs::create_dir_all(project.join(".git")).unwrap();
        std::fs::create_dir_all(project.join("src/hak")).unwrap();
        std::fs::create_dir_all(dir.join("hak")).unwrap();
        let table = b"2DA V2.0\n\n  Label Chance\n0 a 50\n1 b 500\n";
        let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
        for name in ["loot", "spawn", "loot_gold", "plain"] {
            hak.add(ResRef::from_str(name).unwrap(), ResType::TWODA, table.to_vec()).unwrap();
        }
        let hak_path = dir.join("hak/my.hak");
        std::fs::write(&hak_path, hak.to_bytes().unwrap()).unwrap();
        let int = "[columns.Chance]\nkind = \"int\"\nmax = 100\n";
        std::fs::write(dir.join("hak/loot.2da.toml"), int).unwrap();
        std::fs::write(dir.join("hak/family.2da.toml"), format!("tables = \"loot_*\"\n{int}"))
            .unwrap();
        std::fs::write(project.join("src/hak/spawn.2da"), table).unwrap();
        std::fs::write(project.join("src/hak/spawn.2da.toml"), int).unwrap();
        let mut m = Module::new();
        m.save_as(&crate::ModuleLocation::Archive(project.join("my.mod"))).unwrap();
        let m = Module::open(&project.join("my.mod")).unwrap();
        let mut rm = ResMan::new();
        let erf = ErfContainer::open(&hak_path).unwrap();
        rm.add(priority::HAK_USER, "hak:my", LayerClass::Erf, erf);
        let f = examine(&m, &rm, TalkTables::default());
        let described: Vec<(String, String, &str)> = f
            .iter()
            .filter(|f| f.check == "2da-described")
            .map(|f| (f.resource.to_string(), f.at.clone(), f.source.as_str()))
            .collect();
        assert_eq!(
            described,
            [
                ("loot.2da".to_string(), "row 1, Chance".to_string(), "hak:my"),
                ("loot_gold.2da".into(), "row 1, Chance".into(), "hak:my"),
                ("spawn.2da".into(), "row 1, Chance".into(), "hak:my"),
            ],
            "{f:#?}"
        );
    }

    /// A class's spells: its SpellTableColumn is a column spells.2da has,
    /// and its packages' preferred spells are ones it learns (by its
    /// column, or a domain for a class that picks them); a fault the
    /// game's own tables have is not said.
    #[test]
    fn a_class_s_spell_tables_are_held_to_spells_2da() {
        let classes = b"2DA V2.0\n\n   Label SpellCaster SpellTableColumn PickDomains\n\
                        0  Cleric 1 Cleric 1\n1  Mine 1 Mine 0\n2  Odd 1 Nowhere 0\n3  Fighter 0 **** 0\n";
        let packages = b"2DA V2.0\n\n   Label ClassID SpellPref2DA\n0  Cler 0 packsp_cl\n\
                         1  Mine 1 packsp_mine\n2  Fight 3 packsp_mine\n";
        let spells = b"2DA V2.0\n\n   Label Cleric Mine\n0  Aid 1 ****\n1  Bolt **** 2\n\
                       2  Ward **** ****\n";
        let domains = b"2DA V2.0\n\n   Label Level_1 Level_2\n0  Air 2 ****\n";
        let cl = b"2DA V2.0\n\n   SpellIndex\n0  0\n1  2\n2  1\n";
        let mine = b"2DA V2.0\n\n   SpellIndex\n0  1\n1  0\n";
        let rm = resman(&[
            ("classes", ResType::TWODA, classes),
            ("packages", ResType::TWODA, packages),
            ("spells", ResType::TWODA, spells),
            ("domains", ResType::TWODA, domains),
            ("packsp_cl", ResType::TWODA, cl),
            ("packsp_mine", ResType::TWODA, mine),
        ]);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let said: Vec<(String, String)> = f
            .iter()
            .filter(|f| f.check == "2da-spells")
            .map(|f| (f.resource.to_string(), f.at.clone()))
            .collect();
        // (The cleric's Ward is a domain's; a fighter's package's spells
        // are no caster's.)
        assert_eq!(
            said,
            [
                ("classes.2da".to_string(), "row 2, SpellTableColumn".to_string()),
                ("packsp_cl.2da".into(), "row 2, SpellIndex".into()),
                ("packsp_mine.2da".into(), "row 1, SpellIndex".into()),
            ],
            "{f:#?}"
        );
        let bolt = f.iter().find(|f| f.at == "row 2, SpellIndex").unwrap();
        assert!(bolt.message.contains("spell 1 (Bolt) is no spell of Cleric's"), "{bolt:?}");
        // The game's own tables, whose package's spells a hak's spells.2da
        // takes from the class: the game's table said once, and the fault
        // the game's tables have already (its Bolt) not at all.
        let mut game = MemContainer::new();
        for (n, data) in [
            ("classes", &classes[..]),
            ("packages", &packages[..]),
            ("spells", &spells[..]),
            ("packsp_cl", &cl[..]),
        ] {
            game.insert(key(n, ResType::TWODA), data.to_vec());
        }
        let mut hak = MemContainer::new();
        let fewer = b"2DA V2.0\n\n   Label Cleric Mine\n0  Aid **** ****\n1  Bolt **** 2\n\
                      2  Ward 3 ****\n";
        hak.insert(key("spells", ResType::TWODA), fewer.to_vec());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, game);
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let said: Vec<&Finding> = f.iter().filter(|f| f.check == "2da-spells").collect();
        // (Odd's column of none, the game's classes.2da's too, neither.)
        assert_eq!(said.len(), 1, "{said:#?}");
        assert_eq!(
            (said[0].resource.to_string(), said[0].at.as_str()),
            ("packsp_cl.2da".into(), "")
        );
        let message = &said[0].message;
        assert!(
            message.contains("learn 1 of the spells it prefers") && message.contains("(rows 0;")
        );
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

    /// A tile whose model draws `replace_tex` and whose `Tile_ReplaceTex`
    /// the game crashes on (0, or past replacetexture.2da) is an error;
    /// a row of the table, or no field, is not, nor any value on a tile
    /// whose model has no `replace_tex`.
    #[test]
    fn a_tile_s_replacement_texture_the_game_crashes_on_is_found() {
        let model = |name: &str, bitmap: &str| {
            format!(
                "newmodel {name}\nsetsupermodel {name} NULL\nclassification tile\n\
beginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
node trimesh flag\n  parent {name}\n  bitmap {bitmap}\n  verts 3\n    0 0 0\n    1 0 0\n\
    0 1 0\n  faces 1\n    0 1 2 1 0 1 2 1\nendnode\nendmodelgeom {name}\ndonemodel {name}\n"
            )
        };
        let set = "[GENERAL]\nName=TST01\n[TILES]\nCount=2\n[TILE0]\nModel=tst01_flag\n\
[TILE1]\nModel=tst01_grass\n";
        let flag = model("tst01_flag", "replace_tex");
        let grass = model("tst01_grass", "tst_grass");
        let table = "2DA V2.0\n\n   TEXTURENAME\n0  mg_row0\n1  mg_row1\n";
        let rm = resman(&[
            ("tst01", ResType::SET, set.as_bytes()),
            ("tst01_flag", ResType::MDL, flag.as_bytes()),
            ("tst01_grass", ResType::MDL, grass.as_bytes()),
            ("replacetexture", ResType::TWODA, table.as_bytes()),
        ]);
        let mut m = Module::new();
        let mut are = Gff::new(*b"ARE ");
        are.root.set("Tileset", Value::ResRef(b"tst01".to_vec()));
        are.root.set("Width", Value::Int(3));
        let tiles: [(i32, Option<u8>); 7] = [
            (0, Some(0)),
            (0, Some(1)),
            (0, Some(2)),
            (0, None),
            (0, Some(3)),
            (1, Some(0)),
            (1, Some(9)),
        ];
        let list = tiles
            .iter()
            .map(|&(id, v)| {
                let mut t = mg_gff::Struct::new(1);
                t.set("Tile_ID", Value::Int(id));
                if let Some(v) = v {
                    t.set("Tile_ReplaceTex", Value::Byte(v));
                }
                t
            })
            .collect();
        are.root.set("Tile_List", Value::List(list));
        m.set_gff(key("field", ResType::ARE), &are).unwrap();
        let f = examine(&m, &rm, TalkTables::default());
        let found: Vec<&Finding> = f.iter().filter(|x| x.check == "tile-replace").collect();
        let at: Vec<&str> = found.iter().map(|x| x.at.as_str()).collect();
        assert_eq!(at.len(), 2, "{found:?}");
        assert_eq!(at, ["tile (0, 0), Tile_ReplaceTex", "tile (1, 1), Tile_ReplaceTex"]);
        assert!(found.iter().all(|x| x.severity == Severity::Error));
        assert!(found[0].message.contains("Tile_ReplaceTex is 0"), "{}", found[0].message);
        assert!(found[1].message.contains("past replacetexture.2da's 2 rows"));
        assert_eq!(Group::of(found[0]), Group::Objects);
    }

    /// Neighboring tiles that disagree about a corner, its height or an
    /// edge's crosser are named with the neighbor and what each has; an
    /// area whose tiles fit, and a tile the tileset lacks, say nothing.
    #[test]
    fn tiles_that_do_not_fit_their_neighbors_are_found() {
        let set = "[GENERAL]\nName=TST03\nTransition=5\n\
[TERRAIN TYPES]\nCount=2\n[TERRAIN0]\nName=Grass\n[TERRAIN1]\nName=Water\n\
[CROSSER TYPES]\nCount=1\n[CROSSER0]\nName=Road\n\
[TILES]\nCount=3\n\
[TILE0]\nModel=tst03_a\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\n\
[TILE1]\nModel=tst03_b\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Water\nBottomRight=Grass\n\
[TILE2]\nModel=tst03_c\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\n\
Top=Road\nBottom=Road\n[GROUPS]\nCount=0\n";
        let rm = resman(&[("tst03", ResType::SET, set.as_bytes())]);
        let mut m = Module::new();
        // (id, orientation, height), row-major from the south-west.
        let area = |m: &mut Module, name: &str, width: i32, tiles: &[(i32, i32, i32)]| {
            let mut are = Gff::new(*b"ARE ");
            are.root.set("Tileset", Value::ResRef(b"tst03".to_vec()));
            are.root.set("Width", Value::Int(width));
            are.root.set("Height", Value::Int(tiles.len() as i32 / width));
            let list = (tiles.iter())
                .map(|&(id, o, h)| {
                    let mut t = Struct::new(1);
                    t.set("Tile_ID", Value::Int(id));
                    t.set("Tile_Orientation", Value::Int(o));
                    t.set("Tile_Height", Value::Int(h));
                    t
                })
                .collect();
            are.root.set("Tile_List", Value::List(list));
            m.set_gff(key(name, ResType::ARE), &are).unwrap();
        };
        // Water at (0, 0)'s north-east corner, Grass all round (1, 0); a
        // road that stops at the tile north of it; a tile a step up.
        area(&mut m, "seams", 2, &[(1, 2, 0), (0, 0, 0), (2, 0, 0), (0, 0, 1)]);
        // The water corner shared by four tiles turned to meet it, and a
        // tile the tileset lacks.
        area(&mut m, "fitting", 2, &[(1, 2, 0), (1, 3, 0), (1, 1, 0), (1, 0, 0)]);
        area(&mut m, "unknown", 2, &[(0, 0, 0), (7, 0, 0)]);
        let f = examine(&m, &rm, TalkTables::default());
        let found: Vec<&Finding> = f.iter().filter(|x| x.check == "tile-seam").collect();
        let at: Vec<(String, &str)> =
            found.iter().map(|x| (x.resource.to_string(), x.at.as_str())).collect();
        assert_eq!(
            at,
            [
                ("seams.are".into(), "tile (0, 0)"),
                ("seams.are".into(), "tile (0, 0)"),
                ("seams.are".into(), "tile (1, 0)"),
                ("seams.are".into(), "tile (0, 1)"),
            ],
            "{found:#?}"
        );
        assert!(found.iter().all(|x| x.severity == Severity::Warning));
        let said: Vec<&str> = found.iter().map(|x| x.message.as_str()).collect();
        assert!(
            said[0].contains("its east neighbor, tile (1, 0)")
                && said[0].contains(
                    "the north-east corner is Water at height 0 here, Grass at height 0 there"
                ),
            "{}",
            said[0]
        );
        assert!(
            said[1].contains(
                "its north neighbor, tile (0, 1), don't fit together: the north-east corner is \
                 Water at height 0 here, Grass at height 0 there; the north edge has no crosser \
                 here, a Road there."
            ),
            "{}",
            said[1]
        );
        let step = "the north-west corner is Grass at height 0 here, Grass at height 1 there; \
                    the north-east corner is Grass at height 0 here, Grass at height 1 there.";
        assert!(said[2].contains("north neighbor, tile (1, 1)") && said[2].contains(step));
        assert!(said[3].contains("east neighbor, tile (1, 1)"), "{}", said[3]);
        assert!(said[0].contains("In the tileset's terms their ground doesn't meet: paint"));
        assert!(said[1].contains("doesn't meet, and what crosses one tile's edge stops there"));
        assert_eq!(Check::TileSeam.group(), Group::Objects);
    }

    /// A placeable whose Appearance is past 65535, placed or a blueprint:
    /// the game draws the row less 65536.
    #[test]
    fn a_placeable_past_row_65535_is_found() {
        let mut m = Module::new();
        for (name, row) in [("ok_box", 65535u32), ("far_box", 65537)] {
            let mut bp = mg_gff::Gff::new(*b"UTP ");
            bp.root.set("Appearance", Value::Dword(row));
            m.set_gff(key(name, ResType::UTP), &bp).unwrap();
        }
        let mut git = mg_gff::Gff::new(*b"GIT ");
        let mut placed = Struct::new(9);
        placed.set("Appearance", Value::Dword(70000));
        git.root.set("Placeable List", Value::List(vec![Struct::new(9), placed]));
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<(&str, String, String)> = f
            .iter()
            .filter(|x| x.check == "appearance-range")
            .map(|x| (&*x.check, x.resource.to_string(), x.message.clone()))
            .collect();
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[0].1, "area.git");
        assert!(found[0].2.contains("row 70000") && found[0].2.contains("row 4464's model"));
        assert_eq!(found[1].1, "far_box.utp");
        assert!(found[1].2.contains("draws row 1's model instead"), "{}", found[1].2);
        assert_eq!(Check::AppearanceRange.group(), Group::Objects);
    }

    /// The walkmesh bake's checks: [bake]'s problems and a range over rows
    /// the table holds (bake-rules), rows the haks' table lacks or has
    /// with another model (bake-row-pending), and rows and baked models
    /// no placeable uses (bake-unused).
    #[test]
    fn the_bake_s_rules_rows_and_leftovers_are_checked() {
        let dir = mg_testkit::scratch_dir("doctor-bake");
        let mut m = Module::new();
        let mut git = mg_gff::Gff::new(*b"GIT ");
        let mut placed = Struct::new(9);
        placed.set("Appearance", Value::Dword(3));
        git.root.set("Placeable List", Value::List(vec![placed]));
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        m.save_as(&crate::ModuleLocation::Folder(dir.clone())).unwrap();
        // Without [bake]: nothing. (The hak's layer named as the game data
        // names a module's hak.)
        let mut rm = resman(&[]);
        let mut hak = MemContainer::new();
        hak.insert(
            key("placeables", ResType::TWODA),
            b"2DA V2.0\n\n   Label ModelName\n0 Chair plc_chair\n1 **** ****\n2 Lamp plc_lamp\n\
              3 Bk mgbk_0001\n"
                .to_vec(),
        );
        for m in ["mgbk_0001", "mgbk_0002", "mgbk_0007"] {
            hak.insert(key(m, ResType::MDL), b"x".to_vec());
        }
        rm.add(priority::HAK_USER, "hak:my", LayerClass::Erf, hak);
        let m0 = Module::open(&dir).unwrap();
        assert!(bake(&m0, &rm, Codepage::default(), &Skip::NONE).is_empty());
        std::fs::write(
            dir.join("moonglow.toml"),
            "[bake]\nrows = \"1-5\"\nhak = \"my\"\nnames = \"mgbk_\"\n",
        )
        .unwrap();
        let row = |r: u32, model: &str| {
            format!("[[row]]\nrow = {r}\ncopy_of = 0\nmodel = \"{model}\"\nlabel = \"x\"\n")
        };
        let rows = [row(3, "mgbk_0001"), row(4, "mgbk_0002"), row(2, "mgbk_0003")].concat();
        std::fs::write(dir.join("baked_rows.toml"), rows).unwrap();
        let m1 = Module::open(&dir).unwrap();
        let f = bake(&m1, &rm, Codepage::default(), &Skip::NONE);
        let found: Vec<(&str, &str)> = f.iter().map(|x| (&*x.check, &*x.at)).collect();
        assert_eq!(
            found,
            [
                ("bake-rules", "moonglow.toml [bake]"),
                ("bake-row-pending", "baked_rows.toml, row 2"),
                ("bake-row-pending", "baked_rows.toml, row 4"),
                ("bake-unused", "baked_rows.toml, row 2"),
                ("bake-unused", "baked_rows.toml, row 4"),
                ("bake-unused", "my.hak"),
            ],
            "{f:#?}"
        );
        assert!(
            f[0].message.contains("(1-5), row 2 is one placeables.2da has"),
            "{}",
            f[0].message
        );
        assert!(f[1].message.contains("another model (plc_lamp)"), "{}", f[1].message);
        assert!(f[2].message.contains("has no row 4"), "{}", f[2].message);
        assert!(f[5].message.contains("mgbk_0007.mdl"), "{}", f[5].message);
        // Skipped by its id; a range past 65535 and a broken rows file said.
        let skip = Skip::parse(&["bake-unused"]).unwrap();
        assert_eq!(bake(&m1, &rm, Codepage::default(), &skip).len(), 3);
        std::fs::write(dir.join("moonglow.toml"), "[bake]\nrows = \"65000-70000\"\nhak = \"my\"\n")
            .unwrap();
        std::fs::write(dir.join("baked_rows.toml"), "[[row]]\nrow = 3\n").unwrap();
        let m2 = Module::open(&dir).unwrap();
        let f = bake(&m2, &rm, Codepage::default(), &Skip::NONE);
        let found: Vec<(&str, Severity)> = f.iter().map(|x| (&*x.check, x.severity)).collect();
        assert_eq!(found, [("bake-rules", Severity::Warning), ("bake-rules", Severity::Error)]);
        assert!(f[0].message.contains("past row 65535"), "{}", f[0].message);
        assert!(f[1].message.contains("no copy_of"), "{}", f[1].message);
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

    /// While a baked row waits for the project's build, its layer is over
    /// the haks with a placeables.2da of its own: the hak's copy is still
    /// the one the game reads, and the one checked (the review's V3: it was
    /// no longer "the winning copy", and none of the 2DA checks ran on it).
    #[test]
    fn a_hak_s_placeables_are_checked_under_pending_baked_rows() {
        let table = b"2DA V2.0\n\n   Label ModelName StrRef\n0  Chair plc_chair 5\n1  Stool plc_stool 16777217\n2  Box plc_box 3 Crate plc_crate 4\n";
        let mut rm = resman(&[("placeables", ResType::TWODA, table)]);
        let tlk = TalkTables { base: 100, custom: Some(1) };
        let before = examine(&Module::new(), &rm, tlk);
        let mut pending = mg_resman::MemContainer::new();
        let sound = b"2DA V2.0\n\n   Label ModelName StrRef\n0  Chair plc_chair 5\n";
        pending.insert(key("placeables", ResType::TWODA), sound.to_vec());
        let (at, label) = (crate::bake::PENDING_PRIORITY, crate::bake::PENDING_LABEL);
        rm.add(at, label, mg_resman::LayerClass::Erf, pending);
        assert_eq!(rm.origin(&key("placeables", ResType::TWODA)), Some(label));
        let f = examine(&Module::new(), &rm, tlk);
        assert_eq!(
            checks(&f),
            [("2da-row", "row 2".into()), ("2da-strref", "row 1, StrRef".into())]
        );
        assert_eq!(f, before);
    }

    /// Verify and the 2DA editor hold a table's cells to one rule (the
    /// review's V21: it was written twice): a table with a fault of each
    /// kind (a string, a row of another table, a table that isn't there)
    /// has the same cells wrong in both, with the same words.
    #[test]
    fn verify_and_the_2da_editor_fault_the_same_cells() {
        let classes = b"2DA V2.0\n\n   Label Name Package FeatsTable\n0  A 5 0 cls_feat_a\n\
                        1  B 99999 9 cls_feat_none\n2  C 16777217 **** ****\n";
        let packages = b"2DA V2.0\n\n   Label\n0  P\n";
        let feats = b"2DA V2.0\n\n   FeatIndex\n0  0\n";
        let rm = resman(&[
            ("classes", ResType::TWODA, classes),
            ("packages", ResType::TWODA, packages),
            ("cls_feat_a", ResType::TWODA, feats),
        ]);
        let tlk = TalkTables { base: 100, custom: Some(1) };
        let f = examine(&Module::new(), &rm, tlk);
        let mut verify: Vec<(String, String)> = (f.iter())
            .filter(|x| x.resource == key("classes", ResType::TWODA))
            .filter(|x| x.check == "2da-strref" || x.check == "2da-ref")
            .map(|x| (x.at.clone(), x.message.replace(" (\"Bad Strref\")", "")))
            .collect();
        verify.sort();
        let read = |name: &str| {
            let data = rm.get(&ResKey::parse(name, ResType::TWODA)?).ok()?;
            TwoDa::parse(&data, Codepage::default()).ok().map(std::sync::Arc::new)
        };
        let t = read("classes").unwrap();
        let kinds = column_kinds_as_named("classes", &read);
        let mut editor: Vec<(String, String)> =
            (table_faults("classes", &t, &kinds, Some(&read), tlk).into_iter())
                .map(|x| (format!("row {}, {}", x.row, t.columns()[x.column]), x.message))
                .collect();
        editor.sort();
        assert_eq!(verify, editor);
        let cells: Vec<&str> = editor.iter().map(|(at, _)| at.as_str()).collect();
        assert_eq!(
            cells,
            ["row 1, FeatsTable", "row 1, Name", "row 1, Package", "row 2, Name"],
            "{editor:#?}"
        );
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

    /// The lines typed text takes: room left, mg tlk-alloc's lines in the
    /// range, builders sharing lines, a builder with no part of their own.
    #[test]
    fn the_lines_typed_text_takes_are_held_to_the_table() {
        let mut m = Module::new();
        let none = talk_range(&m, None, &Skip::default());
        assert!(none.is_empty(), "no [talk]: nothing said");
        m.project_file.apply(
            "moonglow.toml",
            "[talk]\nlines = \"0-19\"\n[talk.builders]\nab = \"0-9\"\ncd = \"8-15\"\n",
            false,
        );
        m.project_file.user.builder_code = Some("ab".into());
        let mut t = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
        (0..9).for_each(|i| t.entries.push(mg_tlk::TlkEntry::text(format!("line {i}"))));
        let f = talk_range(&m, Some(&t), &Skip::default());
        let said: Vec<(&str, Severity)> =
            f.iter().map(|f| (f.message.as_str(), f.severity)).collect();
        assert_eq!(f.len(), 2, "{said:?}");
        assert!(f[0].message.contains("share lines"), "{said:?}");
        assert!(f[1].message.contains("1 of the 10 lines 0-9"), "{said:?}");
        assert_eq!((f[0].check.as_ref(), f[0].at.as_str()), ("talk-range", "moonglow.toml [talk]"));
        t.entries.push(mg_tlk::TlkEntry::text("line 9"));
        let full = talk_range(&m, Some(&t), &Skip::default());
        assert!(
            full.iter().any(|f| f.severity == Severity::Error && f.message.contains("all used"))
        );
        m.project_file.user.builder_code = Some("zz".into());
        let other = talk_range(&m, Some(&t), &Skip::default());
        assert!(other.iter().any(|f| f.message.contains("you (code zz) have none")), "{other:?}");
        let skip = Skip::parse(&["talk-range"]).unwrap();
        assert!(talk_range(&m, Some(&t), &skip).is_empty());
    }

    /// A palette grouping's rules not understood, a group no blueprint of
    /// its palettes falls in, and blueprints its list names that are
    /// nowhere: each said, by its check.
    #[test]
    fn palette_groupings_are_held_to_the_palettes() {
        use crate::palette::PaletteName;
        use mg_gff::{Gff, Struct, Value};
        let itp = |entries: &[(&str, &str)]| {
            let mut g = Gff::new(*b"ITP ");
            let mut category = Struct::new(1);
            category.set("NAME", Value::String(b"Tavern".to_vec()));
            category.set("ID", Value::Byte(1));
            let leaves = entries
                .iter()
                .map(|(r, n)| {
                    let mut leaf = Struct::new(0);
                    leaf.set("RESREF", Value::resref(ResRef::from_str(r).unwrap()));
                    leaf.set("NAME", Value::String(n.as_bytes().to_vec()));
                    leaf
                })
                .collect();
            category.set("LIST", Value::List(leaves));
            g.root.set("MAIN", Value::List(vec![category]));
            g.to_bytes().unwrap()
        };
        let mut game = MemContainer::new();
        game.insert(key("placeablepalstd", ResType::ITP), itp(&[("dd_tbl", "Tavern: Table")]));
        game.insert(key("dd_tbl", ResType::UTP), vec![0]);
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, game);
        let mut m = Module::new();
        m.set(key("placeablepalcus", ResType::ITP), itp(&[("my_stool", "Stool")]));
        m.set(key("my_stool", ResType::UTP), vec![0]);
        let name = |n: &PaletteName| match n {
            PaletteName::Text(t) => String::from_utf8_lossy(t).into_owned(),
            PaletteName::StrRef(_) => String::new(),
        };
        assert!(palette_groups(&m, &rm, &name, &Skip::default()).is_empty(), "no groupings");
        m.project_file.apply(
            "moonglow.toml",
            "[palette.kind]\ntypes = [\"utp\"]\nfrom = \"k.toml\"\ncolour = 1\n\
             [[palette.kind.group]]\nname = \"Tables\"\nnames = [\"table\"]\n\
             [[palette.kind.group]]\nname = \"Seats\"\nnames = [\"stool\"]\n\
             [[palette.kind.group]]\nname = \"Beds\"\nnames = [\"bed\"]\n",
            false,
        );
        let mut warnings = Vec::new();
        crate::palette_groups::read_list(
            "k.toml",
            "\"Tables\" = [\"dd_tbl\", \"dd_gone\", \"my_stool.uti\"]\n\"Rugs\" = [\"dd_rug\"]\n",
            &mut std::sync::Arc::make_mut(&mut m.project_file.palette).list[0],
            &mut warnings,
        );
        let f = palette_groups(&m, &rm, &name, &Skip::default());
        let said: Vec<(&str, &str, &str)> =
            f.iter().map(|f| (f.check.as_ref(), f.at.as_str(), f.message.as_str())).collect();
        assert_eq!(f.len(), 5, "{said:#?}");
        assert_eq!(said[0].0, "palette-rules");
        assert!(said[0].2.contains("\"colour\" is not a key of [palette.kind]"), "{said:?}");
        // Tables (the game's) and Seats (the module's) have theirs: Beds
        // and Rugs (the list's, of a blueprint that's nowhere) none.
        assert_eq!(
            (said[1].0, said[1].1, said[1].2),
            (
                "palette-group-empty",
                "moonglow.toml [palette.kind]",
                "the group \"Beds\" of Kind is empty: no blueprint of the placeable palettes \
                 falls in it"
            )
        );
        assert!(said[2].2.starts_with("the group \"Rugs\""), "{said:?}");
        assert_eq!(
            (said[3].0, said[3].1, said[3].2),
            (
                "palette-list-missing",
                "k.toml \"Tables\"",
                "2 blueprints the list of \"Tables\" names are nowhere (the module, its haks, \
                 the game): dd_gone, my_stool.uti"
            )
        );
        assert!(said[4].2.starts_with("1 blueprint the list of \"Rugs\" names is nowhere"));
        // Each left out by its id.
        let skip = Skip::parse(&["palette-group-empty", "palette-rules"]).unwrap();
        let f = palette_groups(&m, &rm, &name, &skip);
        assert!(f.iter().all(|f| f.check == "palette-list-missing") && f.len() == 2);
        // A group of several types (TDoD's Animals: placeables and
        // creatures) is empty only where it is empty in every palette it
        // is offered in: here the placeables' have its blueprint, the
        // creatures' none.
        m.project_file.apply(
            "moonglow.toml",
            "[palette.mixed]\ntypes = [\"utp\", \"utc\"]\nfrom = \"m.toml\"\n",
            false,
        );
        crate::palette_groups::read_list(
            "m.toml",
            "\"Animals\" = [\"dd_tbl.utp\", \"nw_wolf.utc\"]\n\"Bats\" = [\"nw_bat.utc\"]\n",
            &mut std::sync::Arc::make_mut(&mut m.project_file.palette).list[1],
            &mut warnings,
        );
        let skip = Skip::parse(&["palette-list-missing", "palette-rules"]).unwrap();
        let f = palette_groups(&m, &rm, &name, &skip);
        let empty: Vec<&str> = f.iter().map(|f| f.message.as_str()).collect();
        assert!(!empty.iter().any(|e| e.contains("\"Animals\"")), "{empty:?}");
        assert!(
            empty.contains(
                &"the group \"Bats\" of Mixed is empty: no blueprint of the creature, placeable \
                  palettes falls in it"
            ),
            "{empty:?}"
        );
        // A module with no custom palette file: its own blueprints are what
        // the pane's Custom shows (TDoD's tdod_* blueprints, 2026-11-03),
        // those hidden from the palette (255) aside.
        m.remove(&key("placeablepalcus", ResType::ITP));
        let blueprint = |name: &str, palette_id: u8| {
            let mut g = Gff::new(*b"UTP ");
            let text = mg_core::LocString::from_text(
                mg_core::Language::ENGLISH,
                mg_core::Gender::Male,
                name,
            );
            g.root.set("LocName", Value::LocString(text));
            g.root.set("PaletteID", Value::Byte(palette_id));
            g.to_bytes().unwrap()
        };
        m.set(key("my_stool", ResType::UTP), blueprint("Stool", 1));
        m.set(key("my_bed", ResType::UTP), blueprint("Bed", 255));
        let f = palette_groups(&m, &rm, &name, &skip);
        let empty: Vec<&str> = f.iter().map(|f| f.message.as_str()).collect();
        assert!(!empty.iter().any(|e| e.contains("\"Seats\"")), "the module's stool: {empty:?}");
        assert!(empty.iter().any(|e| e.contains("\"Beds\"")), "a hidden bed: {empty:?}");
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
        // With some lines in LF: a warning, naming the first.
        let mixed = set.replacen("\r\n", "\n", 2);
        let rm = resman(&[("tst", ResType::SET, mixed.as_bytes())]);
        let f = examine(&Module::new(), &rm, TalkTables::default());
        let lf = f.iter().find(|f| f.check == Check::SetLineEnds.id()).expect("line ends named");
        assert_eq!(lf.severity, Severity::Warning);
        assert!(
            lf.message.starts_with("2 of its lines") && lf.message.contains("line 1)"),
            "{}",
            lf.message
        );
        // More than 256 bytes in LF after its last CRLF, or a line of more
        // than 255 characters: errors, as the server refuses them.
        let ends = |text: &str| {
            let rm = resman(&[("tst", ResType::SET, text.as_bytes())]);
            let f = examine(&Module::new(), &rm, TalkTables::default());
            let mut found: Vec<(Severity, String, String)> = f
                .into_iter()
                .filter(|f| f.check == Check::SetLineEnds.id())
                .map(|f| (f.severity, f.at, f.message))
                .collect();
            found.sort();
            found
        };
        let tail = |n: usize| format!("{set}[MGPAD]\r\n{}", "X=1\n".repeat(n));
        assert_eq!(ends(&tail(64))[0].0, Severity::Warning, "256 bytes: read");
        let refused = ends(&tail(65));
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].0, Severity::Error);
        assert!(refused[0].2.starts_with("its last 260 bytes, after its last CRLF"), "{refused:?}");
        let before = format!("{}Y=1\r\n", tail(150));
        assert_eq!(ends(&before)[0].0, Severity::Warning, "before a last CRLF line: read");
        let long = |n: usize| format!("{set}[MGPAD]\r\nX={}\r\n", "1".repeat(n - 2));
        assert!(ends(&long(255)).is_empty(), "255 characters: read");
        let refused = ends(&long(256));
        let lines = set.matches('\n').count();
        assert_eq!(
            refused,
            [(
                Severity::Error,
                format!("line {}", lines + 2),
                format!(
                    "line {} is 256 characters long: the game's server never finishes loading \
                     a module that uses it (a line has at most 255)",
                    lines + 2
                )
            )]
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
        // (`ok` is there, so that `mtr-texture` has nothing to say.)
        let rm = resman(&[
            ("my_mat", ResType::MTR, b"texture0 a_texture_name_too_long\ntexture1 ok\n"),
            ("ok", ResType::TGA, b""),
        ]);
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
        let rm = resman(&[
            ("waypoint", ResType::TWODA, table),
            ("gi_waypoint01", ResType::MDL, &model_text("gi_waypoint01")),
        ]);
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

    /// A model of ours named `name`, which reads.
    fn model_text(name: &str) -> Vec<u8> {
        format!(
            "newmodel {name}\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
             endmodelgeom {name}\ndonemodel {name}\n"
        )
        .into_bytes()
    }

    /// Past an appearance's row, the models an object's parts name: an
    /// item's piece that isn't there is a warning, placed or not (the rest
    /// is drawn), as is a body's part; a body with no skeleton is as an
    /// appearance with no model. And a model of a hak's that is there and
    /// can't be read is said to be, with why: an error placed, as the
    /// game's area view would find it.
    #[test]
    fn an_object_s_parts_and_unreadable_models() {
        let placeables = b"2DA V2.0\n\n   Label ModelName\n0  Broken plc_broken\n";
        let baseitems = b"2DA V2.0\n\n   label ItemClass ModelType DefaultModel\n\
                          0 shortsword WSwSs 2 ****\n";
        let appearance = b"2DA V2.0\n\n   LABEL RACE MODELTYPE\n0 Dwarf D P\n1 Gnome G P\n";
        let rm = resman(&[
            ("placeables", ResType::TWODA, placeables),
            ("baseitems", ResType::TWODA, baseitems),
            ("appearance", ResType::TWODA, appearance),
            ("plc_broken", ResType::MDL, b"not a model"),
            ("wswss_b_001", ResType::MDL, &model_text("wswss_b_001")),
            ("wswss_m_001", ResType::MDL, &model_text("wswss_m_001")),
            ("pmd0", ResType::MDL, &model_text("pmd0")),
            ("pmd0_head001", ResType::MDL, b"truncated"),
        ]);
        let mut m = Module::new();
        let mut sword = Gff::new(*b"UTI ");
        for (field, n) in [("BaseItem", 0), ("ModelPart1", 1), ("ModelPart2", 1), ("ModelPart3", 2)]
        {
            sword.root.set(field, Value::Int(n));
        }
        m.set_gff(key("sword", ResType::UTI), &sword).unwrap();
        let mut dwarf = Gff::new(*b"UTC ");
        dwarf.root.set("Appearance_Type", Value::Word(0));
        dwarf.root.set("Appearance_Head", Value::Byte(1));
        dwarf.root.set("BodyPart_LShin", Value::Byte(3));
        m.set_gff(key("dwarf", ResType::UTC), &dwarf).unwrap();
        let mut gnome = Gff::new(*b"UTC ");
        gnome.root.set("Appearance_Type", Value::Word(1));
        m.set_gff(key("gnome", ResType::UTC), &gnome).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut box_ = Struct::new(9);
        box_.set("Appearance", Value::Dword(0));
        box_.set("Tag", Value::String(b"BOX".to_vec()));
        let mut placed = sword.root.clone();
        placed.id = 0;
        placed.set("Tag", Value::String(b"SWORD".to_vec()));
        git.root.set("Placeable List", Value::List(vec![box_]));
        git.root.set("List", Value::List(vec![placed]));
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &rm, TalkTables::default());
        let mut rows: Vec<(Severity, &str, &str)> = f
            .iter()
            .filter(|f| f.check == "object-row")
            .map(|f| (f.severity, f.at.as_str(), f.message.as_str()))
            .collect();
        rows.sort();
        let said = |at: &str| rows.iter().find(|r| r.1 == at).map(|r| (r.0, r.2));
        let (severity, message) = said("area › placeable BOX › Appearance").expect("the box");
        assert_eq!(severity, Severity::Error);
        assert!(
            message.starts_with(
                "Unreadable model: plc_broken (placeables.2da row 0, ModelName) is there and \
                 can't be read: "
            ),
            "{message}"
        );
        let missing = "No model: wswss_t_002 (its ModelPart3) isn't in the module, its haks or \
                       the game";
        assert_eq!(said("sword.uti › ModelPart3"), Some((Severity::Warning, missing)), "{rows:?}");
        let placed = rows.iter().find(|r| r.1.starts_with("area ›") && r.1.ends_with("ModelPart3"));
        assert_eq!(placed.map(|r| (r.0, r.2)), Some((Severity::Warning, missing)), "{rows:?}");
        let (_, shin) = said("dwarf.utc › BodyPart_LShin").expect("the shin");
        assert!(shin.starts_with("No model: pmd0_shinl003 (its BodyPart_LShin)"), "{shin}");
        let (_, head) = said("dwarf.utc › Appearance_Head").expect("the head");
        assert!(head.starts_with("Unreadable model: pmd0_head001 (its Appearance_Head)"), "{head}");
        let (_, skeleton) = said("gnome.utc › Appearance_Type").expect("the gnome");
        assert!(skeleton.starts_with("No model: the skeleton pmg0 (appearance.2da row 1"));
        assert_eq!(rows.len(), 6, "{rows:?}");
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

    /// A static placeable with a tint's local is warned of (the client
    /// draws it as made); a dynamic one, or a static one without, is not;
    /// a static blueprint's too.
    #[test]
    fn static_placeables_with_a_tint_are_warned_of() {
        let placed = |tag: &str, is_static: u8, local: Option<&str>| {
            let mut s = Struct::new(9);
            s.set("Tag", Value::String(tag.as_bytes().to_vec()));
            s.set("Static", Value::Byte(is_static));
            let vars = local.map(|name| {
                let mut v = Struct::new(0);
                v.set("Name", Value::String(name.as_bytes().to_vec()));
                v.set("Type", Value::Dword(3));
                v.set("Value", Value::String(b"#2a4d8f".to_vec()));
                v
            });
            s.set("VarTable", Value::List(vars.into_iter().collect()));
            s
        };
        let mut git = Gff::new(*b"GIT ");
        git.root.set(
            "Placeable List",
            Value::List(vec![
                placed("RUG", 1, Some("tdod_tint")),
                placed("BED", 1, Some("tdod_tint_dd_ctn_bedb")),
                placed("DYNAMIC", 0, Some("tdod_tint")),
                placed("PLAIN", 1, Some("x_note")),
            ]),
        );
        let mut m = Module::new();
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let mut utp = Gff::new(*b"UTP ");
        utp.root = placed("CARPET", 1, Some("tdod_tint"));
        m.set_gff(key("carpet", ResType::UTP), &utp).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<_> = f.iter().filter(|f| f.check == "static-tint").collect();
        let at: Vec<&str> = found.iter().map(|f| f.at.as_str()).collect();
        assert_eq!(at.len(), 3, "{at:?}");
        assert!(at[0].contains("RUG") && at[1].contains("BED"), "{at:?}");
        assert!(found[0].message.contains("untick Static"), "{found:?}");
        assert_eq!(Check::StaticTint.group(), Group::Objects);
    }

    /// A `tdodTint` stored for a material a tint's local also tints is
    /// warned of; one for another material, or without a local, is not.
    #[test]
    fn a_tint_stored_and_set_by_a_local_is_warned_of() {
        let text = |t: &str| Value::String(t.as_bytes().to_vec());
        let placed = |tag: &str, local: Option<&str>, stored: &[(&str, &str)]| {
            let mut s = Struct::new(9);
            s.set("Tag", text(tag));
            if let Some(name) = local {
                let mut v = Struct::new(0);
                v.set("Name", text(name));
                v.set("Type", Value::Dword(3));
                v.set("Value", text("#2a4d8f"));
                s.set("VarTable", Value::List(vec![v]));
            }
            let entries = stored
                .iter()
                .map(|(material, param)| {
                    let mut e = Struct::new(0);
                    e.set("Material", text(material));
                    e.set("Param", text(param));
                    e.set("Type", Value::Byte(2));
                    e
                })
                .collect();
            let mut m = Struct::new(7);
            m.set("ShaderParams", Value::List(entries));
            s.set("Material", Value::Struct(m));
            s
        };
        let mut git = Gff::new(*b"GIT ");
        git.root.set(
            "Placeable List",
            Value::List(vec![
                placed("RUG", Some("tdod_tint"), &[("dd_gca_carpeta", "tdodTint")]),
                placed("BED", Some("tdod_tint_dd_ctn_bedb"), &[("dd_ctn_bedc", "tdodTint")]),
                placed("CLOTH", None, &[("dd_cloth", "tdodTint")]),
                placed("LAMP", Some("tdod_tint"), &[("dd_lamp", "fGlow")]),
            ]),
        );
        let mut m = Module::new();
        m.set_gff(key("area", ResType::GIT), &git).unwrap();
        let f = examine(&m, &resman(&[]), TalkTables::default());
        let found: Vec<_> = f.iter().filter(|f| f.check == "tint-stored").collect();
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].at.contains("RUG"), "{found:?}");
        assert!(found[0].message.contains("dd_gca_carpeta"), "{found:?}");
        assert_eq!(Check::TintStored.group(), Group::Objects);
    }

    /// A world of forty areas with something for every check that reads
    /// the module's areas, objects and blueprints (a row that isn't there,
    /// the PC faction, static placeables with a transform and with a tint,
    /// a tint stored, a row past 65535, an ARE's variables, tiles that
    /// don't fit, tags to be unique, described variables and those the
    /// scripts read), kept as a folder with its project's files.
    fn a_world(dir: &std::path::Path) -> (Module, ResMan) {
        let s = |t: &str| Value::String(t.as_bytes().to_vec());
        let var = |name: &str, value: Value| {
            let t = match value {
                Value::Int(_) => 1,
                Value::Float(_) => 2,
                _ => 3,
            };
            let mut v = Struct::new(0);
            v.set("Name", s(name));
            v.set("Type", Value::Dword(t));
            v.set("Value", value);
            v
        };
        let object = |id: u32, tag: &str, vars: Vec<Struct>| {
            let mut o = Struct::new(id);
            o.set("Tag", s(tag));
            if !vars.is_empty() {
                o.set("VarTable", Value::List(vars));
            }
            o
        };
        let scaled = |o: &mut Struct| {
            let mut entry = Struct::new(6);
            entry.set("Scope", Value::Int(0));
            for a in ["ScaleX", "ScaleY", "ScaleZ"] {
                let mut axis = Struct::new(0);
                axis.set("ValueTo", Value::Float(0.5));
                entry.set(a, Value::Struct(axis));
            }
            o.set("VisTransformList", Value::List(vec![entry]));
        };
        let stored = |o: &mut Struct| {
            let mut e = Struct::new(0);
            e.set("Material", s("dd_gca_carpeta"));
            e.set("Param", s("tdodTint"));
            e.set("Type", Value::Byte(2));
            let mut m = Struct::new(7);
            m.set("ShaderParams", Value::List(vec![e]));
            o.set("Material", Value::Struct(m));
        };
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.set("VarTable", Value::List(vec![var("tdod_lamplit", Value::Int(1))]));
        m.set_gff(key("module", ResType::IFO), &ifo).unwrap();
        for i in 0..40u32 {
            let name = format!("area{i:02}");
            let mut are = Gff::new(*b"ARE ");
            are.root.set("Tag", s(&format!("AREA_{i}")));
            are.root.set("Tileset", Value::ResRef(b"tst03".to_vec()));
            are.root.set("Width", Value::Int(2));
            are.root.set("Height", Value::Int(2));
            // (As `tiles_that_do_not_fit_their_neighbors_are_found` has
            // them: tiles that don't fit, tiles that do.)
            let tiles: &[(i32, i32, i32)] = match i % 3 {
                0 => &[(1, 2, 0), (0, 0, 0), (2, 0, 0), (0, 0, 1)],
                1 => &[(1, 2, 0), (1, 3, 0), (1, 1, 0), (1, 0, 0)],
                _ => &[(0, 0, 0), (7, 0, 0), (0, 0, 0), (0, 0, 0)],
            };
            let list = (tiles.iter())
                .map(|&(id, o, h)| {
                    let mut t = Struct::new(1);
                    t.set("Tile_ID", Value::Int(id));
                    t.set("Tile_Orientation", Value::Int(o));
                    t.set("Tile_Height", Value::Int(h));
                    t
                })
                .collect();
            are.root.set("Tile_List", Value::List(list));
            if i % 5 == 0 {
                are.root.set("VarTable", Value::List(vec![var("old_one", Value::Int(i as i32))]));
            }
            m.set_gff(key(&name, ResType::ARE), &are).unwrap();

            let mut git = Gff::new(*b"GIT ");
            git.root.set("VarTable", Value::List(vec![var("tdod_lamplit", Value::Int(1))]));
            let mut guard = object(4, "GUARD", vec![var("tdod_kind", s("monster"))]);
            if i % 2 == 0 {
                guard.set("FactionID", Value::Word(0));
            } else {
                guard.set("FactionID", Value::Word(2));
            }
            let item = object(0, "", vec![var("tdod_kind", s("beast"))]);
            guard.set("ItemList", Value::List(vec![item]));
            git.root.set("Creature List", Value::List(vec![guard]));
            let mut bench = object(
                9,
                "BENCH",
                vec![
                    var("tdod_workbench", s("Forge")),
                    var("tdod_lit", s("1")),
                    var("TDOD_light", Value::Int(1)),
                    var("tdod_harvest_dc", Value::Int(99)),
                    var("tdod_craft2", Value::Int(7)),
                    var("tdod_trodden", Value::Int(1)),
                    var("tdod_note", s("read as an int")),
                    var("tdod_rental", s("inn")),
                    var("tdod_missing", s("anything")),
                ],
            );
            bench.set("Appearance", Value::Dword(0));
            bench.set("Faction", Value::Dword(1));
            let mut rug = object(9, "RUG", vec![var("tdod_tint", s("#2a4d8f"))]);
            rug.set("Static", Value::Byte(u8::from(i % 4 == 0)));
            rug.set("Appearance", Value::Dword(0));
            rug.set("Faction", Value::Dword(1));
            stored(&mut rug);
            let mut mirror = object(9, "MIRROR", Vec::new());
            mirror.set("Static", Value::Byte(u8::from(i % 3 == 0)));
            mirror.set("Appearance", Value::Dword(if i % 7 == 0 { 70_000 + i } else { 7 }));
            mirror.set("Faction", Value::Dword(1));
            scaled(&mut mirror);
            git.root.set("Placeable List", Value::List(vec![bench, rug, mirror]));
            let waypoints = vec![
                object(5, &format!("TDOD_START_{}", i % 4), Vec::new()),
                object(5, "SHOP_STORE_A", Vec::new()),
                object(5, if i % 6 == 0 { "SHOP_STORE_A" } else { "LAMP" }, Vec::new()),
            ];
            git.root.set("WaypointList", Value::List(waypoints));
            m.set_gff(key(&name, ResType::GIT), &git).unwrap();

            // A blueprint an area: of no faction, its row not there.
            let mut utp = Gff::new(*b"UTP ");
            utp.root = object(u32::MAX, "CHAIR", vec![var("tdod_tint", s("#ffffff"))]);
            utp.root.set("Appearance", Value::Dword(if i % 2 == 0 { 0 } else { 9 }));
            utp.root.set("Static", Value::Byte(u8::from(i % 8 == 0)));
            m.set_gff(key(&format!("chair{i:02}"), ResType::UTP), &utp).unwrap();
        }
        m.set(
            key("tdod_kind_inc", ResType::NSS),
            b"const string TDOD_KIND_BEAST = \"beast\";\nconst string TDOD_KIND_UNDEAD = \"undead\";\n"
                .to_vec(),
        );
        m.set(
            key("tdod_reads", ResType::NSS),
            b"void main()\n{\n    int n = GetLocalInt(OBJECT_SELF, \"tdod_note\");\n}\n".to_vec(),
        );
        m.save_as(&crate::ModuleLocation::Folder(dir.to_path_buf())).unwrap();
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
             [tdod_rental]\ntype = \"string\"\nrequires = [\"tdod_rental_door\"]\n\n\
             [tdod_rental_door]\ntype = \"string\"\n\n\
             [tdod_missing]\ntype = \"string\"\nconstants = \"tdod_nowhere_inc\"\nprefix = \"X_\"\n\n\
             [tdod_tint]\ntype = \"string\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("moonglow.toml"),
            "[tags]\nunique = [\"TDOD_START_*\"]\nunique_in_area = [\"*_STORE_*\"]\n",
        )
        .unwrap();
        let set = "[GENERAL]\nName=TST03\nTransition=5\n\
[TERRAIN TYPES]\nCount=2\n[TERRAIN0]\nName=Grass\n[TERRAIN1]\nName=Water\n\
[CROSSER TYPES]\nCount=1\n[CROSSER0]\nName=Road\n\
[TILES]\nCount=3\n\
[TILE0]\nModel=tst03_a\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\n\
[TILE1]\nModel=tst03_b\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Water\nBottomRight=Grass\n\
[TILE2]\nModel=tst03_c\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\n\
Top=Road\nBottom=Road\n[GROUPS]\nCount=0\n";
        let rm = resman(&[
            ("tst03", ResType::SET, set.as_bytes()),
            ("crafts", ResType::TWODA, b"2DA V2.0\n\n   Label\n0  Smithing\n1  Tailoring\n"),
        ]);
        let m = Module::open(dir).unwrap();
        assert_eq!(m.project_file.warnings, Vec::<String>::new());
        (m, rm)
    }

    /// The module's areas, objects and blueprints are each read once, on
    /// every core, and handed to every check that reads their type (the
    /// review's V6: each check read every one of them again, one after
    /// another). The findings are those the checks made one after another,
    /// in their order: held to what that code found of this world (their
    /// number and a hash of every word of them, taken before the change),
    /// and the same on one thread as on many, each time.
    #[test]
    fn findings_of_resources_read_once_are_as_one_check_after_another_made_them() {
        let dir = mg_testkit::scratch_dir("doctor-one-pass");
        let (m, rm) = a_world(&dir);
        let tlk = TalkTables::default();
        let f = examine(&m, &rm, tlk);
        // (Every check of the pass found something.)
        for check in [
            "object-row",
            "pc-faction",
            "static-transform",
            "static-tint",
            "tint-stored",
            "appearance-range",
            "area-variables",
            "tile-seam",
            "tag-unique",
            "var-type",
            "var-value",
            "var-name",
            "var-object",
            "var-runtime",
            "var-requires",
            "var-script",
        ] {
            assert!(f.iter().any(|x| x.check == check), "{check}");
        }
        // (FNV-1a over every finding, in order.)
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for x in &f {
            let line = format!(
                "{:?}|{}|{}|{}|{}|{}\n",
                x.severity, x.check, x.source, x.resource, x.at, x.message
            );
            for b in line.bytes() {
                hash = (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
        }
        assert_eq!((f.len(), hash), GOLDEN, "the findings changed");
        // A list that can't be read is said once, at the first variable of
        // it in the first resource.
        let unlisted: Vec<&Finding> =
            f.iter().filter(|x| x.message.contains("can't be listed")).collect();
        assert_eq!(unlisted.len(), 1, "{unlisted:#?}");
        assert_eq!(unlisted[0].resource, key("area00", ResType::GIT));
        // The same on one thread, and each time.
        let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        assert_eq!(one.install(|| examine(&m, &rm, tlk)), f);
        for _ in 0..3 {
            assert_eq!(examine(&m, &rm, tlk), f);
        }
    }

    /// What `findings_of_resources_read_once_…` found before the checks
    /// shared one read of each resource: how many findings, and their hash.
    const GOLDEN: (usize, u64) = (716, 10_594_041_698_474_318_005);
}
