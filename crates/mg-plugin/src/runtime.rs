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
use mg_edit::wire::field_from_str;
use mg_edit::{Command, Edit, GffPath, Step, Workspace};
use mg_gff::{FieldType, Gff, Struct, Value};
use mg_module::doctor::{Finding, Severity};
use mg_resman::ResKey;
use mg_rules::GameData;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value as LuaValue, Variadic, VmState};

use crate::{
    Answer, CheckDecl, CommandDecl, FieldKind, FormField, Host, Input, Level, MEMORY_LIMIT,
    Outcome, Plugin, PluginError, Question,
};

/// The codepage of a module's text, as the plugin's UTF-8 strings go in
/// and come out.
const CODEPAGE: Codepage = Codepage::WINDOWS_1252;

/// What a job's API works on.
struct Shared {
    dir: PathBuf,
    ws: RefCell<Workspace>,
    edits: RefCell<Vec<Edit>>,
    game: Option<Arc<GameData>>,
    host: Rc<dyn Host>,
}

fn fail<T>(message: impl std::fmt::Display) -> mlua::Result<T> {
    Err(mlua::Error::runtime(message))
}

fn key_of(name: &str) -> mlua::Result<ResKey> {
    match ResKey::from_filename(name) {
        Some(key) => Ok(key),
        None => fail(format!("{name:?} is not a resource's name (name.ext)")),
    }
}

impl Shared {
    /// Applies an edit to the private copy and records it.
    fn apply(&self, edit: Edit) -> mlua::Result<()> {
        let applied = self.ws.borrow_mut().apply(Command::new("plugin", vec![edit.clone()]));
        match applied {
            Ok(()) => {
                self.edits.borrow_mut().push(edit);
                Ok(())
            }
            Err(e) => fail(e),
        }
    }

