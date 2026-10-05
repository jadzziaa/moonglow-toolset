//! New modules and areas, made as Aurora's Module and Area wizards make them
//! (captured under Wine: `tools/aurora/capture_new_areas.py`).
//!
//! A new module has Aurora's `module.ifo` defaults (tag `MODULE`, the
//! standard x2/x3 module event scripts, a random `Mod_ID`), the five default
//! factions, and an empty custom palette per blueprint type. A new area takes
//! its properties from `areag.ini` and `environment.2da`, and its tiles from
//! the tileset: `Default` terrain with a patch of `Floor` at the centre.

use mg_core::{Gender, Language, LocString, ResRef, ResType, StrRef};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_schema::{ExoString, StructExt, are, fac, gic, git, ifo};
use mg_set::{Ini, Tileset};
use mg_tiles::{TileIndex, TilesError};
use thiserror::Error;

use crate::{Module, ModuleError};

#[derive(Debug, Error)]
pub enum NewError {
    #[error(transparent)]
    Module(#[from] ModuleError),
    #[error("{0}: {1}")]
    Resource(String, String),
    #[error("{0}")]
    Invalid(String),
    #[error("{tileset}: {source}")]
    Tiles { tileset: String, source: TilesError },
}

fn res_error(name: &str, e: impl std::fmt::Display) -> NewError {
    NewError::Resource(name.to_string(), e.to_string())
}

/// Blueprint types with a palette, as in `<type>pal.itp` and `<type>palcus.itp`.
pub const PALETTE_TYPES: [&str; 9] =
    ["creature", "door", "encounter", "item", "placeable", "sound", "store", "trigger", "waypoint"];

/// The module event scripts Aurora sets on a new module.
const MODULE_EVENTS: [(&str, &str); 22] = [
    ("Mod_OnHeartbeat", ""),
    ("Mod_OnModLoad", "x2_mod_def_load"),
    ("Mod_OnModStart", ""),
    ("Mod_OnClientEntr", "x3_mod_def_enter"),
    ("Mod_OnClientLeav", ""),
    ("Mod_OnActvtItem", "x2_mod_def_act"),
    ("Mod_OnAcquirItem", "x2_mod_def_aqu"),
    ("Mod_OnUsrDefined", ""),
    ("Mod_OnUnAqreItem", "x2_mod_def_unaqu"),
    ("Mod_OnPlrDeath", "nw_o0_death"),
    ("Mod_OnPlrDying", "nw_o0_dying"),
    ("Mod_OnPlrEqItm", "x2_mod_def_equ"),
    ("Mod_OnPlrLvlUp", ""),
    ("Mod_OnSpawnBtnDn", "nw_o0_respawn"),
    ("Mod_OnPlrRest", "x2_mod_def_rest"),
    ("Mod_OnPlrUnEqItm", "x2_mod_def_unequ"),
    ("Mod_OnCutsnAbort", ""),
    ("Mod_OnPlrChat", ""),
    ("Mod_OnPlrTarget", ""),
    ("Mod_OnPlrGuiEvt", ""),
    ("Mod_OnPlrTileAct", ""),
    ("Mod_OnNuiEvent", ""),
];

fn resref(s: &str) -> ResRef {
    ResRef::from_str(s).expect("valid resref")
}

fn english(game: &GameData, text: &str) -> Result<LocString, NewError> {
    let bytes = game.language.codepage().encode(text).ok_or_else(|| {
        NewError::Invalid(format!("{text:?} cannot be written in the game's codepage"))
    })?;
    Ok(LocString::from_text(Language::ENGLISH, Gender::Male, bytes.into_owned()))
}

/// A new module named `name`, without areas (add one with [`add_area`]
/// before playing it).
pub fn new_module(
    game: &GameData,
    name: &str,
    rng: &mut fastrand::Rng,
) -> Result<Module, NewError> {
    let mut m = Module::new();
    m.set_info(&module_info(game, name, rng)?)?;
    m.set_gff(ResKey::parse("repute", ResType::FAC).expect("valid"), &default_factions())
        .map_err(|e| res_error("repute.fac", e))?;
    for kind in PALETTE_TYPES {
        let name = format!("{kind}pal");
        let data = game.resman.get_named(&name, ResType::ITP).map_err(|e| res_error(&name, e))?;
        let skeleton = Gff::read(&data).map_err(|e| res_error(&name, e))?;
        let key = ResKey::parse(&format!("{kind}palcus"), ResType::ITP).expect("valid");
        m.set_gff(key, &custom_palette(&skeleton, |s| game.string(StrRef(s)).unwrap_or_default()))
            .map_err(|e| res_error(&name, e))?;
    }
    Ok(m)
}

fn module_info(game: &GameData, name: &str, rng: &mut fastrand::Rng) -> Result<Gff, NewError> {
    let mut info = Gff::new(*b"IFO ");
    let r = &mut info.root;
    let mut id = [0u8; 16];
    rng.fill(&mut id);
    r.set("Mod_ID", Value::Void(id.to_vec()));
    r.write(&ifo::MOD_MIN_GAME_VER, ExoString::from("1.89"));
    r.write(&ifo::MOD_CREATOR_ID, 2);
    r.write(&ifo::MOD_VERSION, 3);
    r.write(&ifo::EXPANSION_PACK, 3);
    r.write(&ifo::MOD_NAME, english(game, name)?);
    r.write(&ifo::MOD_TAG, ExoString::from("MODULE"));
    r.write(&ifo::MOD_DESCRIPTION, LocString::default());
    r.write(&ifo::MOD_IS_SAVE_GAME, 0);
    r.write(&ifo::MOD_CUSTOM_TLK, ExoString::from(""));
    r.write(&ifo::MOD_ENTRY_AREA, ResRef::EMPTY);
    for f in [
        &ifo::MOD_ENTRY_X,
        &ifo::MOD_ENTRY_Y,
        &ifo::MOD_ENTRY_Z,
        &ifo::MOD_ENTRY_DIR_X,
        &ifo::MOD_ENTRY_DIR_Y,
    ] {
        r.write(f, 0.0);
    }
    r.items_mut(&ifo::MOD_EXPAN_LIST);
    r.write(&ifo::MOD_DAWN_HOUR, 6);
    r.write(&ifo::MOD_DUSK_HOUR, 18);
    r.write(&ifo::MOD_MIN_PER_HOUR, 2);
    r.write(&ifo::MOD_START_MONTH, 6);
    r.write(&ifo::MOD_START_DAY, 1);
    r.write(&ifo::MOD_START_HOUR, 13);
    r.write(&ifo::MOD_START_YEAR, 1372);
    r.write(&ifo::MOD_XP_SCALE, 10);
    for (label, script) in MODULE_EVENTS {
        r.set(label, Value::resref(resref(script)));
    }
    r.write(&ifo::MOD_START_MOVIE, ResRef::EMPTY);
    r.write(&ifo::MOD_DEFAULT_BIC, ResRef::EMPTY);
    r.write(&ifo::MOD_UUID, ExoString::from(""));
    r.write(&ifo::MOD_PARTY_CONTROL, 0);
    r.items_mut(&ifo::MOD_CUT_SCENE_LIST);
    r.items_mut(&ifo::MOD_G_VAR_LIST);
    r.items_mut(&ifo::MOD_AREA_LIST);
    r.items_mut(&ifo::MOD_HAK_LIST);
    Ok(info)
}

/// The factions of a new module (PC, Hostile, Commoner, Merchant, Defender)
/// and how each regards the others. How factions regard PCs is not stored.
pub fn default_factions() -> Gff {
    const NAMES: [&str; 5] = ["PC", "Hostile", "Commoner", "Merchant", "Defender"];
    // REPUTATION[t][p - 1]: how faction p (1..=4) regards faction t; how
    // the PC faction regards others is not stored.
    const REPUTATION: [[u32; 4]; 5] =
        [[0, 50, 50, 50], [100, 0, 0, 0], [0, 100, 50, 100], [0, 50, 100, 100], [0, 50, 100, 100]];
    let mut g = Gff::new(*b"FAC ");
    let factions = g.root.items_mut(&fac::FACTION_LIST);
    for (i, name) in NAMES.iter().enumerate() {
        let mut s = Struct::new(i as u32);
        s.write(&fac::faction_list::FACTION_PARENT_ID, u32::MAX);
        s.write(&fac::faction_list::FACTION_NAME, ExoString::from(*name));
        s.write(&fac::faction_list::FACTION_GLOBAL, 1);
        factions.push(s);
    }
    let reps = g.root.items_mut(&fac::REP_LIST);
    for (a, row) in REPUTATION.iter().enumerate() {
        for (b, rep) in row.iter().enumerate() {
            let mut s = Struct::new(reps.len() as u32);
            // FactionID1 is regarded by FactionID2.
            s.write(&fac::rep_list::FACTION_ID1, a as u32);
            s.write(&fac::rep_list::FACTION_ID2, b as u32 + 1);
            s.write(&fac::rep_list::FACTION_REP, *rep);
            reps.push(s);
        }
    }
    g
}

fn int(v: Option<&Value>) -> Option<i64> {
    Some(match v? {
        Value::Byte(x) => i64::from(*x),
        Value::Char(x) => i64::from(*x),
        Value::Word(x) => i64::from(*x),
        Value::Short(x) => i64::from(*x),
        Value::Dword(x) => i64::from(*x),
        Value::Int(x) => i64::from(*x),
        _ => return None,
    })
}

/// Sort key of Windows' default ("word sort") string comparison, which
/// Aurora uses for its lists: case-insensitive, hyphens and apostrophes
/// ignored except to break ties.
pub fn word_sort_key(s: &str) -> (String, String) {
    (
        s.chars().filter(|c| !matches!(c, '-' | '\'')).flat_map(char::to_lowercase).collect(),
        s.to_string(),
    )
}

/// A module's empty custom palette for a blueprint type, from the game's
/// palette skeleton (`<type>pal.itp`): its categories without the
/// "assign to new category" placeholder (`TYPE` 0) and the engine's own
/// categories (`TYPE` 1: familiars, summoned creatures), without the build
/// fields (`TYPE`, `DELETE_ME`, `NEXT_USEABLE_ID`, `RESTYPE`), each level
/// sorted by name as Aurora lists it, struct ids 0. `name` looks up a StrRef.
pub fn custom_palette(skeleton: &Gff, name: impl Fn(u32) -> String) -> Gff {
    custom_palette_in(skeleton, name, true)
}

/// [`custom_palette`]; with `by_name` false, the categories in the
/// skeleton's own order: a module's own skeleton is as its builder
/// arranged it.
pub fn custom_palette_in(skeleton: &Gff, name: impl Fn(u32) -> String, by_name: bool) -> Gff {
    fn nodes(list: &[Struct], name: &dyn Fn(u32) -> String, by_name: bool) -> Vec<Struct> {
        let mut out: Vec<((String, String), Struct)> = list
            .iter()
            .filter(|n| !matches!(int(n.get("TYPE")), Some(0 | 1)))
            .map(|n| {
                // Aurora writes STRREF, ID, LIST in that order.
                let mut s = Struct::new(0);
                for label in ["STRREF", "ID"] {
                    if let Some(f) = n.field(label) {
                        s.fields.push(f.clone());
                    }
                }
                if let Some(Value::List(children)) = n.get("LIST") {
                    s.set("LIST", Value::List(nodes(children, name, by_name)));
                }
                for f in &n.fields {
                    if !matches!(
                        f.label.as_bytes(),
                        b"STRREF" | b"ID" | b"LIST" | b"TYPE" | b"DELETE_ME"
                    ) {
                        s.fields.push(f.clone());
                    }
                }
                // By its name: written out (a module's own category), or
                // the talk table's.
                let strref = int(n.get("STRREF")).unwrap_or(-1);
                let shown = match n.string("NAME") {
                    Some(text) => String::from_utf8_lossy(text).into_owned(),
                    None => u32::try_from(strref).map(name).unwrap_or_default(),
                };
                (word_sort_key(&shown), s)
            })
            .collect();
        if by_name {
            out.sort_by(|a, b| a.0.cmp(&b.0));
        }
        out.into_iter().map(|(_, s)| s).collect()
    }
    let mut g = Gff::new(*b"ITP ");
    let main = nodes(skeleton.root.list("MAIN").unwrap_or(&[]), &name, by_name);
    g.root.set("MAIN", Value::List(main));
    g
}

/// What the Area Wizard asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaSpec {
    pub name: String,
    pub tileset: ResRef,
    /// Tiles across (west to east), 2 to 32 in Aurora.
    pub width: u32,
    /// Tiles down (south to north), 2 to 32 in Aurora.
    pub height: u32,
}

