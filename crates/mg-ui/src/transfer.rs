//! File > Export and File > Import: resources to and from `.erf` archives,
//! with what they depend on; File > Import Files and Import Folder: loose
//! files' resources, into the same window as an archive's.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::Ui;
use mg_core::ResType;
use mg_module::transfer::{ImportPlan, Incoming, import_incoming, plan_export, plan_import_of};
use mg_resman::{ResKey, ResMan};

use crate::{Action, FileKind, Moonglow};

/// The Export window's choices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExportDraft {
    pub selected: BTreeSet<ResKey>,
    pub filter: String,
    /// The kind of resource listed; `None`: all of them, kind by kind.
    pub kind: Option<ResType>,
    /// Also export the module resources the chosen ones use.
    pub dependencies: bool,
    /// Move creatures in the module's own factions to the standard ones.
    pub reset_factions: bool,
    pub comments: String,
    /// The module's keys in the window's order, at a workspace revision,
    /// and those the kind and filter then listed: made again only when
    /// one of them changes (a module's 25,000 sorted and each named for
    /// the filter, every frame).
    pub(crate) listed: Option<Listed>,
}

/// [`ExportDraft::listed`]: the revision, kind and filter, the keys in
/// order, and those listed.
pub(crate) type Listed = (u64, Option<ResType>, String, Arc<Vec<ResKey>>, Arc<Vec<ResKey>>);

/// An Export to an archive that is there, waiting on the question
/// Aurora's Export asks of it: append to it, overwrite it, or do nothing
/// (Cancel); and whether to do so always (`always`, the question's box).
#[derive(Debug, Clone)]
pub struct ExportAsk {
    pub draft: ExportDraft,
    pub resources: Vec<ResKey>,
    pub path: PathBuf,
    pub always: bool,
}

/// The answer to [`ExportAsk`]'s question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportAnswer {
    /// The resources are added to the archive (those of the same names
    /// replaced; the rest and its description stay): Aurora's Yes.
    Append,
    /// The archive is replaced: Aurora's No.
    Overwrite,
    /// Cancel: nothing is written.
    Cancel,
}

/// Resources being imported (an archive's, or loose files') and what to
/// overwrite.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDraft {
    /// What they come from: the archive's file name, the folder's (with a
    /// `/`), the file's or how many files.
    pub name: String,
    /// From an archive (which says what it was made from, or not), not
    /// files.
    pub archive: bool,
    pub resources: Vec<ResKey>,
    /// Resources the module already has, and whether to overwrite each.
    pub overwrites: Vec<(ResKey, bool)>,
    /// References nothing satisfies, as text.
    pub missing: Vec<String>,
    /// What the archive was made from, as Moonglow's export stamps it
    /// (`None`: no stamp, as from Aurora).
    pub stamp: Option<String>,
    /// Why the module may have moved on since (another commit now).
    pub base_warning: Option<String>,
    /// What is not imported, and why ("file: why").
    pub refused: Vec<String>,
    /// An area's files without the others.
    pub warnings: Vec<String>,
    /// The scripts that come without their compiled scripts (compiled as
    /// they are imported).
    pub compile: Vec<String>,
    /// Those of the overwrites whose editors hold unsaved changes.
    pub unsaved: Vec<ResKey>,
    /// What is imported.
    pub incoming: Incoming,
}

/// What [`Moonglow::compile_imported`] did.
enum Compiling {
    /// Nothing to compile, or nothing can be.
    Not,
    /// Compiled into the import: those whose compiled scripts changed,
    /// those that failed, and every compiled script made (to note).
    Done { compiled: Vec<String>, failed: Vec<String>, made: Vec<(ResKey, Vec<u8>)> },
    /// Too many for the window's thread: a job, after the import.
    Later(Vec<ResKey>),
}

/// What the import window says of a resource it would replace whose
/// editor holds unsaved changes.
const UNSAVED: &str = "the import replaces it in the module, but its editor keeps your changes, \
                       and saving them writes over what was imported";

/// What To Scratch's hover says: where it copies `what`, or that it asks.
/// The file a resource is exported to in `dir`, named after it with
/// another ending (`NAME.png`, `NAME.csv`): refused where the name can't
/// be a file's, as [`ResKey::path_in`] refuses it (a module from elsewhere
/// may name a resource `../../x`, and the file was written outside the
/// folder chosen).
pub(crate) fn named_in(
    dir: &Path,
    name: &mg_core::ResRef,
    ending: &str,
) -> Result<PathBuf, String> {
    if name.is_empty() || !name.is_file_name() {
        return Err(format!("{name:?} is not a name a file can have"));
    }
    Ok(dir.join(format!("{name}.{ending}")))
}

pub(crate) fn scratch_tip(scratch: Option<&std::path::Path>, what: &str) -> String {
    match scratch {
        Some(d) => format!("Copy {what} into the scratch folder, {}", d.display()),
        None => format!("Copy {what} into a scratch folder, asked for once"),
    }
}

/// Names for the log: all of a few, else the first and how many more
/// ([`crate::text::some_of`], in eight places).
pub(crate) fn listed(names: &[String]) -> String {
    crate::text::some_of(names, 8)
}

/// The rule for a script copied into the scratch folder (a script
/// editor's To Scratch, the module tree's Copy to Scratch Folder): the
/// compiled scripts and debug files of `scripts` in `dir` that this copy
/// did not write (`written`) are removed, under any spelling (a
/// `HELLO.NCS` another program wrote on a file system that tells cases
/// apart). So one that doesn't compile goes as its source alone, without
/// the compiled script an earlier copy left, which the game would still
/// run (El Grillo's request); and one compiled without debug information
/// goes without the debug file of an older compile. What was removed, by
/// file name, and what could not be.
pub(crate) fn remove_stale_compiled(
    dir: &std::path::Path,
    scripts: &[mg_core::ResRef],
    written: &[std::path::PathBuf],
) -> (Vec<String>, Vec<String>) {
    let (mut removed, mut failed) = (Vec::new(), Vec::new());
    let names: Vec<String> = (scripts.iter())
        .flat_map(|r| [ResType::NCS, ResType::NDB].map(|t| ResKey::new(*r, t).to_string()))
        .map(|n| n.to_ascii_lowercase())
        .collect();
    let Ok(entries) = std::fs::read_dir(dir) else { return (removed, failed) };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
        if !names.contains(&name.to_ascii_lowercase())
            || !entry.file_type().is_ok_and(|t| t.is_file())
        {
            continue;
        }
        let path = entry.path();
        if written.iter().any(|w| same_file(w, &path)) {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => removed.push(name),
            Err(e) => failed.push(format!("{}: {e}", path.display())),
        }
    }
    removed.sort();
    (removed, failed)
}

