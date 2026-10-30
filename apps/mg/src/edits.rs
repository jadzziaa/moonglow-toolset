//! `mg apply` and `mg set`: changing a module's fields and resources from
//! the command line, through the same undoable commands the editors use
//! (`mg_edit`), so that nothing else in a resource is touched.

use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use mg_core::Codepage;
use mg_edit::wire::{command_from_json, command_to_json, field_from_str};
use mg_edit::{Command, Edit, GffPath, Workspace};
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
    // What Verify would warn of after, said as it is set.
    let mut warnings = Vec::new();
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
        warnings.extend(pc_faction(key, &path, at, &label, Some(&value)));
        edits.push(Edit::SetField { key, path, label, value: Some(value) });
    }
    for field in remove {
        let (path, label) = field_from_str(field).map_err(|e| anyhow!("{field:?}: {e}"))?;
        let at = path.get(&root).filter(|s| s.get(&label).is_some());
        let Some(at) = at else { bail!("{key} has no field {field}") };
        lines.push(format!("{key} {}: removed", mg_edit::wire::field_to_string(&path, &label)));
        warnings.extend(pc_faction(key, &path, at, &label, None));
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
    let mut out = run(ws, Command::new(label, edits), dry_run, Some(lines), true, codepage, guard)?;
    for w in warnings {
        out.note(w);
    }
    Ok(out)
}

/// A warning that a field set (`value`; `None`: removed) puts an object in
/// the PC faction, as Verify's `pc-faction` warns of it after
/// ([`mg_rules::objects::pc_faction`]): the object a blueprint is, or one
/// an area's GIT places.
fn pc_faction(
    key: ResKey,
    path: &mg_edit::GffPath,
    object: &mg_gff::Struct,
    label: &str,
    value: Option<&Value>,
) -> Option<String> {
    use mg_rules::ObjectKind;
    let kind = match path.0.as_slice() {
        [] => ObjectKind::from_restype(key.restype)?,
        [mg_edit::Step::Item(list, _)] if key.restype == mg_core::ResType::GIT => {
            ObjectKind::from_list(list)?
        }
        _ => return None,
    };
    if kind.faction_field() != Some(label) {
        return None;
    }
    let mut after = object.clone();
    match value {
        Some(v) => after.set(label, v.clone()),
        None => {
            after.remove(label);
        }
    }
    mg_rules::objects::pc_faction(kind, &after)?;
    let place = mg_edit::wire::field_to_string(path, label);
    let what = match value {
        Some(_) => "puts it in the PC faction (0)",
        None => "leaves it no faction, which the game reads as the PC faction (0)",
    };
    Some(format!(
        "warning: {key} {place}: {what}, which is the players' alone; Verify warns of it \
         (pc-faction)"
    ))
}

/// What the tags a command gave (`made`; `set`: the places of the `Tag`
/// fields it sets) put on more than one object of the module, as the
/// window weighs a paste's: by the project's `[tags]`, else each tag the
/// command sets (as a builder's) and each that must be unique. An object
/// it places has its blueprint's tag where the module's blueprint has it
/// (a game's or a hak's blueprint isn't read here: its tag is taken as
/// its own).
fn made_twice(
    ws: &mut Workspace,
    made: &[mg_edit::tags::Made],
    set: &[(ResKey, mg_edit::GffPath)],
) -> Vec<String> {
    let rules = ws.module.project_file.tags.clone();
    let warning = rules.paste_warning_or(true);
    if made.is_empty() || warning == mg_module::project_file::PasteWarning::Off {
        return Vec::new();
    }
    let blueprint =
        |ws: &Workspace, key: ResKey| ws.module.gff(&key).and_then(Result::ok).map(|g| g.root);
    let weighed: Vec<mg_edit::tags::Weighed> = made
        .iter()
        .filter_map(|m| {
            let mut w = mg_edit::tags::weigh(ws, m, &blueprint)?;
            let typed = set.iter().any(|(k, p)| *k == m.key && *p == m.path);
            let found = m.path.0.is_empty() || {
                let doc = ws.doc(&m.key).ok()?;
                let s = m.path.get(&doc.root)?;
                let kind = match m.path.0.last() {
                    Some(mg_edit::Step::Item(list, _)) => mg_rules::ObjectKind::from_list(list),
                    _ => None,
                };
                let template =
                    kind.and_then(|k| Some(ResKey::new(s.resref(k.resref_field())?, k.restype())));
                template.is_some_and(|k| ws.module.contains(&k))
            };
            w.custom = typed || (found && w.custom);
            Some(w)
        })
        .collect();
    let index =
        mg_module::tags::TagCache::default().index(&ws.module, |k| ws.loaded(k).map(|g| &g.root));
    let tags: Vec<mg_module::tags::MadeTag> = weighed
        .iter()
        .map(|w| mg_module::tags::MadeTag {
            tag: &w.tag,
            area: w.area,
            custom: w.custom,
            is_static: w.is_static,
        })
        .collect();
    said_twice(&mg_module::tags::clashes(&index, &tags, &rules, warning))
}