/// Aurora's area sizes: Tiny, Small, Medium, Large.
pub const AREA_SIZES: [(&str, u32); 4] = [("Tiny", 2), ("Small", 4), ("Medium", 8), ("Large", 16)];
/// Smallest and largest width or height the Area Wizard accepts.
pub const AREA_SIZE_RANGE: std::ops::RangeInclusive<u32> = 2..=32;

/// A tileset as the Area Wizard lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilesetChoice {
    pub resref: ResRef,
    pub name: String,
}

/// The tilesets the game (with its haks) offers, sorted by name as Aurora
/// lists them. The name is the SET's `DisplayName` string, or its
/// `UnlocalizedName`, or the resref.
pub fn tilesets(game: &GameData) -> Vec<TilesetChoice> {
    let mut out: Vec<TilesetChoice> = game
        .resman
        .list(ResType::SET)
        .into_iter()
        .filter_map(|resref| {
            let data = game.resman.get(&ResKey::new(resref, ResType::SET)).ok()?;
            let set = Tileset::parse(&data, game.language.codepage()).ok()?;
            let name = game
                .string(set.general.display_name)
                .filter(|s| !s.is_empty() && !set.general.display_name.is_none())
                .or(set.general.unlocalized_name)
                .unwrap_or_else(|| resref.to_string());
            Some(TilesetChoice { resref, name })
        })
        .collect();
    out.sort_by_cached_key(|t| word_sort_key(&t.name));
    out
}