/// Whether two paths of one folder are one file: by the file system's own
/// identity where there is one to ask (Linux and macOS, whose file systems
/// may or may not tell cases apart), else by the name, case aside (Windows
/// file systems don't tell cases apart).
fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (std::fs::metadata(a), std::fs::metadata(b)) {
            (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let name = |p: &std::path::Path| p.file_name().map(|n| n.to_string_lossy().to_lowercase());
        name(a) == name(b)
    }
}

/// What the log says of the compiled scripts and debug files
/// [`remove_stale_compiled`] removed from the scratch folder.
pub(crate) fn removed_said(removed: &[String]) -> String {
    let compiled = removed.iter().any(|n| !n.to_ascii_lowercase().ends_with(".ndb"));
    let why = match compiled {
        true => "the game would still run it",
        false => "made by an older compile",
    };
    format!("removed {}, left by an earlier copy ({why})", listed(removed))
}

impl Moonglow {
    /// The scratch folder To Scratch copies into: chosen once, then kept
    /// in the settings (Options › Folders); None if none is chosen when
    /// asked.
    pub(crate) fn scratch_folder(&mut self) -> Option<PathBuf> {
        self.settings.scratch_dir.clone().or_else(|| {
            let start = self.module_path().and_then(|p| p.parent().map(|d| d.to_path_buf()));
            let picked = self.dialogs.pick_folder("Scratch folder", start.as_deref());
            self.settings.scratch_dir.clone_from(&picked);
            picked
        })
    }

    /// Where Export's and Import's file dialogs open: the folder of the
    /// archive last written or read, else the user folder's `erf` (where
    /// `nwn.ini` puts it), as Aurora's open, else the module's folder.
    pub(crate) fn erf_folder(&self) -> Option<PathBuf> {
        let last = self.settings.erf_folder.clone().filter(|d| d.is_dir());
        last.or_else(|| {
            let erf = self.install.as_ref().and_then(|i| i.user_folder("erf"));
            erf.filter(|d| d.is_dir())
        })
        .or_else(|| self.module_path().and_then(|p| p.parent().map(Path::to_path_buf)))
    }

