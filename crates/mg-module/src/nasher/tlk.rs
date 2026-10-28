//! A nasher project's talk tables, kept as JSON (or as CSV, `tlkFormat =
//! "csv"`). A target whose file is a `.tlk` takes `<name>.tlk.json` from
//! its sources (nasher converts it with `nwn_tlk` and packs it as the
//! target's file; another JSON file among its sources is converted and
//! left there), so a table is known by the name the module's
//! `Mod_CustomTlk` gives it. One kept as CSV is `<name>.tlk.csv`, as
//! `nasher unpack` writes it (nasher 1.1.3's pack refuses it: "csv is not
//! a supported source type"; Moonglow makes the table of it all the
//! same). Several targets of one name
//! (`tlk/en/mytlk.tlk`, `tlk/de/mytlk.tlk`) are one table in as many
//! languages, as the game's players install the one of theirs.

use std::path::{Path, PathBuf};

use mg_core::Language;
use mg_tlk::Tlk;
use mg_tlk::csv::Format;
use mg_tlk::json::JsonStyle;

use super::{Package, Settings, Target};
use crate::ModuleError;

impl Target {
    /// Whether the target packs a talk table.
    pub fn is_talk_table(&self) -> bool {
        Path::new(&self.file).extension().is_some_and(|e| e.eq_ignore_ascii_case("tlk"))
    }

    /// The talk table's name (its file's, without `.tlk`), for a target
    /// that packs one.
    pub fn talk_table_name(&self) -> Option<String> {
        let stem = Path::new(&self.file).file_stem()?.to_string_lossy().into_owned();
        self.is_talk_table().then_some(stem)
    }

    /// A talk table's source among the target's files: `<name>.tlk.json`,
    /// as nasher packs it (`<name>.tlk.csv` for a project that keeps
    /// them as CSV).
    pub fn talk_table_source(
        &self,
        root: &Path,
        format: Format,
    ) -> Result<Option<PathBuf>, String> {
        let Some(name) = self.talk_table_name() else { return Ok(None) };
        let wanted = format!("{name}.tlk.{}", extension(format));
        Ok(self.source_files(root)?.into_iter().find(|p| {
            p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(&wanted))
        }))
    }
}

/// The extension of a talk table's file kept as `format` (after `.tlk`).
fn extension(format: Format) -> &'static str {
    match format {
        Format::Json => "json",
        Format::Csv => "csv",
    }
}

/// How a project keeps its talk tables (`tlkFormat`); `None` for a form
/// `nwn_tlk` has no reader of.
pub fn kept_as(settings: &Settings) -> Option<Format> {
    match settings.tlk_format.to_ascii_lowercase().as_str() {
        "json" => Some(Format::Json),
        "csv" => Some(Format::Csv),
        _ => None,
    }
}

/// How a talk table's file is kept, by its name: `.csv` CSV, else JSON.
pub fn format_of(path: &Path) -> Format {
    match path.extension() {
        Some(e) if e.eq_ignore_ascii_case("csv") => Format::Csv,
        _ => Format::Json,
    }
}

/// A talk table kept as JSON (or CSV) in a project.
#[derive(Debug, Clone)]
pub struct TalkTable {
    /// The target that packs it.
    pub target: String,
    /// Its JSON (or CSV) file.
    pub path: PathBuf,
    /// The file as read.
    pub file: Vec<u8>,
    /// The table it stands for.
    pub tlk: Tlk,
    /// How it is written back (`tlkFlags`).
    pub style: JsonStyle,
}

impl TalkTable {
    pub fn language(&self) -> Language {
        self.tlk.language
    }
}

/// Reads a talk table's JSON (or CSV) file as `nwn_tlk` does.
pub fn read_talk_table(path: &Path, style: JsonStyle) -> Result<(Vec<u8>, Tlk), ModuleError> {
    let bad = |message: String| ModuleError::Source { name: path.display().to_string(), message };
    let file = std::fs::read(path).map_err(|e| bad(e.to_string()))?;
    let tlk = mg_tlk::csv::read(&file, format_of(path), style.codepage).map_err(bad)?;
    Ok((file, tlk))
}

/// The talk tables named `name` (as `Mod_CustomTlk` names one) that the
/// package at `root` keeps as JSON or CSV, in its targets' order: none
/// where it keeps them in another form. An error for one that can't be
/// read.
pub fn talk_tables(
    package: &Package,
    settings: &Settings,
    name: &str,
) -> Result<Vec<TalkTable>, ModuleError> {
    let Some(format) = kept_as(settings) else { return Ok(Vec::new()) };
    let root = package.root.as_path();
    let mut out = Vec::new();
    for t in &package.targets {
        if !t.talk_table_name().is_some_and(|n| n.eq_ignore_ascii_case(name.trim())) {
            continue;
        }
        let source = t.talk_table_source(root, format).map_err(|message| ModuleError::Source {
            name: Package::config_path(root).display().to_string(),
            message,
        })?;
        let Some(path) = source else { continue };
        let (file, tlk) = read_talk_table(&path, settings.tlk_style)?;
        out.push(TalkTable { target: t.name.clone(), path, file, tlk, style: settings.tlk_style });
    }
    Ok(out)
}

