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
    /// Print the result as one JSON object, for scripts and build
    /// pipelines (warnings in its "notes"; an error as {"error": …}).
    #[arg(long, global = true)]
    json: bool,
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
    /// Write a conversation as plain text, CSV, Twine (Twee) or Ink, by the
    /// output's extension (.txt, .csv, .twee, .ink).
    DialogExport {
        module: PathBuf,
        /// The conversation, `name.dlg`.
        dialog: String,
        output: PathBuf,
    },
    /// Read a Twine (.twee) or Ink (.ink) story into a module as a
    /// conversation (replacing one of the same name) and save; or, from a
    /// .csv export, read back the text of the conversation's lines.
    DialogImport {
        module: PathBuf,
        file: PathBuf,
        /// The conversation's name (default: the file's).
        #[arg(long)]
        name: Option<String>,
    },
    /// Replace text in a module's names, descriptions, conversation lines
    /// and journal (every language they're written in) and save; prints
    /// each string changed.
    Replace {
        module: PathBuf,
        find: String,
        with: String,
        #[arg(long)]
        match_case: bool,
        /// Only where it isn't part of a longer word.
        #[arg(long)]
        whole_word: bool,
        /// Only these kinds: names, descriptions, conversations, journal,
        /// other (comma-separated).
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
        /// List what would change; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// List a module's areas, or set properties of several at once and
    /// save. Which areas: those named, narrowed by the filters (all that are
    /// given must hold). With `--set`, `--var` or `--remove-var`, each
    /// change is made to each of them: all of the changes, or none if one
    /// cannot be made.
    Areas {
        module: PathBuf,
        /// Only these areas (ResRefs); none: every area.
        areas: Vec<String>,
        /// Areas with this text in their name, tag or ResRef.
        #[arg(long = "match")]
        text: Option<String>,
        /// Areas of this tileset (its ResRef, e.g. `tdc01`).
        #[arg(long)]
        tileset: Option<String>,
        #[arg(long, conflicts_with = "exterior")]
        interior: bool,
        #[arg(long)]
        exterior: bool,
        #[arg(long, conflicts_with = "above_ground")]
        underground: bool,
        #[arg(long)]
        above_ground: bool,
        #[arg(long, conflicts_with = "artificial")]
        natural: bool,
        #[arg(long)]
        artificial: bool,
        /// `FIELD=VALUE`: a field of the area (`SunFogAmount=5`,
        /// `MoonAmbientColor=0x402010`, `OnEnter=my_script`), of its
        /// ambient sounds and music (`MusicDay=57`), or a flag
        /// (`Interior`, `Underground`, `Natural`: `yes` or `no`). A field
        /// keeps its type; a number may be hexadecimal (`0x…`).
        #[arg(long = "set")]
        sets: Vec<String>,
        /// `NAME=VALUE` or `NAME:int=VALUE` (`int`, `float`, `string`): a
        /// scripting variable to set on each area, which keeps its others.
        #[arg(long = "var")]
        vars: Vec<String>,
        /// A scripting variable to delete from each area.
        #[arg(long)]
        remove_var: Vec<String>,
        /// Change every area of the module (needed when no area is named
        /// and no filter given).
        #[arg(long)]
        all: bool,
        /// List what would change; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Make the objects placed from blueprints again from them, where they
    /// stand (Aurora's Update Instances), and save: their tags, names,
    /// scripts and variables become the blueprints'.
    UpdateInstances {
        module: PathBuf,
        /// `name.ext` of each blueprint, e.g. `guard.utc`; none: every
        /// blueprint in the module.
        blueprints: Vec<String>,
        /// Only in this area.
        #[arg(long)]
        area: Option<String>,
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
    /// Attach haks and a talk table to a module: copies them into the user
    /// folder's hak and tlk (where the game looks), lists the haks at the
    /// top of the module's hak list in the order given, names the talk
    /// table, and saves the module.
    Attach {
        module: PathBuf,
        /// .hak and .tlk files, highest priority first.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Replace a different file of the same name in the user folder.
        #[arg(long)]
        replace: bool,
    },
    /// Write an area's minimap as a PNG: each tile's picture, turned as the
    /// tile is, as the game's map shows it.
    Minimap {
        module: PathBuf,
        /// The area's resref.
        area: String,
        /// The PNG to write.
        out: PathBuf,
        /// Pixels a tile (default: the pictures' size).
        #[arg(long)]
        size: Option<u32>,
    },
    /// Find a module's blueprints and the objects placed in its areas, by
    /// type, tag, name, resref, area or field values.
    Find {
        module: PathBuf,
        /// Blueprint types: utc, utd, ute, uti, utp, uts, utm, utt, utw
        /// (comma-separated; default all).
        #[arg(long = "type", value_delimiter = ',')]
        types: Vec<String>,
        /// The tag (`*` matches any run of characters; case ignored).
        #[arg(long)]
        tag: Option<String>,
        /// Words the name contains.
        #[arg(long)]
        name: Option<String>,
        /// The blueprint's resref (a placed object's blueprint); `*` as in tags.
        #[arg(long)]
        resref: Option<String>,
        /// Only objects placed in this area.
        #[arg(long)]
        area: Option<String>,
        /// Only placed objects.
        #[arg(long, conflicts_with = "blueprints")]
        placed: bool,
        /// Only blueprints.
        #[arg(long)]
        blueprints: bool,
        /// A field and its value, `Label=Value` (`*` as in tags), or `Label`
        /// for any value; repeatable.
        #[arg(long = "where")]
        fields: Vec<String>,
    },
    /// What a module is: its name, tag, areas, haks, talk table and
    /// resources by type.
    Info { module: PathBuf },
    /// Make a tileset's palette (<tileset>palstd.itp) from its .set: its
    /// groups, features, terrains and crossers, as the painter offers them.
    TilesetPalette {
        /// The .set file.
        set: PathBuf,
        /// Where to write it (default: beside the .set).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Publish a module's haks and talk table for NWSync: write a manifest
    /// and its data into a repository folder, for a web server to serve and
    /// nwserver to name (-nwsyncurl), as nwn_nwsync_write does.
    Nwsync {
        module: PathBuf,
        /// The repository folder (made if missing).
        repository: PathBuf,
        /// The module's own resources too, for single-player distribution
        /// (not for persistent worlds).
        #[arg(long)]
        with_module: bool,
        /// The module's UUID, with --with-module (default: its own, or a
        /// new one).
        #[arg(long)]
        uuid: Option<String>,
        /// The name players see (default with --with-module: the module's).
        #[arg(long)]
        name: Option<String>,
        /// The description players see (default with --with-module: the
        /// module's).
        #[arg(long)]
        description: Option<String>,
        /// For servers sharing a repository: the game removes a group's
        /// older downloads.
        #[arg(long, default_value_t = 0)]
        group_id: u32,
        /// Don't point `latest` at the new manifest.
        #[arg(long)]
        no_latest: bool,
        /// The largest file allowed, in megabytes (0: no limit).
        #[arg(long, default_value_t = 15)]
        limit_file_size: u64,
        /// Write data files that are there already again.
        #[arg(long)]
        force: bool,
        /// Work it out; write nothing.
        #[arg(long)]
        dry_run: bool,
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

/// What a command produced: its result as JSON, and the same for people
/// as lines of standard output and notes (warnings, summaries) on
/// standard error.
#[derive(Default)]
struct Output {
    json: serde_json::Value,
    text: Vec<String>,
    notes: Vec<String>,
    /// Found errors (verify): printed, then a failing exit.
    failed: bool,
    /// Written already (cat's bytes, the language server).
    done: bool,
}

impl Output {
    fn new(json: serde_json::Value) -> Output {
        Output { json, ..Default::default() }
    }

    fn line(&mut self, s: impl Into<String>) {
        self.text.push(s.into());
    }

    fn note(&mut self, s: impl Into<String>) {
        self.notes.push(s.into());
    }

    /// Prints it as text, or as JSON with the notes in "notes".
    fn print(mut self, json: bool) -> io::Result<()> {
        if self.done {
            return Ok(());
        }
        let mut out = io::stdout().lock();
        if json {
            if let serde_json::Value::Object(o) = &mut self.json
                && !self.notes.is_empty()
            {
                o.insert("notes".into(), self.notes.into());
            }
            let text = serde_json::to_string_pretty(&self.json).map_err(io::Error::other)?;
            writeln!(out, "{text}")?;
        } else {
            for l in &self.text {
                writeln!(out, "{l}")?;
            }
            for n in &self.notes {
                eprintln!("{n}");
            }
        }
        Ok(())
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    let result = run(&cli).and_then(|out| {
        let failed = out.failed;
        out.print(json)?;
        Ok(failed)
    });
    match result {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        // The reader went away (`mg cat x | head`): not an error.
        Err(e)
            if e.downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe) =>
        {
            ExitCode::SUCCESS
        }
        Err(e) => {
            if json {
                println!("{}", serde_json::json!({ "error": format!("{e:#}") }));
            } else {
                eprintln!("mg: {e:#}");
            }
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

fn path_text(p: &Path) -> String {
    p.display().to_string()
}

/// Compile results: errors as lines and as JSON.
fn compiled(out: &mut Output, results: &[mg_module::build::ScriptResult]) -> serde_json::Value {
    let mut errors = Vec::new();
    for r in results {
        if let Err(e) = &r.result {
            out.line(e.message.clone());
            let (script, line) =
                e.location().map_or((r.script.resref.to_string(), None), |(s, l)| (s, Some(l)));
            errors
                .push(serde_json::json!({ "script": script, "line": line, "message": e.message }));
        }
    }
    serde_json::json!({ "scripts": results.len(), "failed": errors.len(), "errors": errors })
}

fn run(cli: &Cli) -> Result<Output> {
    use serde_json::json;
    let out = match &cli.cmd {
        Cmd::Ls { archive } => {
            let data = std::fs::read(archive).with_context(|| path_text(archive))?;
            let erf = Erf::read(&data)?;
            let mut out = Output::new(json!({
                "archive": path_text(archive),
                "type": String::from_utf8_lossy(&erf.file_type).trim(),
                "resources": erf.entries.iter().map(|e| json!({ "name": e.filename(), "size": e.size })).collect::<Vec<_>>(),
            }));
            for e in &erf.entries {
                out.line(format!("{:>10}  {}", e.size, e.filename()));
            }
            out
        }
        Cmd::Unpack { archive, out: dir } => {
            let data = std::fs::read(archive).with_context(|| path_text(archive))?;
            let erf = Erf::read(&data)?;
            std::fs::create_dir_all(dir)?;
            for e in &erf.entries {
                std::fs::write(dir.join(e.filename()), erf.data(e)?)?;
            }
            let mut out =
                Output::new(json!({ "folder": path_text(dir), "files": erf.entries.len() }));
            out.note(format!("unpacked {} files", erf.entries.len()));
            out
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
            let mut file = io::BufWriter::new(std::fs::File::create(archive)?);
            w.write_to(&mut file)?;
            file.flush()?;
            let past = w.past_read_limit();
            let mut out = Output::new(json!({
                "archive": path_text(archive),
                "files": w.len(),
                "past_read_limit": past.iter().map(|(r, t)| format!("{r}.{t}")).collect::<Vec<_>>(),
            }));
            out.note(format!("packed {} files", w.len()));
            if let Some(first) = past.first() {
                out.note(format!(
                    "warning: {} file(s) from {}.{} on start past 2 GiB into the archive, \
                     which the game can't read; split it into two",
                    past.len(),
                    first.0,
                    first.1
                ));
            }
            out
        }
        Cmd::Gff { input, output } => gff(input, output.as_deref(), cli.json)?,
        Cmd::Which { resource } => {
            let rm = ResMan::for_game(&install(cli)?)?;
            let key = resource_key(resource)?;
            let Some(label) = rm.origin(&key) else { bail!("{key} not found") };
            let mut out = Output::new(json!({ "resource": key.to_string(), "layer": label }));
            out.line(label);
            out
        }
        Cmd::Cat { resource } => {
            let rm = ResMan::for_game(&install(cli)?)?;
            let key = resource_key(resource)?;
            let data = rm.get(&key)?;
            if cli.json {
                // Text as text; other bytes as hexadecimal.
                let text = std::str::from_utf8(&data).ok().map(str::to_string);
                let hex = text
                    .is_none()
                    .then(|| data.iter().map(|b| format!("{b:02x}")).collect::<String>());
                Output::new(json!({
                    "resource": key.to_string(),
                    "layer": rm.origin(&key),
                    "size": data.len(),
                    "text": text,
                    "hex": hex,
                }))
            } else {
                io::stdout().lock().write_all(&data)?;
                Output { done: true, ..Default::default() }
            }
        }
        Cmd::Layers => {
            let rm = ResMan::for_game(&install(cli)?)?;
            let mut out = Output::new(json!({
                "layers": rm.layers().iter().map(|l| json!({
                    "label": l.label,
                    "priority": l.priority,
                    "class": format!("{:?}", l.class),
                    "resources": l.container.len(),
                })).collect::<Vec<_>>(),
            }));
            for l in rm.layers() {
                out.line(format!(
                    "{:>3}  {:>7}  {:?}  {}",
                    l.priority,
                    l.container.len(),
                    l.class,
                    l.label
                ));
            }
            out
        }
        Cmd::Verify { module, unused } => verify(&install(cli)?, module, *unused)?,
        Cmd::Haks { module } => {
            let m = Module::open(module)?;
            let report = mg_module::haks::hak_report(&install(cli)?, &m.haks()?)?;
            let rows = |it: &mut dyn Iterator<Item = (&ResKey, &Vec<String>)>| {
                it.map(|(k, h)| json!({ "resource": k.to_string(), "haks": h })).collect::<Vec<_>>()
            };
            let mut out = Output::new(json!({
                "haks": report.haks.iter().map(|(n, p)| json!({ "name": n, "path": p.as_deref().map(path_text) })).collect::<Vec<_>>(),
                "conflicts": rows(&mut report.conflicts()),
                "overrides": rows(&mut report.overrides.iter()),
                "resources": report.resources.len(),
            }));
            out.line(report.to_text().trim_end());
            out
        }
        Cmd::Export { module, resources, output, comment, keep_factions } => {
            let m = Module::open(module)?;
            let mut out = Output::default();
            let rm = module_resman(&install(cli)?, &m, &mut out)?;
            let roots = resources.iter().map(|r| resource_key(r)).collect::<Result<Vec<_>>>()?;
            for r in &roots {
                if !m.contains(r) {
                    bail!("{r} is not in the module");
                }
            }
            let plan = mg_module::transfer::plan_export(&m, &roots, &rm);
            for r in &plan.missing {
                out.note(format!(
                    "warning: {} {} needs {:?} {}, found nowhere",
                    r.from, r.path, r.kind, r.target
                ));
            }
            let erf =
                mg_module::transfer::export_erf(&m, &plan.resources, comment, !keep_factions)?;
            std::fs::write(output, erf)?;
            out.json = json!({
                "erf": path_text(output),
                "resources": plan.resources.iter().map(ToString::to_string).collect::<Vec<_>>(),
            });
            out.note(format!("exported {} resources", plan.resources.len()));
            out
        }
        Cmd::Compile { module, uncompiled } => {
            let mut m = Module::open(module)?;
            let mut out = Output::default();
            let rm = module_resman(&install(cli)?, &m, &mut out)?;
            let sel = if *uncompiled {
                mg_module::build::ScriptSelection::Uncompiled
            } else {
                mg_module::build::ScriptSelection::All
            };
            let results = mg_module::build::compile_scripts(&mut m, &rm, sel);
            out.json = compiled(&mut out, &results);
            m.save()?;
            let failed = out.json["failed"].as_u64().unwrap_or(0) as usize;
            out.note(format!("compiled {} scripts, {failed} failed", results.len() - failed));
            out
        }
        Cmd::Lsp => {
            if cli.json {
                bail!("mg lsp speaks the Language Server Protocol, which is JSON already");
            }
            // The game's scripts and nwscript.nss, if there is a game.
            let game = match install(cli) {
                Ok(gi) => ResMan::for_game(&gi).ok(),
                Err(e) => {
                    eprintln!("mg lsp: {e:#}; only the workspace's scripts are known");
                    None
                }
            };
            lsp::serve(game)?;
            Output { done: true, ..Default::default() }
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
            let mut out = Output::new(json!({
                "uses": usages.iter().map(|u| json!({ "place": u.place, "from": u.from.to_string(), "path": u.path })).collect::<Vec<_>>(),
                "script_strings": mentions.iter().map(|x| json!({ "script": x.script.to_string(), "line": x.line, "text": x.text })).collect::<Vec<_>>(),
            }));
            for u in &usages {
                out.line(format!("{}\t{} {}", u.place, u.from, u.path));
            }
            for x in &mentions {
                out.line(format!("{} line {}: {}", x.script, x.line, x.text));
            }
            out.note(format!("{} uses, {} script strings", usages.len(), mentions.len()));
            out
        }
        Cmd::Rename { module, from, to, strings } => {
            let mut m = Module::open(module)?;
            let from = ResKey::from_filename(from).context("give the resource as name.ext")?;
            let to = mg_core::ResRef::from_str(to).map_err(|e| anyhow::anyhow!("{to}: {e}"))?;
            let report = mg_module::rename::rename(&mut m, from, to, *strings)?;
            let mut out = Output::default();
            let mut compile = json!(null);
            if !report.recompile.is_empty() {
                let rm = module_resman(&install(cli)?, &m, &mut out)?;
                let results = mg_module::build::compile_scripts(
                    &mut m,
                    &rm,
                    mg_module::build::ScriptSelection::Uncompiled,
                );
                compile = compiled(&mut out, &results);
            }
            m.save()?;
            let left = mg_module::rename::mentions(&m, &from.resref.to_string(), true);
            for x in &left {
                out.note(format!(
                    "warning: {} line {} still spells {}: {}",
                    x.script, x.line, from.resref, x.text
                ));
            }
            out.json = json!({
                "from": from.to_string(),
                "to": to.to_string(),
                "references": report.references,
                "in_scripts": report.in_scripts,
                "changed": report.changed.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "compiled": compile,
            });
            out.note(format!(
                "renamed {from} to {to}: {} references, {} in scripts, {} scripts compiled again",
                report.references,
                report.in_scripts,
                report.recompile.len()
            ));
            out
        }
        Cmd::DialogExport { module, dialog, output } => {
            use mg_module::dialog_io::Format;
            let m = Module::open(module)?;
            let key = ResKey::from_filename(dialog).context("give the conversation as name.dlg")?;
            let g = m.gff(&key).with_context(|| format!("{key} is not in the module"))??;
            let f = Format::of(output).context("the output must end .txt, .csv, .twee or .ink")?;
            std::fs::write(output, f.write(&g, &key.resref.to_string()))?;
            Output::new(
                json!({ "conversation": key.to_string(), "file": path_text(output), "format": f.name() }),
            )
        }
        Cmd::DialogImport { module, file, name } => {
            use mg_module::dialog_io::{Format, from_ink, from_twee, update_from_csv};
            let mut m = Module::open(module)?;
            let stem = file.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase());
            let name = name.clone().or(stem).context("name the conversation with --name")?;
            let key = ResKey::parse(&name, mg_core::ResType::DLG)
                .with_context(|| format!("{name:?} isn't a resource name"))?;
            let source = std::fs::read_to_string(file)?;
            let mut out = Output::default();
            let mut changed = None;
            let g = match Format::of(file) {
                Some(Format::Twine) => from_twee(&source).map_err(|e| anyhow::anyhow!("{e}"))?,
                Some(Format::Ink) => from_ink(&source).map_err(|e| anyhow::anyhow!("{e}"))?,
                Some(Format::Csv) => {
                    let mut g =
                        m.gff(&key).with_context(|| format!("{key} is not in the module"))??;
                    let n = update_from_csv(&mut g, &source).map_err(|e| anyhow::anyhow!("{e}"))?;
                    out.note(format!("{n} lines changed"));
                    changed = Some(n);
                    g
                }
                _ => bail!("{}: not a .twee, .ink or .csv file", file.display()),
            };
            m.set_gff(key, &g)?;
            m.save()?;
            let lines = mg_module::dialog::nodes(&g, mg_module::dialog::Kind::Entry).len()
                + mg_module::dialog::nodes(&g, mg_module::dialog::Kind::Reply).len();
            out.json =
                json!({ "conversation": key.to_string(), "lines": lines, "changed": changed });
            out.note(format!("{key}: {lines} lines"));
            out
        }
        Cmd::Replace { module, find, with, match_case, whole_word, only, dry_run } => {
            use mg_module::text::{Options, TextKind};
            let kinds: Vec<TextKind> = if only.is_empty() {
                TextKind::ALL.to_vec()
            } else {
                only.iter()
                    .map(|o| {
                        TextKind::ALL
                            .into_iter()
                            .find(|k| k.label().to_lowercase().starts_with(&o.to_lowercase()))
                            .with_context(|| {
                                format!(
                                    "{o}: not names, descriptions, conversations, journal or other"
                                )
                            })
                    })
                    .collect::<Result<_>>()?
            };
            let o = Options { match_case: *match_case, whole_word: *whole_word };
            let mut m = Module::open(module)?;
            let hits = mg_module::text::find(&m, find, o, &kinds);
            let mut out = Output::default();
            for h in &hits {
                out.line(format!("{}\t{}", h.place, h.text));
            }
            let times: usize = hits.iter().map(|h| h.count).sum();
            let mut replaced = 0;
            if *dry_run || hits.is_empty() {
                out.note(format!("{} strings, {times} times", hits.len()));
            } else {
                let (n, errors) = mg_module::text::replace(&mut m, &hits, find, with, o);
                for e in &errors {
                    out.note(format!("warning: {e}"));
                }
                m.save()?;
                replaced = n;
                out.note(format!("replaced {n} times in {} strings", hits.len() - errors.len()));
            }
            out.json = json!({
                "strings": hits.iter().map(|h| json!({ "place": h.place, "resource": h.key.to_string(), "kind": h.kind.label(), "text": h.text, "times": h.count })).collect::<Vec<_>>(),
                "times": times,
                "replaced": replaced,
                "dry_run": *dry_run,
            });
            out
        }
        Cmd::Areas {
            module,
            areas,
            text,
            tileset,
            interior,
            exterior,
            underground,
            above_ground,
            natural,
            artificial,
            sets,
            vars,
            remove_var,
            all,
            dry_run,
        } => {
            use mg_module::areas::{self, Change, Filter};
            let resref =
                |s: &String| mg_core::ResRef::from_str(s).map_err(|e| anyhow::anyhow!("{s}: {e}"));
            // `--interior` or `--exterior`: so, not so, or either.
            let kind = |yes: bool, no: bool| if yes { Some(true) } else { no.then_some(false) };
            let filter = Filter {
                only: areas.iter().map(resref).collect::<Result<_>>()?,
                text: text.clone(),
                tileset: tileset.as_ref().map(resref).transpose()?,
                interior: kind(*interior, *exterior),
                underground: kind(*underground, *above_ground),
                natural: kind(*natural, *artificial),
            };
            let mut changes: Vec<Change> = Vec::new();
            for s in sets {
                changes.push(Change::parse_set(s).map_err(anyhow::Error::msg)?);
            }
            for v in vars {
                changes.push(Change::parse_var(v).map_err(anyhow::Error::msg)?);
            }
            changes.extend(remove_var.iter().cloned().map(Change::RemoveVar));
            let mut m = Module::open(module)?;
            let listed = areas::list(&m);
            for a in &filter.only {
                if !listed.iter().any(|l| l.resref == *a) {
                    bail!("{a} is not an area of the module");
                }
            }
            let chosen: Vec<&areas::AreaInfo> =
                listed.iter().filter(|a| filter.matches(a)).collect();
            if !changes.is_empty() && filter.is_empty() && !*all {
                bail!("name areas or give a filter, or pass --all to change every area");
            }
            let resrefs: Vec<mg_core::ResRef> = chosen.iter().map(|a| a.resref).collect();
            let done = areas::apply(&mut m, &resrefs, &changes).map_err(anyhow::Error::msg)?;
            let mut out = Output::default();
            let kinds = |flags: u32| {
                let word = |bit: u32, yes: &'static str, no: &'static str| {
                    if flags & bit != 0 { yes } else { no }
                };
                [
                    word(areas::INTERIOR, "interior", "exterior"),
                    word(areas::UNDERGROUND, "underground", "above ground"),
                    word(areas::NATURAL, "natural", "artificial"),
                ]
                .join(", ")
            };
            if changes.is_empty() {
                for a in &chosen {
                    out.line(format!(
                        "{}\t{}\t{}\t{}",
                        a.resref,
                        a.name,
                        a.tileset,
                        kinds(a.flags)
                    ));
                }
                out.note(format!("{} of {} areas", chosen.len(), listed.len()));
            } else {
                let text = |v: &Option<String>| v.clone().unwrap_or_else(|| "(none)".into());
                for c in &done {
                    out.line(format!(
                        "{}\t{}\t{} -> {}",
                        c.area,
                        c.what,
                        text(&c.from),
                        text(&c.to)
                    ));
                }
                let changed: std::collections::BTreeSet<_> = done.iter().map(|c| c.area).collect();
                if *dry_run {
                    out.note(format!(
                        "would change {} of {} areas chosen",
                        changed.len(),
                        chosen.len()
                    ));
                } else {
                    if !done.is_empty() {
                        m.save()?;
                    }
                    out.note(format!("changed {} of {} areas chosen", changed.len(), chosen.len()));
                }
            }
            out.json = json!({
                "areas": chosen.iter().map(|a| json!({
                    "resref": a.resref.to_string(),
                    "name": a.name,
                    "tag": a.tag,
                    "tileset": a.tileset.to_string(),
                    "interior": a.flags & areas::INTERIOR != 0,
                    "underground": a.flags & areas::UNDERGROUND != 0,
                    "natural": a.flags & areas::NATURAL != 0,
                })).collect::<Vec<_>>(),
                "changes": done.iter().map(|c| json!({
                    "area": c.area.to_string(),
                    "field": c.what,
                    "from": c.from,
                    "to": c.to,
                })).collect::<Vec<_>>(),
                "dry_run": *dry_run,
            });
            out
        }
        Cmd::UpdateInstances { module, blueprints, area } => {
            let mut m = Module::open(module)?;
            let gi = install(cli)?;
            let wanted: Vec<ResKey> = if blueprints.is_empty() {
                let types: Vec<_> =
                    mg_module::instances::GIT_LISTS.iter().map(|(_, t)| *t).collect();
                m.keys().copied().filter(|k| types.contains(&k.restype)).collect()
            } else {
                blueprints
                    .iter()
                    .map(|b| ResKey::from_filename(b).context("give each blueprint as name.ext"))
                    .collect::<Result<_>>()?
            };
            for k in &wanted {
                if !m.contains(k) {
                    bail!("{k} is not in the module");
                }
            }
            let mut out = Output::default();
            let tlk = mg_tlk::Tlk::read(&std::fs::read(gi.talk_table(false))?)?;
            let game = mg_rules::GameData::new(module_resman(&gi, &m, &mut out)?, tlk);
            let only = area
                .as_deref()
                .map(mg_core::ResRef::from_str)
                .transpose()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut total = 0;
            let mut by_area = serde_json::Map::new();
            for a in m.areas()? {
                if only.is_some_and(|o| o != a) {
                    continue;
                }
                let key = ResKey::new(a, mg_core::ResType::GIT);
                let Some(Ok(git)) = m.gff(&key) else { continue };
                let read = |k: ResKey| -> Option<mg_gff::Struct> {
                    let data = m
                        .get(&k)
                        .map(<[u8]>::to_vec)
                        .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
                    mg_gff::Gff::read(&data).ok().map(|g| g.root)
                };
                let item = |r: mg_core::ResRef| read(ResKey::new(r, mg_core::ResType::UTI));
                let placing = mg_module::instances::Placing { game: &game, item: &item };
                let blueprint = |t: mg_core::ResType, r: mg_core::ResRef| {
                    let k = ResKey::new(r, t);
                    if wanted.contains(&k) { read(k) } else { None }
                };
                if let Some((new, n)) =
                    mg_module::instances::update(&placing, &git.root, &blueprint, &|_, _| true)
                {
                    let mut g = git.clone();
                    g.root = new;
                    m.set_gff(key, &g)?;
                    out.line(format!("{a}: {n}"));
                    by_area.insert(a.to_string(), n.into());
                    total += n;
                }
            }
            if total > 0 {
                m.save()?;
            }
            out.json = json!({ "updated": total, "areas": by_area, "blueprints": wanted.len() });
            out.note(format!("updated {total} objects from {} blueprints", wanted.len()));
            out
        }
        Cmd::Build { project, target, output, keep_going } => {
            let mut m = Module::open_project(project, target.as_deref())?;
            let mut out = Output::default();
            if let Some(p) = &m.project {
                for w in &p.warnings {
                    out.note(format!("warning: {w}"));
                }
            }
            let rm = module_resman(&install(cli)?, &m, &mut out)?;
            let results = mg_module::build::compile_scripts(
                &mut m,
                &rm,
                mg_module::build::ScriptSelection::All,
            );
            let compile = compiled(&mut out, &results);
            let failed = compile["failed"].as_u64().unwrap_or(0) as usize;
            if failed > 0 && !keep_going {
                // The compiler's messages, then the failure.
                if cli.json {
                    out.json = json!({ "compiled": compile, "packed": null });
                    out.failed = true;
                    return Ok(out);
                }
                for l in &out.text {
                    println!("{l}");
                }
                bail!("{failed} of {} scripts failed to compile", results.len());
            }
            let (path, bytes) = m.target_archive()?.context("not a nasher project")?;
            let path = output.clone().unwrap_or(path);
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, bytes)?;
            out.json = json!({ "compiled": compile, "packed": path_text(&path) });
            out.note(format!(
                "compiled {} of {} scripts; packed {}",
                results.len() - failed,
                results.len(),
                path.display()
            ));
            out
        }
        Cmd::Init { module, project } => {
            let mut m = Module::open(module)?;
            m.save_as(&ModuleLocation::Project {
                root: project.clone(),
                target: "default".into(),
            })?;
            let p = m.project.as_ref().context("no project")?;
            let mut out = Output::new(json!({
                "project": path_text(project),
                "resources": m.len(),
                "target": p.target,
                "file": p.target().file,
            }));
            for w in &p.warnings {
                out.note(format!("warning: {w}"));
            }
            out.note(format!(
                "{} resources in {} (target {}, packs {})",
                m.len(),
                project.display(),
                p.target,
                p.target().file
            ));
            out
        }
        Cmd::Import { module, erf, overwrite } => {
            let mut m = Module::open(module)?;
            let data = std::fs::read(erf)?;
            let gi = install(cli)?;
            let mut out = Output::default();
            let rm = module_resman(&gi, &m, &mut out)?;
            let plan = mg_module::transfer::plan_import(&m, &data, &rm)?;
            for r in &plan.missing {
                out.note(format!(
                    "warning: {} needs {:?} {}, found nowhere",
                    r.from, r.kind, r.target
                ));
            }
            let s = mg_module::transfer::import_erf(&mut m, &data, |_| *overwrite)?;
            m.save()?;
            let names = |v: &[ResKey]| v.iter().map(ToString::to_string).collect::<Vec<_>>();
            out.json = json!({
                "added": names(&s.added),
                "replaced": names(&s.replaced),
                "skipped": names(&s.skipped),
                "new_areas": s.new_areas.iter().map(ToString::to_string).collect::<Vec<_>>(),
            });
            out.note(format!(
                "imported {} new, replaced {}, skipped {} existing; new areas: {:?}",
                s.added.len(),
                s.replaced.len(),
                s.skipped.len(),
                s.new_areas.iter().map(|a| a.to_string()).collect::<Vec<_>>()
            ));
            out
        }
        Cmd::Attach { module, files, replace } => {
            use mg_module::attach::{There, copy, hak_list, placements};
            let gi = install(cli)?;
            let user = gi.user_dir.clone().context("no user folder; pass --user-dir")?;
            let placed = placements(&user, files).map_err(anyhow::Error::msg)?;
            let differ: Vec<String> = placed
                .iter()
                .filter(|p| p.there == There::Different)
                .map(|p| p.to.display().to_string())
                .collect();
            if !differ.is_empty() && !*replace {
                bail!(
                    "other files of these names are there (--replace replaces them): {}",
                    differ.join(", ")
                );
            }
            let copied = copy(&placed, *replace).map_err(anyhow::Error::msg)?;
            let mut m = Module::open(module)?;
            let mut info = m.info()?;
            let haks: Vec<String> =
                placed.iter().filter(|p| !p.is_tlk).map(|p| p.name.clone()).collect();
            if !haks.is_empty() {
                info.root.set("Mod_HakList", hak_list(&info.root, &haks));
            }
            let tlk = placed.iter().find(|p| p.is_tlk).map(|t| t.name.clone());
            if let Some(t) = &tlk {
                info.root.set("Mod_CustomTlk", mg_gff::Value::String(t.clone().into_bytes()));
            }
            m.set_info(&info)?;
            m.save()?;
            let mut out =
                Output::new(json!({ "copied": copied, "haks": m.haks()?, "custom_tlk": tlk }));
            out.note(format!(
                "copied {copied} file(s); the module has {} hak(s) attached",
                m.haks()?.len()
            ));
            out
        }
        Cmd::Minimap { module, area, out: png, size } => {
            let m = Module::open(module)?;
            let gi = install(cli)?;
            let mut out = Output::default();
            let rm = module_resman(&gi, &m, &mut out)?;
            let key = ResKey::parse(area, ResType::ARE).context("not an area name")?;
            let data = rm.get(&key).with_context(|| format!("{area}.are"))?;
            let are = mg_gff::Gff::read(&data)?;
            let set = mg_module::minimap::tileset(&rm, &are.root).map_err(anyhow::Error::msg)?;
            let image = mg_module::minimap::minimap(&rm, &are.root, &set, *size)
                .map_err(anyhow::Error::msg)?;
            std::fs::write(png, mg_module::minimap::png(&image).map_err(anyhow::Error::msg)?)?;
            out.json =
                json!({ "file": path_text(png), "width": image.width, "height": image.height });
            out.note(format!("{}×{} pixels", image.width, image.height));
            out
        }
        Cmd::Find { module, types, tag, name, resref, area, placed, blueprints, fields } => {
            let m = Module::open(module)?;
            let types = types
                .iter()
                .map(|t| {
                    ResType::from_extension(t.trim_start_matches('.'))
                        .filter(|t| mg_module::instances::GIT_LISTS.iter().any(|(_, x)| x == t))
                        .with_context(|| format!("{t}: not a blueprint type (utc, utp, …)"))
                })
                .collect::<Result<Vec<_>>>()?;
            let q = mg_module::query::Query {
                types,
                tag: tag.clone(),
                name: name.clone(),
                resref: resref.clone(),
                area: area
                    .as_deref()
                    .map(mg_core::ResRef::from_str)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                placed: if *placed {
                    Some(true)
                } else if *blueprints {
                    Some(false)
                } else {
                    None
                },
                fields: fields
                    .iter()
                    .map(|f| match f.split_once('=') {
                        Some((l, v)) => (l.trim().to_string(), Some(v.to_string())),
                        None => (f.trim().to_string(), None),
                    })
                    .collect(),
            };
            let found = mg_module::query::find(&m, &q);
            let mut out = Output::new(json!({
                "found": found.iter().map(|f| json!({
                    "kind": if f.placed.is_some() { "placed" } else { "blueprint" },
                    "type": f.restype.extension(),
                    "resref": f.resref.map(|r| r.to_lowercase().to_string()),
                    "tag": f.tag,
                    "name": f.name,
                    "area": f.placed.map(|(a, _)| a.to_string()),
                    "index": f.placed.map(|(_, i)| i),
                    "position": f.position,
                })).collect::<Vec<_>>(),
            }));
            for f in &found {
                let what = format!(
                    "{}\t{}\t{}\t{}",
                    f.restype.extension().unwrap_or("?"),
                    f.resref.map(|r| r.to_lowercase().to_string()).unwrap_or_default(),
                    f.tag,
                    f.name
                );
                match (f.placed, f.position) {
                    (Some((a, i)), Some([x, y, z])) => {
                        out.line(format!("{a}[{i}]\t{what}\t{x:.2}, {y:.2}, {z:.2}"))
                    }
                    (Some((a, i)), None) => out.line(format!("{a}[{i}]\t{what}")),
                    _ => out.line(format!("blueprint\t{what}")),
                }
            }
            out.note(format!("{} found", found.len()));
            out
        }
        Cmd::Nwsync {
            module,
            repository,
            with_module,
            uuid,
            name,
            description,
            group_id,
            no_latest,
            limit_file_size,
            force,
            dry_run,
        } => {
            let m = Module::open(module)?;
            let gi = install(cli)?;
            let mut out = Output::default();
            let rm = module_resman(&gi, &m, &mut out)?;
            let info = mg_module::query::info(&m)?;
            let own_uuid = m
                .info()?
                .root
                .string("Mod_UUID")
                .map(|u| String::from_utf8_lossy(u).into_owned())
                .filter(|u| !u.is_empty());
            let uuid = with_module
                .then(|| uuid.clone().or(own_uuid).unwrap_or_else(mg_module::nwsync::new_uuid));
            let (contents, missing) =
                mg_module::nwsync::module_contents(&m, &gi, &rm, *with_module, uuid.as_deref())
                    .map_err(anyhow::Error::msg)?;
            if !missing.is_empty() {
                bail!("haks not found: {}", missing.join(", "));
            }
            let o = mg_module::nwsync::Options {
                with_module: *with_module,
                name: name.clone().unwrap_or(if *with_module { info.name } else { String::new() }),
                description: description.clone().unwrap_or(if *with_module {
                    info.description
                } else {
                    String::new()
                }),
                uuid,
                group_id: *group_id,
                latest: !no_latest,
                limit: (*limit_file_size > 0).then(|| limit_file_size * 1024 * 1024),
                force: *force,
                dry_run: *dry_run,
            };
            let w = mg_module::nwsync::write(repository, &contents, &o, &mut |_, _| {})
                .map_err(anyhow::Error::msg)?;
            out.json = json!({
                "repository": path_text(repository),
                "sha1": w.sha1,
                "files": w.files,
                "bytes": w.bytes,
                "on_disk_bytes": w.on_disk,
                "new_files": w.new_files,
                "dry_run": *dry_run,
            });
            out.line(w.sha1.clone());
            out.note(format!(
                "manifest {}: {} files, {:.1} MB ({:.1} MB compressed, {} data files new){}",
                w.sha1,
                w.files,
                w.bytes as f64 / 1048576.0,
                w.on_disk as f64 / 1048576.0,
                w.new_files,
                if *dry_run { "; nothing written" } else { "" }
            ));
            out.note(format!(
                "serve {} on a web server, and start nwserver with -nwsyncurl <its address>",
                repository.display()
            ));
            out
        }
        Cmd::TilesetPalette { set, output } => {
            let data = std::fs::read(set).with_context(|| path_text(set))?;
            let tileset = mg_set::Tileset::parse(&data, Codepage::default())?;
            let (gff, warnings) = mg_module::tileset::palette(&tileset);
            let stem =
                set.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            let path =
                output.clone().unwrap_or_else(|| set.with_file_name(format!("{stem}palstd.itp")));
            std::fs::write(&path, gff.to_bytes()?)?;
            let count = |id: usize| match gff.root.get("MAIN") {
                Some(mg_gff::Value::List(c)) => match c.get(id).and_then(|c| c.get("LIST")) {
                    Some(mg_gff::Value::List(l)) => l.len(),
                    _ => 0,
                },
                _ => 0,
            };
            let mut out = Output::new(json!({
                "palette": path_text(&path),
                "features": count(0),
                "groups": count(1),
                "terrain": count(2),
            }));
            for w in warnings {
                out.note(format!("warning: {w}"));
            }
            out.note(format!(
                "wrote {}: {} features, {} groups, {} terrain",
                path.display(),
                count(0),
                count(1),
                count(2)
            ));
            out
        }
        Cmd::Info { module } => {
            let m = Module::open(module)?;
            let i = mg_module::query::info(&m)?;
            let mut out = Output::new(json!({
                "name": i.name,
                "tag": i.tag,
                "description": i.description,
                "entry_area": i.entry_area.map(|a| a.to_string()),
                "areas": i.areas.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "haks": i.haks,
                "custom_tlk": i.custom_tlk,
                "min_game_version": i.min_game_version,
                "resources": i.resources.iter().map(|(t, n)| (t.clone(), json!(n))).collect::<serde_json::Map<_, _>>(),
            }));
            out.line(format!("Name\t{}", i.name));
            out.line(format!("Tag\t{}", i.tag));
            out.line(format!(
                "Entry area\t{}",
                i.entry_area.map(|a| a.to_string()).unwrap_or_default()
            ));
            out.line(format!(
                "Areas\t{}",
                i.areas.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
            ));
            out.line(format!("Haks\t{}", i.haks.join(", ")));
            out.line(format!("Talk table\t{}", i.custom_tlk.clone().unwrap_or_default()));
            out.line(format!("Game version\t{}", i.min_game_version));
            let counts: Vec<String> = i.resources.iter().map(|(t, n)| format!("{n} {t}")).collect();
            out.line(format!("Resources\t{}", counts.join(", ")));
            out
        }
        Cmd::Tlk { strrefs } => {
            let gi = install(cli)?;
            let data = std::fs::read(gi.talk_table(false))?;
            let tlk = mg_tlk::Tlk::read(&data)?;
            let texts: Vec<(u32, String)> =
                strrefs.iter().map(|&s| (s, tlk.text(StrRef(s)).unwrap_or_default())).collect();
            let mut out = Output::new(json!({
                "strings": texts.iter().map(|(s, t)| json!({ "strref": s, "text": t })).collect::<Vec<_>>(),
            }));
            for (s, t) in &texts {
                out.line(format!("{s}\t{t}"));
            }
            out
        }
    };
    Ok(out)
}

/// The resman for a module: the game, the module's haks and the module.
fn module_resman(gi: &GameInstall, m: &Module, out: &mut Output) -> Result<ResMan> {
    let mut rm = ResMan::for_game(gi)?;
    let haks = m.haks()?;
    for missing in rm.add_haks(gi, &haks.iter().map(String::as_str).collect::<Vec<_>>())? {
        out.note(format!("warning: hak {missing} not found"));
    }
    rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
    Ok(rm)
}

fn verify(gi: &GameInstall, path: &Path, show_unused: bool) -> Result<Output> {
    let m = Module::open(path)?;
    let mut out = Output::default();
    let rm = module_resman(gi, &m, &mut out)?;
    let missing = mg_module::verify::missing(&m, &rm);
    let unused = if show_unused { mg_module::verify::unused(&m) } else { Vec::new() };
    let count = |data: Vec<u8>| mg_tlk::Tlk::read(&data).map(|t| t.entries.len()).ok();
    let base = std::fs::read(gi.talk_table(false)).ok().and_then(count).unwrap_or(0);
    let custom = m.custom_tlk().ok().flatten().and_then(|name| {
        mg_module::talk::find(&rm, &gi.tlk_dirs(), &name).and_then(|f| count(f.data))
    });
    let findings =
        mg_module::doctor::examine(&m, &rm, mg_module::doctor::TalkTables { base, custom });
    let errors = missing.iter().filter(|x| x.is_error()).count()
        + findings.iter().filter(|f| f.severity == mg_module::doctor::Severity::Error).count();
    let warnings = findings.len() + missing.len() - errors;
    let severity = |s: mg_module::doctor::Severity| match s {
        mg_module::doctor::Severity::Error => "error",
        mg_module::doctor::Severity::Warning => "warning",
    };
    out.json = serde_json::json!({
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
    for x in &missing {
        let what = if x.uncompiled { "not compiled" } else { "missing" };
        let sev = if x.is_error() { "error" } else { "warning" };
        out.line(format!(
            "{sev}\t{:?}\t{}{}\t{} {} {what}",
            x.category,
            x.reference.from,
            x.reference.path,
            x.reference.kind.name(),
            x.reference.target
        ));
    }
    out.line(format!("{} missing references", missing.len()));
    for f in &findings {
        let at = if f.at.is_empty() { String::new() } else { format!(" {}", f.at) };
        out.line(format!(
            "{}\t{} › {}{at}: {}",
            severity(f.severity),
            f.source,
            f.resource,
            f.message
        ));
    }
    out.line(format!("{} content problems", findings.len()));
    if show_unused {
        for k in &unused {
            out.line(format!("unused\t{k}"));
        }
        out.line(format!("{} unused resources", unused.len()));
    }
    if errors > 0 {
        out.failed = true;
        out.note(format!("mg: {} has errors", path.display()));
    }
    Ok(out)
}

fn gff(input: &Path, output: Option<&Path>, json: bool) -> Result<Output> {
    let data = std::fs::read(input).with_context(|| input.display().to_string())?;
    let cp = Codepage::WINDOWS_1252;
    if input.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")) {
        let value: serde_json::Value = serde_json::from_slice(&data)?;
        let gff = mg_gff::from_json(&value, cp)?;
        let out = output.context("writing GFF needs --output")?;
        std::fs::write(out, gff.to_bytes()?)?;
        return Ok(Output::new(serde_json::json!({ "written": path_text(out) })));
    }
    let is_gff = input
        .extension()
        .and_then(|e| e.to_str())
        .and_then(ResType::from_extension)
        .is_none_or(ResType::is_gff);
    if !is_gff {
        bail!("{} is not a GFF file type", input.display());
    }
    let gff = Gff::read(&data)?;
    let value = mg_gff::to_json(&gff, cp)?;
    match output {
        Some(o) => {
            std::fs::write(o, serde_json::to_string_pretty(&value)? + "\n")?;
            Ok(Output::new(serde_json::json!({ "written": path_text(o) })))
        }
        // The GFF as JSON is the output, with --json or without.
        None if json => Ok(Output::new(value)),
        None => {
            let mut out = Output::default();
            out.line(serde_json::to_string_pretty(&value)?);
            Ok(out)
        }
    }
}
