//! The hak editor: a hak's resources, to add files and folders to, remove,
//! rename and extract, with undo, saved as a new archive moved over the old
//! one (`mg_module::hak_edit`). Tools › New Hak, Open Hak… and Build Hak
//! from Folder… open it. Saving a hak the module uses reloads it.
//!
//! As the Hak board draws it (M4's U21): a header (Add Files… and Add
//! Folder… as icons, Remove, Extract…, Extract All…, Find, Compile Models,
//! Save, Save As…, More with Update from Folder, Rename… and Edit Table, OK
//! and Cancel at its end); a notice strip under it (a hak of the game's,
//! the resources past the 2 GiB the game reads); the resources as a table
//! (Name, Type, Size, Source, sorted by a click on a column's head, its
//! arrow saying which way). It brings the Right Dock's **Preview** (the
//! resource chosen: a picture, a sound to play, a table, a model in 3D
//! with its facts) over **Details** (File, Contents, Built from with
//! Update from Folder, Description), and its contents by type to the
//! **Outliner**, a click finding that type (`panels`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::Ui;
use mg_core::ResType;
use mg_module::hak_edit::{Added, Hak, Source};
use mg_resman::ResKey;

use crate::dialogs::FileKind;
use crate::{Moonglow, Tab};

mod panels;
pub use panels::{DETAILS_DESCRIPTION, OPEN_MODEL, PREVIEW_CLOSE, UPDATE_FROM_FOLDER};
pub(crate) use panels::{
    details, hak_of, open_model, open_model_button, outline_types, panel_detail, preview, shown,
};

/// A hak open in a tab.
#[derive(Debug)]
pub struct HakDoc {
    pub id: u32,
    pub hak: Hak,
    pub filter: String,
    pub selected: BTreeSet<ResKey>,
    /// The resource clicked last: Shift + click selects from it to the one
    /// clicked, and the Preview shows it while it is selected.
    anchor: Option<ResKey>,
    /// The load order with the hak as it is now in it, the model viewer's
    /// to draw a model of it from ([`resources`]): made when a model is
    /// shown, let go at a change and when the game data is to change.
    pub(crate) resources: Option<std::sync::Arc<mg_resman::ResMan>>,
    /// The resource being renamed, and its new name as typed.
    pub rename: Option<(ResKey, String)>,
    /// Its field is to take the keyboard (the frame the renaming began:
    /// asked for at every frame, the keyboard never left it, and Enter
    /// never ended it).
    rename_focus: bool,
    /// The description as typed (with plain line ends), and whether it was
    /// typed since it was last set ([`HakDoc::set_typed`]).
    description: Option<String>,
    typed: bool,
    /// The hak's changes ([`Hak::changes`]) its views were last made for.
    seen: u64,
    /// The rows shown, in the table's order, for the Find, the order and
    /// the changes they were made for.
    rows: Option<RowsMade>,
    /// What is counted of the whole hak ([`Facts`]), for the changes it
    /// was counted at.
    facts: Option<(u64, Arc<Facts>)>,
    /// How many times the whole hak was walked (its rows sorted, its facts
    /// counted): the tests'.
    walks: u64,
    /// The table's order: the column, and whether it runs down.
    pub sort: Sort,
    pub descending: bool,
    /// The resource the Preview shows, with the changes it was made for.
    viewing: Option<(ResKey, u64, Viewing)>,
    /// The resource the Preview's Close put away: it shows nothing until
    /// another is chosen (or this one viewed again).
    closed: Option<ResKey>,
    /// The resource whose sound the Preview plays.
    playing: Option<ResKey>,
    /// The frame the Preview was last drawn in (its sound stops when it is
    /// drawn no more: closed, folded, another tab in front).
    preview_pass: u64,
    /// A type the Outliner chose (the Find it set, ".mdl"), and what Find
    /// held before it, put back when the type is let go; typed over, Find
    /// is the builder's own and nothing is put back.
    pub(crate) type_find: Option<String>,
    pub(crate) find_before: Option<String>,
    /// The folder the hak was built from: Update from Folder reads it
    /// again.
    pub folder: Option<PathBuf>,
    /// Files chosen to add, some of which the hak has already: asked
    /// about before anything is replaced.
    pub adding: Option<Adding>,
    /// Resources about to be extracted over files a folder has already:
    /// asked about before anything is written.
    pub extracting: Option<Extracting>,
    /// Whether the hak's file is in the game's folder ([`in_folder`]), for
    /// the path it was worked out for: looked at again when that changes.
    in_game: Option<(PathBuf, bool)>,
    /// Whether the pointer is over the editor (Find's keys go to its field
    /// then; the Status Bar's hints are its).
    pub(crate) hovered: bool,
    /// The resource the table was last brought to (a key moving the
    /// selection brings the row into sight), and for which Find and order
    /// (a sort or a Find keeps the resource clicked last in sight).
    scrolled_to: Option<ResKey>,
    scrolled_order: Option<(String, Sort, bool)>,
}

/// The rows shown, in the table's order, with the Find (folded), the
/// column and its way, and the hak's changes they were made for.
type RowsMade = (String, Sort, bool, u64, Arc<Vec<usize>>);

/// What is counted of a whole hak, once for each of its changes
/// ([`HakDoc::facts`]): its resources and their size, those that start past
/// the 2 GiB the game reads, and its types with how many of each, the most
/// first (the Outliner's).
#[derive(Debug, Default)]
pub(crate) struct Facts {
    pub(crate) count: usize,
    pub(crate) size: u64,
    pub(crate) past: usize,
    pub(crate) types: Vec<(ResType, usize)>,
}

impl Facts {
    /// "2706 resources, 183.8 MB" (the board's).
    pub(crate) fn words(&self) -> String {
        format!("{}, {}", crate::problems::count(self.count, "resource"), size(self.size))
    }
}

/// Whether the file `path` (there or not yet) is in `folder` or under it,
/// as written or once the links on the way to it are followed (a hak
/// folder that is a link into the game's folder).
pub(crate) fn in_folder(path: &Path, folder: &Path) -> bool {
    let resolved = |p: &Path| p.canonicalize().ok();
    let file = match (path.parent().and_then(resolved), path.file_name()) {
        (Some(dir), Some(name)) => dir.join(name),
        _ => path.to_path_buf(),
    };
    path.starts_with(folder) || resolved(folder).is_some_and(|f| file.starts_with(f))
}

/// Whether the file `path` is in the game's folder, which Moonglow never
/// writes into: the hak editor's Save and NUI Images › Into ask by this.
pub(crate) fn in_game_folder(app: &Moonglow, path: &Path) -> bool {
    app.install.as_ref().is_some_and(|gi| in_folder(path, &gi.root))
}

/// A hak's resource as the Preview shows it: a picture (a TGA, DDS or
/// PLT), a sound to play, a 2DA's rows and columns, a model drawn (the copy
/// the game reads being this hak's) with its facts, or as the resource
/// browser shows the rest (fields, text, a compiled model as its text, the
/// first bytes).
pub(crate) enum Viewing {
    /// (Decoded off the frame: `viewers`, the read-only tab's too.)
    Picture(crate::viewers::Picture),
    Sound(std::sync::Arc<[u8]>),
    /// (With which reading of it this is: `browser::made`.)
    Table(Box<mg_module::table_layers::Layered>, u64),
    /// Drawn in the model viewer: from the load order where the game reads
    /// this copy (`read`: it opens in a tab of its own too), else from the
    /// hak as it is now over the load order ([`resources`]); its facts
    /// read once, as it is chosen.
    Model {
        read: bool,
        facts: ModelFacts,
    },
    Plain(crate::browser::Plain),
}

/// A model's facts, as the Preview lists them (the Hak board's): whether it
/// is compiled, its size and its nodes, faces, vertices, textures and
/// animations, and its supermodel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelFacts {
    pub(crate) compiled: bool,
    pub(crate) size: u64,
    pub(crate) nodes: usize,
    pub(crate) faces: usize,
    pub(crate) vertices: usize,
    pub(crate) textures: usize,
    pub(crate) animations: usize,
    pub(crate) supermodel: Option<String>,
}

impl ModelFacts {
    fn of(model: &mg_mdl::Model, data: &[u8]) -> ModelFacts {
        let meshes = || model.nodes.iter().filter_map(|n| n.mesh());
        let textures: BTreeSet<String> = meshes()
            .flat_map(|m| m.textures.iter().flatten())
            .map(|t| t.trim().to_ascii_lowercase())
            .filter(|t| !t.is_empty() && t != "null")
            .collect();
        ModelFacts {
            compiled: mg_mdl::is_binary(data),
            size: data.len() as u64,
            nodes: model.nodes.len(),
            faces: meshes().map(|m| m.faces.len()).sum(),
            vertices: meshes().map(|m| m.vertices.len()).sum(),
            textures: textures.len(),
            animations: model.animations.len(),
            supermodel: model
                .supermodel
                .clone()
                .filter(|s| !s.trim().is_empty() && !s.eq_ignore_ascii_case("null")),
        }
    }

    /// Its facts as the Preview's rows: a word and a value each.
    pub(crate) fn rows(&self) -> [(&'static str, String); 8] {
        [
            ("Type", if self.compiled { "Model, compiled" } else { "Model, text" }.to_string()),
            ("Size", size(self.size)),
            ("Nodes", self.nodes.to_string()),
            ("Faces", self.faces.to_string()),
            ("Vertices", self.vertices.to_string()),
            ("Textures", self.textures.to_string()),
            ("Animations", self.animations.to_string()),
            ("Supermodel", self.supermodel.clone().unwrap_or_else(|| "(none)".to_string())),
        ]
    }
}

impl std::fmt::Debug for Viewing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Viewing::{}", self.kind())
    }
}

impl Viewing {
    /// What a word names the view by (for tests and the log).
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Viewing::Picture(_) => "picture",
            Viewing::Sound(_) => "sound",
            Viewing::Table(..) => "table",
            Viewing::Model { .. } => "model",
            Viewing::Plain(_) => "plain",
        }
    }
}

