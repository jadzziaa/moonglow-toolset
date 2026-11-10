//! `mg 2da-diff` and `mg tlk-diff`: what differs between two tables by
//! row and column (or line), and the two merged when both were made from a
//! third.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mg_2da::TwoDa;
use mg_2da::diff::{Prefer, diff as diff_tables, merge as merge_tables};
use mg_core::Codepage;
use mg_resman::ResKey;
use serde_json::{Value, json};

use crate::Output;

pub(crate) struct Args<'a> {
    pub a: &'a Path,
    pub b: &'a Path,
    pub base: Option<&'a PathBuf>,
    pub output: Option<&'a PathBuf>,
    /// The second way out: an edit file (2DA) or a file of lines (TLK).
    pub changes: Option<&'a PathBuf>,
    pub prefer: Option<&'a str>,
    pub resource: Option<&'a str>,
}

fn table(path: &Path, cp: Codepage) -> Result<TwoDa> {
    let data = std::fs::read(path).with_context(|| path.display().to_string())?;
    TwoDa::parse(&data, cp).with_context(|| path.display().to_string())
}

/// `table` written as the file `like` is laid out (`TwoDa::to_bytes_like`:
/// its columns in line or compact, their widths, its rows' labels, its
/// tabs and line ends): a merge written like the table A it was made
/// into, an update like the table B it is.
fn written_like(table: &TwoDa, like: &Path, cp: Codepage) -> Result<Vec<u8>> {
    let file = std::fs::read(like).with_context(|| like.display().to_string())?;
    Ok(table.to_bytes_like(cp, &file)?)
}

fn shown(cell: &Option<String>) -> String {
    match cell {
        None => "****".into(),
        Some(v) if v.is_empty() || v.contains(' ') => format!("\"{v}\""),
        Some(v) => v.clone(),
    }
}

/// A row as people know it: its number and its label, if the table has
/// one.
fn row_name(tables: &[&TwoDa], row: usize) -> String {
    let label = tables.iter().find_map(|t| t.get(row, "label").or_else(|| t.get(row, "name")));
    match label {
        Some(label) => format!("row {row} ({label})"),
        None => format!("row {row}"),
    }
}

/// An edit file that sets `resource` to `table`, for `mg apply`.
/// (`like`: the file the table is written like, its layout, widths, row
/// labels and tabs kept.)
fn edit_file(
    resource: &str,
    table: &TwoDa,
    like: &Path,
    cp: Codepage,
    label: &str,
) -> Result<Value> {
    let key = ResKey::from_filename(resource)
        .with_context(|| format!("{resource:?} is not a resource name like classes.2da"))?;
    let data = written_like(table, like, cp)?;
    let edit = mg_edit::Edit::SetResource { key, data: Some(data) };
    let command = mg_edit::Command::new(label, vec![edit]);
    Ok(mg_edit::wire::command_to_json(&command, cp)?)
}

