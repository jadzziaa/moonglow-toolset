//! The project file: a team's rules for Moonglow, kept with the module
//! in version control (`moonglow.toml`, beside `nasher.cfg` in a nasher
//! project, at the root of a module folder), and a builder's own part
//! beside it (`.moonglow/user.toml`, which no one else shares).
//!
//! ```toml
//! [make_static]                       # Make Placeables Static leaves these dynamic:
//! keep_variables = ["tdod_*"]         # a placeable with a variable so named
//! keep_tags = ["LAMP_*"]              # a placeable whose tag matches
//! keep_resrefs = ["tdod_*"]           # one made from a blueprint so named
//!
//! [tags]
//! unique = ["TDOD_START_*"]           # on one object only (Verify says where not)
//! paste_warning = "custom"            # builders' tags and unique ones; "any", "unique", "off"
//!
//! [store]                             # Setup Store
//! open_script = "tdod_vendor_open"    # the reply that opens the store runs this
//! tag = "{area}_STORE_{n}"            # the store's tag
//! blueprint = "tdod_vendor"           # the store blueprint offered first
//! keeper_variable = "tdod_store"      # the shopkeeper's String variable: the store's tag
//!
//! [resrefs]
//! pattern = "{code}_{name}"           # a new ResRef, by the builder's code
//! ```
//!
//! And in `.moonglow/user.toml`, the builder's own:
//!
//! ```toml
//! [builder]
//! code = "ab"                         # new ResRefs begin ab_ (by the pattern)
//! ```
//!
//! A key the file doesn't know, or a value of the wrong kind, is a
//! warning with its file and line, and the rest is read; a file that is
//! no TOML is an error, and Moonglow works as without one. Without the
//! file, everything is as Moonglow does it for any module.
//!
//! The builder's own part is a file and not Moonglow's settings so that
//! the command line and a team's installer read and write it as the
//! window does, and it moves with the checkout. Its folder is kept out of
//! version control by itself (Moonglow writes a `.gitignore` of `*` into
//! it), so that no builder's code is committed for everyone, as a
//! `moonglow.user.toml` beside the team's file would be by a `git add .`.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use mg_core::ResRef;
use regex_lite::Regex;
use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::ModuleLocation;

/// The team's file, in the project's root.
pub const FILE_NAME: &str = "moonglow.toml";
/// The builder's own folder, in the project's root, and its file.
pub const USER_DIR: &str = ".moonglow";
pub const USER_FILE: &str = "user.toml";

/// Names matched by globs (`TDOD_START_*`, `lamp_[0-9]*`, `{a,b}_*`),
/// whatever their case.
#[derive(Debug, Clone, Default)]
pub struct Patterns {
    pub globs: Vec<String>,
    regexes: Vec<Regex>,
}

impl PartialEq for Patterns {
    fn eq(&self, other: &Self) -> bool {
        self.globs == other.globs
    }
}

impl Patterns {
    /// The globs, each as nasher reads one; a glob that can't be read is
    /// said, and left out.
    pub fn new(globs: &[&str]) -> Result<Patterns, String> {
        let mut p = Patterns::default();
        for g in globs {
            let rx = crate::nasher::glob::to_regex(g, true)?;
            p.regexes.push(Regex::new(&rx).map_err(|e| format!("{g}: {e}"))?);
            p.globs.push(g.to_string());
        }
        Ok(p)
    }

    pub fn is_empty(&self) -> bool {
        self.globs.is_empty()
    }

    /// The glob a name matches first.
    pub fn matching(&self, name: &str) -> Option<&str> {
        self.regexes.iter().position(|r| r.is_match(name)).map(|i| self.globs[i].as_str())
    }

    pub fn matches(&self, name: &str) -> bool {
        self.matching(name).is_some()
    }
}

/// What Make Placeables Static leaves dynamic beyond what it always does
/// (`[make_static]`).
#[derive(Debug, Clone, PartialEq)]
pub struct StaticRules {
    /// A placeable with a variable of a name these match: any variable
    /// (`*`) unless the file says which (none: `[]`).
    pub keep_variables: Patterns,
    /// A placeable whose tag these match.
    pub keep_tags: Patterns,
    /// A placeable whose blueprint's ResRef (`TemplateResRef`) these match.
    pub keep_resrefs: Patterns,
}

impl Default for StaticRules {
    fn default() -> Self {
        StaticRules {
            keep_variables: Patterns::new(&["*"]).expect("a glob"),
            keep_tags: Patterns::default(),
            keep_resrefs: Patterns::default(),
        }
    }
}

/// Which duplicate tags a paste or a copy says it made (never a static
/// placeable's: scenery).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PasteWarning {
    /// Tags a builder set (an object's tag that isn't its blueprint's
    /// own, or one whose blueprint isn't found), and those `[tags]
    /// unique` names: copies of a rock tagged as its blueprint are quiet.
    #[default]
    Custom,
    /// Every tag the paste puts on a second object.
    Any,
    /// Only those `[tags] unique` names.
    Unique,
    Off,
}

/// `[tags]`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TagRules {
    /// Tags that must be on one object of the module only.
    pub unique: Patterns,
    pub paste_warning: PasteWarning,
}

/// Setup Store's settings (`[store]`); each left out is as Aurora does it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StoreRules {
    /// The script the reply that opens the store runs, in place of the
    /// `openstoreNNN` the wizard writes.
    pub open_script: Option<ResRef>,
    /// The store's tag: `{area}` is the area's tag, `{n}` a number that
    /// makes it the only one in the module (one is added at the end where
    /// the pattern has none and the tag is taken).
    pub tag: Option<String>,
    /// The store blueprint chosen at first.
    pub blueprint: Option<ResRef>,
    /// A String variable set on the shopkeeper to the store's tag.
    pub keeper_variable: Option<String>,
}