    /// An archive chosen in Export's or Import's file dialog: its folder
    /// is where they open next.
    fn erf_chosen(&mut self, path: &Path) {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            self.settings.erf_folder = Some(dir.to_path_buf());
        }
    }

    /// File › Import…: an archive's resources.
    pub(crate) fn open_import(&mut self) {
        if self.ws.is_none() {
            return;
        }
        let start = self.erf_folder();
        let Some(path) = self.dialogs.open_file(FileKind::Erf, start.as_deref()) else { return };
        self.erf_chosen(&path);
        self.import_archive(&path);
    }

    /// Import Resources for the archive at `path`.
    fn import_archive(&mut self, path: &Path) {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                self.log.error(format!("Could not read {}: {e}", path.display()));
                return;
            }
        };
        let incoming = match Incoming::archive(&data) {
            Ok(i) => i,
            Err(e) => {
                self.log.error(format!("Could not read {}: {e}", path.display()));
                return;
            }
        };
        let stamp = mg_module::transfer::archive_stamp(&data);
        self.plan_import(named(&[path.to_path_buf()]), Some(stamp), incoming);
    }

    /// File › Import Files… (`folder`: Import Folder…): loose files'
    /// resources, as Export as Files writes them (a script with its
    /// compiled script, an area with its `.git` and `.gic`), into the
    /// same window as an archive's. The dialog starts where Export as
    /// Files last wrote.
    pub(crate) fn open_import_files(&mut self, folder: bool) {
        if self.ws.is_none() {
            return;
        }
        let start = (self.export_dir.clone())
            .or_else(|| self.module_path().and_then(|p| p.parent().map(Path::to_path_buf)));
        let paths = if folder {
            let Some(dir) = self.dialogs.pick_folder("Import Folder", start.as_deref()) else {
                return;
            };
            vec![dir]
        } else {
            self.dialogs.open_files(FileKind::ResourceFiles, start.as_deref())
        };
        if !paths.is_empty() {
            self.import_files(paths);
        }
    }

    /// Import Resources for loose files and folders.
    fn import_files(&mut self, paths: Vec<PathBuf>) {
        let name = named(&paths);
        let mut incoming = Incoming::files(&paths);
        // (Named from the folder given, or the files' own: the window has
        // no room for whole paths.)
        let base = match paths.as_slice() {
            [one] if one.is_dir() => Some(one.clone()),
            [first, rest @ ..] => {
                let parent = first.parent();
                let same = rest.iter().all(|p| p.parent() == parent);
                parent.filter(|_| same).map(Path::to_path_buf)
            }
            [] => None,
        };
        if let Some(base) = base {
            let at = format!("{}{}", base.display(), std::path::MAIN_SEPARATOR);
            for r in &mut incoming.refused {
                r.what = r.what.replace(&at, "");
                r.why = r.why.replace(&at, "");
            }
        }
        if incoming.resources.is_empty() && incoming.refused.is_empty() {
            self.log.warn(format!("Import: {name} has no files"));
            return;
        }
        self.plan_import(name, None, incoming);
    }

    /// Files and folders dropped onto the window (from the desktop's file
    /// manager), as [`dropped_to`] says what becomes of them.
    pub(crate) fn take_dropped(&mut self, paths: Vec<PathBuf>) {
        // (Not under a job: what it hands over goes to the module it began
        // on, as it was.)
        if let Some(job) = &self.job {
            let title = job.title.clone();
            self.log
                .warn(format!("Nothing taken from {}: {title} is still running", named(&paths)));
            return;
        }
        match dropped_to(&paths, self.ws.is_some()) {
            Dropped::Module(p) => self.actions.push(Action::OpenModule(p)),
            Dropped::Hak(p) => _ = crate::hak_view::open_at(self, &p),
            Dropped::Archive(p) => self.import_archive(&p),
            Dropped::Files => self.import_files(paths),
            Dropped::NoModule => self.log.warn(format!(
                "Nothing imported from {}: dropped files are imported into the open module, and \
                 none is open (a module's .mod dropped alone opens it)",
                named(&paths)
            )),
        }
    }

    /// The Import Resources window for what `incoming` brings (`stamp`: an
    /// archive's, `Some(None)` for one without a stamp; `None`: files).
    fn plan_import(
        &mut self,
        name: String,
        stamp: Option<Option<mg_module::stamp::Stamp>>,
        incoming: Incoming,
    ) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let empty = ResMan::new();
        let resman = self.game.as_deref().map_or(&empty, |g| &g.resman);
        let plan = plan_import_of(&ws.module, &incoming, resman);
        let ImportPlan { resources, overwrites, missing, refused, compile, warnings } = plan;
        let base_warning = (stamp.as_ref())
            .and_then(Option::as_ref)
            .and_then(|s| mg_module::stamp::compare(s, &ws.module));
        let unsaved = self.unsaved_editors(&overwrites);
        let compile = compile.iter().map(|k| k.resref.to_string()).collect();
        self.import = Some(ImportDraft {
            name,
            archive: stamp.is_some(),
            resources,
            overwrites: overwrites.into_iter().map(|k| (k, false)).collect(),
            missing: missing
                .iter()
                .map(|r| format!("{} → {:?} {}", r.from, r.kind, r.target))
                .collect(),
            stamp: stamp.flatten().map(|s| s.summary()),
            base_warning,
            refused: refused.iter().map(|r| format!("{}: {}", r.what, r.why)).collect(),
            warnings,
            compile,
            unsaved,
            incoming,
        })
    }

    /// Those of `keys` whose editors hold changes not yet in the module (a
    /// script's typing, a 2DA's): an import that replaces them leaves the
    /// editors as they are, and saving one then writes over what was
    /// imported. (A blueprint's, an area's or a conversation's changes
    /// are the module's already: the import replaces them as one step,
    /// which Undo takes back.)
    fn unsaved_editors(&self, keys: &[ResKey]) -> Vec<ResKey> {
        let table = |k: &ResKey| {
            self.tables.iter().any(|t| {
                t.home == crate::table_view::Home::Module && t.key == *k && t.has_unsaved()
            })
        };
        let script = |k: &ResKey| self.scripts.get(k).is_some_and(|b| b.is_dirty());
        keys.iter().filter(|k| script(k) || table(k)).copied().collect()
    }

    pub(crate) fn run_import(&mut self, draft: ImportDraft) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let mut staged = ws.module.clone();
        let overwrite: BTreeSet<ResKey> =
            draft.overwrites.iter().filter(|(_, o)| *o).map(|(k, _)| *k).collect();
        let summary = match import_incoming(&mut staged, &draft.incoming, |k| overwrite.contains(k))
        {
            Ok(s) => s,
            Err(e) => {
                self.log.error(format!("Import failed: {e}"));
                return;
            }
        };
        let name = draft.name.clone();
        // Scripts that came without their compiled scripts, and those that
        // include them, compiled as one saved here is (Options › Script
        // Editor › Automatically Compile Scripts on Save), into the import:
        // its one step to undo takes them back too. Many are left to a
        // job, after it.
        let compiling = self.compile_imported(&mut staged, &summary.to_compile);
        let Some(ws) = &mut self.ws else { return };
        let edits = ws.edits_to(&staged);
        if let Some(w) = &draft.base_warning {
            self.log.warn(format!("{name}: {w}"));
        }
        // (Imported: the plugins' resource-created handlers see what is new.)
        let cmd = mg_edit::Command::new(format!("Import {name}"), edits);
        let made = crate::plugin_events::Made::new_in(ws, &cmd, "import");
        let cmd = crate::plugin_events::with_plugins(self, cmd, &made);
        if !cmd.edits.is_empty()
            && let Err(e) = self.apply(cmd)
        {
            self.log.error(e.to_string());
            return;
        }
        self.log.info(format!(
            "Imported {name}: {} added, {} replaced, {} kept",
            summary.added.len(),
            summary.replaced.len(),
            summary.skipped.len()
        ));
        if !draft.refused.is_empty() {
            self.log.warn(format!("Not imported: {}", listed(&draft.refused)));
        }
        let unsaved: Vec<String> = (summary.replaced.iter())
            .filter(|k| draft.unsaved.contains(k))
            .map(ToString::to_string)
            .collect();
        if !unsaved.is_empty() {
            self.log.warn(format!(
                "Replaced while open with unsaved changes (its editor keeps them; saving it \
                 writes over what was imported): {}",
                listed(&unsaved)
            ));
        }
        match compiling {
            Compiling::Done { compiled, failed, made } => {
                self.note_compiled(&made);
                if !compiled.is_empty() {
                    self.log.info(format!("Compiled as imported: {}", listed(&compiled)));
                }
                if !failed.is_empty() {
                    self.log.error(format!(
                        "Did not compile (the game runs the compiled script the module had, if \
                         any; Compile in the script's editor says why): {}",
                        listed(&failed)
                    ));
                }
            }
            Compiling::Later(all) => self.compile_job(Some(all)),
            Compiling::Not => {}
        }
        self.refresh_module_layer();
    }

    /// Compiles `scripts` into `staged` (an import, before it is put in
    /// the module), or says why not: no game data, compiling on save off
    /// (the Compiler tab says they weren't), or more than are compiled on
    /// the window's thread (a job compiles them after).
    fn compile_imported(
        &mut self,
        staged: &mut mg_module::Module,
        scripts: &[ResKey],
    ) -> Compiling {
        if scripts.is_empty() {
            return Compiling::Not;
        }
        let names: Vec<String> = scripts.iter().map(|k| k.resref.to_string()).collect();
        if self.game.is_none() {
            self.log.warn(format!(
                "No game data to compile with: imported without compiled scripts, or including \
                 one that was: {}",
                listed(&names)
            ));
            return Compiling::Not;
        }
        if !self.settings.auto_compile {
            scripts.iter().for_each(|k| self.script_tools.saved_not_compiled(*k));
            self.log.warn(format!(
                "Not compiled (Options › Script Editor › Automatically Compile Scripts on Save \
                 is off; Compile All Scripts compiles them): {}",
                listed(&names)
            ));
            return Compiling::Not;
        }
        let external = self.external_compiler();
        let slow = external.is_some() && scripts.len() > 1;
        if scripts.len() > crate::script_view::COMPILED_AT_ONCE || slow {
            self.log.info(format!(
                "{} scripts (those imported without compiled scripts and those that include \
                 them) are compiled in the background",
                scripts.len()
            ));
            return Compiling::Later(scripts.to_vec());
        }
        let Some(game) = self.game.as_deref() else { return Compiling::Not };
        let before: Vec<Option<Vec<u8>>> = (scripts.iter())
            .map(|k| staged.get(&ResKey::new(k.resref, ResType::NCS)).map(<[u8]>::to_vec))
            .collect();
        let debug = self.settings.debug_info;
        let results = mg_module::build::compile_named_with(
            staged,
            &game.resman,
            scripts,
            external.as_ref(),
            debug,
        );
        let (mut compiled, mut failed, mut made) = (Vec::new(), Vec::new(), Vec::new());
        for r in &results {
            let at = scripts.iter().position(|k| *k == r.script);
            let ncs = staged.get(&ResKey::new(r.script.resref, ResType::NCS)).map(<[u8]>::to_vec);
            match &r.result {
                Ok(()) => {
                    if let Some(ncs) = ncs {
                        if at.is_some_and(|i| before[i].as_ref() != Some(&ncs)) {
                            compiled.push(r.script.resref.to_string());
                        }
                        made.push((r.script, ncs));
                    }
                }
                Err(_) => failed.push(r.script.resref.to_string()),
            }
            for w in &r.warnings {
                self.log.warn(format!("{}: {}", r.script, w.message));
            }
        }
        Compiling::Done { compiled, failed, made }
    }

    /// Writes `keys` as loose files, each with what goes with it (a
    /// script's compiled script and debug file, an area's `.git` and
    /// `.gic`), as they are now (saved or not).
    pub(crate) fn run_export_files(
        &mut self,
        keys: Vec<ResKey>,
        dependencies: bool,
        scratch: bool,
    ) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let roots = if dependencies {
            let empty = ResMan::new();
            let resman = self.game.as_deref().map_or(&empty, |g| &g.resman);
            plan_export(&ws.module, &keys, resman).resources
        } else {
            keys
        };
        // A script changed since it was compiled is compiled first.
        // (A script going out is compiled whatever it includes: stale
        // compiles are found by compiling, not by what changed.)
        let scripts: Vec<ResKey> =
            roots.iter().filter(|k| k.restype == mg_core::ResType::NSS).copied().collect();
        let (compiled, broken) = crate::script_view::compile_stale(self, &scripts);
        if self.game.is_none() && !scripts.is_empty() {
            self.log.warn(
                "No game data to compile with: the scripts go out with the compiled scripts \
                 they have, which may be older than their source",
            );
        }
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let mut resources: Vec<ResKey> = Vec::new();
        for k in &roots {
            for f in mg_module::transfer::file_set(&ws.module, k) {
                if !resources.contains(&f) {
                    resources.push(f);
                }
            }
        }
        // A script without its compiled script is of no use to the game.
        let uncompiled: Vec<String> = resources
            .iter()
            .filter(|k| k.restype == mg_core::ResType::NSS)
            .filter(|k| !resources.contains(&ResKey::new(k.resref, mg_core::ResType::NCS)))
            .map(|k| k.resref.to_string())
            .collect();
        // To the scratch folder, one that no longer compiles goes as its
        // source alone, as To Scratch sends it: not with the older
        // compiled script, which the game would run as this script's.
        if scratch {
            resources.retain(|k| {
                !matches!(k.restype, ResType::NCS | ResType::NDB)
                    || !broken.contains(&k.resref.to_string())
            });
        }
        let dir = if scratch {
            self.scratch_folder()
        } else {
            let start = self
                .export_dir
                .clone()
                .or_else(|| self.module_path().and_then(|p| p.parent().map(|d| d.to_path_buf())));
            self.dialogs.pick_folder("Export as files into", start.as_deref())
        };
        let Some(dir) = dir else { return };
        let Some(ws) = &self.ws else { return };
        match mg_module::transfer::export_files(&ws.module, &resources, &dir) {
            Ok(written) => {
                let names: Vec<String> = written
                    .iter()
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .collect();
                self.log.info(format!(
                    "Wrote {} to {}",
                    crate::transfer::listed(&names),
                    dir.display()
                ));
                if !compiled.is_empty() {
                    self.log.info(format!(
                        "Compiled first (changed since last compiled): {}",
                        crate::transfer::listed(&compiled)
                    ));
                }
                if !broken.is_empty() && scratch {
                    self.log.warn(format!(
                        "No longer compiles (its source copied alone; Compile in the script's \
                         editor says why): {}",
                        crate::transfer::listed(&broken)
                    ));
                } else if !broken.is_empty() {
                    self.log.warn(format!(
                        "No longer compiles (the .ncs written is older than the script): {}",
                        crate::transfer::listed(&broken)
                    ));
                }
                if !uncompiled.is_empty() {
                    self.log.warn(format!(
                        "Not compiled (no .ncs written; compile first): {}",
                        crate::transfer::listed(&uncompiled)
                    ));
                }
                if scratch {
                    let copied: Vec<mg_core::ResRef> = (resources.iter())
                        .filter(|k| k.restype == ResType::NSS)
                        .map(|k| k.resref)
                        .collect();
                    let (removed, failed) = remove_stale_compiled(&dir, &copied, &written);
                    if !removed.is_empty() {
                        self.log.info(format!("Scratch folder: {}", removed_said(&removed)));
                    }
                    failed.into_iter().for_each(|f| self.log.error(f));
                } else {
                    self.export_dir = Some(dir);
                }
            }
            Err(e) => self.log.error(format!("Could not write to {}: {e}", dir.display())),
        }
    }

    /// Export as Blueprint… (a placed object's menu, in the area view and
    /// the module tree): the object written into an archive as a blueprint
    /// of its own, with the items it holds as item blueprints, made as Add
    /// to Palette makes them (named so, its place in the area left out) but
    /// not added to the module; with the module's resources it uses (its
    /// scripts, its conversation), and stamped as an export is, for
    /// another module's Import (GitHub issue 7).
    pub(crate) fn run_export_object(
        &mut self,
        area: mg_core::ResRef,
        kind: mg_area::ObjectKind,
        index: usize,
    ) {
        let (Some(game), Some(ws)) = (self.game.as_deref(), self.ws.as_mut()) else {
            self.log.error("Export as Blueprint: the game's data is needed to make a blueprint");
            return;
        };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let git = ResKey::new(area, ResType::GIT);
        let placed = ws.doc(&git).ok().and_then(|g| g.root.list(kind.list())?.get(index).cloned());
        let Some(placed) = placed else { return };
        let module = &ws.module;
        let original = |k: ResKey| mg_module::gff_root(Some(module), game, &k);
        let taken = |k: &ResKey| module.contains(k) || game.resman.get(k).is_ok();
        let naming = module.project_file.naming();
        let restype = kind.restype();
        let made = mg_module::palette_add::add_to_palette(
            game, restype, &placed, &original, &taken, &naming,
        );
        let Some(made) = made else {
            self.log.error("Export as Blueprint: no blueprint name is free");
            return;
        };
        // (Made in a copy of the module, which then has what it uses.)
        let mut staged = module.clone();
        for (key, gff) in &made.blueprints {
            match gff.to_bytes() {
                Ok(bytes) => staged.set(*key, bytes),
                Err(e) => {
                    self.log.error(format!("{key}: {e}"));
                    return;
                }
            }
        }
        let roots: Vec<ResKey> = made.blueprints.iter().map(|(k, _)| *k).collect();
        let plan = plan_export(&staged, &roots, &game.resman);
        for r in &plan.missing {
            self.log
                .warn(format!("Not exported, not found: {} → {:?} {}", r.from, r.kind, r.target));
        }
        let stamp = mg_module::stamp::Stamp::of(module);
        let comments = mg_module::stamp::describe("", &stamp);
        let bytes = mg_module::transfer::export_erf(&staged, &plan.resources, &comments, false);
        let first = roots[0];
        let suggested = self.erf_folder().map(|d| d.join(format!("{}.erf", first.resref)));
        let Some(path) = self.dialogs.save_file(FileKind::Erf, suggested.as_deref()) else {
            return;
        };
        self.erf_chosen(&path);
        match bytes
            .map_err(|e| e.to_string())
            .and_then(|b| std::fs::write(&path, b).map_err(|e| format!("{}: {e}", path.display())))
        {
            Ok(()) => {
                let held = roots.len() - 1;
                let others = plan.resources.len() - roots.len();
                let mut with = Vec::new();
                if held > 0 {
                    with.push(format!("the {held} items it holds"));
                }
                if others > 0 {
                    with.push(format!("{others} resources it uses"));
                }
                let with = if with.is_empty() {
                    String::new()
                } else {
                    format!(", with {}", with.join(" and "))
                };
                self.log.info(format!(
                    "Exported {first}, a blueprint of the placed object{with}, to {}",
                    path.display()
                ));
            }
            Err(e) => self.log.error(format!("Export failed: {e}")),
        }
    }

    pub(crate) fn run_export(&mut self, draft: ExportDraft) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let roots: Vec<ResKey> = draft.selected.iter().copied().collect();
        let resources = if draft.dependencies {
            let empty = ResMan::new();
            let resman = self.game.as_deref().map_or(&empty, |g| &g.resman);
            let plan = plan_export(&ws.module, &roots, resman);
            for r in &plan.missing {
                self.log.warn(format!(
                    "Not exported, not found: {} → {:?} {}",
                    r.from, r.kind, r.target
                ));
            }
            plan.resources
        } else {
            // (An area's .git and .gic make it whole: Import of its .are
            // alone brings no objects. Export as Files takes them too.)
            mg_module::transfer::with_companions(&ws.module, &roots)
        };
        let suggested = (self.erf_folder())
            .zip(roots.first())
            .map(|(dir, k)| dir.join(format!("{}.erf", k.resref)));
        let Some(path) = self.dialogs.save_file(FileKind::Erf, suggested.as_deref()) else {
            return;
        };
        self.erf_chosen(&path);
        // An archive that is there: asked about first, as Aurora asks,
        // unless Options › General says what to do.
        if path.exists() {
            use crate::settings::ExistingArchive;
            match self.settings.export_existing {
                ExistingArchive::Ask => {
                    self.export_ask = Some(ExportAsk { draft, resources, path, always: false });
                }
                chosen => {
                    let append = chosen == ExistingArchive::Append;
                    self.log.info(format!(
                        "{} was there: {} it, as Tools › Options › General says",
                        path.display(),
                        if append { "appended to" } else { "overwrote" }
                    ));
                    self.write_export(&draft, &resources, &path, append);
                }
            }
            return;
        }
        self.write_export(&draft, &resources, &path, false);
    }

    /// [`ExportAsk`] answered.
    pub fn answer_export(&mut self, answer: ExportAnswer) {
        let Some(ask) = self.export_ask.take() else { return };
        if answer != ExportAnswer::Cancel {
            let append = answer == ExportAnswer::Append;
            // Always so: Options › General's choice, from now on.
            if ask.always {
                use crate::settings::ExistingArchive;
                self.settings.export_existing =
                    if append { ExistingArchive::Append } else { ExistingArchive::Overwrite };
            }
            self.write_export(&ask.draft, &ask.resources, &ask.path, append);
        }
    }

    /// Writes an Export's resources to `path`: added to the archive there
    /// (`append`), or a new one in its place.
    fn write_export(
        &mut self,
        draft: &ExportDraft,
        resources: &[ResKey],
        path: &Path,
        append: bool,
    ) {
        let Some(ws) = &self.ws else { return };
        // Appended: what the archive holds stays, but for resources of the
        // same names.
        let existing = if append { std::fs::read(path).ok() } else { None };
        // What it was made from follows the description (Import reads it).
        let stamp = mg_module::stamp::Stamp::of(&ws.module);
        let comments = mg_module::stamp::describe(&draft.comments, &stamp);
        let (comments, reset) = (comments.as_str(), draft.reset_factions);
        let result = match &existing {
            Some(old) => {
                mg_module::transfer::export_into_erf(old, &ws.module, resources, comments, reset)
            }
            None => mg_module::transfer::export_erf(&ws.module, resources, comments, reset),
        }
        .map_err(|e| e.to_string())
        .and_then(|bytes| std::fs::write(path, bytes).map_err(|e| e.to_string()));
        match result {
            Ok(()) => self.log.info(format!(
                "{} {} resources to {}, stamped with {}",
                if existing.is_some() { "Added" } else { "Exported" },
                resources.len(),
                path.display(),
                stamp.summary()
            )),
            Err(e) => self.log.error(format!("Export failed: {e}")),
        }
    }
}

