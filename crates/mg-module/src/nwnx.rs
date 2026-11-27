//! NWNX for the servers Moonglow runs (`mg serve`, `mg test`, `mg watch
//! --serve`, Test Module's local server): the project's `[nwnx]` settings
//! resolved into what the server is started with, and the checks made
//! before it starts and on what it prints.
//!
//! ```toml
//! [nwnx]                 # moonglow.toml (the team) or .moonglow/user.toml (yours, which wins)
//! use = true             # NWNX for every builder who has a home set
//! home = "~/nwnx"        # NWNX's folder: Binaries/ (NWNX_Core.so and the plugins), NWScript/
//! env = "dev/nwnx.env"   # NAME=value lines for the server, from the project's folder
//! ```
//!
//! NWNX loads into `nwserver` alone, on Linux, through the loader's
//! `LD_PRELOAD`: on another system a project that uses it runs without
//! it, and says so. Its game build can't be read from its files (it is
//! compiled in as numbers): the server's first lines tell it
//! ([`console_note`]), and a mismatch is an error then.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use mg_core::ResType;

use crate::project_file::{Entry, ProjectFile, Reader};

/// `[nwnx]` as the project's two files say it (the builder's own wins).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NwnxRules {
    /// `use`: NWNX on (where a home is set), the builder's over the team's.
    pub use_nwnx: Option<bool>,
    /// `home` in the team's file: a default for every builder.
    pub team_home: Option<String>,
    /// `home` in the builder's own file, which wins.
    pub own_home: Option<String>,
    /// `env`: the environment file, from the project's folder, the
    /// builder's over the team's.
    pub env: Option<String>,
}

/// Reads `[nwnx]` of either file into `p` (`own`: the builder's file).
pub(crate) fn read_rules(p: &mut ProjectFile, r: &mut Reader, entry: Entry, own: bool) {
    for e in r.section(entry) {
        match e.0.get_ref().as_ref() {
            "use" => match e.1.get_ref() {
                toml::de::DeValue::Boolean(b) => p.nwnx.use_nwnx = Some(*b),
                _ => r.warn(e.1.span().start, "use is true or false"),
            },
            "home" => {
                if let Some(t) = r.text(e).map(|t| t.trim().to_string()) {
                    let t = Some(t).filter(|t| !t.is_empty());
                    match own {
                        true => p.nwnx.own_home = t,
                        false => p.nwnx.team_home = t,
                    }
                }
            }
            "env" => {
                if let Some(t) = r.text(e).map(|t| t.trim().to_string()) {
                    p.nwnx.env = Some(t).filter(|t| !t.is_empty());
                }
            }
            _ => r.unknown("nwnx", e),
        }
    }
}

/// How bad a note is: an error stops the start, a warning doesn't.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Info,
    Warning,
    Error,
}

/// Something to say of NWNX, before the start or while the server runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub level: Level,
    pub text: String,
}

impl Note {
    fn error(text: impl Into<String>) -> Note {
        Note { level: Level::Error, text: text.into() }
    }

    fn warning(text: impl Into<String>) -> Note {
        Note { level: Level::Warning, text: text.into() }
    }

    fn info(text: impl Into<String>) -> Note {
        Note { level: Level::Info, text: text.into() }
    }

    /// As a line of `mg`'s: `warning: …`, `error: …`, or the text.
    pub fn line(&self) -> String {
        match self.level {
            Level::Info => self.text.clone(),
            Level::Warning => format!("warning: {}", self.text),
            Level::Error => format!("error: {}", self.text),
        }
    }
}

/// Whose NWNX the run takes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Choice {
    /// The project's `[nwnx]` (none without one).
    #[default]
    Project,
    /// This home, whatever the project says (`--nwnx DIR`).
    Home(PathBuf),
    /// None (`--no-nwnx`).
    Off,
}