/// The builder's own settings (`.moonglow/user.toml`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserSettings {
    /// `[builder] code`: the builder's code, which the ResRefs Moonglow
    /// proposes for new resources start with (by `[resrefs] pattern`),
    /// in lower case.
    pub builder_code: Option<String>,
}

/// The pattern a new ResRef is made by when the builder has a code
/// (`[resrefs] pattern`): the code, then `_`, then the name.
pub const DEFAULT_PATTERN: &str = "{code}_{name}";

/// The longest builder code: a ResRef is 16 characters, and the name
/// keeps most of them.
pub const MAX_CODE: usize = 8;

/// The fewest characters a pattern leaves the name (and its number).
const MIN_ROOM: usize = 6;

/// How the ResRefs Moonglow proposes for new resources are made: the
/// builder's code put into the team's pattern, before and after the name
/// (none: as Aurora makes them).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Naming {
    before: String,
    after: String,
}

impl Naming {
    /// The naming of a code and a pattern (`{code}` and `{name}` in it;
    /// [`pattern_problem`] says whether it is one).
    pub fn new(code: &str, pattern: &str) -> Naming {
        let filled = pattern.replace("{code}", code).to_ascii_lowercase();
        let (before, after) = filled.split_once("{name}").unwrap_or((filled.as_str(), ""));
        Naming { before: before.to_string(), after: after.to_string() }
    }

    /// Whether ResRefs are made as Aurora makes them (no code).
    pub fn is_plain(&self) -> bool {
        self.before.is_empty() && self.after.is_empty()
    }

    /// What goes before the name (`ab_`).
    pub fn before(&self) -> &str {
        &self.before
    }

    /// What goes after it (with a pattern `{name}_{code}`).
    pub fn after(&self) -> &str {
        &self.after
    }

    /// How many characters the name and its number have.
    pub fn room(&self) -> usize {
        16usize.saturating_sub(self.before.len() + self.after.len())
    }

    /// The name without the builder's own code where it begins (or ends)
    /// with it, so that a copy of `ab_sword` is `ab_sword001`, never
    /// `ab_ab_sword001`; another builder's code is kept.
    pub fn strip<'a>(&self, name: &'a str) -> &'a str {
        let mut s = name;
        let b = self.before.len();
        if b > 0
            && s.len() > b
            && s.is_char_boundary(b)
            && s[..b].eq_ignore_ascii_case(&self.before)
        {
            s = &s[b..];
        }
        let a = self.after.len();
        if a > 0
            && s.len() > a
            && s.is_char_boundary(s.len() - a)
            && s[s.len() - a..].eq_ignore_ascii_case(&self.after)
        {
            s = &s[..s.len() - a];
        }
        s
    }

    /// A ResRef's text: the name (without the builder's code it has, cut
    /// so that its number and the code fit in 16 characters) and its
    /// number, in the pattern. Plain, the name cut to leave room for the
    /// number, as Aurora's wizards do.
    pub fn fit(&self, name: &str, number: &str) -> String {
        let name = self.strip(name);
        let room = self.room().saturating_sub(number.len());
        let cut = name.char_indices().nth(room).map_or(name.len(), |(i, _)| i);
        let mut s = format!("{}{}{number}{}", self.before, &name[..cut], self.after);
        s.make_ascii_lowercase();
        while s.len() > 16 {
            s.pop();
        }
        s
    }

    /// The ResRef [`Naming::fit`] makes, or none where that is no ResRef.
    pub fn resref(&self, name: &str, number: &str) -> Option<ResRef> {
        ResRef::from_str(&self.fit(name, number)).ok().filter(|r| !r.is_empty())
    }
}

/// What is wrong with a builder code, or none where it is one: letters,
/// digits and `_`, at most [`MAX_CODE`] of them.
pub fn code_problem(code: &str) -> Option<String> {
    if code.is_empty() {
        return Some("empty".into());
    }
    if let Some(c) = code.chars().find(|c| !(c.is_ascii_alphanumeric() || *c == '_')) {
        return Some(format!("\"{c}\": a code is letters, digits and _"));
    }
    if code.len() > MAX_CODE {
        return Some(format!("{} characters: a code has at most {MAX_CODE}", code.len()));
    }
    None
}

/// What is wrong with a ResRef pattern, or none where it is one:
/// `{code}` and `{name}` once each, the rest letters, digits and `_`,
/// leaving a name room by a code of [`MAX_CODE`].
pub fn pattern_problem(pattern: &str) -> Option<String> {
    let mut rest = pattern;
    let (mut fixed, mut code, mut name) = (0, 0, 0);
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("{code}") {
            code += 1;
            rest = after;
        } else if let Some(after) = rest.strip_prefix("{name}") {
            name += 1;
            rest = after;
        } else if c == '{' {
            let word = rest.split('}').next().unwrap_or(rest);
            return Some(format!("{word}}} is neither {{code}} nor {{name}}"));
        } else if !(c.is_ascii_alphanumeric() || c == '_') {
            return Some(format!("\"{c}\": a ResRef is letters, digits and _"));
        } else {
            fixed += 1;
            rest = &rest[c.len_utf8()..];
        }
    }
    if code != 1 || name != 1 {
        return Some("it is to have {code} and {name}, once each".into());
    }
    if 16 < fixed + MAX_CODE + MIN_ROOM {
        return Some(format!(
            "it leaves a name fewer than {MIN_ROOM} of a ResRef's 16 characters (by a code of \
             {MAX_CODE})"
        ));
    }
    None
}

