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

/// What a job makes the host hold for it, outside its machine's own
/// memory (which Luau counts, to [`MEMORY_LIMIT`]): the tables and talk
/// tables it makes, pictures decoded, edits and files waiting to be
/// written. Counted in bytes, roughly, and held to [`crate::HELD_LIMIT`]:
/// a call that would go past it fails, before it takes the memory.
#[derive(Default)]
pub(crate) struct Budget {
    used: std::cell::Cell<usize>,
    /// The job's machine: what its code let go of is the host's until
    /// the machine collects it, so it collects before a call is refused.
    lua: RefCell<Option<mlua::WeakLua>>,
}

/// What a job is told that made the host hold too much.
const HELD_TOO_MUCH: &str = "held";

impl Budget {
    /// `bytes` more, for as long as the job lasts.
    pub(crate) fn keep(&self, bytes: usize) -> mlua::Result<()> {
        let fits = || self.used.get().saturating_add(bytes) <= crate::HELD_LIMIT;
        if !fits() {
            let lua = self.lua.borrow().as_ref().and_then(mlua::WeakLua::try_upgrade);
            if let Some(lua) = lua {
                let _ = lua.gc_collect();
            }
        }
        if !fits() {
            return Err(mlua::Error::MemoryError(HELD_TOO_MUCH.into()));
        }
        self.used.set(self.used.get() + bytes);
        Ok(())
    }

    /// `bytes` no longer held (a file written again, the one before it).
    pub(crate) fn give_back(&self, bytes: usize) {
        self.used.set(self.used.get().saturating_sub(bytes));
    }

    /// `bytes` held for as long as what is returned lives (a table's, in
    /// the functions the plugin has of it).
    pub(crate) fn hold(self: &Rc<Self>, bytes: usize) -> mlua::Result<Charge> {
        self.keep(bytes)?;
        Ok(Charge { of: self.clone(), bytes: std::cell::Cell::new(bytes) })
    }
}

/// Bytes of a job's [`Budget`], given back when this is dropped.
pub(crate) struct Charge {
    of: Rc<Budget>,
    bytes: std::cell::Cell<usize>,
}

impl Charge {
    /// `bytes` more (what it stands for grew).
    pub(crate) fn more(&self, bytes: usize) -> mlua::Result<()> {
        self.of.keep(bytes)?;
        self.bytes.set(self.bytes.get() + bytes);
        Ok(())
    }
}

impl Drop for Charge {
    fn drop(&mut self) {
        self.of.give_back(self.bytes.get());
    }
}

/// Roughly the bytes a field's value takes.
fn weight(v: &Value) -> usize {
    32 + match v {
        Value::String(b) | Value::ResRef(b) | Value::Void(b) => b.len(),
        Value::LocString(s) => s.strings.iter().map(|(_, b)| 16 + b.len()).sum(),
        Value::Struct(s) => weight_of(s),
        Value::List(items) => items.iter().map(weight_of).sum(),
        _ => 0,
    }
}

/// Roughly the bytes a struct takes.
fn weight_of(s: &Struct) -> usize {
    32 + s.fields.iter().map(|f| 24 + weight(&f.value)).sum::<usize>()
}

/// Roughly the bytes an edit takes.
fn edit_weight(e: &Edit) -> usize {
    64 + match e {
        Edit::SetField { value, .. } => value.as_ref().map_or(0, weight),
        Edit::InsertItem { item, .. } => weight_of(item),
        Edit::RemoveItem { .. } => 0,
        Edit::SetResource { data, .. } => data.as_ref().map_or(0, Vec::len),
    }
}

