//! `mg watch`: a module kept as files (a nasher project, a module
//! folder) followed as it is edited. A changed script is compiled, with
//! the scripts that include it, into a development folder, where a
//! running server takes it up at its next use, as it does a changed
//! blueprint or conversation; anything else (a 2DA, an area, module.ifo)
//! needs the module built and the server started again.
//!
//! The project is read once, at the start. After that the system says
//! which of its files changed ([`mg_module::watch`]; where it can't, the
//! files' times and sizes are walked for), and only they are read and
//! taken into the module ([`mg_module::outside`]), as the window takes
//! files changed outside Moonglow. It is read whole again only where
//! reading it settles something: `nasher.cfg` changed, an area added or
//! removed, `module.ifo` changed (its area list is filled as nasher
//! fills it).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use mg_core::ResType;
use mg_module::Module;
use mg_module::watch::Watch;
use mg_resman::{ResKey, ResMan};
use serde_json::{Value, json};

/// The file in a development folder that lists what `mg watch` put there
/// (to take away again: what it did not write is not its to remove).
const WRITTEN: &str = ".mg-watch";

/// What a change to a resource of this type asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Compiled, with its includers; the compiled scripts go out.
    Script,
    /// Goes out as it is: the server reads it anew at its next use.
    Hot,
    /// The server has it by heart: build and start again.
    Rebuild,
    /// Made from another (a compiled script).
    Made,
}

fn kind(t: ResType) -> Kind {
    const HOT: [ResType; 10] = [
        ResType::UTC,
        ResType::UTI,
        ResType::UTP,
        ResType::UTD,
        ResType::UTM,
        ResType::UTE,
        ResType::UTS,
        ResType::UTT,
        ResType::UTW,
        ResType::DLG,
    ];
    match t {
        ResType::NSS => Kind::Script,
        ResType::NCS | ResType::NDB => Kind::Made,
        t if HOT.contains(&t) => Kind::Hot,
        _ => Kind::Rebuild,
    }
}

/// What one look at the project found and did.
#[derive(Debug, Default)]
pub(crate) struct Step {
    /// Scripts compiled (those that made a compiled script).
    pub compiled: Vec<String>,
    pub errors: Vec<String>,
    /// What the compiles warned of, each once (an include's is said by
    /// its includers).
    pub warnings: Vec<String>,
    /// Files put into the development folder.
    pub written: Vec<String>,
    /// What changed that the server has by heart: the module was built
    /// (if there is somewhere to build it to) and the server is to be
    /// started again.
    pub rebuild: Vec<String>,
}

impl Step {
    /// Nothing changed that the module has (a file touched, written as
    /// it was).
    pub(crate) fn is_empty(&self) -> bool {
        let Step { compiled, errors, warnings, written, rebuild } = self;
        [compiled, errors, warnings, written, rebuild].iter().all(|v| v.is_empty())
    }

    pub(crate) fn json(&self) -> Value {
        json!({
            "event": if self.rebuild.is_empty() { "synced" } else { "rebuilt" },
            "compiled": self.compiled, "errors": self.errors, "warnings": self.warnings,
            "written": self.written, "changed": self.rebuild,
        })
    }

    pub(crate) fn text(&self, dev_dir: &Path) -> Vec<String> {
        let mut out: Vec<String> = self.errors.iter().chain(&self.warnings).cloned().collect();
        if !self.rebuild.is_empty() {
            let more = match self.rebuild.len() {
                1 => String::new(),
                n => format!(" and {} more", n - 1),
            };
            out.push(format!("mg: {}{more} changed: the module is built again", self.rebuild[0]));
        }
        if !self.compiled.is_empty() || !self.errors.is_empty() {
            out.push(format!(
                "mg: compiled {} scripts, {} failed",
                self.compiled.len(),
                self.errors.len()
            ));
        }
        if !self.written.is_empty() {
            out.push(format!("mg: {} -> {}", self.written.join(" "), dev_dir.display()));
        }
        out
    }
}