/// The question an Export to an archive that is there asks: Aurora's
/// (its Export, captured 2026-10-29: Yes appends, No overwrites, Cancel
/// does nothing), with buttons named for what they do and each said, and
/// a box to do so always (Options › General), for a builder's ask
/// (Carcerian: Yes and No said too little).
fn ask_ui(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(ask) = &mut app.export_ask else { return };
    let mut answer = None;
    crate::widgets::modal(ctx, "Export").id(egui::Id::new("export-ask")).show(ctx, |ui| {
        ui.set_max_width(460.0);
        let name = ask
            .path
            .file_name()
            .map_or_else(|| ask.path.display().to_string(), |n| n.to_string_lossy().into_owned());
        ui.label(format!("{name} is there already.")).on_hover_text(ask.path.display().to_string());
        ui.add_space(6.0);
        ui.label(
            "Append adds the resources to it: those it has of the same names are \
             replaced, the rest it holds stay.",
        );
        ui.label("Overwrite replaces it with an archive of these resources alone.");
        ui.add_space(6.0);
        ui.checkbox(&mut ask.always, "Always do this")
            .on_hover_text("Tools › Options › General › Exporting to an archive there changes it");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Append").clicked() {
                answer = Some(ExportAnswer::Append);
            }
            if ui.button("Overwrite").clicked() {
                answer = Some(ExportAnswer::Overwrite);
            }
            if crate::widgets::cancel(ui) {
                answer = Some(ExportAnswer::Cancel);
            }
        });
    });
    if let Some(a) = answer {
        app.answer_export(a);
    }
}

