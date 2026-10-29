//! 2DAs and talk tables as values a plugin reads, changes and writes out:
//! `mg.twoda` and `mg.tlk`. A generator makes its tables with these
//! instead of putting a 2DA's text together by hand.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use mg_2da::TwoDa;
use mg_module::doctor::ColumnKind;
use mg_tlk::{Tlk, TlkEntry};
use mlua::{Lua, ObjectLike, Table, Value as LuaValue};

use crate::runtime::{codepage, fail};

type This = LuaValue;

/// What these objects have, for the list of the API's names.
pub(crate) const NAMES: [&str; 11] = [
    "twoda:set",
    "twoda:add_row",
    "twoda:add_column",
    "twoda:write",
    "tlk.lines",
    "tlk.language",
    "tlk:set_language",
    "tlk:get",
    "tlk:set",
    "tlk:add",
    "tlk:write",
];

/// A cell's text from what a plugin gives: text, or a number written as
/// the game's tables write one. Nil is an empty cell.
fn cell(value: &LuaValue) -> mlua::Result<Option<String>> {
    Ok(match value {
        LuaValue::Nil => None,
        LuaValue::String(s) => Some(s.to_str()?.to_string()).filter(|s| s != "****"),
        LuaValue::Integer(n) => Some(n.to_string()),
        LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => Some((*n as i64).to_string()),
        LuaValue::Number(n) => Some(n.to_string()),
        LuaValue::Boolean(b) => Some(if *b { "1" } else { "0" }.to_string()),
        _ => return fail("a cell is text, a number or nil"),
    })
}

/// A 2DA as plugins have it: what `ctx.game:table` and `mg.twoda` give.
/// It is the plugin's own: changing it changes nothing else (a table of
/// the game's is copied when first changed). `file`: the file it was read
/// from, which `write` keeps the layout of (as it is, where the table is
/// unchanged).
pub(crate) fn twoda(lua: &Lua, table: Arc<TwoDa>, file: Option<Vec<u8>>) -> mlua::Result<Table> {
    let out = lua.create_table()?;
    out.set("rows", table.len())?;
    out.set("columns", table.columns().to_vec())?;
    let shared = Rc::new(RefCell::new(table));
    let t = shared.clone();
    out.set(
        "get",
        lua.create_function(move |_, (_, row, column): (This, usize, String)| {
            Ok(t.borrow().get(row, &column).map(str::to_string))
        })?,
    )?;
    let t = shared.clone();
    out.set(
        "set",
        lua.create_function(move |_, (_, row, column, value): (This, usize, String, LuaValue)| {
            let mut t = t.borrow_mut();
            let t = Arc::make_mut(&mut t);
            let Some(at) = t.column(&column) else {
                return fail(format!("the table has no column {column}"));
            };
            if row >= t.len() {
                return fail(format!("the table has no row {row} (add_row adds one)"));
            }
            t.set(row, at, cell(&value)?).or_else(|_| fail("a cell can't hold a double quote"))
        })?,
    )?;
    let t = shared.clone();
    out.set(
        "add_row",
        lua.create_function(move |_, (this, cells): (Table, Option<Table>)| {
            let mut t = t.borrow_mut();
            let t = Arc::make_mut(&mut t);
            // (All of the row or none: a cell refused leaves the table
            // as it was.)
            let mut row = Vec::new();
            for pair in cells.iter().flat_map(|c| c.pairs::<String, LuaValue>()) {
                let (column, value) = pair?;
                let Some(at) = t.column(&column) else {
                    return fail(format!("the table has no column {column}"));
                };
                let value = cell(&value)?;
                if value.as_ref().is_some_and(|v| v.contains('"')) {
                    return fail("a cell can't hold a double quote");
                }
                row.push((at, value));
            }
            let at = t.add_row();
            for (column, value) in row {
                t.set(at, column, value).or_else(|_| fail("a cell can't hold a double quote"))?;
            }
            this.set("rows", t.len())?;
            Ok(at)
        })?,
    )?;
    let t = shared.clone();
    out.set(
        "add_column",
        lua.create_function(move |_, (this, name): (Table, String)| {
            let mut t = t.borrow_mut();
            let t = Arc::make_mut(&mut t);
            if t.column(&name).is_some() {
                return fail(format!("the table has a column {name}"));
            }
            t.add_column(&name).or_else(fail)?;
            this.set("columns", t.columns().to_vec())
        })?,
    )?;
    let t = shared;
    out.set(
        "write",
        lua.create_function(move |lua, _: This| {
            let t = t.borrow();
            let bytes = match &file {
                Some(file) => t.to_bytes_like(codepage(), file),
                None => t.to_bytes(codepage()),
            };
            lua.create_string(bytes.or_else(fail)?)
        })?,
    )?;
    Ok(out)
}