/// How hak `doc`'s resource `key` (its bytes `data`) is viewed.
fn viewing(
    app: &mut Moonglow,
    ctx: &egui::Context,
    doc: usize,
    key: ResKey,
    data: Vec<u8>,
) -> Viewing {
    let game = app.game.clone();
    let rm = game.as_deref().map(|g| &g.resman);
    let plain = |data: &[u8]| Viewing::Plain(crate::browser::plain(key, data, rm));
    match key.restype {
        // (A PLT in each layer's first color, from the game's palettes;
        // decoded off the frame, as the read-only tab's: M5's W6.)
        t if crate::viewers::is_picture(t) => {
            Viewing::Picture(crate::viewers::Picture::decode(app, ctx, key, data.into()))
        }
        ResType::WAV | ResType::BMU => {
            let data: std::sync::Arc<[u8]> = data.into();
            match mg_audio::Stream::open(data.clone()) {
                Ok(_) => {
                    // (The game plays a WAVE under .bmu in no role.)
                    app.warn_silent(crate::audio::Role::Viewed, key.resref, key.restype, &data);
                    Viewing::Sound(data)
                }
                Err(e) => Viewing::Plain(crate::browser::Plain::Error(e.to_string())),
            }
        }
        ResType::TWODA => {
            let codepage = app.game.as_deref().map(|g| g.codepage()).unwrap_or_default();
            match mg_2da::TwoDa::parse(&data, codepage) {
                Ok(t) => {
                    let name = app.haks[doc].title();
                    let table =
                        mg_module::table_layers::Layered::single(name.trim_end_matches(" *"), t);
                    Viewing::Table(Box::new(table), crate::browser::made())
                }
                Err(_) => plain(&data),
            }
        }
        // Drawn: from the load order where the game reads this copy (the
        // hak is one of the module's, as saved), else from the hak as it
        // is now over the load order (one the module doesn't list, or
        // changed and not saved), its supermodels and textures too.
        // (Without game data, its text: the viewer has nothing to draw
        // it with.)
        ResType::MDL if app.game.is_some() => match mg_mdl::Model::read(&data) {
            Ok(model) => {
                let read = app.game.as_deref().is_some_and(|g| {
                    let stem = app.haks[doc].hak.path.as_deref().and_then(|p| p.file_stem());
                    let layer = stem.map(|s| format!("hak:{}", s.to_string_lossy()));
                    g.resman.get(&key).is_ok_and(|d| *d == data[..])
                        && layer.is_some_and(|l| {
                            g.resman.origin(&key).is_some_and(|o| o.eq_ignore_ascii_case(&l))
                        })
                });
                Viewing::Model { read, facts: ModelFacts::of(&model, &data) }
            }
            Err(_) => plain(&data),
        },
        _ => plain(&data),
    }
}

/// Files chosen to add to a hak that has some of them already.
#[derive(Debug, Clone, PartialEq)]
pub struct Adding {
    pub files: Vec<PathBuf>,
    /// Those that would replace a resource of the hak, and which.
    pub replaced: Vec<(PathBuf, ResKey)>,
}

/// The column a hak's table is sorted by (the Hak board's heads: a click
/// on one sorts by it, again the other way).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    Name,
    /// By type's words, then its extension, then name.
    Type,
    /// The smallest first (down: the largest).
    Size,
    /// By the file it was added from (those of the archive first).
    Source,
}

/// A type's words in the table's Type column (the Hak board's "Model").
pub(crate) fn kind_words(t: ResType) -> &'static str {
    if let Some(kind) = mg_area::ObjectKind::from_restype(t) {
        return match kind {
            mg_area::ObjectKind::Creature => "Creature",
            mg_area::ObjectKind::Door => "Door",
            mg_area::ObjectKind::Encounter => "Encounter",
            mg_area::ObjectKind::Item => "Item",
            mg_area::ObjectKind::Placeable => "Placeable",
            mg_area::ObjectKind::Sound => "Sound object",
            mg_area::ObjectKind::Store => "Store",
            mg_area::ObjectKind::Trigger => "Trigger",
            mg_area::ObjectKind::Waypoint => "Waypoint",
        };
    }
    match t {
        ResType::MDL => "Model",
        ResType::WOK | ResType::PWK | ResType::DWK => "Walkmesh",
        ResType::TGA | ResType::DDS | ResType::PLT => "Texture",
        ResType::TXI => "Texture info",
        ResType::MTR => "Material",
        ResType::SHD => "Shader",
        ResType::TWODA => "Table",
        ResType::WAV => "Sound",
        ResType::BMU => "Music",
        ResType::NSS => "Script",
        ResType::NCS => "Compiled script",
        ResType::NDB => "Script debug",
        ResType::ARE => "Area",
        ResType::GIT => "Area objects",
        ResType::GIC => "Area comments",
        ResType::DLG => "Conversation",
        ResType::JUI => "NUI window",
        ResType::JRL => "Journal",
        ResType::TLK => "Talk table",
        ResType::SET => "Tileset",
        ResType::ITP => "Palette",
        ResType::IFO => "Module info",
        ResType::FAC => "Factions",
        ResType::SSF => "Soundset",
        ResType::LTR => "Name letters",
        ResType::INI => "Settings",
        ResType::TXT => "Text",
        _ => "File",
    }
}

impl HakDoc {
    fn new(id: u32, hak: Hak) -> HakDoc {
        HakDoc {
            id,
            seen: hak.changes(),
            hak,
            filter: String::new(),
            selected: BTreeSet::new(),
            anchor: None,
            resources: None,
            rename: None,
            rename_focus: false,
            description: None,
            typed: false,
            rows: None,
            facts: None,
            walks: 0,
            sort: Sort::Name,
            descending: false,
            viewing: None,
            closed: None,
            playing: None,
            preview_pass: 0,
            type_find: None,
            find_before: None,
            folder: None,
            adding: None,
            extracting: None,
            in_game: None,
            hovered: false,
            scrolled_to: None,
            scrolled_order: None,
        }
    }

    /// Whether the hak's file is in the game's folder (`root`).
    pub(crate) fn is_in_game(&mut self, root: Option<&Path>) -> bool {
        let (Some(path), Some(root)) = (&self.hak.path, root) else { return false };
        match &self.in_game {
            Some((p, is)) if p == path => *is,
            _ => {
                let is = in_folder(path, root);
                self.in_game = Some((path.clone(), is));
                is
            }
        }
    }

    /// What the Preview shows (`picture`, `sound`, `table`, `model`,
    /// `plain`), and which.
    pub fn viewing(&self) -> Option<(ResKey, &'static str)> {
        self.viewing.as_ref().map(|(k, _, v)| (*k, v.kind()))
    }

    /// The resource the Preview shows: the anchor of the selection (the one
    /// clicked, or Ctrl+clicked, last; a Shift run's start: the board's first
    /// of three) where it is selected, else the first of those selected by
    /// name (a Ctrl+click that let go of the anchor); none where the
    /// Preview's Close put it away.
    pub fn preview_key(&self) -> Option<ResKey> {
        let key = self.anchor.filter(|a| self.selected.contains(a));
        let key = key.or_else(|| self.selected.first().copied())?;
        (self.closed != Some(key)).then_some(key)
    }

    /// `key` chosen alone, as a click chooses it (the Preview showing it).
    pub fn choose(&mut self, key: ResKey) {
        self.selected = [key].into();
        self.anchor = Some(key);
        self.closed = None;
    }

    pub fn title(&self) -> String {
        let name = self
            .hak
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("New hak".into(), |n| n.to_string_lossy().into_owned());
        format!("{name}{}", if self.is_dirty() { " *" } else { "" })
    }

    /// Whether it has changes not saved: the hak's, or a description typed
    /// and not yet set.
    pub fn is_dirty(&self) -> bool {
        self.hak.is_dirty() || self.typed
    }

    /// How many times the whole hak was walked (its rows sorted, its
    /// facts counted): made again only for a change, a Find or an order.
    pub fn walks(&self) -> u64 {
        self.walks
    }

    /// The items shown, in the table's order: indices into the hak's
    /// items, made again only where the Find, the order or the hak changed
    /// (a hak of 20,000: never at every frame).
    pub(crate) fn rows(&mut self) -> Arc<Vec<usize>> {
        let needle = mg_core::text::Needle::new(&self.filter);
        let filter = needle.as_str().to_string();
        let changes = self.hak.changes();
        if let Some((f, s, d, c, rows)) = &self.rows
            && *f == filter
            && *s == self.sort
            && *d == self.descending
            && *c == changes
        {
            return rows.clone();
        }
        self.walks += 1;
        let items = self.hak.items();
        let mut rows: Vec<usize> =
            (0..items.len()).filter(|&i| needle.finds(&items[i].key.to_string())).collect();
        // (A name made once an item, not at every comparison: 300,000
        // for a hak of 20,000, at each letter typed into Find.)
        rows.sort_by_cached_key(|&i| items[i].key.to_string());
        // (Stable: by name within a type, a size or a source.)
        match self.sort {
            Sort::Name => {}
            Sort::Type => rows.sort_by_cached_key(|&i| {
                let t = items[i].key.restype;
                (kind_words(t), t.to_string())
            }),
            Sort::Size => rows.sort_by_key(|&i| items[i].size),
            Sort::Source => rows.sort_by_cached_key(|&i| match &items[i].source {
                Source::File(p) => Some(p.display().to_string()),
                Source::Archive | Source::Bytes(_) => None,
            }),
        }
        if self.descending {
            rows.reverse();
        }
        let rows = Arc::new(rows);
        self.rows = Some((filter, self.sort, self.descending, changes, rows.clone()));
        rows
    }