/// A resref from a name, as Aurora makes one ("Area 001" → `area001`),
/// made unique with a number if `taken` says it is in use.
pub fn resref_for(name: &str, taken: impl Fn(&ResRef) -> bool) -> ResRef {
    let mut base: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if base.is_empty() {
        base = "area".to_string();
    }
    base.truncate(16);
    let candidate = resref(&base);
    if !taken(&candidate) {
        return candidate;
    }
    (1..)
        .map(|n| {
            let suffix = n.to_string();
            let mut b = base.clone();
            b.truncate(16 - suffix.len());
            resref(&format!("{b}{suffix}"))
        })
        .find(|r| !taken(r))
        .expect("a free resref")
}

/// A tag from a name, as Aurora makes one ("Area 001" → `Area001`).
pub fn tag_for(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(32).collect()
}

/// Adds a new area to a module, as Aurora's Area Wizard makes it, and
/// returns its resref. The module's first area becomes its starting area,
/// entered at the centre facing north.
/// A lighting scheme (an environment.2da row, Aurora's Area Properties ›
/// Visual): the area's sun, moon, fog, shadows, day and night and weather,
/// and colours its tiles' lights are picked from.
#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub row: usize,
    fields: Vec<(&'static str, Value)>,
    /// Main light 1, main light 2 and source light colours to pick from.
    lights: [[u8; 4]; 3],
}

