//! `mg apply` and `mg set`: changing a module's fields and resources from
//! the command line, through the same undoable commands the editors use
//! (`mg_edit`), so that nothing else in a resource is touched.

use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use mg_core::Codepage;
use mg_edit::wire::{command_from_json, command_to_json, field_from_str};
use mg_edit::{Command, Edit, Workspace};
use mg_gff::{FieldType, Value};
use mg_module::Module;
use mg_resman::ResKey;
use serde_json::json;

use super::plugins::SaveGuard;
use super::{Output, resource_key};

/// `mg apply`: the edits of a file (`-`: standard input), all or none.
/// (`codepage`, here and below: that of the module's text, Windows-1252
/// or the table of its `encoding.2da`.)
pub(crate) fn apply(
    module: Module,
    edits: &Path,
    dry_run: bool,
    codepage: Codepage,
    guard: Option<&SaveGuard>,
) -> Result<Output> {
    let text = if edits == Path::new("-") {
        let mut text = String::new();
        std::io::stdin().lock().read_to_string(&mut text)?;
        text
    } else {
        std::fs::read_to_string(edits).with_context(|| edits.display().to_string())?
    };
    let json: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("{} is not JSON", edits.display()))?;
    let cmd =
        command_from_json(&json, codepage).map_err(|e| anyhow!("{}: {e}", edits.display()))?;
    run(Workspace::new(module), cmd, dry_run, None, false, codepage, guard)
}

/// `mg set`: fields of one resource, each `FIELD=VALUE` (or
/// `FIELD:TYPE=VALUE` for a field the resource lacks), and fields to remove.
pub(crate) fn set(
    module: Module,
    resource: &str,
    assignments: &[String],
    remove: &[String],
    dry_run: bool,
    codepage: Codepage,
    guard: Option<&SaveGuard>,
) -> Result<Output> {
    let key = resource_key(resource)?;
    let mut ws = Workspace::new(module);
    let root = ws.doc(&key)?.root.clone();
    let mut edits = Vec::new();
    // Each field as it was and as it will be, for people.
    let mut lines = Vec::new();
    let shown = |v: &Value| match mg_gff::value_to_json(v, codepage) {
        Ok(json) => json.get("value").or(json.get("value64")).cloned().unwrap_or_default(),
        Err(_) => serde_json::Value::Null,
    };
    for a in assignments {
        let (spec, text) =
            a.split_once('=').with_context(|| format!("{a:?}: give each field as FIELD=VALUE"))?;
        // A type after the last colon, when it is one.
        let (field, given) = match spec.rsplit_once(':') {
            Some((field, ty)) if FieldType::from_json_name(ty).is_some() => {
                (field, FieldType::from_json_name(ty))
            }
            _ => (spec, None),
        };
        let (path, label) = field_from_str(field).map_err(|e| anyhow!("{field:?}: {e}"))?;
        let Some(at) = path.get(&root) else { bail!("{key}: no struct at {path}") };
        let old = at.get(&label);
        // Its own type; else the one given; else the game's for that field.
        let ty = old
            .map(Value::field_type)
            .or(given)
            .or_else(|| {
                path.0.is_empty().then(|| mg_schema::root_field_type(key.restype, &label)).flatten()
            })
            .with_context(|| {
                format!(
                    "{key} has no field {field}: give its type, as in {field}:int=3 \
                     (byte, char, word, short, dword, int, dword64, int64, float, double, \
                     cexostring, resref, cexolocstring)"
                )
            })?;
        let value =
            Value::from_text(ty, text, old, codepage).map_err(|e| anyhow!("{field}: {e}"))?;
        let place = mg_edit::wire::field_to_string(&path, &label);
        lines.push(match old {
            Some(old) => format!("{key} {place}: {} -> {}", shown(old), shown(&value)),
            None => format!("{key} {place}: {} (new, {})", shown(&value), ty.json_name()),
        });
        edits.push(Edit::SetField { key, path, label, value: Some(value) });
    }
    for field in remove {
        let (path, label) = field_from_str(field).map_err(|e| anyhow!("{field:?}: {e}"))?;
        let has = path.get(&root).is_some_and(|s| s.get(&label).is_some());
        if !has {
            bail!("{key} has no field {field}");
        }
        lines.push(format!("{key} {}: removed", mg_edit::wire::field_to_string(&path, &label)));
        edits.push(Edit::SetField { key, path, label, value: None });
    }
    if edits.is_empty() {
        bail!("nothing to set: give FIELD=VALUE, or --remove FIELD");
    }
    let label = match edits.as_slice() {
        [Edit::SetField { label, value: Some(_), .. }] => format!("Set {label}"),
        [Edit::SetField { label, .. }] => format!("Remove {label}"),
        _ => format!("Set {} fields of {key}", edits.len()),
    };
    run(ws, Command::new(label, edits), dry_run, Some(lines), true, codepage, guard)
}

/// Applies a command (all of it or none) and saves, or with `dry_run`
/// only checks that it applies. `lines`: what to print for people in
/// place of the resources changed; `show`: the command is in the JSON, as
/// `mg apply` reads it; `guard`: the plugins that look at what is saved
/// (before: a refusal is an error, and nothing is written; after).
pub(crate) fn run(
    mut ws: Workspace,
    cmd: Command,
    dry_run: bool,
    lines: Option<Vec<String>>,
    show: bool,
    codepage: Codepage,
    guard: Option<&SaveGuard>,
) -> Result<Output> {
    let mut resources: Vec<ResKey> = Vec::new();
    for e in &cmd.edits {
        if !resources.contains(e.key()) {
            resources.push(*e.key());
        }
    }
    let written = command_to_json(&cmd, codepage).map_err(|e| anyhow!("{e}"))?;
    let (label, count) = (cmd.label.clone(), cmd.edits.len());
    let was = guard.map(|_| ws.module.clone());
    ws.apply(cmd)?;
    let mut notes = Output::default();
    let mut said = Vec::new();
    if let (Some(g), Some(was)) = (guard, &was)
        && count > 0
    {
        ws.flush()?;
        said = g.before(was, &ws.module, &mut notes)?;
    }
    if !dry_run && count > 0 {
        ws.save()?;
        if let (Some(g), Some(was)) = (guard, &was) {
            g.after(was, &ws.module, &mut notes);
        }
    }
    let mut out = Output::new(json!({
        "label": label,
        "edits": count,
        "resources": resources.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "dry_run": dry_run,
    }));
    if show && let serde_json::Value::Object(o) = &mut out.json {
        o.insert("command".into(), written);
    }
    if !said.is_empty()
        && let serde_json::Value::Object(o) = &mut out.json
    {
        o.insert("said".into(), said.into());
    }
    out.notes.extend(notes.notes);
    match lines {
        Some(lines) => lines.into_iter().for_each(|l| out.line(l)),
        None => resources.iter().for_each(|r| out.line(r.to_string())),
    }
    let plural = |n: usize, what: &str| format!("{n} {what}{}", if n == 1 { "" } else { "s" });
    let what = if dry_run { "would change" } else { "changed" };
    out.note(format!(
        "{label}: {} {what} {}",
        plural(count, "edit"),
        plural(resources.len(), "resource")
    ));
    Ok(out)
}
