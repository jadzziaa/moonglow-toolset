//! Find References (where a script, area, conversation, blueprint or tag is
//! used, in a tab whose lines go there) and Rename (a resource and
//! everything that names it, as one undoable command).

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit};
use mg_module::dialog::{Kind, Parent, is_link, link_index, links};
use mg_module::rename::{self, Mention, Usage};
use mg_resman::ResKey;

use crate::dialog_view::Row;
use crate::widgets::{autofocus, enter};
use crate::{Action, Moonglow, Tab};

/// What the References tab looks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    Resource(ResKey),
    Tag(String),
    /// A talk-table line, by its StrRef as the files have it.
    StrRef(u32),
}

/// How many resources the window reads into the module's references
/// where they are asked for: more (the first question in a module, or
/// after a file taken in) are read on a thread of their own while the
/// window goes on drawing (a persistent world's first read is 0.2 s).
const READ_HERE: usize = 64;

/// The module's references ([`rename::UsageIndex`]): at hand, or being
/// read on another thread.
#[derive(Debug)]
enum Index {
    Ready(rename::UsageIndex),
    Reading(std::thread::JoinHandle<rename::UsageIndex>),
}

impl Default for Index {
    fn default() -> Index {
        Index::Ready(rename::UsageIndex::default())
    }
}

/// What the module names of the resources about to be deleted (the
/// Delete window's), for the revision it was looked up at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    keys: Vec<ResKey>,
    revision: u64,
    /// The places that name one of them, outside what is deleted with
    /// them.
    pub usages: Vec<Usage>,
    /// Strings in scripts that spell one's name.
    pub mentions: Vec<Mention>,
}

/// The References tab's state.
#[derive(Debug, Default)]
pub struct References {
    pub query: Option<Query>,
    pub usages: Vec<Usage>,
    pub mentions: Vec<Mention>,
    /// Of a script, the cells of the game's tables (as the module sees
    /// them) that run it: a spell's impact script...
    pub run: Vec<mg_module::refs::Reference>,
    /// Of a talk-table line, the cells of the module's haks' 2DAs that
    /// name it.
    pub hak_cells: Vec<rename::HakCell>,
    /// Of a resource, the project file's `[used]` glob that counts it as
    /// used.
    pub marked: Option<String>,
    /// The workspace revision the results are for.
    revision: Option<u64>,
    /// The codepage the scripts' lines were read in (Options › Language).
    codepage: Option<mg_core::Codepage>,
    /// The Find field.
    pub input: String,
    /// The module's references, kept between questions.
    index: Index,
    /// What names the resources the Delete window is about.
    pub named: Option<Named>,
}

impl References {
    /// Looks again at the next frame: what it says changed outside the
    /// module (the project file's `[used]`).
    pub(crate) fn look_again(&mut self) {
        self.revision = None;
    }

    /// The module's references as `module` is now (flushed): read here
    /// where little changed since the last question, else on a thread of
    /// its own (with `background`, as the desktop app runs; tests read
    /// here), and `None` until it is done: ask again at a later frame.
    /// What changed while it was read is read when it is asked again.
    pub(crate) fn index(
        &mut self,
        module: &mg_module::Module,
        background: bool,
    ) -> Option<&rename::UsageIndex> {
        if let Index::Reading(thread) = &self.index {
            if !thread.is_finished() {
                return None;
            }
            let Index::Reading(thread) = std::mem::take(&mut self.index) else { unreachable!() };
            // (A read that failed is read again, here or on a thread.)
            self.index = Index::Ready(thread.join().unwrap_or_default());
        }
        let Index::Ready(index) = &mut self.index else { return None };
        if !background {
            index.refresh(module);
        } else if !index.refresh_at_most(module, READ_HERE) {
            let mut index = std::mem::take(index);
            let copy = module.clone();
            let reading =
                std::thread::Builder::new().name("moonglow references".into()).spawn(move || {
                    index.refresh(&copy);
                    index
                });
            match reading {
                Ok(thread) => {
                    self.index = Index::Reading(thread);
                    return None;
                }
                // (No thread: read here, as before.)
                Err(_) => {
                    let mut index = rename::UsageIndex::default();
                    index.refresh(module);
                    self.index = Index::Ready(index);
                }
            }
        }
        let Index::Ready(index) = &self.index else { return None };
        Some(index)
    }
}

/// The Rename window, while open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDraft {
    pub from: ResKey,
    pub to: String,
    /// Also rename script strings that spell the old name.
    pub literals: bool,
    /// How many places name it, and how many strings in scripts spell
    /// it: `None` while the module's references are read.
    pub counted: Option<(usize, usize)>,
    /// The NUI window's script Rename… was asked of, when `from` is the
    /// window it belongs to.
    pub script: Option<ResKey>,
}

/// The resource a family member stands for (an area's `.git` is the area,
/// a compiled script is its script).
pub fn main_key(key: ResKey) -> ResKey {
    let t = match key.restype {
        ResType::GIT | ResType::GIC => ResType::ARE,
        ResType::NCS | ResType::NDB => ResType::NSS,
        t => t,
    };
    ResKey::new(key.resref, t)
}

impl Moonglow {
    /// Shows where something is used, in the References tab.
    pub fn find_references(&mut self, query: Query) {
        let query = match query {
            Query::Resource(k) => Query::Resource(main_key(k)),
            q => q,
        };
        self.references.input = match &query {
            Query::Resource(k) => k.resref.to_string(),
            Query::Tag(t) => t.clone(),
            Query::StrRef(n) => n.to_string(),
        };
        self.references.query = Some(query);
        self.references.revision = None;
        self.actions.push(Action::OpenTab(Tab::References));
    }

    /// The NUI window whose opener or event script `key` is (`NAME_o.nss`,
    /// `NAME_e.nss` beside `NAME.jui` and its settings): renaming the
    /// script alone would part it from the window.
    pub(crate) fn window_of_script(&self, key: ResKey) -> Option<ResKey> {
        let ws = self.ws.as_ref()?;
        if key.restype != ResType::NSS {
            return None;
        }
        let name = key.resref.to_string();
        let window = name.strip_suffix("_o").or_else(|| name.strip_suffix("_e"))?;
        let jui = ResKey::parse(window, ResType::JUI)?;
        crate::nui_view::companions(&ws.module, jui).contains(&key).then_some(jui)
    }