/// What `mg areas --set Tag=…` put on more than one object: each of
/// `areas`' own tag (a builder's), weighed as [`made_twice`] weighs a
/// command's, in the module as it is after.
pub(crate) fn areas_made_twice(m: &Module, areas: &[mg_core::ResRef]) -> Vec<String> {
    let areas: Vec<(ResKey, GffPath)> =
        areas.iter().map(|&a| (ResKey::new(a, mg_core::ResType::ARE), GffPath::root())).collect();
    objects_made_twice(m, &areas)
}

/// What the tags a command set put on more than one object (`objects`:
/// an area's ARE and its root, or its GIT and the path of an object
/// placed in it), each a builder's, weighed as [`made_twice`] weighs a
/// command's, in the module as it is after (`mg areas`, `mg replace`).
pub(crate) fn objects_made_twice(m: &Module, objects: &[(ResKey, GffPath)]) -> Vec<String> {
    let rules = &m.project_file.tags;
    let warning = rules.paste_warning_or(true);
    if objects.is_empty() || warning == mg_module::project_file::PasteWarning::Off {
        return Vec::new();
    }
    let tags: Vec<(mg_core::ResRef, Vec<u8>, bool)> = objects
        .iter()
        .filter_map(|(key, path)| {
            let g = m.gff(key)?.ok()?;
            let s = path.get(&g.root)?;
            let placeable =
                matches!(&path.0[..], [mg_edit::Step::Item(l, _)] if l == "Placeable List");
            let is_static = placeable && s.integer("Static").unwrap_or(0) != 0;
            Some((key.resref, s.string("Tag")?.to_vec(), is_static))
        })
        .collect();
    let made: Vec<mg_module::tags::MadeTag> = (tags.iter())
        .map(|(a, t, is_static)| mg_module::tags::MadeTag {
            tag: t,
            area: *a,
            custom: true,
            is_static: *is_static,
        })
        .collect();
    let index = mg_module::tags::TagIndex::of_module(m);
    said_twice(&mg_module::tags::clashes(&index, &made, rules, warning))
}

/// The warnings of tags made twice.
fn said_twice(clashes: &[mg_module::tags::Clash]) -> Vec<String> {
    (clashes.iter())
        .map(|c| {
            format!("warning: {}: a script finds one of them by it (GetObjectByTag)", c.sentence())
        })
        .collect()
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
    let tagged = mg_edit::tags::made_by(&mut ws, &cmd.edits);
    let set: Vec<(ResKey, mg_edit::GffPath)> = (cmd.edits.iter())
        .filter_map(|e| match e {
            Edit::SetField { key, path, label, .. } if label == "Tag" => Some((*key, path.clone())),
            _ => None,
        })
        .collect();
    ws.apply(cmd)?;
    let mut notes = Output::default();
    for w in made_twice(&mut ws, &tagged, &set) {
        notes.note(w);
    }
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
