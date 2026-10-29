//! `mg tlk-alloc`: a custom talk table written from strings kept by name,
//! each on a line that stays its own (`mg_module::tlk_alloc`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use mg_module::talk::Line;
use mg_module::tlk_alloc::{self, Map, Name};
use serde_json::{Value, json};

use crate::Output;

pub(crate) struct Args<'a> {
    pub table: &'a Path,
    pub from: &'a Path,
    pub map: Option<&'a PathBuf>,
    pub constants: Option<&'a PathBuf>,
    pub prefix: &'a str,
    pub start: u32,
    pub check: bool,
    pub against: Option<&'a PathBuf>,
}

/// The strings of a names file, as JSON read them: a text is a string; a
/// table with a `text` (and perhaps a `feminine`, a `sound` and a
/// `sound_length`) is a string with those; any other table is a group,
/// whose name goes before its strings' with a dot.
fn names_of(value: &Value, under: &str, out: &mut Vec<Name>) -> Result<()> {
    let Some(table) = value.as_object() else { bail!("{under}: not a table of strings") };
    for (k, v) in table {
        let key = if under.is_empty() { k.clone() } else { format!("{under}.{k}") };
        match v {
            Value::String(text) => out.push(Name { key, text: text.clone(), ..Default::default() }),
            Value::Object(o)
                if o.get("text").is_some_and(Value::is_string)
                    && o.keys().all(|k| {
                        matches!(k.as_str(), "text" | "feminine" | "sound" | "sound_length")
                    }) =>
            {
                let text =
                    |k: &str| o.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
                let sound_length = o.get("sound_length").and_then(Value::as_f64).unwrap_or(0.0);
                let feminine = o.get("feminine").map(|v| v.as_str().map(str::to_string));
                let Some(feminine) = feminine.map_or(Some(None), |f| f.map(Some)) else {
                    bail!("{key}: its feminine is not a text")
                };
                out.push(Name {
                    key,
                    text: text("text"),
                    feminine,
                    sound: text("sound"),
                    sound_length: sound_length as f32,
                });
            }
            Value::Object(_) => names_of(v, &key, out)?,
            _ => bail!("{key}: not a text (a string, or a table with a \"text\")"),
        }
    }
    Ok(())
}

fn names(path: &Path) -> Result<Vec<Name>> {
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let value: Value = match crate::extension(path).as_str() {
        // (In the file's order: new keys take lines in it.)
        "toml" => {
            tlk_alloc::toml_in_order(&text).map_err(|e| anyhow!("{}: {e}", path.display()))?
        }
        "json" => serde_json::from_str(&text).with_context(|| path.display().to_string())?,
        _ => bail!("the names file must end .toml or .json"),
    };
    let mut out = Vec::new();
    names_of(&value, "", &mut out).with_context(|| path.display().to_string())?;
    Ok(out)
}