    /// What is counted of the whole hak ([`Facts`]), made again only where
    /// it changed.
    pub(crate) fn facts(&mut self) -> Arc<Facts> {
        let changes = self.hak.changes();
        if let Some((c, facts)) = &self.facts
            && *c == changes
        {
            return facts.clone();
        }
        self.walks += 1;
        let items = self.hak.items();
        let mut types: std::collections::BTreeMap<ResType, usize> = Default::default();
        for item in items {
            *types.entry(item.key.restype).or_default() += 1;
        }
        let mut types: Vec<(ResType, usize)> = types.into_iter().collect();
        // (The most first, the board's; of as many, by the type's number, as
        // the board's .are before .gic, .dlg before .utw, .utt before .utm.)
        types.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.0.cmp(&b.0.0)));
        let facts = Arc::new(Facts {
            count: items.len(),
            size: self.hak.size(),
            past: self.hak.past_read_limit().len(),
            types,
        });
        self.facts = Some((changes, facts.clone()));
        facts
    }

    /// Whether the table, as last made, has rows (the Find leaves some).
    pub(crate) fn has_rows_shown(&self) -> bool {
        self.rows.as_ref().is_some_and(|(.., rows)| !rows.is_empty())
    }

    /// [`HakDoc::facts`] as last counted, where they are the hak's now.
    pub(crate) fn facts_seen(&self) -> Option<&Arc<Facts>> {
        self.facts.as_ref().filter(|(c, _)| *c == self.hak.changes()).map(|(_, f)| f)
    }

    /// Whether the load order its models are drawn from holds the game
    /// data ([`resources`]: made again whenever that changes, so it is
    /// the window's).
    pub(crate) fn holds_game(&self) -> bool {
        self.resources.as_ref().is_some_and(|rm| rm.layers().len() > 1)
    }

    /// The hak's changes ([`Hak::changes`]): what is made from its
    /// resources is kept while this stays.
    pub(crate) fn revision(&self) -> u64 {
        self.hak.changes()
    }

    /// What is made of the hak's resources let go of where it changed:
    /// those selected that it has no more, the load order its models are
    /// drawn from; the description shown again, unless one is typed.
    pub(crate) fn changed(&mut self) {
        self.seen = self.hak.changes();
        if !self.typed {
            self.description = None;
        }
        if !self.selected.is_empty() {
            let keys: BTreeSet<ResKey> = self.hak.items().iter().map(|i| i.key).collect();
            self.selected.retain(|k| keys.contains(k));
        }
        self.resources = None;
    }

    /// The description as typed, set where it was typed (one step to undo):
    /// whether it changed the hak.
    pub(crate) fn set_typed(&mut self) -> bool {
        if !std::mem::take(&mut self.typed) {
            return false;
        }
        let Some(text) = self.description.take() else { return false };
        let stored = self.hak.description.clone();
        let text = crate::text::with_line_ends(&text, &stored);
        let before = self.hak.changes();
        self.hak.set_description(&text);
        self.changed();
        self.hak.changes() != before
    }

    /// Whether a description is typed and not yet set.
    pub(crate) fn typing_description(&self) -> bool {
        self.typed
    }

    /// The description being typed, as shown (from the hak where none is).
    pub(crate) fn description_text(&mut self) -> &mut String {
        let stored = &self.hak.description;
        self.description.get_or_insert_with(|| crate::text::to_editor(stored).0)
    }
}

/// The Description's field's id, by its hak (typed text keeps to its hak).
pub(crate) fn description_id(id: u32) -> egui::Id {
    egui::Id::new(("hak-description", id))
}

/// Each open hak followed as a frame begins: what was made of a hak that
/// changed by any way let go of (and its facts counted again, once); a
/// description typed set where its field has the keyboard no more (another
/// hak in front, the dock folded, the panel closed).
pub(crate) fn follow_all(app: &mut Moonglow, ctx: &egui::Context) {
    let focused = ctx.memory(|m| m.focused());
    let pass = ctx.cumulative_pass_nr();
    // (A hak's sound stops with its hak closed, or another module.)
    let module = app.module_path();
    let mut stop = match &app.hak_sound {
        Some((id, m)) => {
            *m != module || !app.haks.iter().any(|d| d.id == *id && d.playing.is_some())
        }
        None => false,
    };
    for d in &mut app.haks {
        // (Typed over, Find is the builder's: nothing put back.)
        if d.type_find.as_ref().is_some_and(|t| *t != d.filter) {
            d.type_find = None;
            d.find_before = None;
        }
        // (Its Preview drawn no more: closed, folded, behind another tab.)
        if d.playing.is_some() && d.preview_pass + 1 < pass {
            d.playing = None;
            stop = true;
        }
        if d.seen != d.hak.changes() {
            d.changed();
        }
        if d.typed && focused != Some(description_id(d.id)) {
            d.set_typed();
        }
        d.facts();
        // (A sound played for a resource no longer chosen stops: the
        // window rests again.)
        if d.playing.is_some() && d.playing != d.preview_key() {
            d.playing = None;
            stop = true;
        }
    }
    if stop {
        // (Only where the hak's sound still holds the channel: another
        // Play anywhere has taken it since.)
        if let Some((id, _)) = app.hak_sound.take() {
            crate::viewers::stop_by(app, &crate::viewers::Player::Hak(id));
        }
        for d in &mut app.haks {
            d.playing = None;
        }
    }
    // (The views of models of haks closed since go.)
    let haks = &app.haks;
    app.model_views.retain(|s, _| match s {
        crate::model_view::Source::Hak { hak, .. } => haks.iter().any(|d| d.id == *hak),
        _ => true,
    });
}

/// Every hak's description typed and not yet set, set (before a save, a
/// closing question, Undo).
pub(crate) fn set_typed(app: &mut Moonglow, id: u32) {
    if let Some(d) = app.haks.iter_mut().find(|d| d.id == id) {
        d.set_typed();
    }
}

/// A layer of the game data's load order, read through: the game's own
/// layers under a hak being edited ([`resources`]), all of them holding
/// the game data once.
#[derive(Debug)]
struct LayerOf {
    game: std::sync::Arc<Held>,
    at: usize,
}

/// The game data, held once for all of a load order's [`LayerOf`].
#[derive(Debug)]
struct Held(std::sync::Arc<mg_rules::GameData>);

impl mg_resman::Container for LayerOf {
    fn contains(&self, key: &ResKey) -> bool {
        self.game.0.resman.layers()[self.at].container.contains(key)
    }

    fn read(&self, key: &ResKey) -> Result<std::borrow::Cow<'_, [u8]>, mg_resman::ResError> {
        self.game.0.resman.layers()[self.at].container.read(key)
    }

    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        self.game.0.resman.layers()[self.at].container.keys()
    }

    fn len(&self) -> usize {
        self.game.0.resman.layers()[self.at].container.len()
    }
}

/// The load order with hak `doc` as it is now (saved or not) among the
/// haks, over the module, `override/` and the game, in place of its saved
/// copy where the module lists it: what a model of it is drawn from, its
/// supermodels and textures (`model_view`'s `Source::Hak`). Each of the
/// game data's layers is read through, keeping its place and kind (a DDS
/// over a TGA within the archives, as the game finds textures). Made
/// once until the hak changes; let go with the game data
/// ([`Moonglow::release_game_data`]), which it holds.
pub(crate) fn resources(app: &mut Moonglow, doc: usize) -> std::sync::Arc<mg_resman::ResMan> {
    use mg_resman::{LayerClass, ResMan, priority};
    if let Some(made) = &app.haks[doc].resources {
        return made.clone();
    }
    let hak = &app.haks[doc].hak;
    let own = hak.path.as_deref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy());
    let own = own.map(|s| format!("hak:{s}"));
    let mut rm = ResMan::new();
    // (Above the module's haks: as the first of them.)
    rm.add(priority::HAK_USER + 1, "the hak being edited", LayerClass::Erf, hak.contents());
    if let Some(game) = &app.game {
        let held = std::sync::Arc::new(Held(game.clone()));
        for (at, layer) in game.resman.layers().iter().enumerate() {
            if own.as_deref().is_some_and(|o| layer.label.eq_ignore_ascii_case(o)) {
                continue;
            }
            let through = LayerOf { game: held.clone(), at };
            rm.add(layer.priority, layer.label.clone(), layer.class, through);
        }
    }
    let rm = std::sync::Arc::new(rm);
    app.haks[doc].resources = Some(rm.clone());
    rm
}

fn open_tab(app: &mut Moonglow, hak: Hak) -> u32 {
    app.next_hak += 1;
    let id = app.next_hak;
    app.haks.push(HakDoc::new(id, hak));
    app.actions.push(crate::Action::OpenTab(Tab::Hak(id)));
    id
}

fn report(app: &mut Moonglow, added: &Added) {
    app.log.info(format!("Added {} files, replaced {}", added.added, added.replaced));
    for (path, why) in &added.skipped {
        app.log.warn(format!("Left out {}: {why}", path.display()));
    }
}

/// Tools › New Hak.
pub fn new_hak(app: &mut Moonglow) {
    open_tab(app, Hak::new());
}

/// Tools › Open Hak….
pub fn open_hak(app: &mut Moonglow) {
    let start = hak_dir(app);
    let Some(path) = app.dialogs.open_file(FileKind::Hak, start.as_deref()) else { return };
    open_at(app, &path);
}

/// Opens a hak's file in the hak editor (its tab brought to the front if
/// it is open already); its id.
pub fn open_at(app: &mut Moonglow, path: &std::path::Path) -> Option<u32> {
    if let Some(d) = app.haks.iter().find(|d| d.hak.path.as_deref() == Some(path)) {
        let id = d.id;
        app.actions.push(crate::Action::OpenTab(Tab::Hak(id)));
        return Some(id);
    }
    match Hak::open(path) {
        Ok(h) => {
            let id = open_tab(app, h);
            // (Built from a folder before: Update from Folder knows it.)
            let folder = app.settings.hak_folder(path).map(std::path::Path::to_path_buf);
            if let Some(doc) = app.haks.iter_mut().find(|d| d.id == id) {
                doc.folder = folder;
            }
            Some(id)
        }
        Err(e) => {
            app.log.error(format!("Could not open {}: {e}", path.display()));
            None
        }
    }
}

/// Tools › Build Hak from Folder…: a new hak with a folder's files (and
/// those of the folders in it), to look over and save.
pub fn build_from_folder(app: &mut Moonglow) {
    let Some(dir) = app.dialogs.pick_folder("Build Hak from Folder", None) else { return };
    let mut hak = Hak::new();
    let added = hak.add_folder(&dir);
    report(app, &added);
    // Saved by default in the user's hak folder, named after the folder.
    let name = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let id = open_tab(app, hak);
    if let Some(doc) = app.haks.iter_mut().find(|d| d.id == id) {
        doc.folder = Some(dir.clone());
    }
    app.suggested_hak = hak_dir(app).map(|d| d.join(format!("{name}.hak")));
}

/// The user's hak folder.
fn hak_dir(app: &Moonglow) -> Option<PathBuf> {
    app.install.as_ref()?.user_hak_dir()
}