/// A project's rules, as its files say them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProjectFile {
    /// The team's file, where it was looked for (there or not).
    pub path: Option<PathBuf>,
    /// Whether the team's file is there and was read.
    pub found: bool,
    pub make_static: StaticRules,
    pub tags: TagRules,
    pub store: Option<StoreRules>,
    /// `[resrefs] pattern`: how a new ResRef is made by the builder's
    /// code (none: [`DEFAULT_PATTERN`]).
    pub resref_pattern: Option<String>,
    pub user: UserSettings,
    /// The team's described variables (`variables.toml` beside the
    /// team's file): none without it.
    pub variables: crate::var_desc::Variables,
    /// Whether `variables.toml` is there and was read.
    pub variables_found: bool,
    /// `[plugins]` and the builder's `[project_plugins]`: the project's
    /// plugins, their settings' values and what the builder allowed
    /// ([`crate::project_plugins`]).
    pub plugins: crate::project_plugins::PluginRules,
    /// What was not understood, which leaves the rest as it reads:
    /// `moonglow.toml:12: "colour" is not a key of [store]`.
    pub warnings: Vec<String>,
    /// A file that could not be read at all (its rules are then left out).
    pub errors: Vec<String>,
    /// The files' times and sizes when read, to see a change.
    stamps: Vec<Option<(SystemTime, u64)>>,
}

fn stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// The line (from 1) a byte of `source` is on.
pub(crate) fn line_of(source: &str, at: usize) -> usize {
    source.as_bytes()[..at.min(source.len())].iter().filter(|b| **b == b'\n').count() + 1
}

/// What reads one file: its name, as messages give it, and its text.
pub(crate) struct Reader<'a> {
    pub(crate) name: &'a str,
    pub(crate) source: &'a str,
    pub(crate) warnings: Vec<String>,
}

pub(crate) type Entry<'i> = (&'i Spanned<Cow<'i, str>>, &'i Spanned<DeValue<'i>>);

/// A table's entries in the order the file has them.
pub(crate) fn in_order<'i>(t: &'i DeTable<'i>) -> Vec<Entry<'i>> {
    let mut entries: Vec<Entry<'i>> = t.iter().collect();
    entries.sort_by_key(|(k, _)| k.span().start);
    entries
}

impl<'a> Reader<'a> {
    pub(crate) fn warn(&mut self, at: usize, message: impl std::fmt::Display) {
        self.warnings.push(format!("{}:{}: {message}", self.name, line_of(self.source, at)));
    }

    /// A section's entries (`[name]`), or none where it is not a table.
    pub(crate) fn section<'i>(&mut self, (key, value): Entry<'i>) -> Vec<Entry<'i>> {
        match value.get_ref() {
            DeValue::Table(t) => in_order(t),
            _ => {
                self.warn(
                    key.span().start,
                    format!("\"{}\" is not a section ([…])", key.get_ref()),
                );
                Vec::new()
            }
        }
    }

    pub(crate) fn unknown(&mut self, section: &str, (key, _): Entry) {
        let message = format!("\"{}\" is not a key of [{section}]", key.get_ref());
        self.warn(key.span().start, message);
    }

    pub(crate) fn text(&mut self, (key, value): Entry) -> Option<String> {
        match value.get_ref() {
            DeValue::String(s) => Some(s.to_string()),
            _ => {
                let message = format!("{} is to be text in quotes", key.get_ref());
                self.warn(value.span().start, message);
                None
            }
        }
    }

    fn resref(&mut self, entry: Entry) -> Option<ResRef> {
        let text = self.text(entry)?;
        match ResRef::from_str(text.trim()) {
            Ok(r) if !r.is_empty() => Some(r),
            Ok(_) => {
                self.warn(entry.1.span().start, format!("{} is empty", entry.0.get_ref()));
                None
            }
            Err(e) => {
                self.warn(entry.1.span().start, format!("{}: {text}: {e}", entry.0.get_ref()));
                None
            }
        }
    }

    fn patterns(&mut self, (key, value): Entry) -> Option<Patterns> {
        let DeValue::Array(items) = value.get_ref() else {
            let message = format!("{} is to be a list of names: [\"A_*\", \"B\"]", key.get_ref());
            self.warn(value.span().start, message);
            return None;
        };
        let mut globs = Vec::new();
        for item in items.iter() {
            match item.get_ref() {
                DeValue::String(s) => match Patterns::new(&[s]) {
                    Ok(_) => globs.push(s.to_string()),
                    Err(e) => self.warn(item.span().start, e),
                },
                _ => self.warn(item.span().start, format!("{}: not text", key.get_ref())),
            }
        }
        let globs: Vec<&str> = globs.iter().map(String::as_str).collect();
        Some(Patterns::new(&globs).expect("each read"))
    }
}

impl ProjectFile {
    /// Where a module's team file is: in a nasher project's root or a
    /// module folder; none for a module kept in one file (`.mod`), whose
    /// folder holds other modules too.
    pub fn root_of(location: &ModuleLocation) -> Option<&Path> {
        match location {
            ModuleLocation::Project { root, .. } => Some(root),
            ModuleLocation::Folder(dir) => Some(dir),
            ModuleLocation::Archive(_) => None,
        }
    }

    /// The builder's own file, for a project's root.
    pub fn user_path(root: &Path) -> PathBuf {
        root.join(USER_DIR).join(USER_FILE)
    }