    /// Opens the Rename window for a resource (for a NUI window's script,
    /// the window's).
    pub fn rename_dialog(&mut self, key: ResKey) {
        let key = main_key(key);
        let script = self.window_of_script(key).map(|_| key);
        let key = self.window_of_script(key).unwrap_or(key);
        let Some(ws) = &mut self.ws else { return };
        // (A NUI window is renamed with what it has beside it: `rename_nui`.)
        if !rename::renamable(key.restype) && key.restype != ResType::JUI {
            self.log.error(format!("{key} can't be renamed"));
            return;
        }
        if ws.flush().is_err() {
            return;
        }
        let mut draft = RenameDraft {
            from: key,
            to: key.resref.to_string(),
            literals: false,
            counted: None,
            script,
        };
        draft.counted = self.count_names(key);
        self.rename = Some(draft);
    }

    /// How many places name `key` and how many strings in scripts spell
    /// it, from the module's references: `None` while they are read.
    fn count_names(&mut self, key: ResKey) -> Option<(usize, usize)> {
        let ws = self.ws.as_ref()?;
        let index = self.references.index(&ws.module, self.background_jobs)?;
        let codepage = crate::text::script_codepage();
        let mentions = index.mentions(&key.resref.to_string(), true, codepage).len();
        Some((index.usages(key).len(), mentions))
    }

    /// Renames a resource everywhere as one undoable command: its family,
    /// what names it, and (with `literals`) script strings; scripts whose
    /// source changed are compiled again.
    pub fn rename_resource(&mut self, from: ResKey, to: ResRef, literals: bool) {
        // (A NUI window's script is renamed with its window, which takes
        // the new name less the script's suffix.)
        if let Some(window) = self.window_of_script(main_key(from)) {
            let suffix = &from.resref.to_string()[window.resref.to_string().len()..];
            let Some(to) =
                to.to_string().strip_suffix(suffix).and_then(|n| ResRef::from_str(n).ok())
            else {
                self.log.error(format!(
                    "{from} is {window}'s script: give a new name ending {suffix}, or rename the \
                     window"
                ));
                return;
            };
            return self.rename_nui(window, to, literals);
        }
        if from.restype == ResType::JUI {
            return self.rename_nui(from, to, literals);
        }
        self.store_script_text();
        // (Compiled as the module's builds are: Options › Script Editor.)
        let (external, debug) = (self.external_compiler(), self.settings.debug_info);
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let mut staged = ws.module.clone();
        let report = match rename::rename(&mut staged, from, to, literals) {
            Ok(r) => r,
            Err(e) => {
                self.log.error(format!("Rename: {e}"));
                return;
            }
        };
        // Compile what lost its compiled code; only those results are kept.
        let mut compiled_ok = true;
        if !report.recompile.is_empty() {
            match &self.game {
                Some(game) => {
                    let results = rename::recompile(
                        &mut staged,
                        &game.resman,
                        &report,
                        external.as_ref(),
                        debug,
                    );
                    for r in &results {
                        if let Err(e) = &r.result
                            && report
                                .recompile
                                .iter()
                                .any(|k| e.message.contains(&k.resref.to_string()))
                        {
                            self.log.error(e.message.clone());
                            compiled_ok = false;
                        }
                    }
                }
                None => compiled_ok = false,
            }
        }
        let new_family: Vec<ResKey> =
            [ResType::NCS, ResType::NDB].iter().map(|t| ResKey::new(to, *t)).collect();
        let keep = |k: &ResKey| {
            !matches!(k.restype, ResType::NCS | ResType::NDB)
                || report.recompile.iter().any(|s| s.resref == k.resref)
                || k.resref == from.resref
                || new_family.contains(k)
        };
        let mut keys: Vec<ResKey> = staged.keys().chain(ws.module.keys()).copied().collect();
        keys.sort();
        keys.dedup();
        let edits: Vec<Edit> = keys
            .into_iter()
            .filter(|k| keep(k) && staged.get(k) != ws.module.get(k))
            .map(|k| Edit::SetResource { key: k, data: staged.get(&k).map(<[u8]>::to_vec) })
            .collect();
        let new = ResKey::new(to, from.restype);
        if let Err(e) = self.apply(Command::new(format!("Rename {from} to {new}"), edits)) {
            self.log.error(e.to_string());
            return;
        }
        self.retarget(from, to);
        self.log.info(format!(
            "Renamed {from} to {new}: {} reference(s), {} in scripts",
            report.references, report.in_scripts
        ));
        self.warn_still_spelled(&from.resref.to_string());
        if !compiled_ok {
            self.log.warn(
                "Some scripts the rename changed aren't compiled; compile them before testing",
            );
        }
        if self.references.query.is_some() {
            self.references.query = Some(Query::Resource(new));
            self.references.input = to.to_string();
            self.references.revision = None;
        }
    }

