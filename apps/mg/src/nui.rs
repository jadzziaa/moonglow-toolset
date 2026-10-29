//! `mg nui`: the NUI Creator's windows from the command line, with the
//! checks and the generator the window uses (`mg-nui`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use mg_core::ResType;
use mg_edit::{Command, Workspace};
use mg_module::{Module, build};
use mg_nui::Severity;
use mg_resman::ResKey;
use serde_json::json;

use crate::{Cli, Output, compiled, install, module_resman, path_text};

mod import;
use import::import;
#[cfg(feature = "nui-preview")]
mod preview;
#[cfg(feature = "nui-preview")]
use preview::Drawn;

/// `mg nui`'s commands.
#[derive(Subcommand)]
pub(crate) enum NuiCmd {
    /// Add a new NUI window to a module and save: its layout (NAME.jui, a
    /// window with a Close button) and what the NUI Creator keeps about it
    /// (NAME.txt). Refuses a name whose resources the module already has.
    New {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// The window's name, the ResRef of its resources: 1 to 14 lower
        /// case letters, digits or underscores (its scripts add `_o` and
        /// `_e`).
        name: String,
        /// List the resources it would add; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Check a NUI window as the NUI Creator does: the layout is JSON the
    /// game's NUI functions take, and every bind has a value. Prints each
    /// problem; exits with an error if one is an error. Changes nothing.
    Validate {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// The window's name: the ResRef of its layout (NAME.jui).
        name: String,
    },
    /// Write a NUI window's scripts into a module and save, as Build does
    /// in the NUI Creator: the include that opens it (NAME_o.nss) and its
    /// event script (NAME_e.nss, compiled; the code written into it is
    /// kept), and the module's scripts that include the opener compiled
    /// again, all as `mg compile` compiles. The window's scripts are all
    /// or nothing: a window with errors, an opener or event script edited
    /// by hand, or one that does not compile, changes nothing. A script of
    /// yours that includes the opener and does not compile is printed and
    /// keeps the compiled script it had.
    Generate {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// The window's name: the ResRef of its layout (NAME.jui).
        name: String,
        /// Check and compile, and list the resources it would change;
        /// change nothing.
        #[arg(long)]
        dry_run: bool,
        /// Make each compiled script's debug file (.ndb) too, as `mg
        /// compile --ndb` does (the NUI Creator's Build, with Generate Debug
        /// Information on).
        #[arg(long)]
        ndb: bool,
    },
    /// Draw a NUI window into a PNG as the NUI Creator's preview draws it:
    /// the game's skin, pictures and fonts (from the install and the
    /// module's haks), nothing selected. It is not the game: verify spacing
    /// and events there. Needs a GPU; changes nothing.
    #[cfg(feature = "nui-preview")]
    Render {
        /// A module archive, folder or nasher project; or a window's
        /// NAME.jui file, with its NAME.txt and the pictures it names
        /// beside it.
        module: PathBuf,
        /// The window's name: the ResRef of its layout (not given with a
        /// .jui file).
        name: Option<String>,
        /// The PNG to write (default: NAME.png in the current folder).
        #[arg(long, short)]
        out: Option<PathBuf>,
        #[command(flatten)]
        drawn: Drawn,
    },
    /// Bring NUI windows into a module and save: each window's layout
    /// (NAME.jui), its settings (NAME.txt) and its event script's source
    /// (NAME_e.nss, the code written in its handlers), from .jui files (with
    /// what is beside them), folders, ERF archives (.erf, .hak, .mod, .nwm)
    /// or zip files. The opener is not brought: `mg nui generate` writes the
    /// scripts for the module. A window the module has is refused unless
    /// --replace; all or nothing. Each window is checked as `validate`
    /// checks it.
    Import {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// What to bring in: .jui files, folders, ERF archives or zip files.
        #[arg(required = true)]
        sources: Vec<PathBuf>,
        /// Replace the module's resources of a window of the same name.
        #[arg(long)]
        replace: bool,
        /// List the resources it would add; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the form of a window's settings file (NAME.txt, the format
    /// moonglow.nui/1) as a JSON Schema: every key, its type, its default
    /// and what it does, and each handler's form. With --layout: a new
    /// window's layout, and each control as the NUI Creator adds it.
    Schema {
        /// The layout's controls (NAME.jui) instead of the settings.
        #[arg(long)]
        layout: bool,
    },
    /// Where the NUI Creator's preview puts each control of a NUI window:
    /// its rectangle in the window's points (the window's top left at 0,
    /// 0), and whether it is clipped, overlaps a neighbor, has no room or
    /// cuts its text. The same layout `render` draws. Changes nothing.
    #[cfg(feature = "nui-preview")]
    Layout {
        /// A module archive, folder or nasher project; or a window's
        /// NAME.jui file, with its NAME.txt beside it.
        module: PathBuf,
        /// The window's name (not given with a .jui file).
        name: Option<String>,
        #[command(flatten)]
        drawn: Drawn,
    },
}

pub(crate) fn run(cli: &Cli, cmd: &NuiCmd) -> Result<Output> {
    match cmd {
        NuiCmd::New { module, name, dry_run } => {
            let ws = Workspace::new(crate::open_module(module)?);
            let made =
                mg_nui::create(&ws.module, name).with_context(|| window_text(module, name))?;
            write(ws, made, module, name, *dry_run, Output::default())
        }
        NuiCmd::Generate { module, name, dry_run, ndb } => {
            let ws = Workspace::new(crate::open_module(module)?);
            let mut out = Output::default();
            let gi = install(cli)?;
            let rm = module_resman(&gi, &ws.module, &mut out)?;
            // What `mg compile` and the window remember of the scripts
            // they compiled is kept in one place: these are noted there.
            let dir = build::compiled_dir();
            let made_with = build::made_with(Some(module), None, Some(&gi.root), &rm);
            let known = build::Remembered::kept(dir.as_deref(), Some(module), made_with);
            let how = mg_nui::Build { known: Some(&known), debug: *ndb, ..mg_nui::Build::new(&rm) };
            let made = mg_nui::generate(&ws.module, name, &how)
                .with_context(|| window_text(module, name))?;
            // (A script of the module's that includes the opener and does
            // not compile: said as `mg compile` says it.)
            let failed = compiled(&mut out, &made.failed);
            // (Code of the builder's changed: said, a dry run too.)
            for r in &made.refits {
                out.note(format!(
                    "note: {name}_e.nss:{}: in the section {:?}, {} is now {}, as {}",
                    r.line,
                    r.section,
                    r.was,
                    r.now,
                    if r.now == "return;" {
                        "the event script is a script of its own (void main)".to_owned()
                    } else {
                        format!("{} returns whether the event was the window's", event_fn(name))
                    }
                ));
            }
            let mut out = write(ws, made.command, module, name, *dry_run, out)?;
            out.json["failed"] = failed["failed"].clone();
            out.json["errors"] = failed["errors"].clone();
            out.json["returns"] = serde_json::to_value(&made.refits)?;
            if out.json["saved"] == true
                && let Some(dir) = &dir
                && let Err(e) = made.remembered.write(&build::Remembered::file(dir, module))
            {
                out.note(format!("warning: what was compiled is not remembered: {e}"));
            }
            Ok(out)
        }
        NuiCmd::Validate { module, name } => validate(module, name),
        #[cfg(feature = "nui-preview")]
        NuiCmd::Render { module, name, out, drawn } => {
            preview::render(cli, module, name.as_deref(), out.as_deref(), &drawn.options()?)
        }
        NuiCmd::Import { module, sources, replace, dry_run } => {
            import(module, sources, *replace, *dry_run)
        }
        NuiCmd::Schema { layout } => {
            let schema = if *layout { mg_nui::controls() } else { mg_nui::settings_schema() };
            let mut out = Output::default();
            for line in serde_json::to_string_pretty(&schema)?.lines() {
                out.line(line);
            }
            out.json = schema;
            Ok(out)
        }
        #[cfg(feature = "nui-preview")]
        NuiCmd::Layout { module, name, drawn } => {
            preview::layout(cli, module, name.as_deref(), &drawn.options()?)
        }
    }
}

/// `mg rename MODULE NAME.jui NEW`: a NUI window renamed as the NUI
/// Creator's Rename… renames it (`mg_nui::rename`: its settings, its two
/// scripts and the module's scripts that open it), and built again under
/// its new name as `mg nui generate` builds, the one with the other, and
/// saved. A window that can't be built is renamed all the same, and said.
pub(crate) fn rename(
    cli: &Cli,
    module: &Path,
    from: &str,
    to: &str,
    strings: bool,
) -> Result<Output> {
    let mut ws = Workspace::new(crate::open_module(module)?);
    let mut out = Output::default();
    let mut made =
        mg_nui::rename(&ws.module, from, to, strings).with_context(|| window_text(module, from))?;
    let mut compile = json!(null);
    let mut remembered = None;
    if made.build {
        let gi = install(cli)?;
        let rm = module_resman(&gi, &ws.module, &mut out)?;
        let dir = build::compiled_dir();
        let made_with = build::made_with(Some(module), None, Some(&gi.root), &rm);
        let known = build::Remembered::kept(dir.as_deref(), Some(module), made_with);
        let how = mg_nui::Build { known: Some(&known), ..mg_nui::Build::new(&rm) };
        match mg_nui::build_renamed(&ws.module, &mut made, to, &how) {
            Ok(Some(built)) => {
                compile = compiled(&mut out, &built.failed);
                remembered = Some(built.remembered);
            }
            Ok(None) => {}
            Err(e) => out.note(format!(
                "warning: {to}'s scripts, and the scripts that open it, are not compiled under \
                 its new name ({e}): run mg nui generate"
            )),
        }
    }
    let changed: Vec<String> = made.command.edits.iter().map(|e| e.key().to_string()).collect();
    ws.apply(made.command).with_context(|| window_text(module, from))?;
    ws.save()?;
    if let (Some(now), Some(dir)) = (&remembered, build::compiled_dir())
        && let Err(e) = now.write(&build::Remembered::file(&dir, module))
    {
        out.note(format!("warning: what was compiled is not remembered: {e}"));
    }
    let codepage = crate::scripts_codepage(cli, &ws.module);
    for name in &made.spelled {
        for x in mg_module::rename::mentions(&ws.module, name, true, codepage) {
            out.note(format!(
                "warning: {} line {} still spells {name}: {}",
                x.script, x.line, x.text
            ));
        }
    }
    out.json = json!({
        "from": format!("{from}.jui"),
        "to": to,
        "in_scripts": made.in_scripts,
        "changed": changed,
        "built": remembered.is_some(),
        "compiled": compile,
    });
    out.note(format!(
        "renamed the NUI window {from} to {to}, with its settings and scripts: {} places in \
         scripts{}",
        made.in_scripts,
        if remembered.is_some() { ", built under its new name" } else { "" }
    ));
    Ok(out)
}

/// `NAME_OnEvent()`, the function a window's event script is when its
/// events go to the module's own event script.
fn event_fn(name: &str) -> String {
    format!("{}()", mg_nui::event_function(name))
}

/// The window an error is about: `MODULE: window NAME`.
fn window_text(module: &Path, name: &str) -> String {
    format!("{}: window {name}", path_text(module))
}

/// Applies what `new` or `generate` made (all of it or none) and saves,
/// or with `dry_run` only checks that it applies.
fn write(
    mut ws: Workspace,
    made: Command,
    module: &Path,
    name: &str,
    dry_run: bool,
    mut out: Output,
) -> Result<Output> {
    let mut resources: Vec<ResKey> = Vec::new();
    for e in &made.edits {
        if !resources.contains(e.key()) {
            resources.push(*e.key());
        }
    }
    // (Said before it is written, a dry run too.)
    let left = left_out(&ws.module, &resources);
    ws.apply(made).with_context(|| window_text(module, name))?;
    let saved = !dry_run && !resources.is_empty();
    if saved {
        ws.save()?;
    }
    out.json = json!({
        "module": path_text(module),
        "window": name,
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
    let plural = if resources.len() == 1 { "" } else { "s" };
    out.line(format!("{name}: {what} {} resource{plural}", resources.len()));
    Ok(out)
}

/// A window's files a nasher project's include leaves out: on disk where
/// it doesn't look, or to be written there.
struct LeftOut {
    files: Vec<PathBuf>,
    /// The `include` line that takes them in.
    line: String,
}

impl LeftOut {
    /// The files, as nasher.cfg's patterns name them: from the project's
    /// folder, with `/`.
    fn files(&self, m: &Module) -> Vec<String> {
        let root = m.project.as_deref().map(|p| p.root().to_path_buf()).unwrap_or_default();
        (self.files.iter())
            .map(|f| f.strip_prefix(&root).unwrap_or(f).display().to_string().replace('\\', "/"))
            .collect()
    }

    fn said(&self, m: &Module) -> String {
        let includes = m.project.as_deref().map(|p| p.target().includes.join(", "));
        format!(
            "{}: nasher.cfg's include ({}) leaves {} out: nasher won't pack {} and it won't be \
             read back. Add this line to the target's sources in nasher.cfg: {}",
            self.files(m).join(", "),
            includes.unwrap_or_default(),
            if self.files.len() == 1 { "it" } else { "them" },
            if self.files.len() == 1 { "it" } else { "them" },
            self.line
        )
    }
}

/// Which of `keys` the module's nasher project leaves out, if it is in one.
fn left_out(m: &Module, keys: &[ResKey]) -> Option<LeftOut> {
    let p = m.project.as_deref()?;
    let names: Vec<String> = keys.iter().map(|k| k.to_string().to_ascii_lowercase()).collect();
    let mut files: Vec<PathBuf> = (p.left_out.iter())
        .filter(|f| {
            let name = f.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
            names.contains(&name)
        })
        .cloned()
        .collect();
    files.extend(p.uncovered(keys));
    files.sort();
    files.dedup();
    let line = p.include_for(&files)?;
    Some(LeftOut { files, line })
}

/// `mg nui validate`: the NUI Creator's checks of one window, reported as
/// `mg verify` reports its findings.
fn validate(module: &Path, name: &str) -> Result<Output> {
    let m = crate::open_module(module)?;
    // (Any window the module has: not only one named as `new` names them.)
    let key = |ty: ResType| {
        ResKey::parse(name, ty).ok_or_else(|| anyhow!("{name:?} is not a resource's name"))
    };
    let jui = key(ResType::JUI)?;
    let txt = key(ResType::TXT)?;
    // In a nasher project whose include leaves the window out, that is
    // why it isn't in the module.
    let left = left_out(&m, &[jui, txt]);
    let Some(data) = m.get(&jui) else {
        match left {
            Some(left) => bail!("{jui} is not in the module: {}", left.said(&m)),
            None => bail!("{jui} is not in the module"),
        }
    };
    let window = mg_nui::parse(data).with_context(|| format!("{}: {jui}", path_text(module)))?;
    let settings = m
        .get(&txt)
        .map(mg_nui::Settings::parse)
        .transpose()
        .with_context(|| format!("{}: {txt}", path_text(module)))?
        .unwrap_or_default();
    let mut findings = mg_nui::validate(&window, &settings);
    if let Some(left) = left {
        findings.push(mg_nui::Diagnostic {
            severity: Severity::Warning,
            path: "nasher.cfg".into(),
            message: left.said(&m),
        });
    }
    let errors = findings.iter().filter(|d| d.severity == Severity::Error).count();
    let severity = |s: Severity| match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let mut out = Output::new(json!({
        "module": path_text(module),
        "window": name,
        "errors": errors,
        "warnings": findings.len() - errors,
        "findings": findings.iter().map(|d| json!({
            "severity": severity(d.severity),
            "path": d.path,
            "message": d.message,
        })).collect::<Vec<_>>(),
    }));
    for d in &findings {
        out.line(format!("{}\t{}: {}", severity(d.severity), d.path, d.message));
    }
    out.line(format!("{} problems", findings.len()));
    if errors > 0 {
        out.failed = true;
        out.note(format!("mg: {jui} has errors"));
    }
    Ok(out)
}