impl Scheme {
    pub fn read(game: &GameData, row: usize) -> Result<Scheme, NewError> {
        let env = game.table("environment").map_err(|e| res_error("environment.2da", e))?;
        let int = |col: &str| env.get_int(row, col).unwrap_or(0);
        let color = |prefix: &str| {
            let c = |part: &str| (int(&format!("{prefix}_{part}")) as u32) & 0xff;
            Value::Dword(c("RED") | c("GREEN") << 8 | c("BLUE") << 16)
        };
        let byte = |col: &str| Value::Byte(int(col) as u8);
        let day_night = env.get(row, "DAYNIGHT").unwrap_or("cycle").to_ascii_lowercase();
        let shadow_alpha = env.get_float(row, "SHADOW_ALPHA").unwrap_or(0.0);
        let fields = vec![
            ("MoonAmbientColor", color("DARK_AMB")),
            ("MoonDiffuseColor", color("DARK_DIFF")),
            ("MoonFogAmount", byte("DARK_FOG")),
            ("MoonFogColor", color("DARK_FOG")),
            ("MoonShadows", byte("DARK_SHADOWS")),
            ("SunAmbientColor", color("LIGHT_AMB")),
            ("SunDiffuseColor", color("LIGHT_DIFF")),
            ("SunFogAmount", byte("LIGHT_FOG")),
            ("SunFogColor", color("LIGHT_FOG")),
            ("SunShadows", byte("LIGHT_SHADOWS")),
            ("IsNight", Value::Byte(u8::from(day_night == "night"))),
            ("LightingScheme", Value::Byte(row as u8)),
            ("ShadowOpacity", Value::Byte((shadow_alpha * 100.0).round() as u8)),
            ("DayNightCycle", Value::Byte(u8::from(day_night == "cycle"))),
            ("ChanceRain", Value::Int(int("RAIN"))),
            ("ChanceSnow", Value::Int(int("SNOW"))),
            ("ChanceLightning", Value::Int(int("LIGHTNING"))),
            ("WindPower", Value::Int(int("WIND"))),
        ];
        let lights = ["MAIN1_COLOR", "MAIN2_COLOR", "SECONDARY_COLOR"]
            .map(|prefix| [1, 2, 3, 4].map(|n| int(&format!("{prefix}{n}")) as u8));
        Ok(Scheme { row, fields, lights })
    }