    /// Says which strings in the module's scripts still spell `name`, a
    /// name a rename just took away.
    fn warn_still_spelled(&mut self, name: &str) {
        let Some(ws) = &self.ws else { return };
        let left = rename::mentions(&ws.module, name, true, crate::text::script_codepage());
        if !left.is_empty() {
            self.log.warn(format!(
                "{} still spelled “{}” in scripts ({}); the game won't find it by that name. \
                 Find References lists them",
                if left.len() == 1 {
                    "A string".to_string()
                } else {
                    format!("{} strings", left.len())
                },
                name,
                left.iter()
                    .map(|m| format!("{} line {}", m.script, m.line))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    /// Renames a NUI window as one undoable command (`mg_nui::rename`):
    /// its JUI, its settings, its opener and its event script, and the
    /// module's scripts that open it; with `literals`, script strings
    /// spelling its name too. A window that was built is built again under
    /// its new name in the same command (`mg_nui::build_renamed`), as the
    /// other renames compile what they changed; where it can't be, the log
    /// says why and asks for a Build. The tabs of the window and of its two
    /// scripts follow.
    fn rename_nui(&mut self, from: ResKey, to: ResRef, literals: bool) {
        self.store_script_text();
        // (Built as the window's Build builds: Options › Script Editor's
        // compiler, the compile memory kept.)
        let external = self.external_compiler();
        let kept = self.remembered_scripts(external.as_ref());
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let (old, name) = (from.resref.to_string(), to.to_string());
        let mut made = match mg_nui::rename(&ws.module, &old, &name, literals) {
            Ok(made) => made,
            Err(e) => {
                self.log.error(format!("Rename: {e}"));
                return;
            }
        };
        // Built under its new name, in the same command: or why not.
        let built = match &self.game {
            Some(game) => mg_nui::build_renamed(
                &ws.module,
                &mut made,
                &name,
                &mg_nui::Build {
                    resman: &game.resman,
                    external: external.as_ref(),
                    debug: self.settings.debug_info,
                    known: Some(&self.script_stamps),
                },
            )
            .map_err(|e| e.to_string()),
            None if made.build => Err("no game install to compile with (Options)".to_owned()),
            None => Ok(None),
        };
        let new = ResKey::new(to, from.restype);
        if let Err(e) = self.apply(made.command) {
            self.log.error(e.to_string());
            return;
        }
        // The window's tab and what it keeps, and the tabs and buffers of
        // its two scripts where they moved with it. (Only those: another
        // resource of the window's name is not the window's.)
        let script =
            |name: &str, suffix: &str| ResKey::parse(&format!("{name}{suffix}"), ResType::NSS);
        let scripts: Vec<(ResKey, ResKey)> = ["_o", "_e"]
            .into_iter()
            .filter_map(|suffix| script(&old, suffix).zip(script(&name, suffix)))
            .filter(|(was, _)| self.ws.as_ref().is_some_and(|ws| !ws.module.contains(was)))
            .collect();
        let moved = |k: &ResKey| scripts.iter().find(|(was, _)| was == k).map(|(_, now)| *now);
        for (_, tab) in self.dock.iter_all_tabs_mut() {
            match tab {
                Tab::Nui(k) if *k == from => *k = new,
                Tab::Script(k) => *k = moved(k).unwrap_or(*k),
                _ => {}
            }
        }
        if let Some(view) = self.nui_views.remove(&from) {
            self.nui_views.insert(new, view);
        }
        // (A buffer is as the module has its script, stored above: it
        // follows the script as the rename left it when its tab is drawn.)
        for (was, now) in &scripts {
            if let Some(buffer) = self.scripts.remove(was) {
                self.scripts.insert(*now, buffer);
            }
        }
        if self.nui_build_error.as_ref().is_some_and(|(n, _)| *n == old) {
            self.nui_build_error = None;
        }
        self.log.info(format!(
            "Renamed {from} to {new}, with its settings and its scripts: {} place(s) in scripts",
            made.in_scripts
        ));
        for spelled in &made.spelled {
            self.warn_still_spelled(spelled);
        }
        match built {
            Ok(Some(b)) => {
                self.log.info(format!(
                    "{new}'s scripts, and the scripts that open it, are compiled under its new \
                     name"
                ));
                crate::nui_view::after_build(
                    self,
                    &name,
                    &b.failed,
                    b.remembered,
                    &b.refits,
                    kept.as_deref(),
                );
            }
            Ok(None) => {}
            Err(why) => self.log.warn(format!(
                "{new}'s scripts, and the scripts that open it, aren't compiled under its new \
                 name ({why}): Build it before testing"
            )),
        }
    }

    /// Points open editors at a renamed resource's new name.
    /// (Those of the resource renamed, and of what goes by its name with
    /// it: an area's three files, a script's compiled ones. A store named
    /// as a script is, or as a NUI window, leaves their editors be.)
    fn retarget(&mut self, from: ResKey, to: ResRef) {
        const TOGETHER: [&[ResType]; 2] = [
            &[ResType::ARE, ResType::GIT, ResType::GIC],
            &[ResType::NSS, ResType::NCS, ResType::NDB],
        ];
        let kind = from.restype;
        let with = move |t: ResType| {
            t == kind || TOGETHER.iter().any(|set| set.contains(&t) && set.contains(&kind))
        };
        let is_area = with(ResType::ARE);
        let from = from.resref;
        let moved = |k: &mut ResKey| {
            if k.resref == from && with(k.restype) {
                k.resref = to;
            }
        };
        let moved_ref = |r: &mut ResRef| {
            if *r == from && is_area {
                *r = to;
            }
        };
        for (_, tab) in self.dock.iter_all_tabs_mut() {
            match tab {
                Tab::Script(k) | Tab::Gff(k) | Tab::Dialog(k) | Tab::Blueprint(k) => moved(k),
                Tab::Nui(k) => moved(k),
                Tab::Area(r) | Tab::AreaProperties(r) => moved_ref(r),
                Tab::Instance { area, .. }
                | Tab::Instances { area, .. }
                | Tab::InstanceModel { area, .. } => moved_ref(area),
                _ => {}
            }
        }
        let old_keys: Vec<ResKey> =
            self.scripts.keys().filter(|k| k.resref == from && with(k.restype)).copied().collect();
        for k in old_keys {
            if let Some(b) = self.scripts.remove(&k) {
                self.scripts.insert(ResKey::new(to, k.restype), b);
            }
        }
        let old_keys: Vec<ResKey> = self
            .dialog_views
            .keys()
            .filter(|k| k.resref == from && with(k.restype))
            .copied()
            .collect();
        for k in old_keys {
            if let Some(v) = self.dialog_views.remove(&k) {
                self.dialog_views.insert(ResKey::new(to, k.restype), v);
            }
        }
        let old_keys: Vec<ResKey> = self
            .nui_views
            .keys()
            .filter(|k| k.resref == from && with(k.restype))
            .copied()
            .collect();
        for k in old_keys {
            if let Some(v) = self.nui_views.remove(&k) {
                self.nui_views.insert(ResKey::new(to, k.restype), v);
            }
        }
        // An area's view is rebuilt under its new name.
        if is_area {
            self.area_views.remove(&from);
        }
        let pages: Vec<_> = (self.blueprint_pages.keys())
            .filter(|(k, _)| k.resref == from && with(k.restype))
            .cloned()
            .collect();
        for (k, p) in pages {
            if let Some(v) = self.blueprint_pages.remove(&(k, p.clone())) {
                self.blueprint_pages.insert((ResKey::new(to, k.restype), p), v);
            }
        }
        if is_area {
            let ids: Vec<_> =
                self.window_ids.keys().filter(|(area, _)| *area == from).cloned().collect();
            for (area, p) in ids {
                if let Some(id) = self.window_ids.remove(&(area, p.clone())) {
                    self.window_ids.insert((to, p), id);
                }
            }
        }
    }

    /// Why `key` can't be deleted, if it can't: the area the module starts
    /// in stays.
    pub(crate) fn undeletable(&mut self, key: ResKey) -> Option<String> {
        let ws = self.ws.as_mut()?;
        let info = ws.doc(&crate::module_props::info_key()).ok()?;
        let start = info.root.resref("Mod_Entry_Area")?;
        (key.restype == ResType::ARE && start == key.resref).then(|| {
            "The module's start location is in this area: set it in another area first \
             (Set Start Location Here in that area's view)."
                .to_string()
        })
    }

    /// Why `key` stays out of a deletion of several (the Delete window of
    /// the resources chosen in the module tree): the start area, and a
    /// protected area (deleted alone, it is asked about first).
    pub(crate) fn kept_from_several(&mut self, key: ResKey) -> Option<String> {
        if self.undeletable(key).is_some() {
            return Some(format!("{key}: the module's start location is in it"));
        }
        let protected = (key.restype == ResType::ARE)
            .then(|| crate::claims::project(self)?.protected(key.resref))
            .flatten();
        protected.map(|why| format!("{why}: delete it alone, to be asked"))
    }

    /// The resources deleting `key` takes from the module: it, an area's
    /// GIT and GIC, a script's compiled one, a NUI window's settings,
    /// opener and event script.
    pub(crate) fn deleted_with(&self, key: ResKey) -> Vec<ResKey> {
        let Some(ws) = &self.ws else { return Vec::new() };
        let with: &[ResType] = match key.restype {
            ResType::ARE => &[ResType::GIT, ResType::GIC],
            ResType::NSS => &[ResType::NCS],
            _ => &[],
        };
        std::iter::once(key)
            .chain(with.iter().map(|t| ResKey::new(key.resref, *t)))
            .chain(crate::nui_view::companions(&ws.module, key))
            .filter(|k| ws.module.contains(k))
            .collect()
    }

    /// Deletes `keys` from the module, as one command: an area with its
    /// objects (its GIT and GIC) and its entry in the module's area list,
    /// a script with its compiled one, a NUI window with its settings and
    /// its two scripts. Their tabs close. The start area stays (said in
    /// the log), and of several, a protected area ([`kept_from_several`]).
    ///
    /// [`kept_from_several`]: Moonglow::kept_from_several
    pub(crate) fn delete_resources(&mut self, keys: &[ResKey]) {
        let mut going: Vec<ResKey> = Vec::new();
        for &key in keys {
            let why = match keys.len() {
                1 => self.undeletable(key),
                _ => self.kept_from_several(key),
            };
            match why {
                Some(why) => self.log.warn(why),
                None if !going.contains(&key) => going.push(key),
                None => {}
            }
        }
        if going.is_empty() {
            return;
        }
        // A script going with them whose editor holds unsaved text: the
        // text is stored first, a step of its own, so that Undo brings the
        // script back as it was typed.
        let mut unsaved: Vec<ResKey> = going.iter().flat_map(|k| self.unsaved_with(*k)).collect();
        unsaved.sort();
        unsaved.dedup();
        if !unsaved.is_empty() {
            let edits = unsaved
                .iter()
                .filter_map(|k| {
                    let data = Some(self.scripts.get_mut(k)?.mark_saved());
                    Some(Edit::SetResource { key: *k, data })
                })
                .collect();
            if let Err(e) = self.apply(Command::new(format!("Edit {}", unsaved[0]), edits)) {
                self.log.error(e.to_string());
                return;
            }
        }
        let mut gone: Vec<ResKey> = going.iter().flat_map(|k| self.deleted_with(*k)).collect();
        gone.sort();
        gone.dedup();
        let areas: Vec<ResRef> =
            going.iter().filter(|k| k.restype == ResType::ARE).map(|k| k.resref).collect();
        let Some(ws) = self.ws.as_mut() else { return };
        let mut edits = Vec::new();
        // The areas' entries in the module's area list, the last first (so
        // that each taken out leaves the others where they are).
        let ifo = crate::module_props::info_key();
        if let Ok(info) = ws.doc(&ifo)
            && let Some(list) = info.root.list("Mod_Area_list")
        {
            let listed =
                |a: &mg_gff::Struct| a.resref("Area_Name").is_some_and(|r| areas.contains(&r));
            let at: Vec<usize> = (0..list.len()).rev().filter(|&i| listed(&list[i])).collect();
            let root = mg_edit::GffPath::root;
            edits
                .extend(at.into_iter().map(|i| Edit::remove_item(ifo, root(), "Mod_Area_list", i)));
        }
        edits.extend(gone.iter().map(|k| Edit::SetResource { key: *k, data: None }));
        let label = match going.as_slice() {
            [key] => format!("Delete {key}"),
            several => format!("Delete {} resources", several.len()),
        };
        if let Err(e) = self.apply(Command::new(label, edits)) {
            self.log.error(e.to_string());
            return;
        }
        let area_gone = |r: &ResRef| areas.contains(r);
        crate::tabs::close_tabs(&mut self.dock, |t| match t {
            Tab::Script(k) | Tab::Gff(k) | Tab::Dialog(k) | Tab::Blueprint(k) | Tab::Nui(k) => {
                !gone.contains(k)
            }
            Tab::Area(r) | Tab::AreaProperties(r) => !area_gone(r),
            Tab::Instance { area: r, .. }
            | Tab::Instances { area: r, .. }
            | Tab::InstanceModel { area: r, .. } => !area_gone(r),
            _ => true,
        });
        for area in areas {
            self.area_views.remove(&area);
        }
    }

    /// What the module names of `keys`, about to be deleted: the places
    /// outside what goes with them that name one (a creature's script, a
    /// door's conversation, a placed object's blueprint, the module's
    /// cached scripts), the strings in scripts that spell one's name, and
    /// the transitions of other areas into a deleted area's objects. From
    /// the module's references, kept in `references.named`: `false` while
    /// they are read.
    pub(crate) fn names_of(&mut self, keys: &[ResKey]) -> bool {
        let Some(ws) = self.ws.as_mut() else { return false };
        let revision = ws.revision();
        if let Some(named) = &self.references.named
            && named.keys == keys
            && named.revision == revision
        {
            return true;
        }
        if ws.flush().is_err() {
            return false;
        }
        let mut gone: Vec<ResKey> = keys.iter().flat_map(|k| self.deleted_with(*k)).collect();
        gone.extend(keys);
        let areas: Vec<ResRef> =
            keys.iter().filter(|k| k.restype == ResType::ARE).map(|k| k.resref).collect();
        let Some(ws) = self.ws.as_ref() else { return false };
        let Some(index) = self.references.index(&ws.module, self.background_jobs) else {
            return false;
        };
        let codepage = crate::text::script_codepage();
        // (Outside what goes with them; an area's entry in the module's
        // area list goes too.)
        let outside = |u: &Usage| {
            !gone.contains(&u.from)
                && !(u.from.restype == ResType::IFO && u.path.starts_with("/Mod_Area_list"))
        };
        let mut usages = Vec::new();
        let mut mentions = Vec::new();
        for &key in keys {
            if rename::renamable(key.restype) {
                usages.extend(index.usages(key).into_iter().filter(outside));
                let name = key.resref.to_string();
                mentions.extend(
                    (index.mentions(&name, true, codepage).into_iter())
                        .filter(|m| !gone.contains(&m.script)),
                );
            }
        }
        for &area in &areas {
            usages.extend(index.links_into(area).into_iter().filter(outside));
        }
        usages.sort_by(|a, b| (a.from, &a.path).cmp(&(b.from, &b.path)));
        usages.dedup();
        mentions.dedup();
        self.references.named = Some(Named { keys: keys.to_vec(), revision, usages, mentions });
        true
    }

    /// The scripts that deleting `key` takes (a script itself, a NUI
    /// window's two) whose editors hold unsaved text.
    pub(crate) fn unsaved_with(&self, key: ResKey) -> Vec<ResKey> {
        let Some(ws) = &self.ws else { return Vec::new() };
        let mut scripts = crate::nui_view::companions(&ws.module, key);
        scripts.push(key);
        scripts.retain(|k| {
            k.restype == ResType::NSS && self.scripts.get(k).is_some_and(|b| b.is_dirty())
        });
        scripts.sort();
        scripts
    }

    /// Opens what a usage names, at the place.
    pub(crate) fn go_to_usage(&mut self, u: &Usage) {
        if let Some((list, index)) = u.instance() {
            let kind = mg_area::ObjectKind::ALL.into_iter().find(|k| k.list() == list);
            if let Some(kind) = kind {
                self.area_focus = Some((u.from.resref, kind, index));
            }
            self.actions.push(Action::OpenTab(Tab::Area(u.from.resref)));
            return;
        }
        match u.from.restype {
            ResType::NSS => self.go_to_line(u.from, u.line().unwrap_or(1)),
            ResType::ARE => self.actions.push(Action::OpenTab(Tab::AreaProperties(u.from.resref))),
            ResType::GIT => self.actions.push(Action::OpenTab(Tab::Area(u.from.resref))),
            ResType::DLG => {
                if let Some(row) = self.ws.as_ref().and_then(|ws| dialog_row(&ws.module, u)) {
                    self.dialog_views.entry(u.from).or_default().selected = Some(row);
                }
                self.actions.push(Action::OpenTab(Tab::Dialog(u.from)));
            }
            _ => {
                if let Some(tab) = Tab::for_resource(u.from) {
                    self.actions.push(Action::OpenTab(tab));
                }
            }
        }
    }

    /// Opens a script at a line (1-based).
    fn go_to_line(&mut self, key: ResKey, line: usize) {
        let text = self.script_source(&key).unwrap_or_default();
        self.script_tools.jump =
            Some((key, crate::script_tools::line_start(&text, line.saturating_sub(1))));
        self.actions.push(Action::OpenTab(Tab::Script(key)));
    }
}

/// The conversation row a usage is in: the link to its node (or the link
/// itself, for a link's condition).
fn dialog_row(module: &mg_module::Module, u: &Usage) -> Option<Row> {
    let g = module.gff(&u.from)?.ok()?;
    let steps: Vec<&str> = u.path.trim_start_matches('/').split('/').collect();
    let item = |s: &str| -> Option<(String, usize)> {
        let (l, rest) = s.split_once('[')?;
        Some((l.to_string(), rest.strip_suffix(']')?.parse().ok()?))
    };
    let (list, index) = item(steps.first()?)?;
    let kind = match list.as_str() {
        "EntryList" => Kind::Entry,
        "ReplyList" => Kind::Reply,
        "StartingList" => return Some(Row { parent: Parent::Root, pos: index }),
        _ => return None,
    };
    // A link's own field (its condition): that link's row.
    if steps.len() > 2
        && let Some((_, pos)) = item(steps[1])
    {
        return Some(Row { parent: Parent::Node(kind, index as u32), pos });
    }
    // Else the link that owns the node (not a link to it), from its parent.
    let parents: Vec<Parent> = match kind {
        Kind::Entry => std::iter::once(Parent::Root)
            .chain((0..count(&g, Kind::Reply)).map(|i| Parent::Node(Kind::Reply, i as u32)))
            .collect(),
        Kind::Reply => {
            (0..count(&g, Kind::Entry)).map(|i| Parent::Node(Kind::Entry, i as u32)).collect()
        }
    };
    let mut fallback = None;
    for parent in parents {
        for (pos, link) in links(&g, parent).iter().enumerate() {
            if link_index(link) as usize == index {
                if !is_link(link) {
                    return Some(Row { parent, pos });
                }
                fallback.get_or_insert(Row { parent, pos });
            }
        }
    }
    fallback
}

fn count(g: &mg_gff::Gff, kind: Kind) -> usize {
    match g.root.get(kind.list()) {
        Some(mg_gff::Value::List(items)) => items.len(),
        _ => 0,
    }
}

/// The References tab.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let Some(ws) = &mut app.ws else {
        ui.label("Open a module first.");
        return;
    };
    // The Find field: a resource name or a tag.
    let mut find = None;
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Find references to");
        let r = ui.add(
            egui::TextEdit::singleline(&mut app.references.input)
                .hint_text("a script, area, conversation, blueprint, tag or StrRef")
                .desired_width(260.0),
        );
        let submitted = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("Find").clicked() || submitted {
            find = Some(app.references.input.trim().to_string());
        }
    });
    if let Some(name) = find.filter(|n| !n.is_empty()) {
        let resource = ResRef::from_str(&name).ok().and_then(|r| {
            let mut keys: Vec<ResKey> = ws
                .module
                .keys()
                .filter(|k| k.resref == r && rename::renamable(k.restype))
                .map(|k| main_key(*k))
                .collect();
            keys.sort();
            keys.dedup();
            keys.first().copied()
        });
        // (A number that is no resource's name: a talk-table line.)
        app.references.query = Some(match (resource, name.parse::<u32>()) {
            (Some(k), _) => Query::Resource(k),
            (None, Ok(strref)) => Query::StrRef(strref),
            (None, Err(_)) => Query::Tag(name),
        });
        app.references.revision = None;
    }
    let Some(query) = app.references.query.clone() else {
        ui.weak("Choose Find References on a script, area, conversation or blueprint, or type a name or tag above.");
        return;
    };
    // (The scripts' lines read again in another language's codepage.)
    let codepage = crate::text::script_codepage();
    let mut reading = false;
    if app.references.revision != Some(ws.revision()) || app.references.codepage != Some(codepage) {
        if ws.flush().is_err() {
            return;
        }
        // (From the index: only what changed since the last question is
        // read again; the first question's read is on a thread of its own,
        // waited for below.)
        let found =
            app.references.index(&ws.module, app.background_jobs).map(|index| match &query {
                Query::Resource(k) => {
                    (index.usages(*k), index.mentions(&k.resref.to_string(), true, codepage))
                }
                Query::Tag(t) => (index.tag_usages(t), index.mentions(t, false, codepage)),
                Query::StrRef(n) => (index.strref_usages(*n), Vec::new()),
            });
        reading = found.is_none();
        if let Some((usages, mentions)) = found {
            app.references.usages = usages;
            app.references.mentions = mentions;
        }
    }
    if reading {
        ui.separator();
        ui.horizontal(|ui| {
            ui.spinner();
            ui.weak("Reading the module's references…");
        });
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        return;
    }
    if app.references.revision != Some(ws.revision()) || app.references.codepage != Some(codepage) {
        // (The tables are the game data's, kept read: a scan of their
        // script columns.)
        app.references.run = match (&query, app.game.as_deref()) {
            (Query::Resource(k), Some(game)) if k.restype == ResType::NSS => {
                let read = |name: &str| game.table(name).ok();
                let mut run = mg_module::verify::table_scripts(&read);
                run.extend(mg_module::verify::described_scripts(&ws.module, Some(&game.resman)));
                run.retain(|r| r.target == k.resref);
                run
            }
            _ => Vec::new(),
        };
        app.references.hak_cells = match (&query, app.game.as_deref()) {
            (Query::StrRef(n), Some(game)) => rename::hak_strrefs(&game.resman, *n),
            _ => Vec::new(),
        };
        app.references.marked = match &query {
            Query::Resource(k) => {
                ws.module.project_file.used.marking(k.resref, k.restype).map(String::from)
            }
            _ => None,
        };
        app.references.revision = Some(ws.revision());
        app.references.codepage = Some(codepage);
    }
    ui.separator();
    let mut go = None;
    let mut go_line = None;
    let mut go_table = None;
    let mut rename_key = None;
    ui.horizontal(|ui| match &query {
        Query::Resource(k) => {
            ui.heading(format!("{k}"));
            if ui
                .button("Rename…")
                .on_hover_text("Rename it and everything that names it")
                .clicked()
            {
                rename_key = Some(*k);
            }
        }
        Query::Tag(t) => _ = ui.heading(format!("Tag “{t}”")),
        Query::StrRef(n) => {
            ui.heading(format!("Talk-table line {n}")).on_hover_text(
                "Where the module's blueprints, areas, conversations, palettes, soundsets, \
                 2DAs and scripts name this line, and its haks' 2DAs",
            );
        }
    });
    // (Which tables that run it are the module's own, to open.)
    let own_tables: Vec<bool> =
        app.references.run.iter().map(|r| ws.module.contains(&r.from)).collect();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        // An area's objects are said to be in the area as the rest of the
        // window names it (Options › General: Show areas by name).
        let shown = |app: &mut Moonglow, u: &Usage| {
            let of_area = matches!(u.from.restype, ResType::ARE | ResType::GIT | ResType::GIC);
            if !of_area || !app.settings.area_names {
                return u.place.clone();
            }
            let name = crate::tabs::area_label(app, u.from.resref);
            u.place.replacen(&u.from.resref.to_string(), &name, 1)
        };
        // (Only the rows in sight are named and laid out: a script that
        // thousands of creatures use is thousands of rows. The others
        // take a row's room.)
        let row = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
        let out_of_sight = |ui: &mut Ui| {
            let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(1.0, row));
            let hidden = !ui.is_rect_visible(room);
            if hidden {
                ui.add_space(row);
            }
            hidden
        };
        // (Taken out while an area's name is looked up, which keeps it.)
        let usages = std::mem::take(&mut app.references.usages);
        if usages.is_empty() {
            ui.weak("Nothing in the module names it.");
        } else {
            ui.label(format!("Used in {}:", places(usages.len())));
            for (i, u) in usages.iter().enumerate() {
                if out_of_sight(ui) {
                    continue;
                }
                let label = shown(app, u);
                if ui.link(label).on_hover_text(format!("{} {}", u.from, u.path)).clicked() {
                    go = Some(i);
                }
            }
        }
        app.references.usages = usages;
        let refs = &app.references;
        if !refs.run.is_empty() {
            ui.add_space(8.0);
            ui.label(format!("Run by the game, from its 2DAs ({}):", refs.run.len()))
                .on_hover_text(
                    "Cells of the tables the game runs scripts from (a spell's impact script, \
                     an area of effect's, a state's, a trap's, a disease's, a poison's), as the \
                     module sees them: its own 2DAs and its haks' over the game's; and of the \
                     module's own tables whose description says a column holds a script",
                );
            for (r, own) in refs.run.iter().zip(&own_tables) {
                if out_of_sight(ui) {
                    continue;
                }
                let text = format!("{} › {}", r.from, r.path);
                if *own {
                    if ui.link(text).on_hover_text("Open the module's table").clicked() {
                        go_table = Some(r.from);
                    }
                } else {
                    ui.label(text);
                }
            }
        }
        if !refs.hak_cells.is_empty() {
            ui.add_space(8.0);
            ui.label(format!("In the haks' 2DAs ({}):", refs.hak_cells.len())).on_hover_text(
                "Cells of the module's haks' tables, in the columns known to hold a talk-table \
                 line (a class's name, a feat's, a spell's…): each hak's own copy, the one the \
                 game reads and those under it",
            );
            for c in &refs.hak_cells {
                if out_of_sight(ui) {
                    continue;
                }
                ui.label(c.place());
            }
        }
        if let Some(glob) = &refs.marked {
            ui.add_space(8.0);
            ui.label(format!("Counted as used by the project file's [used]: {glob}"))
                .on_hover_text("Build Module's Unused and mg verify --unused don't list it");
        }
        if !refs.mentions.is_empty() {
            ui.add_space(8.0);
            ui.label(format!("Spelled out in scripts ({}):", refs.mentions.len())).on_hover_text(
                "Strings in scripts with this name: the script may use it, or mean something else",
            );
            for m in &refs.mentions {
                if out_of_sight(ui) {
                    continue;
                }
                if ui.link(format!("{} line {}: {}", m.script, m.line, m.text)).clicked() {
                    go_line = Some((m.script, m.line));
                }
            }
        }
    });
    if let Some(i) = go {
        let u = app.references.usages[i].clone();
        app.go_to_usage(&u);
    }
    if let Some((k, line)) = go_line {
        app.go_to_line(k, line);
    }
    if let Some(k) = go_table {
        crate::table_view::open_in_module(app, k);
    }
    if let Some(k) = rename_key {
        app.rename_dialog(k);
    }
}