pub(crate) fn twoda(args: &Args<'_>, cp: Codepage) -> Result<Output> {
    let (a, b) = (table(args.a, cp)?, table(args.b, cp)?);
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().to_lowercase());
    let resource = args.resource.map(str::to_string).or_else(|| name(args.b)).unwrap_or_default();
    let cells = |d: &mg_2da::diff::Diff| -> Vec<Value> {
        (d.cells.iter())
            .map(|c| json!({ "row": c.row, "column": c.column, "from": c.from, "to": c.to }))
            .collect()
    };
    let as_json = |d: &mg_2da::diff::Diff| {
        json!({
            "columns_added": d.columns_added, "columns_removed": d.columns_removed,
            "rows_added": [d.rows_added.start, d.rows_added.end],
            "rows_removed": [d.rows_removed.start, d.rows_removed.end],
            "default": d.default.as_ref().map(|(from, to)| json!({ "from": from, "to": to })),
            "cells": cells(d),
        })
    };
    let lines = |out: &mut Output, d: &mg_2da::diff::Diff, tables: &[&TwoDa], mark: &str| {
        for c in &d.columns_added {
            out.line(format!("{mark}+ column {c}"));
        }
        for c in &d.columns_removed {
            out.line(format!("{mark}- column {c}"));
        }
        if !d.rows_added.is_empty() {
            out.line(format!("{mark}+ rows {} to {}", d.rows_added.start, d.rows_added.end - 1));
        }
        if !d.rows_removed.is_empty() {
            out.line(format!(
                "{mark}- rows {} to {}",
                d.rows_removed.start,
                d.rows_removed.end - 1
            ));
        }
        if let Some((from, to)) = &d.default {
            out.line(format!("{mark}  DEFAULT: {} -> {}", shown(from), shown(to)));
        }
        for c in &d.cells {
            let (row, from, to) = (row_name(tables, c.row), shown(&c.from), shown(&c.to));
            out.line(format!("{mark}  {row} {}: {from} -> {to}", c.column));
        }
    };
    let Some(base) = args.base else {
        if args.output.is_some() || args.prefer.is_some() {
            bail!(
                "-o and --prefer are for a merge: name what both tables were made from with --base"
            );
        }
        let d = diff_tables(&a, &b);
        let mut out = Output::new(as_json(&d));
        out.json["same"] = json!(d.is_empty());
        lines(&mut out, &d, &[&b, &a], "");
        if let Some(file) = args.changes {
            let edits = edit_file(&resource, &b, args.b, cp, &format!("Update {resource}"))?;
            let text = serde_json::to_string_pretty(&edits)? + "\n";
            crate::write_bytes_whole(file, text.as_bytes())?;
        }
        out.note(match d.is_empty() {
            true => "the tables are the same".to_string(),
            false => format!("{} cells differ", d.cells.len()),
        });
        return Ok(out);
    };
    let base = table(base, cp)?;
    let prefer = match args.prefer {
        None | Some("a") => Prefer::Ours,
        Some("b") => Prefer::Theirs,
        Some(other) => bail!("--prefer a or b, not {other}"),
    };
    let merged = merge_tables(&base, &a, &b, prefer);
    let (ours, theirs) = (diff_tables(&base, &a), diff_tables(&base, &b));
    let mut out = Output::new(json!({
        "a": as_json(&ours), "b": as_json(&theirs),
        "conflicts": merged.conflicts.iter().map(|c| json!({
            "row": c.row, "column": c.column, "base": c.base, "a": c.ours, "b": c.theirs,
        })).collect::<Vec<_>>(),
    }));
    lines(&mut out, &ours, &[&a, &base], "A ");
    lines(&mut out, &theirs, &[&b, &base], "B ");
    for c in &merged.conflicts {
        out.line(format!(
            "CONFLICT {} {}: {} -> A {}, B {}",
            row_name(&[&a, &b, &base], c.row),
            c.column,
            shown(&c.base),
            shown(&c.ours),
            shown(&c.theirs)
        ));
    }
    let unsettled = !merged.conflicts.is_empty() && args.prefer.is_none();
    out.json["merged"] = json!(!unsettled);
    if unsettled {
        out.failed = true;
        out.note(format!(
            "{} cells were changed by both, each to something else: nothing written (--prefer \
             a or b takes one side's)",
            merged.conflicts.len()
        ));
        return Ok(out);
    }
    if let Some(file) = args.output {
        crate::write_bytes_whole(file, &written_like(&merged.table, args.a, cp)?)?;
    }
    if let Some(file) = args.changes {
        let label = format!("Merge {resource}");
        let edits = edit_file(&resource, &merged.table, args.a, cp, &label)?;
        let text = serde_json::to_string_pretty(&edits)? + "\n";
        crate::write_bytes_whole(file, text.as_bytes())?;
    }
    out.note(format!(
        "merged: {} cells from A, {} from B, {} changed by both",
        ours.cells.len(),
        theirs.cells.len(),
        merged.conflicts.len()
    ));
    Ok(out)
}

fn talk(path: &Path) -> Result<mg_tlk::Tlk> {
    let data = std::fs::read(path).with_context(|| path.display().to_string())?;
    mg_tlk::Tlk::read(&data).with_context(|| path.display().to_string())
}

