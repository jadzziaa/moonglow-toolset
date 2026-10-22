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
                    Ok(t) if t.is_file() => files.push(e.path()),
                    _ => {}
                }
            }
        }
        files.sort();
        Beside { files }
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

    /// The tables of the folder that have a description beside them: each
    /// with its description's file.
    pub fn described(&self) -> Vec<(PathBuf, PathBuf)> {
        let mut out: Vec<(PathBuf, PathBuf)> = self
            .files
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("2da")))
            .map(|p| (p.clone(), path_beside(p)))
            .filter(|(_, d)| self.files.contains(d))
            .collect();
        out.sort();
        out
    }
}

/// A script's constants whose names begin with `prefix`, each with its
/// value: a number as written, or what an expression of whole numbers and
/// the script's other constants comes to (`A | B`, `1 << 3`); with `text`,
/// a string constant's text too. And how many could not be worked out (a
/// function's result).
pub(crate) fn constants(source: &[u8], prefix: &str, text: bool) -> (Vec<Entry>, usize) {
    let outline = mg_script::outline::outline(source);
    // (A constant may be made of others of its script: `A | B`.)
    let given = |g: &&mg_script::outline::Global| g.is_const || g.value.is_some();
    let mut known: Vec<(&str, i64)> = Vec::new();
    for g in outline.globals.iter().filter(given) {
        if let Some(v) = g.value.as_deref().and_then(|v| evaluate(v, &known)) {
            known.push((g.name.as_str(), v));
        }
    }
    let (mut entries, mut unread) = (Vec::new(), 0);
    // (Constants, and the globals given a value: the game's own
    // `nwscript` declares its constants so, `int DAMAGE_TYPE_ACID =
    // 16;`.)
    for g in outline.globals.iter().filter(given).filter(|g| g.name.starts_with(prefix)) {
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
/// reads them. `None` for anything else.
fn evaluate(text: &str, known: &[(&str, i64)]) -> Option<i64> {
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
    fn level(toks: &[Tok], at: &mut usize, depth: usize) -> Option<i64> {
        const LEVELS: [&[&str]; 6] = [&["|"], &["^"], &["&"], &["<<", ">>"], &["+", "-"], &["*"]];
        if depth == LEVELS.len() {
            return match toks.get(*at)? {
                Tok::Num(v) => {
                    *at += 1;
                    Some(*v)
                }
                Tok::Op("(") => {
                    *at += 1;
                    let v = level(toks, at, 0)?;
                    (toks.get(*at) == Some(&Tok::Op(")"))).then(|| *at += 1)?;
                    Some(v)
                }
                Tok::Op(op @ ("-" | "~" | "+")) => {
                    *at += 1;
                    let v = level(toks, at, depth)?;
                    Some(match *op {
                        "-" => v.checked_neg()?,
                        "~" => !v & 0xFFFF_FFFF,
                        _ => v,
                    })
                }
                Tok::Op(_) => None,
            };
        }
        let mut left = level(toks, at, depth + 1)?;
        while let Some(Tok::Op(op)) = toks.get(*at)
            && LEVELS[depth].contains(op)
        {
            *at += 1;
            let right = level(toks, at, depth + 1)?;
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
    let value = level(&toks, &mut at, 0)?;
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

impl Description {
    /// Reads a description. `script` reads a script's source by name
    /// (without `.nss`), for the lists that are a script's constants.
    pub fn parse(
        source: &str,
        script: &dyn Fn(&str) -> Option<Vec<u8>>,
    ) -> Result<Description, String> {
        let file: toml::Table = source.parse().map_err(|e| format!("{e}"))?;
        let mut d = Description { rows: text(&file, "rows"), ..Default::default() };
        for key in file.keys() {
            if !["rows", "columns", "rules"].contains(&key.as_str()) {
                d.notes.push(format!("\"{key}\" is not a key this reads"));
            }
        }
        let columns = match file.get("columns") {
            Some(toml::Value::Table(t)) => t.clone(),
            Some(_) => return Err("\"columns\" is not a table of columns".to_string()),
            None => toml::Table::new(),
        };
        for (name, column) in &columns {
            let Some(c) = column.as_table() else {
                return Err(format!("column {name}: not a table ([columns.{name}])"));
            };
            let kind = d.kind(name, c, script)?;
            d.columns.push((name.clone(), Column { kind, note: text(c, "note") }));
        }
        let rules = match file.get("rules") {
            Some(toml::Value::Array(a)) => a.clone(),
            Some(_) => return Err("\"rules\" is not a list ([[rules]])".to_string()),
            None => Vec::new(),
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
                    return Err(format!(
                        "rule {at}: it reads `when = {{ column = \"…\", is = \"…\" }}` and \
                         `require = \"…\"`"
                    ));
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
    ) -> Result<Kind, String> {
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
            "row" => match text(c, "table") {
                Some(table) => Kind::Row(table.trim_end_matches(".2da").to_ascii_lowercase()),
                None => {
                    return Err(format!("column {name}: a row of which table? (table = \"…\")"));
                }
            },
            "list" | "flags" => {
                let entries = self.entries(name, c, script)?;
                if entries.is_empty() {
                    return Err(format!(
                        "column {name}: no values (values = {{ NAME = 1 }}, or constants = \
                         \"script\" with prefix = \"…\")"
                    ));
                }
                match kind.as_str() {
                    "list" => Kind::List {
                        entries,
                        other: c.get("other").and_then(|v| v.as_bool()).unwrap_or(false),
                    },
                    _ => {
                        if let Some(e) = entries.iter().find(|e| whole(&e.value).is_none()) {
                            return Err(format!(
                                "column {name}: the flag {} is not a whole number ({})",
                                e.name, e.value
                            ));
                        }
                        Kind::Flags { entries }
                    }
                }
            }
            other => {
                return Err(format!(
                    "column {name}: \"{other}\" is no kind (text, int, float, list, flags, row, \
                     strref)"
                ));
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
    ) -> Result<Vec<Entry>, String> {
        let mut entries: Vec<Entry> = Vec::new();
        match c.get("values") {
            Some(toml::Value::Table(values)) => {
                for (n, v) in values {
                    let Some(value) = cell_text(v) else {
                        return Err(format!("column {name}: the value of {n} is not a value"));
                    };
                    entries.push(Entry { name: n.clone(), value });
                }
            }
            Some(_) => {
                return Err(format!("column {name}: values are written {{ NAME = 1, OTHER = 2 }}"));
            }
            None => {}
        }
        if let Some(from) = text(c, "constants") {
            let prefix = text(c, "prefix").unwrap_or_default();
            let from = from.trim_end_matches(".nss");
            let Some(source) = script(from) else {
                return Err(format!("column {name}: the script {from} was not found"));
            };
            let (found, unread) = constants(&source, &prefix, false);
            for e in found {
                if !entries.iter().any(|x| x.name == e.name) {
                    entries.push(e);
                }
            }
            if unread > 0 {
                self.notes.push(format!(
                    "column {name}: {unread} of {from}'s constants that begin {prefix} are \
                     not numbers that can be worked out, and are left out"
                ));
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
        assert_eq!((d.rows.as_deref(), d.columns.len(), d.rules.len()), (Some("spells"), 5, 1));
        let kinds: Vec<String> = d.columns.iter().map(|(_, c)| c.kind.words()).collect();
        for word in ["a number from -1 to 10", "any of 1 flags", "or another", "classes.2da"] {
            assert!(kinds.iter().any(|k| k.contains(word)), "{word} in {kinds:?}");
        }
        assert_eq!(d.column("Name").unwrap().kind, Kind::StrRef);
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
    }
}