/// "1 place", "3 places".
fn places(n: usize) -> String {
    if n == 1 { "1 place".into() } else { format!("{n} places") }
}

/// How many of what names the resources the Delete window lists, each
/// kind: the rest are counted.
const NAMED_SHOWN: usize = 12;

/// The module tree's Delete…, of one resource or of those chosen: asks
/// first (an area's objects go with it), and says what in the module
/// names them, which is left as it is.
pub(crate) fn delete_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(asked) = app.confirm_delete.clone() else { return };
    // (Of several: those that stay, and why; one alone says why it can't.)
    let (keys, kept): (Vec<ResKey>, Vec<String>) = match asked.as_slice() {
        [key] => (vec![*key], Vec::new()),
        several => {
            let mut keys = Vec::new();
            let mut kept = Vec::new();
            for &k in several {
                match app.kept_from_several(k) {
                    Some(why) => kept.push(why),
                    None => keys.push(k),
                }
            }
            (keys, kept)
        }
    };
    let why_not = match asked.as_slice() {
        [key] => app.undeletable(*key),
        _ if keys.is_empty() => Some(format!("None of them can be deleted: {}.", kept.join("; "))),
        _ => None,
    };
    let mut unsaved: Vec<ResKey> = keys.iter().flat_map(|k| app.unsaved_with(*k)).collect();
    unsaved.sort();
    unsaved.dedup();
    let named = why_not.is_none() && app.names_of(&keys);
    if why_not.is_none() && !named {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    let named = named.then(|| app.references.named.take()).flatten();
    let mut open = true;
    let mut find = None;
    crate::widgets::modal(ctx, "Delete from Module").show(ctx, |ui| {
        if let Some(why) = &why_not {
            ui.label(why);
            if ui.button("OK").clicked() {
                open = false;
            }
            return;
        }
        ui.label(match keys.as_slice() {
            [key] if key.restype == ResType::ARE => {
                format!("Delete the area {} and everything placed in it?", key.resref)
            }
            [key] if key.restype == ResType::NSS => {
                format!("Delete {key} and its compiled script?")
            }
            [key] if key.restype == ResType::JUI && app.deleted_with(*key).len() > 1 => {
                format!("Delete the NUI window {key} with its settings and its scripts?")
            }
            [key] => format!("Delete {key}?"),
            several => {
                let of = |t: ResType| several.iter().filter(|k| k.restype == t).count();
                let area = match of(ResType::ARE) {
                    0 => None,
                    1 => Some("the area with everything placed in it"),
                    _ => Some("the areas with everything placed in them"),
                };
                let script = match of(ResType::NSS) {
                    0 => None,
                    1 => Some("the script with its compiled one"),
                    _ => Some("the scripts with their compiled ones"),
                };
                let what: Vec<&str> = area.into_iter().chain(script).collect();
                let what = match what.is_empty() {
                    true => String::new(),
                    false => format!(" ({})", what.join(", ")),
                };
                format!("Delete these {} resources{what}?", several.len())
            }
        });
        if keys.len() > 1 {
            let names: Vec<String> = keys.iter().map(ToString::to_string).collect();
            egui::ScrollArea::vertical()
                .id_salt("delete-several")
                .max_height(120.0)
                .show(ui, |ui| ui.weak(names.join(", ")));
        }
        for why in &kept {
            ui.weak(format!("Not deleted: {why}."));
        }
        named_ui(ui, named.as_ref(), &mut find);
        if !unsaved.is_empty() {
            let names: Vec<String> = unsaved.iter().map(ToString::to_string).collect();
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!(
                    "{} {} unsaved text in {}: {} deleted as typed, and the {} closed.",
                    names.join(" and "),
                    if unsaved.len() == 1 { "has" } else { "have" },
                    if unsaved.len() == 1 { "its editor" } else { "their editors" },
                    if unsaved.len() == 1 { "it is" } else { "they are" },
                    if unsaved.len() == 1 { "editor" } else { "editors" },
                ),
            );
        }
        ui.weak(match keys.len() {
            1 => "Edit › Undo brings it back.",
            _ => "Edit › Undo brings them back.",
        });
        ui.horizontal(|ui| {
            // (Once what names them is known, to be said first.)
            let delete = ui.add_enabled(named.is_some(), egui::Button::new("Delete"));
            if delete.clicked() {
                app.actions.push(Action::DeleteResources(keys.clone()));
                open = false;
            }
            if crate::widgets::cancel(ui) {
                open = false;
            }
        });
    });
    if named.is_some() {
        app.references.named = named;
    }
    if let Some(key) = find {
        app.find_references(Query::Resource(key));
        open = false;
    }
    if !open {
        app.confirm_delete = None;
    }
}