/// Where a save of hak `i` goes: its own file (`Ok(None)`), or the one
/// chosen (asked for a new hak, one of the game's, or with `ask`). `Err`:
/// nowhere (the dialog canceled, or the game's folder chosen: said).
fn save_to(app: &mut Moonglow, i: usize, ask: bool) -> Result<Option<PathBuf>, ()> {
    let root = app.install.as_ref().map(|gi| gi.root.clone());
    let in_game = app.haks[i].is_in_game(root.as_deref());
    if !(ask || in_game || app.haks[i].hak.path.is_none()) {
        return Ok(None);
    }
    let suggested = app
        .suggested_hak
        .take()
        .or_else(|| app.haks[i].hak.path.clone())
        .or_else(|| hak_dir(app).map(|d| d.join("new.hak")));
    match app.dialogs.save_file(FileKind::Hak, suggested.as_deref()) {
        Some(p) if in_game_folder(app, &p) => {
            let why = "Moonglow doesn't write into the game's folder: save the hak elsewhere";
            app.log.error(why);
            app.note_failed(&Tab::Hak(app.haks[i].id), why);
            Err(())
        }
        Some(p) => Ok(Some(p)),
        None => Err(()),
    }
}

/// After hak `i` was saved: its views made again, the folder it was
/// built from remembered, the log, and the module's haks read again.
fn after_save(app: &mut Moonglow, i: usize) {
    let doc = &mut app.haks[i];
    doc.changed();
    let path = doc.hak.path.clone().expect("saved");
    if let Some(folder) = &doc.folder {
        app.settings.remember_hak_folder(&path, folder);
    }
    let past = doc.hak.past_read_limit();
    app.log.info(format!("Saved {} ({} resources)", path.display(), doc.hak.items().len()));
    if let Some(first) = past.first() {
        app.log.warn(format!(
            "{} resources from {first} on start past 2 GiB into the hak, where the game stops \
             reading: split it into two",
            past.len()
        ));
    }
    // The module's haks are read again if this is one.
    app.reload_resources(false);
}

/// Saves a hak (asking where for a new one, or with `ask`), here and
/// now: whether it was. For what goes on with the answer at once: the
/// module's Save, which is followed in the same step by what reads the
/// files written (the game started for a test, a project's pack, the
/// plugins told of the save, Exit), and a NUI window's pictures, which
/// says what was written. The hak editor's own Save, its tab's OK and
/// its question on closing are jobs ([`save_then`]). A hak the module
/// uses is read again.
pub(crate) fn save(app: &mut Moonglow, id: u32, ask: bool) -> bool {
    set_typed(app, id);
    let Some(i) = app.haks.iter().position(|d| d.id == id) else { return false };
    let Ok(to) = save_to(app, i, ask) else { return false };
    match app.haks[i].hak.save(to.as_deref()) {
        Ok(()) => {
            after_save(app, i);
            true
        }
        Err(e) => {
            app.log.error(format!("Could not save the hak: {e}"));
            app.note_failed(&Tab::Hak(id), e);
            false
        }
    }
}

/// How far a job that writes `total` bytes is, `done` of them written:
/// the bar by megabytes (none for less than one), and the sizes said.
/// Whether to go on (not canceled).
fn written(progress: &crate::jobs::Progress, done: u64, total: u64) -> bool {
    progress.step((done >> 20) as usize, (total >> 20) as usize);
    progress.say(format!("{} of {}", size(done), size(total)));
    !progress.cancelled()
}

/// The title of the job that saves a hak ([`save_then`]).
pub(crate) const SAVE_JOB: &str = "Save Hak";

/// What goes on once a hak is saved as a job ([`save_then`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Then {
    /// Nothing more: the editor's Save.
    Stay,
    /// Its tab closes (its OK, and Save in the question a tab closed with
    /// unsaved changes asks), unless it was changed again meanwhile.
    Close,
}

/// The hak editor's Save and Save As…, as a job: the hak is written on a
/// thread of its own, with how far it is and Cancel (which leaves the
/// file as it was), and taken as saved when it is written: also where
/// Cancel came too late to stop it (the job runs to its end and says
/// which). A hak of gigabytes was written inside the frame, and the
/// window stood still for the whole copy.
fn save_as_job(app: &mut Moonglow, id: u32, ask: bool) {
    save_then(app, id, ask, Then::Stay);
}

/// Saves a hak as a job ([`save_as_job`]), and goes on with `then` once
/// it is written: a tab's OK and its closing question's Save wrote it
/// inside the frame, to close the tab with the answer. One not saved (the
/// place not chosen, the disk full, the job canceled) stays as it was,
/// open and unsaved.
pub(crate) fn save_then(app: &mut Moonglow, id: u32, ask: bool, then: Then) {
    set_typed(app, id);
    let Some(i) = app.haks.iter().position(|d| d.id == id) else { return };
    app.clear_failed(&Tab::Hak(id));
    // (Not where the Save dialog was canceled: that is no failure.)
    let Ok(to) = save_to(app, i, ask) else { return failed(app, id, then) };
    let saving = match app.haks[i].hak.saving(to.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            app.log.error(format!("Could not save the hak: {e}"));
            app.note_failed(&Tab::Hak(id), e);
            return failed(app, id, then);
        }
    };
    let total = saving.size();
    app.start_job_apart(
        SAVE_JOB,
        move |cx| saving.write(&mut |done| written(&cx.progress, done, total)),
        move |app, saved| {
            let Some(i) = app.haks.iter().position(|d| d.id == id) else { return };
            match saved.and_then(|s| s.map(|s| app.haks[i].hak.saved(s)).transpose()) {
                Ok(Some(())) => {
                    after_save(app, i);
                    // (Changed again while it was written: it stays, with
                    // what is unsaved.)
                    let saved = app.haks.get(i).is_some_and(|d| d.id == id && !d.hak.is_dirty());
                    if then == Then::Close && saved {
                        let tab = crate::Tab::Hak(id);
                        app.side_closing.retain(|t| *t != tab);
                        app.forget_side(&tab);
                        app.actions.push(crate::Action::CloseTab(tab));
                    }
                }
                // (Stopped: the file is as it was.)
                Ok(None) => app.log.info("Save Hak: canceled"),
                Err(e) => {
                    app.log.error(format!("Could not save the hak: {e}"));
                    app.note_failed(&Tab::Hak(id), e);
                    failed(app, id, then);
                }
            }
        },
    );
}

/// A hak's save that was to close its tab failed, with a reason
/// ([`Moonglow::failed_saving`]): its question is asked (again), saying
/// why, with Save, Don't Save and Cancel; its OK did nothing to be seen.
fn failed(app: &mut Moonglow, id: u32, then: Then) {
    let tab = Tab::Hak(id);
    if then == Then::Close && app.failed_saving(&tab).is_some() && !app.side_closing.contains(&tab)
    {
        app.side_closing.push_back(tab);
    }
}

/// Resources of a hak about to be extracted into a folder that has files
/// of some of them: asked about before any is written over.
#[derive(Debug, Clone)]
pub struct Extracting {
    plan: std::sync::Arc<mg_module::hak_edit::Extraction>,
    pub dir: PathBuf,
    /// Those the folder has a file of already.
    pub there: Vec<ResKey>,
}

/// Extract… and Extract All…: the folder asked for, then the files there
/// already asked about (they were written over without a word: the
/// hak's older copies over the files it was built from, edited since),
/// and the rest written as a job.
fn extract(app: &mut Moonglow, i: usize, keys: &[ResKey]) {
    let Some(dir) = app.dialogs.pick_folder("Extract to", None) else { return };
    let plan = std::sync::Arc::new(app.haks[i].hak.extraction(keys));
    let there = plan.there(&dir);
    if there.is_empty() {
        extract_as_job(app, plan, dir, Vec::new());
    } else {
        app.haks[i].extracting = Some(Extracting { plan, dir, there });
    }
}

/// Writes `plan`'s files into `dir`, but for `skip`, as a job: with how
/// far it is and Cancel (the files written by then stay, none cut short).
fn extract_as_job(
    app: &mut Moonglow,
    plan: std::sync::Arc<mg_module::hak_edit::Extraction>,
    dir: PathBuf,
    skip: Vec<ResKey>,
) {
    let total = plan.size();
    let to = dir.clone();
    app.start_job_anywhere(
        "Extract Files",
        move |cx| {
            let skip = skip.into_iter().collect();
            plan.write(&dir, &skip, &mut |done| written(&cx.progress, done, total))
        },
        move |app, wrote| match wrote {
            Ok(n) => app.log.info(format!("Extracted {n} files to {}", to.display())),
            Err(e) => app.log.error(format!("Extract: {e}")),
        },
    );
}

/// Asks about the files an extraction would write over.
fn extracting_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(i) = app.haks.iter().position(|d| d.extracting.is_some()) else { return };
    let extracting = app.haks[i].extracting.clone().expect("found");
    let (n, all) = (extracting.there.len(), extracting.plan.len());
    let said = format!("{} already has {n} of the {all} files:", extracting.dir.display());
    let files: Vec<_> = extracting.there.iter().map(|k| (k.to_string(), None)).collect();
    let tips = [
        "Write all the files: the hak's take the place of the folder's",
        "Write the other files and leave the folder's as they are",
    ];
    use crate::widgets::Already;
    let there = &mut app.settings.files_there;
    let Some(answer) = crate::widgets::files_already(ctx, there, "Folder", &said, &files, tips)
    else {
        return;
    };
    app.haks[i].extracting = None;
    let skip = match answer {
        Already::Replace => Vec::new(),
        Already::Skip => extracting.there,
        Already::Cancel => return,
    };
    extract_as_job(app, extracting.plan, extracting.dir, skip);
}

pub(crate) fn size(n: u64) -> String {
    match n {
        n if n >= 1 << 30 => format!("{:.2} GB", n as f64 / f64::from(1u32 << 30)),
        n if n >= 1 << 20 => format!("{:.1} MB", n as f64 / f64::from(1u32 << 20)),
        n if n >= 1 << 10 => format!("{:.1} KB", n as f64 / 1024.0),
        n => format!("{n} bytes"),
    }
}

