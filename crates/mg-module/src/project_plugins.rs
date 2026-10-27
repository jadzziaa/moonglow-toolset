//! A project's plugins as its files say them: where the plugins that come
//! with the checkout are, and the values of their settings the team shares
//! and each builder's own.
//!
//! Which of them may run is never read from here: a project's files come
//! with its checkout, and a repository could carry answers of its own
//! (a `.moonglow/user.toml` committed with its plugins "allowed"). The
//! answers are kept in Moonglow's own folder (mg-plugin's `project`);
//! a `[project_plugins]` section in either file is said, and not read.
//!
//! In the team's file (`moonglow.toml`):
//!
//! ```toml
//! [plugins]
//! folder = "tools/plugins"            # the project's plugins (else plugins/)
//!
//! [plugins."tdod.rules"]              # a plugin's settings of scope "project"
//! claims = "data/claims.toml"
//! ```
//!
//! In the builder's own (`.moonglow/user.toml`):
//!
//! ```toml
//! [plugins."tdod.rules"]              # its settings of scope "user-project"
//! code = "ab"
//! ```
//!
//! A plugin's id has dots in it, so its table's name is quoted; one
//! written without quotes (`[plugins.tdod.rules]`) reads the same, a table
//! whose values are tables naming the id's next part (a setting's value is
//! never a table). What the plugins are and declare is mg-plugin's: this
//! reads the values as written, and the plugins hold them to their
//! manifests.

use std::collections::BTreeMap;
use std::path::Path;

use toml::de::DeValue;

use crate::project_file::{Entry, FILE_NAME, ProjectFile, Reader, USER_DIR, USER_FILE, in_order};

/// Where a project's plugins are when its file doesn't say.
pub const DEFAULT_FOLDER: &str = "plugins";

/// A section of answers about a project's plugins, which is not read from
/// a project's files (they could come with the checkout).
pub const DECIDED: &str = "project_plugins";

/// A setting's value as a file holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingValue {
    Text(String),
    Int(i64),
    Bool(bool),
}

impl std::fmt::Display for SettingValue {
    /// As TOML writes it: `"text"`, `3`, `true`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingValue::Text(t) => write!(f, "{t:?}"),
            SettingValue::Int(n) => write!(f, "{n}"),
            SettingValue::Bool(b) => write!(f, "{b}"),
        }
    }
}

/// A value read: what it is (or, where it is none of a setting's kinds,
/// what it is instead: `"a list"`), and where (`moonglow.toml:12`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub value: Result<SettingValue, String>,
    pub at: String,
}

/// A plugin's table of values (`[plugins."id"]`): where it begins, and
/// its values by setting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Values {
    pub at: String,
    pub values: BTreeMap<String, Stored>,
}

/// `[plugins]`, of both files.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PluginRules {
    /// `[plugins] folder` (the team's): the project's plugins' folder,
    /// from the project's root; none: [`DEFAULT_FOLDER`].
    pub folder: Option<String>,
    /// The values of settings the team shares (scope `project`), by
    /// plugin id.
    pub team: BTreeMap<String, Values>,
    /// The builder's own values for this project (scope `user-project`).
    pub own: BTreeMap<String, Values>,
}

impl PluginRules {
    /// The folder of the project whose root is `root`.
    pub fn folder_in(&self, root: &Path) -> std::path::PathBuf {
        root.join(self.folder.as_deref().unwrap_or(DEFAULT_FOLDER))
    }
}

/// What is wrong with a folder named for the project's plugins: it is to
/// be inside the project (no absolute path, no `..`).
pub fn folder_problem(folder: &str) -> Option<String> {
    let f = folder.trim().replace('\\', "/");
    if f.is_empty() {
        return Some("empty".into());
    }
    if f.starts_with('/') || f.contains(':') {
        return Some("it is to be a folder of the project, from its root".into());
    }
    if f.split('/').any(|p| p == "..") {
        return Some("it is to be inside the project (no ..)".into());
    }
    None
}

