//! `mg`: Moonglow's command-line tool for inspecting and converting game
//! files and the resources a game install provides.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use mg_core::{Codepage, ResType, StrRef};
use mg_erf::{Erf, ErfWriter};
use mg_gff::Gff;
use mg_module::Module;
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
    /// Check a module for missing and unused resources.
    Verify {
        /// A module archive or folder.
        module: PathBuf,
        /// Also list module resources nothing references.
        #[arg(long)]
        unused: bool,
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
            std::fs::write(archive, w.to_bytes()?)?;
            eprintln!("packed {} files", w.len());
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
        Cmd::Verify { module, unused } => verify(&install(&cli)?, module, *unused)?,
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

fn verify(gi: &GameInstall, path: &Path, show_unused: bool) -> Result<()> {
    let m = Module::open(path)?;
    let rm = module_resman(gi, &m)?;
    let missing = mg_module::verify::missing(&m, &rm);
    for x in &missing {
        let what = if x.uncompiled { "not compiled" } else { "missing" };
        println!(
            "{:?}\t{}{}\t{:?} {} {what}",
            x.category, x.reference.from, x.reference.path, x.reference.kind, x.reference.target
        );
    }
    println!("{} missing references", missing.len());
    if show_unused {
        let unused = mg_module::verify::unused(&m);
        for k in &unused {
            println!("unused\t{k}");
        }
        println!("{} unused resources", unused.len());
    }
    Ok(())
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