/// What a job's API works on.
pub(crate) struct Shared {
    files: Source,
    pub(crate) ws: RefCell<Workspace>,
    pub(crate) edits: RefCell<Vec<Edit>>,
    /// What the job wants written into haks, each hak once.
    pub(crate) haks: RefCell<Vec<HakWrite>>,
    /// How many bytes of it are held in memory ([`crate::HELD_IN_MEMORY`]),
    /// and the scratch files of those past them.
    pub(crate) held_bytes: std::cell::Cell<usize>,
    pub(crate) held: RefCell<Option<Arc<crate::Held>>>,
    /// What the job makes the host hold for it.
    pub(crate) budget_held: Rc<Budget>,
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
    /// The values of the panel's fields the command was run from
    /// (`ctx.values`; API 0.9).
    values: std::collections::BTreeMap<String, serde_json::Value>,
    /// The plugin's settings, their values as the job goes (what it set
    /// read back), and what it set.
    decls: RefCell<Vec<crate::SettingDecl>>,
    settings: RefCell<crate::Settings>,
    pub(crate) settings_set: RefCell<Vec<crate::SettingWrite>>,
    /// Why `ctx.settings:set` is refused in this job (a check's), if it is.
    pub(crate) no_set: std::cell::Cell<Option<&'static str>>,
    /// How long the job may run ([`Input::budget`]), and when it must
    /// have ended.
    budget: Option<std::time::Duration>,
    deadline: std::cell::Cell<Option<std::time::Instant>>,
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
        self.deadline.get().is_some_and(|d| std::time::Instant::now() > d)
    }

    /// Whether the job is to stop: canceled, or out of time.
    pub(crate) fn stopped(&self) -> bool {
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
        // (Held twice: in the job's copy of the module, to be read back
        // and undone, and for the host.)
        self.budget_held.keep(2 * edits.iter().map(edit_weight).sum::<usize>())?;
        let applied = self.ws.borrow_mut().apply(Command::new("plugin", edits.clone()));
        match applied {
            Ok(()) => {
                self.edits.borrow_mut().extend(edits);
                Ok(())
            }
            Err(e) => fail(e),
        }
    }

    /// What `read` makes of a resource the job has put into a hak (the
    /// last one written, of the hak written to last), where it is held:
    /// its bytes, or the file of the user's it is a copy of, which is not
    /// read for it.
    fn in_haks<R>(&self, key: &ResKey, read: impl FnOnce(&crate::HakData) -> R) -> Option<R> {
        let haks = self.haks.borrow();
        let found = haks.iter().rev().find_map(|h| h.files.iter().rev().find(|(k, _)| k == key))?;
        match &found.1 {
            crate::HakData::Archive => None,
            data => Some(read(data)),
        }
    }

    /// Whether the job has put the resource into a hak.
    fn is_in_haks(&self, key: &ResKey) -> bool {
        self.in_haks(key, |_| ()).is_some()
    }

    /// A resource's bytes: the module's (with the job's edits), or with
    /// `game` what the game would load (what the job has put into haks
    /// first).
    pub(crate) fn bytes(&self, key: &ResKey, game: bool) -> mlua::Result<Vec<u8>> {
        if game {
            // (A file of the user's is held to what a plugin reads of one.)
            let put = self.in_haks(key, |data| match data {
                crate::HakData::File(path) => crate::files::read(path),
                crate::HakData::Bytes(bytes) => Ok(bytes.to_vec()),
                crate::HakData::Archive => unreachable!("not handed on"),
            });
            if let Some(bytes) = put {
                return bytes;
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

    /// A resource's bytes as one layer of the game's load order has them
    /// (by its label: `hak:castle`, `key:nwn_base`), overridden or not.
    pub(crate) fn layer_bytes(&self, key: &ResKey, layer: &str) -> mlua::Result<Vec<u8>> {
        let Some(data) = &self.game else {
            return fail(format!("{key}: there is no game data (no game install)"));
        };
        let rm = &data.resman;
        let Some(i) = rm.layers().iter().position(|l| l.label == layer) else {
            let labels: Vec<&str> = rm.layers().iter().map(|l| l.label.as_str()).collect();
            return fail(format!(
                "{layer:?} is not a layer of the game's load order ({})",
                labels.join(", ")
            ));
        };
        match rm.get_in(i, key) {
            Ok(bytes) => Ok(bytes.into_owned()),
            Err(_) => fail(format!("{layer} has no {key}")),
        }
    }

    /// What `read` makes of a GFF resource of the module, read where the
    /// job's copy has it (not copied: an area's GIT is megabytes, and an
    /// entry of the selection reads two fields of it).
    fn read_gff<R>(&self, key: &ResKey, read: impl FnOnce(&Gff) -> R) -> Option<R> {
        self.ws.borrow_mut().doc(key).ok().map(read)
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
            // (Its bytes in the module's codepage, as a field read gives
            // them: not turned back into text, which a letter outside
            // ASCII does not survive.)
            let bytes = text_of(v, ty)?;
            let r = ResRef::from_bytes(&bytes).map_err(|e| {
                format!("{:?} is not a resource name: {e}", codepage().decode(&bytes))
            })?;
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

/// The layer a read of `ctx.game` names (`ctx.game:bytes(name, layer)`),
/// which `ctx.module`'s reads have none of.
fn layer_of(game: bool, layer: Option<String>) -> mlua::Result<Option<String>> {
    match layer {
        Some(_) if !game => fail("ctx.module has no layers: a layer's copy is ctx.game's"),
        layer => Ok(layer),
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
                s.is_in_haks(&key) || s.game.as_ref().is_some_and(|g| g.resman.contains(&key))
            } else {
                s.ws.borrow().module.contains(&key)
            })
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "gff",
        lua.create_function(move |lua, (_, name, layer): (This, String, Option<String>)| {
            let key = key_of(&name)?;
            let gff = match layer_of(game, layer)? {
                Some(layer) => {
                    let bytes = s.layer_bytes(&key, &layer)?;
                    Gff::read(&bytes).or_else(|e| fail(format!("{key}: {e}")))?
                }
                None => s.gff(&key, game)?,
            };
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
        lua.create_function(move |lua, (_, name, layer): (This, String, Option<String>)| {
            let key = key_of(&name)?;
            let bytes = match layer_of(game, layer)? {
                Some(layer) => s.layer_bytes(&key, &layer)?,
                None => s.bytes(&key, game)?,
            };
            lua.create_string(codepage().decode(&bytes).as_bytes())
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "bytes",
        lua.create_function(move |lua, (_, name, layer): (This, String, Option<String>)| {
            let key = key_of(&name)?;
            let bytes = match layer_of(game, layer)? {
                Some(layer) => s.layer_bytes(&key, &layer)?,
                None => s.bytes(&key, game)?,
            };
            lua.create_string(bytes)
        })?,
    )?;
    if !game {
        // A name for a new resource, as the window proposes one: the
        // builder's code where the team's pattern puts it, the first free.
        let s = sh.clone();
        t.set(
            "new_name",
            lua.create_function(move |_, (_, name, ext): (This, String, String)| {
                use mg_core::ResType;
                let Some(restype) = ResType::from_extension(ext.trim_start_matches('.')) else {
                    return fail(format!("{ext:?} is not a kind of resource (utc, dlg, are…)"));
                };
                let ws = s.ws.borrow();
                let m = &ws.module;
                let naming = m.project_file.naming_for(restype);
                let taken = |r: &ResRef| m.contains(&ResKey::new(*r, restype));
                // (An area's as the Area Wizard's, the others' as a
                // blueprint wizard's.)
                let resref = match restype {
                    ResType::ARE => mg_module::new::resref_for_named(&name, &naming, taken),
                    _ => mg_module::blueprints::resref_named(&name, &naming, taken),
                };
                Ok(ResKey::new(resref, restype).to_string())
            })?,
        )?;
    }
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
                let file = ResKey::from_filename(&file).and_then(|k| data.resman.get(&k).ok());
                let file = file.map(|f| f.into_owned());
                crate::tables::twoda(lua, &s.budget_held, table.clone(), file)
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
pub(crate) fn form_fields(fields: Table) -> mlua::Result<Vec<FormField>> {
    let mut out = Vec::new();
    for field in fields.sequence_values::<Table>() {
        let f = field?;
        let kind = f.get::<Option<String>>("type")?;
        let field = form_field(&f, kind.as_deref().unwrap_or("text"), "default")?;
        if out.iter().any(|o: &FormField| o.id == field.id) {
            return fail(format!("a form has two fields {:?}", field.id));
        }
        out.push(field);
    }
    Ok(out)
}

/// A field from its table: a form's (its kind its `type`, what it starts
/// with its `default`), or a panel's (API 0.9: its `kind`, and its
/// `value`). `start` is the key of what it starts with.
pub(crate) fn form_field(f: &Table, kind: &str, start: &str) -> mlua::Result<FormField> {
    let id: String = match f.get::<LuaValue>("id")? {
        LuaValue::String(s) => s.to_str()?.to_string(),
        _ => return fail("a field has an id, its value's key: { id = \"…\", … }"),
    };
    let label = f.get::<Option<String>>("label")?.unwrap_or_else(|| id.clone());
    let kind = match kind {
        "text" => FieldKind::Text { default: f.get::<Option<String>>(start)?.unwrap_or_default() },
        "number" => FieldKind::Number {
            default: f.get::<Option<f64>>(start)?.unwrap_or(0.0),
            min: f.get("min")?,
            max: f.get("max")?,
        },
        "check" => FieldKind::Check { default: f.get::<Option<bool>>(start)?.unwrap_or(false) },
        "choice" => {
            let choices = choices_of(&id, f.get("choices")?)?;
            if choices.is_empty() {
                return fail(format!("the choice {id:?} has no choices"));
            }
            let wanted = f.get::<Option<String>>(start)?;
            let default =
                wanted.and_then(|w| choices.iter().position(|c| c.text == w)).unwrap_or(0);
            FieldKind::Choice { choices, default }
        }
        "color" => {
            let default = match f.get::<Option<String>>(start)? {
                None => [255; 3],
                Some(text) => match crate::color_of(&text) {
                    Some(c) => c,
                    None => {
                        return fail(format!(
                            "the color {id:?}'s {start} is \"#rrggbb\", not {text:?}"
                        ));
                    }
                },
            };
            let variable = f.get::<Option<String>>("variable")?.filter(|v| !v.is_empty());
            FieldKind::Color { default, variable }
        }
        other => {
            return fail(format!(
                "a field is \"text\", \"number\", \"check\", \"choice\" or \"color\", not \
                 {other:?}"
            ));
        }
    };
    Ok(FormField { id, label, kind })
}

/// A choice's options: each its text, or (API 0.9) `{ text = "Crimson",
/// color = "#a01010" }`, the color of the swatch drawn before it.
fn choices_of(id: &str, given: Option<Table>) -> mlua::Result<Vec<crate::Choice>> {
    let mut out = Vec::new();
    let Some(given) = given else { return Ok(out) };
    let shape = "an option is text, or { text = \"…\", color = \"#rrggbb\" }";
    for c in given.sequence_values::<LuaValue>() {
        out.push(match c? {
            LuaValue::String(s) => crate::Choice::text(s.to_str()?.to_string()),
            LuaValue::Table(t) => {
                let Some(text) = t.get::<Option<String>>("text")? else {
                    return fail(format!("the choice {id:?}: {shape}"));
                };
                let color = match t.get::<Option<String>>("color")? {
                    None => None,
                    Some(c) => match crate::color_of(&c) {
                        Some(c) => Some(c),
                        None => {
                            return fail(format!(
                                "the choice {id:?}: {text:?}'s color is \"#rrggbb\", not {c:?}"
                            ));
                        }
                    },
                };
                crate::Choice { text, color }
            }
            _ => return fail(format!("the choice {id:?}: {shape}")),
        });
    }
    Ok(out)
}

/// The fields of an entry of `ctx.selection` (and of `ctx.open`), as the
/// reference names them.
pub(crate) const SELECTED: [&str; 13] = [
    "selected.kind",
    "selected.resource",
    "selected.path",
    "selected.blueprint",
    "selected.module",
    "selected.area",
    "selected.index",
    "selected.tag",
    "selected.resref",
    "selected.layer",
    "selected.x",
    "selected.y",
    "selected.together",
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
        // (Its column and row, by the area's width.)
        Selected::Tile { area, index } => {
            t.set("area", area.to_string())?;
            t.set("index", *index)?;
            let width = sh.read_gff(&key, |g| g.root.integer("Width")).flatten();
            if let Some(w) = width.filter(|w| *w > 0).map(|w| w as usize) {
                t.set("x", index % w)?;
                t.set("y", index / w)?;
            }
        }
        Selected::Area(area) => t.set("area", area.to_string())?,
        Selected::Copy { layer, .. } => t.set("layer", layer.as_str())?,
        Selected::Blueprint(_) | Selected::Resource(_) => {}
    }
    // (A layer's copy is the game's, whether or not the module has its own.)
    let in_module = s.layer().is_none() && sh.ws.borrow().module.contains(&key);
    t.set("module", in_module)?;
    // Its tag, and what a placed object was made from, where it is.
    let facts = |g: &Gff| {
        let at = path.get(&g.root)?;
        Some((at.string("Tag").map(<[u8]>::to_vec), at.resref("TemplateResRef")))
    };
    let read = match s {
        Selected::Resource(_) => None,
        Selected::Copy { layer, .. } if key.restype.is_gff() => sh
            .layer_bytes(&key, layer)
            .ok()
            .and_then(|b| Gff::read(&b).ok())
            .and_then(|g| facts(&g)),
        Selected::Copy { .. } => None,
        Selected::Blueprint(_) if !in_module => sh.gff(&key, true).ok().and_then(|g| facts(&g)),
        _ => sh.read_gff(&key, facts).flatten(),
    };
    let text = |b: &[u8]| lua.create_string(codepage().decode(b).as_bytes());
    if let Some((tag, made_from)) = read {
        if let Some(tag) = tag {
            t.set("tag", text(&tag)?)?;
        }
        // (What a placed object was made from; else its own name.)
        let resref = match s {
            Selected::Object { .. } => made_from,
            Selected::Tile { .. } => None,
            _ => Some(key.resref),
        };
        if let Some(r) = resref {
            t.set("resref", r.to_string())?;
        }
    } else if !matches!(s, Selected::Object { .. } | Selected::Tile { .. }) {
        t.set("resref", key.resref.to_string())?;
    }
    Ok(t)
}

/// The lines of a text that differ between `old` and `new`, as
/// `ctx.event.changed` gives them: what is between the lines they begin
/// and end with alike, each line taken out (`12: - text`) and put in
/// (`12: + text`) by its number, at most `most`.
pub(crate) fn text_changes(old: &str, new: &str, most: usize) -> Vec<String> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let tail =
        (a[head..].iter().rev()).zip(b[head..].iter().rev()).take_while(|(x, y)| x == y).count();
    let mut out = Vec::new();
    for (i, l) in a[head..a.len() - tail].iter().enumerate() {
        out.push(format!("{}: - {l}", head + i + 1));
    }
    for (i, l) in b[head..b.len() - tail].iter().enumerate() {
        out.push(format!("{}: + {l}", head + i + 1));
    }
    out.truncate(most);
    out
}

/// A setting's value as the plugin reads it.
fn setting_value(lua: &Lua, v: &crate::SettingValue) -> mlua::Result<LuaValue> {
    Ok(match v {
        crate::SettingValue::Text(t) => LuaValue::String(lua.create_string(t)?),
        crate::SettingValue::Int(n) => LuaValue::Integer(*n),
        crate::SettingValue::Bool(b) => LuaValue::Boolean(*b),
        crate::SettingValue::Float(x) => LuaValue::Number(*x),
        crate::SettingValue::List(items) => {
            LuaValue::Table(lua.create_sequence_from(items.iter().map(String::as_str))?)
        }
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
        let float = matches!(d.kind, crate::SettingKind::Float { .. });
        let given = match &value {
            LuaValue::Nil => None,
            LuaValue::String(t) => Some(crate::SettingValue::Text(t.to_str()?.to_string())),
            LuaValue::Boolean(b) => Some(crate::SettingValue::Bool(*b)),
            LuaValue::Integer(n) if float => Some(crate::SettingValue::Float(*n as f64)),
            LuaValue::Integer(n) => Some(crate::SettingValue::Int(*n)),
            LuaValue::Number(n) if float => Some(crate::SettingValue::Float(*n)),
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => {
                Some(crate::SettingValue::Int(*n as i64))
            }
            LuaValue::Number(n) => Some(crate::SettingValue::Float(*n)),
            // (A list: its texts, from 1.)
            LuaValue::Table(t) => {
                let mut items = Vec::new();
                for v in t.clone().sequence_values::<LuaValue>() {
                    match v? {
                        LuaValue::String(s) => items.push(s.to_str()?.to_string()),
                        other => {
                            return fail(format!(
                                "ctx.settings:set({name:?}, …): a list's items are texts, not {}",
                                other.type_name()
                            ));
                        }
                    }
                }
                Some(crate::SettingValue::List(items))
            }
            other => {
                return fail(format!(
                    "ctx.settings:set({name:?}, …): {} is no setting's value (text, a number, \
                     true or false, a list of texts; nil for its default)",
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
pub(crate) fn context(lua: &Lua, sh: &Rc<Shared>, about: [&str; 3]) -> mlua::Result<Table> {
    let began = std::time::Instant::now();
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
    // (Each as a form gives it back; none: an empty table.)
    let values = lua.create_table()?;
    for (k, v) in &sh.values {
        values.set(k.as_str(), lua.to_value(v)?)?;
    }
    ctx.set("values", values)?;
    if let Some(open) = &sh.selection.open {
        let entry = selected(lua, sh, open)?;
        // (Several edited together: each, the first being `open` itself.)
        if !sh.selection.together.is_empty() {
            let together = lua.create_table()?;
            for (i, s) in sh.selection.together.iter().enumerate() {
                together.raw_seti(i + 1, selected(lua, sh, s)?)?;
            }
            entry.set("together", together)?;
        }
        ctx.set("open", entry)?;
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
    // (Making it is the host's work, not time of the job's budget.)
    sh.deadline.set(sh.deadline.get().map(|d| d + began.elapsed()));
    Ok(ctx)
}

/// What `require("@moonglow")` gives: where handlers are registered, and
/// helpers.
fn api(lua: &Lua, sh: &Rc<Shared>) -> mlua::Result<Table> {
    let mg = lua.create_table()?;
    mg.set("api", crate::API)?;
    for (kind, name, registry) in crate::manifest::KINDS {
        lua.set_named_registry_value(registry, lua.create_table()?)?;
        mg.set(
            name,
            lua.create_function(move |lua, (id, handler): (String, LuaValue)| {
                let handlers: Table = lua.named_registry_value(registry)?;
                if handlers.contains_key(id.as_str())? {
                    return fail(format!("the {kind} {id:?} is registered twice"));
                }
                // (A wizard is its pages and its finish; the rest, a
                // function.)
                match (&handler, kind) {
                    (LuaValue::Table(t), "wizard") => {
                        if !t.get::<LuaValue>("pages")?.is_table() {
                            return fail(format!(
                                "mg.wizard({id:?}, {{ pages = {{ … }}, finish = … }}): its pages, \
                                 a list"
                            ));
                        }
                        if !t.get::<LuaValue>("finish")?.is_function() {
                            return fail(format!(
                                "mg.wizard({id:?}, {{ pages = …, finish = function(ctx, answers) \
                                 … end }}): what its Finish does"
                            ));
                        }
                    }
                    (_, "wizard") => {
                        return fail(format!(
                            "mg.wizard({id:?}, {{ pages = {{ … }}, finish = … }}): a table"
                        ));
                    }
                    (LuaValue::Function(_), _) => {}
                    _ => return fail(format!("mg.{name}({id:?}, handler): a function")),
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
    let held = sh.budget_held.clone();
    mg.set("image", lua.create_function(move |lua, args| crate::files::image(lua, &held, args))?)?;
    mg.set("twoda", crate::tables::twoda_api(lua, &sh.budget_held, sh.game.clone())?)?;
    mg.set("tlk", crate::tables::tlk_api(lua, &sh.budget_held)?)?;
    mg.set("vars", vars_api(lua, sh)?)?;
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

/// `mg.vars`: the variables the module's project describes
/// (`variables.toml`), as the Variables window and `mg vars` read them:
/// `describe(name)`, the description of a variable so named (of that very
/// name, else the first pattern that takes it), nil for none; and
/// `described()`, every description, in the file's order.
fn vars_api(lua: &Lua, sh: &Rc<Shared>) -> mlua::Result<Table> {
    use mg_module::var_desc::{Described, ModuleLookup};
    fn table(lua: &Lua, sh: &Shared, d: &Described) -> mlua::Result<Table> {
        let ws = sh.ws.borrow();
        let module = &ws.module;
        let lookup = ModuleLookup::new(module, sh.game.as_deref().map(|g| &g.resman));
        let choices = d.choices(&lookup);
        let t = lua.create_table()?;
        t.set("name", d.name.as_str())?;
        t.set("pattern", d.is_pattern())?;
        t.set("file", module.project_file.variables.file())?;
        t.set("line", d.line)?;
        let kind = d.var_type.map(|v| mg_module::var_desc::type_label(v).to_ascii_lowercase());
        t.set("type", kind)?;
        t.set("on", d.on.iter().map(|o| o.name()).collect::<Vec<_>>())?;
        t.set("help", d.help.as_deref())?;
        let values = lua.create_table()?;
        for c in &choices.list {
            let v = lua.create_table()?;
            v.set("value", c.value.as_str())?;
            v.set("note", c.note.as_deref())?;
            values.push(v)?;
        }
        t.set("values", values)?;
        t.set("rows", choices.rows)?;
        t.set("problem", choices.problem.as_deref())?;
        t.set("other", d.other)?;
        t.set("min", d.min)?;
        t.set("max", d.max)?;
        t.set("runtime", d.runtime)?;
        t.set("requires", lua.create_sequence_from(d.requires.iter().map(String::as_str))?)?;
        t.set("default", d.default.as_deref())?;
        Ok(t)
    }
    let api = lua.create_table()?;
    let s = sh.clone();
    api.set(
        "describe",
        lua.create_function(move |lua, name: String| {
            let d = s.ws.borrow().module.project_file.variables.describe(&name).cloned();
            d.map(|d| table(lua, &s, &d)).transpose()
        })?,
    )?;
    let s = sh.clone();
    api.set(
        "described",
        lua.create_function(move |lua, ()| {
            let all = s.ws.borrow().module.project_file.variables.described.clone();
            all.iter().map(|d| table(lua, &s, d)).collect::<mlua::Result<Vec<_>>>()
        })?,
    )?;
    Ok(api)
}

/// A machine for one job, the plugin's code loaded (`entry`; the console
/// has none).
pub(crate) fn start(entry: Option<&str>, sh: &Rc<Shared>) -> mlua::Result<Lua> {
    let lua = Lua::new();
    lua.set_memory_limit(MEMORY_LIMIT)?;
    *sh.budget_held.lua.borrow_mut() = Some(lua.weak());
    lua.set_named_registry_value("mg.api", api(&lua, sh)?)?;
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

pub(crate) fn about(plugin: &Plugin) -> [&str; 3] {
    [&plugin.manifest.id, &plugin.manifest.name, &plugin.manifest.version]
}

/// A job's failure as the host reports it.
pub(crate) fn failed(plugin: &Plugin, sh: &Shared, e: mlua::Error) -> PluginError {
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
    // (The machine's own memory, or what the host holds for the job:
    // raised in a function of the API, it comes wrapped.)
    fn memory(e: &mlua::Error) -> Option<&str> {
        match e {
            mlua::Error::MemoryError(what) => Some(what),
            mlua::Error::CallbackError { cause, .. } | mlua::Error::WithContext { cause, .. } => {
                memory(cause)
            }
            _ => None,
        }
    }
    let message = match memory(&e) {
        Some(HELD_TOO_MUCH) => format!(
            "it made Moonglow hold more than the {} MB a job may (the tables and pictures it \
             has, the edits and files it wants written)",
            crate::HELD_LIMIT >> 20
        ),
        Some(_) => format!("it ran out of the {} MB a plugin may use", MEMORY_LIMIT >> 20),
        None => e.to_string(),
    };
    PluginError::Script { plugin: name.to_string(), message }
}

pub(crate) fn shared(plugin: &Plugin, input: Input, host: &Rc<dyn Host>) -> Rc<Shared> {
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
    shared_holding(files, who, input, host, Rc::default())
}

/// [`shared_in`] for a job within another (a check's fixes), which
/// counts what the host holds for it to the other's budget.
fn shared_holding(
    files: Source,
    who: &str,
    input: Input,
    host: &Rc<dyn Host>,
    budget_held: Rc<Budget>,
) -> Rc<Shared> {
    let game = input.game.as_deref().map_or(CODEPAGE, |g| g.text_codepage());
    GAME_CODEPAGE.with(|c| c.set(game));
    let deadline = input.budget.map(|b| std::time::Instant::now() + b);
    Rc::new(Shared {
        budget: input.budget,
        deadline: std::cell::Cell::new(deadline),
        files,
        ws: RefCell::new(Workspace::new(input.module)),
        edits: RefCell::new(Vec::new()),
        haks: RefCell::new(Vec::new()),
        held_bytes: std::cell::Cell::new(0),
        held: RefCell::new(None),
        budget_held,
        folders: RefCell::new(Vec::new()),
        game: input.game,
        host: host.clone(),
        who: who.to_string(),
        rng: RefCell::new(fastrand::Rng::new()),
        selection: input.selection,
        values: input.values,
        decls: RefCell::default(),
        settings: RefCell::new(input.settings.unwrap_or_default()),
        settings_set: RefCell::default(),
        no_set: std::cell::Cell::new(None),
    })
}

pub(crate) fn handler(lua: &Lua, registry: &str, kind: &str, id: &str) -> mlua::Result<Function> {
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
    records: &Rc<RefCell<Fixes>>,
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
                            // (A script or another text: its lines that
                            // differ.)
                            (false, _, _) => match (before.as_deref(), now().as_deref()) {
                                (Some(a), Some(b)) => {
                                    let text = |b: &[u8]| codepage().decode(b).into_owned();
                                    let lines = text_changes(&text(a), &text(b), CHANGED_LINES);
                                    LuaValue::Table(lua.create_sequence_from(lines)?)
                                }
                                _ => LuaValue::Nil,
                            },
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
                    let fix = fix_of(&records, fix, &what, None)?;
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
    let records: Rc<RefCell<Fixes>> = Rc::default();
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

/// A fix a check records (`ctx.fix`): what its button says, and the edits
/// made through it.
struct FixRecord {
    label: String,
    edits: Vec<Edit>,
    /// For a fix that asks: the plugin's command its button runs, and on
    /// what (a resource and where in it; none: the finding's resource).
    asks: Option<(String, Option<(ResKey, GffPath)>)>,
}

/// The fixes a check (or a resource saving's handler) records, and the
/// job their edits are made in: one copy of the module as the check reads
/// it, made at the first edit of a fix, for all of them (so that the check
/// goes on reading the module as it is). Each fix starts from the module
/// as it is: the copy has the edits of one fix at a time ([`turn_to`]),
/// not a copy of the module a fix.
#[derive(Default)]
struct Fixes {
    records: Vec<FixRecord>,
    job: Option<Rc<Shared>>,
    /// The fix whose edits the job's copy has now.
    current: Option<usize>,
}

/// Makes the copy of the module the fixes edit that of fix `n`: the
/// module as the check reads it with that fix's edits so far, those of
/// the fix it had before taken back.
fn turn_to(fixes: &RefCell<Fixes>, job: &Shared, n: usize) -> mlua::Result<()> {
    let mut all = fixes.borrow_mut();
    if all.current == Some(n) {
        return Ok(());
    }
    let mut ws = job.ws.borrow_mut();
    all.current = None;
    while ws.can_undo().is_some() {
        ws.undo().or_else(fail)?;
    }
    let edits = all.records[n].edits.clone();
    if !edits.is_empty() {
        ws.apply(Command::new("plugin", edits)).or_else(fail)?;
    }
    all.current = Some(n);
    Ok(())
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
fn fix_maker(lua: &Lua, sh: &Rc<Shared>, records: &Rc<RefCell<Fixes>>) -> mlua::Result<Function> {
    let (sh, records) = (sh.clone(), records.clone());
    lua.create_function(move |lua, (label, options): (String, Option<Table>)| {
        if label.trim().is_empty() {
            return fail("ctx.fix(label): the label is what its button says, and needs words");
        }
        // (A fix that asks: a command of the plugin's, run on the finding's
        // place when the button is clicked; API 0.5.)
        let asks = match &options {
            None => None,
            Some(o) => {
                let command: String = o.get("command").or_else(|_| {
                    fail("ctx.fix(label, { command = \"…\" }): the command its button runs")
                })?;
                let on = match o.get::<Option<Table>>("on")? {
                    None => None,
                    Some(s) => {
                        let resource: String = s.get("resource")?;
                        let path: Option<String> = s.get("path")?;
                        let path = match path.as_deref() {
                            None | Some("") => GffPath::root(),
                            Some(p) => p.parse().or_else(fail)?,
                        };
                        Some((key_of(&resource)?, path))
                    }
                };
                Some((command, on))
            }
        };
        let n = records.borrow().records.len();
        records.borrow_mut().records.push(FixRecord {
            label: label.clone(),
            edits: Vec::new(),
            asks,
        });
        let fix = lua.create_table()?;
        fix.set("label", label)?;
        let meta = lua.create_table()?;
        meta.set("__fix", n)?;
        // Its methods are made at the first one used (and the copy of the
        // module the fixes edit, at the first of them all): each makes
        // its edit on the copy as this fix has it, and the edit is the
        // fix's.
        let (sh, records) = (sh.clone(), records.clone());
        meta.set(
            "__index",
            lua.create_function(move |lua, (fix, name): (Table, LuaValue)| {
                let job = {
                    let mut all = records.borrow_mut();
                    all.job
                        .get_or_insert_with(|| {
                            let module = sh.ws.borrow().module.clone();
                            let input = Input::new(module, sh.game.clone());
                            let held = sh.budget_held.clone();
                            shared_holding(Source::Nothing, &sh.who, input, &sh.host, held)
                        })
                        .clone()
                };
                let methods = editor(lua, &job)?;
                for pair in methods.pairs::<String, Function>() {
                    let (k, f) = pair?;
                    let (records, job) = (records.clone(), job.clone());
                    let method = lua.create_function(move |_, args: mlua::MultiValue| {
                        turn_to(&records, &job, n)?;
                        let made = f.call::<mlua::MultiValue>(args);
                        records.borrow_mut().records[n].edits.append(&mut job.edits.borrow_mut());
                        // (The method's own error, not one of a call within
                        // a call.)
                        made.map_err(|e| match e {
                            mlua::Error::CallbackError { cause, .. } => Arc::unwrap_or_clone(cause),
                            e => e,
                        })
                    })?;
                    fix.raw_set(k, method)?;
                }
                fix.raw_get::<LuaValue>(name)
            })?,
        )?;
        fix.set_metatable(Some(meta))?;
        Ok(fix)
    })
}

/// A fix given with a finding or a warning: one made with `ctx.fix`, and
/// only if it edits (`what`: what it was given as, for the error), or (a
/// finding's, `about` its resource; `plugin` the plugin) runs a command.
fn fix_of(
    records: &RefCell<Fixes>,
    given: LuaValue,
    what: &str,
    asking: Option<(&Manifest, ResKey)>,
) -> mlua::Result<Option<Fix>> {
    match given {
        LuaValue::Nil => Ok(None),
        LuaValue::Table(t) => {
            let n = match t.metatable() {
                Some(meta) => meta.raw_get::<Option<usize>>("__fix")?,
                None => None,
            };
            let all = records.borrow();
            let Some(record) = n.and_then(|n| all.records.get(n)) else {
                return fail(format!("{what} is one made with ctx.fix(label)"));
            };
            if let Some((command, on)) = &record.asks {
                let Some((manifest, about)) = asking else {
                    return fail(format!(
                        "{what} is one that edits: a fix that asks is a finding's"
                    ));
                };
                if !manifest.commands.iter().any(|c| c.id == *command) {
                    return fail(format!(
                        "{what}: the manifest declares no command {command:?} for it to run"
                    ));
                }
                if !record.edits.is_empty() {
                    return fail(format!(
                        "{what}: a fix that asks runs its command, and edits none"
                    ));
                }
                let on = on.clone().unwrap_or((about, GffPath::root()));
                return Ok(Some(Fix {
                    label: record.label.clone(),
                    edits: Vec::new(),
                    asks: Some(mg_edit::fix::Asks {
                        plugin: manifest.id.clone(),
                        command: command.clone(),
                        on: vec![on],
                    }),
                }));
            }
            Ok((!record.edits.is_empty()).then(|| Fix {
                label: record.label.clone(),
                edits: record.edits.clone(),
                asks: None,
            }))
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
    let records: Rc<RefCell<Fixes>> = Rc::default();
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
            // Its fix: one made with ctx.fix, and only if it edits (or
            // asks, running a command).
            let resource = key_of(&f.get::<String>("resource")?)?;
            let asking = Some((&plugin.manifest, resource));
            let fix = fix_of(&records, f.get::<LuaValue>("fix")?, "a finding's fix", asking)?;
            let finding = Finding {
                severity,
                check: plugin.check_id(&decl.id).into(),
                source: "module".into(),
                resource,
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
        let declared: BTreeMap<&str, Vec<&str>> =
            crate::manifest::KINDS.iter().map(|(kind, _, _)| (*kind, manifest.ids(kind))).collect();
        for (kind, _, registry) in crate::manifest::KINDS {
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
        out.extend(["ctx.selection", "ctx.open", "ctx.values"].map(String::from));
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
        out.extend(crate::kinds::NAMES.map(String::from));
        out.extend(crate::tables::NAMES.map(String::from));
        out.extend(crate::terrain::NAMES.map(String::from));
        out.sort();
        Ok(out)
    };
    names().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A resource's name set from text is the text's bytes in the
    /// module's codepage, as reading the field gave them: a letter outside
    /// ASCII was written as three other bytes, and counted as three.
    #[test]
    fn a_resref_set_from_text_keeps_its_letters() {
        let lua = Lua::new();
        let text = |t: &str| LuaValue::String(lua.create_string(t).unwrap());
        assert_eq!(codepage(), Codepage::WINDOWS_1252);
        let made = typed(FieldType::ResRef, &text("caf\u{e9}"), None).unwrap();
        assert_eq!(made, Value::ResRef(vec![0x63, 0x61, 0x66, 0xe9]));
        // Sixteen letters are sixteen bytes, whichever they are.
        let long = "\u{e9}".repeat(16);
        assert_eq!(typed(FieldType::ResRef, &text(&long), None), Ok(Value::ResRef(vec![0xe9; 16])));
        let e = typed(FieldType::ResRef, &text(&"\u{e9}".repeat(17)), None).unwrap_err();
        assert!(e.contains("is not a resource name"), "{e}");
        // And it reads back as it was given.
        let mut s = Struct::new(0);
        s.set("Conversation", made);
        let read = plain(&lua, &s, &GffPath::root()).unwrap();
        assert_eq!(read.get::<String>("Conversation").unwrap(), "caf\u{e9}");
    }

    struct Nobody;
    impl Host for Nobody {
        fn log(&self, _: Level, _: &str) {}
        fn ask(&self, _: &Question) -> Option<Answer> {
            None
        }
    }

    /// A large selection in a large area is read where the job's copy of
    /// the area has it: 500 of an area's 4,000 placed objects make a
    /// `ctx.selection` in a moment (each entry copied the whole GIT, which
    /// took a panel's quarter second many times over), and the entries
    /// say what they said.
    #[test]
    fn a_large_selection_is_read_without_copying_the_area() {
        use mg_rules::ObjectKind;
        let area = ResRef::from_str("big").unwrap();
        let mut git = Gff::new(*b"GIT ");
        let placeables: Vec<Struct> = (0..4000)
            .map(|i| {
                let mut p = Struct::new(9);
                p.set("Tag", Value::String(format!("crate_{i}").into_bytes()));
                p.set("TemplateResRef", Value::resref(ResRef::from_str("plc_crate").unwrap()));
                for n in 0..20 {
                    p.set(&format!("Field{n}"), Value::String(b"some text of a field".to_vec()));
                }
                p
            })
            .collect();
        git.root.set(ObjectKind::Placeable.list(), Value::List(placeables));
        let mut are = Gff::new(*b"ARE ");
        are.root.set("Width", Value::Int(8));
        let tiles: Vec<Struct> = (0..64).map(|_| Struct::new(1)).collect();
        are.root.set("Tile_List", Value::List(tiles));
        let mut module = mg_module::Module::new();
        module.set_gff(ResKey::new(area, mg_core::ResType::GIT), &git).unwrap();
        module.set_gff(ResKey::new(area, mg_core::ResType::ARE), &are).unwrap();
        let mut input = Input::new(module, None);
        input.selection.selected = (0..500)
            .map(|i| Selected::Object { area, kind: ObjectKind::Placeable, index: i * 8 })
            .chain((0..64).map(|index| Selected::Tile { area, index }))
            .collect();
        input.budget = Some(std::time::Duration::from_millis(250));
        let host: Rc<dyn Host> = Rc::new(Nobody);
        let sh = shared_in(Source::Nothing, "", input, &host);
        let lua = start(None, &sh).unwrap();
        let started = std::time::Instant::now();
        let ctx = context(&lua, &sh, ["", "", ""]).unwrap();
        let took = started.elapsed();
        let chosen: Table = ctx.get("selection").unwrap();
        assert_eq!(chosen.raw_len(), 564);
        let entry: Table = chosen.get(3).unwrap();
        assert_eq!(entry.get::<String>("tag").unwrap(), "crate_16");
        assert_eq!(entry.get::<String>("resref").unwrap(), "plc_crate");
        assert_eq!(entry.get::<String>("path").unwrap(), "/Placeable List[16]");
        let tile: Table = chosen.get(500 + 11).unwrap();
        assert_eq!((tile.get::<usize>("x").unwrap(), tile.get::<usize>("y").unwrap()), (2, 1));
        assert!(took < std::time::Duration::from_millis(500), "{took:?}");
        // And the time it took is not the job's.
        assert!(!sh.out_of_time());
        let left = sh.deadline.get().unwrap() - std::time::Instant::now();
        assert!(left > std::time::Duration::from_millis(100), "{left:?}");
    }
}