/// What [`resolve`] is asked.
#[derive(Debug, Clone)]
pub struct Options<'a> {
    pub choice: Choice,
    /// Whether the server runs on Linux, the one system NWNX loads on
    /// (asked, so that the other systems' way is tested too).
    pub linux: bool,
    /// The module, whose scripts are looked at (what they include of
    /// NWNX's, whether they call it).
    pub module: Option<&'a crate::Module>,
    /// Scripts are replaced while the server runs (`mg watch --serve`,
    /// Test Module's `development` folder).
    pub live: bool,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            choice: Choice::Project,
            linux: cfg!(target_os = "linux"),
            module: None,
            live: false,
        }
    }
}

/// NWNX as a server is started with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nwnx {
    /// NWNX's folder.
    pub home: PathBuf,
    /// Where its plugins are loaded from (`<home>/Binaries`, or the
    /// environment file's `NWNX_CORE_LOAD_PATH`).
    pub binaries: PathBuf,
    /// `NWNX_Core.so`, the library preloaded.
    pub core: PathBuf,
    /// The environment file, where the project names one.
    pub env_file: Option<PathBuf>,
    /// Its lines, in its order.
    pub env: Vec<(String, String)>,
    /// The plugins NWNX loads by these settings (`Util`, `Tdod`), Core's
    /// included.
    pub plugins: BTreeSet<String>,
}

impl Nwnx {
    /// The server's environment: `LD_PRELOAD` (NWNX's core),
    /// `NWNX_CORE_LOAD_PATH` (its plugins), then the environment file's
    /// lines (which may set them otherwise). Nothing of the caller's own
    /// environment: the server is started with a clean one
    /// (`mg_serve::Options::clean_env`).
    pub fn server_env(&self) -> Vec<(OsString, OsString)> {
        let mut out: Vec<(OsString, OsString)> = vec![
            ("LD_PRELOAD".into(), self.core.clone().into()),
            ("NWNX_CORE_LOAD_PATH".into(), self.binaries.clone().into()),
        ];
        // (The file's own NWNX_CORE_LOAD_PATH is `binaries`, made whole:
        // the server runs in its own folder, where a relative one is not.)
        let theirs = self.env.iter().filter(|(k, _)| k != "NWNX_CORE_LOAD_PATH");
        out.extend(theirs.map(|(k, v)| (k.into(), v.into())));
        out
    }

    /// Whether NWNX loads the plugin `name` (`Util`, `util`).
    pub fn loads(&self, name: &str) -> bool {
        self.plugins.iter().any(|p| p.eq_ignore_ascii_case(name))
    }
}

/// What [`resolve`] found: NWNX to start the server with (none: without
/// it), and what to say first.
#[derive(Debug, Clone, Default)]
pub struct Resolved {
    pub nwnx: Option<Nwnx>,
    pub notes: Vec<Note>,
}

impl Resolved {
    /// Whether a note is an error: the server is not to be started.
    pub fn failed(&self) -> bool {
        self.notes.iter().any(|n| n.level == Level::Error)
    }

    /// The errors' texts, joined, for a message that stops the start.
    pub fn errors(&self) -> Vec<&str> {
        (self.notes.iter()).filter(|n| n.level == Level::Error).map(|n| n.text.as_str()).collect()
    }
}

/// What a run of a module says once, whatever the system: NWNX runs on
/// Linux servers only.
pub const LINUX_ONLY: &str =
    "NWNX runs only on Linux servers: the server runs without it on this system";

/// The words NWNX reads as true and false, case aside (its
/// `String::FromString<bool>`); anything else is no value.
fn truth(value: &str) -> Option<bool> {
    let v = value.trim();
    let is = |words: &[&str]| words.iter().any(|w| w.eq_ignore_ascii_case(v));
    if is(&["t", "true", "y", "yes", "1"]) {
        Some(true)
    } else if is(&["f", "false", "n", "no", "0"]) {
        Some(false)
    } else {
        None
    }
}

/// A path as a project file writes one: `~/` from the builder's home, a
/// relative one from the project's folder.
fn path_from(text: &str, root: Option<&Path>) -> PathBuf {
    if let Some(rest) = text.strip_prefix("~/").or_else(|| text.strip_prefix("~\\"))
        && let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
    {
        return PathBuf::from(home).join(rest);
    }
    let path = PathBuf::from(text);
    match root {
        Some(root) if path.is_relative() => root.join(path),
        _ => path,
    }
}

