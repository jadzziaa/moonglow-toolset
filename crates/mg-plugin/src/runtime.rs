//! A plugin's code at work: a Luau machine for one job, sandboxed, with the
//! API it reads and edits a module through.
//!
//! The machine has no file, network or process functions (Luau has none,
//! and none are given). Its reach is the `ctx` its handlers get: the
//! module as a private copy (edits are applied to it at once, so the code
//! reads back what it wrote and a bad edit fails where it is made, and are
//! recorded for the host), the game data, the log, progress and questions.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use mg_core::{Codepage, Gender, Language, LocString, LocStringKey, ResRef, StrRef};
use mg_edit::fix::{Fix, Found};
use mg_edit::wire::field_from_str;
use mg_edit::{Command, Edit, GffPath, Step, Workspace};
use mg_gff::{FieldType, Gff, Struct, Value};
use mg_module::doctor::{Finding, Severity};
use mg_resman::ResKey;
use mg_rules::GameData;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value as LuaValue, Variadic, VmState};

use crate::{
    Answer, CheckDecl, CommandDecl, FieldKind, FormField, HakWrite, Host, Input, Level,
    MEMORY_LIMIT, Manifest, Outcome, Plugin, PluginError, Question, Selected, Selection,
};

/// The codepage of a module's text where its game is not known.
pub(crate) const CODEPAGE: Codepage = Codepage::WINDOWS_1252;

/// How a plugin's code is loaded: as text, never as bytecode (which Luau
/// runs unverified, so a crafted file could crash out of the sandbox; left
/// unset, mlua loads either).
const TEXT: mlua::chunk::ChunkMode = mlua::chunk::ChunkMode::Text;

thread_local! {
    /// The codepage of the text of the job this thread runs: see
    /// [`codepage`].
    static GAME_CODEPAGE: std::cell::Cell<Codepage> = const { std::cell::Cell::new(CODEPAGE) };
}

/// The codepage of a module's text, as the plugin's UTF-8 strings go in
/// and come out: Windows-1252, or the table of an `encoding.2da` in the
/// module's haks. Set when a job's API is made (a job is one thread's).
pub(crate) fn codepage() -> Codepage {
    GAME_CODEPAGE.with(std::cell::Cell::get)
}

/// Where a job's scripts are read from.
pub(crate) enum Source {
    /// Nowhere: the console has no files.
    Nothing,
    /// A plugin's folder.
    Folder(PathBuf),
    /// A plugin's archive, not unpacked, or a project's plugin as it was
    /// read: its files by their paths in it.
    Memory(Arc<BTreeMap<String, Vec<u8>>>),
}

/// Where a plugin's scripts are read from: its folder, or the copy of it
/// a project's plugin was read into (what was allowed).
pub(crate) fn source_of(plugin: &Plugin) -> Source {
    match &plugin.frozen {
        Some(f) => Source::Memory(f.files.clone()),
        None => Source::Folder(plugin.dir.clone()),
    }
}

impl Source {
    /// A script, by its path in the plugin's folder.
    fn read(&self, name: &str) -> std::io::Result<String> {
        use std::io::{Error, ErrorKind};
        match self {
            Source::Nothing => Err(Error::new(ErrorKind::NotFound, "there are no files here")),
            Source::Folder(dir) => std::fs::read_to_string(dir.join(name)),
            Source::Memory(files) => match files.get(name) {
                Some(bytes) => String::from_utf8(bytes.clone())
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "it is not text (UTF-8)")),
                None => Err(Error::new(ErrorKind::NotFound, "no such file")),
            },
        }
    }
}

/// What a job's API works on.
pub(crate) struct Shared {
    files: Source,
    pub(crate) ws: RefCell<Workspace>,
    edits: RefCell<Vec<Edit>>,
    /// What the job wants written into haks, each hak once.
    pub(crate) haks: RefCell<Vec<HakWrite>>,
    /// What the job wants written into folders the user chose.
    pub(crate) folders: RefCell<Vec<crate::FolderWrite>>,
    pub(crate) game: Option<Arc<GameData>>,
    pub(crate) host: Rc<dyn Host>,
    /// Whose job it is, for a question the API itself asks.
    pub(crate) who: String,
    /// Chooses among the tiles that fit.
    pub(crate) rng: RefCell<fastrand::Rng>,
    /// What is selected and open in the window.
    selection: Selection,
    /// The plugin's settings, their values as the job goes (what it set
    /// read back), and what it set.
    decls: RefCell<Vec<crate::SettingDecl>>,
    settings: RefCell<crate::Settings>,
    settings_set: RefCell<Vec<crate::SettingWrite>>,
    /// Why `ctx.settings:set` is refused in this job (a check's), if it is.
    no_set: std::cell::Cell<Option<&'static str>>,
    /// How long the job may run ([`Input::budget`]), and when it must
    /// have ended.
    budget: Option<std::time::Duration>,
    deadline: Option<std::time::Instant>,
}

pub(crate) fn fail<T>(message: impl std::fmt::Display) -> mlua::Result<T> {
    Err(mlua::Error::runtime(message))
}

pub(crate) fn key_of(name: &str) -> mlua::Result<ResKey> {
    match ResKey::from_filename(name) {
        Some(key) => Ok(key),
        None => fail(format!("{name:?} is not a resource's name (name.ext)")),
    }
}

impl Shared {
    /// Whether the job ran past its time budget.
    pub(crate) fn out_of_time(&self) -> bool {
        self.deadline.is_some_and(|d| std::time::Instant::now() > d)
    }

    /// Whether the job is to stop: canceled, or out of time.
    fn stopped(&self) -> bool {
        self.host.cancelled() || self.out_of_time()
    }

    /// Applies an edit to the private copy and records it.
    pub(crate) fn apply(&self, edit: Edit) -> mlua::Result<()> {
        self.apply_all(vec![edit])
    }

    /// Applies edits to the private copy, all or none, and records them.
    pub(crate) fn apply_all(&self, edits: Vec<Edit>) -> mlua::Result<()> {
        if edits.is_empty() {
            return Ok(());
        }
        let applied = self.ws.borrow_mut().apply(Command::new("plugin", edits.clone()));
        match applied {
            Ok(()) => {
                self.edits.borrow_mut().extend(edits);
                Ok(())
            }
            Err(e) => fail(e),
        }
    }

    /// A resource the job has put into a hak (the last one written, of
    /// the hak written to last).
    fn in_haks(&self, key: &ResKey) -> Option<Vec<u8>> {
        let haks = self.haks.borrow();
        let found = haks.iter().rev().find_map(|h| h.files.iter().rev().find(|(k, _)| k == key))?;
        match &found.1 {
            crate::HakData::Bytes(b) => Some(b.to_vec()),
            crate::HakData::File(p) => std::fs::read(p).ok(),
            crate::HakData::Archive => None,
        }
    }

    /// A resource's bytes: the module's (with the job's edits), or with
    /// `game` what the game would load (what the job has put into haks
    /// first).
    pub(crate) fn bytes(&self, key: &ResKey, game: bool) -> mlua::Result<Vec<u8>> {
        if game {
            if let Some(bytes) = self.in_haks(key) {
                return Ok(bytes);
            }
            let Some(data) = &self.game else {
                self.host.not_in_game(&key.to_string());
                return fail(format!("{key}: there is no game data (no game install)"));
            };
            return match data.resman.get(key) {
                Ok(bytes) => Ok(bytes.into_owned()),
                Err(_) => {
                    self.host.not_in_game(&key.to_string());
                    fail(format!("{key} is not in the game's resources"))
                }
            };
        }
        let mut ws = self.ws.borrow_mut();
        if let Err(e) = ws.flush() {
            return fail(e);
        }
        match ws.module.get(key) {
            Some(bytes) => Ok(bytes.to_vec()),
            None => fail(format!("{key} is not in the module")),
        }
    }

    /// A GFF resource's root.
    fn gff(&self, key: &ResKey, game: bool) -> mlua::Result<Gff> {
        if !game {
            return match self.ws.borrow_mut().doc(key) {
                Ok(gff) => Ok(gff.clone()),
                Err(e) => fail(e),
            };
        }
        let bytes = self.bytes(key, true)?;
        Gff::read(&bytes).or_else(|e| fail(format!("{key}: {e}")))
    }
}