    /// Reads a project's files from its root (neither there: as without
    /// a project file).
    pub fn read(root: &Path) -> ProjectFile {
        let path = root.join(FILE_NAME);
        let user = Self::user_path(root);
        let described = crate::var_desc::path_in(root);
        let mut p = ProjectFile {
            path: Some(path.clone()),
            stamps: vec![stamp(&path), stamp(&user), stamp(&described)],
            ..Default::default()
        };
        for (file, own) in [(&path, false), (&user, true)] {
            let source = match std::fs::read(file) {
                Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    p.errors.push(format!("{}: {e}", file.display()));
                    continue;
                }
            };
            let name = if own { format!("{USER_DIR}/{USER_FILE}") } else { FILE_NAME.to_string() };
            // (One made by hand: its folder would be committed for all.)
            if own && !root.join(USER_DIR).join(".gitignore").exists() {
                p.warnings.push(format!(
                    "{name}: {USER_DIR} has no .gitignore; put one there holding * so that \
                     your own settings stay out of version control"
                ));
            }
            p.found |= !own;
            p.apply(&name, &source, own);
        }
        // The team's described variables, a file of their own.
        match std::fs::read(&described) {
            Ok(bytes) => {
                let source = String::from_utf8_lossy(&bytes);
                p.read_variables(crate::var_desc::FILE_NAME, &source);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => p.errors.push(format!("{}: {e}", described.display())),
        }
        p
    }

    /// Reads the described variables' file's text (`variables.toml`):
    /// what it doesn't understand is a warning, a file that is no TOML an
    /// error (and none are described).
    pub fn read_variables(&mut self, name: &str, source: &str) {
        let mut v = crate::var_desc::Variables::default();
        match v.apply(name, source, &mut self.warnings) {
            Ok(()) => {
                self.variables = v;
                self.variables_found = true;
            }
            Err(e) => self.errors.push(e),
        }
    }

    /// Reads one file's text into these rules (`own`: the builder's file).
    pub fn apply(&mut self, name: &str, source: &str, own: bool) {
        let table = match DeTable::parse(source) {
            Ok(t) => t,
            Err(e) => {
                let line = e.span().map_or(1, |s| line_of(source, s.start));
                self.errors.push(format!("{name}:{line}: {}", e.message().trim()));
                return;
            }
        };
        let mut r = Reader { name, source, warnings: Vec::new() };
        for entry in in_order(table.get_ref()) {
            let (key, _) = entry;
            let section = key.get_ref().as_ref();
            match (section, own) {
                ("make_static", false) => {
                    for e in r.section(entry) {
                        match e.0.get_ref().as_ref() {
                            "keep_variables" => {
                                if let Some(p) = r.patterns(e) {
                                    self.make_static.keep_variables = p;
                                }
                            }
                            "keep_tags" => {
                                if let Some(p) = r.patterns(e) {
                                    self.make_static.keep_tags = p;
                                }
                            }
                            "keep_resrefs" => {
                                if let Some(p) = r.patterns(e) {
                                    self.make_static.keep_resrefs = p;
                                }
                            }
                            _ => r.unknown(section, e),
                        }
                    }
                }
                ("tags", false) => {
                    for e in r.section(entry) {
                        match e.0.get_ref().as_ref() {
                            "unique" => {
                                if let Some(p) = r.patterns(e) {
                                    self.tags.unique = p;
                                }
                            }
                            "paste_warning" => match r.text(e).as_deref() {
                                Some("custom") => self.tags.paste_warning = PasteWarning::Custom,
                                Some("any") => self.tags.paste_warning = PasteWarning::Any,
                                Some("unique") => self.tags.paste_warning = PasteWarning::Unique,
                                Some("off") => self.tags.paste_warning = PasteWarning::Off,
                                Some(other) => r.warn(
                                    e.1.span().start,
                                    format!(
                                        "paste_warning \"{other}\": \"custom\", \"any\", \"unique\" or \
                                         \"off\""
                                    ),
                                ),
                                None => {}
                            },
                            _ => r.unknown(section, e),
                        }
                    }
                }
                ("resrefs", false) => {
                    for e in r.section(entry) {
                        match e.0.get_ref().as_ref() {
                            "pattern" => {
                                if let Some(t) = r.text(e) {
                                    match pattern_problem(t.trim()) {
                                        Some(problem) => r.warn(
                                            e.1.span().start,
                                            format!("pattern \"{t}\": {problem}"),
                                        ),
                                        None => self.resref_pattern = Some(t.trim().to_string()),
                                    }
                                }
                            }
                            _ => r.unknown(section, e),
                        }
                    }
                }
                ("store", false) => {
                    let mut store = StoreRules::default();
                    for e in r.section(entry) {
                        match e.0.get_ref().as_ref() {
                            "open_script" => store.open_script = r.resref(e),
                            "blueprint" => store.blueprint = r.resref(e),
                            "tag" => {
                                if let Some(t) = r.text(e) {
                                    if let Err(problem) = tag_pattern_problem(&t) {
                                        r.warn(e.1.span().start, format!("tag \"{t}\": {problem}"));
                                    } else {
                                        store.tag = Some(t);
                                    }
                                }
                            }
                            "keeper_variable" => {
                                store.keeper_variable = r.text(e).filter(|v| !v.trim().is_empty());
                            }
                            _ => r.unknown(section, e),
                        }
                    }
                    self.store = Some(store);
                }
                ("builder", true) => {
                    for e in r.section(entry) {
                        match e.0.get_ref().as_ref() {
                            "code" => {
                                let code = r.text(e).map(|c| c.trim().to_ascii_lowercase());
                                match code.as_deref().map(|c| (c, code_problem(c))) {
                                    Some(("", _)) | None => {}
                                    Some((c, Some(problem))) => r.warn(
                                        e.1.span().start,
                                        format!(
                                            "code \"{c}\": {problem}; new ResRefs are made \
                                             without it"
                                        ),
                                    ),
                                    Some((c, None)) => self.user.builder_code = Some(c.to_string()),
                                }
                            }
                            _ => r.unknown(section, e),
                        }
                    }
                }
                ("builder", false) => r.warn(
                    key.span().start,
                    format!(
                        "[builder] is each builder's own: it goes in {USER_DIR}/{USER_FILE}, \
                         not in the file the team shares"
                    ),
                ),
                ("plugins", _) => crate::project_plugins::read_plugins(self, &mut r, entry, own),
                (crate::project_plugins::DECIDED, _) => {
                    crate::project_plugins::read_decided(self, &mut r, entry, own)
                }
                ("make_static" | "tags" | "store" | "resrefs", true) => r.warn(
                    key.span().start,
                    format!("[{section}] is the team's: it goes in {FILE_NAME}"),
                ),
                _ => {
                    r.warn(key.span().start, format!("[{section}] is not a section Moonglow reads"))
                }
            }
        }
        self.warnings.append(&mut r.warnings);
    }