/// The plugins in a `Binaries` folder by their name without `NWNX_` and
/// `.so` (`Util`), each with its file.
fn plugins_in(binaries: &Path) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(binaries).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if let Some(stem) = name.strip_prefix("NWNX_").and_then(|n| n.strip_suffix(".so"))
            && !stem.is_empty()
        {
            out.insert(stem.to_string(), e.path());
        }
    }
    out
}

/// Whether NWNX skips a plugin by these settings, as its core reads them:
/// `NWNX_<PLUGIN>_SKIP`, else `NWNX_CORE_SKIP_ALL`; Core is always
/// loaded; Experimental only with `NWNX_CORE_LOAD_EXPERIMENTAL_PLUGIN`.
fn skipped(plugin: &str, env: &BTreeMap<String, String>) -> bool {
    let get = |k: &str| env.get(k).and_then(|v| truth(v));
    if plugin.eq_ignore_ascii_case("Core") {
        return false;
    }
    if plugin.eq_ignore_ascii_case("Experimental")
        && !get("NWNX_CORE_LOAD_EXPERIMENTAL_PLUGIN").unwrap_or(false)
    {
        return true;
    }
    let all = get("NWNX_CORE_SKIP_ALL").unwrap_or(false);
    get(&format!("NWNX_{}_SKIP", plugin.to_ascii_uppercase())).unwrap_or(all)
}

/// Why a plugin is skipped, in the settings' words.
fn why_skipped(plugin: &str, env: &BTreeMap<String, String>) -> String {
    let key = format!("NWNX_{}_SKIP", plugin.to_ascii_uppercase());
    match env.get(&key) {
        Some(v) if truth(v).is_some() => format!("{key}={v}"),
        _ => match env.get("NWNX_CORE_SKIP_ALL") {
            Some(all) => format!("NWNX_CORE_SKIP_ALL={all} and no {key}=n"),
            None => format!("{key} is not set"),
        },
    }
}

/// The project's NWNX for a server, by `o`, with what to say first: the
/// errors that stop the start (a home or a file missing, a plugin switched
/// on that isn't there) and the warnings that don't (a plugin the scripts
/// include switched off, scripts that call NWNX while it is off, scripts
/// that keep their first version). The project's files are only read.
pub fn resolve(project: &ProjectFile, o: &Options) -> Resolved {
    let mut out = Resolved::default();
    let root = project.place().map(|p| p.root.clone());
    let rules = &project.nwnx;
    let home = match &o.choice {
        Choice::Off => None,
        Choice::Home(dir) => Some(dir.clone()),
        Choice::Project => match rules.use_nwnx {
            Some(true) => {
                let home = rules.own_home.as_deref().or(rules.team_home.as_deref());
                if home.is_none() {
                    out.notes.push(Note::warning(
                        "the project uses NWNX ([nwnx] use = true), but no home is set: put \
                         [nwnx] home = \"NWNX's folder\" in .moonglow/user.toml; the server runs \
                         without NWNX",
                    ));
                }
                home.map(|h| path_from(h, root.as_deref()))
            }
            _ => None,
        },
    };
    let home = match home {
        Some(home) if !o.linux => {
            let _ = home;
            out.notes.push(Note::warning(LINUX_ONLY));
            None
        }
        other => other,
    };
    out.nwnx = home.and_then(|home| nwnx_at(home, rules, root.as_deref(), &mut out.notes));
    if let Some(module) = o.module {
        out.notes.extend(script_notes(module, out.nwnx.as_ref(), o.live));
    }
    if out.failed() {
        out.nwnx = None;
    }
    out
}