/// The name of the editor's header (the kit's group, the tests').
pub const HAK_HEADER: &str = "Hak header";
/// The header's commands' names (their words; Remove's shows its count,
/// "Remove (3)").
pub const ADD_FILES: &str = "Add Files…";
pub const ADD_FOLDER: &str = "Add Folder…";
pub const REMOVE: &str = "Remove";
pub const EXTRACT: &str = "Extract…";
pub const EXTRACT_ALL: &str = "Extract All…";
pub const FIND: &str = "Find";
pub const COMPILE_MODELS: &str = "Compile Models";
pub const SAVE: &str = "Save";
pub const SAVE_AS: &str = "Save As…";
pub const RENAME: &str = "Rename…";
pub const EDIT_TABLE: &str = "Edit Table";
/// The header's controls in its order, as its record names them (the
/// tests' `header_holds`).
pub const HAK_CONTROLS: [&str; 12] = [
    ADD_FILES,
    ADD_FOLDER,
    REMOVE,
    EXTRACT,
    EXTRACT_ALL,
    FIND,
    COMPILE_MODELS,
    SAVE,
    SAVE_AS,
    UPDATE_FROM_FOLDER,
    RENAME,
    EDIT_TABLE,
];
/// The table's name (a screen reader's, the tests').
pub const HAK_TABLE: &str = "Resources in the hak";
/// The notice of a hak of the game's.
pub const GAME_NOTICE: &str =
    "This hak is part of the game: Save As… keeps your changes in a copy.";

/// Find's width on the header (the board's `.find-field`, 196), its clear
/// button inside it.
const FIND_WIDTH: f32 = 196.0;

/// What the header asked for, done after it.
#[derive(Default)]
struct HeaderAsked {
    add_files: bool,
    add_folder: bool,
    remove: bool,
    extract: bool,
    extract_all: bool,
    compile: bool,
    save: bool,
    save_as: bool,
    update: bool,
    rename: bool,
    edit_table: bool,
    pressed: Option<crate::blueprint::Pressed>,
    /// Down in Find: on to the table, at its first row.
    down: bool,
}

/// The editor's header (the Hak board's, `Hak.png` and `Hak-h.png`, its
/// right end `Hak_fix_hdr3.png`): Add Files… and Add Folder… (icons); a
/// line; Remove (N), Extract…, Extract All…; a line; Find; then at the
/// right Compile Models, a line, Save and Save As…, More (Update from
/// Folder, Rename…, Edit Table), and OK and Cancel at its end. What has no
/// room loses its words, then goes into More, Find, Save and Save As…
/// last. Its Find typed sets the hak's (`filter`).
fn header(ui: &mut Ui, d: &mut HakDoc, in_game: bool) -> HeaderAsked {
    use mg_uikit::Icon;
    use mg_uikit::header::{EditorHeader, Item};
    use std::cell::RefCell;
    let id = d.id;
    let over = d.hovered;
    let filter = RefCell::new(std::mem::take(&mut d.filter));
    let down = std::cell::Cell::new(false);
    let find = |ui: &mut Ui| {
        let mut text = filter.borrow_mut();
        let left =
            crate::widgets::SearchField::new(egui::Id::new(("hak-filter-field", id)), &mut text)
                .name(FIND)
                .hint("name or .type")
                .width(FIND_WIDTH)
                .clear_inside()
                .find_when(over)
                .down()
                .show(ui)
                .left;
        if left == Some(crate::widgets::FilterLeft::Down) {
            down.set(true);
        }
    };
    let n = d.selected.len();
    let one = (n == 1).then(|| d.selected.first().copied()).flatten();
    let table = one.is_some_and(|k| k.restype == ResType::TWODA);
    let all = !d.hak.items().is_empty();
    let folder_tip = match &d.folder {
        Some(f) => {
            format!("Read {} again: the hak's files as the folder has them now", f.display())
        }
        None => "For a hak built with Build Hak from Folder".to_string(),
    };
    let save_tip = match in_game {
        true => "Save: a hak of the game is saved with Save As…",
        false => "Save: write the hak to its file",
    };
    let header = EditorHeader::new(HAK_HEADER)
        .left(Item::icon(Icon::Import, ADD_FILES).tip("Add Files…: add files to the hak"))
        .left(
            Item::icon(Icon::FolderOpen, ADD_FOLDER)
                .tip("Add Folder…: add the files of a folder and of the folders in it"),
        )
        .left(Item::separator())
        .left(
            Item::button(Icon::Delete, REMOVE)
                .words_icon(false)
                .shown_words(format!("Remove ({n})"))
                .enabled(n > 0)
                .tip("Remove: take the selected resources out of the hak"),
        )
        .left(
            Item::button(Icon::Export, EXTRACT)
                .words_icon(false)
                .enabled(n > 0)
                .tip("Extract…: write the selected resources into a folder"),
        )
        .left(
            Item::button(Icon::ExtractAll, EXTRACT_ALL)
                .words_icon(false)
                .enabled(all)
                .tip("Extract All…: write every resource into a folder"),
        )
        .left(Item::separator())
        .left(
            Item::custom(FIND, FIND_WIDTH, find)
                .more(FIND_WIDTH, |ui| {
                    let mut text = filter.borrow_mut();
                    let field = egui::Id::new(("hak-filter-field", id));
                    let left = crate::widgets::SearchField::new(field, &mut text)
                        .name(FIND)
                        .hint("name or .type")
                        .width(FIND_WIDTH)
                        .clear_inside()
                        .find_when(over)
                        .down()
                        .show(ui)
                        .left;
                    if left == Some(crate::widgets::FilterLeft::Down) {
                        down.set(true);
                        // (The popover closes, as choosing there does: the
                        // table has the keys at once.)
                        ui.close();
                    }
                })
                .primary(),
        )
        .right(
            Item::button(Icon::Compile, COMPILE_MODELS)
                .words_icon(false)
                .tip("Compile Models: compile the hak's ASCII models against their supermodels"),
        )
        .right(Item::separator())
        .right(
            Item::button(Icon::Save, SAVE)
                .words_icon(false)
                .primary()
                .enabled(d.is_dirty() && !in_game)
                .tip(save_tip),
        )
        .right(
            Item::button(Icon::SaveAs, SAVE_AS)
                .words_icon(false)
                .primary()
                .tip("Save As…: save the hak under another name"),
        )
        .right(
            Item::button(Icon::Update, UPDATE_FROM_FOLDER)
                .in_more()
                .enabled(d.folder.is_some())
                .tip(folder_tip),
        )
        .right(
            Item::button(Icon::Rename, RENAME)
                .in_more()
                .enabled(one.is_some())
                .tip("Rename the selected resource"),
        )
        .right(
            Item::button(Icon::Table, EDIT_TABLE)
                .in_more()
                .enabled(table)
                .tip("Edit the selected 2DA in the 2DA editor. The hak's Save writes it."),
        );
    let out =
        crate::widgets::header_with_ok_cancel(ui, header, None, crate::side_doc::OK_CANCEL_TIPS);
    d.filter = filter.into_inner();
    HeaderAsked {
        add_files: out.clicked(ADD_FILES),
        add_folder: out.clicked(ADD_FOLDER),
        remove: out.clicked(REMOVE),
        extract: out.clicked(EXTRACT),
        extract_all: out.clicked(EXTRACT_ALL),
        compile: out.clicked(COMPILE_MODELS),
        save: out.clicked(SAVE),
        save_as: out.clicked(SAVE_AS),
        update: out.clicked(UPDATE_FROM_FOLDER),
        rename: out.clicked(RENAME),
        edit_table: out.clicked(EDIT_TABLE),
        pressed: out.end,
        down: down.get(),
    }
}

/// The notice strip under the header (the board's `.notice`, 26 high on
/// `bar` with a `line` under it): a warning icon and its words in `warn`,
/// cut to the editor's width, whole in its tip. Nothing where `lines` is
/// empty.
fn notice(ui: &mut Ui, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let t = mg_uikit::tokens(ui.ctx());
    let high = 26.0;
    let (rect, r) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), high * lines.len() as f32),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0.0, t.bar);
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, egui::Stroke::new(1.0, t.line));
    let font = egui::TextStyle::Body.resolve(ui.style());
    for (n, line) in lines.iter().enumerate() {
        let row = egui::Rect::from_min_size(
            rect.min + egui::vec2(0.0, n as f32 * high),
            egui::vec2(rect.width(), high),
        );
        let icon = egui::Rect::from_center_size(
            egui::pos2(row.left() + 12.0 + 8.0, row.center().y),
            egui::vec2(16.0, 16.0),
        );
        mg_uikit::Icon::Warning.paint(ui.painter(), icon, t.warn);
        let left = icon.right() + 8.0;
        let mut job = egui::text::LayoutJob::simple_singleline(line.clone(), font.clone(), t.warn);
        job.wrap =
            egui::text::TextWrapping::truncate_at_width((row.right() - 12.0 - left).max(1.0));
        let galley = ui.painter().layout_job(job);
        let at = egui::pos2(left, row.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, t.warn);
    }
    let said = lines.join(" ");
    r.on_hover_text(&said)
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said));
}

/// What a row of the table asked for (its menu, a double click), done after
/// the table is drawn.
#[derive(Default)]
struct RowAsked {
    view: Option<ResKey>,
    edit_table: Option<ResKey>,
    extract: Option<ResKey>,
    remove: Option<ResKey>,
    /// The renaming ended: `Some` with the name typed, `None` canceled.
    renamed: Option<Option<(ResKey, String)>>,
}

/// The table's columns' widths: Type and Size (the board's 120 each); Name
/// and Source share the rest, Name the board's 364 of 770.
const TYPE_WIDTH: f32 = 120.0;
const SIZE_WIDTH: f32 = 120.0;
const NAME_SHARE: f32 = 364.0 / 770.0;

/// The table's columns, left to right, from `left`, `width` wide in all.
fn columns(left: f32, width: f32) -> [std::ops::RangeInclusive<f32>; 4] {
    let rest = (width - TYPE_WIDTH - SIZE_WIDTH).max(0.0);
    let name = (rest * NAME_SHARE).max(rest.min(180.0)).floor();
    let mut x = left;
    [name, TYPE_WIDTH, SIZE_WIDTH, (rest - name).max(0.0)].map(|w| {
        let r = x..=x + w;
        x += w;
        r
    })
}

/// The paths cut by [`cut_start`] for one font, scale and width: kept
/// while those stay. (By the font's size and the width rounded alone,
/// another font, interface size or a width half a point narrower kept the
/// cuts made for the old.)
#[derive(Clone, Default)]
struct CutStartCache {
    font: Option<egui::FontId>,
    pixels: u32,
    width: u32,
    map: std::collections::HashMap<String, String>,
}