/// A struct as a table of plain values, by label: numbers, text, tables
/// for structs and (from 1) for lists' items; a localized string as its
/// texts by language number, with `strref`. Each struct's table knows
/// where it is (`path`).
fn plain(lua: &Lua, s: &Struct, path: &GffPath) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        let text = |b: &[u8]| lua.create_string(codepage().decode(b).as_bytes());
        let v = match &f.value {
            Value::Byte(v) => LuaValue::Number(f64::from(*v)),
            Value::Char(v) => LuaValue::Number(f64::from(*v)),
            Value::Word(v) => LuaValue::Number(f64::from(*v)),
            Value::Short(v) => LuaValue::Number(f64::from(*v)),
            Value::Dword(v) => LuaValue::Number(f64::from(*v)),
            Value::Int(v) => LuaValue::Number(f64::from(*v)),
            Value::Dword64(v) => LuaValue::Number(*v as f64),
            Value::Int64(v) => LuaValue::Number(*v as f64),
            Value::Float(v) => LuaValue::Number(f64::from(*v)),
            Value::Double(v) => LuaValue::Number(*v),
            Value::String(b) | Value::ResRef(b) => LuaValue::String(text(b)?),
            Value::Void(b) => LuaValue::String(lua.create_string(b)?),
            Value::LocString(ls) => {
                let l = lua.create_table()?;
                if !ls.strref.is_none() {
                    l.raw_set("strref", f64::from(ls.strref.0))?;
                }
                for (key, bytes) in &ls.strings {
                    l.raw_set(f64::from(key.0), text(bytes)?)?;
                }
                LuaValue::Table(l)
            }
            Value::Struct(child) => LuaValue::Table(plain(lua, child, &path.field(&label))?),
            Value::List(items) => {
                let list = lua.create_table()?;
                for (i, item) in items.iter().enumerate() {
                    list.raw_seti(i + 1, plain(lua, item, &path.item(&label, i))?)?;
                }
                LuaValue::Table(list)
            }
        };
        t.raw_set(label, v)?;
    }
    let meta = lua.create_table()?;
    // (The root's is empty, so that a field's path is this and "/Label".)
    meta.raw_set("path", if path.0.is_empty() { String::new() } else { path.to_string() })?;
    meta.raw_set("struct_id", f64::from(s.id))?;
    t.set_metatable(Some(meta))?;
    Ok(t)
}

/// A number given for a whole-number field.
fn whole(v: &LuaValue, ty: FieldType, min: i128, max: i128) -> Result<i128, String> {
    let n = match v {
        LuaValue::Integer(i) => i128::from(*i),
        LuaValue::Number(f) if f.fract() == 0.0 && f.abs() < 9e15 => *f as i128,
        LuaValue::Boolean(b) => i128::from(*b),
        // (Past what a number holds exactly.)
        LuaValue::String(s) => match s.to_str().ok().and_then(|s| s.trim().parse().ok()) {
            Some(n) => n,
            None => return Err(format!("a {} takes a whole number", ty.json_name())),
        },
        _ => return Err(format!("a {} takes a whole number", ty.json_name())),
    };
    if n < min || n > max {
        return Err(format!("{n} does not fit a {} ({min} to {max})", ty.json_name()));
    }
    Ok(n)
}

fn text_of(v: &LuaValue, ty: FieldType) -> Result<Vec<u8>, String> {
    let LuaValue::String(s) = v else { return Err(format!("a {} takes text", ty.json_name())) };
    let s = s.to_str().map_err(|_| "text must be UTF-8".to_string())?;
    match codepage().encode(&s) {
        Some(bytes) => Ok(bytes.into_owned()),
        None => Err("text has characters the module's codepage cannot hold".into()),
    }
}

/// A plain value as a field of type `ty` (text for a localized string
/// becomes its English, the rest of `old` kept).
fn typed(ty: FieldType, v: &LuaValue, old: Option<&Value>) -> Result<Value, String> {
    let int = |min: i128, max: i128| whole(v, ty, min, max);
    Ok(match ty {
        FieldType::Byte
        | FieldType::Char
        | FieldType::Word
        | FieldType::Short
        | FieldType::Dword
        | FieldType::Int
        | FieldType::Dword64
        | FieldType::Int64 => {
            let (min, max) = ty.int_range().expect("an integer type");
            ty.int_value(int(min, max)?).expect("within the type's range")
        }
        FieldType::Float | FieldType::Double => {
            let n = match v {
                LuaValue::Integer(i) => *i as f64,
                LuaValue::Number(f) if f.is_finite() => *f,
                _ => return Err(format!("a {} takes a number", ty.json_name())),
            };
            if ty == FieldType::Float { Value::Float(n as f32) } else { Value::Double(n) }
        }
        FieldType::String => Value::String(text_of(v, ty)?),
        FieldType::ResRef => {
            let bytes = text_of(v, ty)?;
            let name = String::from_utf8_lossy(&bytes).into_owned();
            let r = ResRef::from_str(&name)
                .map_err(|e| format!("{name:?} is not a resource name: {e}"))?;
            Value::resref(r)
        }
        FieldType::Void => match v {
            LuaValue::String(s) => Value::Void(s.as_bytes().to_vec()),
            _ => return Err("a void takes a string of bytes".into()),
        },
        FieldType::LocString => match v {
            LuaValue::String(_) => {
                let mut s = match old {
                    Some(Value::LocString(s)) => s.clone(),
                    _ => LocString::default(),
                };
                let bytes = text_of(v, ty)?;
                if bytes.is_empty() {
                    s.remove(Language::ENGLISH, Gender::Male);
                } else {
                    s.set(Language::ENGLISH, Gender::Male, bytes);
                }
                Value::LocString(s)
            }
            // The whole of it: `strref`, and its texts by language number.
            LuaValue::Table(t) => {
                let mut s = LocString::default();
                for pair in t.pairs::<LuaValue, LuaValue>() {
                    let (k, text) = pair.map_err(|e| e.to_string())?;
                    match &k {
                        LuaValue::String(name) if name.as_bytes() == b"strref" => {
                            s.strref =
                                StrRef(whole(&text, FieldType::Dword, 0, u32::MAX.into())? as u32);
                        }
                        LuaValue::Integer(_) | LuaValue::Number(_) => {
                            let n = whole(&k, FieldType::Dword, 0, u32::MAX.into())? as u32;
                            s.strings.push((LocStringKey(n), text_of(&text, ty)?));
                        }
                        _ => {
                            return Err(
                                "a localized string has `strref` and texts by language number"
                                    .into(),
                            );
                        }
                    }
                }
                s.strings.sort_by_key(|(k, _)| k.0);
                Value::LocString(s)
            }
            _ => return Err("a cexolocstring takes text, or a table of texts".into()),
        },
        FieldType::Struct | FieldType::List => {
            return Err(format!(
                "a {} is not set as a value: set its fields, or insert and remove its items",
                ty.json_name()
            ));
        }
    })
}

/// A table of the plugin's as JSON. A list with no items written by hand,
/// `{ type = "list", value = {} }`, is an empty table, which reads as an
/// object: it is made the empty list it stands for.
fn json_of(lua: &Lua, v: LuaValue) -> mlua::Result<serde_json::Value> {
    fn lists(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                let empty = o.get("type").and_then(|t| t.as_str()) == Some("list")
                    && o.get("value").and_then(|v| v.as_object()).is_some_and(|v| v.is_empty());
                if empty {
                    o.insert("value".into(), serde_json::Value::Array(Vec::new()));
                }
                o.values_mut().for_each(lists);
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(lists),
            _ => {}
        }
    }
    let mut json: serde_json::Value = lua.from_value(v)?;
    lists(&mut json);
    Ok(json)
}

/// A table that is a value with its type, as `mg.int(3)` makes it and
/// `mg gff` writes it: `{ type = "int", value = 3 }` (`what`: the field
/// it is for, for an error).
fn typed_table(lua: &Lua, v: &LuaValue, what: &str) -> mlua::Result<Option<Value>> {
    let LuaValue::Table(t) = v else { return Ok(None) };
    if !matches!(t.raw_get::<LuaValue>("type")?, LuaValue::String(_)) {
        return Ok(None);
    }
    let json = json_of(lua, v.clone())?;
    match mg_gff::value_from_json(&json, codepage()) {
        Ok(value) => Ok(Some(value)),
        Err(e) => fail(format!("{what}{e}")),
    }
}

/// The first argument of a call made with a colon (`ctx.module:has(..)`):
/// the table the function is in, not used.
pub(crate) type This = LuaValue;