fn line_json(e: &Option<mg_tlk::TlkEntry>, cp: Codepage) -> Value {
    match e {
        None => Value::Null,
        Some(e) => json!({
            "text": cp.decode(&e.text),
            "sound": e.sound_resref().map(|r| r.to_string()),
            "sound_length": e.sound_length,
        }),
    }
}

fn line_shown(e: &Option<mg_tlk::TlkEntry>, cp: Codepage) -> String {
    match e {
        None => "(none)".into(),
        Some(e) => {
            let text = cp.decode(&e.text).replace('\n', "\\n");
            let text: String = match text.chars().count() > 70 {
                true => text.chars().take(69).chain(['…']).collect(),
                false => text,
            };
            match e.sound_resref() {
                Some(sound) => format!("\"{text}\" [{sound}]"),
                None => format!("\"{text}\""),
            }
        }
    }
}

/// The lines as a file `mg tlk-import` reads: each with what it is to be.
fn lines_file(table: &mg_tlk::Tlk, lines: &[u32], cp: Codepage) -> Value {
    let entries: Vec<Value> = (lines.iter())
        .map(|line| {
            let e = table.entries.get(*line as usize).cloned().unwrap_or_default();
            let mut o = serde_json::Map::new();
            o.insert("id".into(), json!(line));
            o.insert("text".into(), json!(cp.decode(&e.text)));
            if let Some(sound) = e.sound_resref() {
                o.insert("sound".into(), json!(sound.to_string()));
            }
            if e.sound_length != 0.0 {
                o.insert("soundLen".into(), json!(e.sound_length));
            }
            Value::Object(o)
        })
        .collect();
    json!({ "language": table.language.0, "entries": entries })
}

pub(crate) fn tlk(args: &Args<'_>, cp: Codepage) -> Result<Output> {
    let (a, b) = (talk(args.a)?, talk(args.b)?);
    let changes_json = |d: &[mg_tlk::diff::Change]| -> Vec<Value> {
        d.iter()
            .map(|c| json!({ "line": c.line, "from": line_json(&c.from, cp), "to": line_json(&c.to, cp) }))
            .collect()
    };
    let lines = |out: &mut Output, d: &[mg_tlk::diff::Change], mark: &str| {
        for c in d {
            let (from, to) = (line_shown(&c.from, cp), line_shown(&c.to, cp));
            out.line(format!("{mark}line {}: {from} -> {to}", c.line));
        }
    };
    let Some(base) = args.base else {
        if args.output.is_some() || args.prefer.is_some() {
            bail!(
                "-o and --prefer are for a merge: name what both tables were made from with --base"
            );
        }
        let d = mg_tlk::diff::diff(&a, &b);
        let mut out = Output::new(json!({ "same": d.is_empty(), "lines": changes_json(&d) }));
        lines(&mut out, &d, "");
        if let Some(file) = args.changes {
            let changed: Vec<u32> = d.iter().map(|c| c.line).collect();
            let text = serde_json::to_string_pretty(&lines_file(&b, &changed, cp))? + "\n";
            crate::write_bytes_whole(file, text.as_bytes())?;
        }
        out.note(match d.is_empty() {
            true => "the tables are the same".to_string(),
            false => format!("{} lines differ", d.len()),
        });
        return Ok(out);
    };
    let base = talk(base)?;
    let prefer_theirs = match args.prefer {
        None | Some("a") => false,
        Some("b") => true,
        Some(other) => bail!("--prefer a or b, not {other}"),
    };
    let (merged, conflicts) = mg_tlk::diff::merge(&base, &a, &b, prefer_theirs);
    let (ours, theirs) = (mg_tlk::diff::diff(&base, &a), mg_tlk::diff::diff(&base, &b));
    let mut out = Output::new(json!({
        "a": changes_json(&ours), "b": changes_json(&theirs),
        "conflicts": conflicts.iter().map(|c| json!({
            "line": c.line, "base": line_json(&c.base, cp),
            "a": line_json(&c.ours, cp), "b": line_json(&c.theirs, cp),
        })).collect::<Vec<_>>(),
    }));
    lines(&mut out, &ours, "A ");
    lines(&mut out, &theirs, "B ");
    for c in &conflicts {
        out.line(format!(
            "CONFLICT line {}: {} -> A {}, B {}",
            c.line,
            line_shown(&c.base, cp),
            line_shown(&c.ours, cp),
            line_shown(&c.theirs, cp)
        ));
    }
    let unsettled = !conflicts.is_empty() && args.prefer.is_none();
    out.json["merged"] = json!(!unsettled);
    if unsettled {
        out.failed = true;
        out.note(format!(
            "{} lines were changed by both, each to something else: nothing written (--prefer a \
             or b takes one side's)",
            conflicts.len()
        ));
        return Ok(out);
    }
    if let Some(file) = args.output {
        crate::write_bytes_whole(file, &merged.to_bytes()?)?;
    }
    if let Some(file) = args.changes {
        let changed: Vec<u32> = mg_tlk::diff::diff(&base, &merged).iter().map(|c| c.line).collect();
        let text = serde_json::to_string_pretty(&lines_file(&merged, &changed, cp))? + "\n";
        crate::write_bytes_whole(file, text.as_bytes())?;
    }
    out.note(format!(
        "merged: {} lines from A, {} from B, {} changed by both",
        ours.len(),
        theirs.len(),
        conflicts.len()
    ));
    Ok(out)
}