    /// How the ResRefs proposed for new resources are made: by the
    /// builder's code, where they have one, in the team's pattern.
    pub fn naming(&self) -> Naming {
        let pattern = self.resref_pattern.as_deref().unwrap_or(DEFAULT_PATTERN);
        self.user.builder_code.as_deref().map_or_else(Naming::default, |c| Naming::new(c, pattern))
    }

    /// Whether the files changed (were made, edited or deleted) since they
    /// were read.
    pub fn changed(&self) -> bool {
        let Some(path) = &self.path else { return false };
        let root = path.parent().unwrap_or(Path::new("."));
        let now = vec![
            stamp(path),
            stamp(&Self::user_path(root)),
            stamp(&crate::var_desc::path_in(root)),
        ];
        now != self.stamps
    }

    /// Writes the builder's own file (making its folder, which keeps
    /// itself out of version control).
    pub fn write_user(root: &Path, text: &str) -> std::io::Result<()> {
        let dir = root.join(USER_DIR);
        std::fs::create_dir_all(&dir)?;
        let ignore = dir.join(".gitignore");
        if !ignore.exists() {
            std::fs::write(
                &ignore,
                "# Each builder's own settings for Moonglow: not shared.\n*\n",
            )?;
        }
        std::fs::write(dir.join(USER_FILE), text)
    }

    /// Sets (or, `None`, takes out) the builder's code in their own file,
    /// keeping whatever else it holds as it is written.
    pub fn set_builder_code(root: &Path, code: Option<&str>) -> std::io::Result<()> {
        let invalid = |e: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, e);
        if let Some(problem) = code.and_then(code_problem) {
            return Err(invalid(format!("the code \"{}\": {problem}", code.unwrap_or(""))));
        }
        let path = Self::user_path(root);
        let text = match std::fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        };
        let code = code.map(str::to_ascii_lowercase);
        let text = with_code(&text, code.as_deref()).map_err(invalid)?;
        Self::write_user(root, &text)
    }
}

/// A builder's file with its `[builder] code` set to `code` (or taken
/// out), the rest kept as written; an error where the text is no TOML.
fn with_code(text: &str, code: Option<&str>) -> Result<String, String> {
    let name = format!("{USER_DIR}/{USER_FILE}");
    let table = DeTable::parse(text).map_err(|e| {
        let line = e.span().map_or(1, |s| line_of(text, s.start));
        format!("{name}:{line}: {} (mend it first)", e.message().trim())
    })?;
    let line_start = |at: usize| text[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = |at: usize| text[at..].find('\n').map_or(text.len(), |i| at + i + 1);
    let mut out = text.to_string();
    let entries = in_order(table.get_ref());
    let builder = entries.iter().find(|(k, _)| k.get_ref() == "builder");
    match builder.map(|(k, v)| (k, v.get_ref())) {
        Some((header, DeValue::Table(t))) => {
            let entry = in_order(t).into_iter().find(|(k, _)| k.get_ref() == "code");
            match (entry, code) {
                (Some((_, v)), Some(code)) => out.replace_range(v.span(), &format!("\"{code}\"")),
                (Some((k, v)), None) => {
                    out.replace_range(line_start(k.span().start)..line_end(v.span().end), "")
                }
                (None, Some(code)) => {
                    // (The line after its header.)
                    let mut at = line_end(header.span().start);
                    if at == out.len() && !out.ends_with('\n') {
                        out.push('\n');
                        at = out.len();
                    }
                    out.insert_str(at, &format!("code = \"{code}\"\n"));
                }
                (None, None) => {}
            }
        }
        Some(_) => return Err(format!("{name}: \"builder\" is not a section (mend it first)")),
        None => {
            if let Some(code) = code {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!("[builder]\ncode = \"{code}\"\n"));
            }
        }
    }
    Ok(out)
}

