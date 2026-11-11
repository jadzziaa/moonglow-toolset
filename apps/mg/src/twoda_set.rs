//! `mg 2da-set`: a 2DA file's cells set, in a row found by its number or
//! by a column's value, or in a row added at the end; the file written
//! as it is laid out (`TwoDa::to_bytes_like`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mg_2da::TwoDa;
use mg_core::Codepage;
use serde_json::json;

use crate::{Output, path_text};

/// What `mg 2da-set` is asked.
pub(crate) struct Asked<'a> {
    pub(crate) table: &'a Path,
    /// The row by its number.
    pub(crate) row: Option<usize>,
    /// The row by a column's value (`COLUMN=VALUE`).
    pub(crate) matching: Option<&'a str>,
    /// A row added at the end.
    pub(crate) append: bool,
    /// Each `COLUMN=VALUE`.
    pub(crate) cells: &'a [String],
    pub(crate) output: Option<&'a PathBuf>,
    pub(crate) dry_run: bool,
}

/// A `COLUMN=VALUE`: the column's name and the value, `None` for an empty
/// cell (`****`, or nothing after the `=`).
fn split(cell: &str) -> Result<(&str, Option<String>)> {
    let (column, value) = (cell.split_once('='))
        .filter(|(c, _)| !c.trim().is_empty())
        .with_context(|| format!("{cell:?}: give a cell as COLUMN=VALUE"))?;
    let value = Some(value.to_string()).filter(|v| !v.is_empty() && v != "****");
    Ok((column.trim(), value))
}

/// A cell as the file writes it.
fn shown(cell: Option<&str>) -> String {
    match cell {
        None => "****".into(),
        Some(v) if v.is_empty() || v.contains([' ', '\t']) => format!("\"{v}\""),
        Some(v) => v.to_string(),
    }
}

pub(crate) fn run(asked: &Asked<'_>, cp: Codepage) -> Result<Output> {
    let path = asked.table;
    let original = std::fs::read(path).with_context(|| path_text(path))?;
    let mut table = TwoDa::parse(&original, cp).with_context(|| path_text(path))?;
    let column = |name: &str| {
        table.column(name).with_context(|| {
            format!("{}: no column {name} (it has {})", path.display(), table.columns().join(", "))
        })
    };
    // Each cell's column and value, checked before anything changes.
    let mut cells = Vec::new();
    for cell in asked.cells {
        let (name, value) = split(cell)?;
        if let Some(v) = &value {
            TwoDa::holds(v).with_context(|| format!("{name}={v}"))?;
        }
        cells.push((column(name)?, value));
    }
    let row = match (asked.row, asked.matching, asked.append) {
        (Some(row), None, false) if row < table.len() => row,
        (Some(row), None, false) => {
            bail!("{}: no row {row} (it has {} rows, 0 to {})", path.display(), table.len(), {
                table.len().saturating_sub(1)
            })
        }
        (None, Some(matching), false) => {
            let (name, value) = split(matching)?;
            let c = column(name)?;
            let found: Vec<usize> =
                (0..table.len()).filter(|&r| table.cell(r, c) == value.as_deref()).collect();
            match found[..] {
                [row] => row,
                [] => bail!("{}: no row has {name} {}", path.display(), shown(value.as_deref())),
                _ => bail!(
                    "{}: rows {} have {name} {}: give the row by its number (--row)",
                    path.display(),
                    found.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "),
                    shown(value.as_deref())
                ),
            }
        }
        (None, None, true) => table.add_row(),
        _ => bail!("give one of --row N, --where COLUMN=VALUE or --append"),
    };
    let mut changed = Vec::new();
    for (c, value) in cells {
        let from = table.cell(row, c).map(str::to_string);
        if from != value {
            changed.push((c, from, value.clone()));
        }
        table.set(row, c, value)?;
    }
    let to = asked.output.map_or(path, PathBuf::as_path);
    let mut out = Output::default();
    let name = |c: usize| table.columns()[c].clone();
    for (c, from, value) in &changed {
        out.line(format!(
            "row {row} {}: {} -> {}",
            name(*c),
            shown(from.as_deref()),
            shown(value.as_deref())
        ));
    }
    // (Written where a cell changed or a row was added, or to another file.)
    let write = !changed.is_empty() || asked.append || to != path;
    if write && !asked.dry_run {
        crate::write_bytes_whole(to, &table.to_bytes_like(cp, &original)?)?;
    }
    let done = match (write, asked.dry_run) {
        (false, _) => "nothing to change".to_string(),
        (true, true) => format!("would write {}", to.display()),
        (true, false) => format!("wrote {}", to.display()),
    };
    out.note(done);
    out.json = json!({
        "table": path_text(path),
        "output": path_text(to),
        "row": row,
        "appended": asked.append,
        "cells": (changed.iter())
            .map(|(c, from, to)| json!({ "column": name(*c), "from": from, "to": to }))
            .collect::<Vec<_>>(),
        "written": write && !asked.dry_run,
    });
    Ok(out)
}