/// What an import is called, in its window and the log: the file's name,
/// the folder's (with a `/`), or how many.
fn named(paths: &[PathBuf]) -> String {
    let name = |p: &Path| {
        p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned())
    };
    match paths {
        [one] if one.is_dir() => format!("{}/", name(one)),
        [one] => name(one),
        many if many.iter().any(|p| p.is_dir()) => format!("{} files and folders", many.len()),
        many => format!("{} files", many.len()),
    }
}

/// What files dropped onto the window become ([`dropped_to`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Dropped {
    /// A module's `.mod` (or `.nwm`) alone: opened, as File › Open Module
    /// opens it (asking first about unsaved work).
    Module(PathBuf),
    /// A hak alone: opened in the hak editor, as Tools › Open Hak opens it.
    Hak(PathBuf),
    /// An `.erf` alone: its resources imported, as File › Import… does.
    Archive(PathBuf),
    /// Files and folders: imported, as File › Import Files… does (an
    /// archive among others is refused, and said why).
    Files,
    /// Files to import, and no module to import them into.
    NoModule,
}

/// What files dropped onto the window become, with a module open or not.
pub(crate) fn dropped_to(paths: &[PathBuf], module_open: bool) -> Dropped {
    if let [one] = paths
        && !one.is_dir()
    {
        let ext = one.extension().and_then(|e| e.to_str()).and_then(ResType::from_extension);
        match ext {
            Some(ResType::MOD | ResType::NWM) => return Dropped::Module(one.clone()),
            Some(ResType::HAK) => return Dropped::Hak(one.clone()),
            Some(ResType::ERF) if module_open => return Dropped::Archive(one.clone()),
            _ => {}
        }
    }
    if module_open { Dropped::Files } else { Dropped::NoModule }
}

