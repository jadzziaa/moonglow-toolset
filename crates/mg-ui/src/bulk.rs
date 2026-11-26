//! Bulk edits: Update Instances for any set of blueprints (a selection in
//! the custom palette, a whole category, or one blueprint's editor), as
//! Aurora's palette does it for one: every area or the one shown, with the
//! objects it would change listed to untick.

use mg_uikit::Controls as _;
use std::collections::HashSet;

use egui::Context;
use mg_area::ObjectKind;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Struct;
use mg_module::instances::{Held, held_items, update, update_held};
use mg_resman::ResKey;

use crate::{Action, Moonglow};

/// A placed object Update Instances would change.
#[derive(Debug, Clone, PartialEq)]
pub struct Affected {
    pub area: ResRef,
    /// Its GIT list and index there (an item held by a placed object:
    /// that object's).
    pub list: &'static str,
    pub index: usize,
    /// An item held by a placed object (in a chest, a creature's pack or
    /// hands, a store, a bag): where, down from the GIT's root. Empty for
    /// the placed objects themselves.
    pub held: Held,
    /// `area › creature GUARD (Guard Captain)`.
    pub label: String,
    /// Ticked: updated.
    pub on: bool,
}

/// The Update Instances window.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateDraft {
    pub blueprints: Vec<ResKey>,
    /// `Some`: only this area (the one shown), else every area.
    pub only: Option<ResRef>,
    /// The area shown, offered as the only one.
    pub shown: Option<ResRef>,
    pub objects: Vec<Affected>,
    /// How many times each listed area's GIT had its lists rewritten as
    /// the objects were listed (`Workspace::rewrites`): they are held by
    /// their places in those lists, which are others' once an object is
    /// put in or taken out.
    listed_at: Vec<(ResRef, u64)>,
    /// Which listing it is (counted across the window's life): the rows
    /// shown of it are kept for it and the area chosen.
    listing: u64,
    /// The ticked rows of the area chosen, counted for it as the window
    /// last changed a tick (`None` once it changes one): not counted
    /// each frame of 30,000 listed.
    ticked: Option<(Option<ResRef>, usize)>,
}

impl UpdateDraft {
    /// Whether an area's objects were put in or taken out since they
    /// were listed: a place listed may be another object's now.
    pub fn stale(&self, ws: &mg_edit::Workspace) -> bool {
        self.listed_at.iter().any(|(a, n)| ws.rewrites(&ResKey::new(*a, ResType::GIT)) != *n)
    }
}

/// What the window and the log say of a list that is no longer the
/// module's.
const UPDATE_STALE: &str = "Objects were added to or removed from these areas since they were \
     listed";

/// The ticked objects of a draft, to ask of each object of an area
/// whether it is one: the placed objects by area, list and place, and
/// the items held by area and where. (Asked of every object of every
/// area listed: looked up in the list itself, Update All Instances of
/// 30,000 among 90,000 placed took thousands of millions of steps.)
struct Ticked<'a> {
    placed: HashSet<(ResRef, &'a str, usize)>,
    held: HashSet<(ResRef, &'a [(&'static str, usize)])>,
}