/// What a value read is, or what it is instead.
fn stored(value: &DeValue) -> Result<SettingValue, String> {
    match value {
        DeValue::String(s) => Ok(SettingValue::Text(s.to_string())),
        DeValue::Integer(n) => i64::from_str_radix(n.as_str(), n.radix())
            .map(SettingValue::Int)
            .map_err(|_| "a number too large".to_string()),
        DeValue::Boolean(b) => Ok(SettingValue::Bool(*b)),
        DeValue::Float(_) => Err("a number with a fraction".into()),
        DeValue::Datetime(_) => Err("a date".into()),
        DeValue::Array(_) => Err("a list".into()),
        DeValue::Table(_) => Err("a table".into()),
    }
}

/// A plugin's table read into `into` under `id`; a table in it is the
/// table of the plugin whose id goes on (`[plugins.tdod.rules]`).
fn values_of(r: &mut Reader, id: String, (key, value): Entry, into: &mut BTreeMap<String, Values>) {
    let DeValue::Table(t) = value.get_ref() else { return };
    let at = format!("{}:{}", r.name, crate::project_file::line_of(r.source, key.span().start));
    for (k, v) in in_order(t) {
        let name = k.get_ref().to_string();
        if matches!(v.get_ref(), DeValue::Table(_)) {
            values_of(r, format!("{id}.{name}"), (k, v), into);
            continue;
        }
        let line = crate::project_file::line_of(r.source, v.span().start);
        let s = Stored { value: stored(v.get_ref()), at: format!("{}:{line}", r.name) };
        let table = into
            .entry(id.clone())
            .or_insert_with(|| Values { at: at.clone(), ..Default::default() });
        table.values.insert(name, s);
    }
}

/// `[plugins]` of either file (`own`: the builder's).
pub(crate) fn read_plugins(p: &mut ProjectFile, r: &mut Reader, entry: Entry, own: bool) {
    for e in r.section(entry) {
        let (key, value) = e;
        let name = key.get_ref().to_string();
        match (value.get_ref(), name.as_str()) {
            (DeValue::Table(_), _) => {
                let into = if own { &mut p.plugins.own } else { &mut p.plugins.team };
                values_of(r, name, e, into);
            }
            (_, "folder") if own => r.warn(
                key.span().start,
                format!("[plugins] folder is the team's: it goes in {FILE_NAME}"),
            ),
            (_, "folder") => {
                if let Some(f) = r.text(e) {
                    match folder_problem(&f) {
                        Some(problem) => r.warn(
                            value.span().start,
                            format!(
                                "folder \"{f}\": {problem}; the project's plugins are \
                                     looked for in {DEFAULT_FOLDER}/"
                            ),
                        ),
                        None => p.plugins.folder = Some(f.trim().replace('\\', "/")),
                    }
                }
            }
            _ => r.warn(
                key.span().start,
                format!(
                    "\"{name}\" is not a key of [plugins]: a plugin's settings are a table of \
                     their own ([plugins.\"{name}\"])"
                ),
            ),
        }
    }
}

/// `[project_plugins]`, in either file: not read. Which of a project's
/// plugins may run is kept in Moonglow's own folder, never in the
/// project's files, which come with its checkout: one there (a
/// `.moonglow/user.toml` committed with answers in it, say) is said, and
/// nothing in it allows anything.
pub(crate) fn read_decided(_: &mut ProjectFile, r: &mut Reader, entry: Entry, _own: bool) {
    r.warn(
        entry.0.span().start,
        format!(
            "[{DECIDED}] is not read: which of a project's plugins may run is kept in \
             Moonglow's own settings, never in the project's files (a checkout could carry \
             answers of its own); Moonglow asks you"
        ),
    );
}

fn invalid(e: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
}

/// A file's text read for editing (none there: empty).
fn document(path: &Path, name: &str) -> std::io::Result<toml_edit::DocumentMut> {
    let text = match std::fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    text.parse::<toml_edit::DocumentMut>().map_err(|e| {
        let line = e.span().map_or(1, |s| crate::project_file::line_of(&text, s.start));
        invalid(format!("{name}:{line}: {} (mend it first)", e.message().trim()))
    })
}