/// Of a table's languages, the one in `language` (the game's), else the
/// first.
pub fn in_language(tables: Vec<TalkTable>, language: Option<Language>) -> Option<TalkTable> {
    let at = tables.iter().position(|t| Some(t.language()) == language).unwrap_or(0);
    tables.into_iter().nth(at)
}

/// A target that packs a talk table, packed as nasher packs it: its
/// `<name>.tlk.json` (or `.csv`) converted as `nwn_tlk` converts it.
/// Returns the target's file (in the project) and its bytes.
pub fn pack_talk_table(root: &Path, target: &Target) -> Result<(PathBuf, Vec<u8>), ModuleError> {
    let settings = Settings::read(root);
    let cfg = Package::config_path(root).display().to_string();
    let bad = |message: String| ModuleError::Source { name: cfg.clone(), message };
    let format = kept_as(&settings).ok_or_else(|| {
        bad(format!(
            "the project keeps talk tables as {}; Moonglow reads JSON and CSV ones only",
            settings.tlk_format
        ))
    })?;
    let name = target.talk_table_name().ok_or_else(|| bad("not a talk table's target".into()))?;
    let path = target.talk_table_source(root, format).map_err(bad)?.ok_or_else(|| {
        bad(format!(
            "target {}: no {name}.tlk.{} among its sources",
            target.name,
            extension(format)
        ))
    })?;
    let (_, tlk) = read_talk_table(&path, settings.tlk_style)?;
    let bytes = tlk.to_bytes_as_nwn_tlk().map_err(|e| ModuleError::Source {
        name: path.display().to_string(),
        message: e.to_string(),
    })?;
    Ok((root.join(&target.file), bytes))
}