/// `text` cut at its start to `width` (an ellipsis before the end kept):
/// the board's Source, "…/test-output/screens-hak/mg_chair.mdl".
fn cut_start(ui: &Ui, text: &str, font: &egui::FontId, width: f32) -> String {
    let (pixels, width_key) = (ui.ctx().pixels_per_point().to_bits(), width.to_bits());
    let id = egui::Id::new("hak-cut-start-cache");
    let same = |c: &CutStartCache| {
        c.font.as_ref() == Some(font) && c.pixels == pixels && c.width == width_key
    };
    if let Some(cached) = ui.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<CutStartCache>(id);
        if same(cache) { cache.map.get(text).cloned() } else { None }
    }) {
        return cached;
    }

    let wide = |s: &str| {
        ui.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), egui::Color32::WHITE))
            .size()
            .x
    };
    let result = if wide(text) <= width {
        text.to_string()
    } else {
        // (Whole folders dropped first, the path's own separators kept:
        // "…/data/hk/id_resources.hak", on Windows "…\data\hk\id_resources.hak".)
        let windows = crate::shown_path::WINDOWS;
        if let Some(s) = crate::shown_path::cut_folders(text, windows, |s| wide(s) <= width) {
            s
        } else {
            let chars: Vec<char> = text.chars().collect();
            let (mut lo, mut hi) = (0, chars.len());
            // (The fewest characters dropped from the start that fit.)
            while lo < hi {
                let mid = (lo + hi) / 2;
                let s: String = std::iter::once('…').chain(chars[mid..].iter().copied()).collect();
                if wide(&s) <= width { hi = mid } else { lo = mid + 1 }
            }
            std::iter::once('…').chain(chars[lo..].iter().copied()).collect()
        }
    };
    ui.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<CutStartCache>(id);
        if !same(cache) || cache.map.len() >= 256 {
            cache.map.clear();
            cache.font = Some(font.clone());
            cache.pixels = pixels;
            cache.width = width_key;
        }
        cache.map.insert(text.to_string(), result.clone());
    });
    result
}

/// The hak whose table was given the keys last.
fn keys_of(ctx: &egui::Context) -> Option<u32> {
    ctx.data(|d| d.get_temp(egui::Id::new("hak-keys-of")))
}

/// The keys given to hak `id`'s table (a click on a row, F6).
pub(crate) fn take_keys(ctx: &egui::Context, id: u32) {
    crate::palette_view::give_keys(ctx, crate::palette_view::Keys::Hak);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("hak-keys-of"), id));
}

/// Whether hak `id`'s table has the keys: they are a hak table's, its, its
/// editor is the active one, and nothing modal or a popover is in front
/// (as a 2DA's grid's, `table_view::grid_has_keys`).
pub(crate) fn table_has_keys(app: &Moonglow, ctx: &egui::Context, id: u32) -> bool {
    crate::palette_view::keys_are(ctx, crate::palette_view::Keys::Hak)
        && keys_of(ctx) == Some(id)
        && crate::table_view::active_tab(app).as_ref() == Some(&Tab::Hak(id))
        && !crate::shell::keys_blocked(ctx)
        && !mg_uikit::popover::Popover::any_open(ctx)
}