/// The table `name` of `doc`, made where it is not there.
fn table<'d>(
    doc: &'d mut toml_edit::Table,
    name: &str,
    implicit: bool,
) -> std::io::Result<&'d mut toml_edit::Table> {
    let item = doc.entry(name).or_insert_with(|| {
        let mut t = toml_edit::Table::new();
        t.set_implicit(implicit);
        toml_edit::Item::Table(t)
    });
    item.as_table_mut().ok_or_else(|| invalid(format!("\"{name}\" is not a table (mend it first)")))
}

/// A plugin's table under `[plugins]`: as its id names it, or nested by
/// the parts of its id.
fn plugin_table<'t>(
    plugins: &'t mut toml_edit::Table,
    id: &str,
) -> Option<&'t mut toml_edit::Table> {
    if plugins.get(id).is_some_and(|i| i.is_table()) {
        return plugins.get_mut(id)?.as_table_mut();
    }
    let at = id.match_indices('.').map(|(i, _)| i).find(|&i| {
        plugins.get(&id[..i]).is_some_and(|t| {
            t.as_table().is_some_and(|t| {
                let rest = &id[i + 1..];
                t.get(rest).is_some_and(|x| x.is_table()) || rest.contains('.')
            })
        })
    })?;
    plugin_table(plugins.get_mut(&id[..at])?.as_table_mut()?, &id[at + 1..])
}

fn value_of(v: &SettingValue) -> toml_edit::Value {
    match v {
        SettingValue::Text(t) => t.as_str().into(),
        SettingValue::Int(n) => (*n).into(),
        SettingValue::Bool(b) => (*b).into(),
    }
}

/// The text of a file with a plugin's setting set to `value` (or, `None`,
/// taken out), the rest as written.
pub fn with_setting(
    doc: &mut toml_edit::DocumentMut,
    id: &str,
    name: &str,
    value: Option<&SettingValue>,
) -> std::io::Result<()> {
    let plugins = table(doc.as_table_mut(), "plugins", true)?;
    match value {
        Some(v) => {
            if plugin_table(plugins, id).is_none() {
                plugins.insert(id, toml_edit::Item::Table(toml_edit::Table::new()));
            }
            let t = plugin_table(plugins, id).ok_or_else(|| invalid("[plugins]: no table"))?;
            match t.get_mut(name).and_then(|i| i.as_value_mut()) {
                // (Its comment, after it, kept.)
                Some(old) => {
                    let decor = old.decor().clone();
                    *old = value_of(v);
                    *old.decor_mut() = decor;
                }
                None => {
                    t.insert(name, toml_edit::value(value_of(v)));
                }
            }
        }
        None => {
            if let Some(t) = plugin_table(plugins, id) {
                t.remove(name);
                // (Left empty, it goes; but not a comment written over it.)
                let said =
                    t.decor().prefix().and_then(|p| p.as_str()).is_some_and(|p| p.contains('#'));
                if t.is_empty() && !said {
                    plugins.remove(id);
                }
            }
        }
    }
    Ok(())
}