/// A talk table's count of strings, from its file: a `.tlk`, or a
/// nasher project's `.tlk.json`.
fn talk_table_count(path: &Path, cp: Codepage) -> Result<usize> {
    let data = std::fs::read(path).with_context(|| path.display().to_string())?;
    let name = path.to_string_lossy().to_ascii_lowercase();
    let tlk = match name.ends_with(".json") {
        true => mg_tlk::json::from_json(&data, cp).map_err(|e| anyhow::anyhow!(e)),
        false => mg_tlk::Tlk::read(&data).map_err(|e| anyhow::anyhow!(e.to_string())),
    };
    Ok(tlk.with_context(|| path.display().to_string())?.entries.len())
}

/// `mg 2da-check`: tables held to their descriptions. A custom string
/// (16777216 and on) is held to `talk` (a talk table's file), else to the
/// one talk table of the folder walked, if it has one; with neither, it
/// is not looked up (and said so).
pub(crate) fn check(
    paths: &[PathBuf],
    beside: Option<&Path>,
    talk: Option<&Path>,
    game: Option<&mg_rules::GameData>,
    cp: Codepage,
) -> Result<Output> {
    use mg_module::doctor::TalkTables;
    use mg_module::table_desc::{Beside, Description, find_for, game_description, path_beside};
    let base = game.map_or(0, |g| g.tlk().entries.len());
    let custom = match talk {
        Some(path) => Some(talk_table_count(path, cp)?),
        None => game.and_then(|g| g.custom_tlk().map(|t| t.entries.len())),
    };
    let mut tables = Vec::new();
    let (mut wrong, mut unread) = (0, 0);
    let mut out = Output::new(Value::Null);
    for path in paths {
        // A folder: its described tables; a table: its own description (or
        // its family's), else Moonglow's of the game's table of its name.
        let (root, found) = if path.is_dir() {
            (path.clone(), Beside::of(path).described())
        } else {
            let name = path.file_stem().map(|n| n.to_string_lossy().into_owned());
            let description = find_for(path)
                .or_else(|| game_description(name.as_deref()?).map(|_| PathBuf::new()));
            let Some(description) = description else {
                bail!(
                    "{} has no description ({}, or a family's in its folder)",
                    path.display(),
                    path_beside(path).display()
                );
            };
            (Beside::root_of(path), vec![(path.clone(), description)])
        };
        let files = Beside::of(beside.unwrap_or(&root));
        // The talk table its custom strings are held to: the one asked
        // for, the game's module's, else the folder's own if it has one.
        let custom = custom.or_else(|| {
            let tables: Vec<&PathBuf> = files
                .files()
                .iter()
                .filter(|p| {
                    let n = p.to_string_lossy().to_ascii_lowercase();
                    n.ends_with(".tlk") || n.ends_with(".tlk.json")
                })
                .collect();
            let [one] = tables[..] else { return None };
            let count = talk_table_count(one, cp).ok()?;
            out.note(format!("custom strings are held to {} ({count} strings)", one.display()));
            Some(count)
        });
        let tlk = TalkTables {
            base,
            // (None known: a custom string is not looked up.)
            custom: Some(custom.unwrap_or(TalkTables::UNCOUNTED)),
        };
        if custom.is_none() {
            out.note(
                "no talk table: custom strings (16777216 and on) are not looked up (--tlk FILE \
                 names one)",
            );
        }
        let script = |name: &str| {
            let key = ResKey::from_filename(&format!("{name}.nss"))?;
            files.script(name).or_else(|| Some(game?.resman.get(&key).ok()?.into_owned()))
        };
        let rows_of = |name: &str| {
            files.rows_of(name, cp).or_else(|| game?.table(name).ok().map(|t| t.len()))
        };
        for (table_path, description_path) in found {
            let name = table_path.display().to_string();
            let t = table(&table_path, cp)?;
            // (No file: Moonglow's description of the game's table.)
            let ours = description_path.as_os_str().is_empty();
            let stem = table_path.file_stem().map(|n| n.to_string_lossy().into_owned());
            let source = match ours {
                true => game_description(stem.as_deref().unwrap_or_default())
                    .unwrap_or_default()
                    .to_string(),
                false => std::fs::read_to_string(&description_path)
                    .with_context(|| description_path.display().to_string())?,
            };
            let description_path = match ours {
                true => PathBuf::from(format!(
                    "Moonglow's description of the game's {}.2da",
                    stem.as_deref().unwrap_or_default()
                )),
                false => description_path,
            };
            let d = match Description::parse(&source, &script) {
                Ok(d) => d,
                Err(e) => {
                    unread += 1;
                    out.line(format!("{}: not read: {e}", description_path.display()));
                    tables.push(json!({
                        "table": name,
                        "description": description_path.display().to_string(),
                        "error": e,
                    }));
                    continue;
                }
            };
            let mut faults = d.faults(&t, &rows_of, tlk);
            // (Moonglow's description of a game's table: a cell as the
            // game's own copy has it is the game's, not the builder's.)
            if ours && let Some(base) = stem.as_deref().and_then(|n| game?.table(n).ok()) {
                faults.retain(|f| {
                    let column = &t.columns()[f.column];
                    base.get(f.row, column) != t.cell(f.row, f.column)
                });
            }
            let missing = d.missing_columns(&t);
            wrong += faults.len();
            out.line(match faults.len() {
                0 => format!("{name}: {} rows, nothing wrong", t.len()),
                1 => format!("{name}: 1 cell is wrong"),
                n => format!("{name}: {n} cells are wrong"),
            });
            for f in &faults {
                let column = &t.columns()[f.column];
                out.line(format!("  {}, {column}: {}", row_name(&[&t], f.row), f.message));
            }
            for note in &d.notes {
                out.note(format!("{}: {note}", description_path.display()));
            }
            if !missing.is_empty() {
                out.note(format!("{name} has no column {}", missing.join(", ")));
            }
            tables.push(json!({
                "table": name,
                "description": description_path.display().to_string(),
                "rows": t.len(),
                "faults": faults.iter().map(|f| json!({
                    "row": f.row,
                    "column": t.columns()[f.column],
                    "value": t.cell(f.row, f.column),
                    "message": f.message,
                })).collect::<Vec<_>>(),
                "missing_columns": missing,
                "notes": d.notes,
            }));
        }
    }
    if tables.is_empty() {
        out.line("No table with a description (name.2da.toml beside name.2da)");
    }
    out.json = json!({ "tables": tables, "wrong": wrong, "unread": unread });
    out.failed = wrong > 0 || unread > 0;
    Ok(out)
}
