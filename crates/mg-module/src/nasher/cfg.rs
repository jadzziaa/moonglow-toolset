//! `nasher.cfg` and nasher's user settings, read as nasher reads them: Nim's
//! parsecfg format (sections, `key = value` with keys that may repeat), with
//! nasher's rules for targets, inheritance and variables.

use std::path::{Path, PathBuf};

use mg_core::Codepage;

use super::glob;
use crate::ModuleError;

fn bad(file: &Path, message: impl Into<String>) -> ModuleError {
    ModuleError::Source { name: file.display().to_string(), message: message.into() }
}

/// The sections of a file in `nasher.cfg`'s format, in order: each one's
/// name, the line it starts on, and its `key = value` pairs (a key may
/// repeat). For other files written the same way (a plugin's manifest).
pub fn sections(text: &str) -> Result<Vec<Section>, String> {
    let mut out: Vec<Section> = Vec::new();
    for (line, event) in parse_events(text)? {
        match event {
            Event::Section(name) => out.push(Section { name, line, pairs: Vec::new() }),
            Event::Pair(key, value) => match out.last_mut() {
                Some(section) => section.pairs.push((key, value)),
                None => return Err(format!("line {line}: {key} is in no [section]")),
            },
        }
    }
    Ok(out)
}

/// A section of a file in `nasher.cfg`'s format ([`sections`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub line: usize,
    pub pairs: Vec<(String, String)>,
}

impl Section {
    /// The first value of a key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

/// One parsecfg event.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Section(String),
    /// `key = value` (the value is empty for a bare key).
    Pair(String, String),
}

