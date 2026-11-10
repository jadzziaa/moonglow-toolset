//! A table's description: what the columns of a builder's own 2DA hold,
//! said in a file of text beside it (`name.2da.toml`), so that the table
//! is checked against it and its cells are chosen from lists.
//!
//! The table stays the truth: a cell keeps its value (`0x0010`), and the
//! description says the value's name (`DAMAGE_TYPE_ACID`). A list is
//! written in the file or read from a script's constants, which it then
//! cannot fall behind.
//!
//! ```toml
//! rows = "spells"                 # each row says more of spells.2da's row
//!
//! [columns.Level]
//! kind = "int"
//! min = -1
//! max = 10
//!
//! [columns.DamageType]
//! kind = "flags"                  # several of the values at once
//! constants = "nwscript"          # the script's constants...
//! prefix = "DAMAGE_TYPE_"         # ...whose names begin so
//!
//! [columns.Shape]
//! kind = "list"                   # one of the values
//! values = { Sphere = 0, Cone = 1 }
//! other = true                    # or a value typed by hand
//!
//! [columns.Class]
//! kind = "row"
//! table = "classes"
//!
//! [[rules]]                       # filled where another cell says so
//! when = { column = "Effect", is = "CURE" }
//! require = "Cures"
//! ```
//!
//! One description may be a family's: `tables = ["loot_*"]` in a file of
//! the folder (`loot.2da.toml`, or any `*.2da.toml`) describes each table
//! there whose name is so and has no description of its own. And Moonglow
//! brings descriptions of some of the game's own tables
//! ([`game_description`]), for a copy of one that has none.

use std::path::{Path, PathBuf};

use mg_2da::TwoDa;

use crate::doctor::{CellFault, TalkTables, strref_problem};

/// A value a list or a set of flags has, and its name.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    /// The value as a cell holds it.
    pub value: String,
}

/// What a described column holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Anything.
    Text,
    Int {
        min: Option<i64>,
        max: Option<i64>,
    },
    Float {
        min: Option<f64>,
        max: Option<f64>,
    },
    /// One of the entries (or, `other`, a value of the builder's own).
    List {
        entries: Vec<Entry>,
        other: bool,
    },
    /// Any of the entries at once: their values are bits of the cell's.
    Flags {
        entries: Vec<Entry>,
    },
    /// A row's number in the table named (without `.2da`).
    Row(String),
    /// A talk-table string's number.
    StrRef,
    /// A script's name (one `ExecuteScript` runs by the cell's value): the
    /// Unused report counts the scripts the module's table names as used.
    Script,
}

/// A described column.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub kind: Kind,
    /// What the file says of it, shown with the column.
    pub note: Option<String>,
}

/// A rule between cells: where `column` is `is` (a name or a value),
/// `require` is filled.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub column: String,
    pub is: String,
    pub require: String,
}

/// A table's description.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Description {
    /// The names of the tables it describes, beside its own (`loot_*`: a
    /// family's; `*` any run of letters, `?` one): empty, the table of
    /// its file's name alone.
    pub tables: Vec<String>,
    /// The table (without `.2da`) each of whose rows this table's row of
    /// the same number says more of.
    pub rows: Option<String>,
    pub columns: Vec<(String, Column)>,
    pub rules: Vec<Rule>,
    /// What was not understood, which leaves the rest as it reads: a
    /// script's constant that is not a plain number, a key unknown.
    pub notes: Vec<String>,
}

/// The description file of the table at `table` (`name.2da` gives
/// `name.2da.toml`, beside it).
pub fn path_beside(table: &Path) -> PathBuf {
    let mut name = table.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".toml");
    table.with_file_name(name)
}

/// Whether a table's name (without `.2da`) is one a description's
/// `tables` names: `*` any run of letters, `?` one, whatever their case.
pub fn names_table(pattern: &str, name: &str) -> bool {
    fn fits(p: &[u8], n: &[u8]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(b'*'), _) => fits(&p[1..], n) || (!n.is_empty() && fits(p, &n[1..])),
            (Some(b'?'), Some(_)) => fits(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => fits(&p[1..], &n[1..]),
            _ => false,
        }
    }
    let pattern = pattern.trim().trim_end_matches(".2da");
    fits(pattern.as_bytes(), name.as_bytes())
}

/// The names a description file says it describes (its `tables`), read
/// alone: no lists are read from scripts for it.
fn tables_of(file: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(file) else { return Vec::new() };
    let Ok(t) = text.parse::<toml::Table>() else { return Vec::new() };
    patterns(&t)
}

fn patterns(t: &toml::Table) -> Vec<String> {
    match t.get("tables") {
        Some(toml::Value::String(s)) => vec![s.clone()],
        Some(toml::Value::Array(a)) => {
            a.iter().filter_map(|v| v.as_str()).map(str::to_string).collect()
        }
        _ => Vec::new(),
    }
}

/// How closely a pattern names a table: its letters that are not `*` or
/// `?` (`loot_gold*` before `loot_*`).
fn closeness(pattern: &str) -> usize {
    pattern.chars().filter(|c| *c != '*' && *c != '?').count()
}

/// Of the description files `files` (each with the tables it names), the
/// one that describes the table `name`: the closest pattern, then the
/// first file by name.
fn family_of<'a>(name: &str, files: &'a [(PathBuf, Vec<String>)]) -> Option<&'a Path> {
    files
        .iter()
        .filter_map(|(file, names)| {
            let best = names.iter().filter(|p| names_table(p, name)).map(|p| closeness(p)).max()?;
            Some((best, file))
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(a.1)))
        .map(|(_, f)| f.as_path())
}

/// The description files of a folder that name tables of a family
/// (`tables = […]`), each with the names.
fn families_in(dir: &Path) -> Vec<(PathBuf, Vec<String>)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<(PathBuf, Vec<String>)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().to_ascii_lowercase().ends_with(".2da.toml"))
        .map(|p| {
            let names = tables_of(&p);
            (p, names)
        })
        .filter(|(_, names)| !names.is_empty())
        .collect();
    out.sort();
    out
}

/// The description of the table at `table` (its file, or where its file
/// would be): `name.2da.toml` beside it, else a family's in its folder
/// (a `*.2da.toml` whose `tables` name it). `None`: neither is there.
pub fn find_for(table: &Path) -> Option<PathBuf> {
    let own = path_beside(table);
    if own.is_file() {
        return Some(own);
    }
    let name = table.file_stem()?.to_string_lossy().into_owned();
    let dir = table.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    family_of(&name, &families_in(dir)).map(Path::to_path_buf)
}

/// Where the module's table would be on disk, for its description: its
/// file (a nasher project's source, a module folder's file), or, for a
/// module kept in one file, beside that file.
pub fn table_in_module(module: &crate::Module, key: &mg_resman::ResKey) -> Option<PathBuf> {
    if let Some(file) = module.file_of(key) {
        return Some(file.to_path_buf());
    }
    match &module.location {
        Some(crate::ModuleLocation::Archive(path)) => Some(path.with_file_name(key.to_string())),
        _ => None,
    }
}

