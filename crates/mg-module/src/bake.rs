//! Bake Walkmesh to Transform: a placed placeable that blocks as it is
//! drawn (`docs/walkmesh-bake-proposal.md`).
//!
//! The engine never moves a placeable's walkmesh by its visual transform
//! (`walkmeshes_by_visual_transforms` in `engine_placeable_walkmesh.rs`):
//! a large stone scaled down to sit on a desk still blocks the floor
//! around the desk. Only another model, with another walkmesh, can block
//! as it is drawn. So a bake makes, for one model and one transform:
//!
//! - **a copy of the model** under a new name (`mgbk_0001`, from the
//!   project's `[bake] names`): the same geometry, animations and textures,
//!   only its own name changed, compiled again ([`copy_model`]);
//! - **its walkmesh** (`mgbk_0001.pwk`): the original's faces and points
//!   moved by the object's visual transform as the game draws the model
//!   ([`mg_mdl::walkmesh::Walkmesh::moved_text`] of the matrix the caller
//!   gives: `mg_area::VisualTransform::matrix`);
//! - **a placeables.2da row** that is the original's with that model, in
//!   the project's rows file ([`RowsFile`], `baked_rows.toml`), which the
//!   project's own build adds to the table it writes; the object's
//!   `Appearance` then names it, and its visual transform stays as it is.
//!
//! The models go in the project's hak (`[bake] hak`), which the module
//! lists. A row is never one the game's table (the haks', the game's)
//! already holds, and is at most 65535: the engine and the network carry
//! the row as a 16-bit number (65536 is read as 0).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use mg_2da::TwoDa;
use mg_core::{Codepage, ResRef, ResType};
use mg_mdl::walkmesh::{Walkmesh, WalkmeshKind};
use mg_resman::{MemContainer, ResKey, ResMan, priority};

use crate::Module;
use crate::placement::Transform;

/// The last placeables.2da row the game can use: the engine and the
/// network carry an object's row as a 16-bit number, and read 65536 as 0,
/// 65537 as 1 (another model, silently).
pub const LAST_ROW: u32 = 65535;

/// `[bake] names` when the project file names none.
pub const DEFAULT_NAMES: &str = "mgbk_";

/// `[bake] rows_file` when the project file names none.
pub const DEFAULT_ROWS_FILE: &str = "baked_rows.toml";

/// The label of the layer Moonglow reads the rows file's rows from while
/// the project's build hasn't added them yet ([`pending_layer`]): what
/// the build will make, drawn as it will be.
pub const PENDING_LABEL: &str = "bake:rows";

/// The priority of [`PENDING_LABEL`]'s layer: over the haks (whose
/// placeables.2da it carries), under a development folder.
pub const PENDING_PRIORITY: u32 = priority::HAK_USER + 1;

/// A model's name is at most this long (a resource name).
const NAME_LEN: usize = 16;

/// `[bake] rows`: the placeables.2da rows baked models take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rows {
    pub from: u32,
    pub to: u32,
}

impl Rows {
    /// Reads `"16500-16999"` (an en dash or `_` between thousands too),
    /// or one row. Past [`LAST_ROW`] is refused.
    pub fn parse(text: &str) -> Result<Rows, String> {
        let t: String = text.trim().chars().filter(|c| *c != '_').collect();
        let (a, b) = t.split_once(['-', '\u{2013}']).unwrap_or((&t, &t));
        let n = |s: &str| {
            s.trim().parse::<u64>().map_err(|_| format!("\"{text}\" is not rows: \"16500-16999\""))
        };
        let (from, to) = (n(a)?, n(b)?);
        if from > to {
            return Err(format!("\"{text}\": the first row is after the last"));
        }
        if to > u64::from(LAST_ROW) {
            return Err(format!(
                "\"{text}\" reaches past row {LAST_ROW}, the last the game can use: it carries \
                 an object's row as a 16-bit number and reads 65536 as row 0 (another model)"
            ));
        }
        Ok(Rows { from: from as u32, to: to as u32 })
    }

    pub fn contains(&self, row: u32) -> bool {
        (self.from..=self.to).contains(&row)
    }
}