/// NWNX in `home` with the project's environment file, checked: `None`
/// (and the errors said) where it can't be started.
fn nwnx_at(
    home: PathBuf,
    rules: &NwnxRules,
    root: Option<&Path>,
    notes: &mut Vec<Note>,
) -> Option<Nwnx> {
    let home = std::path::absolute(&home).unwrap_or(home);
    if !home.is_dir() {
        notes.push(Note::error(format!("NWNX's folder {} is not there", home.display())));
        return None;
    }
    let mut env = Vec::new();
    let env_file = rules.env.as_deref().map(|e| path_from(e, root));
    if let Some(file) = &env_file {
        match std::fs::read(file) {
            Ok(bytes) => match mg_core::env_file::parse(&String::from_utf8_lossy(&bytes)) {
                Ok(lines) => env = lines,
                Err(e) => {
                    notes.push(Note::error(format!("{}: {e}", file.display())));
                    return None;
                }
            },
            Err(e) => {
                notes.push(Note::error(format!(
                    "NWNX's environment file {} can't be read: {e}",
                    file.display()
                )));
                return None;
            }
        }
    }
    let settings: BTreeMap<String, String> = env.iter().cloned().collect();
    let binaries = match settings.get("NWNX_CORE_LOAD_PATH").filter(|p| !p.trim().is_empty()) {
        Some(p) => path_from(p.trim(), root),
        None => home.join("Binaries"),
    };
    let core = home.join("Binaries").join("NWNX_Core.so");
    if !core.is_file() {
        notes.push(Note::error(format!("NWNX's core {} is not there", core.display())));
        return None;
    }
    let present = plugins_in(&binaries);
    let mut plugins = BTreeSet::new();
    let mut missing = Vec::new();
    // (Every plugin the settings name, and every one in the folder.)
    let named = settings.keys().filter_map(|k| {
        let p = k.strip_prefix("NWNX_")?.strip_suffix("_SKIP")?;
        (p != "CORE").then(|| p.to_string())
    });
    for plugin in present.keys().cloned().chain(named) {
        let there = present.keys().find(|p| p.eq_ignore_ascii_case(&plugin));
        if skipped(&plugin, &settings) {
            continue;
        }
        match there {
            Some(p) => {
                plugins.insert(p.clone());
            }
            None if !missing.iter().any(|m: &String| m.eq_ignore_ascii_case(&plugin)) => {
                missing.push(plugin)
            }
            None => {}
        }
    }
    plugins.insert("Core".into());
    for plugin in &missing {
        let up = plugin.to_ascii_uppercase();
        notes.push(Note::error(format!(
            "NWNX_{up}_SKIP switches the plugin {plugin} on, but {} has no NWNX_{plugin}.so",
            binaries.display()
        )));
    }
    if settings.get("NWNX_CORE_SKIP").and_then(|v| truth(v)) == Some(true) {
        notes.push(Note::warning("NWNX_CORE_SKIP is set: NWNX loads, but starts nothing"));
    }
    if !missing.is_empty() {
        return None;
    }
    Some(Nwnx { home, binaries, core, env_file, env, plugins })
}

/// What a module's scripts use of NWNX: the includes of NWNX's they name
/// (`nwnx_creature`), by how many scripts, and the scripts that call it at
/// all (an NWNX include, or EE's `NWNXCall`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptUse {
    /// The includes by name (lower case), each with the scripts that
    /// include it.
    pub includes: BTreeMap<String, Vec<String>>,
    /// The scripts that include one of NWNX's or call `NWNXCall`.
    pub calling: Vec<String>,
}

/// What `module`'s scripts use of NWNX ([`ScriptUse`]).
pub fn script_use(module: &crate::Module) -> ScriptUse {
    let mut out = ScriptUse::default();
    let mut keys: Vec<_> = module.keys_of(ResType::NSS).copied().collect();
    keys.sort_by_key(|k| k.resref.to_string());
    for key in keys {
        let Some(src) = module.get(&key) else { continue };
        let name = key.resref.to_string();
        let mut calls = false;
        for inc in mg_script::lex::includes(src) {
            let inc = String::from_utf8_lossy(&src[inc.name]).to_ascii_lowercase();
            if inc.starts_with("nwnx") {
                calls = true;
                let names = out.includes.entry(inc).or_default();
                if !names.contains(&name) {
                    names.push(name.clone());
                }
            }
        }
        calls |= calls_nwnx(src);
        if calls {
            out.calling.push(name);
        }
    }
    out
}