/// Sets (or takes out) a plugin's setting in the team's file (`own`
/// false: scope `project`) or the builder's own (scope `user-project`),
/// keeping the rest of the file as it is written.
pub fn set_setting(
    root: &Path,
    own: bool,
    id: &str,
    name: &str,
    value: Option<&SettingValue>,
) -> std::io::Result<()> {
    let (path, shown) = if own {
        (ProjectFile::user_path(root), format!("{USER_DIR}/{USER_FILE}"))
    } else {
        (root.join(FILE_NAME), FILE_NAME.to_string())
    };
    let mut doc = document(&path, &shown)?;
    with_setting(&mut doc, id, name, value)?;
    if own {
        ProjectFile::write_user(root, &doc.to_string())
    } else {
        crate::write_replacing(&path, doc.to_string().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_file::FILE_NAME;

    fn read(team: &str, own: &str) -> ProjectFile {
        let mut p = ProjectFile::default();
        p.apply(FILE_NAME, team, false);
        p.apply(".moonglow/user.toml", own, true);
        p
    }

    #[test]
    fn the_team_s_and_the_builder_s_plugin_tables_are_read() {
        let p = read(
            "[plugins]\nfolder = \"tools/plugins\"\n\n[plugins.\"tdod.rules\"]\nclaims = \
             \"data/claims.toml\"\nmax = 3\nstrict = true\nratio = 0.5\n\n[plugins.x.y]\nz = \
             \"w\"\n",
            "[plugins.\"tdod.rules\"]\ncode = \"ab\"\n",
        );
        assert_eq!(p.warnings, Vec::<String>::new());
        let r = &p.plugins;
        assert_eq!(r.folder.as_deref(), Some("tools/plugins"));
        assert_eq!(r.folder_in(Path::new("/p")), Path::new("/p/tools/plugins"));
        let team = &r.team["tdod.rules"];
        assert_eq!(team.at, "moonglow.toml:4");
        assert_eq!(team.values["claims"].value, Ok(SettingValue::Text("data/claims.toml".into())));
        assert_eq!(team.values["max"].value, Ok(SettingValue::Int(3)));
        assert_eq!(team.values["strict"].value, Ok(SettingValue::Bool(true)));
        assert_eq!(team.values["ratio"].value, Err("a number with a fraction".into()));
        assert_eq!(team.values["ratio"].at, "moonglow.toml:8");
        // (Unquoted, the id's parts nest.)
        assert_eq!(r.team["x.y"].values["z"].value, Ok(SettingValue::Text("w".into())));
        assert_eq!(r.own["tdod.rules"].values["code"].value, Ok(SettingValue::Text("ab".into())));
    }

    #[test]
    fn what_is_out_of_place_is_said_by_line() {
        let p = read(
            "[plugins]\nfolder = \"../elsewhere\"\nloud = 3\n[project_plugins]\n\"a.b\" = { \
             allow = true }\n",
            "[plugins]\nfolder = \"x\"\n[project_plugins]\n\"a.b\" = { allow = true, digest = \
             \"sha256:00\" }\n",
        );
        assert_eq!(
            p.warnings,
            [
                "moonglow.toml:2: folder \"../elsewhere\": it is to be inside the project (no \
                 ..); the project's plugins are looked for in plugins/",
                "moonglow.toml:3: \"loud\" is not a key of [plugins]: a plugin's settings are a \
                 table of their own ([plugins.\"loud\"])",
                "moonglow.toml:4: [project_plugins] is not read: which of a project's plugins \
                 may run is kept in Moonglow's own settings, never in the project's files (a \
                 checkout could carry answers of its own); Moonglow asks you",
                ".moonglow/user.toml:2: [plugins] folder is the team's: it goes in moonglow.toml",
                ".moonglow/user.toml:3: [project_plugins] is not read: which of a project's \
                 plugins may run is kept in Moonglow's own settings, never in the project's \
                 files (a checkout could carry answers of its own); Moonglow asks you",
            ]
        );
        assert!(p.plugins.folder.is_none());
        assert_eq!(
            folder_problem("/abs"),
            Some("it is to be a folder of the project, from its root".into())
        );
        assert!(folder_problem("c:\\x").is_some() && folder_problem("a/b").is_none());
    }

    #[test]
    fn a_setting_is_written_in_place_and_the_rest_kept() {
        let mut doc: toml_edit::DocumentMut =
            "# the team's\n[tags]\nunique = [\"A_*\"]\n\n[plugins.\"t.r\"]\nmax = 3 # most\n"
                .parse()
                .unwrap();
        with_setting(&mut doc, "t.r", "max", Some(&SettingValue::Int(5))).unwrap();
        with_setting(&mut doc, "t.r", "name", Some(&SettingValue::Text("x".into()))).unwrap();
        assert_eq!(
            doc.to_string(),
            "# the team's\n[tags]\nunique = [\"A_*\"]\n\n[plugins.\"t.r\"]\nmax = 5 # most\nname \
             = \"x\"\n"
        );
        with_setting(&mut doc, "o.p", "on", Some(&SettingValue::Bool(true))).unwrap();
        assert!(doc.to_string().ends_with("\n[plugins.\"o.p\"]\non = true\n"), "{doc}");
        // Taken out; a table left empty goes.
        with_setting(&mut doc, "o.p", "on", None).unwrap();
        with_setting(&mut doc, "t.r", "name", None).unwrap();
        assert_eq!(
            doc.to_string(),
            "# the team's\n[tags]\nunique = [\"A_*\"]\n\n[plugins.\"t.r\"]\nmax = 5 # most\n"
        );
        // One with a comment over it stays, for the comment.
        let mut doc: toml_edit::DocumentMut = "# ours\n[plugins.\"t.r\"]\nx = 1\n".parse().unwrap();
        with_setting(&mut doc, "t.r", "x", None).unwrap();
        assert_eq!(doc.to_string(), "# ours\n[plugins.\"t.r\"]\n");
        // A table nested by the id's parts is found.
        let mut doc: toml_edit::DocumentMut = "[plugins.t.r]\nmax = 3\n".parse().unwrap();
        with_setting(&mut doc, "t.r", "max", Some(&SettingValue::Int(4))).unwrap();
        assert_eq!(doc.to_string(), "[plugins.t.r]\nmax = 4\n");
        // And what reads back is what was written.
        let mut p = ProjectFile::default();
        p.apply(FILE_NAME, &doc.to_string(), false);
        assert_eq!(p.plugins.team["t.r"].values["max"].value, Ok(SettingValue::Int(4)));
    }

    #[test]
    fn settings_are_written_into_the_project_s_files() {
        let dir = mg_testkit::scratch_dir("project-plugins-settings");
        ProjectFile::write_user(&dir, "[builder]\ncode = \"ab\"\n").unwrap();
        // A setting of the builder's own, and one of the team's.
        set_setting(&dir, true, "t.r", "code", Some(&SettingValue::Text("cd".into()))).unwrap();
        set_setting(&dir, false, "t.r", "max", Some(&SettingValue::Int(2))).unwrap();
        let p = ProjectFile::read(&dir);
        assert_eq!(p.warnings, Vec::<String>::new());
        assert_eq!(p.user.builder_code.as_deref(), Some("ab"), "the rest of the file kept");
        assert_eq!(p.plugins.own["t.r"].values["code"].value, Ok(SettingValue::Text("cd".into())));
        assert_eq!(p.plugins.team["t.r"].values["max"].value, Ok(SettingValue::Int(2)));
        assert!(p.found);
        // A file that is no TOML is not written over.
        std::fs::write(dir.join(FILE_NAME), "[tags\n").unwrap();
        let e = set_setting(&dir, false, "t.r", "max", Some(&SettingValue::Int(1))).unwrap_err();
        assert!(e.to_string().starts_with("moonglow.toml:1: "), "{e}");
        assert_eq!(std::fs::read_to_string(dir.join(FILE_NAME)).unwrap(), "[tags\n");
    }

    #[test]
    fn every_cut_reads_without_a_panic() {
        let full = "[plugins]\nfolder = \"p\"\n[plugins.\"a.b\"]\nx = 1\n[plugins.c.d]\ny = \
                    true\n[project_plugins]\n\"a.b\" = { allow = true, digest = \"s\" }\n";
        for end in (0..=full.len()).filter(|&i| full.is_char_boundary(i)) {
            let _ = read(&full[..end], &full[..end]);
            if let Ok(mut doc) = full[..end].parse::<toml_edit::DocumentMut>() {
                let _ = with_setting(&mut doc, "a.b", "x", Some(&SettingValue::Int(2)));
                let _ = with_setting(&mut doc, "c.d", "y", None);
            }
        }
    }
}
