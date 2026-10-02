//! `mg`: Moonglow's command-line tool for inspecting and converting game
//! files and the resources a game install provides.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod lsp;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use mg_core::{Codepage, ResType, StrRef};
use mg_erf::{Erf, ErfWriter};
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};

#[derive(Parser)]
#[command(name = "mg", version, about = "Moonglow Toolset command-line tools")]
struct Cli {
    /// Game install (default: $NWN_ROOT or the usual Steam location).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// NWN user directory (default: $NWN_HOME or the platform default).
    #[arg(long, global = true)]
    user_dir: Option<PathBuf>,
    /// Ignore the user directory (base game only).
    #[arg(long, global = true)]
    no_user_dir: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List the contents of an ERF archive (mod, hak, erf, nwm, sav).
    Ls { archive: PathBuf },
    /// Unpack an ERF archive into a directory.
    Unpack { archive: PathBuf, out: PathBuf },
    /// Pack a directory's files into an ERF archive (type from the extension).
    Pack { dir: PathBuf, archive: PathBuf },
    /// Convert GFF to JSON (neverwinter.nim / nasher format), or JSON to GFF.
    Gff {
        input: PathBuf,
        /// Output file (default: stdout for JSON).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show where a resource comes from in the game's load order.
    Which {
        /// `name.ext`, e.g. `classes.2da`.
        resource: String,
    },
    /// Print a resource from the game's load order to stdout.
    Cat { resource: String },
    /// List the resource layers in load order with their sizes.
    Layers,
    /// Print talk-table strings by StrRef.
    Tlk { strrefs: Vec<u32> },
    /// Check a module: references to resources that exist nowhere, and its
    /// custom content's problems (tilesets, 2DAs, materials, objects naming
    /// rows that don't exist). Exits with an error if there are errors.
    Verify {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// Also list module resources nothing references.
        #[arg(long)]
        unused: bool,
        /// Print the results as JSON (for build pipelines).
        #[arg(long)]
        json: bool,
    },
    /// Report the resources a module's haks provide, their conflicts and the
    /// base-game resources they override.
    Haks { module: PathBuf },
    /// Export resources and their dependencies from a module to an ERF.
    Export {
        module: PathBuf,
        /// Resources to export (`name.ext`; an area brings its .git/.gic).
        #[arg(required = true)]
        resources: Vec<String>,
        #[arg(short, long)]
        output: PathBuf,
        /// Description stored in the ERF.
        #[arg(long, default_value = "")]
        comment: String,
        /// Keep custom factions instead of resetting them to their parents.
        #[arg(long)]
        keep_factions: bool,
    },
    /// Compile a module's scripts and save it (Build › Compile).
    Compile {
        module: PathBuf,
        /// Only scripts without a compiled version.
        #[arg(long)]
        uncompiled: bool,
    },
    /// An NWScript language server (the Language Server Protocol on standard
    /// input and output) for VS Code, Neovim and other editors: errors as you
    /// type, definitions, references, rename, hover, completion, outline.
    Lsp,
    /// Where a resource (or, with --tag, a tag) is used in a module, and the
    /// script strings that spell it.
    Refs {
        module: PathBuf,
        /// `name.ext`, e.g. `guard_spawn.nss`, `keep.are`, `guard.utc`.
        name: String,
        /// Look for a tag instead of a resource.
        #[arg(long)]
        tag: bool,
    },
    /// Rename a script, area, conversation or blueprint everywhere in a
    /// module and save it: what names it follows, scripts whose source
    /// changed are compiled again.
    Rename {
        module: PathBuf,
        /// `name.ext` of the resource to rename.
        from: String,
        /// The new name (without extension).
        to: String,
        /// Also change script strings that spell the old name.
        #[arg(long)]
        strings: bool,
    },
    /// Build a nasher project's module: compile its scripts and pack its
    /// target as nasher does (filters, modName and the rest). Fails if a
    /// script doesn't compile, unless --keep-going.
    Build {
        /// The project's folder (with its nasher.cfg).
        project: PathBuf,
        /// The target to pack (default: the target that packs a module).
        #[arg(long)]
        target: Option<String>,
        /// Where to write it (default: the target's file in the project).
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Pack even if scripts fail to compile (as nasher does by default).
        #[arg(long)]
        keep_going: bool,
    },
    /// Put a module into a nasher project: its resources as source files
    /// (GFF as JSON), with a nasher.cfg like `nasher init` writes unless the
    /// folder has one.
    Init {
        /// A module archive or folder.
        module: PathBuf,
        /// The project's folder.
        project: PathBuf,
    },
    /// Import an ERF into a module and save it.
    Import {
        module: PathBuf,
        erf: PathBuf,
        /// Overwrite resources the module already has.
        #[arg(long)]
        overwrite: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        // The reader went away (`mg cat x | head`): not an error.
        Err(e)
            if e.downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe) =>
        {
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("mg: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn install(cli: &Cli) -> Result<GameInstall> {
    let mut gi = match &cli.root {
        Some(r) => GameInstall::new(r, None, "en"),
        None => {
            GameInstall::detect().context("no game install found; pass --root or set NWN_ROOT")?
        }
    };
    if !GameInstall::is_install(&gi.root) {
        bail!("{} is not a game install (no data/nwn_base.key)", gi.root.display());
    }
    if let Some(u) = &cli.user_dir {
        gi.user_dir = Some(u.clone());
    }
    if cli.no_user_dir {
        gi.user_dir = None;
    }
    Ok(gi)
}

fn resource_key(name: &str) -> Result<ResKey> {
    ResKey::from_filename(name)
        .with_context(|| format!("{name:?} is not a resource name like classes.2da"))
}

fn run(cli: Cli) -> Result<()> {
    match &cli.cmd {
        Cmd::Ls { archive } => {
            let data = std::fs::read(archive).with_context(|| archive.display().to_string())?;
            let erf = Erf::read(&data)?;
            let mut out = io::stdout().lock();
            for e in &erf.entries {
                writeln!(out, "{:>10}  {}", e.size, e.filename())?;
            }
        }
        Cmd::Unpack { archive, out } => {
            let data = std::fs::read(archive).with_context(|| archive.display().to_string())?;
            let erf = Erf::read(&data)?;
            std::fs::create_dir_all(out)?;
            for e in &erf.entries {
                std::fs::write(out.join(e.filename()), erf.data(e)?)?;
            }
            eprintln!("unpacked {} files", erf.entries.len());
        }
        Cmd::Pack { dir, archive } => {
            let ext =
                archive.extension().and_then(|e| e.to_str()).unwrap_or("erf").to_ascii_uppercase();
            let mut file_type = [b' '; 4];
            file_type[..ext.len().min(4)].copy_from_slice(&ext.as_bytes()[..ext.len().min(4)]);
            let mut w = ErfWriter::new(file_type);
            let mut paths: Vec<PathBuf> =
                std::fs::read_dir(dir)?.flatten().map(|e| e.path()).collect();
            paths.sort();
            for p in paths.iter().filter(|p| p.is_file()) {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                let key = resource_key(name)?;
                w.add(key.resref, key.restype, std::fs::read(p)?)?;
            }
            // Streamed: a hak can be gigabytes.
            let mut out = io::BufWriter::new(std::fs::File::create(archive)?);
            w.write_to(&mut out)?;
            out.flush()?;
            eprintln!("packed {} files", w.len());
            let past = w.past_read_limit();
            if let Some(first) = past.first() {
                eprintln!(
                    "warning: {} file(s) from {}.{} on start past 2 GiB into the archive, \
                     which the game can't read; split it into two",
                    past.len(),
                    first.0,
                    first.1
                );
            }
        }
        Cmd::Gff { input, output } => gff(input, output.as_deref())?,
        Cmd::Which { resource } => {
            let rm = ResMan::for_game(&install(&cli)?)?;
            let key = resource_key(resource)?;
            match rm.origin(&key) {
                Some(label) => println!("{label}"),
                None => bail!("{key} not found"),
            }
        }
        Cmd::Cat { resource } => {
            let rm = ResMan::for_game(&install(&cli)?)?;
            io::stdout().lock().write_all(&rm.get(&resource_key(resource)?)?)?;
        }
        Cmd::Layers => {
            let rm = ResMan::for_game(&install(&cli)?)?;
            for l in rm.layers() {
                println!("{:>3}  {:>7}  {:?}  {}", l.priority, l.container.len(), l.class, l.label);
            }
        }
        Cmd::Verify { module, unused, json } => {
            if !verify(&install(&cli)?, module, *unused, *json)? {
                bail!("{} has errors", module.display());
            }
        }
        Cmd::Haks { module } => {
            let m = Module::open(module)?;
            let report = mg_module::haks::hak_report(&install(&cli)?, &m.haks()?)?;
            print!("{}", report.to_text());
        }
        Cmd::Export { module, resources, output, comment, keep_factions } => {
            let m = Module::open(module)?;
            let rm = module_resman(&install(&cli)?, &m)?;
            let roots = resources.iter().map(|r| resource_key(r)).collect::<Result<Vec<_>>>()?;
            for r in &roots {
                if !m.contains(r) {
                    bail!("{r} is not in the module");
                }
            }
            let plan = mg_module::transfer::plan_export(&m, &roots, &rm);
            for r in &plan.missing {
                eprintln!(
                    "warning: {} {} needs {:?} {}, found nowhere",
                    r.from, r.path, r.kind, r.target
                );
            }
            let erf =
                mg_module::transfer::export_erf(&m, &plan.resources, comment, !keep_factions)?;
            std::fs::write(output, erf)?;
            eprintln!("exported {} resources", plan.resources.len());
        }
        Cmd::Compile { module, uncompiled } => {
            let mut m = Module::open(module)?;
            let rm = module_resman(&install(&cli)?, &m)?;
            let sel = if *uncompiled {
                mg_module::build::ScriptSelection::Uncompiled
            } else {
                mg_module::build::ScriptSelection::All
            };
            let results = mg_module::build::compile_scripts(&mut m, &rm, sel);
            let failed: Vec<_> = results.iter().filter_map(|r| r.result.as_ref().err()).collect();
            for e in &failed {
                println!("{}", e.message);
            }
            m.save()?;
            eprintln!("compiled {} scripts, {} failed", results.len() - failed.len(), failed.len());
        }
        Cmd::Lsp => {
            // The game's scripts and nwscript.nss, if there is a game.
            let game = match install(&cli) {
                Ok(gi) => ResMan::for_game(&gi).ok(),
                Err(e) => {
                    eprintln!("mg lsp: {e:#}; only the workspace's scripts are known");
                    None
                }
            };
            lsp::serve(game)?;
        }
        Cmd::Refs { module, name, tag } => {
            let m = Module::open(module)?;
            let (usages, mentions) = if *tag {
                (
                    mg_module::rename::tag_usages(&m, name),
                    mg_module::rename::mentions(&m, name, false),
                )
            } else {
                let key = ResKey::from_filename(name).context("give the resource as name.ext")?;
                (
                    mg_module::rename::usages(&m, key),
                    mg_module::rename::mentions(&m, &key.resref.to_string(), true),
                )
            };
            for u in &usages {
                println!("{}\t{} {}", u.place, u.from, u.path);
            }
            for x in &mentions {
                println!("{} line {}: {}", x.script, x.line, x.text);
            }
            eprintln!("{} uses, {} script strings", usages.len(), mentions.len());
        }
        Cmd::Rename { module, from, to, strings } => {
            let mut m = Module::open(module)?;
            let from = ResKey::from_filename(from).context("give the resource as name.ext")?;
            let to = mg_core::ResRef::from_str(to).map_err(|e| anyhow::anyhow!("{to}: {e}"))?;
            let report = mg_module::rename::rename(&mut m, from, to, *strings)?;
            if !report.recompile.is_empty() {
                let rm = module_resman(&install(&cli)?, &m)?;
                let results = mg_module::build::compile_scripts(
                    &mut m,
                    &rm,
                    mg_module::build::ScriptSelection::Uncompiled,
                );
                for r in &results {
                    if let Err(e) = &r.result {
                        println!("{}", e.message);
                    }
                }
            }
            m.save()?;
            let left = mg_module::rename::mentions(&m, &from.resref.to_string(), true);
            for x in &left {
                eprintln!(
                    "warning: {} line {} still spells {}: {}",
                    x.script, x.line, from.resref, x.text
                );
            }
            eprintln!(
                "renamed {from} to {to}: {} references, {} in scripts, {} scripts compiled again",
                report.references,
                report.in_scripts,
                report.recompile.len()
            );
        }
        Cmd::Build { project, target, output, keep_going } => {
            let mut m = Module::open_project(project, target.as_deref())?;
            if let Some(p) = &m.project {
                for w in &p.warnings {
                    eprintln!("warning: {w}");
                }
            }
            let rm = module_resman(&install(&cli)?, &m)?;
            let results = mg_module::build::compile_scripts(
                &mut m,
                &rm,
                mg_module::build::ScriptSelection::All,
            );
            let failed: Vec<_> = results.iter().filter_map(|r| r.result.as_ref().err()).collect();
            for e in &failed {
                println!("{}", e.message);
            }
            if !failed.is_empty() && !keep_going {
                bail!("{} of {} scripts failed to compile", failed.len(), results.len());
            }
            let (path, bytes) = m.target_archive()?.context("not a nasher project")?;
            let path = output.clone().unwrap_or(path);
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, bytes)?;
            eprintln!(
                "compiled {} of {} scripts; packed {}",
                results.len() - failed.len(),
                results.len(),
                path.display()
            );
        }
        Cmd::Init { module, project } => {
            let mut m = Module::open(module)?;
            m.save_as(&ModuleLocation::Project {
                root: project.clone(),
                target: "default".into(),
            })?;
            let p = m.project.as_ref().context("no project")?;
            for w in &p.warnings {
                eprintln!("warning: {w}");
            }
            eprintln!(
                "{} resources in {} (target {}, packs {})",
                m.len(),
                project.display(),
                p.target,
                p.target().file
            );
        }
        Cmd::Import { module, erf, overwrite } => {
            let mut m = Module::open(module)?;
            let data = std::fs::read(erf)?;
            let gi = install(&cli)?;
            let rm = module_resman(&gi, &m)?;
            let plan = mg_module::transfer::plan_import(&m, &data, &rm)?;
            for r in &plan.missing {
                eprintln!("warning: {} needs {:?} {}, found nowhere", r.from, r.kind, r.target);
            }
            let s = mg_module::transfer::import_erf(&mut m, &data, |_| *overwrite)?;
            m.save()?;
            eprintln!(
                "imported {} new, replaced {}, skipped {} existing; new areas: {:?}",
                s.added.len(),
                s.replaced.len(),
                s.skipped.len(),
                s.new_areas.iter().map(|a| a.to_string()).collect::<Vec<_>>()
            );
        }
        Cmd::Tlk { strrefs } => {
            let gi = install(&cli)?;
            let data = std::fs::read(gi.talk_table(false))?;
            let tlk = mg_tlk::Tlk::read(&data)?;
            for &s in strrefs {
                println!("{s}\t{}", tlk.text(StrRef(s)).unwrap_or_default());
            }
        }
    }
    Ok(())
}

/// The resman for a module: the game, the module's haks and the module.
fn module_resman(gi: &GameInstall, m: &Module) -> Result<ResMan> {
    let mut rm = ResMan::for_game(gi)?;
    let haks = m.haks()?;
    for missing in rm.add_haks(gi, &haks.iter().map(String::as_str).collect::<Vec<_>>())? {
        eprintln!("warning: hak {missing} not found");
    }
    rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
    Ok(rm)
}

fn verify(gi: &GameInstall, path: &Path, show_unused: bool, json: bool) -> Result<bool> {
    let m = Module::open(path)?;
    let rm = module_resman(gi, &m)?;
    let missing = mg_module::verify::missing(&m, &rm);
    let unused = if show_unused { mg_module::verify::unused(&m) } else { Vec::new() };
    let count = |data: Vec<u8>| mg_tlk::Tlk::read(&data).map(|t| t.entries.len()).ok();
    let base = std::fs::read(gi.talk_table(false)).ok().and_then(count).unwrap_or(0);
    let custom = m.custom_tlk().ok().flatten().filter(|n| !n.trim().is_empty()).and_then(|name| {
        gi.tlk_dirs()
            .iter()
            .find_map(|d| std::fs::read(d.join(format!("{name}.tlk"))).ok().and_then(count))
    });
    let findings =
        mg_module::doctor::examine(&m, &rm, mg_module::doctor::TalkTables { base, custom });
    let errors = missing.iter().filter(|x| x.is_error()).count()
        + findings.iter().filter(|f| f.severity == mg_module::doctor::Severity::Error).count();
    let warnings = findings.len() + missing.len() - errors;
    if json {
        let severity = |s: mg_module::doctor::Severity| match s {
            mg_module::doctor::Severity::Error => "error",
            mg_module::doctor::Severity::Warning => "warning",
        };
        let out = serde_json::json!({
            "module": path.display().to_string(),
            "errors": errors,
            "warnings": warnings,
            "missing": missing.iter().map(|x| serde_json::json!({
                "severity": if x.is_error() { "error" } else { "warning" },
                "category": format!("{:?}", x.category),
                "from": x.reference.from.to_string(),
                "path": x.reference.path,
                "kind": x.reference.kind.name(),
                "target": x.reference.target.to_string(),
                "uncompiled": x.uncompiled,
            })).collect::<Vec<_>>(),
            "findings": findings.iter().map(|f| serde_json::json!({
                "severity": severity(f.severity),
                "check": f.check,
                "source": f.source,
                "resource": f.resource.to_string(),
                "at": f.at,
                "message": f.message,
            })).collect::<Vec<_>>(),
            "unused": unused.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(errors == 0);
    }
    for x in &missing {
        let what = if x.uncompiled { "not compiled" } else { "missing" };
        let sev = if x.is_error() { "error" } else { "warning" };
        println!(
            "{sev}\t{:?}\t{}{}\t{} {} {what}",
            x.category,
            x.reference.from,
            x.reference.path,
            x.reference.kind.name(),
            x.reference.target
        );
    }
    println!("{} missing references", missing.len());
    for f in &findings {
        let sev = match f.severity {
            mg_module::doctor::Severity::Error => "error",
            mg_module::doctor::Severity::Warning => "warning",
        };
        let at = if f.at.is_empty() { String::new() } else { format!(" {}", f.at) };
        println!("{sev}\t{} › {}{at}: {}", f.source, f.resource, f.message);
    }
    println!("{} content problems", findings.len());
    if show_unused {
        for k in &unused {
            println!("unused\t{k}");
        }
        println!("{} unused resources", unused.len());
    }
    Ok(errors == 0)
}

fn gff(input: &Path, output: Option<&Path>) -> Result<()> {
    let data = std::fs::read(input).with_context(|| input.display().to_string())?;
    let cp = Codepage::WINDOWS_1252;
    if input.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")) {
        let json: serde_json::Value = serde_json::from_slice(&data)?;
        let gff = mg_gff::from_json(&json, cp)?;
        let out = output.context("writing GFF needs --output")?;
        std::fs::write(out, gff.to_bytes()?)?;
    } else {
        let is_gff = input
            .extension()
            .and_then(|e| e.to_str())
            .and_then(ResType::from_extension)
            .is_none_or(ResType::is_gff);
        if !is_gff {
            bail!("{} is not a GFF file type", input.display());
        }
        let gff = Gff::read(&data)?;
        let text = serde_json::to_string_pretty(&mg_gff::to_json(&gff, cp)?)?;
        match output {
            Some(o) => std::fs::write(o, text + "\n")?,
            None => println!("{text}"),
        }
    }
    Ok(())
}