/// The resources as a table (the Hak board's `.table.lined`, on the 2DA
/// grid's look, `table_view::look`): a header row (Name, Type, Size,
/// Source), a click on a head sorting by it and again the other way, its
/// arrow saying which; the rows 22 high, only those in sight laid out:
/// the type's icon and the name (monospace), the type's words, the size
/// at the right, the file a resource was added from (cut at its start).
/// Those selected filled, the one clicked last ringed while the table has
/// the keys. A click selects (Ctrl adds, Shift selects a run) and gives the
/// table the keys: Up, Down, Page Up, Page Down, Home and End choose a
/// resource, while the hak is the active editor (Undo and Redo its own
/// then) and Enter views it, the table outlined meanwhile; Down in Find goes
/// to its first row; a double click views it; a right click has View, Edit
/// Table, Rename…, Extract… and Remove.
fn table(app: &mut Moonglow, ui: &mut Ui, d: &mut HakDoc) -> RowAsked {
    use crate::table_view::look;
    let ctx = ui.ctx().clone();
    let has = table_has_keys(app, &ctx, d.id);
    if has {
        app.undo_keys = Some(Tab::Hak(d.id));
    }
    let rows = d.rows();
    let tokens = mg_uikit::tokens(&ctx);
    mg_uikit::name_group(ui, HAK_TABLE);
    let mut asked = RowAsked::default();
    if has && !ctx.egui_wants_keyboard_input() && !rows.is_empty() && d.rename.is_none() {
        use egui::Key;
        let last = rows.len() as isize - 1;
        let page = ((ui.available_height() / mg_uikit::row_metrics(ui).1) as isize - 1).max(1);
        let steps = [
            (Key::ArrowDown, 1),
            (Key::ArrowUp, -1),
            (Key::PageDown, page),
            (Key::PageUp, -page),
            (Key::End, isize::MAX / 2),
            (Key::Home, isize::MIN / 2),
        ];
        let pressed = ui.input_mut(|i| {
            steps.into_iter().find(|(k, _)| i.consume_key(egui::Modifiers::NONE, *k))
        });
        if let Some((_, by)) = pressed {
            let items = d.hak.items();
            let at = d.anchor.and_then(|a| rows.iter().position(|&r| items[r].key == a));
            let to = at.map_or(0, |at| (at as isize).saturating_add(by).clamp(0, last)) as usize;
            let key = items[rows[to]].key;
            d.choose(key);
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)) {
            asked.view = d.preview_key().or(d.anchor);
        }
    }
    let width = ui.available_width();
    let (head, _) =
        ui.allocate_exact_size(egui::vec2(width, look::HEADER_HIGH), egui::Sense::hover());
    let font = egui::TextStyle::Body.resolve(ui.style());
    let heads = [
        (Sort::Name, "Name"),
        (Sort::Type, "Type"),
        (Sort::Size, "Size"),
        (Sort::Source, "Source"),
    ];
    let mut sorted = None;
    for (i, (x, (sort, words))) in columns(head.left(), width).into_iter().zip(heads).enumerate() {
        let cell = egui::Rect::from_x_y_ranges(x, head.y_range());
        look::header_cell(ui, cell);
        let on = d.sort == sort;
        let r = ui.interact(cell, ui.id().with(("hak-head", d.id, i)), egui::Sense::click());
        let said = match (on, d.descending) {
            (true, false) => format!("{words}, sorted up"),
            (true, true) => format!("{words}, sorted down"),
            (false, _) => words.to_string(),
        };
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &said));
        let r = r.on_hover_text(match on {
            true => format!("Sort by {} the other way", words.to_lowercase()),
            false => format!("Sort by {}", words.to_lowercase()),
        });
        if r.clicked() {
            sorted = Some(sort);
        }
        let color = if on { tokens.text } else { tokens.text_2 };
        let inner = cell.shrink2(egui::vec2(8.0, 0.0));
        let galley = ui.painter().layout(words.to_string(), font.clone(), color, inner.width());
        let right = sort == Sort::Size;
        let arrow = if on { 16.0 } else { 0.0 };
        let x = match right {
            true => inner.right() - galley.size().x - arrow,
            false => inner.left(),
        };
        let w = galley.size().x;
        let at = egui::pos2(x, cell.center().y - galley.size().y / 2.0);
        ui.painter().with_clip_rect(cell).galley(at, galley, color);
        if on {
            let icon =
                if d.descending { mg_uikit::Icon::ArrowDown } else { mg_uikit::Icon::ArrowUp };
            let r = egui::Rect::from_center_size(
                egui::pos2(x + w + 4.0 + 6.0, cell.center().y),
                egui::vec2(12.0, 12.0),
            );
            icon.paint(&ui.painter().with_clip_rect(cell), r, tokens.accent);
        }
    }
    if let Some(sort) = sorted {
        // (Size runs down at first, the largest first, as By size did.)
        d.descending = match d.sort == sort {
            true => !d.descending,
            false => sort == Sort::Size,
        };
        d.sort = sort;
    }
    let rows = d.rows();
    // (A rename of a resource Find hides is let go: its field is nowhere,
    // and the table's keys wait for it.)
    if let Some((k, _)) = d.rename
        && !rows.iter().any(|&r| d.hak.items()[r].key == k)
    {
        d.rename = None;
    }
    let pitch = mg_uikit::row_metrics(ui).1;
    let mut scroll =
        egui::ScrollArea::vertical().id_salt(("hak-rows", d.id)).auto_shrink([false, false]);
    // (A resource chosen otherwise than by a click in the table, its keys
    // among them: brought into sight.)
    let order = Some((d.filter.clone(), d.sort, d.descending));
    if d.anchor != d.scrolled_to || d.scrolled_order != order {
        d.scrolled_to = d.anchor;
        d.scrolled_order = order;
        let items = d.hak.items();
        if let Some(at) = d.anchor.and_then(|a| rows.iter().position(|&r| items[r].key == a)) {
            let seen: Option<(usize, usize)> =
                ui.data(|m| m.get_temp(egui::Id::new(("hak-rows-seen", d.id))));
            if seen.is_none_or(|(from, end)| at <= from || at + 2 >= end) {
                let height = ui.available_height();
                scroll = scroll.vertical_scroll_offset((at as f32 * pitch - height / 2.0).max(0.0));
            }
        }
    }
    let mono = egui::FontId::monospace(12.0);
    let id = d.id;
    // (The rename field takes the keyboard the frame it is first drawn.)
    let mut rename_focus = std::mem::take(&mut d.rename_focus);
    let shown = mg_uikit::show_rows(scroll, ui, rows.len(), |ui, range| {
        ui.data_mut(|m| {
            m.insert_temp(egui::Id::new(("hak-rows-seen", id)), (range.start, range.end))
        });
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut clicked = None;
        let mut lines = look::Lines::default();
        for &r in &rows[range] {
            let item = &d.hak.items()[r];
            let key = item.key;
            let (rect, resp) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), pitch), egui::Sense::click());
            let selected = d.selected.contains(&key);
            let name = key.to_string();
            let words = kind_words(key.restype);
            let size_str = size(item.size);
            resp.widget_info(|| {
                let mut info = egui::WidgetInfo::selected(
                    egui::WidgetType::SelectableLabel,
                    true,
                    selected,
                    &name,
                );
                info.current_text_value = Some(format!("{words}, {size_str}"));
                info
            });
            let state = match (selected, resp.hovered()) {
                (true, _) => look::Cell::Current { ringed: has && d.anchor == Some(key) },
                (false, true) => look::Cell::Hovered,
                (false, false) => look::Cell::Plain,
            };
            look::cell(ui.painter(), &tokens, rect, state);
            let xs = columns(rect.left(), rect.width());
            for x in &xs {
                let cell = egui::Rect::from_x_y_ranges(x.clone(), rect.y_range());
                lines.cell(
                    cell,
                    if state == look::Cell::Plain { state } else { look::Cell::Hovered },
                );
            }
            if let look::Cell::Current { ringed: true } = state {
                lines.cell(rect, state);
            }
            lines.row(rect.left(), rect.right(), rect.bottom());
            let (text, quiet) = match selected {
                true => (tokens.on_fill, tokens.on_fill),
                false => (tokens.text, tokens.text_2),
            };
            let painter = ui.painter().clone();
            let cut = |s: String,
                       f: egui::FontId,
                       c: egui::Color32,
                       x: &std::ops::RangeInclusive<f32>,
                       left: f32,
                       right: bool| {
                let cell = egui::Rect::from_x_y_ranges(x.clone(), rect.y_range());
                let inner = cell.shrink2(egui::vec2(8.0, 0.0)).with_min_x(cell.left() + 8.0 + left);
                let mut job = egui::text::LayoutJob::simple_singleline(s, f, c);
                job.wrap = egui::text::TextWrapping::truncate_at_width(inner.width().max(1.0));
                let galley = painter.layout_job(job);
                let y = cell.center().y - galley.size().y / 2.0;
                let x = if right { inner.right() - galley.size().x } else { inner.left() };
                painter.galley(egui::pos2(x, y), galley, c);
            };
            // The name: its type's icon and the name, or the field it is
            // renamed in.
            let icon = egui::Rect::from_center_size(
                egui::pos2(xs[0].start() + 8.0 + 8.0, rect.center().y),
                egui::vec2(16.0, 16.0),
            );
            // (Its kind's hue with Options › General › Colored icons, unless
            // the row is selected.)
            let kind = crate::browser::kind_icon(key.restype);
            let hued = if selected { quiet } else { mg_uikit::hues::tint(ui.ctx(), kind, quiet) };
            kind.paint(ui.painter(), icon, hued);
            match d.rename.as_mut().filter(|(k, _)| *k == key) {
                Some((_, typed)) => {
                    let cell = egui::Rect::from_x_y_ranges(xs[0].clone(), rect.y_range())
                        .with_min_x(icon.right() + 6.0)
                        .shrink2(egui::vec2(2.0, 1.0));
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(cell));
                    let edit =
                        child.add(mg_uikit::text_field(typed).desired_width(cell.width() - 40.0));
                    if std::mem::take(&mut rename_focus) {
                        edit.request_focus();
                    }
                    child.label(format!(".{}", key.restype));
                    if child.input(|i| i.key_pressed(egui::Key::Escape)) {
                        asked.renamed = Some(None);
                    } else if edit.lost_focus() {
                        asked.renamed = Some(Some((key, typed.clone())));
                    }
                }
                None => cut(name.clone(), mono.clone(), text, &xs[0], 16.0 + 8.0, false),
            }
            cut(words.to_string(), font.clone(), quiet, &xs[1], 0.0, false);
            cut(size_str, font.clone(), text, &xs[2], 0.0, true);
            if let Source::File(p) = &item.source {
                let room = (xs[3].end() - xs[3].start() - 16.0).max(1.0);
                let shown = cut_start(ui, &p.to_string_lossy(), &font, room);
                cut(shown, font.clone(), quiet, &xs[3], 0.0, false);
            }
            if resp.clicked() {
                clicked = Some(key);
            }
            if resp.double_clicked() {
                asked.view = Some(key);
            }
            let resp = if let Source::File(p) = &item.source {
                resp.on_hover_ui(|ui| {
                    ui.label(format!("{name}, added from {}", p.display()));
                })
            } else {
                resp
            };
            mg_uikit::context_menu(&resp, |ui| {
                if ui.button("View").clicked() {
                    asked.view = Some(key);
                }
                if key.restype == ResType::TWODA
                    && ui
                        .button(EDIT_TABLE)
                        .on_hover_text("Edit it in the 2DA editor. The hak's Save writes it.")
                        .clicked()
                {
                    asked.edit_table = Some(key);
                }
                if ui.button(RENAME).clicked() {
                    d.rename = Some((key, key.resref.to_string()));
                    rename_focus = true;
                }
                if ui.button(EXTRACT).clicked() {
                    asked.extract = Some(key);
                }
                if ui.button(REMOVE).clicked() {
                    // (After the table is drawn: its rows are places in
                    // the hak's items as they were.)
                    asked.remove = Some(key);
                }
            });
        }
        lines.paint(ui.painter(), &tokens);
        clicked
    });
    d.rename_focus = rename_focus;
    crate::palette_view::keys_outline_when(ui, shown.inner_rect, has);
    if let Some(key) = shown.inner {
        let (shift, command) = ui.input(|i| (i.modifiers.shift, i.modifiers.command));
        let items = d.hak.items();
        let listed: Vec<ResKey> = rows.iter().map(|&r| items[r].key).collect();
        crate::widgets::select_in_list(
            &mut d.selected,
            &mut d.anchor,
            &listed,
            key,
            command,
            shift,
        );
        d.scrolled_to = d.anchor;
        // (A click shows what it chose, where Close put a resource away.)
        d.closed = None;
        take_keys(&ctx, d.id);
    }
    if rows.is_empty() {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            match (d.hak.items().is_empty(), d.filter.trim().is_empty()) {
                (true, _) => ui.weak("No resources: Add Files… or Add Folder… adds some."),
                (false, true) => ui.weak("Nothing to show."),
                (false, false) => ui.weak("No resource has that name or type."),
            };
        });
    }
    asked
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, id: u32) {
    let Some(i) = app.haks.iter().position(|d| d.id == id) else {
        ui.label("This hak was closed.");
        return;
    };
    let tab = Tab::Hak(id);
    // The game's own haks are never written: Save As makes a copy.
    let game_root = app.install.as_ref().map(|gi| gi.root.clone());
    let doc = &mut app.haks[i];
    doc.hovered = ui.rect_contains_pointer(ui.max_rect());
    let in_game = doc.is_in_game(game_root.as_deref());
    let asked = header(ui, doc, in_game);
    // OK and Cancel at the header's end, as the other editors have them: a
    // hak is saved when asked, so OK saves it and closes it, Cancel closes
    // it as last saved (asked about first, as its × asks); a description
    // typed is set first.
    if let Some(pressed) = asked.pressed
        && crate::side_doc::pressed_ok_cancel(app, &tab, false, pressed, |app| set_typed(app, id))
    {
        return;
    }
    let Some(i) = app.haks.iter().position(|d| d.id == id) else { return };
    // The notice strip: a hak of the game's, and what the game can't read.
    let mut lines = Vec::new();
    if in_game {
        lines.push(GAME_NOTICE.to_string());
    }
    let facts = app.haks[i].facts();
    if facts.past > 0 {
        lines.push(format!(
            "{} resources would start past 2 GiB into the hak, where the game stops reading: \
             split it into two.",
            facts.past
        ));
    }
    notice(ui, &lines);
    // (Why it could not be saved, at the foot.)
    crate::side_doc::failed_foot(app, ui, &tab);
    // (A floating hak's Outliner folded to a strip at the body's left,
    // where its window has not the room for its pane: `shell::window_panes`.)
    let owner = crate::outliner::Owner::Hak(id);
    let strip = app.outliner.strips_now.contains(&owner);
    if strip {
        crate::outliner::strip(app, ui, owner);
    }
    let mut doc = app.haks.remove(i);
    // (Down out of Find: the table has the keys, at its first row.)
    if asked.down {
        take_keys(ui.ctx(), id);
        if let Some(&first) = doc.rows().first() {
            let key = doc.hak.items()[first].key;
            doc.choose(key);
            // (Brought into sight, though it was chosen already.)
            doc.scrolled_to = None;
        }
    }
    let rows = table(app, ui, &mut doc);
    app.haks.insert(i, doc);
    if strip {
        crate::outliner::over(app, ui, owner);
    }
    run(app, ui.ctx(), id, asked, rows);
}

/// What the header and the table asked for, done once both are drawn.
fn run(app: &mut Moonglow, ctx: &egui::Context, id: u32, asked: HeaderAsked, rows: RowAsked) {
    let Some(i) = app.haks.iter().position(|d| d.id == id) else { return };
    let d = &mut app.haks[i];
    let selected: Vec<ResKey> = d.selected.iter().copied().collect();
    if let Some(done) = rows.renamed {
        if let Some((key, name)) = done.filter(|(k, n)| *n != k.resref.to_string()) {
            match d.hak.rename(key, name.trim()) {
                Ok(to) => {
                    d.changed();
                    d.choose(to);
                }
                Err(e) => app.log.error(format!("Rename {key}: {e}")),
            }
        }
        app.haks[i].rename = None;
    }
    let d = &mut app.haks[i];
    if asked.remove && !selected.is_empty() {
        d.hak.remove(&selected);
        d.changed();
    }
    if let Some(key) = rows.remove {
        d.hak.remove(&[key]);
        d.changed();
    }
    if asked.rename
        && let Some(&key) = selected.first()
    {
        d.rename = Some((key, key.resref.to_string()));
        d.rename_focus = true;
        // (Its row brought into sight, where its field is.)
        d.anchor = Some(key);
        d.scrolled_to = None;
    }
    if let Some(key) = rows.view {
        view(app, ctx, id, key);
    }
    let table = rows.edit_table.or(asked.edit_table.then(|| selected.first().copied()).flatten());
    if let Some(key) = table {
        crate::table_view::open_in_hak(app, id, key);
    }
    if let Some(key) = rows.extract {
        extract(app, i, &[key]);
    }
    if asked.extract {
        extract(app, i, &selected);
    }
    if asked.extract_all {
        let all: Vec<ResKey> = app.haks[i].hak.items().iter().map(|i| i.key).collect();
        extract(app, i, &all);
    }
    if asked.save || asked.save_as {
        set_typed(app, id);
        save_as_job(app, id, asked.save_as);
    }
    if asked.compile {
        compile_models(app, i);
    }
    if asked.update {
        update_from_folder(app, i);
    }
    if asked.add_files {
        let files = app.dialogs.open_files(FileKind::HakFiles, None);
        add(app, i, files);
    }
    if asked.add_folder
        && let Some(dir) = app.dialogs.pick_folder("Add Folder", None)
    {
        add(app, i, mg_module::hak_edit::folder_files(&dir));
    }
}

