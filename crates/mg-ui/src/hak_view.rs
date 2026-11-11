//! The hak editor: a hak's resources, to add files and folders to, remove,
//! rename and extract, with undo, saved as a new archive moved over the old
//! one (`mg_module::hak_edit`). Tools › New Hak, Open Hak… and Build Hak
//! from Folder… open it. Saving a hak the module uses reloads it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use egui::Ui;
use mg_module::hak_edit::{Added, Hak, Source};
use mg_resman::ResKey;

use crate::dialogs::FileKind;
use crate::icons::Icon;
use crate::{Moonglow, Tab};

/// A hak open in a tab.
#[derive(Debug)]
pub struct HakDoc {
    pub id: u32,
    pub hak: Hak,
    pub filter: String,
    pub selected: BTreeSet<ResKey>,
    /// The resource clicked last: Shift + click selects from it to the one
    /// clicked.
    anchor: Option<ResKey>,
    /// The load order with the hak as it is now in it, the model viewer's
    /// to draw a model of it from ([`resources`]): made when a model is
    /// shown, let go at a change and when the game data is to change.
    pub(crate) resources: Option<std::sync::Arc<mg_resman::ResMan>>,
    /// The resource being renamed, and its new name as typed.
    pub rename: Option<(ResKey, String)>,
    /// The description as typed (with plain line ends).
    description: Option<String>,
    /// Bumped on every change, for the cached rows.
    revision: u64,
    rows: Option<(String, Sort, u64, Vec<usize>)>,
    /// The order of the list.
    pub sort: Sort,
    /// The resource shown under the list.
    viewing: Option<(ResKey, Viewing)>,
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

/// A hak's resource as it is shown under the list: a picture (a TGA, DDS
/// or PLT), a sound to play, a 2DA's rows and columns, a model drawn (the
/// copy the game reads being this hak's), or as the resource browser shows
/// the rest (fields, text, a compiled model as its text, the first bytes).
pub(crate) enum Viewing {
    Picture(egui::TextureHandle),
    Sound(std::sync::Arc<[u8]>),
    /// (With which reading of it this is: `browser::made`.)
    Table(Box<mg_module::table_layers::Layered>, u64),
    /// Drawn in the model viewer: from the load order where the game reads
    /// this copy (`true`: it can pop out into a tab of its own), else from
    /// the hak as it is now over the load order ([`resources`]).
    Model(bool),
    Plain(crate::browser::Plain),
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
            Viewing::Model(_) => "model",
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
    use mg_core::ResType;
    let game = app.game.clone();
    let rm = game.as_deref().map(|g| &g.resman);
    let plain = |data: &[u8]| Viewing::Plain(crate::browser::plain(key, data, rm));
    let picture = |rgba: mg_image::Rgba| {
        let rgba = rgba.top_down();
        let size = [rgba.width as usize, rgba.height as usize];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.data);
        let name = format!("hak-view-{key}");
        Viewing::Picture(ctx.load_texture(name, image, egui::TextureOptions::LINEAR))
    };
    match key.restype {
        ResType::TGA | ResType::DDS => match mg_image::read(key.restype, &data) {
            Ok(t) => picture(t.to_rgba()),
            Err(e) => Viewing::Plain(crate::browser::Plain::Error(e.to_string())),
        },
        ResType::PLT => match mg_image::plt::Plt::read(&data) {
            // (In each layer's first color, from the game's palettes.)
            Ok(plt) => {
                let palettes: Vec<Option<std::sync::Arc<mg_image::Rgba>>> = match app.loader() {
                    Some(mut l) => mg_image::plt::PALETTES.iter().map(|p| l.palette(p)).collect(),
                    None => vec![None; 10],
                };
                let refs: [Option<&mg_image::Rgba>; 10] =
                    std::array::from_fn(|i| palettes.get(i).and_then(|p| p.as_deref()));
                picture(plt.colorize(&refs, [0; 10]))
            }
            Err(e) => Viewing::Plain(crate::browser::Plain::Error(e.to_string())),
        },
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
        ResType::MDL if app.game.is_some() && mg_mdl::Model::read(&data).is_ok() => {
            let read = app.game.as_deref().is_some_and(|g| {
                let stem = app.haks[doc].hak.path.as_deref().and_then(|p| p.file_stem());
                let layer = stem.map(|s| format!("hak:{}", s.to_string_lossy()));
                g.resman.get(&key).is_ok_and(|d| *d == data[..])
                    && layer.is_some_and(|l| {
                        g.resman.origin(&key).is_some_and(|o| o.eq_ignore_ascii_case(&l))
                    })
            });
            Viewing::Model(read)
        }
        _ => plain(&data),
    }
}