impl std::fmt::Display for Rows {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

/// `[bake]`: where baked models and their rows go. Without the section
/// the command isn't offered.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BakeRules {
    /// The project file has `[bake]`.
    pub given: bool,
    /// `rows`: the placeables.2da rows baked models take.
    pub rows: Option<Rows>,
    /// `hak`: the hak (its name, without `.hak`) the models go in.
    pub hak: Option<String>,
    /// `names`: what baked models' names begin with (none:
    /// [`DEFAULT_NAMES`]).
    pub names: Option<String>,
    /// `rows_file`: the rows, beside the project file (none:
    /// [`DEFAULT_ROWS_FILE`]).
    pub rows_file: Option<String>,
    /// What is wrong with the section as written (its warnings, each with
    /// the file and line): while there is any, nothing is baked.
    pub problems: Vec<String>,
}

/// [`BakeRules`] that can be baked by.
#[derive(Debug, Clone, PartialEq)]
pub struct Ready {
    pub rows: Rows,
    pub hak: String,
    pub names: String,
    pub rows_file: String,
}

impl BakeRules {
    /// What the section lacks, said as the project file's warnings are
    /// (the keys it must have).
    pub fn missing(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.given {
            return out;
        }
        if self.rows.is_none() {
            out.push("[bake] has no rows: the placeables.2da rows baked models take".into());
        }
        if self.hak.is_none() {
            out.push("[bake] has no hak: the hak baked models go in".into());
        }
        out
    }

    /// The rules to bake by, or why there are none (the tip of a command
    /// not offered).
    pub fn ready(&self) -> Result<Ready, String> {
        if !self.given {
            return Err("The project file (moonglow.toml) has no [bake] section: it says which \
                        placeables.2da rows and which hak baked models take"
                .into());
        }
        if let Some(p) = self.problems.first() {
            return Err(format!("The project file's [bake] is not right: {p}"));
        }
        if let Some(p) = self.missing().first() {
            return Err(format!("The project file's {p}"));
        }
        Ok(Ready {
            rows: self.rows.expect("checked"),
            hak: self.hak.clone().expect("checked"),
            names: self.names.clone().unwrap_or_else(|| DEFAULT_NAMES.into()),
            rows_file: self.rows_file.clone().unwrap_or_else(|| DEFAULT_ROWS_FILE.into()),
        })
    }

    /// The rows file's path in `dir` (the project file's folder).
    pub fn rows_path(&self, dir: &Path) -> Option<PathBuf> {
        self.given.then(|| dir.join(self.rows_file.as_deref().unwrap_or(DEFAULT_ROWS_FILE)))
    }

    /// Why `names` can't begin baked models' names, if it can't.
    pub fn names_problem(names: &str) -> Option<String> {
        if names.is_empty() {
            return Some("names is empty: baked models' names begin with it (\"mgbk_\")".into());
        }
        if let Some(c) = names.chars().find(|c| !(c.is_ascii_alphanumeric() || *c == '_')) {
            return Some(format!(
                "names \"{names}\": \"{c}\": a model's name is letters, digits and _"
            ));
        }
        if names.len() + 4 > NAME_LEN {
            return Some(format!(
                "names \"{names}\" is too long: with four digits after it a model's name is at \
                 most {NAME_LEN} characters"
            ));
        }
        None
    }
}

/// A row of the rows file: a placeables.2da row the project's build adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakedRow {
    /// The row it is, in `[bake] rows`.
    pub row: u32,
    /// The row it copies: every column of it, but `ModelName` and `Label`.
    pub copy_of: u32,
    /// Its `ModelName`: the baked model.
    pub model: String,
    /// Its `Label`.
    pub label: String,
}

/// The rows file (`baked_rows.toml`): the rows baked models take, written
/// by Moonglow, read by the project's build.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowsFile {
    /// By row.
    pub rows: Vec<BakedRow>,
}