/// Whether a source calls `NWNXCall` (outside its comments).
fn calls_nwnx(src: &[u8]) -> bool {
    let toks = mg_script::lex::tokenize(src);
    toks.iter().any(|t| t.kind.is_significant() && &src[t.span.clone()] == b"NWNXCall")
}

/// The plugin an NWNX include is for, as it names it
/// (`const string NWNX_Administration = "NWNX_Administration";` in
/// `nwnx_admin`): from the module's own copy of the include, else NWNX's
/// `NWScript` folder. None for one of NWNX's own helpers (`nwnx_consts`).
fn plugin_of(include: &str, module: &crate::Module, home: Option<&Path>) -> Option<String> {
    let own = mg_core::ResRef::from_bytes(include.as_bytes())
        .ok()
        .and_then(|r| module.get(&mg_resman::ResKey::new(r, ResType::NSS)).map(|b| b.to_vec()));
    let src =
        own.or_else(|| std::fs::read(home?.join("NWScript").join(format!("{include}.nss"))).ok())?;
    let text = String::from_utf8_lossy(&src);
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("const string NWNX_")?;
        let (_, value) = rest.split_once('=')?;
        let value = value.trim().trim_end_matches(';').trim().trim_matches('"');
        value.strip_prefix("NWNX_").filter(|p| !p.is_empty()).map(str::to_string)
    })
}

/// A few names of a list, for a message: `a, b, c and 4 more`.
fn a_few(names: &[String]) -> String {
    const SHOWN: usize = 3;
    let mut text = names.iter().take(SHOWN).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > SHOWN {
        text.push_str(&format!(" and {} more", names.len() - SHOWN));
    }
    text
}

fn scripts(n: usize) -> String {
    format!("{n} script{}", if n == 1 { "" } else { "s" })
}

/// The warnings of a module's scripts with NWNX as it is (`None`: off).
pub fn script_notes(module: &crate::Module, nwnx: Option<&Nwnx>, live: bool) -> Vec<Note> {
    let uses = script_use(module);
    let mut notes = Vec::new();
    let Some(nwnx) = nwnx else {
        if !uses.calling.is_empty() {
            notes.push(Note::warning(format!(
                "{} call NWNX, which is off: those calls return nothing ({})",
                scripts(uses.calling.len()),
                a_few(&uses.calling)
            )));
        }
        return notes;
    };
    let settings: BTreeMap<String, String> = nwnx.env.iter().cloned().collect();
    let mut said = BTreeSet::new();
    for (include, by) in &uses.includes {
        let Some(plugin) = plugin_of(include, module, Some(&nwnx.home)) else { continue };
        if nwnx.loads(&plugin) || !said.insert(plugin.to_ascii_lowercase()) {
            continue;
        }
        let there = plugins_in(&nwnx.binaries).keys().any(|p| p.eq_ignore_ascii_case(&plugin));
        let why = match there {
            true => why_skipped(&plugin, &settings),
            false => format!("{} has no NWNX_{plugin}.so", nwnx.binaries.display()),
        };
        notes.push(Note::warning(format!(
            "the plugin {plugin} is off ({why}), and {} include {include}: its calls return \
             nothing",
            scripts(by.len())
        )));
    }
    let caches = settings.get("NWNX_OPTIMIZATIONS_CACHE_SCRIPTS").and_then(|v| truth(v));
    if live && caches == Some(true) && nwnx.loads("Optimizations") {
        notes.push(Note::warning(
            "scripts keep their first version: development/ and NWNX_Util_AddScript won't \
             replace them (NWNX_OPTIMIZATIONS_CACHE_SCRIPTS)",
        ));
    }
    notes
}