pub(crate) fn run(a: &Args<'_>) -> Result<Output> {
    let names = names(a.from)?;
    let map_path = a.map.cloned().unwrap_or_else(|| a.from.with_extension("strrefs.json"));
    let known = match std::fs::read_to_string(&map_path) {
        Ok(text) => Map::read(&text).map_err(|e| anyhow!("{}: {e}", map_path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Map::default(),
        Err(e) => return Err(e).with_context(|| map_path.display().to_string()),
    };
    let mut table = crate::talk_table(a.table, true)?;
    // A key with a feminine text of its own makes the feminine table
    // (`TABLEf.tlk` beside it), the others' lines in it as in the table.
    if table.feminine.is_none() && names.iter().any(|n| n.feminine.is_some()) {
        let name = mg_module::talk::feminine(&table.name);
        let path = a.table.with_file_name(format!("{name}.tlk"));
        let tlk = mg_tlk::Tlk::new(table.tlk.language);
        table.feminine = Some((mg_module::talk::Source::File(path), tlk));
    }
    let ours: std::collections::BTreeSet<u32> =
        known.lines.values().chain(known.retired.values()).copied().collect();
    let taken = |line: u32| {
        let row = line as usize;
        !ours.contains(&line) && row < table.len() && {
            let l = table.line(row);
            !l.text.is_empty() || !l.sound.is_empty()
        }
    };
    let plan = tlk_alloc::allocate(&names, &known, a.start, taken).map_err(|e| anyhow!(e))?;
    // The table's lines as they are to be.
    let mut lines = std::collections::BTreeMap::new();
    for n in &names {
        let row = plan.map.lines[&n.key] as usize;
        let mut line = match row < table.len() {
            true => table.line(row),
            false => Line {
                text: String::new(),
                feminine: table.feminine.as_ref().map(|_| String::new()),
                sound: String::new(),
                sound_length: 0.0,
            },
        };
        let was = line.clone();
        (line.text, line.sound, line.sound_length) =
            (n.text.clone(), n.sound.clone(), n.sound_length);
        // (The feminine table's line is the text, unless it has its own.)
        if table.feminine.is_some() {
            line.feminine = Some(n.feminine.clone().unwrap_or_else(|| n.text.clone()));
        }
        if line != was || row >= table.len() {
            lines.insert(row, line);
        }
    }
    let stale: Vec<String> =
        lines.keys().map(|row| format!("line {row} of the table is not as named")).collect();
    let constants = match a.constants {
        None => None,
        Some(path) => {
            let text = match crate::extension(path).as_str() {
                "nss" => tlk_alloc::constants_nss(&plan.map, a.prefix),
                "json" => tlk_alloc::constants_json(&plan.map, a.prefix),
                _ => bail!("the constants file must end .nss or .json"),
            };
            Some((path, text.map_err(|e| anyhow!(e))?))
        }
    };
    let moved = match a.against {
        None => Vec::new(),
        Some(path) => {
            let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
            let older = Map::read(&text).map_err(|e| anyhow!("{}: {e}", path.display()))?;
            plan.map.moved(&older)
        }
    };
    let map_text = plan.map.to_json();
    let differs =
        |path: &Path, text: &str| std::fs::read_to_string(path).ok().as_deref() != Some(text);
    let mut out = Output::new(json!({
        "table": crate::path_text(a.table),
        "map": crate::path_text(&map_path),
        "keys": plan.map.lines.len(),
        "added": plan.added.iter().map(|(k, n)| json!({ "key": k, "line": n })).collect::<Vec<_>>(),
        "retired": plan.retired,
        "restored": plan.restored,
        "lines_changed": lines.len(),
        "moved": moved.iter().map(|(k, a, b)| json!({ "key": k, "from": a, "to": b })).collect::<Vec<_>>(),
    }));
    for (key, from, to) in &moved {
        out.line(format!("MOVED {key}: line {from} -> {to}"));
    }
    if a.check {
        let mut wrong: Vec<String> = Vec::new();
        wrong.extend(plan.added.iter().map(|(k, _)| format!("{k} has no line yet")));
        wrong.extend(plan.retired.iter().map(|k| format!("{k} is no longer named")));
        wrong.extend(plan.restored.iter().map(|k| format!("{k} is named again")));
        wrong.extend(stale);
        if differs(&map_path, &map_text) && wrong.is_empty() {
            wrong.push(format!("{} is not as it would be written", map_path.display()));
        }
        if let Some((path, text)) = &constants
            && differs(path, text)
        {
            wrong.push(format!("{} is not as it would be written", path.display()));
        }
        for w in &wrong {
            out.line(w.clone());
        }
        out.json["up_to_date"] = json!(wrong.is_empty() && moved.is_empty());
        out.failed = !wrong.is_empty() || !moved.is_empty();
        out.note(match out.failed {
            true => "not up to date: run mg tlk-alloc without --check".to_string(),
            false => format!("up to date: {} keys", plan.map.lines.len()),
        });
        return Ok(out);
    }
    if !moved.is_empty() {
        bail!(
            "{} keys have another line than in {}: nothing written",
            moved.len(),
            a.against.map(|p| p.display().to_string()).unwrap_or_default()
        );
    }
    let changed = lines.len();
    if !lines.is_empty() || !a.table.is_file() {
        table.import(lines).map_err(|e| anyhow!(e))?;
        table.save().map_err(|e| anyhow!(e))?;
    }
    if differs(&map_path, &map_text) {
        std::fs::write(&map_path, &map_text).with_context(|| map_path.display().to_string())?;
    }
    if let Some((path, text)) = &constants
        && differs(path, text)
    {
        std::fs::write(path, text).with_context(|| path.display().to_string())?;
    }
    for (key, line) in &plan.added {
        out.line(format!("+ {key}: line {line} ({})", line | mg_core::StrRef::CUSTOM_FLAG));
    }
    for key in &plan.retired {
        out.line(format!("- {key}: its line is kept for it"));
    }
    out.note(format!(
        "{} keys: {} new, {} retired, {changed} lines of the table written",
        plan.map.lines.len(),
        plan.added.len(),
        plan.retired.len()
    ));
    Ok(out)
}