/// Reads parsecfg text into events.
fn parse_events(text: &str) -> Result<Vec<(usize, Event)>, String> {
    let b = text.as_bytes();
    let mut i = 0;
    let mut line = 1;
    let mut events = Vec::new();
    // A symbol: parsecfg's SymChars, trailing spaces trimmed.
    let sym_char = |c: u8| {
        c.is_ascii_alphanumeric()
            || matches!(c, b'_' | b' ' | b'.' | b'/' | b'\\' | b'-')
            || c >= 0x80
    };
    let skip = |i: &mut usize, line: &mut usize| {
        while *i < b.len() {
            match b[*i] {
                b' ' | b'\t' => *i += 1,
                b'#' | b';' => {
                    while *i < b.len() && b[*i] != b'\n' && b[*i] != b'\r' {
                        *i += 1;
                    }
                }
                b'\n' => {
                    *line += 1;
                    *i += 1;
                }
                b'\r' => {
                    *i += 1;
                    if *i < b.len() && b[*i] == b'\n' {
                        *i += 1;
                    }
                    *line += 1;
                }
                _ => break,
            }
        }
    };
    #[derive(PartialEq)]
    enum Tok {
        Sym(String),
        Eq,
        Colon,
        DashDash,
        Open,
        Close,
        Eof,
    }
    let token = |i: &mut usize, line: &mut usize| -> Result<Tok, String> {
        skip(i, line);
        let Some(&c) = b.get(*i) else { return Ok(Tok::Eof) };
        let string = |i: &mut usize, line: &mut usize, raw: bool| -> Result<String, String> {
            let mut out = Vec::new();
            if b.get(*i + 1) == Some(&b'"') && b.get(*i + 2) == Some(&b'"') {
                // """long string""", its leading newline skipped.
                *i += 3;
                if b.get(*i) == Some(&b'\r') {
                    *i += 1;
                }
                if b.get(*i) == Some(&b'\n') {
                    *i += 1;
                    *line += 1;
                }
                loop {
                    match b.get(*i) {
                        None => return Err(format!("line {line}: unterminated string")),
                        Some(b'"')
                            if b.get(*i + 1) == Some(&b'"') && b.get(*i + 2) == Some(&b'"') =>
                        {
                            *i += 3;
                            break;
                        }
                        Some(b'\r') => {
                            *i += 1;
                            if b.get(*i) == Some(&b'\n') {
                                *i += 1;
                            }
                            *line += 1;
                            out.push(b'\n');
                        }
                        Some(&ch) => {
                            if ch == b'\n' {
                                *line += 1;
                            }
                            out.push(ch);
                            *i += 1;
                        }
                    }
                }
            } else {
                *i += 1;
                loop {
                    match b.get(*i) {
                        None | Some(b'\n' | b'\r') => {
                            return Err(format!("line {line}: unterminated string"));
                        }
                        Some(b'"') => {
                            *i += 1;
                            break;
                        }
                        Some(b'\\') if !raw => {
                            *i += 1;
                            let e = b.get(*i).copied().unwrap_or(0);
                            *i += 1;
                            match e.to_ascii_lowercase() {
                                b'n' | b'l' => out.push(b'\n'),
                                b'r' | b'c' => out.push(b'\r'),
                                b'f' => out.push(0x0c),
                                b'e' => out.push(0x1b),
                                b'a' => out.push(0x07),
                                b'b' => out.push(0x08),
                                b'v' => out.push(0x0b),
                                b't' => out.push(b'\t'),
                                b'\'' | b'"' | b'\\' => out.push(e),
                                b'x' => {
                                    let mut v = 0u32;
                                    for _ in 0..2 {
                                        match b.get(*i).and_then(|c| (*c as char).to_digit(16)) {
                                            Some(d) => {
                                                v = v * 16 + d;
                                                *i += 1;
                                            }
                                            None => break,
                                        }
                                    }
                                    out.push(v as u8);
                                }
                                b'0'..=b'9' => {
                                    let mut v = u32::from(e - b'0');
                                    while let Some(d) = b.get(*i).filter(|c| c.is_ascii_digit()) {
                                        v = v * 10 + u32::from(d - b'0');
                                        *i += 1;
                                    }
                                    if v > 255 {
                                        return Err(format!("line {line}: bad escape"));
                                    }
                                    out.push(v as u8);
                                }
                                _ => return Err(format!("line {line}: bad escape")),
                            }
                        }
                        Some(&ch) => {
                            out.push(ch);
                            *i += 1;
                        }
                    }
                }
            }
            Ok(String::from_utf8_lossy(&out).into_owned())
        };
        Ok(match c {
            b'=' => {
                *i += 1;
                Tok::Eq
            }
            b':' => {
                *i += 1;
                Tok::Colon
            }
            b'[' => {
                *i += 1;
                Tok::Open
            }
            b']' => {
                *i += 1;
                Tok::Close
            }
            b'-' if b.get(*i + 1) == Some(&b'-') => {
                *i += 2;
                Tok::DashDash
            }
            b'"' => Tok::Sym(string(i, line, false)?),
            b'r' | b'R' if b.get(*i + 1) == Some(&b'"') => {
                *i += 1;
                Tok::Sym(string(i, line, true)?)
            }
            _ => {
                let start = *i;
                *i += 1;
                while *i < b.len() && sym_char(b[*i]) {
                    *i += 1;
                }
                let s = String::from_utf8_lossy(&b[start..*i]);
                Tok::Sym(s.trim_end_matches(' ').to_string())
            }
        })
    };
    let mut tok = token(&mut i, &mut line)?;
    loop {
        let at = line;
        match tok {
            Tok::Eof => break,
            Tok::Open => {
                let Tok::Sym(name) = token(&mut i, &mut line)? else {
                    return Err(format!("line {at}: section name expected"));
                };
                if token(&mut i, &mut line)? != Tok::Close {
                    return Err(format!("line {at}: ']' expected"));
                }
                events.push((at, Event::Section(name)));
                tok = token(&mut i, &mut line)?;
            }
            Tok::DashDash | Tok::Sym(_) => {
                if tok == Tok::DashDash {
                    tok = token(&mut i, &mut line)?;
                }
                let Tok::Sym(key) = tok else { return Err(format!("line {at}: key expected")) };
                tok = token(&mut i, &mut line)?;
                let mut value = String::new();
                if matches!(tok, Tok::Eq | Tok::Colon) {
                    let Tok::Sym(v) = token(&mut i, &mut line)? else {
                        return Err(format!("line {at}: value expected after {key:?}"));
                    };
                    value = v;
                    tok = token(&mut i, &mut line)?;
                }
                events.push((at, Event::Pair(key, value)));
            }
            Tok::Eq | Tok::Colon | Tok::Close => {
                return Err(format!("line {at}: unexpected token"));
            }
        }
    }
    Ok(events)
}