/// `mg.twoda`: tables made and read; `game`, the job's game data, by
/// which `kinds` knows a table by the column that names it too.
pub(crate) fn twoda_api(lua: &Lua, game: Option<Arc<mg_rules::GameData>>) -> mlua::Result<Table> {
    let api = lua.create_table()?;
    api.set(
        "parse",
        lua.create_function(|lua, bytes: mlua::LuaString| {
            let table = TwoDa::parse(&bytes.as_bytes(), codepage()).or_else(fail)?;
            twoda(lua, Arc::new(table), Some(bytes.as_bytes().to_vec()))
        })?,
    )?;
    api.set(
        "new",
        lua.create_function(|lua, columns: Vec<String>| {
            twoda(lua, Arc::new(TwoDa::new(columns).or_else(fail)?), None)
        })?,
    )?;
    api.set(
        "kinds",
        lua.create_function(move |lua, name: String| {
            let name = name.trim_end_matches(".2da");
            let read = |n: &str| game.as_deref().and_then(|g| g.table(n).ok());
            let out = lua.create_table()?;
            for (column, kind) in mg_module::doctor::column_kinds_as_named(name, &read) {
                let kind = match kind {
                    ColumnKind::StrRef => LuaValue::String(lua.create_string("strref")?),
                    ColumnKind::Model => LuaValue::String(lua.create_string("model")?),
                    ColumnKind::Row(table) => {
                        LuaValue::String(lua.create_string(format!("row:{table}"))?)
                    }
                    ColumnKind::Table => LuaValue::String(lua.create_string("table")?),
                    ColumnKind::Script => LuaValue::String(lua.create_string("script")?),
                    // (The table by another column's value.)
                    ColumnKind::RowBy { by, tables } => {
                        let t = lua.create_table()?;
                        t.set("by", by)?;
                        let rows = lua.create_table()?;
                        for (value, table) in tables {
                            rows.set(*value, *table)?;
                        }
                        t.set("rows", rows)?;
                        LuaValue::Table(t)
                    }
                };
                out.set(column, kind)?;
            }
            Ok(out)
        })?,
    )?;
    api.set("describe", lua.create_function(describe)?)?;
    Ok(api)
}

/// `mg.twoda.describe(text, scripts)`: a table's description
/// ([`mg_module::table_desc`]) read from its file's text; `scripts` gives
/// the sources of the scripts whose constants it names, by name.
fn describe(lua: &Lua, (text, scripts): (String, Option<Table>)) -> mlua::Result<Table> {
    use mg_module::table_desc::{Description, Kind};
    let script = |name: &str| -> Option<Vec<u8>> {
        let source: mlua::LuaString = scripts.as_ref()?.get(name).ok()?;
        Some(source.as_bytes().to_vec())
    };
    let description = Rc::new(Description::parse(&text, &script).or_else(fail)?);
    let out = lua.create_table()?;
    out.set("rows", description.rows.clone())?;
    out.set("notes", description.notes.clone())?;
    let columns = lua.create_table()?;
    for (name, column) in &description.columns {
        let kind = match &column.kind {
            Kind::Text => "text".to_string(),
            Kind::Int { .. } => "int".to_string(),
            Kind::Float { .. } => "float".to_string(),
            Kind::List { .. } => "list".to_string(),
            Kind::Flags { .. } => "flags".to_string(),
            Kind::Row(table) => format!("row:{table}"),
            Kind::StrRef => "strref".to_string(),
            Kind::Script => "script".to_string(),
        };
        columns.set(name.as_str(), kind)?;
    }
    out.set("columns", columns)?;
    let d = description.clone();
    out.set(
        "name",
        lua.create_function(move |_, (_, column, value): (This, String, LuaValue)| {
            let Some(value) = cell(&value)? else { return Ok(None) };
            Ok(d.column(&column).and_then(|c| c.kind.named(&value)))
        })?,
    )?;
    let d = description;
    out.set(
        "faults",
        lua.create_function(move |lua, (_, table, lengths): (This, Table, Option<Table>)| {
            // (The table as it is now: what its `write` gives.)
            let bytes: mlua::LuaString = table.call_method("write", ())?;
            let t = TwoDa::parse(&bytes.as_bytes(), codepage()).or_else(fail)?;
            let rows_of = |name: &str| -> Option<usize> { lengths.as_ref()?.get(name).ok()? };
            let found = lua.create_table()?;
            let tlk = mg_module::doctor::TalkTables::default();
            for f in d.faults(&t, &rows_of, tlk) {
                // (Strings are not looked up here: a plugin has no talk
                // table to hold them to.)
                if matches!(d.column(&t.columns()[f.column]).map(|c| &c.kind), Some(Kind::StrRef))
                    && f.message.contains("doesn't exist")
                {
                    continue;
                }
                let fault = lua.create_table()?;
                fault.set("row", f.row)?;
                fault.set("column", t.columns()[f.column].as_str())?;
                fault.set("value", t.cell(f.row, f.column))?;
                fault.set("message", f.message)?;
                found.push(fault)?;
            }
            Ok(found)
        })?,
    )?;
    Ok(out)
}

/// The codepage of a talk table's text: its language's, in the game's.
fn codepage_of(t: &Tlk) -> mg_core::Codepage {
    codepage().for_language(t.language)
}