impl RowsFile {
    /// Reads the file's text (`name` for messages): an error says what and
    /// where. A file Moonglow can't read whole is not written over.
    pub fn parse(name: &str, text: &str) -> Result<RowsFile, String> {
        let table: toml::Table = text.parse().map_err(|e: toml::de::Error| {
            let line = e.span().map_or(1, |s| crate::project_file::line_of(text, s.start));
            format!("{name}:{line}: {}", e.message().trim())
        })?;
        let mut rows: Vec<BakedRow> = Vec::new();
        for (key, value) in &table {
            if key != "row" {
                return Err(format!("{name}: \"{key}\" is not a part of a rows file: [[row]] is"));
            }
            let toml::Value::Array(list) = value else {
                return Err(format!("{name}: row is a list of tables: [[row]]"));
            };
            for (i, entry) in list.iter().enumerate() {
                let at = format!("{name}: row {}", i + 1);
                let toml::Value::Table(t) = entry else {
                    return Err(format!("{at}: not a table ([[row]])"));
                };
                let number = |k: &str| -> Result<u32, String> {
                    match t.get(k) {
                        Some(toml::Value::Integer(n)) if (0..=i64::from(LAST_ROW)).contains(n) => {
                            Ok(*n as u32)
                        }
                        Some(v) => Err(format!("{at}: {k} {v} is not a row from 0 to {LAST_ROW}")),
                        None => Err(format!("{at}: no {k}")),
                    }
                };
                let text = |k: &str| -> Result<String, String> {
                    match t.get(k) {
                        Some(toml::Value::String(s)) => Ok(s.clone()),
                        Some(v) => Err(format!("{at}: {k} {v} is not text in quotes")),
                        None => Err(format!("{at}: no {k}")),
                    }
                };
                if let Some(k) =
                    t.keys().find(|k| !matches!(k.as_str(), "row" | "copy_of" | "model" | "label"))
                {
                    return Err(format!(
                        "{at}: \"{k}\" is not a key of a row (row, copy_of, model, label)"
                    ));
                }
                let row = BakedRow {
                    row: number("row")?,
                    copy_of: number("copy_of")?,
                    model: text("model")?,
                    label: text("label")?,
                };
                let named = |c: char| c.is_ascii_alphanumeric() || c == '_';
                if ResRef::from_str(&row.model).is_err()
                    || row.model.is_empty()
                    || !row.model.chars().all(named)
                {
                    return Err(format!("{at}: model \"{}\" is not a model's name", row.model));
                }
                if rows.iter().any(|r| r.row == row.row) {
                    return Err(format!("{at}: row {} is in the file twice", row.row));
                }
                rows.push(row);
            }
        }
        rows.sort_by_key(|r| r.row);
        Ok(RowsFile { rows })
    }