/// What names the resources about to be deleted, in the Delete window:
/// the places, the strings in scripts, each with a link to Find
/// References of the resource they name (`find`); or that it is being
/// looked for.
fn named_ui(ui: &mut Ui, named: Option<&Named>, find: &mut Option<ResKey>) {
    let Some(named) = named else {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.weak("Reading the module's references…");
        });
        return;
    };
    let one = named.keys.len() == 1;
    let them = if one { "it" } else { "them" };
    if named.usages.is_empty() && named.mentions.is_empty() {
        ui.weak(format!("Nothing else in the module names {them}."));
        return;
    }
    let warn = ui.visuals().warn_fg_color;
    if !named.usages.is_empty() {
        let n = named.usages.len();
        let name = if n == 1 { "names" } else { "name" };
        ui.colored_label(
            warn,
            format!("{} in the module {name} {them}, left as they are:", places(n)),
        );
        for u in named.usages.iter().take(NAMED_SHOWN) {
            ui.label(format!("  {}", u.place)).on_hover_text(format!("{} {}", u.from, u.path));
        }
        if n > NAMED_SHOWN {
            ui.weak(format!("  and {} more", n - NAMED_SHOWN));
        }
    }
    if !named.mentions.is_empty() {
        let n = named.mentions.len();
        ui.colored_label(warn, format!("Spelled out in scripts ({n}), which may mean {them}:"));
        for m in named.mentions.iter().take(NAMED_SHOWN) {
            ui.label(format!("  {} line {}: {}", m.script, m.line, m.text));
        }
        if n > NAMED_SHOWN {
            ui.weak(format!("  and {} more", n - NAMED_SHOWN));
        }
    }
    if one && ui.link("Find References").on_hover_text("Open the References tab").clicked() {
        *find = named.keys.first().copied();
    }
}