/// A nasher target: a file packed from the source tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub description: String,
    /// The file packed, e.g. `mymodule.mod` (may include folders).
    pub file: String,
    pub parent: String,
    pub branch: String,
    pub mod_name: String,
    pub mod_min_game_version: String,
    pub mod_description: String,
    pub groups: Vec<String>,
    pub flags: Vec<String>,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    /// Packed files left out of the target's file.
    pub filters: Vec<String>,
    /// Scripts not compiled.
    pub skip_compile: Vec<String>,
    pub variables: Vec<(String, String)>,
    /// Unpack rules: a file-name pattern and the folder for files it matches.
    pub rules: Vec<(String, String)>,
}

impl Target {
    /// Whether the target packs a module.
    pub fn is_module(&self) -> bool {
        Path::new(&self.file)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mod") || e.eq_ignore_ascii_case("nwm"))
    }

    /// Fills fields it doesn't set from `defaults` (its parent or the
    /// package), as nasher does: lists whole, variables merged.
    fn inherit(&mut self, defaults: &Target) {
        macro_rules! take {
            ($($f:ident),*) => {$(
                if self.$f.is_empty() {
                    self.$f = defaults.$f.clone();
                }
            )*};
        }
        take!(
            file,
            branch,
            mod_name,
            mod_min_game_version,
            mod_description,
            groups,
            flags,
            includes,
            excludes,
            filters,
            skip_compile,
            rules
        );
        for (k, v) in &defaults.variables {
            if !self.variables.iter().any(|(key, _)| key == k) {
                self.variables.push((k.clone(), v.clone()));
            }
        }
    }

    /// Expands `$var` and `${var}` in every field but the name.
    fn resolve(&mut self, env: &dyn Fn(&str) -> Option<String>) -> Result<(), String> {
        let mut vars = self.variables.clone();
        vars.push(("target".into(), self.name.clone()));
        vars.push(("ext".into(), "$ext".into()));
        let lookup = |key: &str| -> Result<String, String> {
            if let Some((_, v)) = vars.iter().rev().find(|(k, _)| k == key) {
                return Ok(v.clone());
            }
            match env(key).filter(|v| !v.is_empty()) {
                Some(v) => Ok(v),
                None => Err(format!("unknown variable ${key} in target {}", self.name)),
            }
        };
        let expand = |s: &str| -> Result<String, String> {
            let b = s.as_bytes();
            let mut out = String::new();
            let mut i = 0;
            while i < b.len() {
                if b[i] != b'$' {
                    let start = i;
                    while i < b.len() && b[i] != b'$' {
                        i += 1;
                    }
                    out.push_str(&s[start..i]);
                    continue;
                }
                match b.get(i + 1) {
                    Some(b'$') => {
                        out.push('$');
                        i += 2;
                    }
                    Some(b'{') => {
                        let end = s[i + 2..].find('}').map_or(s.len(), |e| i + 2 + e);
                        out.push_str(&lookup(&s[i + 2..end])?);
                        i = end + 1;
                    }
                    Some(c) if c.is_ascii_alphabetic() || *c == b'_' || *c >= 0x80 => {
                        let mut j = i + 1;
                        while j < b.len()
                            && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] >= 0x80)
                        {
                            j += 1;
                        }
                        out.push_str(&lookup(&s[i + 1..j])?);
                        i = j;
                    }
                    _ => {
                        out.push('$');
                        i += 1;
                    }
                }
            }
            Ok(out)
        };
        for f in [
            &mut self.description,
            &mut self.file,
            &mut self.parent,
            &mut self.branch,
            &mut self.mod_name,
            &mut self.mod_min_game_version,
            &mut self.mod_description,
        ] {
            *f = expand(f)?;
        }
        for list in [
            &mut self.groups,
            &mut self.flags,
            &mut self.includes,
            &mut self.excludes,
            &mut self.filters,
            &mut self.skip_compile,
        ] {
            for item in list.iter_mut() {
                *item = expand(item)?;
            }
        }
        for (pattern, dest) in &mut self.rules {
            *pattern = expand(pattern)?;
            *dest = expand(dest)?;
        }
        Ok(())
    }

    /// The target's source files: what its `include` globs find under
    /// `root`, less what its `exclude` globs find, without repeats.
    pub fn source_files(&self, root: &Path) -> Result<Vec<PathBuf>, String> {
        let mut excluded = std::collections::HashSet::new();
        for p in &self.excludes {
            excluded.extend(glob::walk(root, p)?);
        }
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for p in &self.includes {
            for f in glob::walk(root, p)? {
                if !excluded.contains(&f) && seen.insert(f.clone()) {
                    out.push(f);
                }
            }
        }
        Ok(out)
    }

    /// The folder (relative to the package root) nasher's rules give a new
    /// file, `$ext` filled in; `None` if no rule matches (nasher then uses
    /// `unknown`). A folder of `/dev/null` means the file is discarded.
    pub fn rule_folder(&self, file_name: &str) -> Option<String> {
        let ext = Path::new(file_name).extension().map(|e| e.to_string_lossy()).unwrap_or_default();
        self.rules
            .iter()
            .find(|(pattern, _)| glob::matches(file_name, pattern))
            .map(|(_, dest)| dest.replace("$ext", &ext))
    }

    /// Whether a packed file is left out of the target's file.
    pub fn filtered(&self, file_name: &str) -> bool {
        self.filters.iter().any(|p| glob::matches(file_name, p))
    }

    /// Whether a script is left uncompiled.
    pub fn skips_compiling(&self, file_name: &str) -> bool {
        self.skip_compile.iter().any(|p| glob::matches(file_name, p))
    }
}