    /// Reads the file at `path`: none where it isn't there yet.
    pub fn read(path: &Path) -> Result<Option<RowsFile>, String> {
        let name = path.file_name().map_or(DEFAULT_ROWS_FILE.into(), |n| n.to_string_lossy());
        match std::fs::read(path) {
            Ok(bytes) => RowsFile::parse(&name, &String::from_utf8_lossy(&bytes)).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    /// The file's text, named `name` in its first line.
    pub fn text(&self, name: &str) -> String {
        let mut out = format!(
            "# {name}: written by Moonglow Toolset (Bake Walkmesh to Transform); the project's\n\
             # build adds each row to the placeables.2da it writes. Edit it in Moonglow, not by hand.\n"
        );
        for r in &self.rows {
            let quoted = |s: &str| toml::Value::String(s.into()).to_string();
            let _ = write!(
                out,
                "\n[[row]]\nrow = {}\ncopy_of = {}\nmodel = {}\nlabel = {}\n",
                r.row,
                r.copy_of,
                quoted(&r.model),
                quoted(&r.label)
            );
        }
        out
    }

    /// Writes the file's text at `path` (its folder made if need be).
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let name = path.file_name().map_or(DEFAULT_ROWS_FILE.into(), |n| n.to_string_lossy());
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(path, self.text(&name)).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Row `row`, if the file has it.
    pub fn row(&self, row: u32) -> Option<&BakedRow> {
        self.rows.iter().find(|r| r.row == row)
    }

    /// The row whose model is `model` (any case).
    pub fn of_model(&self, model: &str) -> Option<&BakedRow> {
        self.rows.iter().find(|r| r.model.eq_ignore_ascii_case(model))
    }

    fn insert(&mut self, row: BakedRow) {
        self.rows.retain(|r| r.row != row.row);
        self.rows.push(row);
        self.rows.sort_by_key(|r| r.row);
    }
}

/// The placeables.2da the game reads with these resources, but for the
/// module file's own copy and [`PENDING_LABEL`]'s: the haks' or the
/// game's, and the layer it is read from.
pub fn table_under(rm: &ResMan, codepage: Codepage) -> Result<(TwoDa, String), String> {
    let key = crate::lights::placeables_key();
    let layer = (rm.layers().iter())
        .find(|l| {
            l.priority != priority::MODULE && l.label != PENDING_LABEL && l.container.contains(&key)
        })
        .ok_or("the game data has no placeables.2da")?;
    let data = layer.container.read(&key).map_err(|e| format!("placeables.2da: {e}"))?;
    let table = TwoDa::parse(&data, codepage).map_err(|e| format!("placeables.2da: {e}"))?;
    Ok((table, layer.label.clone()))
}

/// Whether `table` holds row `row`: it has a model or a label there (a
/// row of `****` alone, a padded table's, is free).
pub fn holds(table: &TwoDa, row: u32) -> bool {
    let row = row as usize;
    row < table.len()
        && (table.get(row, "ModelName").is_some() || table.get(row, "Label").is_some())
}

/// How a rows file's row stands in the game's table ([`table_under`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowState {
    /// The table has it, with its model: the build added it.
    Added,
    /// The table lacks it: the build hasn't added it yet.
    Pending,
    /// The table has another model there (its `ModelName`, if any).
    Other(Option<String>),
}

/// How `row` stands in `table`.
pub fn row_state(table: &TwoDa, row: &BakedRow) -> RowState {
    if !holds(table, row.row) {
        return RowState::Pending;
    }
    match table.get(row.row as usize, "ModelName") {
        Some(m) if m.eq_ignore_ascii_case(&row.model) => RowState::Added,
        other => RowState::Other(other.map(str::to_string)),
    }
}

/// A placeables.2da of `table` (the game's, [`table_under`]) with the rows
/// file's rows it lacks added: what the project's build will make, for
/// Moonglow to draw baked objects as they will be drawn while it hasn't
/// run (none where it lacks none). A row the table has with another model
/// stays as the game has it.
pub fn pending_table(table: &TwoDa, rows: &RowsFile) -> Option<TwoDa> {
    let pending: Vec<&BakedRow> =
        rows.rows.iter().filter(|r| row_state(table, r) == RowState::Pending).collect();
    if pending.is_empty() {
        return None;
    }
    let mut out = table.clone();
    for r in pending {
        while out.len() <= r.row as usize {
            out.add_row();
        }
        for c in 0..out.columns().len() {
            let name = out.columns()[c].clone();
            let value = match name.to_ascii_lowercase().as_str() {
                "modelname" => Some(r.model.clone()),
                "label" => Some(r.label.replace(' ', "_")),
                _ => table.cell(r.copy_of as usize, c).map(str::to_string),
            };
            let _ = out.set(r.row as usize, c, value);
        }
    }
    Some(out)
}

/// [`pending_table`] as a layer's container ([`PENDING_LABEL`]), of the
/// game data `rm`: none where the table lacks no row of the file.
pub fn pending_layer(rm: &ResMan, codepage: Codepage, rows: &RowsFile) -> Option<MemContainer> {
    let (table, _) = table_under(rm, codepage).ok()?;
    let table = pending_table(&table, rows)?;
    let bytes = table.to_bytes_as(codepage, mg_2da::Layout::Aligned).ok()?;
    let mut c = MemContainer::new();
    c.insert(crate::lights::placeables_key(), bytes);
    Some(c)
}

/// Puts [`pending_layer`] over `rm`'s haks (or takes it out, where nothing
/// is pending): whether it was there or is now.
pub fn layer_pending(rm: &mut ResMan, codepage: Codepage, rows: Option<&RowsFile>) -> bool {
    let had = rm.remove(PENDING_LABEL).is_some();
    let layer = rows.and_then(|r| pending_layer(rm, codepage, r));
    let has = layer.is_some();
    if let Some(c) = layer {
        rm.add(PENDING_PRIORITY, PENDING_LABEL, mg_resman::LayerClass::Erf, c);
    }
    had || has
}

/// A copy of model `original` (found in `rm`) under the name `to`: the
/// same model, its own name changed ([`mg_mdl::Model::renamed`]), compiled
/// again against its supermodel, so that its nodes keep their part
/// numbers and a supermodel's animations still find them. A model kept as
/// text is compiled first.
pub fn copy_model(rm: &ResMan, original: &str, to: &str) -> Result<Vec<u8>, String> {
    let find = |name: &str| rm.get_named(name, ResType::MDL).ok().map(|d| d.into_owned());
    let material = |name: &str| rm.get_named(name, ResType::MTR).ok().map(|d| d.into_owned());
    let data = find(original).ok_or_else(|| format!("{original}.mdl is nowhere"))?;
    let binary = if mg_mdl::is_binary(&data) {
        data
    } else {
        let none = |_: &str| None;
        mg_mdl::compile::compile_named_with(&data, original, &find, &none, &material)
            .map_err(|e| format!("{original}.mdl: {e}"))?
            .binary
    };
    let model = mg_mdl::Model::read(&binary).map_err(|e| format!("{original}.mdl: {e}"))?;
    let parts = mg_mdl::binary_write::PartNumbers::of(&model);
    let order = match &parts {
        Some(p) => mg_mdl::ascii_write::file_order(&model, &p.numbers),
        None => (0..model.nodes.len()).collect(),
    };
    let copy = model.renamed(to);
    let opts = mg_mdl::ascii_write::Options::default();
    let text = mg_mdl::ascii_write::to_ascii_in_order(&copy, &opts, &order);
    let none = |_: &str| None;
    let compiled =
        mg_mdl::compile::compile_named_with(text.as_bytes(), to, &find, &none, &material)
            .map_err(|e| format!("{to}.mdl (a copy of {original}): {e}"))?;
    Ok(compiled.binary)
}

/// Model `original`'s walkmesh (`.pwk`, found in `rm`), read; why there is
/// nothing to bake where it has none or it is empty.
pub fn walkmesh_of(rm: &ResMan, original: &str) -> Result<Walkmesh, String> {
    let none = || {
        format!("{original} has no walkmesh (.pwk): it blocks nothing, so there is nothing to bake")
    };
    let data = rm.get_named(original, ResType::PWK).map_err(|_| none())?;
    let w = Walkmesh::read(&data, WalkmeshKind::Placeable)
        .map_err(|e| format!("{original}.pwk: {e}"))?;
    if w.object_faces(None).is_empty() {
        return Err(none());
    }
    Ok(w)
}

/// `faces` moved by `to` (a matrix, column by column).
pub fn moved(faces: &[([[f32; 3]; 3], u32)], to: [[f32; 4]; 4]) -> Vec<[[f32; 3]; 3]> {
    let apply = |p: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|r| to[0][r] * p[0] + to[1][r] * p[1] + to[2][r] * p[2] + to[3][r])
    };
    faces.iter().map(|(c, _)| c.map(apply)).collect()
}