/// The Rename window.
pub(crate) fn rename_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut draft) = app.rename.take() else { return };
    // (Counted once the module's references are read.)
    if draft.counted.is_none() {
        draft.counted = app.count_names(draft.from);
        if draft.counted.is_none() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
    let mut open = true;
    let mut cancel = false;
    let mut done = false;
    let family = crate::copy_as::family(draft.from.restype);
    let exists = |to: &ResRef| {
        app.ws
            .as_ref()
            .is_some_and(|ws| family.iter().any(|t| ws.module.contains(&ResKey::new(*to, *t))))
    };
    // A NUI window's name leaves room for its scripts' suffixes, and its
    // settings and scripts need theirs free too.
    let nui_problem = |to: &ResRef| {
        let ws = app.ws.as_ref().filter(|_| draft.from.restype == ResType::JUI)?;
        mg_nui::can_rename(&ws.module, &draft.from.resref.to_string(), &to.to_string()).err()
    };
    crate::widgets::dialog(ctx, &format!("Rename {}", draft.from), &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "New name");
                let r = ui.add(egui::TextEdit::singleline(&mut draft.to).desired_width(200.0));
                autofocus(ui, &r);
            });
            let name = draft.to.trim();
            let parsed = ResRef::from_str(name);
            // (message, whether it's a warning rather than a hint)
            let problem = match &parsed {
                _ if name.is_empty() => Some(("Type the new name".to_string(), false)),
                Err(e) => Some((e.to_string(), true)),
                Ok(r) if *r == draft.from.resref => Some(("Type the new name".to_string(), false)),
                Ok(r) if exists(r) => Some((format!("{name} already exists"), true)),
                Ok(r) => nui_problem(r).map(|e| (e.to_string(), true)),
            };
            if let Some(script) = draft.script {
                ui.label(format!(
                    "{script} is the {} of the NUI window {}: the window is renamed, and the \
                     script takes its new name.",
                    if script.resref.to_string().ends_with("_o") {
                        "opener"
                    } else {
                        "event script"
                    },
                    draft.from.resref
                ));
            }
            let mentions = draft.counted.map_or(0, |(_, m)| m);
            match draft.counted {
                _ if draft.from.restype == ResType::JUI => _ = ui.label(
                    "Its settings and its scripts are renamed with it, and the scripts that \
                     open it are pointed at the new name.",
                ),
                None => _ = ui.horizontal(|ui| {
                    ui.spinner();
                    ui.weak("Looking for what names it…");
                }),
                Some((0, _)) => _ = ui.label("Nothing else in the module names it."),
                Some((n, _)) => _ = ui.label(format!(
                    "{} in the module {} it; they will name the new one.",
                    places(n),
                    if n == 1 { "names" } else { "name" }
                )),
            }
            if mentions > 0 {
                ui.checkbox(
                    &mut draft.literals,
                    format!(
                        "Also change the {} spelling “{}” in scripts",
                        if mentions == 1 { "string".to_string() } else { format!("{mentions} strings") },
                        draft.from.resref
                    ),
                )
                .on_hover_text("Strings like ExecuteScript(\"name\") or CreateObject(…, \"name\", …); a string may also mean something else, so check them in Find References first");
            }
            match &problem {
                Some((p, true)) => _ = ui.colored_label(ui.visuals().warn_fg_color, p),
                Some((p, false)) => _ = ui.weak(p),
                None => {}
            }
            // (Renamed once what names it is known: the strings it offers
            // to change with it.)
            let ready = problem.is_none()
                && (draft.counted.is_some() || draft.from.restype == ResType::JUI);
            ui.horizontal(|ui| {
                let ok = ui.add_enabled(ready, egui::Button::new("Rename"));
                if ok.clicked() || (ready && enter(ui)) {
                    done = true;
                }
                if crate::widgets::cancel(ui) {
                    cancel = true;
                }
            });
        });
    if done {
        if let Ok(to) = ResRef::from_str(draft.to.trim()) {
            app.rename_resource(draft.from, to, draft.literals);
        }
    } else if open && !cancel {
        app.rename = Some(draft);
    }
}