    /// The ARE fields the scheme sets, in the order a new area has them.
    pub fn fields(&self) -> Vec<(&'static str, Value)> {
        self.fields.clone()
    }

    /// A tile's lights (main 1, main 2 and source), each one of the
    /// scheme's four colours at random.
    pub fn tile_lights(&self, rng: &mut fastrand::Rng) -> [u8; 3] {
        self.lights.map(|colors| colors[rng.u32(1..=4) as usize - 1])
    }
}

pub fn add_area(
    module: &mut Module,
    game: &GameData,
    spec: &AreaSpec,
    rng: &mut fastrand::Rng,
) -> Result<ResRef, NewError> {
    if !AREA_SIZE_RANGE.contains(&spec.width) || !AREA_SIZE_RANGE.contains(&spec.height) {
        return Err(NewError::Invalid(format!(
            "an area is 2 to 32 tiles across, not {}×{}",
            spec.width, spec.height
        )));
    }
    let set_name = format!("{}.set", spec.tileset);
    let data = game
        .resman
        .get(&ResKey::new(spec.tileset, ResType::SET))
        .map_err(|e| res_error(&set_name, e))?;
    let set =
        Tileset::parse(&data, game.language.codepage()).map_err(|e| res_error(&set_name, e))?;
    let index = TileIndex::new(&set);
    let tiles_error = |source| NewError::Tiles { tileset: spec.tileset.to_string(), source };
    let lattice = mg_tiles::new_area(&index, &set, spec.width, spec.height).map_err(tiles_error)?;
    let tiles = mg_tiles::fill(&index, &lattice, rng).map_err(tiles_error)?;

    let defaults = AreaDefaults::for_tileset(game, spec.tileset)?;
    let scheme = Scheme::read(game, defaults.env_scheme)?;
    let taken = |r: &ResRef| module.contains(&ResKey::new(*r, ResType::ARE));
    let area = resref_for(&spec.name, taken);
    let mut are = Gff::new(*b"ARE ");
    let r = &mut are.root;
    r.write(&are::ID, -1);
    r.write(&are::CREATOR_ID, -1);
    r.write(&are::VERSION, 1);
    r.write(&are::TAG, ExoString::from(tag_for(&spec.name).as_str()));
    r.write(&are::NAME, english(game, &spec.name)?);
    r.write(&are::RES_REF, area);
    r.write(&are::COMMENTS, ExoString::from(""));
    r.items_mut(&are::EXPANSION_LIST);
    r.write(&are::FLAGS, defaults.flags);
    r.write(&are::MOD_SPOT_CHECK, defaults.spot_check);
    r.write(&are::MOD_LISTEN_CHECK, defaults.listen_check);
    for (label, value) in scheme.fields() {
        // Aurora writes these two between the lighting and the weather.
        if label == "DayNightCycle" {
            r.write(&are::FOG_CLIP_DIST, 45.0);
            r.write(&are::SKY_BOX, 0);
        }
        r.set(label, value);
    }
    r.write(&are::LOAD_SCREEN_ID, 0);
    r.write(&are::PLAYER_VS_PLAYER, defaults.pvp);
    r.write(&are::NO_REST, defaults.no_rest);
    r.write(&are::WIDTH, spec.width as i32);
    r.write(&are::HEIGHT, spec.height as i32);
    r.write(&are::ON_ENTER, defaults.scripts[0]);
    r.write(&are::ON_EXIT, defaults.scripts[1]);
    r.write(&are::ON_HEARTBEAT, defaults.scripts[2]);
    r.write(&are::ON_USER_DEFINED, defaults.scripts[3]);
    r.write(&are::TILE_BRDR_DISABLED, 0);
    r.write(&are::TILESET, spec.tileset);
    let list = r.items_mut(&are::TILE_LIST);
    for p in tiles {
        use are::tile_list as t;
        let tile = &set.tiles[p.tile as usize];
        let mut s = are::TILE_LIST.new_item();
        s.write(&t::TILE_ID, p.tile as i32);
        s.write(&t::TILE_ORIENTATION, i32::from(p.orientation));
        s.write(&t::TILE_HEIGHT, p.height);
        let [main1, main2, source] = scheme.tile_lights(rng);
        s.write(&t::TILE_MAIN_LIGHT1, main1);
        s.write(&t::TILE_MAIN_LIGHT2, main2);
        s.write(&t::TILE_SRC_LIGHT1, source);
        s.write(&t::TILE_SRC_LIGHT2, source);
        s.write(&t::TILE_ANIM_LOOP1, u8::from(tile.anim_loops[0]));
        s.write(&t::TILE_ANIM_LOOP2, u8::from(tile.anim_loops[1]));
        s.write(&t::TILE_ANIM_LOOP3, u8::from(tile.anim_loops[2]));
        list.push(s);
    }

    let mut git = Gff::new(*b"GIT ");
    let props = git.root.child_struct_mut(&git::AREA_PROPERTIES);
    props.id = 100;
    {
        use git::area_properties as p;
        let a = &defaults.audio;
        for (f, v) in [
            (&p::AMBIENT_SND_DAY, a.ambient_day),
            (&p::AMBIENT_SND_NIGHT, a.ambient_night),
            (&p::AMBIENT_SND_DAY_VOL, a.ambient_day_volume),
            (&p::AMBIENT_SND_NIT_VOL, a.ambient_night_volume),
            (&p::ENV_AUDIO, a.env_audio),
            (&p::MUSIC_BATTLE, a.music_battle),
            (&p::MUSIC_DAY, a.music_day),
            (&p::MUSIC_NIGHT, a.music_night),
            (&p::MUSIC_DELAY, a.music_delay),
        ] {
            props.write(f, v);
        }
    }
    for f in [
        &git::CREATURE_LIST,
        &git::DOOR_LIST,
        &git::ENCOUNTER_LIST,
        &git::LIST,
        &git::SOUND_LIST,
        &git::STORE_LIST,
        &git::TRIGGER_LIST,
        &git::WAYPOINT_LIST,
        &git::PLACEABLE_LIST,
    ] {
        git.root.items_mut(f);
    }
    let mut gic = Gff::new(*b"GIC ");
    for f in [
        &gic::CREATURE_LIST,
        &gic::DOOR_LIST,
        &gic::ENCOUNTER_LIST,
        &gic::LIST,
        &gic::SOUND_LIST,
        &gic::STORE_LIST,
        &gic::TRIGGER_LIST,
        &gic::WAYPOINT_LIST,
        &gic::PLACEABLE_LIST,
    ] {
        gic.root.items_mut(f);
    }

    let write_error = |e: mg_gff::WriteError| res_error(&area.to_string(), e);
    module.set_gff(ResKey::new(area, ResType::ARE), &are).map_err(write_error)?;
    module.set_gff(ResKey::new(area, ResType::GIT), &git).map_err(write_error)?;
    module.set_gff(ResKey::new(area, ResType::GIC), &gic).map_err(write_error)?;

    let mut info = module.info()?;
    let r = &mut info.root;
    let mut item = ifo::MOD_AREA_LIST.new_item();
    item.write(&ifo::mod_area_list::AREA_NAME, area);
    r.items_mut(&ifo::MOD_AREA_LIST).push(item);
    if r.read(&ifo::MOD_ENTRY_AREA).is_empty() {
        r.write(&ifo::MOD_ENTRY_AREA, area);
        r.write(&ifo::MOD_ENTRY_X, spec.width as f32 * 5.0);
        r.write(&ifo::MOD_ENTRY_Y, spec.height as f32 * 5.0);
        r.write(&ifo::MOD_ENTRY_Z, 0.0);
        r.write(&ifo::MOD_ENTRY_DIR_X, 0.0);
        r.write(&ifo::MOD_ENTRY_DIR_Y, 1.0);
    }
    module.set_info(&info)?;
    Ok(area)
}