/// The visual transform a placed placeable `s` is baked to, or why it
/// can't be: a static placeable is drawn without one, and one without a
/// transform has nothing to bake to.
pub fn transform_of(s: &mg_gff::Struct) -> Result<Transform, String> {
    if s.integer("Static").unwrap_or(0) != 0 {
        return Err("it is static, so the game draws it without its visual transform (untick \
                    Static first)"
            .into());
    }
    match crate::placement::visual_transform(s) {
        Some(t) if !t.is_identity() => Ok(t),
        _ => {
            Err("it has no visual transform (Adjust Location): its walkmesh is as it is drawn"
                .into())
        }
    }
}

/// The row a baked object's `Appearance` goes back to (the row it copies),
/// or why it has none: it isn't a row of the rows file.
pub fn unbaked_row(rows: &RowsFile, appearance: u32) -> Result<u32, String> {
    rows.row(appearance).map(|r| r.copy_of).ok_or_else(|| {
        format!("its Appearance, row {appearance}, is no baked row (of the rows file): not baked")
    })
}

/// One object to bake.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Its `Appearance` (a baked row's is baked from the row it copies).
    pub appearance: u32,
    /// Its visual transform (for the row's label).
    pub transform: Transform,
    /// The transform as the game draws the model
    /// (`mg_area::VisualTransform::matrix`), column by column.
    pub matrix: [[f32; 4]; 4],
}

/// What baking one object makes, or takes again.
#[derive(Debug, Clone, PartialEq)]
pub struct Bake {
    /// The row the object's `Appearance` becomes.
    pub row: u32,
    /// The row it copies.
    pub copy_of: u32,
    /// The baked model.
    pub model: String,
    /// The model it is a copy of.
    pub original: String,
    pub label: String,
    /// An earlier bake of the same model and transform is taken again.
    pub reused: bool,
    /// The original's walkmesh's faces, in the object's space.
    pub before: Vec<[[f32; 3]; 3]>,
    /// The baked one's.
    pub after: Vec<[[f32; 3]; 3]>,
}