/// View (a double click, the row's menu, Enter): `key` chosen, the
/// Preview brought forward to show it (the Right Dock unfolded), where it
/// was closed shown again.
pub(crate) fn view(app: &mut Moonglow, ctx: &egui::Context, id: u32, key: ResKey) {
    if let Some(d) = app.haks.iter_mut().find(|d| d.id == id) {
        if !d.selected.contains(&key) {
            d.choose(key);
        }
        d.anchor = Some(key);
        d.closed = None;
    }
    // (Docked: the main window's Right Dock unfolded on it. Floating, its
    // window's pane shows it.)
    let tab = Tab::Hak(id);
    if app.dock.find_tab(&tab).is_some_and(|p| p.surface.is_main()) {
        app.actions.push(crate::Action::ShowPanel(crate::shell::Panel::Preview));
    }
    // (The Right Dock was drawn before the editor this frame.)
    ctx.request_repaint();
}

/// Update from Folder (More, Details): what the folder has now, its files
/// in place of the hak's, in one step to undo; what the folder has no file
/// of goes, and is named (a hak added to by hand, or built from several
/// folders, loses it: Undo puts it back). Said in the log.
pub(crate) fn update_from_folder(app: &mut Moonglow, i: usize) {
    let doc = &mut app.haks[i];
    let Some(dir) = doc.folder.clone() else { return };
    let (added, gone) = doc.hak.update_from_folder(&dir);
    doc.changed();
    app.log.info(format!("Read {} again", dir.display()));
    report(app, &added);
    if !gone.is_empty() {
        let names: Vec<String> = gone.iter().take(12).map(ResKey::to_string).collect();
        let more = gone.len().saturating_sub(names.len());
        let more = if more > 0 { format!(" and {more} more") } else { String::new() };
        app.log.warn(format!(
            "Took out {} resources the folder has no file of (Undo puts them back): {}{more}",
            gone.len(),
            names.join(", ")
        ));
    }
}

/// Compile Models: the models of hak `i` kept as text are compiled
/// (`mg_module::models`) as a job, off the window's thread, and put into
/// the hak as one step to undo; those that can't be are named in the log
/// and stay as they are.
fn compile_models(app: &mut Moonglow, i: usize) {
    use mg_module::models::{Compiled, compile_models_each};
    let (id, hak) = (app.haks[i].id, app.haks[i].hak.clone());
    app.start_job_anywhere(
        "Compile Models",
        move |job| -> Vec<Compiled> {
            job.progress.say("Reading the hak's models");
            // (Its materials too: a mesh with no render hint takes its
            // material's.)
            let models: Vec<(ResKey, Vec<u8>)> = (hak.items().iter())
                .filter(|item| {
                    matches!(item.key.restype, mg_core::ResType::MDL | mg_core::ResType::MTR)
                })
                .filter_map(|item| Some((item.key, hak.data(item.key).ok()?)))
                .collect();
            let files: Vec<(ResKey, &[u8])> =
                models.iter().map(|(k, d)| (*k, d.as_slice())).collect();
            let lookup = |name: &str, t: mg_core::ResType| {
                let rm = &job.game.as_deref()?.resman;
                rm.get_named(name, t).ok().map(|d| d.into_owned())
            };
            compile_models_each(&files, &lookup, &|done, total| {
                job.progress.step(done, total);
                job.progress.say(format!("{done} of {total} models"));
                !job.progress.cancelled()
            })
        },
        move |app, results| {
            // (The window took no input meanwhile: the hak is as it was.)
            let Some(i) = app.haks.iter().position(|d| d.id == id) else { return };
            let sorted = crate::model_view::CompiledModels::sort(&mut app.log, "the hak", results);
            let Some(sorted) = sorted else { return };
            let made: Vec<_> = (sorted.made.iter())
                .map(|(k, b)| (*k, mg_module::hak_edit::Source::Bytes(b.clone().into())))
                .collect();
            if !made.is_empty() {
                match app.haks[i].hak.add_sources_as("Compile Models", made) {
                    Ok(_) => app.haks[i].changed(),
                    Err(e) => {
                        app.log.error(format!("Compile Models: {e}"));
                        return;
                    }
                }
            }
            sorted.say(&mut app.log);
        },
    );
}

/// Adds files to hak `i`: at once, unless the hak has some of them
/// already, which is asked about first (as Aurora's hak editor asks).
fn add(app: &mut Moonglow, i: usize, files: Vec<PathBuf>) {
    let replaced = app.haks[i].hak.replaced_by(&files);
    if replaced.is_empty() {
        let added = app.haks[i].hak.add_files(&files);
        app.haks[i].changed();
        report(app, &added);
    } else {
        app.haks[i].adding = Some(Adding { files, replaced });
    }
}

/// Asks about files being added that a hak has already: replace them,
/// leave those out, or add nothing.
pub(crate) fn adding_window(app: &mut Moonglow, ctx: &egui::Context) {
    // (And the files an extraction would write over, the other way.)
    extracting_window(app, ctx);
    let Some(i) = app.haks.iter().position(|d| d.adding.is_some()) else { return };
    let adding = app.haks[i].adding.clone().expect("found");
    let title = app.haks[i].title();
    let (n, all) = (adding.replaced.len(), adding.files.len());
    let said =
        format!("{} already has {n} of the {all} files chosen:", title.trim_end_matches(" *"));
    let files: Vec<_> = (adding.replaced.iter())
        .map(|(path, key)| (key.to_string(), Some(path.display().to_string())))
        .collect();
    let tips = [
        "Add all the files: these take the place of the hak's",
        "Add the other files and leave the hak's as they are",
    ];
    use crate::widgets::Already;
    let there = &mut app.settings.files_there;
    let Some(answer) = crate::widgets::files_already(ctx, there, "Hak", &said, &files, tips) else {
        return;
    };
    app.haks[i].adding = None;
    let replace = match answer {
        Already::Replace => true,
        Already::Skip => false,
        Already::Cancel => return,
    };
    let files: Vec<PathBuf> = if replace {
        adding.files
    } else {
        let skip: Vec<&PathBuf> = adding.replaced.iter().map(|(p, _)| p).collect();
        adding.files.into_iter().filter(|f| !skip.contains(&f)).collect()
    };
    let added = app.haks[i].hak.add_files(&files);
    app.haks[i].changed();
    report(app, &added);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path cut for one font is not taken for another of the same size,
    /// nor for another scale (the review's D9: kept by the size alone).
    #[test]
    fn a_cut_path_is_kept_for_its_font_and_scale() {
        let ctx = egui::Context::default();
        let text = "/home/builder/modules/a-long-name/hak/the_resources_of_the_module.hak";
        let cut = |ctx: &egui::Context, font: &egui::FontId, fresh: bool| {
            let mut out = String::new();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                if fresh {
                    ui.data_mut(|d| {
                        d.remove::<CutStartCache>(egui::Id::new("hak-cut-start-cache"))
                    });
                }
                out = cut_start(ui, text, font, 200.0);
            })
            .textures_delta
            .clear();
            out
        };
        let (proportional, mono) =
            (egui::FontId::proportional(14.0), egui::FontId::monospace(14.0));
        assert_ne!(cut(&ctx, &proportional, true), cut(&ctx, &mono, true), "the fonts differ");
        let _ = cut(&ctx, &proportional, true);
        assert_eq!(cut(&ctx, &mono, false), cut(&ctx, &mono, true), "the monospace font's own");
        // Another scale.
        let _ = cut(&ctx, &proportional, true);
        ctx.set_pixels_per_point(2.0);
        assert_eq!(cut(&ctx, &proportional, false), cut(&ctx, &proportional, true));
    }

    /// A model's facts, as the Preview lists them, read from its text: a
    /// root and a triangle drawn with one texture, no supermodel.
    #[test]
    fn a_model_s_facts_are_counted_from_it() {
        let text = "newmodel zz_box\nsetsupermodel zz_box NULL\nclassification character\n\
beginmodelgeom zz_box\nnode dummy zz_box\n  parent NULL\nendnode\nnode trimesh tri\n\
  parent zz_box\n  bitmap zz_tex\n  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n  tverts 3\n\
    0 0 0\n    1 0 0\n    0 1 0\n  faces 1\n    0 1 2 1 0 1 2 0\nendnode\n\
endmodelgeom zz_box\ndonemodel zz_box\n";
        let model = mg_mdl::Model::read(text.as_bytes()).unwrap();
        let facts = ModelFacts::of(&model, text.as_bytes());
        assert_eq!(
            facts,
            ModelFacts {
                compiled: false,
                size: text.len() as u64,
                nodes: 2,
                faces: 1,
                vertices: 3,
                textures: 1,
                animations: 0,
                supermodel: None,
            }
        );
        let rows = facts.rows();
        assert_eq!(rows[0], ("Type", "Model, text".to_string()));
        assert_eq!(rows[7], ("Supermodel", "(none)".to_string()));
    }

    /// A hak opened through a hak folder that is a link into the game's
    /// folder is the game's: its Save is refused and the module's Save
    /// leaves it, as for one opened by the game's own path. The check
    /// compared the path as written, which passed it (the last pass's
    /// item 10; NUI Images › Into goes by the same function).
    #[test]
    fn a_hak_reached_through_a_link_into_the_game_s_folder_is_the_game_s() {
        let dir = mg_testkit::scratch_dir("hak-view-in-game");
        let (game, user) = (dir.join("game"), dir.join("user"));
        std::fs::create_dir_all(game.join("data/hk")).unwrap();
        std::fs::create_dir_all(user.join("hak")).unwrap();
        let mut doc = HakDoc::new(1, Hak::new());
        assert!(!doc.is_in_game(Some(&game)), "not saved: nowhere");
        doc.hak.path = Some(user.join("hak/mine.hak"));
        assert!(!doc.is_in_game(Some(&game)));
        assert!(!doc.is_in_game(None), "no game: in none");
        doc.hak.path = Some(game.join("data/hk/theirs.hak"));
        assert!(doc.is_in_game(Some(&game)), "the path changed: looked at again");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(game.join("data/hk"), user.join("linked")).unwrap();
            doc.hak.path = Some(user.join("linked/theirs.hak"));
            assert!(doc.is_in_game(Some(&game)), "through the link");
        }
    }
}
