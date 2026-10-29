//! `mg nui import`: windows brought into a module from files, folders, ERF
//! archives and zip files, as the NUI Creator's Import… brings them
//! (`mg_nui::import`).

use std::path::Path;

use anyhow::{Context, Result, bail};
use mg_edit::Workspace;
use mg_module::Module;
use mg_nui::import::{Found, command, gather, incoming};
use mg_resman::ResKey;
use serde_json::json;

use crate::{Output, path_text};

/// `mg nui import`.
pub(crate) fn import(
    module: &Path,
    sources: &[std::path::PathBuf],
    replace: bool,
    dry_run: bool,
) -> Result<Output> {
    let mut found = Found::new();
    for source in sources {
        gather(source, &mut found).with_context(|| path_text(source))?;
    }
    let mut ws = Workspace::new(Module::open(module)?);
    let windows = incoming(&ws.module, &found)
        .with_context(|| sources.iter().map(|s| path_text(s)).collect::<Vec<_>>().join(", "))?;
    let mut out = Output::default();
    let mut taken = Vec::new();
    for w in &windows {
        for note in &w.notes {
            out.note(format!("warning: {note}"));
        }
        if w.errors > 0 {
            out.note(format!(
                "warning: {}.jui has {} error(s): mg nui validate {} {} lists them",
                w.name,
                w.errors,
                path_text(module),
                w.name
            ));
        }
        taken.extend(w.taken.iter().map(ToString::to_string));
    }
    if !taken.is_empty() && !replace {
        bail!(
            "the module has {} already: --replace to replace {}",
            taken.join(", "),
            if taken.len() == 1 { "it" } else { "them" }
        );
    }
    let made = command(&windows);
    let resources: Vec<ResKey> = made.edits.iter().map(|e| *e.key()).collect();
    let left = super::left_out(&ws.module, &resources);
    let saved = !dry_run && !resources.is_empty();
    if !resources.is_empty() {
        ws.apply(made)?;
    }
    if saved {
        ws.save()?;
    }
    out.json = json!({
        "module": path_text(module),
        "windows": windows.iter().map(|w| json!({
            "window": w.name,
            "from": w.from,
            "resources": w.resources.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "errors": w.errors,
            "warnings": w.warnings,
        })).collect::<Vec<_>>(),
        "resources": resources.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "saved": saved,
        "dry_run": dry_run,
    });
    if let Some(left) = left {
        out.note(format!("warning: {}", left.said(&ws.module)));
        out.json["left_out"] = left.files(&ws.module).into();
        out.json["include"] = left.line.into();
    }
    for r in &resources {
        out.line(r.to_string());
    }
    let what = if dry_run { "would change" } else { "changed" };
    let plural = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    out.line(format!(
        "{}: {what} {}",
        plural(windows.len(), "window"),
        plural(resources.len(), "resource")
    ));
    if saved {
        out.note(format!(
            "mg nui generate {} NAME writes each window's scripts",
            path_text(module)
        ));
    }
    Ok(out)
}