/// What is wrong with a store tag's pattern: a `{…}` that is neither
/// `{area}` nor `{n}`.
fn tag_pattern_problem(pattern: &str) -> Result<(), String> {
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            return Err("a { without its }".into());
        };
        let name = &rest[open + 1..open + close];
        if name != "area" && name != "n" {
            return Err(format!("{{{name}}} is neither {{area}} nor {{n}}"));
        }
        rest = &rest[open + close + 1..];
    }
    if pattern.trim().is_empty() {
        return Err("empty".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
[make_static]
keep_variables = ["tdod_*"]
keep_tags = ["LAMP_*", "TORCH"]
keep_resrefs = ["tdod_*"]

[tags]
unique = ["TDOD_START_*", "*_STORE_*"]
paste_warning = "unique"

[store]
open_script = "tdod_vendor_open"
tag = "{area}_STORE_{n}"
blueprint = "tdod_vendor"
keeper_variable = "tdod_store"
"#;

    fn read(source: &str) -> ProjectFile {
        let mut p = ProjectFile::default();
        p.apply(FILE_NAME, source, false);
        p
    }

    #[test]
    fn the_example_reads_whole() {
        let p = read(EXAMPLE);
        assert_eq!(p.warnings, Vec::<String>::new());
        assert_eq!(p.errors, Vec::<String>::new());
        assert!(p.make_static.keep_variables.matches("TDOD_LORE"), "whatever its case");
        assert!(!p.make_static.keep_variables.matches("x_tdod"));
        assert_eq!(p.make_static.keep_tags.matching("LAMP_3"), Some("LAMP_*"));
        assert!(
            p.make_static.keep_tags.matches("torch") && !p.make_static.keep_tags.matches("TORCH2")
        );
        assert!(p.tags.unique.matches("KREZK_STORE_1"));
        assert_eq!(p.tags.paste_warning, PasteWarning::Unique);
        let store = p.store.unwrap();
        assert_eq!(store.open_script, ResRef::from_str("tdod_vendor_open").ok());
        assert_eq!(store.tag.as_deref(), Some("{area}_STORE_{n}"));
        assert_eq!(store.blueprint, ResRef::from_str("tdod_vendor").ok());
        assert_eq!(store.keeper_variable.as_deref(), Some("tdod_store"));
    }

    /// The manual's example (Modules, the project file) reads whole.
    #[test]
    fn the_manual_s_example_reads_whole() {
        let manual = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/manual/03-modules.md");
        let text = std::fs::read_to_string(manual).unwrap();
        let section = &text[text.find("## The project file").expect("the section")..];
        let start = section.find("```toml\n").expect("its example") + 8;
        let example = &section[start..start + section[start..].find("```").unwrap()];
        let p = read(example);
        assert_eq!((p.warnings.len(), p.errors.len()), (0, 0), "{:?} {:?}", p.warnings, p.errors);
        assert!(p.store.is_some() && !p.tags.unique.is_empty());
    }

    #[test]
    fn without_a_file_everything_is_as_for_any_module() {
        let p = read("");
        assert_eq!(p, ProjectFile::default());
        assert!(p.make_static.keep_variables.matches("anything"), "a variable keeps it dynamic");
        assert!(p.make_static.keep_tags.is_empty() && p.tags.unique.is_empty());
        assert_eq!(p.tags.paste_warning, PasteWarning::Custom);
        assert!(p.store.is_none());
        // And `[]`: variables don't keep a placeable dynamic.
        let p = read("[make_static]\nkeep_variables = []\n");
        assert!(!p.make_static.keep_variables.matches("tdod_x"));
    }

    #[test]
    fn what_is_not_understood_is_said_by_line_and_the_rest_read() {
        let p = read(
            "[store]\nopen_script = \"tdod_vendor_open\"\ncolour = 3\n\n[tags]\n\
             unique = \"TDOD_*\"\npaste_warning = \"loud\"\n[lights]\nx = 1\n\
             [make_static]\nkeep_tags = [\"A_*\", 4, \"[\"]\n[store2]\n[builder]\ncode = \"x\"\n",
        );
        assert_eq!(
            p.warnings,
            [
                "moonglow.toml:3: \"colour\" is not a key of [store]",
                "moonglow.toml:6: unique is to be a list of names: [\"A_*\", \"B\"]",
                "moonglow.toml:7: paste_warning \"loud\": \"custom\", \"any\", \"unique\" or \"off\"",
                "moonglow.toml:8: [lights] is not a section Moonglow reads",
                "moonglow.toml:11: keep_tags: not text",
                "moonglow.toml:11: [: missing ']'",
                "moonglow.toml:12: [store2] is not a section Moonglow reads",
                "moonglow.toml:13: [builder] is each builder's own: it goes in .moonglow/user.toml, \
                 not in the file the team shares",
            ]
        );
        assert_eq!(p.store.unwrap().open_script, ResRef::from_str("tdod_vendor_open").ok());
        assert_eq!(p.make_static.keep_tags.globs, ["A_*"]);
        assert_eq!(p.user.builder_code, None, "not from the team's file");
        let p = read("[store]\nopen_script = \"much_too_long_a_name\"\ntag = \"{area}_{x}\"\n");
        assert_eq!(p.warnings.len(), 2, "{:?}", p.warnings);
        assert!(p.warnings[0].starts_with("moonglow.toml:2: open_script: much_too_long_a_name"));
        assert_eq!(
            p.warnings[1],
            "moonglow.toml:3: tag \"{area}_{x}\": {x} is neither {area} nor {n}"
        );
    }

    #[test]
    fn a_builder_s_code_goes_into_the_team_s_pattern() {
        let plain = Naming::default();
        assert!(plain.is_plain());
        assert_eq!(plain.fit("wizardsword", "001"), "wizardsword001");
        assert_eq!(plain.fit("averyveryverylongname", "001"), "averyveryvery001");
        let ab = Naming::new("ab", DEFAULT_PATTERN);
        assert_eq!((ab.before(), ab.after(), ab.room()), ("ab_", "", 13));
        assert_eq!(ab.fit("market", ""), "ab_market");
        assert_eq!(ab.fit("averyveryverylongname", "001"), "ab_averyveryv001", "the name cut");
        assert_eq!(ab.fit("ab_sword", "002"), "ab_sword002", "never ab_ab_");
        assert_eq!(ab.fit("AB_Sword", ""), "ab_sword", "whatever its case");
        assert_eq!(ab.fit("cd_sword", ""), "ab_cd_sword", "another's code is kept");
        let after = Naming::new("Zed", "{name}_{code}");
        assert_eq!(after.fit("lamp", "001"), "lamp001_zed");
        assert_eq!(after.fit("lamp001_zed", ""), "lamp001_zed");
        let t = Naming::new("ab", "t{code}_{name}");
        assert_eq!(t.fit("averyveryverylongname", "1"), "tab_averyveryve1");
        assert_eq!(Naming::new("abcdefgh", DEFAULT_PATTERN).room(), 7);
        // An area's (the Area Wizard's): a number added where it is taken.
        let area = |name: &str, taken: &dyn Fn(&ResRef) -> bool| {
            crate::new::resref_for_named(name, &ab, taken).to_string()
        };
        assert_eq!(area("Area 001", &|_| false), "ab_area001");
        assert_eq!(area("", &|_| false), "ab_area");
        let r = |s: &str| ResRef::from_str(s).unwrap();
        assert_eq!(area("Market", &|x| *x == r("ab_market")), "ab_market1");
        assert_eq!(area("The Long Market Street", &|_| false), "ab_thelongmarket");
        assert_eq!(crate::new::resref_for_named("Area 001", &plain, |_| false), r("area001"));
    }

    #[test]
    fn a_code_and_a_pattern_are_held_to_what_a_resref_can_be() {
        assert_eq!(code_problem("ab"), None);
        assert_eq!(code_problem("a_1"), None);
        assert!(code_problem("").is_some());
        assert_eq!(code_problem("a b").unwrap(), "\" \": a code is letters, digits and _");
        assert!(code_problem("abcdefghi").unwrap().contains("at most 8"));
        assert_eq!(pattern_problem("{code}_{name}"), None);
        assert_eq!(pattern_problem("{name}{code}"), None);
        assert_eq!(
            pattern_problem("{area}_{name}").unwrap(),
            "{area} is neither {code} nor {name}"
        );
        assert!(pattern_problem("{name}").unwrap().contains("once each"));
        assert!(pattern_problem("{code}{name}{name}").unwrap().contains("once each"));
        assert!(pattern_problem("{code}-{name}").unwrap().starts_with("\"-\""));
        assert!(pattern_problem("tdod_{code}_{name}").unwrap().contains("fewer than 6"));
        assert_eq!(pattern_problem("t{code}{name}"), None);
    }

    #[test]
    fn the_pattern_and_the_code_are_read_and_their_faults_said_by_line() {
        let mut p = read("[resrefs]\npattern = \"{name}_{code}\"\n");
        assert_eq!(p.naming(), Naming::default(), "no code: as Aurora");
        p.apply("user.toml", "[builder]\ncode = \"Zed\"\n", true);
        assert_eq!(p.user.builder_code.as_deref(), Some("zed"));
        assert_eq!(p.naming().fit("lamp", "001"), "lamp001_zed");
        let p = read("[resrefs]\npattern = \"{code}-{name}\"\nsize = 3\n");
        assert_eq!(
            p.warnings,
            [
                "moonglow.toml:2: pattern \"{code}-{name}\": \"-\": a ResRef is letters, digits \
                 and _",
                "moonglow.toml:3: \"size\" is not a key of [resrefs]",
            ]
        );
        assert_eq!(p.resref_pattern, None);
        let mut p = ProjectFile::default();
        p.apply("user.toml", "[builder]\ncode = \"a b\"\n[resrefs]\npattern = \"x\"\n", true);
        assert_eq!(
            p.warnings,
            [
                "user.toml:2: code \"a b\": \" \": a code is letters, digits and _; new ResRefs \
                 are made without it",
                "user.toml:3: [resrefs] is the team's: it goes in moonglow.toml",
            ]
        );
        assert_eq!(p.naming(), Naming::default());
    }

    #[test]
    fn setting_the_code_keeps_the_rest_of_the_builder_s_file() {
        // None yet: made, with its folder's ignore file.
        assert_eq!(with_code("", Some("ab")).unwrap(), "[builder]\ncode = \"ab\"\n");
        let text = "# mine\n[builder]\ncode = \"ab\"   # my code\n\n[later]\nx = 1\n";
        assert_eq!(
            with_code(text, Some("cd")).unwrap(),
            "# mine\n[builder]\ncode = \"cd\"   # my code\n\n[later]\nx = 1\n"
        );
        assert_eq!(with_code(text, None).unwrap(), "# mine\n[builder]\n\n[later]\nx = 1\n");
        assert_eq!(
            with_code("[builder]\n# none yet\n", Some("ab")).unwrap(),
            "[builder]\ncode = \"ab\"\n# none yet\n"
        );
        assert_eq!(with_code("[builder]", Some("ab")).unwrap(), "[builder]\ncode = \"ab\"\n");
        assert_eq!(
            with_code("[x]\ny = 1", Some("ab")).unwrap(),
            "[x]\ny = 1\n\n[builder]\ncode = \"ab\"\n"
        );
        assert_eq!(with_code("[x]\n", None).unwrap(), "[x]\n");
        let e = with_code("[builder\n", Some("ab")).unwrap_err();
        assert!(e.starts_with(".moonglow/user.toml:1: "), "{e}");
        assert!(with_code("builder = 3\n", Some("ab")).is_err());

        let dir = mg_testkit::scratch_dir("project-file-builder-code");
        ProjectFile::write_user(&dir, "# mine\n[builder]\ncode = \"ab\"\n").unwrap();
        ProjectFile::set_builder_code(&dir, Some("XY")).unwrap();
        let text = std::fs::read_to_string(ProjectFile::user_path(&dir)).unwrap();
        assert_eq!(text, "# mine\n[builder]\ncode = \"xy\"\n");
        assert_eq!(ProjectFile::read(&dir).user.builder_code.as_deref(), Some("xy"));
        let e = ProjectFile::set_builder_code(&dir, Some("much_too_long")).unwrap_err();
        assert!(e.to_string().contains("at most 8"), "{e}");
        ProjectFile::set_builder_code(&dir, None).unwrap();
        assert_eq!(ProjectFile::read(&dir).user.builder_code, None);
        // (A folder of its own, made with its ignore file.)
        let fresh = mg_testkit::scratch_dir("project-file-builder-code-fresh");
        ProjectFile::set_builder_code(&fresh, Some("ab")).unwrap();
        assert!(fresh.join(USER_DIR).join(".gitignore").is_file());
        assert_eq!(ProjectFile::read(&fresh).warnings, Vec::<String>::new());
    }

    #[test]
    fn no_toml_is_an_error_with_its_line_and_no_rules() {
        let p = read("[tags]\nunique = [\"A\"]\n[store\nblueprint = \"x\"\n");
        assert_eq!(p.errors.len(), 1, "{:?}", p.errors);
        assert!(p.errors[0].starts_with("moonglow.toml:3: "), "{}", p.errors[0]);
        assert!(p.tags.unique.is_empty() && p.store.is_none());
    }

    #[test]
    fn every_cut_of_the_file_reads_without_a_panic() {
        let full = format!(
            "{EXAMPLE}\n[resrefs]\npattern = \"{{code}}_{{name}}\"\n[builder]\ncode = \"aug\"\n"
        );
        for end in (0..=full.len()).filter(|&i| full.is_char_boundary(i)) {
            let mut p = ProjectFile::default();
            p.apply(FILE_NAME, &full[..end], false);
            p.apply("user.toml", &full[..end], true);
            let _ = p.naming().fit("name", "001");
            let _ = with_code(&full[..end], Some("ab"));
            let _ = with_code(&full[..end], None);
        }
        // And bytes that are no text.
        let mut p = ProjectFile::default();
        p.apply(FILE_NAME, &String::from_utf8_lossy(&[0xFF, b'[', 0, b'"', 0xC3]), false);
        assert_eq!(p.errors.len(), 1);
    }

    /// A module folder reads its project file when opened, and again when
    /// it changes; a module kept in one file has none.
    #[test]
    fn a_module_folder_reads_its_file_and_reads_it_again() {
        let dir = mg_testkit::scratch_dir("project-file-module");
        let mut m = crate::Module::new();
        m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
        std::fs::write(dir.join(FILE_NAME), "[tags]\nunique = [\"A_*\"]\n").unwrap();
        let mut m = crate::Module::open(&dir).unwrap();
        assert!(m.project_file.found && m.project_file.tags.unique.matches("A_1"));
        assert!(!m.reread_project_file(), "unchanged");
        // (A later time, or another size: a change seen.)
        std::fs::write(dir.join(FILE_NAME), "[tags]\nunique = [\"BB_*\"]\n").unwrap();
        assert!(m.reread_project_file());
        assert!(m.project_file.tags.unique.matches("BB_1"));
        m.save_as(&ModuleLocation::Archive(dir.with_extension("mod"))).unwrap();
        assert_eq!(m.project_file, ProjectFile::default(), "a .mod has none");
    }

    #[test]
    fn the_builder_s_own_file_is_read_beside_the_team_s_and_kept_out_of_git() {
        let dir = std::env::temp_dir().join(format!("mg-project-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let none = ProjectFile::read(&dir);
        assert!(!none.found && none.errors.is_empty() && !none.changed());
        std::fs::write(dir.join(FILE_NAME), "[tags]\nunique = [\"A_*\"]\n").unwrap();
        assert!(none.changed(), "a file made is a change");
        ProjectFile::write_user(&dir, "[builder]\ncode = \"aug\"\n[tags]\nunique = []\n").unwrap();
        let p = ProjectFile::read(&dir);
        assert!(p.found && !p.changed());
        assert_eq!(p.user.builder_code.as_deref(), Some("aug"));
        assert!(p.tags.unique.matches("A_1"), "the user's file doesn't set the team's rules");
        assert_eq!(
            p.warnings,
            [".moonglow/user.toml:3: [tags] is the team's: it goes in moonglow.toml"]
        );
        let ignore = std::fs::read_to_string(dir.join(USER_DIR).join(".gitignore")).unwrap();
        assert!(ignore.lines().any(|l| l == "*"));
        // A folder made by hand, without it: said.
        std::fs::remove_file(dir.join(USER_DIR).join(".gitignore")).unwrap();
        let p = ProjectFile::read(&dir);
        assert!(p.warnings[0].contains(".moonglow has no .gitignore"), "{:?}", p.warnings);
        std::fs::remove_file(dir.join(FILE_NAME)).unwrap();
        assert!(p.changed(), "a file deleted is a change");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