/// What baking some objects will do, worked out before anything is
/// written.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// Each request's bake, or why it can't be baked.
    pub each: Vec<Result<Bake, String>>,
    /// The models to write into the hak (`.mdl`, `.pwk`).
    pub files: Vec<(ResKey, Vec<u8>)>,
    /// The rows file as it will be.
    pub rows: RowsFile,
    /// Whether the rows file changes.
    pub rows_changed: bool,
    pub hak: String,
    /// The rows file's name.
    pub rows_file: String,
}

impl Plan {
    /// What the window says before anything is written.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        for b in self.each.iter() {
            let line = match b {
                Ok(b) if b.reused => format!(
                    "{} is baked already as {} (row {}): taken again.",
                    b.original, b.model, b.row
                ),
                Ok(b) => format!(
                    "{} is copied as {}, its walkmesh moved as the model is drawn; row {} \
                     (a copy of row {}, \"{}\").",
                    b.original, b.model, b.row, b.copy_of, b.label
                ),
                Err(e) => format!("Not baked: {e}."),
            };
            out.push_str(&line);
            out.push('\n');
        }
        if !self.files.is_empty() {
            let _ = writeln!(
                out,
                "Writes {} into {}.hak, which the module lists.",
                self.files.iter().map(|(k, _)| k.to_string()).collect::<Vec<_>>().join(", "),
                self.hak
            );
        }
        if self.rows_changed {
            let _ = writeln!(
                out,
                "Adds the rows to {}, for the project's build to add to placeables.2da: until \
                 it has, the game doesn't have them (Verify says so: bake-row-pending).",
                self.rows_file
            );
        }
        out.trim_end().to_string()
    }

    /// The bakes that can be made.
    pub fn bakes(&self) -> impl Iterator<Item = &Bake> {
        self.each.iter().filter_map(|b| b.as_ref().ok())
    }
}

/// The largest distance of a corner from the object's origin.
fn reach(faces: &[[[f32; 3]; 3]]) -> f32 {
    faces
        .iter()
        .flatten()
        .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
        .fold(0.0, f32::max)
}