/// A nasher package: its root folder and targets, the default first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub root: PathBuf,
    pub targets: Vec<Target>,
}

impl Package {
    /// The `nasher.cfg` of a package root.
    pub fn config_path(root: &Path) -> PathBuf {
        root.join("nasher.cfg")
    }

    /// Whether `dir` is a nasher package root.
    pub fn is_package(dir: &Path) -> bool {
        Self::config_path(dir).is_file()
    }

    /// Reads the package at `root`, resolving variables from the
    /// environment.
    pub fn read(root: &Path) -> Result<Package, ModuleError> {
        let path = Self::config_path(root);
        let text = std::fs::read_to_string(&path)
            .map_err(|source| ModuleError::Io { path: path.clone(), source })?;
        Self::parse(root, &text, &|k| std::env::var(k).ok()).map_err(|m| bad(&path, m))
    }

    /// Parses `nasher.cfg` text, as nasher does.
    pub fn parse(
        root: &Path,
        text: &str,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Package, String> {
        let mut targets: Vec<Target> = Vec::new();
        let mut defaults = Target::default();
        let mut target = Target::default();
        let (mut context, mut section) = (String::new(), String::new());
        let mut default_name = String::new();
        let mut is_default = false;
        let mut default_index: Option<usize> = None;
        let finish = |targets: &mut Vec<Target>,
                      mut t: Target,
                      defaults: &Target,
                      is_default: bool,
                      default_index: &mut Option<usize>|
         -> Result<(), String> {
            if t.name.is_empty() {
                return Err(format!("target {} has no name", targets.len() + 1));
            }
            let parent = if t.parent.is_empty() {
                defaults.clone()
            } else {
                targets.iter().find(|p| p.name == t.parent).cloned().unwrap_or_default()
            };
            t.inherit(&parent);
            if is_default {
                *default_index = Some(targets.len());
            }
            targets.push(t);
            Ok(())
        };
        for (line, event) in parse_events(text)? {
            match event {
                Event::Section(name) => {
                    let lower = name.to_ascii_lowercase();
                    match lower.as_str() {
                        "package" => {
                            if !section.is_empty() {
                                return Err(format!(
                                    "line {line}: [package] must come first, once"
                                ));
                            }
                            context = "package".into();
                        }
                        "target" => {
                            match context.as_str() {
                                "package" | "" => defaults = std::mem::take(&mut target),
                                _ => {
                                    let t = std::mem::take(&mut target);
                                    finish(
                                        &mut targets,
                                        t,
                                        &defaults,
                                        is_default,
                                        &mut default_index,
                                    )?;
                                }
                            }
                            context = "target".into();
                            is_default = false;
                        }
                        "sources" | "rules" | "variables" => {}
                        "package.sources" | "package.rules" | "package.variables"
                            if context == "target" =>
                        {
                            return Err(format!("line {line}: [{name}] must be within [package]"));
                        }
                        "target.sources" | "target.rules" | "target.variables"
                            if context != "target" =>
                        {
                            return Err(format!("line {line}: [{name}] must be within [target]"));
                        }
                        "package.sources" | "package.rules" | "package.variables"
                        | "target.sources" | "target.rules" | "target.variables" => {}
                        _ => return Err(format!("line {line}: invalid section [{name}]")),
                    }
                    section = lower.rsplit('.').next().unwrap_or_default().to_string();
                }
                Event::Pair(key, value) => match section.as_str() {
                    "package" | "target" => match key.as_str() {
                        "default" if section == "package" => default_name = value,
                        "default" => {
                            is_default = match value.to_ascii_lowercase().as_str() {
                                "true" | "yes" | "on" | "y" | "1" => true,
                                "false" | "no" | "off" | "n" | "0" => false,
                                _ => {
                                    return Err(format!(
                                        "line {line}: target.default must be true or false"
                                    ));
                                }
                            };
                        }
                        "name" if section == "target" => {
                            if value.is_empty() || value == "all" {
                                return Err(format!("line {line}: invalid target name {value:?}"));
                            }
                            if targets.iter().any(|t| t.name == value) {
                                return Err(format!("line {line}: duplicate target {value:?}"));
                            }
                            is_default = is_default || value == default_name;
                            target.name = value;
                        }
                        "name" => {}
                        "description" => target.description = value,
                        "file" => target.file = value,
                        "branch" => target.branch = value,
                        "parent" => {
                            if !targets.iter().any(|t| t.name == value) {
                                return Err(format!("line {line}: unknown parent {value:?}"));
                            }
                            target.parent = value;
                        }
                        "modName" => target.mod_name = value,
                        "modMinGameVersion" => target.mod_min_game_version = value,
                        "modDescription" => target.mod_description = value,
                        "group" => target.groups.push(value),
                        "flags" => target.flags.push(value),
                        "source" | "include" => target.includes.push(value),
                        "exclude" => target.excludes.push(value),
                        "filter" => target.filters.push(value),
                        "skipCompile" => target.skip_compile.push(value),
                        "version" | "url" | "author" => {}
                        // Older packages put unpack rules here.
                        _ => target.rules.push((key, value)),
                    },
                    "sources" => match key.as_str() {
                        "include" => target.includes.push(value),
                        "exclude" => target.excludes.push(value),
                        "filter" => target.filters.push(value),
                        "skipCompile" => target.skip_compile.push(value),
                        _ => {
                            return Err(format!("line {line}: invalid key {key:?} in [{section}]"));
                        }
                    },
                    "rules" => target.rules.push((key, value)),
                    "variables" => target.variables.push((key, value)),
                    _ => {}
                },
            }
        }
        if context == "target" {
            let t = std::mem::take(&mut target);
            finish(&mut targets, t, &defaults, is_default, &mut default_index)?;
        }
        if let Some(i) = default_index {
            let t = targets.remove(i);
            targets.insert(0, t);
        }
        for t in &mut targets {
            t.resolve(env)?;
        }
        Ok(Package { root: root.to_path_buf(), targets })
    }

    /// The target Moonglow edits: the default target if it packs a module,
    /// else the first that does.
    pub fn module_target(&self) -> Option<&Target> {
        self.targets
            .first()
            .filter(|t| t.is_module())
            .or_else(|| self.targets.iter().find(|t| t.is_module()))
    }

    /// A target by name.
    pub fn target(&self, name: &str) -> Option<&Target> {
        self.targets.iter().find(|t| t.name == name)
    }
}

/// nasher's per-user settings that change what a source tree holds: from
/// the package's `.nasher/user.cfg`, else the global one, else nasher's
/// defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// `gffFormat`: `json` (the default) or `nwnt`.
    pub gff_format: String,
    /// The codepage of `gffFlags`' `--nwn-encoding` (windows-1252 without).
    pub codepage: Codepage,
    /// `truncateFloats` (4).
    pub truncate_floats: u8,
    /// `removeUnusedAreas` (true): the module's area list is the areas in
    /// the tree.
    pub remove_unused_areas: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            gff_format: "json".into(),
            codepage: Codepage::WINDOWS_1252,
            truncate_floats: 4,
            remove_unused_areas: true,
        }
    }
}

