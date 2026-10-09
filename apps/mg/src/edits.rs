//! `mg apply` and `mg set`: changing a module's fields and resources from
//! the command line, through the same undoable commands the editors use
//! (`mg_edit`), so that nothing else in a resource is touched.

use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use mg_core::{Codepage, Gender, Language, LocString, ResRef};
use mg_edit::wire::{command_from_json, command_to_json, field_from_str};
use mg_edit::{Command, Edit, Workspace};
use mg_gff::{FieldType, Value};
use mg_module::Module;
use mg_resman::ResKey;
use serde_json::json;

use super::{Output, resource_key};

/// The codepage of the text in commands: Windows-1252 (as `mg gff` reads
/// and writes), or the table of the module's `encoding.2da`, once
/// [`use_codepage`] has said so.
static MODULE_CODEPAGE: std::sync::OnceLock<Codepage> = std::sync::OnceLock::new();

/// The module's codepage, for the commands of this run.
pub(crate) fn use_codepage(codepage: Codepage) {
    let _ = MODULE_CODEPAGE.set(codepage);
}

fn codepage() -> Codepage {
    MODULE_CODEPAGE.get().copied().unwrap_or(Codepage::WINDOWS_1252)
}

/// `mg apply`: the edits of a file (`-`: standard input), all or none.
pub(crate) fn apply(module: &Path, edits: &Path, dry_run: bool) -> Result<Output> {
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
        command_from_json(&json, codepage()).map_err(|e| anyhow!("{}: {e}", edits.display()))?;
    let ws = Workspace::new(Module::open(module)?);
    run(ws, cmd, dry_run, None, false)
}

/// `mg set`: fields of one resource, each `FIELD=VALUE` (or
/// `FIELD:TYPE=VALUE` for a field the resource lacks), and fields to remove.
pub(crate) fn set(
    module: &Path,
    resource: &str,
    assignments: &[String],
    remove: &[String],
    dry_run: bool,
) -> Result<Output> {
    let key = resource_key(resource)?;
    let mut ws = Workspace::new(Module::open(module)?);
    let root = ws.doc(&key)?.root.clone();
    let mut edits = Vec::new();
    // Each field as it was and as it will be, for people.
    let mut lines = Vec::new();
    let shown = |v: &Value| match mg_gff::value_to_json(v, codepage()) {
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
        let value = typed(ty, text, old).with_context(|| field.to_string())?;
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
    run(ws, Command::new(label, edits), dry_run, Some(lines), true)
}

/// `text` as a value of type `ty` (a localized string keeps what `old` has
/// and takes the text as its English).
fn typed(ty: FieldType, text: &str, old: Option<&Value>) -> Result<Value> {
    let int = |min: i128, max: i128| -> Result<i128> {
        let v: i128 = text.trim().parse().with_context(|| format!("{text:?} is not a number"))?;
        if v < min || v > max {
            bail!("{v} does not fit a {} ({min} to {max})", ty.json_name());
        }
        Ok(v)
    };
    let float = || -> Result<f64> {
        let v: f64 = text.trim().parse().with_context(|| format!("{text:?} is not a number"))?;
        if !v.is_finite() {
            bail!("{text:?} is not a finite number");
        }
        Ok(v)
    };
    let encoded =
        || codepage().encode(text).map(|b| b.into_owned()).context("text the codepage cannot hold");
    Ok(match ty {
        FieldType::Byte => Value::Byte(int(0, u8::MAX.into())? as u8),
        FieldType::Char => Value::Char(int(i8::MIN.into(), i8::MAX.into())? as i8),
        FieldType::Word => Value::Word(int(0, u16::MAX.into())? as u16),
        FieldType::Short => Value::Short(int(i16::MIN.into(), i16::MAX.into())? as i16),
        FieldType::Dword => Value::Dword(int(0, u32::MAX.into())? as u32),
        FieldType::Int => Value::Int(int(i32::MIN.into(), i32::MAX.into())? as i32),
        FieldType::Dword64 => Value::Dword64(int(0, u64::MAX.into())? as u64),
        FieldType::Int64 => Value::Int64(int(i64::MIN.into(), i64::MAX.into())? as i64),
        FieldType::Float => Value::Float(float()? as f32),
        FieldType::Double => Value::Double(float()?),
        FieldType::String => Value::String(encoded()?),
        FieldType::ResRef => {
            let r = ResRef::from_str(text)
                .map_err(|e| anyhow!("{text:?} is not a resource name: {e}"))?;
            Value::resref(r)
        }
        FieldType::LocString => {
            let mut s = match old {
                Some(Value::LocString(s)) => s.clone(),
                _ => LocString::default(),
            };
            if text.is_empty() {
                s.remove(Language::ENGLISH, Gender::Male);
            } else {
                s.set(Language::ENGLISH, Gender::Male, encoded()?);
            }
            Value::LocString(s)
        }
        FieldType::Void | FieldType::Struct | FieldType::List => {
            bail!("a {} is set with mg apply", ty.json_name())
        }
    })
}

/// Applies a command (all of it or none) and saves, or with `dry_run`
/// only checks that it applies. `lines`: what to print for people in
/// place of the resources changed; `show`: the command is in the JSON, as
/// `mg apply` reads it.
pub(crate) fn run(
    mut ws: Workspace,
    cmd: Command,
    dry_run: bool,
    lines: Option<Vec<String>>,
    show: bool,
) -> Result<Output> {
    let mut resources: Vec<ResKey> = Vec::new();
    for e in &cmd.edits {
        if !resources.contains(e.key()) {
            resources.push(*e.key());
        }
    }
    let written = command_to_json(&cmd, codepage()).map_err(|e| anyhow!("{e}"))?;
    let (label, count) = (cmd.label.clone(), cmd.edits.len());
    ws.apply(cmd)?;
    if !dry_run && count > 0 {
        ws.save()?;
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