/// The description of one of the module's tables that is there
/// ([`find_for`] where [`table_in_module`] puts it).
pub fn description_in_module(module: &crate::Module, key: &mg_resman::ResKey) -> Option<PathBuf> {
    find_for(&table_in_module(module, key)?)
}

/// The description Moonglow brings of one of the game's own tables, by
/// its name (from nwn.wiki's pages on it): a table of that name with no
/// description of the builder's is held to it, its lists and flags shown
/// by name.
pub fn game_description(name: &str) -> Option<&'static str> {
    match name.trim_end_matches(".2da").to_ascii_lowercase().as_str() {
        "spells" => Some(include_str!("table_desc/spells.2da.toml")),
        "baseitems" => Some(include_str!("table_desc/baseitems.2da.toml")),
        _ => None,
    }
}

/// Where the description of one of a module's tables would be: beside
/// the table's file (a nasher project's source, a module folder's file),
/// or, for a module kept in one file, beside that file (`loot.2da.toml`
/// in the folder `my.mod` is in).
pub fn path_in_module(module: &crate::Module, key: &mg_resman::ResKey) -> Option<PathBuf> {
    if let Some(file) = module.file_of(key) {
        return Some(path_beside(file));
    }
    match &module.location {
        Some(crate::ModuleLocation::Archive(path)) => {
            Some(path_beside(&path.with_file_name(key.to_string())))
        }
        _ => None,
    }
}

/// The folders a build fills, which the walk passes over.
const BUILT: [&str; 6] = ["target", "node_modules", "build", "dist", "bin", "obj"];

/// The files of a folder (a repository, a project) and of those in it,
/// for what a description names: a script by its name, a table by its.
#[derive(Debug, Clone, Default)]
pub struct Beside {
    files: Vec<PathBuf>,
}

impl Beside {
    /// The folder a table's description looks in for what it names: the
    /// repository or project the table is in (the nearest folder above it
    /// with a `nasher.cfg` or a `.git`), else the table's own folder.
    pub fn root_of(table: &Path) -> PathBuf {
        let own = table.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let own = own.canonicalize().unwrap_or_else(|_| own.to_path_buf());
        let root = own
            .ancestors()
            .find(|d| d.join("nasher.cfg").is_file() || d.join(".git").exists())
            .unwrap_or(&own);
        // (A table in a folder the walk passes over, a build's or one
        // whose name begins with a dot: its own folder, which is walked.)
        let passed_over = own.strip_prefix(root).is_ok_and(|inside| {
            inside.components().any(|c| {
                let name = c.as_os_str().to_string_lossy();
                name.starts_with('.') || BUILT.iter().any(|b| name.eq_ignore_ascii_case(b))
            })
        });
        if passed_over { own.clone() } else { root.to_path_buf() }
    }

    /// The files under `root`: not those of folders whose names begin
    /// with a dot, nor of the folders a build fills (`target`,
    /// `node_modules`, `build`, `dist`, `bin`, `obj`), which hold no
    /// sources and may be very large.
    pub fn of(root: &Path) -> Beside {
        let mut files = Vec::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                match e.file_type() {
                    Ok(t) if t.is_dir() => {
                        if !BUILT.iter().any(|b| name.eq_ignore_ascii_case(b)) {
                            dirs.push(e.path());
                        }
                    }
                    Ok(_) if mg_resman::is_file_or_link_to_one(&e) => files.push(e.path()),
                    _ => {}
                }
            }
        }
        files.sort();
        Beside { files }
    }

    /// Every file walked.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// A table of the folder's file, by its name: where it is.
    pub fn table_file(&self, name: &str) -> Option<&Path> {
        self.named(name, "2da")
    }

    /// A table of the folder, by its name.
    pub fn table(&self, name: &str, codepage: mg_core::Codepage) -> Option<TwoDa> {
        let data = std::fs::read(self.named(name, "2da")?).ok()?;
        TwoDa::parse(&data, codepage).ok()
    }

    fn named(&self, name: &str, extension: &str) -> Option<&Path> {
        let file = format!("{name}.{extension}");
        self.files
            .iter()
            .find(|p| {
                p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(&file))
            })
            .map(PathBuf::as_path)
    }

    /// A script's source, by its name.
    pub fn script(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.named(name, "nss")?).ok()
    }

    /// How many rows a table of the folder has, by its name.
    pub fn rows_of(&self, name: &str, codepage: mg_core::Codepage) -> Option<usize> {
        self.table(name, codepage).map(|t| t.len())
    }

    /// The tables of the folder that have a description: each with its
    /// description's file (beside it, or a family's in its folder).
    pub fn described(&self) -> Vec<(PathBuf, PathBuf)> {
        let mut families: std::collections::HashMap<PathBuf, Vec<(PathBuf, Vec<String>)>> =
            Default::default();
        let mut out: Vec<(PathBuf, PathBuf)> = Vec::new();
        for p in self
            .files
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("2da")))
        {
            let own = path_beside(p);
            if self.files.contains(&own) {
                out.push((p.clone(), own));
                continue;
            }
            let dir = p.parent().unwrap_or(Path::new(".")).to_path_buf();
            let in_dir = families.entry(dir.clone()).or_insert_with(|| {
                self.files
                    .iter()
                    .filter(|f| f.parent() == Some(dir.as_path()))
                    .filter(|f| f.to_string_lossy().to_ascii_lowercase().ends_with(".2da.toml"))
                    .map(|f| (f.clone(), tables_of(f)))
                    .filter(|(_, names)| !names.is_empty())
                    .collect()
            });
            let name = p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if let Some(family) = family_of(&name, in_dir) {
                out.push((p.clone(), family.to_path_buf()));
            }
        }
        out.sort();
        out
    }
}