/// Files dragged over the window: what dropping them does, said over it;
/// and dropped, taken ([`Moonglow::take_dropped`]).
fn drop_ui(app: &mut Moonglow, ctx: &egui::Context) {
    let (hovered, dropped) = ctx.input(|i| {
        let hovered: Vec<Option<PathBuf>> =
            i.raw.hovered_files.iter().map(|f| f.path.clone()).collect();
        let dropped: Vec<PathBuf> = (i.raw.dropped_files.iter())
            .map(|f| f.path().to_path_buf())
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
        (hovered, dropped)
    });
    if !dropped.is_empty() {
        app.take_dropped(dropped);
        return;
    }
    if hovered.is_empty() {
        return;
    }
    // (Where the system doesn't say which files they are yet: as files.)
    let paths: Vec<PathBuf> = hovered.iter().flatten().cloned().collect();
    let module = app.ws.as_ref().map(|ws| {
        let path = ws.module.location.as_ref().map(|l| l.path().to_path_buf());
        path.and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "the module".to_string())
    });
    let said = match dropped_to(&paths, module.is_some()) {
        Dropped::Module(p) => format!("Drop to open the module {}", named(&[p])),
        Dropped::Hak(p) => format!("Drop to open {} in the hak editor", named(&[p])),
        Dropped::Archive(p) => {
            format!(
                "Drop to import {}'s resources into {}",
                named(&[p]),
                module.unwrap_or_default()
            )
        }
        Dropped::Files => format!(
            "Drop to import into {} (the import window lists them first)",
            module.unwrap_or_default()
        ),
        Dropped::NoModule => "Open a module first: dropped files are imported into it".into(),
    };
    let painter =
        ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("drop-files")));
    let room = ctx.content_rect();
    let visuals = ctx.global_style().visuals.clone();
    painter.rect_filled(room, 0.0, egui::Color32::from_black_alpha(150));
    let font = egui::FontId::proportional(20.0);
    let text = painter.layout_no_wrap(said, font, visuals.strong_text_color());
    let plate = egui::Rect::from_center_size(room.center(), text.size() + egui::vec2(48.0, 32.0));
    painter.rect_filled(plate, 8.0, visuals.window_fill);
    painter.rect_stroke(plate, 8.0, visuals.selection.stroke, egui::StrokeKind::Inside);
    painter.galley(plate.center() - text.size() / 2.0, text, visuals.strong_text_color());
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    drop_ui(app, &ctx);
    ask_ui(app, &ctx);
    if let (Some(draft), Some(ws)) = (&mut app.export, &app.ws) {
        let mut close = false;
        // Kind by kind, those of the module's tree first and in its order.
        let rank = |t: ResType| {
            let group = crate::tree::GROUPS.iter().position(|(_, types)| types.contains(&t));
            (group.unwrap_or(usize::MAX), t.extension().unwrap_or_default())
        };
        let kind_name = |t: ResType| {
            let ext = t.extension().unwrap_or_default();
            match crate::tree::GROUPS.iter().find(|(_, types)| *types == [t]) {
                Some((name, _)) => format!("{name} (.{ext})"),
                None => format!(".{ext}"),
            }
        };
        let revision = ws.revision();
        let kept = (draft.listed.as_ref()).filter(|(r, ..)| *r == revision).map(|l| l.3.clone());
        let keys = kept.unwrap_or_else(|| {
            let mut keys: Vec<ResKey> = ws.module.keys().copied().collect();
            keys.sort_by_cached_key(|k| (rank(k.restype), k.resref));
            Arc::new(keys)
        });
        let mut kinds: Vec<(ResType, usize)> = Vec::new();
        for k in keys.iter() {
            match kinds.last_mut() {
                Some((t, n)) if *t == k.restype => *n += 1,
                _ => kinds.push((k.restype, 1)),
            }
        }
        if draft.kind.is_some_and(|t| kinds.iter().all(|(k, _)| *k != t)) {
            draft.kind = None;
        }
        crate::widgets::modal(&ctx, "Export Resources").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Kind");
                let shown = match draft.kind {
                    Some(t) => kind_name(t),
                    None => "All".to_string(),
                };
                let options: Vec<_> =
                    std::iter::once(None).chain(kinds.iter().map(|(t, _)| Some(*t))).collect();
                crate::widgets::dropdown("export-kind").selected_text(shown).width(200.0).show(
                    ui,
                    &mut draft.kind,
                    &options,
                    |t| match t.and_then(|t| kinds.iter().find(|k| k.0 == t)) {
                        Some((t, n)) => format!("{}: {n}", kind_name(*t)),
                        None => format!("All: {}", keys.len()),
                    },
                );
            });
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Filter");
                let width = ui.spacing().text_edit_width;
                let field = crate::widgets::SearchField::new(
                    ui.id().with("transfer-filter-field"),
                    &mut draft.filter,
                )
                .width(width)
                .find_over()
                .show(ui)
                .response;
                crate::widgets::autofocus(ui, &field);
            });
            let same = |(r, kind, filter, ..): &&Listed| {
                *r == revision && *kind == draft.kind && *filter == draft.filter
            };
            let listed = match draft.listed.as_ref().filter(same) {
                Some(l) => l.4.clone(),
                None => {
                    let filter = mg_core::text::Needle::new(&draft.filter);
                    let listed: Vec<ResKey> = keys
                        .iter()
                        .filter(|k| draft.kind.is_none_or(|t| k.restype == t))
                        .filter(|k| filter.finds(&k.to_string()))
                        .copied()
                        .collect();
                    let listed = Arc::new(listed);
                    let (kind, filter) = (draft.kind, draft.filter.clone());
                    draft.listed = Some((revision, kind, filter, keys.clone(), listed.clone()));
                    listed
                }
            };
            // The rows: with every kind listed, each under its heading.
            let mut rows: Vec<Result<ResKey, ResType>> = Vec::with_capacity(listed.len());
            for k in listed.iter() {
                let first = !matches!(rows.last(), Some(Ok(l)) if l.restype == k.restype);
                if draft.kind.is_none() && first {
                    rows.push(Err(k.restype));
                }
                rows.push(Ok(*k));
            }
            egui::Frame::group(ui.style()).show(ui, |ui| {
                let height = ui.spacing().interact_size.y;
                egui::ScrollArea::vertical().max_height(260.0).show_rows(
                    ui,
                    height,
                    rows.len(),
                    |ui, range| {
                        ui.set_width(320.0);
                        for row in &rows[range] {
                            let k = match row {
                                Ok(k) => k,
                                Err(t) => {
                                    let size = egui::vec2(ui.available_width(), height);
                                    let across = egui::Layout::left_to_right(egui::Align::Center);
                                    ui.allocate_ui_with_layout(size, across, |ui| {
                                        ui.strong(kind_name(*t));
                                    });
                                    continue;
                                }
                            };
                            let mut on = draft.selected.contains(k);
                            if ui.checkbox(&mut on, k.to_string()).changed() {
                                if on {
                                    draft.selected.insert(*k);
                                } else {
                                    draft.selected.remove(k);
                                }
                            }
                        }
                    },
                );
            });
            ui.horizontal(|ui| {
                let all = listed.iter().all(|k| draft.selected.contains(k));
                if ui
                    .add_enabled(
                        !all,
                        egui::Button::new(format!("Select the {} Listed", listed.len())),
                    )
                    .clicked()
                {
                    draft.selected.extend(listed.iter().copied());
                }
                let any = listed.iter().any(|k| draft.selected.contains(k));
                if ui.add_enabled(any, egui::Button::new("Clear Them")).clicked() {
                    for k in listed.iter() {
                        draft.selected.remove(k);
                    }
                }
            });
            ui.checkbox(&mut draft.dependencies, "Include the resources they use");
            ui.checkbox(&mut draft.reset_factions, "Put creatures in the standard factions");
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Description");
                ui.text_edit_singleline(&mut draft.comments).on_hover_text(
                    "The module's git commit, Moonglow's version and name go with it (Import shows \
                     them)",
                );
            });
            ui.horizontal(|ui| {
                let label = format!("Export {}…", draft.selected.len());
                if ui.add_enabled(!draft.selected.is_empty(), egui::Button::new(label)).clicked() {
                    app.actions.push(Action::Export(draft.clone()));
                    close = true;
                }
                let files = format!("Export {} as Files…", draft.selected.len());
                if ui
                    .add_enabled(!draft.selected.is_empty(), egui::Button::new(files))
                    .on_hover_text(
                        "Write them into a folder as loose files, as they are in the module",
                    )
                    .clicked()
                {
                    app.actions.push(Action::ExportFiles {
                        keys: draft.selected.iter().copied().collect(),
                        dependencies: draft.dependencies,
                        scratch: false,
                    });
                    close = true;
                }
                if crate::widgets::cancel(ui) {
                    close = true;
                }
            });
        });
        if close {
            app.export = None;
        }
    }

    if let Some(draft) = &mut app.import {
        let mut close = false;
        let name = draft.name.clone();
        let auto_compile = app.settings.auto_compile;
        // Resized as a builder likes (GitHub issue 40: an archive of
        // hundreds of resources), the list of those the module has taking
        // the room.
        let window = crate::widgets::modal(&ctx, "Import Resources");
        let window = crate::widgets::resizable(&ctx, window, egui::vec2(440.0, 480.0))
            .min_size([320.0, 200.0]);
        window.show(&ctx, |ui| {
            let new = draft.resources.len() - draft.overwrites.len();
            ui.label(format!("{name}: {} resources, {new} new", draft.resources.len()));
            match &draft.stamp {
                Some(s) => _ = ui.label(format!("Made from {s}")),
                None if draft.archive => _ = ui.weak("It doesn't say what it was made from."),
                None => {}
            };
            if let Some(w) = &draft.base_warning {
                ui.colored_label(ui.visuals().warn_fg_color, format!("Careful: {w}."));
            }
            // (What is under the list: its height the frame before.)
            let under_id = ui.id().with("import-under-list");
            let under_was: Option<f32> = ui.data(|d| d.get_temp(under_id));
            let mut under_from = None;
            if !draft.overwrites.is_empty() {
                ui.add_space(6.0);
                ui.label("The module already has these; overwrite:");
                let row = ui.spacing().interact_size.y;
                let under = under_was.unwrap_or(4.0 * row);
                let height = (ui.available_height() - under).max(4.0 * row);
                egui::ScrollArea::vertical()
                    .id_salt("overwrites")
                    .auto_shrink([false, true])
                    .max_height(height)
                    .show_rows(ui, row, draft.overwrites.len(), |ui, range| {
                        for (k, on) in &mut draft.overwrites[range] {
                            let open = draft.unsaved.contains(k);
                            let text = match open {
                                true => format!("{k} (open, with unsaved changes)"),
                                false => k.to_string(),
                            };
                            let r = ui.checkbox(on, text);
                            if open {
                                r.on_hover_text(format!("Open with unsaved changes: {UNSAVED}"));
                            }
                        }
                    });
                under_from = Some(ui.min_rect().bottom());
                // Every one at once, as Aurora's Overwrite page has them.
                ui.horizontal(|ui| {
                    let all = draft.overwrites.iter().all(|(_, on)| *on);
                    let tip = format!("Overwrite all {}", draft.overwrites.len());
                    if ui
                        .add_enabled(!all, egui::Button::new("Select All"))
                        .on_hover_text(tip)
                        .clicked()
                    {
                        draft.overwrites.iter_mut().for_each(|(_, on)| *on = true);
                    }
                    let any = draft.overwrites.iter().any(|(_, on)| *on);
                    let tip = "Overwrite none of them: the module keeps its own";
                    if ui
                        .add_enabled(any, egui::Button::new("Select None"))
                        .on_hover_text(tip)
                        .clicked()
                    {
                        draft.overwrites.iter_mut().for_each(|(_, on)| *on = false);
                    }
                    let n = draft.overwrites.iter().filter(|(_, on)| *on).count();
                    ui.weak(format!("{n} of {} to overwrite", draft.overwrites.len()));
                });
            }
            if draft.unsaved.iter().any(|k| draft.overwrites.contains(&(*k, true))) {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!("Careful: one ticked is open with unsaved changes; {UNSAVED}."),
                );
            }
            if !draft.compile.is_empty() {
                ui.add_space(6.0);
                let names = listed(&draft.compile);
                if auto_compile {
                    ui.label(format!(
                        "Compiled as they are imported, with the scripts that include them (no \
                         .ncs comes with them): {names}"
                    ));
                } else {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!(
                            "Not compiled (no .ncs comes with them, and Options › Script Editor \
                             › Automatically Compile Scripts on Save is off): {names}"
                        ),
                    );
                }
            }
            for w in &draft.warnings {
                ui.colored_label(ui.visuals().warn_fg_color, w);
            }
            if !draft.refused.is_empty() {
                ui.add_space(6.0);
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!("Not imported ({}):", draft.refused.len()),
                );
                egui::ScrollArea::vertical().id_salt("refused").max_height(120.0).show(ui, |ui| {
                    for r in &draft.refused {
                        ui.label(r);
                    }
                });
            }
            if !draft.missing.is_empty() {
                ui.add_space(6.0);
                ui.colored_label(ui.visuals().warn_fg_color, "Used but found nowhere:");
                egui::ScrollArea::vertical().id_salt("missing").max_height(120.0).show(ui, |ui| {
                    for m in &draft.missing {
                        ui.monospace(m);
                    }
                });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let any = !draft.resources.is_empty();
                if ui.add_enabled(any, egui::Button::new("Import")).clicked()
                    || (any && crate::widgets::enter(ui))
                {
                    app.actions.push(Action::Import(draft.clone()));
                    close = true;
                }
                if crate::widgets::cancel(ui) {
                    close = true;
                }
            });
            if let Some(from) = under_from {
                let under = ui.min_rect().bottom() - from + ui.spacing().item_spacing.y;
                if under_was.is_none_or(|was| (was - under).abs() > 0.5) {
                    ui.data_mut(|d| d.insert_temp(under_id, under));
                    ui.ctx().request_repaint();
                }
            }
        });
        if close {
            app.import = None;
        }
    }
}