/// Whether two bakes of one walkmesh are of the same transform: every
/// corner within a millimeter and a tenth of a degree (at its reach) of
/// the other's.
pub fn same_bake(a: &[[[f32; 3]; 3]], b: &[[[f32; 3]; 3]]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let tolerance = 0.001 + reach(a).max(reach(b)) * 0.1f32.to_radians();
    a.iter().flatten().zip(b.iter().flatten()).all(|(p, q)| {
        let d = [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() <= tolerance
    })
}

/// A number as a label has it: at most two decimals.
fn short(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// A baked row's label: `Baked: Boulder 03 at 0.25`, and its turn and
/// move where it has them.
pub fn label(original: &str, t: &Transform) -> String {
    let mut parts = Vec::new();
    if t.scale[0] != 1.0 {
        parts.push(format!("at {}", short(t.scale[0])));
    }
    let three = |v: [f32; 3]| v.map(short).join(" ");
    if t.rotate != [0.0; 3] {
        parts.push(format!("turned {}", three(t.rotate)));
    }
    if t.translate != [0.0; 3] {
        parts.push(format!("moved {}", three(t.translate)));
    }
    let name = original.replace('_', " ");
    match parts.is_empty() {
        true => format!("Baked: {name}"),
        false => format!("Baked: {name} {}", parts.join(", ")),
    }
}

/// Works out baking `requests` by `rules` against the game data `rm` (the
/// module's haks layered, as the game reads them) and the rows file as it
/// is (`rows`): for each, an earlier bake of the same model and transform
/// (within a millimeter and a tenth of a degree: [`same_bake`]), among the
/// rows file's and the others', is taken again; else the next free name
/// and row of the project's are taken, never a row the game's table holds.
pub fn plan(
    rules: &Ready,
    rows: &RowsFile,
    rm: &ResMan,
    codepage: Codepage,
    requests: &[Request],
) -> Result<Plan, String> {
    let (table, _) = table_under(rm, codepage)?;
    let mut after = rows.clone();
    let mut files: Vec<(ResKey, Vec<u8>)> = Vec::new();
    let mut each = Vec::new();
    // Bakes made here: their row, copy and walkmesh.
    let mut made: Vec<Bake> = Vec::new();
    // Earlier bakes' walkmeshes, read once.
    let mut earlier: BTreeMap<String, Option<Vec<[[f32; 3]; 3]>>> = BTreeMap::new();
    for req in requests {
        let copy_of = rows.row(req.appearance).map_or(req.appearance, |r| r.copy_of);
        let bake = (|| -> Result<Bake, String> {
            let original = table
                .get(copy_of as usize, "ModelName")
                .ok_or_else(|| format!("row {copy_of} of placeables.2da has no model"))?
                .to_ascii_lowercase();
            let w = walkmesh_of(rm, &original)?;
            let faces = w.object_faces(None);
            let before = moved(&faces, IDENTITY);
            let baked = moved(&faces, req.matrix);
            // The same bake made here, or before.
            if let Some(b) =
                made.iter().find(|b| b.copy_of == copy_of && same_bake(&b.after, &baked))
            {
                return Ok(Bake { reused: true, before, ..b.clone() });
            }
            for r in rows.rows.iter().filter(|r| r.copy_of == copy_of) {
                let theirs = earlier
                    .entry(r.model.to_ascii_lowercase())
                    .or_insert_with(|| {
                        let data = rm.get_named(&r.model, ResType::PWK).ok()?;
                        let w = Walkmesh::read(&data, WalkmeshKind::Placeable).ok()?;
                        Some(moved(&w.object_faces(None), IDENTITY))
                    })
                    .clone();
                if theirs.is_some_and(|t| same_bake(&t, &baked)) {
                    // (Its model gone from the hak: written again.)
                    if !rm.contains(&ResKey::new(name_ref(&r.model)?, ResType::MDL)) {
                        files.push(model_file(&r.model, copy_model(rm, &original, &r.model)?)?);
                    }
                    return Ok(Bake {
                        row: r.row,
                        copy_of,
                        model: r.model.clone(),
                        original,
                        label: r.label.clone(),
                        reused: true,
                        before,
                        after: baked,
                    });
                }
            }
            let model = next_name(rules, &after, rm, &made)?;
            let row = next_row(rules, &after, &table, &made)?;
            let copy = copy_model(rm, &original, &model)?;
            let banner = format!(
                "Baked by Moonglow Toolset (Bake Walkmesh to Transform) from {original}.pwk:\n\
                 its faces moved as the game draws the model"
            );
            let pwk = w.moved_text(&model, req.matrix, &banner).into_bytes();
            files.push(model_file(&model, copy)?);
            files.push((ResKey::new(name_ref(&model)?, ResType::PWK), pwk));
            let row_label =
                table.get(copy_of as usize, "Label").unwrap_or(original.as_str()).to_string();
            let label = label(&row_label, &req.transform);
            after.insert(BakedRow { row, copy_of, model: model.clone(), label: label.clone() });
            Ok(Bake { row, copy_of, model, original, label, reused: false, before, after: baked })
        })();
        if let Ok(b) = &bake
            && !b.reused
        {
            made.push(b.clone());
        }
        each.push(bake);
    }
    let rows_changed = after != *rows;
    Ok(Plan {
        each,
        files,
        rows: after,
        rows_changed,
        hak: rules.hak.clone(),
        rows_file: rules.rows_file.clone(),
    })
}

const IDENTITY: [[f32; 4]; 4] =
    [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

fn name_ref(name: &str) -> Result<ResRef, String> {
    ResRef::from_str(name).map_err(|_| format!("\"{name}\" is not a model's name"))
}

fn model_file(name: &str, data: Vec<u8>) -> Result<(ResKey, Vec<u8>), String> {
    Ok((ResKey::new(name_ref(name)?, ResType::MDL), data))
}

/// The next baked model's name: `names` and four digits, the first that
/// is no row's and no model's or walkmesh's of the game data.
fn next_name(rules: &Ready, rows: &RowsFile, rm: &ResMan, made: &[Bake]) -> Result<String, String> {
    for n in 1..10_000 {
        let name = format!("{}{n:04}", rules.names).to_ascii_lowercase();
        let Ok(r) = ResRef::from_str(&name) else { break };
        let taken = rows.of_model(&name).is_some()
            || made.iter().any(|b| b.model == name)
            || rm.contains(&ResKey::new(r, ResType::MDL))
            || rm.contains(&ResKey::new(r, ResType::PWK));
        if !taken {
            return Ok(name);
        }
    }
    Err(format!("no name beginning {} is free for a baked model", rules.names))
}

/// The next free row of the project's: no row of the rows file's, none
/// the game's table holds ([`holds`]).
fn next_row(rules: &Ready, rows: &RowsFile, table: &TwoDa, made: &[Bake]) -> Result<u32, String> {
    (rules.rows.from..=rules.rows.to)
        .find(|&r| rows.row(r).is_none() && !holds(table, r) && !made.iter().any(|b| b.row == r))
        .ok_or_else(|| {
            format!(
                "every row of [bake] rows {} is taken (by the rows file or the game's \
                 placeables.2da)",
                rules.rows
            )
        })
}

/// The rows of `rules`' range the game's table already holds, not the rows
/// file's own: a range that overlaps them has fewer rows free than it
/// says (a bake never takes one).
pub fn rows_held(rules: &Rows, table: &TwoDa, rows: &RowsFile) -> Vec<u32> {
    let last = rules.to.min(table.len().saturating_sub(1) as u32);
    (rules.from..=last)
        .filter(|&r| {
            holds(table, r) && rows.row(r).is_none_or(|b| row_state(table, b) != RowState::Added)
        })
        .collect()
}

/// Each `Appearance` the module's placeables have, placed and blueprints,
/// with how many have it.
pub fn appearances(module: &Module) -> BTreeMap<u32, usize> {
    let mut out = BTreeMap::new();
    for key in module.keys_of(ResType::UTP) {
        if let Some(Ok(g)) = module.gff(key)
            && let Some(a) = g.root.integer("Appearance")
        {
            *out.entry(a as u32).or_default() += 1;
        }
    }
    for key in module.keys_of(ResType::GIT) {
        let Some(Ok(g)) = module.gff(key) else { continue };
        for p in g.root.list("Placeable List").unwrap_or(&[]) {
            if let Some(a) = p.integer("Appearance") {
                *out.entry(a as u32).or_default() += 1;
            }
        }
    }
    out
}

/// What no placeable of a module uses any longer: the rows file's rows no
/// object's `Appearance` names, and the baked models in the hak no row
/// names, or only such a row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Unused {
    pub rows: Vec<BakedRow>,
    /// Models (their names) to take out of the hak, with their walkmeshes.
    pub models: Vec<String>,
}

impl Unused {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.models.is_empty()
    }
}

/// [`Unused`] of `module`: `hak_models` are the models the bake's hak has
/// (their names), of which those that begin with `names` are baked ones.
pub fn unused(module: &Module, rows: &RowsFile, names: &str, hak_models: &[String]) -> Unused {
    let used = appearances(module);
    let unused_rows: Vec<BakedRow> =
        rows.rows.iter().filter(|r| !used.contains_key(&r.row)).cloned().collect();
    let kept: BTreeSet<String> = (rows.rows.iter())
        .filter(|r| used.contains_key(&r.row))
        .map(|r| r.model.to_ascii_lowercase())
        .collect();
    let names = names.to_ascii_lowercase();
    let models = (hak_models.iter())
        .map(|m| m.to_ascii_lowercase())
        .filter(|m| m.starts_with(&names) && !kept.contains(m))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Unused { rows: unused_rows, models }
}

/// The models (their names) of the hak whose layer is `hak:<name>` in
/// `rm`.
pub fn hak_models(rm: &ResMan, hak: &str) -> Vec<String> {
    let label = format!("hak:{}", hak.to_ascii_lowercase());
    let Some(layer) = rm.layers().iter().find(|l| l.label.eq_ignore_ascii_case(&label)) else {
        return Vec::new();
    };
    let mut out: Vec<String> = (layer.container.keys())
        .filter(|k| k.restype == ResType::MDL)
        .map(|k| k.resref.to_string())
        .collect();
    out.sort();
    out
}

/// Takes `unused` out: its rows from `rows` and its models (`.mdl`,
/// `.pwk`) from the hak at `hak` (kept beside itself as `.bak` first).
pub fn tidy(unused: &Unused, rows: &mut RowsFile, hak: Option<&Path>) -> Result<(), String> {
    rows.rows.retain(|r| !unused.rows.contains(r));
    if let Some(path) = hak.filter(|_| !unused.models.is_empty()) {
        let mut keys = Vec::new();
        for m in &unused.models {
            let r = name_ref(m)?;
            keys.extend([ResKey::new(r, ResType::MDL), ResKey::new(r, ResType::PWK)]);
        }
        crate::hak_edit::remove_from(path, &keys)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