/// What a line the server printed (its console, or the loader's on its
/// standard error) says of NWNX: the build it was made for against the
/// server's, a plugin that failed or was skipped, a preload that failed.
/// `nwnx`: whether the server was started with NWNX.
pub fn console_note(line: &str, nwnx: bool) -> Option<Note> {
    let l = line.trim();
    // The loader: "ERROR: ld.so: object '…' from LD_PRELOAD cannot be
    // preloaded (…): ignored." The server then runs without NWNX.
    if l.contains("LD_PRELOAD") && l.contains("cannot be preloaded") {
        return Some(Note::error(format!("NWNX was not loaded into the server: {l}")));
    }
    if !nwnx {
        return None;
    }
    if let Some(rest) = l.strip_prefix("NWNX: Expected build ") {
        return Some(Note::error(format!(
            "NWNX is for another game build than the server's ({rest}): it stops the server; use \
             an NWNX built for this game"
        )));
    }
    if l.starts_with("NWNX: Will terminate") || l.starts_with("NWNX: Could not determine build") {
        return Some(Note::error(l.to_string()));
    }
    if l.starts_with("NWNX: WARNING: POSTFIX VERSION MISMATCH") {
        return Some(Note::warning(format!(
            "NWNX is for another revision of the server's build: {l}"
        )));
    }
    // NWNX's own log lines: "E [time] [NWNX_Core] [Plugin.cpp:44] Unable to load plugin …".
    let message = l.split("] ").last().unwrap_or(l);
    if l.starts_with("F ") {
        return Some(Note::error(format!("NWNX: {message}")));
    }
    if l.starts_with("E ") && l.contains("[NWNX_") {
        return Some(Note::warning(format!("NWNX: {message}")));
    }
    if message.starts_with("Skipping plugin ") && l.contains("[NWNX_") {
        return Some(Note::info(format!("NWNX: {message}")));
    }
    None
}

/// The build NWNX says it was made for, from its first line (`Starting
/// NWNX 8193.37-17 [2d8465a]`), or the server's own (`Build: 8193.37-17
/// [26c6e573]`): `8193.37-17`.
pub fn build_in(line: &str) -> Option<(Source, String)> {
    let l = line.trim();
    let (source, rest) = if let Some(r) = l.strip_prefix("Starting NWNX ") {
        (Source::Nwnx, r)
    } else if let Some(r) = l.strip_prefix("Build: ") {
        (Source::Server, r)
    } else {
        return None;
    };
    let build = rest.split_whitespace().next()?;
    build.starts_with(|c: char| c.is_ascii_digit()).then(|| (source, build.to_string()))
}

/// Whose build a line tells ([`build_in`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Nwnx,
    Server,
}

/// What the server's first lines told of the two builds ([`build_in`]),
/// followed as they come.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Builds {
    pub nwnx: Option<String>,
    pub server: Option<String>,
    said: bool,
}

impl Builds {
    /// Takes a line; once both builds are told, the note of how they
    /// compare (once): an error where the game build differs (NWNX stops
    /// the server), a warning where only its revision does.
    pub fn line(&mut self, line: &str) -> Option<Note> {
        let (source, build) = build_in(line)?;
        match source {
            Source::Nwnx => self.nwnx = Some(build),
            Source::Server => self.server = Some(build),
        }
        let (Some(n), Some(s)) = (&self.nwnx, &self.server) else { return None };
        if std::mem::replace(&mut self.said, true) {
            return None;
        }
        let game = |b: &str| b.split('-').next().unwrap_or(b).to_string();
        Some(if n == s {
            Note::info(format!("NWNX {n} on server build {s}"))
        } else if game(n) == game(s) {
            Note::warning(format!(
                "NWNX was built for {n}, the server is {s}: the same game build, another revision"
            ))
        } else {
            Note::error(format!(
                "NWNX was built for game build {n}, the server is {s}: use an NWNX built for \
                 this game"
            ))
        })
    }

    /// What to say once the module has loaded with NWNX asked for but no
    /// line of NWNX's seen: it was not loaded.
    pub fn none_seen(&self) -> Option<Note> {
        self.nwnx.is_none().then(|| {
            Note::error("the server ran without NWNX: it never said \"Starting NWNX\"".to_string())
        })
    }
}

#[cfg(test)]
mod tests;