/// A script's constants whose names begin with `prefix`, each with its
/// value: a number as written, or what an expression of whole numbers and
/// the script's other constants comes to (`A | B`, `1 << 3`); with `text`,
/// a string constant's text too. And how many could not be worked out (a
/// function's result).
///
/// The scripts it includes are read too (`script` reads one by its name):
/// their constants are listed with its own, and a constant made of theirs
/// is worked out.
pub(crate) fn constants_in(
    source: &[u8],
    prefix: &str,
    text: bool,
    script: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> (Vec<Entry>, usize) {
    // The script and those it includes, each once, those included first
    // (as the compiler reads them).
    fn gather(
        source: Vec<u8>,
        script: &dyn Fn(&str) -> Option<Vec<u8>>,
        seen: &mut Vec<String>,
        out: &mut Vec<mg_script::outline::Outline>,
    ) {
        let outline = mg_script::outline::outline(&source);
        for include in &outline.includes {
            let name = include.name.trim_end_matches(".nss").to_ascii_lowercase();
            if seen.contains(&name) || seen.len() > 64 {
                continue;
            }
            seen.push(name.clone());
            if let Some(included) = script(&name) {
                gather(included, script, seen, out);
            }
        }
        out.push(outline);
    }
    let mut outlines = Vec::new();
    gather(source.to_vec(), script, &mut Vec::new(), &mut outlines);
    // (Each once: a script that includes itself, or one included twice.)
    let mut globals: Vec<&mg_script::outline::Global> = Vec::new();
    for g in outlines.iter().flat_map(|o| o.globals.iter()) {
        if !globals.iter().any(|x| x.name == g.name) {
            globals.push(g);
        }
    }
    // (A constant may be made of others of its script, or of one it
    // includes: `A | B`.)
    let given = |g: &&&mg_script::outline::Global| g.is_const || g.value.is_some();
    let mut known: Vec<(&str, i64)> = Vec::new();
    for g in globals.iter().filter(given) {
        if let Some(v) = g.value.as_deref().and_then(|v| evaluate(v, &known)) {
            known.push((g.name.as_str(), v));
        }
    }
    let (mut entries, mut unread) = (Vec::new(), 0);
    // (Constants, and the globals given a value: the game's own
    // `nwscript` declares its constants so, `int DAMAGE_TYPE_ACID =
    // 16;`.)
    for g in globals.iter().filter(given).filter(|g| g.name.starts_with(prefix)) {
        // A number as written, a text or a fraction as it is, or what an
        // expression of whole numbers comes to.
        let written = g.value.as_deref().map(str::trim);
        let quoted = written
            .filter(|v| text && g.ty == "string" && v.len() >= 2)
            .and_then(|v| v.strip_prefix('"')?.strip_suffix('"'))
            .map(str::to_string);
        let written = written.map(|v| v.trim_matches('"').to_string());
        let plain = written.filter(|v| whole(v).is_some() || v.parse::<f64>().is_ok());
        let value = quoted
            .or(plain)
            .or_else(|| known.iter().find(|(n, _)| *n == g.name).map(|(_, v)| v.to_string()));
        match value {
            Some(value) if !entries.iter().any(|e: &Entry| e.name == g.name) => {
                entries.push(Entry { name: g.name.clone(), value });
            }
            Some(_) => {}
            None => unread += 1,
        }
    }
    (entries, unread)
}

/// A cell's or a value's number: an integer as the game reads one (a
/// sign, decimal or `0x`), with nothing after it. (Wider than the game's
/// own 32 bits, so that a flag of the highest bit, `0x80000000`, is the
/// bit and not a negative number.)
pub fn whole(text: &str) -> Option<i64> {
    let s = text.trim();
    let (negative, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let v = match digits.get(..2) {
        Some("0x" | "0X") => i64::from_str_radix(&digits[2..], 16).ok()?,
        _ if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {
            digits.parse::<i64>().ok()?
        }
        _ => return None,
    };
    Some(if negative { -v } else { v })
}

/// What a constant's expression comes to: whole numbers and the names
/// in `known`, with `| & ^ ~ << >> + - *` and brackets, as NWScript
/// reads them. `None` for anything else, and for one nested deeper than
/// 64 brackets and signs (no constant is; a crafted one of thousands
/// overflowed the stack).
fn evaluate(text: &str, known: &[(&str, i64)]) -> Option<i64> {
    const MOST_NESTED: usize = 64;
    #[derive(Clone, Debug, PartialEq)]
    enum Tok {
        Num(i64),
        Op(&'static str),
    }
    const OPS: [&str; 11] = ["<<", ">>", "|", "&", "^", "~", "+", "-", "*", "(", ")"];
    let mut toks = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if let Some(op) = OPS.iter().find(|op| rest.starts_with(**op)) {
            toks.push(Tok::Op(op));
            rest = rest[op.len()..].trim_start();
            continue;
        }
        let end =
            rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        let word = &rest[..end];
        let value = whole(word).or_else(|| known.iter().find(|(n, _)| *n == word).map(|k| k.1))?;
        toks.push(Tok::Num(value));
        rest = rest[end..].trim_start();
    }
    // Lowest first: | ^ & shifts, + and -, *, then a sign or ~ and brackets.
    // (`nested`: the brackets and signs it is within, which a constant
    // of a crafted script may have thousands of, each a call deeper.)
    fn level(toks: &[Tok], at: &mut usize, depth: usize, nested: usize) -> Option<i64> {
        if nested > MOST_NESTED {
            return None;
        }
        const LEVELS: [&[&str]; 6] = [&["|"], &["^"], &["&"], &["<<", ">>"], &["+", "-"], &["*"]];
        if depth == LEVELS.len() {
            return match toks.get(*at)? {
                Tok::Num(v) => {
                    *at += 1;
                    Some(*v)
                }
                Tok::Op("(") => {
                    *at += 1;
                    let v = level(toks, at, 0, nested + 1)?;
                    (toks.get(*at) == Some(&Tok::Op(")"))).then(|| *at += 1)?;
                    Some(v)
                }
                Tok::Op(op @ ("-" | "~" | "+")) => {
                    *at += 1;
                    let v = level(toks, at, depth, nested + 1)?;
                    Some(match *op {
                        "-" => v.checked_neg()?,
                        "~" => !v & 0xFFFF_FFFF,
                        _ => v,
                    })
                }
                Tok::Op(_) => None,
            };
        }
        let mut left = level(toks, at, depth + 1, nested)?;
        while let Some(Tok::Op(op)) = toks.get(*at)
            && LEVELS[depth].contains(op)
        {
            *at += 1;
            let right = level(toks, at, depth + 1, nested)?;
            left = match *op {
                "|" => left | right,
                "^" => left ^ right,
                "&" => left & right,
                "<<" => left.checked_shl(u32::try_from(right).ok()?)?,
                ">>" => left.checked_shr(u32::try_from(right).ok()?)?,
                "+" => left.checked_add(right)?,
                "-" => left.checked_sub(right)?,
                _ => left.checked_mul(right)?,
            };
        }
        Some(left)
    }
    let mut at = 0;
    let value = level(&toks, &mut at, 0, 0)?;
    (at == toks.len()).then_some(value)
}

/// Whether a cell is an entry's value: as numbers where both are, else as
/// text whatever its case.
fn same(cell: &str, value: &str) -> bool {
    match (whole(cell), whole(value)) {
        (Some(a), Some(b)) => a == b,
        _ => match (cell.trim().parse::<f64>(), value.trim().parse::<f64>()) {
            (Ok(a), Ok(b)) => a == b,
            _ => cell.trim().eq_ignore_ascii_case(value.trim()),
        },
    }
}

impl Kind {
    /// The entries of a list or of flags.
    pub fn entries(&self) -> &[Entry] {
        match self {
            Kind::List { entries, .. } | Kind::Flags { entries } => entries,
            _ => &[],
        }
    }

    /// A cell's value by name: a list's entry, or the flags it has (those
    /// no entry names, as a number).
    pub fn named(&self, cell: &str) -> Option<String> {
        match self {
            Kind::List { entries, .. } => {
                entries.iter().find(|e| same(cell, &e.value)).map(|e| e.name.clone())
            }
            Kind::Flags { entries } => {
                let bits = whole(cell)?;
                let (set, rest) = flags_of(entries, bits);
                // (A name for several flags together is left out where
                // each of them is named: `A | B`, not `A | B | BOTH`.)
                let value = |e: &Entry| whole(&e.value).unwrap_or(0);
                let singles = set
                    .iter()
                    .map(|e| value(e))
                    .filter(|v| v.count_ones() == 1)
                    .fold(0, |a, v| a | v);
                let mut names: Vec<String> = set
                    .iter()
                    .filter(|e| value(e).count_ones() == 1 || value(e) & !singles != 0)
                    .map(|e| e.name.clone())
                    .collect();
                if rest != 0 {
                    names.push(format!("{rest:#X}"));
                }
                (!names.is_empty()).then(|| names.join(" | "))
            }
            _ => None,
        }
    }

    /// A cell for a set of flags (`None`: no flag, an empty cell), written
    /// as the cell was (`was`), or else as the entries' own values are:
    /// hexadecimal as wide, or decimal.
    pub fn flags_cell(&self, bits: i64, was: Option<&str>) -> Option<String> {
        if bits == 0 {
            return None;
        }
        let is_hex = |v: &&str| v.starts_with("0x") || v.starts_with("0X");
        let width = match was.map(str::trim).filter(|v| whole(v).is_some()) {
            Some(was) => Some(was).filter(is_hex).map(|v| v.len() - 2),
            None => self
                .entries()
                .iter()
                .map(|e| e.value.trim())
                .filter(is_hex)
                .map(|v| v.len() - 2)
                .max(),
        };
        match width {
            Some(width) => Some(format!("0x{bits:0width$X}")),
            None => Some(bits.to_string()),
        }
    }

    /// In a few words, for a column's tip.
    pub fn words(&self) -> String {
        let range = |min: Option<String>, max: Option<String>| match (min, max) {
            (Some(a), Some(b)) => format!(" from {a} to {b}"),
            (Some(a), None) => format!(" of {a} or more"),
            (None, Some(b)) => format!(" of {b} or less"),
            (None, None) => String::new(),
        };
        match self {
            Kind::Text => "text".to_string(),
            Kind::Int { min, max } => {
                format!("a number{}", range(min.map(|v| v.to_string()), max.map(|v| v.to_string())))
            }
            Kind::Float { min, max } => format!(
                "a number with a fraction{}",
                range(min.map(|v| v.to_string()), max.map(|v| v.to_string()))
            ),
            Kind::List { entries, other: false } => format!("one of {} values", entries.len()),
            Kind::List { entries, other: true } => {
                format!("one of {} values, or another", entries.len())
            }
            Kind::Flags { entries } => format!("any of {} flags", entries.len()),
            Kind::Row(table) => format!("a row of {table}.2da"),
            Kind::StrRef => "a talk-table string's number".to_string(),
            Kind::Script => "a script's name".to_string(),
        }
    }
}

/// The entries whose bits are all in `bits` (a flag of several bits where
/// each is set, and none of its bits another's alone), and the bits no
/// entry has.
pub fn flags_of(entries: &[Entry], bits: i64) -> (Vec<&Entry>, i64) {
    let mut rest = bits;
    let mut set = Vec::new();
    for e in entries {
        let Some(v) = whole(&e.value).filter(|v| *v != 0) else { continue };
        if bits & v == v {
            set.push(e);
            rest &= !v;
        }
    }
    (set, rest)
}

fn text(table: &toml::Table, key: &str) -> Option<String> {
    table.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

/// A value of the file as a cell holds it: a number as written in the
/// table's own way is given as text (`"0x0010"`), an integer as itself.
fn cell_text(v: &toml::Value) -> Option<String> {
    match v {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::Boolean(b) => Some(u8::from(*b).to_string()),
        _ => None,
    }
}

/// Where a description's parts are written, for the lines its messages
/// name.
struct Lines<'s> {
    source: &'s str,
    file: Option<toml::Spanned<toml::de::DeTable<'s>>>,
}

impl<'s> Lines<'s> {
    fn new(source: &'s str) -> Lines<'s> {
        Lines { source, file: toml::de::DeTable::parse(source).ok() }
    }

    /// The line of the key at `path` (`columns`, a column's name, a key
    /// of it), or of as much of the path as is there.
    fn key(&self, path: &[&str]) -> usize {
        let mut line = 1;
        let mut table = self.file.as_ref().map(|f| f.get_ref());
        for part in path {
            let Some(t) = table else { break };
            let Some((k, v)) = t.iter().find(|(k, _)| k.get_ref() == *part) else { break };
            line = crate::project_file::line_of(self.source, k.span().start);
            table = match v.get_ref() {
                toml::de::DeValue::Table(t) => Some(t),
                _ => None,
            };
        }
        line
    }

    /// The line of rule `i` (from 0).
    fn rule(&self, i: usize) -> usize {
        let rules = (self.file.as_ref())
            .and_then(|f| f.get_ref().iter().find(|(k, _)| k.get_ref() == "rules"));
        let at = rules.and_then(|(_, v)| match v.get_ref() {
            toml::de::DeValue::Array(a) => a.get(i).map(|r| r.span().start),
            _ => None,
        });
        at.map_or_else(|| self.key(&["rules"]), |at| crate::project_file::line_of(self.source, at))
    }
}

/// How a description's mistakes are taken: the first an error
/// ([`Description::parse`]), or each a note and the rest read
/// ([`Description::read`]); either way by its line.
struct Said<'s> {
    keep: bool,
    lines: Lines<'s>,
}

impl Said<'_> {
    /// A column or a rule that cannot be read.
    fn mistake(&self, notes: &mut Vec<String>, line: usize, message: String) -> Result<(), String> {
        let message = format!("line {line}: {message}");
        if !self.keep {
            return Err(message);
        }
        notes.push(message);
        Ok(())
    }

    /// What is not understood and leaves the rest as it reads.
    fn note(&self, notes: &mut Vec<String>, line: usize, message: String) {
        notes.push(if self.keep { format!("line {line}: {message}") } else { message });
    }
}

/// A column's mistake: the key of it that is wrong (none: the column
/// itself), and what is.
type Mistake = (&'static str, String);

impl Description {
    /// Reads a description. `script` reads a script's source by name
    /// (without `.nss`), for the lists that are a script's constants. A
    /// column or a rule that cannot be read is an error, by its line
    /// (`line 12: column Damage: "flag" is no kind…`), and nothing is
    /// described: what a check of the tables wants.
    pub fn parse(
        source: &str,
        script: &dyn Fn(&str) -> Option<Vec<u8>>,
    ) -> Result<Description, String> {
        Self::read_as(source, script, false)
    }

    /// [`Description::parse`] for the builder at work on the table: a
    /// column or a rule that cannot be read is a note by its line and is
    /// left out, and the rest of the description is read (one `kind`
    /// mistyped among forty columns left none of them described). Only a
    /// file that is no TOML at all is an error. Every note begins with
    /// its line.
    pub fn read(
        source: &str,
        script: &dyn Fn(&str) -> Option<Vec<u8>>,
    ) -> Result<Description, String> {
        Self::read_as(source, script, true)
    }

    fn read_as(
        source: &str,
        script: &dyn Fn(&str) -> Option<Vec<u8>>,
        keep: bool,
    ) -> Result<Description, String> {
        let file: toml::Table = source.parse().map_err(|e| format!("{e}"))?;
        let said = Said { keep, lines: Lines::new(source) };
        let mut d = Description { rows: text(&file, "rows"), ..Default::default() };
        d.tables = patterns(&file);
        for key in file.keys() {
            if !["rows", "columns", "rules", "tables"].contains(&key.as_str()) {
                let line = said.lines.key(&[key]);
                said.note(&mut d.notes, line, format!("\"{key}\" is not a key this reads"));
            }
        }
        let none = toml::Table::new();
        let columns = match file.get("columns") {
            Some(toml::Value::Table(t)) => t,
            Some(_) => {
                let message = "\"columns\" is not a table of columns".to_string();
                said.mistake(&mut d.notes, said.lines.key(&["columns"]), message)?;
                &none
            }
            None => &none,
        };
        for (name, column) in columns {
            let Some(c) = column.as_table() else {
                let message = format!("column {name}: not a table ([columns.{name}])");
                said.mistake(&mut d.notes, said.lines.key(&["columns", name]), message)?;
                continue;
            };
            match d.kind(name, c, script, &said) {
                Ok(kind) => d.columns.push((name.clone(), Column { kind, note: text(c, "note") })),
                Err((key, message)) => {
                    let line = said.lines.key(&["columns", name, key]);
                    said.mistake(&mut d.notes, line, message)?;
                }
            }
        }
        let rules: &[toml::Value] = match file.get("rules") {
            Some(toml::Value::Array(a)) => a,
            Some(_) => {
                let message = "\"rules\" is not a list ([[rules]])".to_string();
                said.mistake(&mut d.notes, said.lines.key(&["rules"]), message)?;
                &[]
            }
            None => &[],
        };
        for (i, rule) in rules.iter().enumerate() {
            let at = i + 1;
            let when = rule.get("when").and_then(|w| w.as_table());
            let column = when.and_then(|w| text(w, "column"));
            let is = when.and_then(|w| w.get("is")).and_then(cell_text);
            let require = rule.get("require").and_then(|r| r.as_str());
            match (column, is, require) {
                (Some(column), Some(is), Some(require)) => {
                    d.rules.push(Rule { column, is, require: require.to_string() });
                }
                _ => {
                    let message = format!(
                        "rule {at}: it reads `when = {{ column = \"…\", is = \"…\" }}` and \
                         `require = \"…\"`"
                    );
                    said.mistake(&mut d.notes, said.lines.rule(i), message)?;
                }
            }
        }
        Ok(d)
    }

    fn kind(
        &mut self,
        name: &str,
        c: &toml::Table,
        script: &dyn Fn(&str) -> Option<Vec<u8>>,
        said: &Said,
    ) -> Result<Kind, Mistake> {
        let int = |key: &str| c.get(key).and_then(|v| v.as_integer());
        let float = |key: &str| {
            c.get(key).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        };
        let kind = text(c, "kind").unwrap_or_else(|| "text".to_string());
        Ok(match kind.as_str() {
            "text" => Kind::Text,
            "int" => Kind::Int { min: int("min"), max: int("max") },
            "float" => Kind::Float { min: float("min"), max: float("max") },
            "strref" => Kind::StrRef,
            "script" => Kind::Script,
            "row" => match text(c, "table") {
                Some(table) => Kind::Row(table.trim_end_matches(".2da").to_ascii_lowercase()),
                None => {
                    let message = format!("column {name}: a row of which table? (table = \"…\")");
                    return Err(("kind", message));
                }
            },
            "list" | "flags" => {
                let entries = self.entries(name, c, script, said)?;
                if entries.is_empty() {
                    let message = format!(
                        "column {name}: no values (values = {{ NAME = 1 }}, or constants = \
                         \"script\" with prefix = \"…\")"
                    );
                    return Err(("kind", message));
                }
                match kind.as_str() {
                    "list" => Kind::List {
                        entries,
                        other: c.get("other").and_then(|v| v.as_bool()).unwrap_or(false),
                    },
                    _ => {
                        if let Some(e) = entries.iter().find(|e| whole(&e.value).is_none()) {
                            let message = format!(
                                "column {name}: the flag {} is not a whole number ({})",
                                e.name, e.value
                            );
                            return Err(("values", message));
                        }
                        Kind::Flags { entries }
                    }
                }
            }
            other => {
                let message = format!(
                    "column {name}: \"{other}\" is no kind (text, int, float, list, flags, row, \
                     strref)"
                );
                return Err(("kind", message));
            }
        })
    }

    /// A list's or flags' entries: those written, then a script's
    /// constants; ordered by value where the values are numbers.
    fn entries(
        &mut self,
        name: &str,
        c: &toml::Table,
        script: &dyn Fn(&str) -> Option<Vec<u8>>,
        said: &Said,
    ) -> Result<Vec<Entry>, Mistake> {
        let mut entries: Vec<Entry> = Vec::new();
        match c.get("values") {
            Some(toml::Value::Table(values)) => {
                for (n, v) in values {
                    let Some(value) = cell_text(v) else {
                        let message = format!("column {name}: the value of {n} is not a value");
                        return Err(("values", message));
                    };
                    entries.push(Entry { name: n.clone(), value });
                }
            }
            Some(_) => {
                let message =
                    format!("column {name}: values are written {{ NAME = 1, OTHER = 2 }}");
                return Err(("values", message));
            }
            None => {}
        }
        if let Some(from) = text(c, "constants") {
            let prefix = text(c, "prefix").unwrap_or_default();
            let from = from.trim_end_matches(".nss");
            let Some(source) = script(from) else {
                return Err((
                    "constants",
                    format!("column {name}: the script {from} was not found"),
                ));
            };
            let (found, unread) = constants_in(&source, &prefix, false, script);
            for e in found {
                if !entries.iter().any(|x| x.name == e.name) {
                    entries.push(e);
                }
            }
            if unread > 0 {
                let message = format!(
                    "column {name}: {unread} of {from}'s values that begin {prefix} are not \
                     numbers that can be worked out (a global given by a function as the \
                     script runs: no const can be, the compiler refuses it), and are left out"
                );
                let line = said.lines.key(&["columns", name, "constants"]);
                said.note(&mut self.notes, line, message);
            }
        }
        entries.sort_by(|a, b| match (whole(&a.value), whole(&b.value)) {
            (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.name.cmp(&b.name)),
            _ => std::cmp::Ordering::Equal,
        });
        Ok(entries)
    }

    /// A column's description, whatever the case of its name.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, c)| c)
    }

    /// What is wrong in the cells of `t` by this description: a value no
    /// list has, a flag no entry names, a number that is none or out of
    /// its range, a row that is not there (`rows_of`: how many rows a
    /// table has), a string that is not there, a cell empty where a rule
    /// wants it filled; and a row past the end of the table whose rows
    /// these are.
    pub fn faults(
        &self,
        t: &TwoDa,
        rows_of: &dyn Fn(&str) -> Option<usize>,
        tlk: TalkTables,
    ) -> Vec<CellFault> {
        let mut out = Vec::new();
        for (name, column) in &self.columns {
            let Some(at) = t.column(name) else { continue };
            let length = match &column.kind {
                Kind::Row(table) => rows_of(table),
                _ => None,
            };
            for row in 0..t.len() {
                let Some(cell) = t.cell(row, at) else { continue };
                if let Some(message) = cell_fault(&column.kind, cell, length, tlk) {
                    out.push(CellFault { row, column: at, message });
                }
            }
        }
        for rule in &self.rules {
            let (Some(when), Some(require)) = (t.column(&rule.column), t.column(&rule.require))
            else {
                continue;
            };
            let kind = self.column(&rule.column).map(|c| &c.kind);
            for row in 0..t.len() {
                let Some(cell) = t.cell(row, when) else { continue };
                let named = kind.and_then(|k| k.named(cell));
                let is = same(cell, &rule.is)
                    || named.is_some_and(|n| n.split(" | ").any(|n| n == rule.is));
                if is && t.cell(row, require).is_none() {
                    let message =
                        format!("empty, where {} is {}: it is to be filled", rule.column, rule.is);
                    out.push(CellFault { row, column: require, message });
                }
            }
        }
        if let Some(table) = &self.rows
            && let Some(length) = rows_of(table)
        {
            for row in length..t.len() {
                // (An empty row past the end is padding.)
                if let Some(column) = (0..t.columns().len()).find(|c| t.cell(row, *c).is_some()) {
                    let message = format!(
                        "this table's rows are {table}.2da's, which has {length}: there is \
                         no row {row} there"
                    );
                    out.push(CellFault { row, column, message });
                }
            }
        }
        out.sort_by_key(|f| (f.row, f.column));
        out
    }

    /// The columns described that the table does not have.
    pub fn missing_columns(&self, t: &TwoDa) -> Vec<&str> {
        let named = self.columns.iter().map(|(n, _)| n.as_str());
        let ruled = self.rules.iter().flat_map(|r| [r.column.as_str(), r.require.as_str()]);
        let mut out: Vec<&str> = Vec::new();
        for name in named.chain(ruled).filter(|n| t.column(n).is_none()) {
            if !out.contains(&name) {
                out.push(name);
            }
        }
        out
    }
}

fn cell_fault(kind: &Kind, cell: &str, length: Option<usize>, tlk: TalkTables) -> Option<String> {
    let outside = |low: bool, high: bool| match (low, high) {
        (true, _) => Some("below"),
        (_, true) => Some("above"),
        _ => None,
    };
    match kind {
        Kind::Text => None,
        Kind::Int { min, max } => match whole(cell) {
            None => Some(format!("\"{cell}\" is not a whole number")),
            Some(v) => outside(min.is_some_and(|m| v < m), max.is_some_and(|m| v > m))
                .map(|side| format!("{v} is {side} what the column takes ({})", kind.words())),
        },
        Kind::Float { min, max } => match cell.trim().parse::<f64>() {
            Err(_) => Some(format!("\"{cell}\" is not a number")),
            Ok(v) => outside(min.is_some_and(|m| v < m), max.is_some_and(|m| v > m))
                .map(|side| format!("{v} is {side} what the column takes ({})", kind.words())),
        },
        Kind::List { other: true, .. } => None,
        Kind::List { entries, .. } => (!entries.iter().any(|e| same(cell, &e.value)))
            .then(|| format!("\"{cell}\" is none of the column's {} values", entries.len())),
        Kind::Flags { entries } => match whole(cell) {
            None => Some(format!("\"{cell}\" is not a set of flags (a whole number)")),
            Some(bits) => {
                let (_, rest) = flags_of(entries, bits);
                (rest != 0).then(|| format!("{rest:#X} is no flag of the column's"))
            }
        },
        Kind::Row(table) => match whole(cell) {
            None => Some(format!("\"{cell}\" is not a row's number")),
            Some(v) => length
                .filter(|l| v < 0 || v as usize >= *l)
                .map(|l| format!("row {v} of {table}.2da doesn't exist: it has {l} rows")),
        },
        Kind::StrRef => match whole(cell).and_then(|v| u32::try_from(v).ok()) {
            None => Some(format!("\"{cell}\" is not a string's number")),
            Some(v) => strref_problem(v, tlk).map(|p| format!("string {v} doesn't exist: {p}")),
        },
        Kind::Script => mg_core::ResRef::from_str(cell.trim()).is_err().then(|| {
            format!("\"{cell}\" is not a script's name (16 letters, digits and _ at most)")
        }),
    }
}

#[cfg(test)]
mod tests {
    use mg_core::Codepage;

    use super::*;

    const DESCRIPTION: &str = r#"
rows = "spells"
author = "someone"

[columns.Level]
kind = "int"
min = -1
max = 10
note = "The spell's level, or -1"

[columns.Damage]
kind = "flags"
constants = "my_consts"
prefix = "DAMAGE_"

[columns.Shape]
kind = "list"
values = { Sphere = 0, Cone = 1, Cube = "0x02" }

[columns.Radius]
kind = "list"
values = { Small = 1.67, Large = 5 }
other = true

[columns.Class]
kind = "row"
table = "classes.2da"

[columns.Effect]
kind = "list"
values = { NONE = 0, CURE = 4 }

[[rules]]
when = { column = "Effect", is = "CURE" }
require = "Cures"
"#;

    const SCRIPT: &str = "// constants\nconst int DAMAGE_ACID = 0x0010;\nconst int DAMAGE_COLD \
                          = 0x0020;\nconst int DAMAGE_BOTH = DAMAGE_ACID | DAMAGE_COLD;\nconst \
                          int OTHER_THING = 1;\nint DAMAGE_FIRE = 0x0100;\nint DAMAGE_UNSET;\nconst int DAMAGE_ODD \
                          = Unknown();\n";

    fn script(name: &str) -> Option<Vec<u8>> {
        (name == "my_consts").then(|| SCRIPT.as_bytes().to_vec())
    }

    fn table(text: &str) -> TwoDa {
        TwoDa::parse(format!("2DA V2.0\n\n{text}").as_bytes(), Codepage::default()).unwrap()
    }

    #[test]
    fn a_description_is_read_with_its_lists() {
        let d = Description::parse(DESCRIPTION, &script).unwrap();
        assert_eq!(d.rows.as_deref(), Some("spells"));
        assert_eq!(d.column("level").unwrap().kind, Kind::Int { min: Some(-1), max: Some(10) });
        assert_eq!(d.column("Level").unwrap().note.as_deref(), Some("The spell's level, or -1"));
        // A script's constants by their prefix: the plain numbers of them.
        let damage = &d.column("Damage").unwrap().kind;
        let names: Vec<&str> = damage.entries().iter().map(|e| e.name.as_str()).collect();
        // (One made of two others comes to their sum; one that cannot be
        // worked out is left out, and said.)
        assert_eq!(names, ["DAMAGE_ACID", "DAMAGE_COLD", "DAMAGE_BOTH", "DAMAGE_FIRE"]);
        assert_eq!(damage.entries()[2].value, "48");
        let known = [("A", 0x10), ("B", 3)];
        assert_eq!(evaluate("A | 1 << B", &known), Some(0x18));
        assert_eq!(evaluate("(A + 2) * -B", &known), Some(-54));
        assert_eq!(evaluate("~0 & A", &known), Some(0x10));
        assert_eq!((evaluate("A | C", &known), evaluate("A |", &known)), (None, None));
        assert_eq!((evaluate("GetX()", &known), evaluate("(A", &known)), (None, None));
        // Brackets and signs without end (a crafted script's constant, read
        // on a worker's small stack): no value, and no stack overflow.
        let deep = |open: &str, close: &str, n: usize| {
            evaluate(&format!("{}1{}", open.repeat(n), close.repeat(n)), &[])
        };
        assert_eq!(
            (deep("(", ")", 60), deep("-", "", 60), deep("~", "", 2)),
            (Some(1), Some(1), Some(1))
        );
        let small = std::thread::Builder::new().stack_size(256 * 1024);
        let run = small.spawn(move || {
            [deep("(", ")", 100_000), deep("-", "", 100_000), evaluate(&"(".repeat(100_000), &[])]
        });
        assert_eq!(run.unwrap().join().unwrap(), [None, None, None]);
        assert_eq!(damage.named("0x0030").as_deref(), Some("DAMAGE_ACID | DAMAGE_COLD"));
        assert_eq!(damage.named("0x0050").as_deref(), Some("DAMAGE_ACID | 0x40"));
        assert_eq!(damage.flags_cell(0x30, None).as_deref(), Some("0x0030"));
        assert_eq!(damage.flags_cell(0x30, Some("16")).as_deref(), Some("48"), "as it was");
        assert_eq!(damage.flags_cell(0x30, Some("0x10")).as_deref(), Some("0x30"));
        assert_eq!(damage.flags_cell(0x30, Some("#N/A")).as_deref(), Some("0x0030"));
        assert_eq!(damage.flags_cell(0, None), None);
        // (The highest of 32 bits is a bit, and "0x" alone is no number.)
        assert_eq!((whole("0x80000000"), whole("2147483648")), (Some(1 << 31), Some(1 << 31)));
        assert_eq!(
            (whole("-3"), whole("0x"), whole("12a"), whole("")),
            (Some(-3), None, None, None)
        );
        assert_eq!(d.column("Class").unwrap().kind, Kind::Row("classes".into()));
        assert_eq!(d.column("Shape").unwrap().kind.named("2").as_deref(), Some("Cube"));
        // What it did not understand is said, and the rest reads.
        assert_eq!(d.notes.len(), 2, "{:?}", d.notes);
        assert!(d.notes[0].contains("author") && d.notes[1].contains("1 of my_consts"));
        // A flag of several bits is named where each is set.
        assert_eq!(
            damage.named("0x0130").as_deref(),
            Some("DAMAGE_ACID | DAMAGE_COLD | DAMAGE_FIRE"),
            "not DAMAGE_BOTH too"
        );
        assert_eq!(path_beside(Path::new("src/2da/a.2da")), Path::new("src/2da/a.2da.toml"));
    }

    #[test]
    fn a_table_is_held_to_its_description() {
        let d = Description::parse(DESCRIPTION, &script).unwrap();
        let t = table(
            "  Label Level Damage Shape Radius Class Effect Cures\n\
             0 a     3     0x0010 1     1.67   2     0      ****\n\
             1 b     11    0x0040 7     30     40    4      ****\n\
             2 c     #N/A  #N/A   #N/A  3.5    x     4      0x10\n\
             3 d     ****  ****   ****  ****   ****  ****   ****\n\
             4 e     1     ****   ****  ****   ****  ****   ****\n",
        );
        let rows_of = |name: &str| match name {
            "classes" => Some(11),
            "spells" => Some(4),
            _ => None,
        };
        let faults = d.faults(&t, &rows_of, TalkTables::default());
        let said: Vec<(usize, &str, &str)> = faults
            .iter()
            .map(|f| (f.row, t.columns()[f.column].as_str(), f.message.as_str()))
            .collect();
        let has = |row, column: &str, part: &str| {
            assert!(
                said.iter().any(|(r, c, m)| (*r, *c) == (row, column) && m.contains(part)),
                "row {row} {column}: {part} in {said:#?}"
            );
        };
        assert!(!said.iter().any(|(r, ..)| *r == 0 || *r == 3), "{said:#?}");
        has(1, "Level", "11 is above");
        has(1, "Damage", "0x40 is no flag");
        has(1, "Shape", "none of the column's 3 values");
        has(1, "Class", "row 40 of classes.2da doesn't exist");
        has(1, "Cures", "where Effect is CURE");
        has(2, "Level", "not a whole number");
        has(2, "Damage", "not a set of flags");
        has(2, "Shape", "none of");
        has(2, "Class", "not a row's number");
        has(4, "Label", "no row 4 there");
        assert_eq!(faults.len(), 10, "{said:#?}");
        assert_eq!(d.missing_columns(&table("  Label Level\n0 a 1\n")).len(), 6);
    }

    /// The manual's example is a description that reads, every kind of it.
    #[test]
    fn the_manual_s_example_reads() {
        // (A checkout may have given the file other line ends.)
        let manual = include_str!("../../../docs/manual/03-modules.md").replace("\r\n", "\n");
        let from = manual.find("### A table's description").expect("the manual's part");
        let block = &manual[from..];
        let start = block.find("```toml\n").expect("its example") + "```toml\n".len();
        let example = &block[start..start + block[start..].find("```").unwrap()];
        let nwscript =
            |name: &str| (name == "nwscript").then(|| b"int DAMAGE_TYPE_ACID = 16;\n".to_vec());
        let d = Description::parse(example, &nwscript).unwrap();
        assert!(d.notes.is_empty(), "{:?}", d.notes);
        assert_eq!((d.rows.as_deref(), d.columns.len(), d.rules.len()), (Some("spells"), 6, 1));
        let kinds: Vec<String> = d.columns.iter().map(|(_, c)| c.kind.words()).collect();
        for word in ["a number from -1 to 10", "any of 1 flags", "or another", "classes.2da"] {
            assert!(kinds.iter().any(|k| k.contains(word)), "{word} in {kinds:?}");
        }
        assert_eq!(d.column("Name").unwrap().kind, Kind::StrRef);
        assert_eq!(d.column("Script").unwrap().kind, Kind::Script);
    }

    /// A constant made of another script's that the script includes is
    /// worked out, and the included script's constants are listed too; a
    /// global given by a function is left out, and said so.
    #[test]
    fn constants_of_included_scripts_are_worked_out() {
        let scripts = |name: &str| match name {
            "my_flags" => Some(
                b"#include \"my_base\"\nconst int FLAG_BOTH = BASE_A | BASE_B;\nint FLAG_RUN = \
                  Foo();\n#include \"my_flags\"\n"
                    .to_vec(),
            ),
            "my_base" => Some(b"const int BASE_A = 1;\nconst int BASE_B = 0x4;\n".to_vec()),
            _ => None,
        };
        let d = Description::parse(
            "[columns.F]\nkind = \"flags\"\nconstants = \"my_flags\"\nprefix = \"\"\n",
            &scripts,
        )
        .unwrap();
        let entries = d.column("F").unwrap().kind.entries();
        let named: Vec<(&str, &str)> =
            entries.iter().map(|e| (e.name.as_str(), e.value.as_str())).collect();
        assert_eq!(named, [("BASE_A", "1"), ("BASE_B", "0x4"), ("FLAG_BOTH", "5")]);
        assert_eq!(d.notes.len(), 1, "{:?}", d.notes);
        assert!(d.notes[0].contains("1 of my_flags's values") && d.notes[0].contains("function"));
    }

    /// One description for a family of tables (`tables = ["loot_*"]`), in
    /// the folder: a table's own description comes before it, and the
    /// closest name before another family's.
    #[test]
    fn a_family_s_description_is_found_for_its_tables() {
        assert!(names_table("loot_*", "LOOT_gold") && names_table("loot_?", "loot_a"));
        assert!(!names_table("loot_*", "loo") && !names_table("loot_?", "loot_ab"));
        assert!(names_table("spells.2da", "spells") && names_table("*", "x"));
        let dir = mg_testkit::scratch_dir("table-desc-family");
        let write = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
        write("loot.2da.toml", "tables = [\"loot_*\"]\n[columns.Gold]\nkind = \"int\"\n");
        write("gold.2da.toml", "tables = \"loot_gold*\"\n[columns.Gold]\nkind = \"int\"\n");
        write("loot_own.2da.toml", "[columns.Gold]\nkind = \"text\"\n");
        for t in ["loot_a", "loot_gold1", "loot_own", "spells"] {
            write(&format!("{t}.2da"), "2DA V2.0\n\n   Gold\n0  1\n");
        }
        let found = |t: &str| find_for(&dir.join(format!("{t}.2da")));
        assert_eq!(found("loot_a"), Some(dir.join("loot.2da.toml")));
        assert_eq!(found("loot_gold1"), Some(dir.join("gold.2da.toml")), "the closest");
        assert_eq!(found("loot_own"), Some(dir.join("loot_own.2da.toml")), "its own");
        assert_eq!(found("spells"), None);
        let d = Description::parse(
            &std::fs::read_to_string(dir.join("loot.2da.toml")).unwrap(),
            &script,
        )
        .unwrap();
        assert_eq!((d.tables.as_slice(), d.notes.len()), (&["loot_*".to_string()][..], 0));
        // A folder's described tables, the family's among them.
        let described: Vec<String> = Beside::of(&dir)
            .described()
            .iter()
            .map(|(t, d)| {
                let name = |p: &PathBuf| p.file_name().unwrap().to_string_lossy().into_owned();
                format!("{} {}", name(t), name(d))
            })
            .collect();
        assert_eq!(
            described,
            [
                "loot_a.2da loot.2da.toml",
                "loot_gold1.2da gold.2da.toml",
                "loot_own.2da loot_own.2da.toml"
            ]
        );
    }

    /// Moonglow's descriptions of the game's own tables read, with no
    /// script (`nwscript`'s constants are written out in them).
    #[test]
    fn the_game_s_tables_moonglow_describes_read() {
        // (Read with no script: they need no game install.)
        let text = game_description("Spells.2da").expect("spells is described");
        let d = Description::parse(text, &|_| None).unwrap();
        assert!(d.notes.is_empty(), "{:?}", d.notes);
        assert_eq!(d.column("School").unwrap().kind.named("V").as_deref(), Some("Evocation"));
        assert_eq!(
            d.column("MetaMagic").unwrap().kind.named("0x09").as_deref(),
            Some("Empower | Quicken")
        );
        assert_eq!(d.column("Name").unwrap().kind, Kind::StrRef);
        let d = Description::parse(game_description("baseitems").unwrap(), &|_| None).unwrap();
        assert!(d.notes.is_empty(), "{:?}", d.notes);
        let slots = &d.column("EquipableSlots").unwrap().kind;
        assert_eq!(
            slots.named("0x1C030").as_deref(),
            Some("Main hand | Off-hand | Left claw | Right claw | Bite")
        );
        assert!(game_description("feat").is_none());
    }

    #[test]
    fn a_description_that_cannot_be_read_says_where() {
        let bad = |text: &str| Description::parse(text, &script).unwrap_err();
        assert!(bad("[columns.A]\nkind = \"bits\"").contains("column A: \"bits\" is no kind"));
        assert!(bad("[columns.A]\nkind = \"row\"").contains("which table"));
        assert!(bad("[columns.A]\nkind = \"list\"").contains("no values"));
        assert!(
            bad("[columns.A]\nkind = \"flags\"\nconstants = \"nowhere\"").contains("not found")
        );
        assert!(
            bad("[columns.A]\nkind = \"flags\"\nvalues = { X = \"a\" }").contains("not a whole")
        );
        assert!(bad("[[rules]]\nrequire = \"A\"").contains("rule 1"));
        assert!(!bad("[columns").is_empty());
        // By its line: the key that is wrong, else the column's or the
        // rule's own.
        assert!(bad("# mine\n[columns.A]\nnote = \"x\"\nkind = \"bits\"").starts_with("line 4: "));
        assert!(
            bad("[columns.A]\nkind = \"int\"\n\n[columns.B]\nkind = \"row\"")
                .starts_with("line 5: ")
        );
        assert!(
            bad("[columns.A]\nkind = \"list\"\nconstants = \"nowhere\"").starts_with("line 3: ")
        );
        assert!(bad("[columns]\nA = 3\n").starts_with("line 2: column A: not a table"));
        let rules = "[[rules]]\nwhen = { column = \"A\", is = 1 }\nrequire = \"B\"\n\n[[rules]]\n\
                     require = \"A\"\n";
        let line: usize = bad(rules)["line ".len()..].split(':').next().unwrap().parse().unwrap();
        assert!((5..=6).contains(&line), "{}", bad(rules));
    }

    /// For the builder at work on a table, one column mistyped among
    /// many is a note by its line and the rest is described (none was).
    #[test]
    fn a_description_read_for_its_builder_keeps_what_reads() {
        let text = "extra = 1\n\n[columns.Level]\nkind = \"int\"\nmin = 0\n\n[columns.Damage]\n\
                    kind = \"flag\"\n\n[columns.Shape]\nkind = \"list\"\nvalues = { Sphere = 0 }\n\n\
                    [[rules]]\nrequire = \"Level\"\n\n[[rules]]\nwhen = { column = \"Shape\", is = 0 }\n\
                    require = \"Level\"\n";
        assert!(
            Description::parse(text, &script).unwrap_err().starts_with("line 8: column Damage")
        );
        let d = Description::read(text, &script).unwrap();
        let names: Vec<&str> = d.columns.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Level", "Shape"]);
        assert_eq!(d.column("level").unwrap().kind, Kind::Int { min: Some(0), max: None });
        assert_eq!(d.rules.len(), 1);
        assert_eq!(d.notes.len(), 3, "{:?}", d.notes);
        assert_eq!(d.notes[0], "line 1: \"extra\" is not a key this reads");
        assert!(
            d.notes[1].starts_with("line 8: column Damage: \"flag\" is no kind"),
            "{:?}",
            d.notes
        );
        assert!(
            d.notes[2].starts_with("line 1") && d.notes[2].contains(": rule 1: "),
            "{:?}",
            d.notes
        );
        // One with nothing wrong reads the same either way, and what is
        // no TOML is an error either way.
        let strict = Description::parse(DESCRIPTION, &script).unwrap();
        let kept = Description::read(DESCRIPTION, &script).unwrap();
        assert_eq!(
            (&kept.columns, &kept.rules, &kept.rows),
            (&strict.columns, &strict.rules, &strict.rows)
        );
        assert!(Description::read("[columns", &script).is_err());
        assert!(Description::read("columns = 3\nrules = 4\n", &script).unwrap().notes.len() == 2);
    }
}