impl Settings {
    /// nasher's global settings file.
    pub fn global_path() -> Option<PathBuf> {
        if cfg!(windows) {
            std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("nasher/user.cfg"))
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
                .map(|d| d.join("nasher/user.cfg"))
        }
    }

    /// The settings for the package at `root`.
    pub fn read(root: &Path) -> Settings {
        let mut s = Settings::default();
        for file in Self::global_path().into_iter().chain([root.join(".nasher/user.cfg")]) {
            if let Ok(text) = std::fs::read_to_string(&file) {
                s.apply(&text);
            }
        }
        s
    }

    /// Applies one settings file's keys over these.
    pub fn apply(&mut self, text: &str) {
        let Ok(events) = parse_events(text) else { return };
        for (_, e) in events {
            let Event::Pair(key, value) = e else { continue };
            match key.as_str() {
                "gffFormat" => self.gff_format = value,
                "truncateFloats" => {
                    if let Ok(n @ 1..=32) = value.parse() {
                        self.truncate_floats = n;
                    }
                }
                "removeUnusedAreas" => {
                    self.remove_unused_areas = !matches!(
                        value.to_ascii_lowercase().as_str(),
                        "false" | "no" | "off" | "n" | "0"
                    );
                }
                "gffFlags" => {
                    let mut words = value.split_whitespace();
                    while let Some(w) = words.next() {
                        let name = match w.strip_prefix("--nwn-encoding") {
                            Some(rest) if rest.starts_with(['=', ':']) => {
                                Some(rest[1..].to_string())
                            }
                            Some("") => words.next().map(str::to_string),
                            _ => None,
                        };
                        if let Some(cp) = name.and_then(|n| Codepage::for_label(&n)) {
                            self.codepage = cp;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Package {
        Package::parse(Path::new("/p"), text, &|k| (k == "HOME_UTILS").then(|| "/u".into()))
            .unwrap()
    }

    #[test]
    fn nasher_init_default_package() {
        let p = parse(
            "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n\n  \
             [package.rules]\n  \"*\" = \"src\"\n\n[target]\nname = \"default\"\nfile = \"demo.mod\"\ndescription = \"\"\n",
        );
        assert_eq!(p.targets.len(), 1);
        let t = &p.targets[0];
        assert_eq!(t.name, "default");
        assert_eq!(t.file, "demo.mod");
        assert_eq!(t.includes, ["src/**/*.{nss,json}"]);
        assert_eq!(t.rules, [("*".to_string(), "src".to_string())]);
        assert_eq!(p.module_target().unwrap().name, "default");
    }

    /// The larger example of nasher's README.
    #[test]
    fn targets_inherit_and_resolve_variables() {
        let p = parse(
            r#"
[package]
name = "Core Framework"
version = "0.1.0"
author = "Squatting Monk <squattingmonk@gmail.com>"
file = "$target.hak"

  [package.variables]
  sm-utils = "../sm-utils/src"

  [package.sources]
  include = "${sm-utils}/*.nss" # This variable is expanded
  include = "src/**/*.{nss,json}"
  exclude = "**/test_*.nss"
  skipCompile = "util_i_library.nss"

  [package.rules]
  "hook_*.nss" = "src/Hooks"
  "core_*" = "src/Framework"
  "*" = "src/$ext"

[target]
name = "demo"
description = "A demo module"
file = "core_framework.mod"
modName = "Core Framework Demo Module"

[target]
name = "framework"
file = "core_framework.erf"

  [target.sources]
  exclude = "src/demo/**"

[target]
name = "scripts"
group = "haks"
parent = "framework"

  [target.sources]
  include = "src/**/*.nss"
  filter = "*.nss"

[target]
name = "blueprints"
group = "haks"
default = true
; a comment
  [target.sources]
  include = "${HOME_UTILS}/bp/*.json"
"#,
        );
        let names: Vec<_> = p.targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["blueprints", "demo", "framework", "scripts"]);
        let demo = p.target("demo").unwrap();
        assert_eq!(demo.includes, ["../sm-utils/src/*.nss", "src/**/*.{nss,json}"]);
        assert_eq!(demo.excludes, ["**/test_*.nss"]);
        assert_eq!(demo.mod_name, "Core Framework Demo Module");
        assert!(demo.skips_compiling("util_i_library.nss"));
        assert_eq!(demo.rule_folder("hook_x.nss").as_deref(), Some("src/Hooks"));
        assert_eq!(demo.rule_folder("area.git").as_deref(), Some("src/git"));
        let framework = p.target("framework").unwrap();
        assert_eq!(framework.excludes, ["src/demo/**"]);
        let scripts = p.target("scripts").unwrap();
        // From its parent, not the package.
        assert_eq!(scripts.file, "core_framework.erf");
        assert_eq!(scripts.excludes, ["src/demo/**"]);
        assert!(scripts.filtered("x.nss") && !scripts.filtered("x.ncs"));
        let bp = p.target("blueprints").unwrap();
        assert_eq!(bp.file, "blueprints.hak");
        assert_eq!(bp.includes, ["/u/bp/*.json"]);
        // The default target packs a hak, so the module target is the first .mod.
        assert_eq!(p.module_target().unwrap().name, "demo");
    }

    #[test]
    fn bad_packages_are_refused() {
        let env = |_: &str| None;
        let root = Path::new("/p");
        for text in [
            "[target]\nname = \"a\"\n[package]\n",
            "[target]\nfile = \"a.mod\"\n",
            "[target]\nname = \"a\"\n[target]\nname = \"a\"\n",
            "[target]\nname = \"a\"\nparent = \"b\"\n",
            "[target]\nname = \"a\"\ninclude = \"$nope/*\"\n",
            "[wat]\n",
            "[target]\nname = \"a\n",
        ] {
            assert!(Package::parse(root, text, &env).is_err(), "{text}");
        }
    }

    #[test]
    fn strings_and_symbols_read_as_parsecfg_reads_them() {
        let events = parse_events(
            "a = plain value  # comment\nb: \"q\\x41\\t\\\"\"\nc = r\"C:\\dir\"\nd = \"\"\"\nlong\n\"\"\"\n--e:f\nbare\n",
        )
        .unwrap();
        let pairs: Vec<(String, String)> = events
            .into_iter()
            .filter_map(|(_, e)| match e {
                Event::Pair(k, v) => Some((k, v)),
                _ => None,
            })
            .collect();
        let want = [
            ("a", "plain value"),
            ("b", "qA\t\""),
            ("c", "C:\\dir"),
            ("d", "long\n"),
            ("e", "f"),
            ("bare", ""),
        ];
        assert_eq!(pairs, want.map(|(k, v)| (k.to_string(), v.to_string())));
    }

    #[test]
    fn settings_from_user_cfg() {
        let mut s = Settings::default();
        s.apply("truncateFloats = \"6\"\nremoveUnusedAreas = false\ngffFlags = \"--nwn-encoding windows-1250\"\n");
        assert_eq!(s.truncate_floats, 6);
        assert!(!s.remove_unused_areas);
        assert_eq!(s.codepage, Codepage::WINDOWS_1250);
        assert_eq!(s.gff_format, "json");
    }
}