/// Area audio a tileset starts with (`areag.ini`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AreaAudio {
    pub ambient_day: i32,
    pub ambient_night: i32,
    pub ambient_day_volume: i32,
    pub ambient_night_volume: i32,
    pub env_audio: i32,
    pub music_battle: i32,
    pub music_day: i32,
    pub music_night: i32,
    pub music_delay: i32,
}

/// The properties a new area of a tileset starts with: its `areag.ini`
/// section, or, for tilesets without one, an exterior with lighting scheme 0
/// and no sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaDefaults {
    /// Row of `environment.2da`.
    pub env_scheme: usize,
    /// Interior 1, subterranean 2, natural 4.
    pub flags: u32,
    pub spot_check: i32,
    pub listen_check: i32,
    pub pvp: u8,
    pub no_rest: u8,
    /// OnEnter, OnExit, OnHeartbeat, OnUserDefined.
    pub scripts: [ResRef; 4],
    pub audio: AreaAudio,
}

impl AreaDefaults {
    pub fn for_tileset(game: &GameData, tileset: ResRef) -> Result<AreaDefaults, NewError> {
        let mut d = AreaDefaults {
            env_scheme: 0,
            flags: 0,
            spot_check: 0,
            listen_check: 0,
            pvp: 3,
            no_rest: 0,
            scripts: [ResRef::EMPTY; 4],
            audio: AreaAudio::default(),
        };
        let Ok(data) = game.resman.get_named("areag", ResType::INI) else { return Ok(d) };
        let ini = Ini::parse(&game.language.codepage().decode(&data));
        let Some(s) = ini.section(&tileset.to_string()) else { return Ok(d) };
        let int = |k: &str| s.int(k).unwrap_or(0);
        d.env_scheme = usize::try_from(int("EnvScheme")).unwrap_or(0);
        d.flags = u32::from(int("Interior") != 0)
            | u32::from(int("Subterranean") != 0) << 1
            | u32::from(int("Natural") != 0) << 2;
        d.spot_check = int("SpotCheck");
        d.listen_check = int("ListenCheck");
        d.pvp = s.int("PlayerVsPlayer").unwrap_or(3) as u8;
        d.no_rest = u8::from(int("NoRest") != 0);
        for (slot, key) in
            d.scripts.iter_mut().zip(["OnEnter", "OnExit", "OnHeartbeat", "OnUserDefined"])
        {
            *slot = ResRef::from_str(s.get(key).unwrap_or("").trim()).unwrap_or(ResRef::EMPTY);
        }
        d.audio = AreaAudio {
            ambient_day: int("AmbientSoundDay"),
            ambient_night: int("AmbientSoundNight"),
            ambient_day_volume: int("AmbientSoundDayVolume"),
            ambient_night_volume: int("AmbientSoundNightVolume"),
            env_audio: int("EnvAudio"),
            music_battle: int("MusicBattle"),
            music_day: int("MusicDay"),
            music_night: int("MusicNight"),
            music_delay: int("MusicDelay"),
        };
        Ok(d)
    }
}
