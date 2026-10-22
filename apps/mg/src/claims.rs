//! `mg claims`: the project's claims table (`[claims] file` in its
//! moonglow.toml), listed, and areas claimed, released and marked merged
//! as the window's Claim This Area and Release Claim do (The Domains of
//! Dread's builder proposal, C6 and C7).

use std::path::Path;

use anyhow::{Result, bail};
use mg_core::{ResRef, ResType};
use mg_module::Module;
use mg_module::claims::{self, Change, Standing};
use mg_module::project_file::ProjectFile;
use mg_resman::ResKey;
use serde_json::json;

use crate::Output;

/// What `mg claims` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Action {
    /// Every area of the table and the module, whose it is.
    List,
    /// The areas named, claimed by your builder code.
    Claim,
    /// Your claims of the areas named, let go.
    Release,
    /// The areas named marked merged (in the game), with their Tags as
    /// the module has them now: their Tag and ResRef are kept from then on.
    Merged,
}

/// A project's root, from its folder or its nasher.cfg.
fn root_of(project: &Path) -> Result<&Path> {
    let root = match project.file_name() {
        Some(n) if n == "nasher.cfg" => project.parent().unwrap_or(Path::new(".")),
        _ => project,
    };
    if !root.is_dir() {
        bail!("{}: not a project's folder", root.display());
    }
    Ok(root)
}

pub(crate) fn run(project: &Path, action: Action, areas: &[String], force: bool) -> Result<Output> {
    let root = root_of(project)?;
    let p = ProjectFile::read(root);
    let Some(table) = p.claims_path() else {
        bail!(
            "{}: its moonglow.toml names no claims table ([claims] file = \"builders/claims.2da\")",
            root.display()
        );
    };
    let areas: Vec<ResRef> = areas
        .iter()
        .map(|a| ResRef::from_str(a).map_err(|e| anyhow::anyhow!("{a}: {e}")))
        .collect::<Result<_>>()?;
    if action != Action::List && areas.is_empty() {
        bail!("name the areas (their ResRefs)");
    }
    let mut out = Output::default();
    let code = p.user.builder_code.clone();
    match action {
        Action::List => return list(root, &p),
        Action::Claim | Action::Release => {
            let Some(code) = code.as_deref() else {
                bail!(
                    "you have no builder code in this project: mg builder-code {} CODE first",
                    root.display()
                );
            };
            for area in &areas {
                let how = if action == Action::Claim {
                    Change::Claim(code)
                } else {
                    Change::Release(code)
                };
                claims::write_change(&table, *area, how, force).map_err(anyhow::Error::msg)?;
                let done = if action == Action::Claim { "claimed by" } else { "released by" };
                out.line(format!("{area}: {done} {code}"));
            }
        }
        Action::Merged => {
            let m = Module::open(root)?;
            for area in &areas {
                let key = ResKey::new(*area, ResType::ARE);
                let Some(Ok(are)) = m.gff(&key) else {
                    bail!("{area}: the module has no such area");
                };
                let tag = String::from_utf8_lossy(are.root.string("Tag").unwrap_or_default());
                claims::write_merged(&table, *area, &tag).map_err(anyhow::Error::msg)?;
                out.line(format!("{area}: merged, as {tag}"));
            }
        }
    }
    let rel = table.strip_prefix(root).unwrap_or(&table).display().to_string();
    out.note(format!("{rel} written: commit it, so the others see it"));
    out.json =
        json!({ "file": rel, "areas": areas.iter().map(|a| a.to_string()).collect::<Vec<_>>() });
    Ok(out)
}

/// Every area of the table and of the module (where it opens), whose each
/// is to you.
fn list(root: &Path, p: &ProjectFile) -> Result<Output> {
    let table = p.claim_table.clone().unwrap_or_default();
    let mut out = Output::default();
    for line in p.errors.iter().chain(&p.warnings) {
        out.note(line.clone());
    }
    // (The module's areas too, where it opens: those in no row are free.)
    let module = Module::open(root);
    let mut areas: Vec<ResRef> = table.rows.iter().map(|c| c.area).collect();
    let in_module: Vec<ResRef> = match &module {
        Ok(m) => m.keys_of(ResType::ARE).map(|k| k.resref).collect(),
        Err(e) => {
            out.note(format!("the module's areas are not listed: {e}"));
            Vec::new()
        }
    };
    for a in &in_module {
        if !areas.contains(a) {
            areas.push(*a);
        }
    }
    let mut rows = Vec::new();
    for area in areas {
        let standing = Standing::of(&table, area, p.user.builder_code.as_deref());
        let c = standing.claim();
        let whose = match standing {
            Standing::Mine(_) => "yours",
            Standing::Others(_) => "another's",
            Standing::Free(_) => "free",
        };
        let protected = p.protected(area).is_some();
        let mut line = format!("{area}\t{whose}");
        if let Some(h) = c.and_then(|c| c.holder()) {
            line.push_str(&format!("\t{h}"));
        }
        if c.is_some_and(|c| c.merged()) {
            line.push_str("\tmerged");
        } else if protected {
            line.push_str("\tprotected");
        }
        if module.is_ok() && !in_module.contains(&area) {
            line.push_str("\t(not in the module)");
        }
        out.line(line);
        rows.push(json!({
            "area": area.to_string(),
            "builder": c.and_then(|c| c.builder.clone()),
            "status": c.and_then(|c| c.status.clone()),
            "date": c.and_then(|c| c.date.clone()),
            "tag": c.and_then(|c| c.tag.clone()),
            "standing": match standing {
                Standing::Mine(_) => "mine",
                Standing::Others(_) => "others",
                Standing::Free(_) => "free",
            },
            "merged": c.is_some_and(|c| c.merged()),
            "protected": protected,
            "in_module": module.is_err() || in_module.contains(&area),
        }));
    }
    let file = p.claims.file.clone();
    out.json = json!({ "file": file, "code": p.user.builder_code, "areas": rows });
    Ok(out)
}
