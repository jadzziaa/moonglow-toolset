//! `mg watch`: a module kept as files (a nasher project, a module
//! folder) followed as it is edited. A changed script is compiled, with
//! the scripts that include it, into a development folder, where a
//! running server takes it up at its next use, as it does a changed
//! blueprint or conversation; anything else (a 2DA, an area, module.ifo)
//! needs the module built and the server started again.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use mg_core::ResType;
use mg_module::{Module, ModuleLocation};
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
    /// Files put into the development folder.
    pub written: Vec<String>,
    /// What changed that the server has by heart: the module was built
    /// (if there is somewhere to build it to) and the server is to be
    /// started again.
    pub rebuild: Vec<String>,
}

impl Step {
    pub(crate) fn json(&self) -> Value {
        json!({
            "event": if self.rebuild.is_empty() { "synced" } else { "rebuilt" },
            "compiled": self.compiled, "errors": self.errors,
            "written": self.written, "changed": self.rebuild,
        })
    }

    pub(crate) fn text(&self, dev_dir: &Path) -> Vec<String> {
        let mut out: Vec<String> = self.errors.clone();
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
    dev_dir: PathBuf,
    /// Where the module is built to, if anywhere.
    build_to: Option<PathBuf>,
    module: Module,
    seen: HashMap<ResKey, u64>,
    signature: u64,
    looked: Instant,
    /// Debug files (`.ndb`) are made with the compiled scripts.
    debug: bool,
}

fn stamp(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut h);
    h.finish()
}

fn stamps(m: &Module) -> HashMap<ResKey, u64> {
    m.keys()
        .filter(|k| kind(k.restype) != Kind::Made)
        .map(|k| (*k, stamp(m.get(k).unwrap_or_default())))
        .collect()
}

impl Watcher {
    /// Reads the project, compiles all of its scripts and builds the
    /// module (if `build_to`). What an earlier `mg watch` left in the
    /// development folder is taken away: the module just built has it.
    pub(crate) fn start(
        project: &Path,
        dev_dir: &Path,
        build_to: Option<PathBuf>,
        debug: bool,
        rm: &ResMan,
    ) -> Result<(Watcher, Step)> {
        let mut module =
            crate::open_module(project).with_context(|| project.display().to_string())?;
        let all: Vec<ResKey> = module.keys_of(ResType::NSS).copied().collect();
        let mut step = Step::default();
        compile(&mut module, rm, &all, debug, &mut step);
        let mut w = Watcher {
            debug,
            project: project.to_path_buf(),
            dev_dir: dev_dir.to_path_buf(),
            build_to,
            seen: stamps(&module),
            module,
            signature: crate::serve::signature(project),
            looked: Instant::now(),
        };
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
        copy.save_as(&ModuleLocation::Archive(to.clone()))
            .with_context(|| to.display().to_string())?;
        let _ = std::fs::remove_file(to.with_extension("mod.bak"));
        Ok(())
    }

    /// The module as last read.
    pub(crate) fn module(&self) -> &Module {
        &self.module
    }

    /// Looks at the project, if it is time to and its files have changed.
    pub(crate) fn look(&mut self, rm: &ResMan) -> Result<Option<Step>> {
        if self.looked.elapsed() < Duration::from_millis(400) {
            return Ok(None);
        }
        self.looked = Instant::now();
        let signature = crate::serve::signature(&self.project);
        if signature == self.signature {
            return Ok(None);
        }
        self.signature = signature;
        self.step(rm).map(Some)
    }

    /// Takes up what changed in the project's files.
    pub(crate) fn step(&mut self, rm: &ResMan) -> Result<Step> {
        let mut step = Step::default();
        let mut module = match crate::open_module(&self.project) {
            Ok(module) => module,
            // (A file half written: the next look reads it whole.)
            Err(e) => {
                step.errors.push(format!("{}: {e}", self.project.display()));
                self.signature = 0;
                return Ok(step);
            }
        };
        let now = stamps(&module);
        let changed: Vec<ResKey> =
            now.iter().filter(|(k, s)| self.seen.get(k) != Some(s)).map(|(k, _)| *k).collect();
        let removed: Vec<ResKey> =
            self.seen.keys().filter(|k| !now.contains_key(k)).copied().collect();
        if changed.is_empty() && removed.is_empty() {
            return Ok(step);
        }
        let scripts: Vec<ResKey> =
            changed.iter().filter(|k| kind(k.restype) == Kind::Script).copied().collect();
        let mut to_compile = scripts.clone();
        for k in mg_module::refs::includers(&module, &scripts) {
            if !to_compile.contains(&k) {
                to_compile.push(k);
            }
        }
        // The compiled scripts of those that did not change, as they were.
        let made = |k: &&ResKey| matches!(k.restype, ResType::NCS | ResType::NDB);
        for k in self.module.keys().filter(made) {
            let source = ResKey::new(k.resref, ResType::NSS);
            if module.contains(&source)
                && !to_compile.contains(&source)
                && let Some(data) = self.module.shared(k)
            {
                module.set(*k, data.clone());
            }
        }
        compile(&mut module, rm, &to_compile, self.debug, &mut step);
        let mut heavy: Vec<String> = (changed.iter())
            .filter(|k| kind(k.restype) == Kind::Rebuild)
            .chain(&removed)
            .map(ToString::to_string)
            .collect();
        heavy.sort();
        self.module = module;
        self.seen = now;
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
        let list: String = written.iter().map(|n| format!("{n}\n")).collect();
        std::fs::write(self.dev_dir.join(WRITTEN), list)?;
        // (For the next build, too.)
        self.build()?;
        Ok(step)
    }
}

fn compile(module: &mut Module, rm: &ResMan, names: &[ResKey], debug: bool, step: &mut Step) {
    for r in mg_module::build::compile_named_with(module, rm, names, None, debug) {
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