pub(crate) struct Watcher {
    project: PathBuf,
    /// The nasher project's target followed (`None`: its default).
    target: Option<String>,
    dev_dir: PathBuf,
    /// Where the module is built to, if anywhere.
    build_to: Option<PathBuf>,
    /// The module as read at the start, with what changed since taken
    /// into it, file by file ([`mg_module::outside`]).
    module: Module,
    /// The project's own talk table's files, as last read ([`tables`]).
    tables: Vec<(PathBuf, u64)>,
    /// `nasher.cfg`'s time and size as last read: the project is read
    /// whole again when they change (its sources may be others).
    config: Option<(SystemTime, u64)>,
    /// Tells which of the project's files changed (else walks them).
    watch: Watch,
    looked: Instant,
    /// Whether the last pass hit a transient read error and should re-poll even
    /// if the OS watcher delivers no further event.
    retry: bool,
    /// The read errors the last pass told of: a standing error is told once
    /// and re-polled only while the set changes.
    told: Vec<String>,
    /// Debug files (`.ndb`) are made with the compiled scripts.
    debug: bool,
}

fn stamp(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut h);
    h.finish()
}

/// The files of the talk table the module names that its nasher project
/// keeps (as JSON or CSV, each language's and the feminine table's), with
/// what they hold: a change to one is the server's to read anew when it
/// starts, as the table is made for it then.
fn tables(m: &Module) -> Vec<(PathBuf, u64)> {
    let (Some(p), Ok(Some(name))) = (m.project.as_deref(), m.custom_tlk()) else {
        return Vec::new();
    };
    let mut out: Vec<(PathBuf, u64)> = [name.clone(), mg_module::talk::feminine(&name)]
        .iter()
        .flat_map(|n| p.talk_tables(n).unwrap_or_default())
        .map(|t| (t.path, stamp(&t.file)))
        .collect();
    out.sort();
    out
}

/// Whether a file a watch heard of may be one of a talk table's
/// (`name.tlk.json`, `name.tlk.csv`).
fn of_a_table(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    name.contains(".tlk.")
}

/// The folder (or file) a module is kept in, and its `nasher.cfg` if it
/// is a nasher project.
fn place(m: &Module, project: &Path) -> (PathBuf, Option<PathBuf>) {
    match (&m.project, &m.folder) {
        (Some(p), _) => (p.root().to_path_buf(), Some(p.root().join("nasher.cfg"))),
        (None, Some(f)) => (f.dir().to_path_buf(), None),
        (None, None) => (project.to_path_buf(), None),
    }
}