    /// A resource's bytes: the module's (with the job's edits), or with
    /// `game` what the game would load.
    fn bytes(&self, key: &ResKey, game: bool) -> mlua::Result<Vec<u8>> {
        if game {
            let Some(data) = &self.game else { return fail("there is no game data") };
            return match data.resman.get(key) {
                Ok(bytes) => Ok(bytes.into_owned()),
                Err(_) => fail(format!("{key} is not in the game's resources")),
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
        let text = |b: &[u8]| lua.create_string(CODEPAGE.decode(b).as_bytes());
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
    meta.raw_set("path", path.to_string())?;
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
    match CODEPAGE.encode(&s) {
        Some(bytes) => Ok(bytes.into_owned()),
        None => Err("text has characters the module's codepage cannot hold".into()),
    }
}

/// A plain value as a field of type `ty` (text for a localized string
/// becomes its English, the rest of `old` kept).
fn typed(ty: FieldType, v: &LuaValue, old: Option<&Value>) -> Result<Value, String> {
    let int = |min: i128, max: i128| whole(v, ty, min, max);
    Ok(match ty {
        FieldType::Byte => Value::Byte(int(0, u8::MAX.into())? as u8),
        FieldType::Char => Value::Char(int(i8::MIN.into(), i8::MAX.into())? as i8),
        FieldType::Word => Value::Word(int(0, u16::MAX.into())? as u16),
        FieldType::Short => Value::Short(int(i16::MIN.into(), i16::MAX.into())? as i16),
        FieldType::Dword => Value::Dword(int(0, u32::MAX.into())? as u32),
        FieldType::Int => Value::Int(int(i32::MIN.into(), i32::MAX.into())? as i32),
        FieldType::Dword64 => Value::Dword64(int(0, u64::MAX.into())? as u64),
        FieldType::Int64 => Value::Int64(int(i64::MIN.into(), i64::MAX.into())? as i64),
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

/// A table that is a value with its type, as `mg.int(3)` makes it and
/// `mg gff` writes it: `{ type = "int", value = 3 }`.
fn typed_table(lua: &Lua, v: &LuaValue) -> mlua::Result<Option<Value>> {
    let LuaValue::Table(t) = v else { return Ok(None) };
    if !matches!(t.raw_get::<LuaValue>("type")?, LuaValue::String(_)) {
        return Ok(None);
    }
    let json: serde_json::Value = lua.from_value(v.clone())?;
    match mg_gff::value_from_json(&json, CODEPAGE) {
        Ok(value) => Ok(Some(value)),
        Err(e) => fail(e),
    }
}

/// The first argument of a call made with a colon (`ctx.module:has(..)`):
/// the table the function is in, not used.
type This = LuaValue;

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
                s.game.as_ref().is_some_and(|g| g.resman.contains(&key))
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
            let json = match path {
                None => mg_gff::to_json(&gff, CODEPAGE),
                Some(path) => {
                    let path: GffPath = path.parse().or_else(fail)?;
                    match path.get(&gff.root) {
                        Some(at) => mg_gff::struct_to_json(at, CODEPAGE),
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
            lua.create_string(CODEPAGE.decode(&bytes).as_bytes())
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
                let Some(data) = &s.game else { return fail("there is no game data") };
                let table = data.table(&name).or_else(fail)?;
                let out = lua.create_table()?;
                out.set("rows", table.len())?;
                out.set("columns", table.columns().to_vec())?;
                let cells = table.clone();
                out.set(
                    "get",
                    lua.create_function(move |_, (_, row, column): (This, usize, String)| {
                        Ok(cells.get(row, &column).map(str::to_string))
                    })?,
                )?;
                Ok(out)
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
                let value = match typed_table(lua, &value)? {
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
                let json: serde_json::Value = lua.from_value(item)?;
                let item = mg_gff::struct_from_json(&json, CODEPAGE)
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
            let Some(data) = CODEPAGE.encode(&text) else {
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
            let json: serde_json::Value = lua.from_value(doc)?;
            let gff = mg_gff::from_json(&json, CODEPAGE).or_else(|e| fail(format!("{key}{e}")))?;
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

/// What a job's handler gets.
fn context(lua: &Lua, sh: &Rc<Shared>, plugin: &Plugin) -> mlua::Result<Table> {
    let ctx = lua.create_table()?;
    ctx.set("module", reader(lua, sh, false)?)?;
    ctx.set("game", reader(lua, sh, true)?)?;
    ctx.set("edit", editor(lua, sh)?)?;
    let about = lua.create_table()?;
    about.set("id", plugin.manifest.id.as_str())?;
    about.set("name", plugin.manifest.name.as_str())?;
    about.set("version", plugin.manifest.version.as_str())?;
    ctx.set("plugin", about)?;

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
    ctx.set("ui", ui)?;
    Ok(ctx)
}

/// What `require("@moonglow")` gives: where handlers are registered, and
/// helpers.
fn api(lua: &Lua) -> mlua::Result<Table> {
    let mg = lua.create_table()?;
    mg.set("api", crate::API)?;
    for (kind, registry) in [("command", "mg.commands"), ("check", "mg.checks")] {
        lua.set_named_registry_value(registry, lua.create_table()?)?;
        mg.set(
            kind,
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
                // (A localized string's text alone is its English.)
                match (&value, ty) {
                    (LuaValue::String(_), FieldType::LocString) => {
                        let texts = lua.create_table()?;
                        texts.set("0", value)?;
                        t.set("value", texts)?;
                    }
                    _ => t.set("value", value)?,
                }
                Ok(t)
            })?,
        )?;
    }
    // Where a struct read with `gff` is: its path, for the edits.
    mg.set(
        "path",
        lua.create_function(|_, t: Table| match t.metatable() {
            Some(meta) => meta.raw_get::<Option<String>>("path"),
            None => Ok(None),
        })?,
    )?;
    Ok(mg)
}

/// A machine for one job, the plugin's code loaded.
fn start(plugin: &Plugin, sh: &Rc<Shared>) -> mlua::Result<Lua> {
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
    let dir = sh.dir.clone();
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
            let loaded: Table = lua.named_registry_value("mg.modules")?;
            let cached: LuaValue = loaded.get(file)?;
            if !cached.is_nil() {
                return Ok(cached);
            }
            let path = dir.join(format!("{file}.luau"));
            let source = std::fs::read_to_string(&path)
                .or_else(|e| fail(format!("require({name:?}): {e}")))?;
            let value: LuaValue = lua.load(source).set_name(format!("@{file}.luau")).eval()?;
            // (Loaded once, whatever it returns.)
            let kept = if value.is_nil() { LuaValue::Boolean(true) } else { value };
            loaded.set(file, kept.clone())?;
            Ok(kept)
        })?,
    )?;

    // Called off, the code stops: every step raises, and what catches
    // errors (pcall, xpcall) passes this one on.
    let s = sh.clone();
    globals.set("__cancelled", lua.create_function(move |_, ()| Ok(s.host.cancelled()))?)?;
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
    lua.set_interrupt(
        move |_| {
            if s.host.cancelled() { fail("canceled") } else { Ok(VmState::Continue) }
        },
    );

    // From here the globals are fixed, and each script has its own.
    lua.sandbox(true)?;
    let entry = sh.dir.join(&plugin.manifest.entry);
    let source = std::fs::read_to_string(&entry)
        .or_else(|e| fail(format!("{}: {e}", plugin.manifest.entry)))?;
    lua.load(source).set_name(format!("@{}", plugin.manifest.entry)).exec()?;
    Ok(lua)
}

/// A job's failure as the host reports it.
fn failed(plugin: &Plugin, host: &Rc<dyn Host>, e: mlua::Error) -> PluginError {
    if host.cancelled() {
        return PluginError::Canceled(plugin.manifest.name.clone());
    }
    let message = match &e {
        mlua::Error::MemoryError(_) => {
            format!("it ran out of the {} MB a plugin may use", MEMORY_LIMIT >> 20)
        }
        other => other.to_string(),
    };
    PluginError::Script { plugin: plugin.manifest.name.clone(), message }
}

fn shared(plugin: &Plugin, input: Input, host: &Rc<dyn Host>) -> Rc<Shared> {
    Rc::new(Shared {
        dir: plugin.dir.clone(),
        ws: RefCell::new(Workspace::new(input.module)),
        edits: RefCell::new(Vec::new()),
        game: input.game,
        host: host.clone(),
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
        let lua = start(plugin, &sh)?;
        let f = handler(&lua, "mg.commands", "command", &decl.id)?;
        let made: LuaValue = f.call(context(&lua, &sh, plugin)?)?;
        // What it calls the change: `return { label = … }`, or its title.
        let label = match &made {
            LuaValue::Table(t) => t.get::<Option<String>>("label")?,
            LuaValue::String(s) => Some(s.to_str()?.to_string()),
            _ => None,
        };
        Ok(label.unwrap_or_else(|| decl.title.clone()))
    };
    let label = run().map_err(|e| failed(plugin, &host, e))?;
    if host.cancelled() {
        return Err(PluginError::Canceled(plugin.manifest.name.clone()));
    }
    Ok(Outcome { label, edits: sh.edits.take() })
}

pub(crate) fn run_check(
    plugin: &Plugin,
    decl: &CheckDecl,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Vec<Finding>, PluginError> {
    let sh = shared(plugin, input, &host);
    let run = || -> mlua::Result<Vec<Finding>> {
        let lua = start(plugin, &sh)?;
        let f = handler(&lua, "mg.checks", "check", &decl.id)?;
        let made: LuaValue = f.call(context(&lua, &sh, plugin)?)?;
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
            out.push(Finding {
                severity,
                check: plugin.check_id(&decl.id).into(),
                source: "module".into(),
                resource: key_of(&f.get::<String>("resource")?)?,
                at: f.get::<Option<String>>("at")?.unwrap_or_default(),
                message: f.get("message")?,
            });
        }
        Ok(out)
    };
    let findings = run().map_err(|e| failed(plugin, &host, e))?;
    if !sh.edits.borrow().is_empty() {
        return Err(PluginError::Script {
            plugin: plugin.manifest.name.clone(),
            message: format!("the check {:?} edits the module: a check only reads", decl.id),
        });
    }
    Ok(findings)
}

pub(crate) fn inspect(plugin: &Plugin, host: Rc<dyn Host>) -> Result<Vec<String>, PluginError> {
    let input = Input { module: mg_module::Module::new(), game: None };
    let sh = shared(plugin, input, &host);
    let run = || -> mlua::Result<Vec<String>> {
        let lua = start(plugin, &sh)?;
        let mut faults = Vec::new();
        let registered = |registry: &str| -> mlua::Result<Vec<String>> {
            let handlers: Table = lua.named_registry_value(registry)?;
            handlers.pairs::<String, LuaValue>().map(|p| p.map(|(id, _)| id)).collect()
        };
        let declared: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
            ("command", plugin.manifest.commands.iter().map(|c| c.id.as_str()).collect()),
            ("check", plugin.manifest.checks.iter().map(|c| c.id.as_str()).collect()),
        ]);
        for (kind, registry) in [("command", "mg.commands"), ("check", "mg.checks")] {
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
    run().map_err(|e| failed(plugin, &host, e))
}