/// `ctx.module` or `ctx.game`: resources to read.
fn reader(lua: &Lua, sh: &Rc<Shared>, game: bool) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    let s = sh.clone();
    t.set(
        "resources",
        lua.create_function(move |_, (_, kind): (This, Option<String>)| {
            let wanted = |key: &ResKey| {
                kind.as_deref().is_none_or(|k| key.restype.to_string().eq_ignore_ascii_case(k))
            };
            let mut names: Vec<String> = if game {
                s.host.game_listed();
                let Some(data) = &s.game else { return fail("there is no game data") };
                data.resman
                    .entries()
                    .iter()
                    .map(|(k, _)| *k)
                    .filter(wanted)
                    .map(|k| k.to_string())
                    .collect()
            } else {
                s.ws.borrow().module.keys().filter(|k| wanted(k)).map(ToString::to_string).collect()
            };
            names.sort();
            Ok(names)
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "has",
        lua.create_function(move |_, (_, name): (This, String)| {
            let key = key_of(&name)?;
            Ok(if game {
                s.in_haks(&key).is_some()
                    || s.game.as_ref().is_some_and(|g| g.resman.contains(&key))
            } else {
                s.ws.borrow().module.contains(&key)
            })
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "gff",
        lua.create_function(move |lua, (_, name): (This, String)| {
            let gff = s.gff(&key_of(&name)?, game)?;
            plain(lua, &gff.root, &GffPath::root())
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "raw",
        lua.create_function(move |lua, (_, name, path): (This, String, Option<String>)| {
            let key = key_of(&name)?;
            let gff = s.gff(&key, game)?;
            let json = match path.filter(|p| !p.is_empty()) {
                None => mg_gff::to_json(&gff, codepage()),
                Some(path) => {
                    let path: GffPath = path.parse().or_else(fail)?;
                    match path.get(&gff.root) {
                        Some(at) => mg_gff::struct_to_json(at, codepage()),
                        None => return fail(format!("{key}: no struct at {path}")),
                    }
                }
            };
            lua.to_value(&json.or_else(|e| fail(format!("{key}: {e}")))?)
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "text",
        lua.create_function(move |lua, (_, name): (This, String)| {
            let bytes = s.bytes(&key_of(&name)?, game)?;
            lua.create_string(codepage().decode(&bytes).as_bytes())
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "bytes",
        lua.create_function(move |lua, (_, name): (This, String)| {
            lua.create_string(s.bytes(&key_of(&name)?, game)?)
        })?,
    )?;
    if game {
        let s = sh.clone();
        t.set(
            "table",
            lua.create_function(move |lua, (_, name): (This, String)| {
                let file = format!("{}.2da", name.trim_end_matches(".2da"));
                let Some(data) = &s.game else {
                    s.host.not_in_game(&file);
                    return fail(format!("{file}: there is no game data (no game install)"));
                };
                let had = ResKey::from_filename(&file).is_some_and(|k| data.resman.contains(&k));
                if !had {
                    s.host.not_in_game(&file);
                }
                let table = data.table(&name).or_else(fail)?;
                crate::tables::twoda(lua, table.clone())
            })?,
        )?;
        let s = sh.clone();
        t.set(
            "string",
            lua.create_function(move |_, (_, strref): (This, u32)| {
                Ok(s.game.as_ref().and_then(|g| g.string(StrRef(strref))))
            })?,
        )?;
    }
    Ok(t)
}

/// `ctx.edit`: the changes a command makes.
fn editor(lua: &Lua, sh: &Rc<Shared>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    let s = sh.clone();
    t.set(
        "set",
        lua.create_function(
            move |lua, (_, name, field, value, ty): (This, String, String, LuaValue, Option<String>)| {
                let key = key_of(&name)?;
                let (path, label) = field_from_str(&field).or_else(fail)?;
                let old = {
                    let mut ws = s.ws.borrow_mut();
                    let doc = ws.doc(&key).or_else(fail)?;
                    match path.get(&doc.root) {
                        Some(at) => at.get(&label).cloned(),
                        None => return fail(format!("{key}: no struct at {path}")),
                    }
                };
                let value = match typed_table(lua, &value, &format!("{key} {field}"))? {
                    Some(value) => value,
                    None => {
                        // Its own type; else the one given; else the game's.
                        let given = match &ty {
                            Some(name) => match FieldType::from_json_name(name) {
                                Some(t) => Some(t),
                                None => return fail(format!("{name:?} is not a field type")),
                            },
                            None => None,
                        };
                        let schema = || {
                            path.0
                                .is_empty()
                                .then(|| mg_schema::root_field_type(key.restype, &label))
                                .flatten()
                        };
                        let Some(ty) = old.as_ref().map(Value::field_type).or(given).or_else(schema)
                        else {
                            return fail(format!(
                                "{key} has no field {field}: give its type (\"int\"…), or a typed \
                                 value (mg.int(3))"
                            ));
                        };
                        typed(ty, &value, old.as_ref())
                            .or_else(|e| fail(format!("{key} {field}: {e}")))?
                    }
                };
                s.apply(Edit::SetField { key, path, label, value: Some(value) })
            },
        )?,
    )?;
    let s = sh.clone();
    t.set(
        "remove",
        lua.create_function(move |_, (_, name, field): (This, String, String)| {
            let key = key_of(&name)?;
            let (path, label) = field_from_str(&field).or_else(fail)?;
            let has = {
                let mut ws = s.ws.borrow_mut();
                let doc = ws.doc(&key).or_else(fail)?;
                path.get(&doc.root).is_some_and(|at| at.get(&label).is_some())
            };
            if !has {
                return fail(format!("{key} has no field {field}"));
            }
            s.apply(Edit::SetField { key, path, label, value: None })
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "insert",
        lua.create_function(
            move |lua, (_, name, list, item, index): (This, String, String, LuaValue, Option<usize>)| {
                let key = key_of(&name)?;
                let (path, list) = field_from_str(&list).or_else(fail)?;
                let json = json_of(lua, item)?;
                let item = mg_gff::struct_from_json(&json, codepage())
                    .or_else(|e| fail(format!("{key} {list}: the item{e}")))?;
                // At the end, unless a place is given (0 is first).
                let index = match index {
                    Some(i) => i,
                    None => {
                        let mut ws = s.ws.borrow_mut();
                        let doc = ws.doc(&key).or_else(fail)?;
                        path.get(&doc.root).and_then(|at| at.list(&list)).map_or(0, <[_]>::len)
                    }
                };
                s.apply(Edit::InsertItem { key, path, list, index, item })?;
                Ok(index)
            },
        )?,
    )?;
    let s = sh.clone();
    t.set(
        "remove_item",
        lua.create_function(move |_, (_, name, item): (This, String, String)| {
            let key = key_of(&name)?;
            let path: GffPath = item.parse().or_else(fail)?;
            let mut steps = path.0;
            match steps.pop() {
                Some(Step::Item(list, index)) => {
                    s.apply(Edit::RemoveItem { key, path: GffPath(steps), list, index })
                }
                _ => fail(format!("{item:?} is not a list's item (/List[index])")),
            }
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "write",
        lua.create_function(move |_, (_, name, text): (This, String, mlua::LuaString)| {
            let key = key_of(&name)?;
            let text =
                text.to_str().or_else(|_| fail("text must be UTF-8 (write_bytes takes any)"))?;
            let Some(data) = codepage().encode(&text) else {
                return fail("text has characters the module's codepage cannot hold");
            };
            s.apply(Edit::SetResource { key, data: Some(data.into_owned()) })
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "write_bytes",
        lua.create_function(move |_, (_, name, bytes): (This, String, mlua::LuaString)| {
            let key = key_of(&name)?;
            s.apply(Edit::SetResource { key, data: Some(bytes.as_bytes().to_vec()) })
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "write_gff",
        lua.create_function(move |lua, (_, name, doc): (This, String, LuaValue)| {
            let key = key_of(&name)?;
            let json = json_of(lua, doc)?;
            let gff =
                mg_gff::from_json(&json, codepage()).or_else(|e| fail(format!("{key}{e}")))?;
            let data = gff.to_bytes().or_else(|e| fail(format!("{key}: {e}")))?;
            s.apply(Edit::SetResource { key, data: Some(data) })
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "delete",
        lua.create_function(move |_, (_, name): (This, String)| {
            let key = key_of(&name)?;
            if !s.ws.borrow().module.contains(&key) {
                return fail(format!("{key} is not in the module"));
            }
            s.apply(Edit::SetResource { key, data: None })
        })?,
    )?;
    Ok(t)
}

/// A form's fields from the plugin's table of them.
fn form_fields(fields: Table) -> mlua::Result<Vec<FormField>> {
    let mut out = Vec::new();
    for field in fields.sequence_values::<Table>() {
        let f = field?;
        let id: String = f.get("id")?;
        let label = f.get::<Option<String>>("label")?.unwrap_or_else(|| id.clone());
        let kind = match f.get::<Option<String>>("type")?.as_deref().unwrap_or("text") {
            "text" => {
                FieldKind::Text { default: f.get::<Option<String>>("default")?.unwrap_or_default() }
            }
            "number" => FieldKind::Number {
                default: f.get::<Option<f64>>("default")?.unwrap_or(0.0),
                min: f.get("min")?,
                max: f.get("max")?,
            },
            "check" => {
                FieldKind::Check { default: f.get::<Option<bool>>("default")?.unwrap_or(false) }
            }
            "choice" => {
                let choices: Vec<String> = f.get("choices")?;
                if choices.is_empty() {
                    return fail(format!("the choice {id:?} has no choices"));
                }
                let wanted = f.get::<Option<String>>("default")?;
                let default =
                    wanted.and_then(|w| choices.iter().position(|c| *c == w)).unwrap_or(0);
                FieldKind::Choice { choices, default }
            }
            other => {
                return fail(format!(
                    "a form's field is \"text\", \"number\", \"check\" or \"choice\", not {other:?}"
                ));
            }
        };
        if out.iter().any(|o: &FormField| o.id == id) {
            return fail(format!("a form has two fields {id:?}"));
        }
        out.push(FormField { id, label, kind });
    }
    Ok(out)
}

/// The fields of an entry of `ctx.selection` (and of `ctx.open`), as the
/// reference names them.
pub(crate) const SELECTED: [&str; 9] = [
    "selected.kind",
    "selected.resource",
    "selected.path",
    "selected.blueprint",
    "selected.module",
    "selected.area",
    "selected.index",
    "selected.tag",
    "selected.resref",
];

/// An entry of `ctx.selection`, or `ctx.open`: what it is, where it is
/// (the resource and path `ctx.edit` takes), and its tag and resref as
/// the job's copy has them. A blueprint the module lacks is the game's.
fn selected(lua: &Lua, sh: &Shared, s: &Selected) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    let (key, path) = s.place();
    t.set("kind", s.kind())?;
    t.set("resource", key.to_string())?;
    // (The root's is empty, as `mg.path` has it: a field is this and
    // "/Label".)
    t.set("path", if path.0.is_empty() { String::new() } else { path.to_string() })?;
    t.set("blueprint", matches!(s, Selected::Blueprint(_)))?;
    match s {
        Selected::Object { area, index, .. } => {
            t.set("area", area.to_string())?;
            t.set("index", *index)?;
        }
        Selected::Area(area) => t.set("area", area.to_string())?,
        Selected::Blueprint(_) | Selected::Resource(_) => {}
    }
    let in_module = sh.ws.borrow().module.contains(&key);
    t.set("module", in_module)?;
    let read = match s {
        Selected::Resource(_) => None,
        Selected::Blueprint(_) if !in_module => sh.gff(&key, true).ok(),
        _ => sh.gff(&key, false).ok(),
    };
    let text = |b: &[u8]| lua.create_string(codepage().decode(b).as_bytes());
    if let Some(at) = read.as_ref().and_then(|g| path.get(&g.root)) {
        if let Some(tag) = at.string("Tag") {
            t.set("tag", text(tag)?)?;
        }
        // (What a placed object was made from; else its own name.)
        let resref = match s {
            Selected::Object { .. } => at.resref("TemplateResRef"),
            _ => Some(key.resref),
        };
        if let Some(r) = resref {
            t.set("resref", r.to_string())?;
        }
    } else if !matches!(s, Selected::Object { .. }) {
        t.set("resref", key.resref.to_string())?;
    }
    Ok(t)
}

/// A setting's value as the plugin reads it.
fn setting_value(lua: &Lua, v: &crate::SettingValue) -> mlua::Result<LuaValue> {
    Ok(match v {
        crate::SettingValue::Text(t) => LuaValue::String(lua.create_string(t)?),
        crate::SettingValue::Int(n) => LuaValue::Integer(*n),
        crate::SettingValue::Bool(b) => LuaValue::Boolean(*b),
    })
}

/// `ctx.settings`: the plugin's settings to read by name
/// (`ctx.settings.code`), to go through (`for name, value in
/// ctx.settings`), and to set (`ctx.settings:set(name, value)`, nil for
/// the default), which the host keeps when the job is done.
fn settings_table(lua: &Lua, sh: &Rc<Shared>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    let meta = lua.create_table()?;
    let s = sh.clone();
    let set = lua.create_function(move |_, (_, name, value): (This, String, LuaValue)| {
        if let Some(why) = s.no_set.get() {
            return fail(why);
        }
        let decls = s.decls.borrow();
        let Some(d) = decls.iter().find(|d| d.id == name) else {
            return fail(unknown_setting(&name, &decls));
        };
        if d.scope.in_project() && !s.settings.borrow().project {
            return fail(format!(
                "ctx.settings:set({name:?}): it is kept in the project's files (scope {}), and \
                 no project is open (a module kept in one file has none)",
                d.scope.name()
            ));
        }
        let given = match &value {
            LuaValue::Nil => None,
            LuaValue::String(t) => Some(crate::SettingValue::Text(t.to_str()?.to_string())),
            LuaValue::Boolean(b) => Some(crate::SettingValue::Bool(*b)),
            LuaValue::Integer(n) => Some(crate::SettingValue::Int(*n)),
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => {
                Some(crate::SettingValue::Int(*n as i64))
            }
            other => {
                return fail(format!(
                    "ctx.settings:set({name:?}, …): {} is no setting's value (text, a whole \
                     number, true or false; nil for its default)",
                    other.type_name()
                ));
            }
        };
        if let Some(v) = &given
            && let Err(why) = d.check(v)
        {
            return fail(format!("ctx.settings:set({name:?}, …): {why}"));
        }
        let now = given.clone().unwrap_or_else(|| d.default.clone());
        s.settings.borrow_mut().values.insert(name.clone(), now);
        s.settings_set.borrow_mut().push((name, given));
        Ok(())
    })?;
    let s = sh.clone();
    meta.set(
        "__index",
        lua.create_function(move |lua, (_, key): (LuaValue, LuaValue)| {
            let name = match &key {
                LuaValue::String(k) => k.to_str()?.to_string(),
                other => {
                    return fail(format!(
                        "ctx.settings[…]: a setting's name is text, not {}",
                        other.type_name()
                    ));
                }
            };
            if name == "set" {
                return Ok(LuaValue::Function(set.clone()));
            }
            match s.settings.borrow().values.get(&name) {
                Some(v) => setting_value(lua, v),
                None => fail(format!(
                    "ctx.settings.{name}: {}",
                    unknown_setting(&name, &s.decls.borrow())
                )),
            }
        })?,
    )?;
    meta.set(
        "__newindex",
        lua.create_function(|_, (_, key, _): (LuaValue, LuaValue, LuaValue)| -> mlua::Result<()> {
            let name = key.to_string().unwrap_or_default();
            fail(format!(
                "ctx.settings.{name} = …: a setting is set with ctx.settings:set({name:?}, value)"
            ))
        })?,
    )?;
    // (`for name, value in ctx.settings`: each in the manifest's order.)
    let s = sh.clone();
    meta.set(
        "__iter",
        lua.create_function(move |lua, _: LuaValue| {
            let values = s.settings.borrow();
            let mut pairs = Vec::new();
            for d in s.decls.borrow().iter() {
                if let Some(v) = values.values.get(&d.id) {
                    pairs.push((d.id.clone(), setting_value(lua, v)?));
                }
            }
            let at = std::cell::Cell::new(0);
            lua.create_function(move |_, ()| {
                let i = at.get();
                at.set(i + 1);
                Ok(match pairs.get(i) {
                    Some((name, v)) => (Some(name.clone()), v.clone()),
                    None => (None, LuaValue::Nil),
                })
            })
        })?,
    )?;
    meta.set("__metatable", "ctx.settings")?;
    t.set_metatable(Some(meta))?;
    Ok(t)
}

/// What a name that is none of the plugin's settings is told.
fn unknown_setting(name: &str, decls: &[crate::SettingDecl]) -> String {
    match decls {
        [] => {
            format!("the plugin declares no settings ([setting] in its manifest), and no {name:?}")
        }
        decls => {
            let names: Vec<&str> = decls.iter().map(|d| d.id.as_str()).collect();
            format!("the plugin declares no setting {name:?} (it has {})", names.join(", "))
        }
    }
}

/// What a job's handler gets (`about`: the plugin's id, name and version).
fn context(lua: &Lua, sh: &Rc<Shared>, about: [&str; 3]) -> mlua::Result<Table> {
    let ctx = lua.create_table()?;
    ctx.set("module", reader(lua, sh, false)?)?;
    ctx.set("game", reader(lua, sh, true)?)?;
    ctx.set("edit", editor(lua, sh)?)?;
    ctx.set("hak", crate::files::hak(lua, sh)?)?;
    ctx.set("terrain", crate::terrain::terrain(lua, sh)?)?;
    let plugin = lua.create_table()?;
    plugin.set("id", about[0])?;
    plugin.set("name", about[1])?;
    plugin.set("version", about[2])?;
    ctx.set("plugin", plugin)?;
    ctx.set("settings", settings_table(lua, sh)?)?;
    let chosen = lua.create_table()?;
    for (i, s) in sh.selection.selected.iter().enumerate() {
        chosen.raw_seti(i + 1, selected(lua, sh, s)?)?;
    }
    ctx.set("selection", chosen)?;
    if let Some(open) = &sh.selection.open {
        ctx.set("open", selected(lua, sh, open)?)?;
    }

    let log = lua.create_table()?;
    for (name, level) in [("info", Level::Info), ("warn", Level::Warning), ("error", Level::Error)]
    {
        let s = sh.clone();
        log.set(
            name,
            lua.create_function(move |_, (_, text): (This, String)| {
                s.host.log(level, &text);
                Ok(())
            })?,
        )?;
    }
    ctx.set("log", log)?;

    let s = sh.clone();
    ctx.set(
        "progress",
        lua.create_function(move |_, (done, total, note): (usize, usize, Option<String>)| {
            s.host.progress(done, total, note.as_deref().unwrap_or_default());
            Ok(())
        })?,
    )?;

    let ui = lua.create_table()?;
    let s = sh.clone();
    ui.set(
        "message",
        lua.create_function(move |_, (_, text): (This, String)| {
            s.host.ask(&Question::Message(text));
            Ok(())
        })?,
    )?;
    let s = sh.clone();
    ui.set(
        "confirm",
        lua.create_function(move |_, (_, text): (This, String)| {
            // (Nobody to ask: no.)
            Ok(s.host.ask(&Question::Confirm(text)) == Some(Answer::Yes))
        })?,
    )?;
    let s = sh.clone();
    ui.set(
        "form",
        lua.create_function(move |lua, (_, form): (This, Table)| {
            let title = form.get::<Option<String>>("title")?.unwrap_or_default();
            let fields = form_fields(form.get("fields")?)?;
            match s.host.ask(&Question::Form { title, fields }) {
                Some(Answer::Values(values)) => lua.to_value(&values),
                _ => Ok(LuaValue::Nil),
            }
        })?,
    )?;
    crate::files::questions(lua, sh, &ui)?;
    ctx.set("ui", ui)?;
    Ok(ctx)
}

/// What `require("@moonglow")` gives: where handlers are registered, and
/// helpers.
fn api(lua: &Lua) -> mlua::Result<Table> {
    let mg = lua.create_table()?;
    mg.set("api", crate::API)?;
    for (kind, registry) in
        [("command", "mg.commands"), ("check", "mg.checks"), ("event", "mg.events")]
    {
        lua.set_named_registry_value(registry, lua.create_table()?)?;
        mg.set(
            if kind == "event" { "on" } else { kind },
            lua.create_function(move |lua, (id, handler): (String, Function)| {
                let handlers: Table = lua.named_registry_value(registry)?;
                if handlers.contains_key(id.as_str())? {
                    return fail(format!("the {kind} {id:?} is registered twice"));
                }
                handlers.set(id, handler)
            })?,
        )?;
    }
    // A value with its type, for a field that has none yet: mg.int(3).
    for ty in (0..16).filter_map(FieldType::from_id) {
        if matches!(ty, FieldType::Struct | FieldType::List | FieldType::Void) {
            continue;
        }
        let name = match ty {
            FieldType::String => "string",
            FieldType::LocString => "locstring",
            other => other.json_name(),
        };
        mg.set(
            name,
            lua.create_function(move |lua, value: LuaValue| {
                let t = lua.create_table()?;
                t.set("type", ty.json_name())?;
                match (&value, ty) {
                    // (A localized string's text alone is its English.)
                    (LuaValue::String(_), FieldType::LocString) => {
                        let texts = lua.create_table()?;
                        texts.set("0", value)?;
                        t.set("value", texts)?;
                    }
                    // Its texts by language number and `strref`, as `gff`
                    // reads one (written "0" and `id`, as `raw` has it).
                    (LuaValue::Table(given), FieldType::LocString) => {
                        let texts = lua.create_table()?;
                        for pair in given.pairs::<LuaValue, LuaValue>() {
                            let (k, text) = pair?;
                            match k {
                                LuaValue::Integer(n) => texts.set(n.to_string(), text)?,
                                LuaValue::Number(n) if n.fract() == 0.0 => {
                                    texts.set((n as i64).to_string(), text)?
                                }
                                LuaValue::String(s) if s.as_bytes() == b"strref" => {
                                    texts.set("id", text)?
                                }
                                other => texts.set(other, text)?,
                            }
                        }
                        t.set("value", texts)?;
                    }
                    _ => t.set("value", value)?,
                }
                Ok(t)
            })?,
        )?;
    }
    mg.set("image", lua.create_function(crate::files::image)?)?;
    mg.set("twoda", crate::tables::twoda_api(lua)?)?;
    mg.set("tlk", crate::tables::tlk_api(lua)?)?;
    // Where a struct read with `gff` is: its path, for the edits.
    mg.set(
        "path",
        lua.create_function(|_, t: Table| match t.metatable() {
            Some(meta) => meta.raw_get::<Option<String>>("path"),
            None => Ok(None),
        })?,
    )?;
    // A struct's id: kept beside a `gff` table (in its metatable, as its
    // path is), never in it, so that no field's label can be taken for it
    // and the table's fields are its fields alone; a `raw` struct's
    // `__struct_id`.
    mg.set(
        "struct_id",
        lua.create_function(|_, t: Table| {
            if let Some(meta) = t.metatable()
                && let Some(id) = meta.raw_get::<Option<f64>>("struct_id")?
            {
                return Ok(Some(id));
            }
            match t.raw_get::<LuaValue>("__struct_id")? {
                LuaValue::Integer(i) => Ok(Some(i as f64)),
                LuaValue::Number(n) => Ok(Some(n)),
                _ => Ok(None),
            }
        })?,
    )?;
    Ok(mg)
}

/// A machine for one job, the plugin's code loaded (`entry`; the console
/// has none).
fn start(entry: Option<&str>, sh: &Rc<Shared>) -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.set_memory_limit(MEMORY_LIMIT)?;
    lua.set_named_registry_value("mg.api", api(&lua)?)?;
    lua.set_named_registry_value("mg.modules", lua.create_table()?)?;
    let globals = lua.globals();

    // print goes to the log.
    let s = sh.clone();
    globals.set(
        "print",
        lua.create_function(move |lua, values: Variadic<LuaValue>| {
            let tostring: Function = lua.globals().get("tostring")?;
            let parts = values
                .into_iter()
                .map(|v| tostring.call::<String>(v))
                .collect::<mlua::Result<Vec<_>>>()?;
            s.host.log(Level::Info, &parts.join("\t"));
            Ok(())
        })?,
    )?;

    // require: the API, or a file of the plugin's own folder.
    let files = sh.clone();
    globals.set(
        "require",
        lua.create_function(move |lua, name: String| -> mlua::Result<LuaValue> {
            if name == "@moonglow" {
                return lua.named_registry_value("mg.api");
            }
            let file = name.strip_prefix("./").unwrap_or(&name);
            let file = file.strip_suffix(".luau").unwrap_or(file);
            let inside = !file.starts_with('/')
                && !file.contains('\\')
                && !file.contains(':')
                && file.split('/').all(|p| !p.is_empty() && p != ".." && p != ".");
            if !inside {
                return fail(format!("require({name:?}): only files of the plugin's folder"));
            }
            // (The console has no folder.)
            if matches!(files.files, Source::Nothing) {
                return fail(format!("require({name:?}): there are no files to require here"));
            }
            let loaded: Table = lua.named_registry_value("mg.modules")?;
            let cached: LuaValue = loaded.get(file)?;
            if !cached.is_nil() {
                return Ok(cached);
            }
            let source = files
                .files
                .read(&format!("{file}.luau"))
                .or_else(|e| fail(format!("require({name:?}): {e}")))?;
            let value: LuaValue =
                lua.load(source).set_mode(TEXT).set_name(format!("@{file}.luau")).eval()?;
            // (Loaded once, whatever it returns.)
            let kept = if value.is_nil() { LuaValue::Boolean(true) } else { value };
            loaded.set(file, kept.clone())?;
            Ok(kept)
        })?,
    )?;

    // Called off, the code stops: every step raises, and what catches
    // errors (pcall, xpcall) passes this one on.
    let s = sh.clone();
    globals.set("__cancelled", lua.create_function(move |_, ()| Ok(s.stopped()))?)?;
    lua.load(
        r#"
        local cancelled, real_pcall, real_xpcall = __cancelled, pcall, xpcall
        __cancelled = nil
        local function pass(...)
            if cancelled() then error("canceled", 0) end
            return ...
        end
        pcall = function(f, ...) return pass(real_pcall(f, ...)) end
        xpcall = function(f, handler, ...) return pass(real_xpcall(f, handler, ...)) end
        "#,
    )
    .set_name("=moonglow")
    .exec()?;
    let s = sh.clone();
    lua.set_interrupt(move |_| {
        if s.host.cancelled() {
            fail("canceled")
        } else if s.out_of_time() {
            fail("out of time")
        } else {
            Ok(VmState::Continue)
        }
    });

    // From here the globals are fixed, and each script has its own.
    lua.sandbox(true)?;
    if let Some(entry) = entry {
        let source = sh.files.read(entry).or_else(|e| fail(format!("{entry}: {e}")))?;
        lua.load(source).set_mode(TEXT).set_name(format!("@{entry}")).exec()?;
    }
    Ok(lua)
}

fn about(plugin: &Plugin) -> [&str; 3] {
    [&plugin.manifest.id, &plugin.manifest.name, &plugin.manifest.version]
}

/// A job's failure as the host reports it.
fn failed(plugin: &Plugin, sh: &Shared, e: mlua::Error) -> PluginError {
    if let Some(budget) = sh.budget.filter(|_| sh.out_of_time() && !sh.host.cancelled()) {
        return PluginError::Script {
            plugin: plugin.manifest.name.clone(),
            message: format!(
                "it ran out of time: it may take {} seconds here, and was stopped",
                budget.as_secs_f32()
            ),
        };
    }
    failure(&plugin.manifest.name, &sh.host, e)
}

fn failure(name: &str, host: &Rc<dyn Host>, e: mlua::Error) -> PluginError {
    if host.cancelled() {
        return PluginError::Canceled(name.to_string());
    }
    let message = match &e {
        mlua::Error::MemoryError(_) => {
            format!("it ran out of the {} MB a plugin may use", MEMORY_LIMIT >> 20)
        }
        other => other.to_string(),
    };
    PluginError::Script { plugin: name.to_string(), message }
}

fn shared(plugin: &Plugin, input: Input, host: &Rc<dyn Host>) -> Rc<Shared> {
    let sh = shared_in(source_of(plugin), &plugin.manifest.name, input, host);
    // Each declared setting has a value: the host's, else its default.
    {
        let mut settings = sh.settings.borrow_mut();
        for d in &plugin.manifest.settings {
            settings.values.entry(d.id.clone()).or_insert_with(|| d.default.clone());
        }
        settings.values.retain(|k, _| plugin.setting(k).is_some());
    }
    *sh.decls.borrow_mut() = plugin.manifest.settings.clone();
    sh
}

fn shared_in(files: Source, who: &str, input: Input, host: &Rc<dyn Host>) -> Rc<Shared> {
    let game = input.game.as_deref().map_or(CODEPAGE, |g| g.codepage());
    GAME_CODEPAGE.with(|c| c.set(game.for_language(mg_core::Language::ENGLISH)));
    let deadline = input.budget.map(|b| std::time::Instant::now() + b);
    Rc::new(Shared {
        budget: input.budget,
        deadline,
        files,
        ws: RefCell::new(Workspace::new(input.module)),
        edits: RefCell::new(Vec::new()),
        haks: RefCell::new(Vec::new()),
        folders: RefCell::new(Vec::new()),
        game: input.game,
        host: host.clone(),
        who: who.to_string(),
        rng: RefCell::new(fastrand::Rng::new()),
        selection: input.selection,
        decls: RefCell::default(),
        settings: RefCell::new(input.settings.unwrap_or_default()),
        settings_set: RefCell::default(),
        no_set: std::cell::Cell::new(None),
    })
}

fn handler(lua: &Lua, registry: &str, kind: &str, id: &str) -> mlua::Result<Function> {
    let handlers: Table = lua.named_registry_value(registry)?;
    match handlers.get::<Option<Function>>(id)? {
        Some(f) => Ok(f),
        None => fail(format!(
            "the manifest declares the {kind} {id:?}, but the code registers none (mg.{kind}({id:?}, …))"
        )),
    }
}

pub(crate) fn run_command(
    plugin: &Plugin,
    decl: &CommandDecl,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let sh = shared(plugin, input, &host);
    let run = || -> mlua::Result<String> {
        let lua = start(Some(&plugin.manifest.entry), &sh)?;
        let f = handler(&lua, "mg.commands", "command", &decl.id)?;
        let made: LuaValue = f.call(context(&lua, &sh, about(plugin))?)?;
        // What it calls the change: `return { label = … }`, or its title.
        let label = match &made {
            LuaValue::Table(t) => t.get::<Option<String>>("label")?,
            LuaValue::String(s) => Some(s.to_str()?.to_string()),
            _ => None,
        };
        Ok(label.unwrap_or_else(|| decl.title.clone()))
    };
    let label = run().map_err(|e| failed(plugin, &sh, e))?;
    if host.cancelled() {
        return Err(PluginError::Canceled(plugin.manifest.name.clone()));
    }
    Ok(Outcome {
        label,
        edits: sh.edits.take(),
        haks: sh.haks.take(),
        folders: sh.folders.take(),
        settings: sh.settings_set.take(),
        said: Vec::new(),
    })
}

/// The names of `ctx.event`, as the reference writes them.
pub const EVENT_NAMES: [&str; 14] = [
    "ctx.event",
    "ctx.event.name",
    "ctx.event.resources",
    "ctx.event.resource",
    "ctx.event.kind",
    "ctx.event.old",
    "ctx.event.new",
    "ctx.event.changed",
    "ctx.event.made",
    "ctx.event.objects",
    "ctx.event.area",
    "ctx.event.by",
    "ctx.event:warn",
    "ctx.event:refuse",
];

/// What a resource is, as `ctx.event.kind` (and a selection's `kind`)
/// names it: `"area"`, `"creature"`…, else its type (`"nss"`).
fn kind_of(key: &ResKey) -> String {
    match key.restype {
        mg_core::ResType::ARE => Selected::Area(key.resref).kind(),
        t if mg_rules::ObjectKind::from_restype(t).is_some() => Selected::Blueprint(*key).kind(),
        _ => Selected::Resource(*key).kind(),
    }
}

/// A resource's bytes as the plugin reads them in `ctx.event.old` and
/// `new`: a GFF as plain values (as `ctx.module:gff`), another as text
/// (as `ctx.module:text`); nil for none, or a GFF that does not read.
fn value_of(lua: &Lua, key: &ResKey, bytes: Option<&[u8]>) -> mlua::Result<LuaValue> {
    let Some(bytes) = bytes else { return Ok(LuaValue::Nil) };
    if key.restype.is_gff() {
        return match Gff::read(bytes) {
            Ok(g) => Ok(LuaValue::Table(plain(lua, &g.root, &GffPath::root())?)),
            Err(_) => Ok(LuaValue::Nil),
        };
    }
    Ok(LuaValue::String(lua.create_string(codepage().decode(bytes).as_bytes())?))
}

/// The most lines `ctx.event.changed` has.
const CHANGED_LINES: usize = 100;

/// `ctx.event` for one call of an event's handler: what happened, and for
/// a resource (`key`) the resource; for a resource saving, its `warn` and
/// `refuse`, which keep what it says in `said`.
#[allow(clippy::too_many_arguments)]
fn event_table(
    lua: &Lua,
    sh: &Rc<Shared>,
    happened: &crate::Event,
    key: Option<ResKey>,
    objects: &[Selected],
    said: &Rc<RefCell<Vec<crate::Said>>>,
    records: &Rc<RefCell<Vec<FixRecord>>>,
) -> mlua::Result<Table> {
    let event = lua.create_table()?;
    let kind = happened.kind;
    event.set("name", kind.name())?;
    let names: Vec<String> = happened.resources.iter().map(ToString::to_string).collect();
    match kind {
        crate::EventKind::ModuleSaved
        | crate::EventKind::ResourceSaving
        | crate::EventKind::ResourceSaved => event.set("resources", names)?,
        crate::EventKind::ResourceCreated => event.set("made", names)?,
        _ => {}
    }
    if !happened.by.is_empty() {
        event.set("by", happened.by.as_str())?;
    }
    if !objects.is_empty() {
        let list = lua.create_table()?;
        for (i, o) in objects.iter().enumerate() {
            list.raw_seti(i + 1, selected(lua, sh, o)?)?;
        }
        event.set("objects", list)?;
        if let Some(Selected::Object { area, .. }) = objects.first() {
            event.set("area", area.to_string())?;
        }
    }
    let Some(key) = key else { return Ok(event) };
    event.set("resource", key.to_string())?;
    event.set("kind", kind_of(&key))?;
    if kind.takes_types() {
        // As it was, and as it is (or will be): read when first asked for.
        let before = happened.before.get(&key).cloned();
        let s = sh.clone();
        let meta = lua.create_table()?;
        meta.set(
            "__index",
            lua.create_function(move |lua, (t, name): (Table, String)| {
                let now = || -> Option<Vec<u8>> {
                    let mut ws = s.ws.borrow_mut();
                    ws.flush().ok()?;
                    ws.module.get(&key).map(<[u8]>::to_vec)
                };
                let v = match name.as_str() {
                    "old" => value_of(lua, &key, before.as_deref())?,
                    "new" => value_of(lua, &key, now().as_deref())?,
                    "changed" => {
                        let read = |b: Option<&[u8]>| b.and_then(|b| Gff::read(b).ok());
                        match (
                            key.restype.is_gff(),
                            read(before.as_deref()),
                            read(now().as_deref()),
                        ) {
                            (true, Some(a), Some(b)) => {
                                let lines = mg_gff::diff(&a, &b, CHANGED_LINES);
                                LuaValue::Table(lua.create_sequence_from(lines)?)
                            }
                            _ => LuaValue::Nil,
                        }
                    }
                    _ => return Ok(LuaValue::Nil),
                };
                t.raw_set(name, v.clone())?;
                Ok(v)
            })?,
        )?;
        event.set_metatable(Some(meta))?;
    }
    if kind == crate::EventKind::ResourceSaving {
        for (name, refused) in [("warn", false), ("refuse", true)] {
            let (said, records) = (said.clone(), records.clone());
            event.set(
                name,
                lua.create_function(move |_, (_, text, fix): (This, String, LuaValue)| {
                    if text.trim().is_empty() {
                        return fail(format!("ctx.event:{name}(text): say why"));
                    }
                    let what = format!("ctx.event:{name}'s fix");
                    let fix = fix_of(&records, fix, &what)?;
                    said.borrow_mut().push(crate::Said { resource: key, text, refused, fix });
                    Ok(())
                })?,
            )?;
        }
    }
    Ok(event)
}

/// What a plugin does on an event: its handler, called as a command's is,
/// with what happened as `ctx.event`: once, or for each resource (of a
/// save, or made) its manifest takes, in one job. Around a save it only
/// reads: a resource saving's handler may warn, refuse and offer fixes.
pub(crate) fn run_event(
    plugin: &Plugin,
    decl: &crate::EventDecl,
    happened: &crate::Event,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let kind = happened.kind;
    let keys: Vec<ResKey> = match kind.per_resource() {
        true => happened.resources.iter().filter(|k| decl.is_for(k)).copied().collect(),
        false => Vec::new(),
    };
    let objects: Vec<Selected> = (happened.objects.iter())
        .filter(|o| match o {
            Selected::Object { kind, .. } => decl.is_for_kind(*kind),
            _ => true,
        })
        .cloned()
        .collect();
    let objects_event =
        matches!(kind, crate::EventKind::ObjectPlaced | crate::EventKind::ObjectPasted);
    let none = |plugin: &Plugin| Outcome {
        label: format!("{}: {}", plugin.manifest.name, decl.id),
        edits: Vec::new(),
        haks: Vec::new(),
        folders: Vec::new(),
        settings: Vec::new(),
        said: Vec::new(),
    };
    // (Nothing it is for: its code does not run.)
    if (kind.per_resource() && keys.is_empty()) || (objects_event && objects.is_empty()) {
        return Ok(none(plugin));
    }
    let sh = shared(plugin, input, &host);
    if kind == crate::EventKind::ResourceSaving {
        sh.no_set
            .set(Some("a resource saving's handler only reads: ctx.settings:set is not for it"));
    }
    let records: Rc<RefCell<Vec<FixRecord>>> = Rc::default();
    let said: Rc<RefCell<Vec<crate::Said>>> = Rc::default();
    let label_of = |made: LuaValue| -> mlua::Result<Option<String>> {
        Ok(match made {
            LuaValue::Table(t) => t.get::<Option<String>>("label")?,
            LuaValue::String(s) => Some(s.to_str()?.to_string()),
            _ => None,
        })
    };
    let run = || -> mlua::Result<Option<String>> {
        let lua = start(Some(&plugin.manifest.entry), &sh)?;
        let f = handler(&lua, "mg.events", "event", &decl.id)?;
        let ctx = context(&lua, &sh, about(plugin))?;
        if kind == crate::EventKind::ResourceSaving {
            ctx.set("fix", fix_maker(&lua, &sh, &records)?)?;
        }
        if !kind.per_resource() {
            let event = event_table(&lua, &sh, happened, None, &objects, &said, &records)?;
            ctx.set("event", event)?;
            return label_of(f.call::<LuaValue>(ctx)?);
        }
        let mut label = None;
        for key in &keys {
            let event = event_table(&lua, &sh, happened, Some(*key), &objects, &said, &records)?;
            ctx.set("event", event)?;
            label = label_of(f.call::<LuaValue>(ctx.clone())?)?.or(label);
        }
        Ok(label)
    };
    let label = run().map_err(|e| failed(plugin, &sh, e))?;
    if host.cancelled() {
        return Err(PluginError::Canceled(plugin.manifest.name.clone()));
    }
    let refuse = |message: String| {
        Err(PluginError::Script { plugin: plugin.manifest.name.clone(), message })
    };
    if !kind.edits() && !sh.edits.borrow().is_empty() {
        return refuse(format!(
            "its {} handler {:?} edits the module: around a save a plugin only reads (it may \
             give a warning a fix, ctx.fix, which the user makes)",
            kind.name(),
            decl.id
        ));
    }
    let wrote = !sh.haks.borrow().is_empty() || !sh.folders.borrow().is_empty();
    if kind.at_once() && wrote {
        return refuse(format!(
            "its {} handler {:?} writes outside the module: a handler that runs at once, \
             within what the user does, can't (nobody is asked)",
            kind.name(),
            decl.id
        ));
    }
    let label = label.unwrap_or_else(|| format!("{}: {}", plugin.manifest.name, decl.id));
    Ok(Outcome {
        label,
        edits: sh.edits.take(),
        haks: sh.haks.take(),
        folders: sh.folders.take(),
        settings: sh.settings_set.take(),
        said: said.take(),
    })
}

/// A fix a check records (`ctx.fix`): what its button says, and the job
/// its edits go into, made at its first edit on a copy of the module as
/// the check reads it (so that the check goes on reading the module as it
/// is, and each fix starts from it).
struct FixRecord {
    label: String,
    job: Option<Rc<Shared>>,
}

/// The methods of a fix: those of `ctx.edit`, as the reference writes them.
pub(crate) const FIX_NAMES: [&str; 10] = [
    "ctx.fix",
    "fix.label",
    "fix:set",
    "fix:remove",
    "fix:insert",
    "fix:remove_item",
    "fix:write",
    "fix:write_bytes",
    "fix:write_gff",
    "fix:delete",
];

/// `ctx.fix(label)`: a fix to give a finding, which records the edits made
/// through it (its methods are `ctx.edit`'s) and makes none.
fn fix_maker(
    lua: &Lua,
    sh: &Rc<Shared>,
    records: &Rc<RefCell<Vec<FixRecord>>>,
) -> mlua::Result<Function> {
    let (sh, records) = (sh.clone(), records.clone());
    lua.create_function(move |lua, label: String| {
        if label.trim().is_empty() {
            return fail("ctx.fix(label): the label is what its button says, and needs words");
        }
        let n = records.borrow().len();
        records.borrow_mut().push(FixRecord { label: label.clone(), job: None });
        let fix = lua.create_table()?;
        fix.set("label", label)?;
        let meta = lua.create_table()?;
        meta.set("__fix", n)?;
        // Its methods are made at the first one used (a copy of the module
        // for each fix that edits).
        let (sh, records) = (sh.clone(), records.clone());
        meta.set(
            "__index",
            lua.create_function(move |lua, (fix, name): (Table, LuaValue)| {
                let job = {
                    let mut all = records.borrow_mut();
                    let record = &mut all[n];
                    record
                        .job
                        .get_or_insert_with(|| {
                            let module = sh.ws.borrow().module.clone();
                            let input = Input::new(module, sh.game.clone());
                            shared_in(Source::Nothing, &sh.who, input, &sh.host)
                        })
                        .clone()
                };
                let methods = editor(lua, &job)?;
                for pair in methods.pairs::<String, Function>() {
                    let (k, f) = pair?;
                    fix.raw_set(k, f)?;
                }
                fix.raw_get::<LuaValue>(name)
            })?,
        )?;
        fix.set_metatable(Some(meta))?;
        Ok(fix)
    })
}

/// A fix given with a finding or a warning: one made with `ctx.fix`, and
/// only if it edits (`what`: what it was given as, for the error).
fn fix_of(
    records: &RefCell<Vec<FixRecord>>,
    given: LuaValue,
    what: &str,
) -> mlua::Result<Option<Fix>> {
    match given {
        LuaValue::Nil => Ok(None),
        LuaValue::Table(t) => {
            let n = match t.metatable() {
                Some(meta) => meta.raw_get::<Option<usize>>("__fix")?,
                None => None,
            };
            let all = records.borrow();
            let Some(record) = n.and_then(|n| all.get(n)) else {
                return fail(format!("{what} is one made with ctx.fix(label)"));
            };
            let edits = record.job.as_ref().map(|j| j.edits.borrow().clone());
            Ok(edits
                .filter(|e| !e.is_empty())
                .map(|edits| Fix { label: record.label.clone(), edits }))
        }
        _ => fail(format!("{what} is one made with ctx.fix(label)")),
    }
}

pub(crate) fn run_check(
    plugin: &Plugin,
    decl: &CheckDecl,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Vec<Found>, PluginError> {
    let sh = shared(plugin, input, &host);
    sh.no_set.set(Some("a check only reads: ctx.settings:set is for commands and events"));
    let records: Rc<RefCell<Vec<FixRecord>>> = Rc::default();
    let run = || -> mlua::Result<Vec<Found>> {
        let lua = start(Some(&plugin.manifest.entry), &sh)?;
        let f = handler(&lua, "mg.checks", "check", &decl.id)?;
        let ctx = context(&lua, &sh, about(plugin))?;
        ctx.set("fix", fix_maker(&lua, &sh, &records)?)?;
        let made: LuaValue = f.call(ctx)?;
        let list = match made {
            LuaValue::Nil => return Ok(Vec::new()),
            LuaValue::Table(t) => t,
            _ => return fail("a check returns a list of findings"),
        };
        let mut out = Vec::new();
        for finding in list.sequence_values::<Table>() {
            let f = finding?;
            let severity = match f.get::<Option<String>>("severity")?.as_deref() {
                None => decl.severity,
                Some("error") => Severity::Error,
                Some("warning") => Severity::Warning,
                Some(other) => {
                    return fail(format!(
                        "a finding's severity is \"warning\" or \"error\", not {other:?}"
                    ));
                }
            };
            // Its fix: one made with ctx.fix, and only if it edits.
            let fix = fix_of(&records, f.get::<LuaValue>("fix")?, "a finding's fix")?;
            let finding = Finding {
                severity,
                check: plugin.check_id(&decl.id).into(),
                source: "module".into(),
                resource: key_of(&f.get::<String>("resource")?)?,
                at: f.get::<Option<String>>("at")?.unwrap_or_default(),
                message: f.get("message")?,
            };
            out.push(Found { finding, fix });
        }
        Ok(out)
    };
    let findings = run().map_err(|e| failed(plugin, &sh, e))?;
    let wrote = !sh.haks.borrow().is_empty() || !sh.folders.borrow().is_empty();
    if !sh.edits.borrow().is_empty() || wrote {
        return Err(PluginError::Script {
            plugin: plugin.manifest.name.clone(),
            message: format!("the check {:?} edits the module: a check only reads", decl.id),
        });
    }
    Ok(findings)
}

/// Loads a plugin's code from `files` and compares what it registers with
/// what `manifest` declares.
pub(crate) fn inspect(
    files: Source,
    manifest: &Manifest,
    host: Rc<dyn Host>,
) -> Result<Vec<String>, PluginError> {
    let input = Input::new(mg_module::Module::new(), None);
    let sh = shared_in(files, &manifest.name, input, &host);
    let run = || -> mlua::Result<Vec<String>> {
        let lua = start(Some(&manifest.entry), &sh)?;
        let mut faults = Vec::new();
        let registered = |registry: &str| -> mlua::Result<Vec<String>> {
            let handlers: Table = lua.named_registry_value(registry)?;
            handlers.pairs::<String, LuaValue>().map(|p| p.map(|(id, _)| id)).collect()
        };
        let declared: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
            ("command", manifest.commands.iter().map(|c| c.id.as_str()).collect()),
            ("check", manifest.checks.iter().map(|c| c.id.as_str()).collect()),
            ("event", manifest.events.iter().map(|c| c.id.as_str()).collect()),
        ]);
        for (kind, registry) in
            [("command", "mg.commands"), ("check", "mg.checks"), ("event", "mg.events")]
        {
            let mut code = registered(registry)?;
            code.sort();
            for id in &declared[kind] {
                if !code.iter().any(|c| c == id) {
                    faults.push(format!(
                        "the {kind} {id:?} is declared, but the code registers none"
                    ));
                }
            }
            for id in &code {
                if !declared[kind].contains(&id.as_str()) {
                    faults.push(format!(
                        "the code registers the {kind} {id:?}, which the manifest lacks"
                    ));
                }
            }
        }
        Ok(faults)
    };
    run().map_err(|e| failure(&manifest.name, &host, e))
}

/// A value as the console shows it: text and numbers as they are, a table
/// with its entries (to a depth, and not without end).
fn shown(v: &LuaValue, depth: usize, out: &mut String) {
    match v {
        LuaValue::String(s) => out.push_str(&format!("{:?}", s.to_string_lossy())),
        LuaValue::Table(t) if depth < 4 => {
            out.push('{');
            for (n, pair) in t.pairs::<LuaValue, LuaValue>().enumerate() {
                let Ok((k, v)) = pair else { break };
                if n > 0 {
                    out.push_str(", ");
                }
                if n == 40 {
                    out.push('…');
                    break;
                }
                match &k {
                    LuaValue::Integer(_) | LuaValue::Number(_) => {}
                    LuaValue::String(s) => out.push_str(&format!("{} = ", s.to_string_lossy())),
                    other => out.push_str(&format!("[{}] = ", other.type_name())),
                }
                shown(&v, depth + 1, out);
            }
            out.push('}');
        }
        LuaValue::Table(_) => out.push_str("{…}"),
        LuaValue::Nil => out.push_str("nil"),
        LuaValue::Boolean(b) => out.push_str(&b.to_string()),
        LuaValue::Integer(i) => out.push_str(&i.to_string()),
        LuaValue::Number(n) => out.push_str(&n.to_string()),
        other => out.push_str(other.type_name()),
    }
}

/// Code typed into the console: run with `mg` and `ctx` at hand, what it
/// returns shown in the log, its edits handed back like a command's.
pub(crate) fn run_console(
    code: &str,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let sh = shared_in(Source::Nothing, "The plugin console", input, &host);
    sh.no_set.set(Some("the console is no plugin's, and has no settings"));
    let run = || -> mlua::Result<()> {
        let lua = start(None, &sh)?;
        let ctx = context(&lua, &sh, ["console", "Console", crate::API])?;
        let source = format!("local mg = require(\"@moonglow\")\nlocal ctx = ...\n{code}");
        let made: mlua::MultiValue =
            lua.load(source).set_mode(TEXT).set_name("=console").call(ctx)?;
        for v in made {
            let mut text = String::new();
            shown(&v, 0, &mut text);
            sh.host.log(Level::Info, &text);
        }
        Ok(())
    };
    run().map_err(|e| failure("Console", &host, e))?;
    if host.cancelled() {
        return Err(PluginError::Canceled("Console".into()));
    }
    Ok(Outcome {
        label: "Plugin console".into(),
        edits: sh.edits.take(),
        haks: sh.haks.take(),
        folders: sh.folders.take(),
        settings: Vec::new(),
        said: Vec::new(),
    })
}

/// Every name a plugin's code can use, as the reference writes them
/// (`mg.command`, `ctx.module:gff`, `ctx.plugin.id`): read off the API as
/// it is built, so that a test can hold the reference to list them all.
pub(crate) fn api_names() -> Vec<String> {
    struct Nobody;
    impl Host for Nobody {
        fn log(&self, _: Level, _: &str) {}
        fn ask(&self, _: &Question) -> Option<Answer> {
            None
        }
    }
    let host: Rc<dyn Host> = Rc::new(Nobody);
    let input = Input::new(mg_module::Module::new(), None);
    let sh = shared_in(Source::Nothing, "", input, &host);
    let names = || -> mlua::Result<Vec<String>> {
        let lua = start(None, &sh)?;
        let mut out = vec!["require".to_string(), "print".to_string()];
        let mg: Table = lua.named_registry_value("mg.api")?;
        for pair in mg.pairs::<String, LuaValue>() {
            out.push(format!("mg.{}", pair?.0));
        }
        let ctx = context(&lua, &sh, ["", "", ""])?;
        for pair in ctx.pairs::<String, LuaValue>() {
            let (name, value) = pair?;
            match value {
                LuaValue::Table(t) => {
                    for inner in t.pairs::<String, LuaValue>() {
                        let (inner, value) = inner?;
                        let sep = if value.is_function() { ':' } else { '.' };
                        out.push(format!("ctx.{name}{sep}{inner}"));
                    }
                }
                _ => out.push(format!("ctx.{name}")),
            }
        }
        // What is selected and open (an empty list here, and nothing
        // open), and the fields of each.
        out.extend(["ctx.selection", "ctx.open"].map(String::from));
        // The settings (none here) and their method.
        out.extend(["ctx.settings", "ctx.settings:set"].map(String::from));
        out.extend(SELECTED.map(String::from));
        // A check's fixes (`ctx.fix`, a check's and a resource saving's)
        // and theirs.
        out.extend(FIX_NAMES.map(String::from));
        // What happened, for an event's handler.
        out.extend(EVENT_NAMES.map(String::from));
        // What `ctx.game:table` returns, and the other things handed out.
        out.extend(["twoda.rows", "twoda.columns", "twoda:get"].map(String::from));
        out.extend(crate::files::NAMES.map(String::from));
        out.extend(crate::tables::NAMES.map(String::from));
        out.extend(crate::terrain::NAMES.map(String::from));
        out.sort();
        Ok(out)
    };
    names().unwrap_or_default()
}