fn file_stamp(path: Option<&Path>) -> Option<(SystemTime, u64)> {
    let meta = std::fs::metadata(path?).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// The module kept at `project` (a nasher project, a module folder), a
/// nasher project's `target` if one is named (else its default).
pub(crate) fn open(project: &Path, target: Option<&str>) -> Result<Module> {
    let Some(target) = target else {
        return crate::open_module(project).with_context(|| project.display().to_string());
    };
    let root = match project.file_name() {
        Some(n) if n == "nasher.cfg" => {
            project.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."))
        }
        _ => project,
    };
    if !mg_module::nasher::Package::is_package(root) {
        anyhow::bail!("{}: --target is a nasher project's, and this is none", project.display());
    }
    Module::open_project(root, Some(target)).with_context(|| project.display().to_string())
}

impl Watcher {
    /// Takes the module read from the project (with [`open`]), compiles all
    /// of its scripts and builds the module (if `build_to`). What an
    /// earlier `mg watch` left in the development folder is taken away:
    /// the module just built has it.
    pub(crate) fn start(
        mut module: Module,
        project: &Path,
        target: Option<&str>,
        dev_dir: &Path,
        build_to: Option<PathBuf>,
        debug: bool,
        rm: &ResMan,
    ) -> Result<(Watcher, Step)> {
        let (root, config) = place(&module, project);
        // (Walked, where the system can't tell, as often as before.)
        let watch = Watch::start(vec![root], true, Duration::from_millis(400), false, None);
        let all: Vec<ResKey> = module.keys_of(ResType::NSS).copied().collect();
        let mut step = Step::default();
        compile(&mut module, rm, &all, debug, &mut step);
        let mut w = Watcher {
            debug,
            project: project.to_path_buf(),
            target: target.map(str::to_string),
            dev_dir: dev_dir.to_path_buf(),
            build_to,
            tables: tables(&module),
            config: file_stamp(config.as_deref()),
            module,
            watch,
            looked: Instant::now(),
            retry: false,
            told: Vec::new(),
        };
        // (Its first word is "look at all", and all was just read.)
        let _ = w.watch.take();
        w.clear_written();
        w.build()?;
        Ok((w, step))
    }

    fn written(&self) -> BTreeSet<String> {
        std::fs::read_to_string(self.dev_dir.join(WRITTEN))
            .map(|t| t.lines().map(str::to_string).filter(|l| !l.is_empty()).collect())
            .unwrap_or_default()
    }

    fn clear_written(&self) {
        for name in self.written() {
            // (A name, never a path out of the folder.)
            if !name.contains(['/', '\\']) {
                let _ = std::fs::remove_file(self.dev_dir.join(name));
            }
        }
        let _ = std::fs::remove_file(self.dev_dir.join(WRITTEN));
    }

    fn build(&mut self) -> Result<()> {
        let Some(to) = self.build_to.clone() else { return Ok(()) };
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // (A copy is saved: the project stays where the module is read.)
        let mut copy = self.module.clone();
        crate::serve::save_copy(&mut copy, &to)
    }

    /// The module as last read.
    pub(crate) fn module(&self) -> &Module {
        &self.module
    }

    /// Takes up what the watch heard changed, if it heard anything (and
    /// a watch without its thread: all, every few tenths of a second).
    pub(crate) fn look(&mut self, rm: &ResMan) -> Result<Option<Step>> {
        let files = match self.watch.take() {
            Some(heard) => heard.files(),
            None if (self.watch.dead() || self.retry)
                && self.looked.elapsed() >= Duration::from_millis(400) =>
            {
                None
            }
            None => return Ok(None),
        };
        self.looked = Instant::now();
        let step = self.step_of(files, rm)?;
        Ok((!step.is_empty()).then_some(step))
    }

    /// The read errors of this pass that the last one did not tell of. A
    /// pass with errors is tried again (`retry`) only while the set of
    /// errors changes: one that stays would else be a full step every
    /// 400 ms, told again each time.
    fn news_of(&mut self, problems: Vec<String>) -> Vec<String> {
        let same = problems == self.told;
        self.retry = !problems.is_empty() && !same;
        self.told = problems.clone();
        if same { Vec::new() } else { problems }
    }

    /// Takes up what changed in all of the project's files.
    #[cfg(test)]
    pub(crate) fn step(&mut self, rm: &ResMan) -> Result<Step> {
        self.step_of(None, rm)
    }

    /// Takes up what changed in these of the project's files (`None`: in
    /// all of them): only they are read, and only what changed is
    /// compiled or put out.
    fn step_of(&mut self, files: Option<Vec<PathBuf>>, rm: &ResMan) -> Result<Step> {
        self.retry = false;
        let mut step = Step::default();
        let (_, config) = place(&self.module, &self.project);
        let config_now = file_stamp(config.as_deref());
        let tables_heard = files.as_ref().is_none_or(|f| f.iter().any(|p| of_a_table(p)));
        // `nasher.cfg` changed: its sources may be others; read whole again.
        let mut reread = config_now != self.config;
        self.config = config_now;
        let scan = match &files {
            _ if reread => None,
            Some(files) => self.module.outside_scan_of(files),
            None => self.module.outside_scan(),
        };
        let (changes, mut problems) = match scan {
            Some(scan) => self.module.outside_found(scan.run()),
            None => (Vec::new(), Vec::new()),
        };
        // (A file half written, say: the next look reads it whole. One that
        // stays as it is is told once, and not polled again.)
        let (mut changed, mut removed): (Vec<ResKey>, Vec<ResKey>) = (Vec::new(), Vec::new());
        for c in &changes {
            // (Compiled scripts are made here, not taken.)
            if kind(c.key.restype) == Kind::Made {
                self.module.took(c);
                continue;
            }
            let had = self.module.shared(&c.key).cloned();
            match &c.resource {
                Some(now) if had.as_deref() == Some(&now[..]) => {}
                Some(now) => {
                    // An area new to the module, or its module.ifo: the
                    // area list is settled as the project is read.
                    let ifo = c.key == Module::info_key();
                    reread |= ifo || (had.is_none() && c.key.restype == ResType::ARE);
                    self.module.set(c.key, now.clone());
                    changed.push(c.key);
                }
                None if had.is_some() => {
                    self.module.remove(&c.key);
                    if c.key.restype == ResType::NSS {
                        self.module.remove(&ResKey::new(c.key.resref, ResType::NCS));
                        self.module.remove(&ResKey::new(c.key.resref, ResType::NDB));
                    }
                    reread |= c.key.restype == ResType::ARE;
                    removed.push(c.key);
                }
                None => {}
            }
            self.module.took(c);
        }
        if reread {
            match self.reread() {
                Ok(also) => {
                    for k in also.0 {
                        if !changed.contains(&k) {
                            changed.push(k);
                        }
                    }
                    for k in also.1 {
                        if !removed.contains(&k) {
                            removed.push(k);
                        }
                    }
                }
                // (A file half written: the next look reads it whole.)
                Err(e) => {
                    problems.push(format!("{}: {e:#}", self.project.display()));
                    step.errors.extend(self.news_of(problems));
                    self.config = None;
                    return Ok(step);
                }
            }
        }
        step.errors.extend(self.news_of(problems));
        // The talk table's files (not the module's resources).
        let mut tables_changed: Vec<String> = Vec::new();
        if tables_heard || reread {
            let tables_now = tables(&self.module);
            tables_changed = (tables_now.iter())
                .filter(|t| !self.tables.contains(t))
                .chain(self.tables.iter().filter(|t| !tables_now.iter().any(|n| n.0 == t.0)))
                .map(|(p, _)| {
                    // (Named with `/` on every system, as the module's are.)
                    let root = place(&self.module, &self.project).0;
                    let p = p.strip_prefix(&root).unwrap_or(p);
                    let parts: Vec<_> =
                        p.components().map(|c| c.as_os_str().to_string_lossy()).collect();
                    parts.join("/")
                })
                .collect();
            tables_changed.sort();
            tables_changed.dedup();
            self.tables = tables_now;
        }
        if changed.is_empty() && removed.is_empty() && tables_changed.is_empty() {
            return Ok(step);
        }
        let scripts: Vec<ResKey> =
            changed.iter().filter(|k| kind(k.restype) == Kind::Script).copied().collect();
        let mut to_compile = scripts.clone();
        for k in mg_module::refs::includers(&self.module, &scripts) {
            if !to_compile.contains(&k) {
                to_compile.push(k);
            }
        }
        compile(&mut self.module, rm, &to_compile, self.debug, &mut step);
        let mut heavy: Vec<String> = (changed.iter())
            .filter(|k| kind(k.restype) == Kind::Rebuild)
            .chain(&removed)
            .map(ToString::to_string)
            .chain(tables_changed)
            .collect();
        heavy.sort();
        if !heavy.is_empty() {
            step.rebuild = heavy;
            self.clear_written();
            self.build()?;
            return Ok(step);
        }
        // Out they go: the compiled scripts, and what the server reads
        // anew.
        let mut out: Vec<ResKey> = (to_compile.iter())
            .flat_map(|k| [ResType::NCS, ResType::NDB].map(|t| ResKey::new(k.resref, t)))
            .filter(|k| self.module.get(k).is_some_and(|d| !d.is_empty()))
            .collect();
        out.extend(changed.iter().filter(|k| kind(k.restype) == Kind::Hot));
        out.sort_by_key(ToString::to_string);
        let files = mg_module::transfer::export_files(&self.module, &out, &self.dev_dir)
            .with_context(|| self.dev_dir.display().to_string())?;
        let mut written = self.written();
        for f in &files {
            let name = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
            written.insert(name.clone());
            step.written.push(name);
        }
        if !files.is_empty() {
            let list: String = written.iter().map(|n| format!("{n}\n")).collect();
            crate::write_bytes_whole(&self.dev_dir.join(WRITTEN), list.as_bytes())?;
        }
        // (For the next build, too.)
        self.build()?;
        Ok(step)
    }

    /// Reads the project whole again (its `nasher.cfg` changed, an area
    /// was added or removed, its module.ifo changed: what reading it
    /// settles), keeping the compiled scripts of those whose source is as
    /// it was. Returns what differs from the module as it was: resources
    /// changed or new, and gone.
    fn reread(&mut self) -> Result<(Vec<ResKey>, Vec<ResKey>)> {
        let mut module = open(&self.project, self.target.as_deref())?;
        let made = |k: &ResKey| kind(k.restype) == Kind::Made;
        let changed: Vec<ResKey> = (module.keys())
            .filter(|k| !made(k))
            .filter(|k| self.module.get(k) != module.get(k))
            .copied()
            .collect();
        let gone: Vec<ResKey> =
            (self.module.keys()).filter(|k| !made(k) && !module.contains(k)).copied().collect();
        // The compiled scripts of those that did not change, as they were.
        for k in self.module.keys().filter(|k| made(k)) {
            let source = ResKey::new(k.resref, ResType::NSS);
            if module.contains(&source)
                && !changed.contains(&source)
                && let Some(data) = self.module.shared(k)
            {
                module.set(*k, data.clone());
            }
        }
        self.module = module;
        let (_, config) = place(&self.module, &self.project);
        self.config = file_stamp(config.as_deref());
        Ok((changed, gone))
    }
}

fn compile(module: &mut Module, rm: &ResMan, names: &[ResKey], debug: bool, step: &mut Step) {
    let results = mg_module::build::compile_named_with(module, rm, names, None, debug);
    for (w, n) in mg_module::build::distinct_warnings(&results) {
        step.warnings.push(match n {
            1 => w.message.clone(),
            n => format!("{} (in {n} scripts)", w.message),
        });
    }
    for r in results {
        match r.result {
            Ok(()) if module.contains(&ResKey::new(r.script.resref, ResType::NCS)) => {
                step.compiled.push(r.script.resref.to_string());
            }
            Ok(()) => {}
            Err(e) => {
                // (Not an older compiled script in its place.)
                module.remove(&ResKey::new(r.script.resref, ResType::NCS));
                module.remove(&ResKey::new(r.script.resref, ResType::NDB));
                step.errors.push(e.message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use mg_module::ModuleLocation;

    /// The project's own talk table (its JSON, not one of the module's
    /// resources) changes: the watch sees it, as a change the server has
    /// by heart, and builds the module again (the server, started again,
    /// has the table made anew).
    #[test]
    fn the_project_s_talk_table_changing_is_seen() {
        let dir = mg_testkit::scratch_dir("mg-watch-tlk");
        let mut m = Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mytlk".to_vec()));
        m.set_info(&info).unwrap();
        let project = dir.join("project");
        m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
            .unwrap();
        let mut cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
        cfg.push_str(
            "\n[target]\nname = \"tlk\"\nfile = \"mytlk.tlk\"\n  [target.sources]\n  include = \
             \"src/tlk/*.json\"\n",
        );
        std::fs::write(project.join("nasher.cfg"), cfg).unwrap();
        std::fs::create_dir_all(project.join("src/tlk")).unwrap();
        let json = project.join("src/tlk/mytlk.tlk.json");
        std::fs::write(&json, r#"{"language":0,"entries":[{"id":0,"text":"One"}]}"#).unwrap();
        let rm = ResMan::new();
        let built = dir.join("built.mod");
        let m = open(&project, None).unwrap();
        let (mut w, _) =
            Watcher::start(m, &project, None, &dir.join("dev"), Some(built.clone()), false, &rm)
                .unwrap();
        assert_eq!(w.tables.len(), 1);
        // Nothing changed: nothing to do.
        assert!(w.step(&rm).unwrap().rebuild.is_empty());
        std::fs::write(&json, r#"{"language":0,"entries":[{"id":0,"text":"Two"}]}"#).unwrap();
        let step = w.step(&rm).unwrap();
        assert_eq!(step.rebuild, ["src/tlk/mytlk.tlk.json"], "{step:?}");
        assert!(step.text(&dir).iter().any(|l| l.contains("mytlk.tlk.json changed")));
        assert!(w.step(&rm).unwrap().rebuild.is_empty(), "seen once");
    }

    fn blueprint(tag: &str) -> Vec<u8> {
        let mut g = mg_gff::Gff::new(*b"UTC ");
        g.root.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
        g.to_bytes().unwrap()
    }

    /// Changes `key` in the project as another program would (`mg set`):
    /// the project opened apart, changed, saved.
    fn change_outside(project: &Path, key: ResKey, data: Option<Vec<u8>>) {
        let mut m = Module::open(project).unwrap();
        match data {
            Some(d) => m.set(key, d),
            None => {
                m.remove(&key);
            }
        }
        m.save().unwrap();
    }

    /// Only what changed is read and taken: a blueprint changed goes out,
    /// the others are the very bytes read at the start (not read again);
    /// a look at the files the watch named reads them alone; an area
    /// added has the project read whole again, which puts it in
    /// module.ifo's area list, as nasher does.
    #[test]
    fn only_the_files_changed_are_read_again() {
        let dir = mg_testkit::scratch_dir("mg-watch-files");
        let project = dir.join("project");
        let mut m = Module::new();
        m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        let (one, two) = (
            ResKey::parse("one", ResType::UTC).unwrap(),
            ResKey::parse("two", ResType::UTC).unwrap(),
        );
        m.set(one, blueprint("ONE"));
        m.set(two, blueprint("TWO"));
        m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
            .unwrap();
        let rm = ResMan::new();
        let (dev, built) = (dir.join("dev"), dir.join("built.mod"));
        std::fs::create_dir_all(&dev).unwrap();
        let (mut w, _) = Watcher::start(
            open(&project, None).unwrap(),
            &project,
            None,
            &dev,
            Some(built),
            false,
            &rm,
        )
        .unwrap();
        let read = |w: &Watcher, k: &ResKey| w.module.shared(k).cloned().unwrap();
        let two_read = read(&w, &two);
        assert!(w.step(&rm).unwrap().is_empty(), "nothing changed: nothing to do");

        change_outside(&project, one, Some(blueprint("ONE AGAIN")));
        let step = w.step(&rm).unwrap();
        assert_eq!(step.written, ["one.utc"], "{step:?}");
        assert!(step.rebuild.is_empty(), "{step:?}");
        assert_eq!(&read(&w, &one)[..], &blueprint("ONE AGAIN")[..]);
        assert!(Arc::ptr_eq(&two_read, &read(&w, &two)), "the other blueprint wasn't read again");
        assert!(dev.join("one.utc").is_file());

        // Named alone: the files named are looked at, the others aren't.
        change_outside(&project, two, Some(blueprint("TWO AGAIN")));
        let one_file = w.module.file_of(&one).unwrap().to_path_buf();
        assert!(w.step_of(Some(vec![one_file]), &rm).unwrap().is_empty());
        assert!(Arc::ptr_eq(&two_read, &read(&w, &two)), "not named, not read");
        let two_file = w.module.file_of(&two).unwrap().to_path_buf();
        let step = w.step_of(Some(vec![two_file]), &rm).unwrap();
        assert_eq!(step.written, ["two.utc"], "{step:?}");

        // An area added: built again, and listed.
        let area = ResKey::parse("keep", ResType::ARE).unwrap();
        let mut are = mg_gff::Gff::new(*b"ARE ");
        are.root.set("Tag", mg_gff::Value::String(b"KEEP".to_vec()));
        change_outside(&project, area, Some(are.to_bytes().unwrap()));
        let step = w.step(&rm).unwrap();
        assert!(step.rebuild.contains(&"keep.are".to_string()), "{step:?}");
        let listed: Vec<String> =
            w.module().areas().unwrap().iter().map(|a| a.to_string()).collect();
        assert_eq!(listed, ["keep"]);
        // And removed: built again.
        change_outside(&project, area, None);
        let step = w.step(&rm).unwrap();
        assert!(step.rebuild.contains(&"keep.are".to_string()), "{step:?}");
        assert!(!w.module().contains(&area));
    }

    /// `--target`: the nasher project's target named is read, not its
    /// default; a module folder has none to name.
    #[test]
    fn a_target_other_than_the_default_is_followed() {
        let dir = mg_testkit::scratch_dir("mg-watch-target");
        let project = dir.join("project");
        let mut m = Module::new();
        m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        let one = ResKey::parse("one", ResType::UTC).unwrap();
        m.set(one, blueprint("ONE"));
        m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
            .unwrap();
        // A second target, of other sources.
        let other = ResKey::parse("other", ResType::UTC).unwrap();
        let mut o = Module::new();
        o.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        o.set(other, blueprint("OTHER"));
        let elsewhere = dir.join("elsewhere");
        o.save_as(&ModuleLocation::Project { root: elsewhere.clone(), target: "default".into() })
            .unwrap();
        let theirs = Module::open(&elsewhere).unwrap();
        let src = |m: &Module, k: &ResKey| m.file_of(k).unwrap().to_path_buf();
        std::fs::create_dir_all(project.join("other")).unwrap();
        for k in [Module::info_key(), other] {
            let from = src(&theirs, &k);
            std::fs::copy(&from, project.join("other").join(from.file_name().unwrap())).unwrap();
        }
        let mut cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
        cfg.push_str(
            "\n[target]\nname = \"other\"\nfile = \"other.mod\"\n  [target.sources]\n  include = \
             \"other/*\"\n",
        );
        std::fs::write(project.join("nasher.cfg"), cfg).unwrap();

        assert!(open(&project, None).unwrap().contains(&one));
        let m = open(&project, Some("other")).unwrap();
        assert!(m.contains(&other) && !m.contains(&one));
        let rm = ResMan::new();
        let dev = dir.join("dev");
        std::fs::create_dir_all(&dev).unwrap();
        let (mut w, _) =
            Watcher::start(m, &project, Some("other"), &dev, None, false, &rm).unwrap();
        // (Read whole again, after nasher.cfg changes: still the target's.)
        let cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
        std::fs::write(project.join("nasher.cfg"), cfg + "\n").unwrap();
        w.step(&rm).unwrap();
        assert!(w.module().contains(&other) && !w.module().contains(&one));

        let folder = dir.join("folder");
        Module::new().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
        let refused = open(&folder, Some("other")).err().unwrap().to_string();
        assert!(refused.contains("--target"), "{refused}");
    }

    /// Scripts compiled with the base game's `x2_inc_cutscene` (a return
    /// type declared one way and defined another, compiled as in Aurora):
    /// the warning is said once, with how many scripts met it.
    #[test]
    fn a_warning_of_the_compiles_is_said_once() {
        let root = mg_testkit::corpus!();
        let rm = ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        let mut m = Module::new();
        let names: Vec<ResKey> = ["cut_a", "cut_b"]
            .iter()
            .map(|n| {
                let k = ResKey::parse(n, ResType::NSS).unwrap();
                m.set(k, b"#include \"x2_inc_cutscene\"\nvoid main() {}\n".to_vec());
                k
            })
            .collect();
        let mut step = Step::default();
        compile(&mut m, &rm, &names, false, &mut step);
        assert_eq!(step.compiled, ["cut_a", "cut_b"]);
        assert_eq!(step.warnings.len(), 1, "{step:?}");
        assert!(step.warnings[0].contains("CutCreateObjectCopy is declared object"));
        assert!(step.warnings[0].ends_with("(in 2 scripts)"));
        assert!(step.text(&root).contains(&step.warnings[0]));
        assert_eq!(step.json()["warnings"][0], step.warnings[0].as_str());
    }

    #[test]
    fn reread_error_retries_and_failed_recompile_keeps_dev_ncs() {
        let dir = mg_testkit::scratch_dir("mg-watch-stale-and-retry");
        let _ = std::fs::remove_dir_all(&dir);
        let project = dir.join("project");
        let dev = dir.join("dev");
        std::fs::create_dir_all(&dev).unwrap();

        let mut m = Module::new();
        m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        let nss = ResKey::parse("hello", ResType::NSS).unwrap();
        m.set(nss, b"void main() {}\n".to_vec());
        m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
            .unwrap();
        let m = open(&project, None).unwrap();
        let nss_path = m.file_of(&nss).unwrap().to_path_buf();
        let root = mg_testkit::corpus!();
        let rm = ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        let (mut w, _) = Watcher::start(m, &project, None, &dev, None, true, &rm).unwrap();

        // First compile writes hello.ncs and hello.ndb on an edit.
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(&nss_path, "void main() { int a = 1; }\n").unwrap();
        let step = w.step(&rm).unwrap();
        assert_eq!(step.compiled, ["hello"], "{step:?}");
        assert!(dev.join("hello.ncs").is_file());
        assert!(dev.join("hello.ndb").is_file());
        let prev_ncs = std::fs::read(dev.join("hello.ncs")).unwrap();

        // Breaking hello.nss drops in-memory NCS/NDB from the module while keeping dev_dir's copy.
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(&nss_path, "void main() { int a = ; }\n").unwrap();
        let step = w.step(&rm).unwrap();
        assert_eq!(step.errors.len(), 1);
        assert!(!w.module().contains(&ResKey::new(nss.resref, ResType::NCS)));
        assert!(!w.module().contains(&ResKey::new(nss.resref, ResType::NDB)));
        assert_eq!(std::fs::read(dev.join("hello.ncs")).unwrap(), prev_ncs);
        assert!(dev.join(WRITTEN).exists());

        // Broken nasher.cfg sets retry=true so look() retries once restored without an FS event.
        let orig_cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
        std::fs::write(&nss_path, "void main() {}\n").unwrap();
        std::fs::write(project.join("nasher.cfg"), "[broken\n").unwrap();
        let err_step = w.step(&rm).unwrap();
        assert!(!err_step.errors.is_empty());
        assert!(w.retry);
        let _ = w.watch.take();
        std::fs::write(project.join("nasher.cfg"), &orig_cfg).unwrap();
        let _ = w.watch.take();
        w.looked = Instant::now() - Duration::from_millis(500);
        let recovered = w.look(&rm).unwrap().expect("retried on look");
        assert!(recovered.errors.is_empty(), "{:?}", recovered.errors);
        assert!(!w.retry);
    }

    /// A project that stays unreadable (its `nasher.cfg` broken) is told
    /// once, and polled once more for a change of the error: not a full
    /// step every 400 ms with the same error said again (and the module
    /// copied, the tests "not run", by the caller, each time).
    #[test]
    fn a_standing_error_is_told_once_and_not_polled() {
        let dir = mg_testkit::scratch_dir("mg-watch-standing-error");
        let _ = std::fs::remove_dir_all(&dir);
        let (project, dev) = (dir.join("project"), dir.join("dev"));
        std::fs::create_dir_all(&dev).unwrap();
        let mut m = Module::new();
        m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
            .unwrap();
        let rm = ResMan::new();
        let (mut w, _) =
            Watcher::start(open(&project, None).unwrap(), &project, None, &dev, None, false, &rm)
                .unwrap();
        let good = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
        let _ = w.watch.take();
        let later = || Instant::now() - Duration::from_millis(500);
        std::fs::write(project.join("nasher.cfg"), "[broken\n").unwrap();
        let first = w.step(&rm).unwrap();
        assert_eq!(first.errors.len(), 1, "{first:?}");
        assert!(w.retry, "the error is looked at once more");
        // The same error again: nothing said, nothing more to poll.
        w.looked = later();
        assert!(w.look(&rm).unwrap().is_none(), "the error was said again");
        assert!(!w.retry, "a standing error is polled for ever");
        // (A standing error is no reason to look again; a change is, and
        // the same error after it is still not said again.)
        assert!(w.step(&rm).unwrap().errors.is_empty(), "the error was said again");
        assert!(!w.retry);
        // A different error is told, and looked at once more.
        std::fs::write(project.join("nasher.cfg"), "[a]\n[other\n").unwrap();
        let step = w.step(&rm).unwrap();
        assert_eq!(step.errors.len(), 1, "{step:?}");
        assert!(w.retry);
        // Mended: the project is read again; a new break is told anew.
        std::fs::write(project.join("nasher.cfg"), &good).unwrap();
        assert!(w.step(&rm).unwrap().errors.is_empty());
        assert!(!w.retry);
        std::fs::write(project.join("nasher.cfg"), "[broken\n").unwrap();
        assert_eq!(w.step(&rm).unwrap().errors.len(), 1, "a break after a mend is told again");
    }
}