/// Writes a talk table back to its JSON (or CSV) file as `nwn_tlk` writes
/// it, unless the file has that already. Whether it was written.
pub fn write_talk_table(path: &Path, tlk: &Tlk, style: JsonStyle) -> Result<bool, String> {
    let text = mg_tlk::csv::write(tlk, format_of(path), style);
    if std::fs::read(path).is_ok_and(|had| had == text) {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    // Written beside it, then moved over it: never half a table.
    let tmp = crate::sibling(path, ".part");
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: &str = "[package]\n  [package.rules]\n  \"*\" = \"src\"\n\n\
        [target]\nname = \"default\"\nfile = \"m.mod\"\n  [target.sources]\n  \
        include = \"src/**/*.{nss,json}\"\n  exclude = \"**/*.tlk.json\"\n\n\
        [target]\nname = \"tlk\"\nfile = \"mytlk.tlk\"\n  [target.sources]\n  \
        include = \"src/tlk/*.json\"\n\n\
        [target]\nname = \"tlk_de\"\nfile = \"de/mytlk.tlk\"\n  [target.sources]\n  \
        include = \"src/tlk/de/mytlk.tlk.json\"\n\n\
        [target]\nname = \"other\"\nfile = \"other.tlk\"\n  [target.sources]\n  \
        include = \"src/tlk/*.json\"\n";

    fn project(name: &str) -> PathBuf {
        let root = mg_testkit::scratch_dir(name);
        std::fs::create_dir_all(root.join("src/tlk/de")).unwrap();
        std::fs::write(root.join("nasher.cfg"), CFG).unwrap();
        let en = r#"{"language":0,"entries":[{"id":0,"text":"Hello"},{"id":2,"text":"","sound":"vs_x"}]}"#;
        std::fs::write(root.join("src/tlk/mytlk.tlk.json"), en).unwrap();
        std::fs::write(root.join("src/tlk/notes.tlk.json"), en).unwrap();
        let de = r#"{"language":2,"entries":[{"id":0,"text":"Hallo"}]}"#;
        std::fs::write(root.join("src/tlk/de/mytlk.tlk.json"), de).unwrap();
        root
    }

    #[test]
    fn a_table_in_two_languages_found_by_its_name() {
        let root = project("nasher-tlk-find");
        let package = Package::read(&root).unwrap();
        let settings = Settings::default();
        let found = talk_tables(&package, &settings, "mytlk").unwrap();
        let langs: Vec<_> = found.iter().map(|t| (t.target.as_str(), t.language())).collect();
        assert_eq!(langs, [("tlk", Language::ENGLISH), ("tlk_de", Language::GERMAN)]);
        assert_eq!(found[0].tlk.entries.len(), 3);
        let de = in_language(found.clone(), Some(Language::GERMAN)).unwrap();
        assert_eq!(de.target, "tlk_de");
        assert_eq!(in_language(found, Some(Language::FRENCH)).unwrap().target, "tlk");
        // A target whose sources have no `<name>.tlk.json`: nothing.
        assert!(talk_tables(&package, &settings, "other").unwrap().is_empty());
        assert!(talk_tables(&package, &settings, "notes").unwrap().is_empty());
        // Kept as CSV: not read.
        let csv = Settings { tlk_format: "csv".into(), ..Settings::default() };
        assert!(talk_tables(&package, &csv, "mytlk").unwrap().is_empty());
        // One that can't be read says so.
        std::fs::write(root.join("src/tlk/mytlk.tlk.json"), "{\"language\":0}").unwrap();
        let e = talk_tables(&package, &settings, "mytlk").unwrap_err().to_string();
        assert!(e.contains("mytlk.tlk.json") && e.contains("entries"), "{e}");
    }

    #[test]
    fn packed_as_nasher_packs_it() {
        let root = project("nasher-tlk-pack");
        let package = Package::read(&root).unwrap();
        let (path, bytes) = pack_talk_table(&root, package.target("tlk_de").unwrap()).unwrap();
        assert_eq!(path, root.join("de/mytlk.tlk"));
        let t = Tlk::read(&bytes).unwrap();
        assert_eq!((t.language, t.entries[0].text.as_slice()), (Language::GERMAN, &b"Hallo"[..]));
        // (A sound without text points at the text's end, as nwn_tlk has it.)
        let (_, bytes) = pack_talk_table(&root, package.target("tlk").unwrap()).unwrap();
        assert_eq!(u32::from_le_bytes(bytes[20 + 80 + 28..20 + 80 + 32].try_into().unwrap()), 5);
        let e = pack_talk_table(&root, package.target("other").unwrap()).unwrap_err();
        assert!(e.to_string().contains("no other.tlk.json"), "{e}");
        assert!(pack_talk_table(&root, package.target("default").unwrap()).is_err());
    }

    #[test]
    fn written_back_only_when_it_changed() {
        let root = project("nasher-tlk-write");
        let path = root.join("src/tlk/mytlk.tlk.json");
        let (file, mut tlk) = read_talk_table(&path, JsonStyle::default()).unwrap();
        // (As nwn_tlk writes it: nothing to write.)
        assert_eq!(write_talk_table(&path, &tlk, JsonStyle::default()), Ok(false));
        assert_eq!(std::fs::read(&path).unwrap(), file);
        tlk.entries[0].text = b"Hello there".to_vec();
        assert_eq!(write_talk_table(&path, &tlk, JsonStyle::default()), Ok(true));
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            r#"{"language":0,"entries":[{"id":0,"text":"Hello there"},{"id":2,"text":"","sound":"vs_x"}]}"#
        );
    }

    /// A project that keeps its tables as CSV (`tlkFormat = "csv"`, as
    /// `nasher unpack` writes `<name>.tlk.csv`): the table is found, read,
    /// packed and written back as `nwn_tlk` has CSV.
    #[test]
    fn a_table_kept_as_csv() {
        let root = project("nasher-tlk-csv");
        // (Its target taking the CSV in.)
        let cfg = CFG.replacen("src/tlk/*.json", "src/tlk/*.{json,csv}", 1);
        std::fs::write(root.join("nasher.cfg"), cfg).unwrap();
        let csv = root.join("src/tlk/mytlk.tlk.csv");
        std::fs::write(&csv, "0,,,\"Hello, \"\"you\"\"\"\n2,\"vs_x\",1.5,\"\"\n").unwrap();
        std::fs::create_dir_all(root.join(".nasher")).unwrap();
        std::fs::write(root.join(".nasher/user.cfg"), "tlkFormat = \"csv\"\n").unwrap();
        let package = Package::read(&root).unwrap();
        let settings = Settings::read(&root);
        assert_eq!(kept_as(&settings), Some(Format::Csv));
        let found = talk_tables(&package, &settings, "mytlk").unwrap();
        assert_eq!(found.len(), 1, "the CSV, not the JSON beside it");
        assert_eq!((found[0].path.as_path(), found[0].target.as_str()), (csv.as_path(), "tlk"));
        assert_eq!(found[0].tlk.entries[0].text, b"Hello, \"you\"");
        assert_eq!(found[0].tlk.entries[2].sound_length, 1.5);
        let (_, bytes) = pack_talk_table(&root, package.target("tlk").unwrap()).unwrap();
        assert_eq!(Tlk::read(&bytes).unwrap().entries[0].text, b"Hello, \"you\"");
        // Untouched: not written; changed: written as CSV.
        let mut tlk = found[0].tlk.clone();
        assert_eq!(write_talk_table(&csv, &tlk, settings.tlk_style), Ok(false));
        tlk.entries[0].text = b"Bye".to_vec();
        assert_eq!(write_talk_table(&csv, &tlk, settings.tlk_style), Ok(true));
        let text = std::fs::read_to_string(&csv).unwrap();
        assert_eq!(text, "0,,,\"Bye\"\n2,\"vs_x\",1.5,\"\"\n");
        // A form nwn_tlk can't read: none, and packing says so.
        std::fs::write(root.join(".nasher/user.cfg"), "tlkFormat = \"xml\"\n").unwrap();
        let settings = Settings::read(&root);
        assert!(talk_tables(&package, &settings, "mytlk").unwrap().is_empty());
        let e = pack_talk_table(&root, package.target("tlk").unwrap()).unwrap_err().to_string();
        assert!(e.contains("JSON and CSV ones only"), "{e}");
    }
}