impl<'a> Ticked<'a> {
    fn of(objects: &'a [Affected]) -> Ticked<'a> {
        let mut t = Ticked { placed: HashSet::new(), held: HashSet::new() };
        for o in objects.iter().filter(|o| o.on) {
            if o.held.is_empty() {
                t.placed.insert((o.area, o.list, o.index));
            } else {
                t.held.insert((o.area, o.held.as_slice()));
            }
        }
        t
    }

    fn placed(&self, area: ResRef, list: &str, index: usize) -> bool {
        self.placed.contains(&(area, list, index))
    }

    fn held(&self, area: ResRef, at: &[(&'static str, usize)]) -> bool {
        self.held.contains(&(area, at))
    }
}

/// The kind of object a GIT list holds, for people.
fn kind_name(list: &str) -> &'static str {
    ObjectKind::from_list(list).map_or("object", ObjectKind::name)
}

impl Moonglow {
    /// Opens Update Instances for `blueprints` (the module's): what it
    /// would change, in every area. Logs it when nothing is placed from
    /// them.
    pub fn update_instances_of(&mut self, blueprints: Vec<ResKey>) {
        let Some(ws) = self.ws.as_mut() else { return };
        let wanted: HashSet<ResKey> = blueprints.iter().copied().collect();
        let restypes: HashSet<ResType> = wanted.iter().map(|k| k.restype).collect();
        let mut objects = Vec::new();
        for area in mg_module::areas::every(&ws.module) {
            let Ok(git) = ws.doc(&ResKey::new(area, ResType::GIT)) else { continue };
            for &restype in &restypes {
                let Some(kind) = ObjectKind::from_restype(restype) else { continue };
                let list = kind.list();
                for (index, s) in git.root.list(list).unwrap_or(&[]).iter().enumerate() {
                    let Some(r) = s.resref(kind.resref_field()) else { continue };
                    if !wanted.contains(&ResKey::new(r, restype)) {
                        continue;
                    }
                    let tag = crate::text::decode(s.string("Tag").unwrap_or_default());
                    let tag = if tag.is_empty() { format!("#{}", index + 1) } else { tag };
                    let label = format!("{area} › {} {tag} ({r})", kind_name(list));
                    objects.push(Affected { area, list, index, held: Vec::new(), label, on: true });
                }
            }
            // Items of these blueprints that the area's objects hold: in
            // chests, creatures' packs and hands, stores, and bags.
            let of_item = |r: ResRef| wanted.contains(&ResKey::new(r, ResType::UTI));
            for (held, r, tag) in held_items(&git.root, &of_item) {
                let (list, index) = held[0];
                let holder = git.root.list(list).and_then(|l| l.get(index));
                let holder_tag = holder
                    .map(|s| crate::text::decode(s.string("Tag").unwrap_or_default()))
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| format!("#{}", index + 1));
                let tag = if tag.is_empty() { r.to_string() } else { tag };
                let how = if held.iter().any(|(l, _)| *l == "Equip_ItemList") {
                    "equipped"
                } else {
                    "held"
                };
                let label =
                    format!("{area} › {} {holder_tag} › {how} item {tag} ({r})", kind_name(list));
                objects.push(Affected { area, list, index, held, label, on: true });
            }
        }
        let names: Vec<String> = blueprints.iter().map(ResKey::to_string).collect();
        if objects.is_empty() {
            let names = crate::text::some_of(&names, 7);
            self.log.info(format!("Update Instances: nothing is placed from {names}"));
            return;
        }
        let shown = self.palette.area;
        let mut areas: Vec<ResRef> = objects.iter().map(|o| o.area).collect();
        areas.dedup();
        let listed_at =
            areas.into_iter().map(|a| (a, ws.rewrites(&ResKey::new(a, ResType::GIT)))).collect();
        static LISTINGS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let listing = LISTINGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.update_draft = Some(UpdateDraft {
            blueprints,
            only: None,
            shown,
            objects,
            listed_at,
            listing,
            ticked: None,
        });
    }

    /// Opens Update Instances for every blueprint of the module (Edit ›
    /// Update All Instances…), as `mg update-instances` with none named.
    pub fn update_all_instances(&mut self) {
        let Some(ws) = self.ws.as_ref() else { return };
        let mut blueprints: Vec<ResKey> = ws
            .module
            .keys()
            .copied()
            .filter(|k| ObjectKind::from_restype(k.restype).is_some())
            .collect();
        blueprints.sort();
        if blueprints.is_empty() {
            self.log.info("Update Instances: the module has no blueprints");
            return;
        }
        self.update_instances_of(blueprints);
    }

    /// Makes the ticked objects again from their blueprints, as one
    /// command; how many.
    pub fn apply_update(&mut self, draft: &UpdateDraft) -> usize {
        let Some(ws) = self.ws.as_mut() else { return 0 };
        // (Its objects are places in the areas' lists as they were: an
        // unticked one would be made again in another's place.)
        if draft.stale(ws) {
            self.log.warn(format!("Update Instances: {UPDATE_STALE}: nothing updated"));
            return 0;
        }
        let wanted: HashSet<ResKey> = draft.blueprints.iter().copied().collect();
        let mut blueprints = std::collections::HashMap::new();
        for k in &wanted {
            if let Ok(g) = ws.doc(k) {
                blueprints.insert(*k, g.root.clone());
            }
        }
        let areas: Vec<ResRef> = {
            let mut a: Vec<ResRef> = draft.objects.iter().map(|o| o.area).collect();
            a.dedup();
            a.into_iter().filter(|a| draft.only.is_none_or(|only| only == *a)).collect()
        };
        let gits: Vec<(ResKey, Struct)> = areas
            .iter()
            .filter_map(|a| {
                let key = ResKey::new(*a, ResType::GIT);
                ws.doc(&key).ok().map(|g| (key, g.root.clone()))
            })
            .collect();
        let blueprint = |t: ResType, r: ResRef| blueprints.get(&ResKey::new(r, t)).cloned();
        let ticked = Ticked::of(&draft.objects);
        // (The items the objects hold, as their editors have them.)
        let made = crate::area_view::placing(self, |placing, _| {
            let mut edits = Vec::new();
            let mut total = 0;
            for (key, git) in &gits {
                let (key, area) = (*key, key.resref);
                let which = |list: &str, index: usize| ticked.placed(area, list, index);
                // The items held first (where they are is by their places
                // now), then the placed objects.
                let held = |at: &[(&'static str, usize)]| ticked.held(area, at);
                let item = |r: ResRef| blueprint(ResType::UTI, r);
                let (with_held, held_n) =
                    update_held(placing, git, &item, &held).unwrap_or_else(|| (git.clone(), 0));
                let (new, n) = update(placing, &with_held, &blueprint, &which)
                    .map_or((with_held, held_n), |(new, n)| (new, n + held_n));
                if n == 0 {
                    continue;
                }
                total += n;
                for list in ObjectKind::ALL.map(ObjectKind::list) {
                    if new.get(list) != git.get(list) {
                        edits.push(Edit::SetField {
                            key,
                            path: GffPath::root(),
                            label: list.to_string(),
                            value: new.get(list).cloned(),
                        });
                    }
                }
            }
            (edits, total)
        });
        let Some((edits, total)) = made else { return 0 };
        if total > 0 {
            let what = match draft.blueprints.as_slice() {
                [one] => format!("Update instances of {}", one.resref),
                many => format!("Update instances of {} blueprints", many.len()),
            };
            self.actions.push(Action::Apply(Command::new(what, edits)));
            self.log.info(format!(
                "Updated {total} {} from {} {}:",
                if total == 1 { "object" } else { "objects" },
                draft.blueprints.len(),
                if draft.blueprints.len() == 1 { "blueprint" } else { "blueprints" },
            ));
            // Which ones, as Aurora's log lists them: the area, the kind,
            // the tag and the blueprint of each (a long list is cut short).
            const LISTED: usize = 200;
            let updated: Vec<&Affected> =
                draft.objects.iter().filter(|o| o.on && areas.contains(&o.area)).collect();
            for o in updated.iter().take(LISTED) {
                self.log.info(format!("  {}", o.label));
            }
            if updated.len() > LISTED {
                self.log.info(format!("  … and {} more", updated.len() - LISTED));
            }
        }
        total
    }
}

/// The Update Instances window.
pub(crate) fn update_window(app: &mut Moonglow, ctx: &Context) {
    let Some(mut draft) = app.update_draft.take() else { return };
    let mut open = true;
    let (mut ok, mut cancel, mut again) = (false, false, false);
    let footed = crate::widgets::dialog(ctx, "Update Instances", &mut open)
        .collapsible(false)
        .show_foot(ctx, |ui| {
            let names: Vec<String> =
                draft.blueprints.iter().map(|k| k.resref.to_string()).collect();
            let shown_names = crate::text::some_of(&names, 7);
            ui.label(format!(
                "Makes each object placed from {shown_names} again from its blueprint, \
                 where it stands: its tag, name, scripts and variables become the blueprint's."
            ));
            ui.horizontal(|ui| {
                ui.radio_choice(&mut draft.only, None, "Every area");
                if let Some(area) = draft.shown {
                    ui.radio_choice(&mut draft.only, Some(area), format!("Only {area}"));
                }
            });
            let in_scope = |o: &Affected, only: Option<ResRef>| only.is_none_or(|a| a == o.area);
            ui.horizontal(|ui| {
                if ui.small_button("All").clicked() {
                    draft.objects.iter_mut().for_each(|o| o.on = true);
                    draft.ticked = None;
                }
                if ui.small_button("None").clicked() {
                    draft.objects.iter_mut().for_each(|o| o.on = false);
                    draft.ticked = None;
                }
            });
            let only = draft.only;
            // (The rows in sight alone are laid out: Update All Instances
            // lists tens of thousands in a large module. Those of the area
            // chosen, kept for the listing: its objects are listed once.)
            type ShownKey = (Option<ResRef>, u64);
            let shown_key: ShownKey = (only, draft.listing);
            let shown_id = egui::Id::new("bulk-update-shown");
            let shown: std::sync::Arc<Vec<usize>> = if let Some((was, cached)) =
                ui.data(|d| d.get_temp::<(ShownKey, std::sync::Arc<Vec<usize>>)>(shown_id))
                && was == shown_key
            {
                cached
            } else {
                let list: Vec<usize> = (0..draft.objects.len())
                    .filter(|&i| in_scope(&draft.objects[i], only))
                    .collect();
                let cached = std::sync::Arc::new(list);
                ui.data_mut(|d| d.insert_temp(shown_id, (shown_key, cached.clone())));
                cached
            };
            let rows = egui::ScrollArea::vertical().max_height(320.0).auto_shrink([false, true]);
            mg_uikit::show_rows(rows, ui, shown.len(), |ui, range| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                for &i in &shown[range] {
                    let o = &mut draft.objects[i];
                    if ui.check_box(&mut o.on, &o.label).changed() {
                        draft.ticked = None;
                    }
                }
            });
            let n = match draft.ticked {
                Some((was, n)) if was == only => n,
                _ => {
                    let n = shown.iter().filter(|&&i| draft.objects[i].on).count();
                    draft.ticked = Some((only, n));
                    n
                }
            };
            let stale = app.ws.as_ref().is_some_and(|ws| draft.stale(ws));
            if stale {
                ui.colored_label(ui.visuals().warn_fg_color, format!("{UPDATE_STALE}."));
            }
            let mut foot = crate::widgets::Foot::new().main(format!("Update {n}"), n > 0 && !stale);
            if stale {
                foot = foot.button("List Again", true);
            }
            ((), foot.cancel())
        });
    if let Some(f) = footed {
        ok = f.pressed.main();
        again = f.pressed.clicked("List Again");
        cancel = f.pressed.cancelled();
    }
    if again {
        // (The same blueprints and scope, everything ticked: what was
        // unticked is by places that are no more.)
        let only = draft.only;
        app.update_instances_of(draft.blueprints);
        if let Some(d) = &mut app.update_draft {
            d.only = only;
        }
        return;
    }
    if ok {
        app.apply_update(&draft);
    } else if open && !cancel {
        app.update_draft = Some(draft);
    }
}

/// Find and Replace Text (Edit › Find and Replace Text…; M4's U28: Search
/// Results' form and rows, before it a window of its own): the module's
/// player-facing strings, and where ticked its tags, resrefs, variables
/// and other text fields (`mg_module::text`), found as typed or by a
/// regular expression.
#[derive(Debug, Clone, PartialEq)]
pub struct TextReplace {
    pub find: String,
    pub with: String,
    pub options: mg_module::text::Options,
    pub kinds: Vec<mg_module::text::TextKind>,
    /// The strings found for `searched`, each ticked to be replaced.
    pub hits: Vec<(mg_module::text::Hit, bool)>,
    /// What the hits were found for (the text and how).
    pub searched: Option<(String, mg_module::text::Options)>,
    /// The module's revision as they were found, and the resources they
    /// are in: one changed since may have another struct where a hit's
    /// path leads.
    found_at: (u64, Vec<ResKey>),
    /// Whether they are stale, worked out for the module's revision it
    /// holds (Search Results' badge and form ask each frame: not every
    /// resource with a hit looked at each frame).
    stale_seen: std::cell::Cell<Option<(u64, bool)>>,
    /// Why the text asked for is not a regular expression, if it is not
    /// one, and what that was worked out for (not compiled every frame).
    checked: Option<((String, mg_module::text::Options), Option<String>)>,
    /// The hit the keys are on (Search Results' rows: Up and Down move it,
    /// Space ticks it, Enter opens it).
    pub(crate) current: Option<usize>,
    /// The hits' rows laid out last frame (those in sight).
    pub(crate) strings_seen: (usize, usize),
    /// The ticked hits and the times they hold, counted as the ticks
    /// change, not each frame.
    ticked: (usize, usize),
}

impl TextReplace {
    /// Whether a resource with a hit in it changed since the hits were
    /// found.
    pub fn stale(&self, ws: &mg_edit::Workspace) -> bool {
        let now = ws.revision();
        if let Some((seen, stale)) = self.stale_seen.get()
            && seen == now
        {
            return stale;
        }
        let (at, keys) = &self.found_at;
        let stale = keys.iter().any(|k| ws.changed_at(k) > *at);
        self.stale_seen.set(Some((now, stale)));
        stale
    }

    /// Whether the hits are of what is asked now (its text and options).
    pub(crate) fn asked_now(&self) -> bool {
        self.searched.as_ref().is_some_and(|(f, o)| *f == self.find && *o == self.options)
    }

    /// Why Find what is not a regular expression where it is asked to be
    /// one, worked out again only as the text or the options change.
    pub(crate) fn regex_problem(&mut self) -> Option<String> {
        let now = (self.find.clone(), self.options);
        if !self.checked.as_ref().is_some_and(|(what, _)| *what == now) {
            let bad = (self.options.regex && !self.find.is_empty())
                .then(|| mg_module::text::Finder::new(&self.find, self.options).err())
                .flatten();
            self.checked = Some((now, bad));
        }
        self.checked.as_ref().and_then(|(_, bad)| bad.clone())
    }

    /// A hit ticked or unticked; the counts follow.
    pub(crate) fn tick(&mut self, at: usize, on: bool) {
        if let Some(h) = self.hits.get_mut(at) {
            h.1 = on;
        }
        self.count_ticked();
    }

    fn count_ticked(&mut self) {
        let on = self.hits.iter().filter(|h| h.1);
        self.ticked = on.fold((0, 0), |(n, times), h| (n + 1, times + h.0.count));
    }

    /// The ticked hits, and how many times the text is in them.
    pub(crate) fn ticked(&self) -> (usize, usize) {
        self.ticked
    }

    /// "6 strings, 5 times" ("1 string, 1 time"): the strings found and the
    /// times the text is in those ticked.
    pub(crate) fn count_words(&self) -> String {
        let strings = self.hits.len();
        let times = self.ticked.1;
        format!(
            "{strings} {}, {times} {}",
            if strings == 1 { "string" } else { "strings" },
            if times == 1 { "time" } else { "times" }
        )
    }

    /// The strings' rows laid out last frame (for the tests: those in
    /// sight alone).
    #[doc(hidden)]
    pub fn strings_seen(&self) -> (usize, usize) {
        self.strings_seen
    }

    /// The hits let go of (the module closed): the form keeps what was
    /// typed.
    pub(crate) fn forget_hits(&mut self) {
        self.hits.clear();
        self.searched = None;
        self.found_at = Default::default();
        self.stale_seen.set(None);
        self.current = None;
        self.ticked = (0, 0);
    }

    /// Hits found, in place of the last search's, all ticked.
    fn set_hits(
        &mut self,
        hits: Vec<mg_module::text::Hit>,
        at: (u64, Vec<ResKey>),
        searched: (String, mg_module::text::Options),
    ) {
        self.found_at = at;
        self.stale_seen.set(None);
        self.hits = hits.into_iter().map(|h| (h, true)).collect();
        self.searched = Some(searched);
        self.current = None;
        self.count_ticked();
    }
}

impl Default for TextReplace {
    fn default() -> TextReplace {
        TextReplace {
            find: String::new(),
            with: String::new(),
            options: Default::default(),
            kinds: mg_module::text::TextKind::PLAYERS.to_vec(),
            hits: Vec::new(),
            searched: None,
            found_at: Default::default(),
            stale_seen: Default::default(),
            checked: None,
            current: None,
            strings_seen: (0, 0),
            ticked: (0, 0),
        }
    }
}

/// What a find looks for: the module's keys of its hits, its revision as it
/// began.
fn found_keys(hits: &[mg_module::text::Hit]) -> Vec<ResKey> {
    let mut keys: Vec<ResKey> = hits.iter().map(|h| h.key).collect();
    keys.sort();
    keys.dedup();
    keys
}

impl Moonglow {
    /// Finds the text of `draft` in the module (its unsaved edits
    /// included), on this thread: the tests'. The window's Find runs it as
    /// a job ([`start_find_text`]).
    pub fn find_text(&mut self, draft: &mut TextReplace) {
        let Some(ws) = self.ws.as_mut() else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        // (Read as the game reads it: by the module's own table, if any.)
        let game = self.game.as_deref().map(|g| g.codepage());
        let options = mg_module::text::Options { game, ..draft.options };
        let hits = mg_module::text::find(&ws.module, &draft.find, options, &draft.kinds);
        let at = (ws.revision(), found_keys(&hits));
        draft.set_hits(hits, at, (draft.find.clone(), draft.options));
    }

    /// Replaces the found text in the ticked strings, as one command; how
    /// many times.
    pub fn replace_text(&mut self, draft: &TextReplace) -> usize {
        let Some((find, options)) = draft.searched.clone() else { return 0 };
        let game = self.game.as_deref().map(|g| g.codepage());
        let options = mg_module::text::Options { game, ..options };
        let Some(ws) = self.ws.as_mut() else { return 0 };
        // (A hit is a path in its resource as it was.)
        if draft.stale(ws) {
            self.log.warn("Replace: the module changed since the text was found: nothing replaced");
            return 0;
        }
        let mut edits = Vec::new();
        let mut total = 0;
        let mut errors = Vec::new();
        for (hit, _) in draft.hits.iter().filter(|(_, on)| *on) {
            let path = hit.path.clone();
            let Ok(doc) = ws.doc(&hit.key) else { continue };
            let Some(v) = path.get(&doc.root).and_then(|s| s.get(&hit.label)).cloned() else {
                continue;
            };
            match mg_module::text::replaced(&v, hit, &find, &draft.with, options) {
                Ok(Some(new)) if new != v => {
                    total += hit.count;
                    edits.push(Edit::SetField {
                        key: hit.key,
                        path,
                        label: hit.label.clone(),
                        value: Some(new),
                    });
                }
                Ok(_) => {}
                Err(e) => errors.push(format!("{}: {e}", hit.place)),
            }
        }
        for e in errors {
            self.log.error(format!("Replace: {e}"));
        }
        if !edits.is_empty() {
            let what = format!("Replace {find:?} with {:?}", draft.with);
            self.actions.push(Action::Apply(Command::new(what, edits)));
            self.log.info(format!(
                "Replaced {find:?} {total} {}",
                if total == 1 { "time" } else { "times" },
            ));
        }
        total
    }
}

/// Find and Replace Text's Find, as a job (Search Results' Find, Enter in
/// Find what): the module as it is, a resource at a time, its progress in
/// the job's window and Cancel there; what it finds into Search Results in
/// place of the last search's.
pub(crate) fn start_find_text(app: &mut Moonglow) {
    let Some(d) = app.text_replace.as_mut() else { return };
    if d.find.is_empty() || d.regex_problem().is_some() {
        return;
    }
    let Some(ws) = app.ws.as_ref() else { return };
    let (find, kinds) = (d.find.clone(), d.kinds.clone());
    let searched = (d.find.clone(), d.options);
    let game = app.game.as_deref().map(|g| g.codepage());
    let options = mg_module::text::Options { game, ..d.options };
    // (The revision as it begins: the snapshot is of it.)
    let revision = ws.revision();
    app.start_job(
        "Find and Replace Text",
        move |job| {
            mg_module::text::find_with(&job.module, &find, options, &kinds, &mut |done, total| {
                job.progress.step(done, total);
                !job.progress.cancelled()
            })
        },
        move |app, hits| {
            let Some(hits) = hits else { return };
            let Some(d) = app.text_replace.as_mut() else { return };
            let at = (revision, found_keys(&hits));
            d.set_hits(hits, at, searched);
            app.show_search(crate::search_results::Showing::Text);
        },
    );
}

/// Replace in N: the found text replaced in the ticked strings, as one
/// step of Undo, then found again (what is left).
pub(crate) fn replace(app: &mut Moonglow) {
    let Some(d) = app.text_replace.clone() else { return };
    if app.replace_text(&d) > 0 {
        // (Found again once the replace is applied: what's left. The
        // replace is an action of this frame; the find, a job, comes after
        // it.)
        app.text_refind = true;
    }
}

/// Opens hit `at` (its link, Enter): its resource at the field.
pub(crate) fn go_to_hit(app: &mut Moonglow, at: usize) {
    let Some(hit) = app.text_replace.as_ref().and_then(|d| d.hits.get(at)).map(|h| h.0.clone())
    else {
        return;
    };
    let usage = mg_module::rename::Usage {
        from: hit.key,
        path: mg_module::refs::Location::Gff(hit.field()),
        place: hit.place,
    };
    app.go_to_usage(&usage);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The review's A19: the ticked objects are asked for in sets made
    /// once, and answer as the list itself, scanned, did: by area, list
    /// and place for the placed, by area and where for the items held,
    /// the unticked left out.
    #[test]
    fn the_ticked_objects_are_found_as_a_scan_of_the_list_finds_them() {
        let areas = ["a", "b"].map(|a| ResRef::from_str(a).unwrap());
        let lists = ["Placeable List", "Creature List"];
        let mut objects = Vec::new();
        for (n, area) in areas.into_iter().enumerate() {
            for list in lists {
                for index in 0..6 {
                    let on = (index + n) % 3 != 0;
                    let held = Vec::new();
                    objects.push(Affected { area, list, index, held, label: String::new(), on });
                    let held: Held = vec![(list, index), ("ItemList", index % 2)];
                    let on = (index + n) % 2 == 0;
                    objects.push(Affected { area, list, index, held, label: String::new(), on });
                }
            }
        }
        let ticked = Ticked::of(&objects);
        for area in areas {
            for list in lists.into_iter().chain(["SoundList"]) {
                for index in 0..8 {
                    let scan = objects.iter().any(|o| {
                        o.area == area
                            && o.held.is_empty()
                            && o.list == list
                            && o.index == index
                            && o.on
                    });
                    assert_eq!(ticked.placed(area, list, index), scan, "{area} {list} {index}");
                    for slot in 0..2 {
                        let at: Held = vec![(list, index), ("ItemList", slot)];
                        let scan = objects.iter().any(|o| o.area == area && o.held == at && o.on);
                        assert_eq!(ticked.held(area, &at), scan, "{area} {at:?}");
                    }
                }
            }
        }
    }
}
