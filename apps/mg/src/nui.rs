use crate::{Cli, Output, install, module_resman};
use anyhow::{Context, Result};
use clap::Subcommand;
use mg_core::ResType;
use mg_edit::Workspace;
use mg_module::Module;
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum NuiCmd {
    /// Add a standard JUI window and its editor settings to a module; save it.
    New { module: PathBuf, name: String },
    /// Read-only API/shape/bind checks. Does not claim NWN runtime acceptance.
    Validate { module: PathBuf, name: String },
    /// Validate the opener include and compile events before saving resources.
    Generate { module: PathBuf, name: String },
}

pub(crate) fn run(cli: &Cli, cmd: &NuiCmd) -> Result<Output> {
    let (path, name) = match cmd {
        NuiCmd::New { module, name }
        | NuiCmd::Validate { module, name }
        | NuiCmd::Generate { module, name } => (module, name),
    };
    mg_nui::check_name(name).map_err(anyhow::Error::msg)?;
    let mut ws = Workspace::new(Module::open(path)?);
    let mut out = Output::default();
    let command = match cmd {
        NuiCmd::New { .. } => mg_nui::create(&ws.module, name).map_err(anyhow::Error::msg)?,
        NuiCmd::Generate { .. } => {
            let rm = module_resman(&install(cli)?, &ws.module, &mut out)?;
            mg_nui::generate(&ws.module, name, |n, t| {
                rm.get_named(n, t).ok().map(|b| b.into_owned())
            })
            .map_err(anyhow::Error::msg)?
        }
        NuiCmd::Validate { .. } => {
            let key = mg_nui::key(name, ResType::JUI);
            let doc = mg_nui::parse(ws.module.get(&key).with_context(|| format!("Missing {key}"))?)
                .map_err(anyhow::Error::msg)?;
            let settings = ws
                .module
                .get(&mg_nui::key(name, ResType::TXT))
                .map(mg_nui::Settings::parse)
                .transpose()
                .map_err(anyhow::Error::msg)?
                .unwrap_or_default();
            let findings = mg_nui::validate(&doc, &settings);
            out.failed = findings.iter().any(|d| d.severity == mg_nui::Severity::Error);
            out.json = json!({"window":name,"diagnostics":findings,"runtime_verified":false});
            for d in findings {
                out.line(format!("{:?} {}: {}", d.severity, d.path, d.message));
            }
            out.note("Checks use the stock NUI API contract. Client appearance/events require NWN Test Module.");
            return Ok(out);
        }
    };
    let resources: Vec<_> = command.edits.iter().map(|e| e.key().to_string()).collect();
    let changed = !command.edits.is_empty();
    if changed {
        ws.apply(command)?;
        ws.module.save()?;
    }
    out.json =
        json!({"window":name,"resources":resources,"saved":changed,"runtime_verified":false});
    out.note(format!("NUI {name}: {} changed resources", resources.len()));
    Ok(out)
}