/// A language as a plugin names it: its two-letter code (`"fr"`).
fn language(code: &str) -> mlua::Result<mg_core::Language> {
    mg_core::Language::from_short_code(code).map_or_else(
        || fail(format!("{code:?} is not a language of the game's (en, fr, de, it, es, pl)")),
        Ok,
    )
}

fn entry_table(lua: &Lua, e: &TlkEntry, cp: mg_core::Codepage) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("text", lua.create_string(cp.decode(&e.text).as_bytes())?)?;
    if let Some(sound) = e.sound_resref() {
        t.set("sound", sound.to_string())?;
    }
    if e.sound_length != 0.0 {
        t.set("sound_length", e.sound_length)?;
    }
    Ok(t)
}

/// A line from what a plugin gives: its text, or a table with `text`,
/// `sound` and `sound_length`. Nil is no line.
fn entry(value: &LuaValue, cp: mg_core::Codepage) -> mlua::Result<TlkEntry> {
    let encoded = |text: &str| match cp.encode(text) {
        Some(bytes) => Ok(bytes.into_owned()),
        None => fail("the text has letters the game's codepage lacks"),
    };
    Ok(match value {
        LuaValue::Nil => TlkEntry::default(),
        LuaValue::String(s) => TlkEntry::text(encoded(&s.to_str()?)?),
        LuaValue::Table(t) => {
            let text: Option<String> = t.get("text")?;
            let mut e = TlkEntry::text(encoded(text.as_deref().unwrap_or_default())?);
            if let Some(sound) = t.get::<Option<String>>("sound")?.filter(|s| !s.is_empty()) {
                if sound.len() > 16 || !sound.is_ascii() {
                    return fail(format!("{sound}: a sound's name is at most 16 letters"));
                }
                e.sound = sound.into_bytes();
                e.flags |= mg_tlk::FLAG_SOUND;
            }
            if let Some(length) = t.get::<Option<f32>>("sound_length")?.filter(|l| *l != 0.0) {
                e.sound_length = length;
                e.flags |= mg_tlk::FLAG_SOUND_LENGTH;
            }
            e
        }
        _ => return fail("a line is its text, a table with a text, or nil"),
    })
}

/// A talk table as plugins have it. Lines count from 0, as in the file (a
/// custom table's line 0 is the game's string 16777216).
fn tlk(lua: &Lua, table: Tlk) -> mlua::Result<Table> {
    let out = lua.create_table()?;
    out.set("lines", table.entries.len())?;
    out.set("language", table.language.short_code())?;
    let shared = Rc::new(RefCell::new(table));
    let t = shared.clone();
    out.set(
        "set_language",
        lua.create_function(move |_, (this, code): (Table, String)| {
            t.borrow_mut().language = language(&code)?;
            this.set("language", code.to_ascii_lowercase())
        })?,
    )?;
    let t = shared.clone();
    out.set(
        "get",
        lua.create_function(move |lua, (_, line): (This, usize)| {
            let t = t.borrow();
            match t.entries.get(line).filter(|e| **e != TlkEntry::default()) {
                Some(e) => Ok(Some(entry_table(lua, e, codepage_of(&t))?)),
                None => Ok(None),
            }
        })?,
    )?;
    let t = shared.clone();
    out.set(
        "set",
        lua.create_function(move |_, (this, line, value): (Table, usize, LuaValue)| {
            if line >= mg_core::StrRef::CUSTOM_FLAG as usize {
                return fail(format!("{line} is past what a table holds"));
            }
            let mut t = t.borrow_mut();
            let e = entry(&value, codepage_of(&t))?;
            if t.entries.len() <= line {
                t.entries.resize(line + 1, TlkEntry::default());
            }
            t.entries[line] = e;
            this.set("lines", t.entries.len())
        })?,
    )?;
    let t = shared.clone();
    out.set(
        "add",
        lua.create_function(move |_, (this, value): (Table, LuaValue)| {
            let mut t = t.borrow_mut();
            let e = entry(&value, codepage_of(&t))?;
            t.entries.push(e);
            this.set("lines", t.entries.len())?;
            Ok(t.entries.len() - 1)
        })?,
    )?;
    let t = shared;
    out.set(
        "write",
        lua.create_function(move |lua, _: This| {
            lua.create_string(t.borrow().to_bytes().or_else(fail)?)
        })?,
    )?;
    Ok(out)
}

/// `mg.tlk`: talk tables made and read.
pub(crate) fn tlk_api(lua: &Lua) -> mlua::Result<Table> {
    let api = lua.create_table()?;
    api.set(
        "parse",
        lua.create_function(|lua, bytes: mlua::LuaString| {
            tlk(lua, Tlk::read(&bytes.as_bytes()).or_else(fail)?)
        })?,
    )?;
    api.set(
        "new",
        lua.create_function(|lua, code: Option<String>| {
            let l = code.as_deref().map_or(Ok(mg_core::Language::ENGLISH), language)?;
            tlk(lua, Tlk::new(l))
        })?,
    )?;
    Ok(api)
}