/// `add` in the part of `ui` that is in sight: a window of the hak
/// editor scrolls sideways (its list's long paths make it wider than the
/// window), and a model view as wide as all of it drew its model in the
/// middle of that, off to the right of the window.
fn in_sight<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let seen = ui.clip_rect().right() - ui.cursor().left();
    let width = ui.available_width().min(seen).max(32.0);
    ui.allocate_ui(egui::vec2(width, ui.available_height()), add).inner
}

/// Files chosen to add to a hak that has some of them already.
#[derive(Debug, Clone, PartialEq)]
pub struct Adding {
    pub files: Vec<PathBuf>,
    /// Those that would replace a resource of the hak, and which.
    pub replaced: Vec<(PathBuf, ResKey)>,
}

/// The order of a hak's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    Name,
    /// By type, then name.
    Type,
    /// The largest first.
    Size,
}

impl HakDoc {
    fn new(id: u32, hak: Hak) -> HakDoc {
        HakDoc {
            id,
            hak,
            filter: String::new(),
            selected: BTreeSet::new(),
            anchor: None,
            resources: None,
            rename: None,
            description: None,
            revision: 0,
            rows: None,
            sort: Sort::Name,
            viewing: None,
            folder: None,
            adding: None,
            extracting: None,
            in_game: None,
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

    /// What the resource shown under the list is shown as (`picture`,
    /// `sound`, `table`, `model`, `plain`), and which.
    pub fn viewing(&self) -> Option<(ResKey, &'static str)> {
        self.viewing.as_ref().map(|(k, v)| (*k, v.kind()))
    }

    pub fn title(&self) -> String {
        let name = self
            .hak
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("New hak".into(), |n| n.to_string_lossy().into_owned());
        format!("{name}{}", if self.hak.is_dirty() { " *" } else { "" })
    }

    /// The items shown, in the list's order: indices into the hak's items.
    fn rows(&mut self) -> Vec<usize> {
        let needle = mg_core::text::Needle::new(&self.filter);
        let filter = needle.as_str().to_string();
        let fresh = self
            .rows
            .as_ref()
            .is_some_and(|(f, s, r, _)| *f == filter && *s == self.sort && *r == self.revision);
        if !fresh {
            let items = self.hak.items();
            let mut rows: Vec<usize> =
                (0..items.len()).filter(|&i| needle.finds(&items[i].key.to_string())).collect();
            // (A name made once an item, not at every comparison: 300,000
            // for a hak of 20,000, at each letter typed into Find.)
            rows.sort_by_cached_key(|&i| items[i].key.to_string());
            match self.sort {
                Sort::Name => {}
                // (Stable: by name within a type or a size.)
                Sort::Type => rows.sort_by_cached_key(|&i| items[i].key.restype.to_string()),
                Sort::Size => rows.sort_by_key(|&i| std::cmp::Reverse(items[i].size)),
            }
            self.rows = Some((filter, self.sort, self.revision, rows));
        }
        self.rows.as_ref().map(|r| r.3.clone()).unwrap_or_default()
    }

    /// Whether the load order its models are drawn from holds the game
    /// data ([`resources`]: made again whenever that changes, so it is
    /// the window's).
    pub(crate) fn holds_game(&self) -> bool {
        self.resources.as_ref().is_some_and(|rm| rm.layers().len() > 1)
    }

    /// How many times the hak changed in its editor (a save too): what is
    /// made from its resources is kept while this stays.
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn changed(&mut self) {
        self.revision += 1;
        self.description = None;
        let keys: BTreeSet<ResKey> = self.hak.items().iter().map(|i| i.key).collect();
        self.selected.retain(|k| keys.contains(k));
        self.viewing = None;
        self.resources = None;
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
    app.start_job_to_its_end(
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

/// The least room the hak's path is shown in at the end of its row: less,
/// and it begins the next row (cut to a bare ellipsis it said nothing).
const PATH_LEAST: f32 = 120.0;

fn size(n: u64) -> String {
    match n {
        n if n >= 1 << 30 => format!("{:.2} GB", n as f64 / f64::from(1u32 << 30)),
        n if n >= 1 << 20 => format!("{:.1} MB", n as f64 / f64::from(1u32 << 20)),
        n if n >= 1 << 10 => format!("{:.1} KB", n as f64 / 1024.0),
        n => format!("{n} bytes"),
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, id: u32) {
    let Some(i) = app.haks.iter().position(|d| d.id == id) else {
        ui.label("This hak was closed.");
        return;
    };
    // OK and Cancel, as the other editors have: a hak is saved when asked.
    if crate::side_doc::ok_cancel(app, ui, &crate::Tab::Hak(id), false, |_| {}) {
        return;
    }
    // What the toolbar asks for, done once the doc is let go.
    enum Do {
        AddFiles,
        AddFolder,
        Extract(Vec<ResKey>),
        Save(bool),
        View(ResKey),
        EditTable(ResKey),
        UpdateFromFolder,
        CompileModels,
    }
    let mut todo = None;
    // The game's own haks are never written: Save As makes a copy.
    let game_root = app.install.as_ref().map(|gi| gi.root.clone());
    // (The editor's buttons: their words, or their icons alone with Icons
    // only, and where the words have no room in the editor's width.)
    let tools = crate::icons::Tools::of(&app.settings);
    let doc = &mut app.haks[i];
    let in_game = doc.is_in_game(game_root.as_deref());
    let mut changed = false;
    if in_game {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "This hak is part of the game: Save As… keeps your changes in a copy.",
        );
    }
    // (The first row: what the hak holds, then its two buttons; the path
    // after them where it has room, cut short, else on a row of its own.
    // The words give way to the icons before the row wraps, the count
    // measured with them.)
    let where_ = doc.hak.path.as_ref().map_or("not saved yet".into(), |p| p.display().to_string());
    let count = format!("{} resources, {}", doc.hak.items().len(), size(doc.hak.size()));
    let folder = doc.folder.as_ref().map(|f| f.display().to_string());
    let first_row = [
        tools.word_widget(Icon::Update, "Update from Folder"),
        tools.word_widget(Icon::Compile, "Compile Models"),
    ];
    let count_width = {
        let font = egui::FontSelection::Default.resolve(ui.style());
        let bold = egui::WidgetText::from(egui::RichText::new(&count).strong());
        let galley = bold.into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, font);
        galley.size().x + ui.spacing().item_spacing.x
    };
    let first = tools.fitted_after(ui, count_width, &first_row);
    ui.horizontal_wrapped(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.strong(&count);
        let update = first.word_widget(Icon::Update, "Update from Folder");
        let tip = match &folder {
            Some(f) => format!("The hak's files again from {f}: what the folder has now"),
            None => "For a hak built with Build Hak from Folder".into(),
        };
        // (Named in its tip with Icons only, disabled too.)
        let r = ui.add_enabled(folder.is_some(), update);
        if first.named(r, "Update from Folder", &tip).clicked() {
            todo = Some(Do::UpdateFromFolder);
        }
        // (Here, not among the buttons under: that row is full.)
        let compile = first.word_widget(Icon::Compile, "Compile Models");
        let tip =
            "Compile the hak's ASCII models against their supermodels, a failed one kept as text";
        if first.named(ui.add(compile), "Compile Models", tip).clicked() {
            todo = Some(Do::CompileModels);
        }
        // (Cut short where the editor is narrow, whole in its tip: a long
        // path widened the editor past its window, and the rows under it
        // with it. Where what is left of the row is too narrow for some of
        // it, it begins the next row.)
        let row_begun = ui.cursor().left() > ui.max_rect().left() + 1.0;
        if row_begun && ui.available_size_before_wrap().x < PATH_LEAST {
            ui.end_row();
        }
        let path = egui::Label::new(egui::RichText::new(&where_).weak());
        ui.add(path.wrap_mode(egui::TextWrapMode::Truncate)).on_hover_text(&where_);
    });
    ui.horizontal(|ui| {
        ui.label("Description");
        let stored = &doc.hak.description;
        let text = doc.description.get_or_insert_with(|| crate::text::to_editor(stored).0);
        let field = egui::TextEdit::multiline(text)
            .desired_rows(2)
            .desired_width(ui.available_width().min(480.0))
            .hint_text("for people: the game doesn't show it");
        let r =
            crate::widgets::text_area(ui, ui.id().with("hak-description"), field).response.response;
        crate::widgets::leaves_at_a_press(ui.ctx(), &r);
        if r.lost_focus() && *text != crate::text::to_editor(stored).0 {
            let t = crate::text::with_line_ends(text, stored);
            doc.hak.set_description(&t);
            changed = true;
        }
    });
    // The toolbar: its words, else (narrow, or Icons only) its icons
    // alone; a second row where even they have no room (Save As… was cut
    // off at the editor's edge).
    let selected: Vec<ResKey> = doc.selected.iter().copied().collect();
    let remove = format!("Remove ({})", selected.len());
    let row = [
        tools.word_widget(Icon::Import, "Add Files…"),
        tools.word_widget(Icon::Folder, "Add Folder…"),
        tools.word_widget(Icon::Delete, &remove),
        tools.word_widget(Icon::Export, "Extract…"),
        tools.word_widget(Icon::ExtractAll, "Extract All…"),
        tools.word_widget(Icon::Undo, "Undo"),
        tools.word_widget(Icon::Redo, "Redo"),
        tools.word_widget(Icon::Save, "Save"),
        tools.word_widget(Icon::SaveAs, "Save As…"),
    ];
    let tools = tools.fitted(ui, &row);
    ui.horizontal_wrapped(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        let button = |ui: &mut Ui, enabled: bool, icon, name: &str, tip: &str| {
            let r = ui.add_enabled(enabled, tools.word_widget(icon, name));
            tools.named(r, name, tip).clicked()
        };
        if button(ui, true, Icon::Import, "Add Files…", "") {
            todo = Some(Do::AddFiles);
        }
        if button(ui, true, Icon::Folder, "Add Folder…", "Its files and those of the folders in it")
        {
            todo = Some(Do::AddFolder);
        }
        let any = !selected.is_empty();
        if button(ui, any, Icon::Delete, &remove, "Take the selected resources out of the hak") {
            doc.hak.remove(&selected);
            changed = true;
        }
        if button(ui, any, Icon::Export, "Extract…", "Write the selected resources to a folder") {
            todo = Some(Do::Extract(selected));
        }
        let all = !doc.hak.items().is_empty();
        if button(ui, all, Icon::ExtractAll, "Extract All…", "Write every resource to a folder") {
            todo = Some(Do::Extract(doc.hak.items().iter().map(|i| i.key).collect()));
        }
        let undo = doc.hak.undo_label().map(|l| format!("Undo {l}"));
        let redo = doc.hak.redo_label().map(|l| format!("Redo {l}"));
        if button(ui, undo.is_some(), Icon::Undo, "Undo", &undo.unwrap_or_default()) {
            doc.hak.undo();
            changed = true;
        }
        if button(ui, redo.is_some(), Icon::Redo, "Redo", &redo.unwrap_or_default()) {
            doc.hak.redo();
            changed = true;
        }
        if button(ui, doc.hak.is_dirty() && !in_game, Icon::Save, "Save", "") {
            todo = Some(Do::Save(false));
        }
        if button(ui, true, Icon::SaveAs, "Save As…", "") {
            todo = Some(Do::Save(true));
        }
    });
    let past = doc.hak.past_read_limit();
    if !past.is_empty() {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!("{} resources would start past 2 GiB into the hak, where the game stops reading: split it into two.", past.len()),
        );
    }
    ui.horizontal(|ui| {
        ui.label("Find");
        let field = ui.id().with("hak-filter-field");
        crate::widgets::SearchField::new(field, &mut doc.filter)
            .hint("name or .type")
            .width(200.0)
            .find_over()
            .show(ui);
        let sorts = [Sort::Name, Sort::Type, Sort::Size];
        crate::widgets::dropdown(("hak-sort", id))
            .selected_text(match doc.sort {
                Sort::Name => "By name",
                Sort::Type => "By type",
                Sort::Size => "By size",
            })
            .show_with(
                ui,
                &mut doc.sort,
                || sorts.to_vec(),
                |ui, sort| {
                    ui.selectable_value(sort, Sort::Name, "By name");
                    ui.selectable_value(sort, Sort::Type, "By type");
                    ui.selectable_value(sort, Sort::Size, "By size")
                        .on_hover_text("The largest first");
                },
            );
        let hint = "Click to select, Ctrl+click to add; double-click to view; right-click for more";
        ui.add(egui::Label::new(egui::RichText::new(hint).weak()).truncate()).on_hover_text(hint);
    });
    if changed {
        doc.changed();
        changed = false;
    }
    let rows = doc.rows();
    let row_height = mg_uikit::row_metrics(ui).0; // (rows of controls, none between)
    let mut rename_done = None;
    let mut remove = None;
    // The resource being viewed takes the lower part of the tab.
    let list_height = match doc.viewing {
        Some(_) => (ui.available_height() * 0.45).max(120.0),
        None => ui.available_height(),
    };
    let list = egui::ScrollArea::vertical().id_salt(("hak-rows", id)).max_height(list_height);
    mg_uikit::show_rows(list.auto_shrink([false, false]), ui, rows.len(), |ui, range| {
        for &r in &rows[range] {
            let item = doc.hak.items()[r].clone();
            ui.horizontal(|ui| {
                if let Some((key, name)) = doc.rename.as_mut().filter(|(k, _)| *k == item.key) {
                    let edit = ui.add(mg_uikit::text_field(name).desired_width(160.0));
                    edit.request_focus();
                    ui.label(format!(".{}", key.restype));
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        rename_done = Some(None);
                    } else if edit.lost_focus() {
                        rename_done = Some(Some((*key, name.clone())));
                    }
                    return;
                }
                let selected = doc.selected.contains(&item.key);
                let name = egui::RichText::new(item.key.to_string()).monospace();
                let layout = egui::Layout::left_to_right(egui::Align::Center);
                let r = ui
                    .allocate_ui_with_layout(egui::vec2(240.0, row_height), layout, |ui| {
                        ui.set_min_width(240.0);
                        ui.add(egui::Button::selectable(selected, name))
                    })
                    .inner;
                if r.clicked() {
                    let (shift, command) = ui.input(|i| (i.modifiers.shift, i.modifiers.command));
                    let listed: Vec<ResKey> =
                        rows.iter().map(|&r| doc.hak.items()[r].key).collect();
                    crate::widgets::select_in_list(
                        &mut doc.selected,
                        &mut doc.anchor,
                        &listed,
                        item.key,
                        command,
                        shift,
                    );
                }
                if r.double_clicked() {
                    todo = Some(Do::View(item.key));
                }
                r.context_menu(|ui| {
                    mg_uikit::menu_rows(ui);
                    if ui.button("View").clicked() {
                        todo = Some(Do::View(item.key));
                    }
                    if item.key.restype == mg_core::ResType::TWODA
                        && ui
                            .button("Edit Table")
                            .on_hover_text("Edit it in the 2DA editor. The hak's Save writes it.")
                            .clicked()
                    {
                        todo = Some(Do::EditTable(item.key));
                    }
                    if ui.button("Rename…").clicked() {
                        doc.rename = Some((item.key, item.key.resref.to_string()));
                    }
                    if ui.button("Extract…").clicked() {
                        todo = Some(Do::Extract(vec![item.key]));
                    }
                    if ui.button("Remove").clicked() {
                        // (After the list is drawn: its rows are places in
                        // the hak's items as they were.)
                        remove = Some(item.key);
                    }
                });
                ui.add_sized([90.0, row_height], egui::Label::new(size(item.size)));
                match &item.source {
                    Source::Archive | Source::Bytes(_) => {}
                    Source::File(p) => {
                        ui.weak(format!("from {}", p.display()));
                    }
                }
            });
        }
    });
    // (Drawn below, with the window's state: a table's columns, the model
    // viewer, the speakers.)
    let shown = doc.viewing.take();
    if let Some(key) = remove {
        doc.hak.remove(&[key]);
        changed = true;
    }
    if let Some(done) = rename_done {
        if let Some((key, name)) = done.filter(|(k, n)| *n != k.resref.to_string()) {
            match doc.hak.rename(key, name.trim()) {
                Ok(to) => {
                    doc.selected = [to].into();
                    changed = true;
                }
                Err(e) => app.log.error(format!("Rename {key}: {e}")),
            }
        }
        let doc = &mut app.haks[i];
        doc.rename = None;
    }
    if changed {
        app.haks[i].changed();
    } else if let Some((key, view)) = shown {
        ui.separator();
        let mut close = false;
        ui.horizontal(|ui| {
            ui.strong(key.to_string());
            close = ui.small_button("Close").clicked();
            if let Viewing::Sound(data) = &view {
                let name = key.resref;
                let sound = crate::audio::Sound { name, data: data.clone() };
                let playing = app.speaker.playing(crate::audio::Channel::Preview);
                if ui.button(if playing { "Stop" } else { "Play" }).clicked() {
                    if playing {
                        app.stop_sound(crate::audio::Channel::Preview);
                    } else {
                        app.speaker.play(crate::audio::Channel::Preview, sound, 1.0, false);
                    }
                }
            }
        });
        match &view {
            Viewing::Picture(texture) => {
                let size = texture.size_vec2();
                ui.weak(format!("{} by {} pixels", size.x, size.y));
                // (At its own size, or smaller to fit the room under the
                // list.)
                let room = ui.available_size();
                let fit = (room.x / size.x.max(1.0)).min(room.y / size.y.max(1.0)).min(1.0);
                ui.add(egui::Image::new(texture).fit_to_exact_size(size * fit.max(0.01)));
            }
            Viewing::Sound(data) => {
                ui.weak(format!("A sound, {} bytes: Play plays it", data.len()));
                // (Asked each frame, so only of a RIFF file, whose chunks
                // answer at once: an MP3's first frame is searched for.)
                let wave = || {
                    data.starts_with(b"RIFF")
                        && mg_audio::container(data)
                            .is_some_and(|(c, _)| c == mg_audio::Container::Wave)
                };
                if key.restype == mg_core::ResType::BMU && wave() {
                    ui.weak("The game does not play a WAVE under .bmu: music must be MP3");
                }
            }
            Viewing::Table(table, made) => {
                let label = table.copies[0].0.clone();
                crate::browser::two_da(app, ui, key, Some(&label), (table, *made));
            }
            Viewing::Model(true) => {
                use crate::model_view::Source;
                let popped =
                    in_sight(ui, |ui| crate::model_view::embedded(app, ui, Source::Resource(key)));
                if popped {
                    app.actions.push(crate::Action::OpenTab(Tab::Model(key)));
                }
            }
            Viewing::Model(false) => {
                use crate::model_view::Source;
                resources(app, i);
                let (hak, revision) = (app.haks[i].id, app.haks[i].revision);
                // (Its views of the hak as it was, or of a hak closed, go.)
                let open: Vec<u32> = app.haks.iter().map(|d| d.id).collect();
                app.model_views.retain(|s, _| match s {
                    Source::Hak { hak: h, revision: r, .. } => {
                        open.contains(h) && (*h != hak || *r == revision)
                    }
                    _ => true,
                });
                let source = Source::Hak { hak, revision, key };
                in_sight(ui, |ui| crate::model_view::embedded(app, ui, source));
            }
            Viewing::Plain(plain) => crate::browser::plain_ui(ui, key, plain),
        }
        let doc = &mut app.haks[i];
        if !close && doc.viewing.is_none() {
            doc.viewing = Some((key, view));
        }
    }
    match todo {
        None => {}
        Some(Do::AddFiles) => {
            let files = app.dialogs.open_files(FileKind::HakFiles, None);
            add(app, i, files);
        }
        Some(Do::AddFolder) => {
            if let Some(dir) = app.dialogs.pick_folder("Add Folder", None) {
                add(app, i, mg_module::hak_edit::folder_files(&dir));
            }
        }
        Some(Do::EditTable(key)) => {
            let id = app.haks[i].id;
            crate::table_view::open_in_hak(app, id, key);
        }
        Some(Do::Extract(keys)) => extract(app, i, &keys),
        Some(Do::Save(ask)) => save_as_job(app, id, ask),
        Some(Do::View(key)) => match app.haks[i].hak.data(key) {
            Ok(data) => {
                let view = viewing(app, ui.ctx(), i, key, data);
                app.haks[i].viewing = Some((key, view));
            }
            Err(e) => app.log.error(format!("{key}: {e}")),
        },
        Some(Do::CompileModels) => compile_models(app, i),
        Some(Do::UpdateFromFolder) => {
            // What the folder has now: its files in place of the hak's, in
            // one step to undo; what the folder has no file of goes, and
            // is named (a hak added to by hand, or built from several
            // folders, loses it: Undo puts it back).
            let doc = &mut app.haks[i];
            if let Some(dir) = doc.folder.clone() {
                let (added, gone) = doc.hak.update_from_folder(&dir);
                doc.changed();
                app.log.info(format!("Read {} again", dir.display()));
                report(app, &added);
                if !gone.is_empty() {
                    let names: Vec<String> = gone.iter().take(12).map(ResKey::to_string).collect();
                    let more = gone.len().saturating_sub(names.len());
                    let more = if more > 0 { format!(" and {more} more") } else { String::new() };
                    app.log.warn(format!(
                        "Took out {} resources the folder has no file of (Undo puts them back): \
                         {}{more}",
                        gone.len(),
                        names.join(", ")
                    ));
                }
            }
        }
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
