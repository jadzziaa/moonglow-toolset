//! The editors of files outside the module (a hak, a 2DA, a tileset), and
//! what they have in common: their unsaved work (counted in the module's
//! [`Moonglow::has_unsaved_work`], saved with its Save), the question
//! their tab asks when it is closed with unsaved changes (one window for
//! all three, which asks of each tab in turn: Close All with two changed
//! haks asks of both), and their own Undo and Redo
//! ([`Moonglow::local_undo`]: the editor under the pointer, or the 2DA
//! grid that has the keys, takes the step back, not the module).

use crate::{Moonglow, Tab, hak_view, table_view, tileset_view, widgets};

/// A document a side editor edits: a file of its own, not part of the
/// module.
pub(crate) trait SideDoc {
    /// Its id: the tab it is edited in.
    fn tab(&self) -> Tab;
    /// Its name, as it is asked about ("mine.2da", "mine.hak", "New hak").
    fn title(&self) -> String;
    /// Whether it has changes not saved where it lives.
    fn has_unsaved(&self) -> bool;
    /// Whether the module's Save saves it too: not a hak that would have to
    /// ask where (a new one, or the game's own, kept only as a copy).
    fn saved_with_module(&self, _app: &Moonglow) -> bool {
        true
    }
    /// Whether it has a step to undo (or, `redo`, to redo).
    fn can_undo(&self, redo: bool) -> bool;
    /// The name of the step Undo (or, `redo`, Redo) would take, where its
    /// editor names its steps: Edit › Undo says it ("Undo Add Files").
    fn step_name(&self, _redo: bool) -> Option<String> {
        None
    }
    /// Undoes (or redoes) its last step.
    fn undo(&mut self, redo: bool);
    /// Saves it where it lives (asking where, if it must): whether it was.
    fn save(app: &mut Moonglow, id: u32) -> bool
    where
        Self: Sized;
}

impl SideDoc for hak_view::HakDoc {
    fn tab(&self) -> Tab {
        Tab::Hak(self.id)
    }
    fn title(&self) -> String {
        self.title().trim_end_matches(" *").to_string()
    }
    fn has_unsaved(&self) -> bool {
        self.hak.is_dirty()
    }
    fn saved_with_module(&self, app: &Moonglow) -> bool {
        // (By whatever name the path reaches the game's folder: a hak
        // folder linked into it.)
        self.hak.path.as_ref().is_some_and(|p| !hak_view::in_game_folder(app, p))
    }
    fn can_undo(&self, redo: bool) -> bool {
        self.step_name(redo).is_some()
    }
    fn step_name(&self, redo: bool) -> Option<String> {
        (if redo { self.hak.redo_label() } else { self.hak.undo_label() }).map(str::to_string)
    }
    fn undo(&mut self, redo: bool) {
        if redo {
            self.hak.redo();
        } else {
            self.hak.undo();
        }
        self.changed();
    }
    fn save(app: &mut Moonglow, id: u32) -> bool {
        hak_view::save(app, id, false)
    }
}

impl SideDoc for table_view::TableDoc {
    fn tab(&self) -> Tab {
        Tab::Table(self.id)
    }
    fn title(&self) -> String {
        self.key.to_string()
    }
    /// (A cell being typed counts: [`table_view::TableDoc::has_unsaved`].)
    fn has_unsaved(&self) -> bool {
        self.has_unsaved()
    }
    fn can_undo(&self, redo: bool) -> bool {
        self.step_name(redo).is_some()
    }
    /// (None while a cell is being typed: the typing is the cell's own.)
    fn step_name(&self, redo: bool) -> Option<String> {
        if self.typing.is_some() {
            return None;
        }
        (if redo { self.edit.redo_label() } else { self.edit.undo_label() }).map(str::to_string)
    }
    /// (Not while a cell is being typed: the typing is the cell's own.)
    fn undo(&mut self, redo: bool) {
        if self.typing.is_some() {
            return;
        }
        let done = if redo { self.edit.redo() } else { self.edit.undo() };
        if done {
            self.changed();
        }
    }
    fn save(app: &mut Moonglow, id: u32) -> bool {
        table_view::save(app, id)
    }
}

impl SideDoc for tileset_view::TilesetDoc {
    fn tab(&self) -> Tab {
        Tab::Tileset(self.id)
    }
    fn title(&self) -> String {
        self.title().trim_end_matches(" *").to_string()
    }
    fn has_unsaved(&self) -> bool {
        self.is_dirty()
    }
    fn can_undo(&self, redo: bool) -> bool {
        if redo { self.has_redo() } else { self.has_undo() }
    }
    fn undo(&mut self, redo: bool) {
        if redo {
            self.redo();
        } else {
            self.undo();
        }
    }
    fn save(app: &mut Moonglow, id: u32) -> bool {
        let Some(d) = app.tilesets.iter_mut().find(|d| d.id == id) else { return false };
        match d.save() {
            Ok(palette) => {
                app.log.info(format!("Saved {}", d.path.display()));
                if let Some(note) = palette {
                    app.log.info(note);
                }
                true
            }
            Err(e) => {
                app.log.error(e);
                false
            }
        }
    }
}

/// OK and Cancel under the window of an editor of a file that is saved
/// when asked (a hak, a 2DA, a tileset, the talk table, a script), as the
/// other editors have them: OK saves it and closes the window; Cancel
/// closes it as last saved, asking first about unsaved changes
/// ([`Moonglow::may_cancel`]). One only looked at (`read_only`) has
/// Close. Drawn before the window's pages, of whose room it takes the
/// foot; `before` is run once a button is pressed (a cell being typed
/// set). Whether the window is closing: nothing more is drawn of it.
pub(crate) fn ok_cancel(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    tab: &Tab,
    read_only: bool,
    before: impl FnOnce(&mut Moonglow),
) -> bool {
    use crate::blueprint::Pressed;
    let tips = [
        "Save and close the window",
        "Close the window without saving, asking about unsaved changes first",
    ];
    let id = egui::Id::new(("side-ok-cancel", tab));
    let pressed = crate::blueprint::ok_cancel_saying(ui, id, read_only, tips);
    failed_foot(app, ui, tab);
    let Some(pressed) = pressed else { return false };
    before(app);
    // (A save that fails here asks the tab's question: why, and the ways
    // out, are on screen rather than nothing.)
    let close = match pressed {
        // (Close: as its ×.)
        Pressed::Ok if read_only => app.may_close(tab),
        Pressed::Ok => {
            let unsaved = app.asked_about(tab).is_some_and(|(_, unsaved)| unsaved);
            // (A hak is written as a job, which closes its tab once it is
            // saved: gigabytes were written inside this frame.)
            if let (true, Tab::Hak(id)) = (unsaved, tab) {
                hak_view::save_then(app, *id, false, hak_view::Then::Close);
                // (Saved and let go already, where jobs run at once:
                // nothing more is drawn of it.)
                return app.side_doc(tab).is_none();
            }
            let saved = !unsaved || app.save_side(tab);
            if saved {
                app.forget_side(tab);
            } else if app.failed_saving(tab).is_some() && !app.side_closing.contains(tab) {
                app.side_closing.push_back(tab.clone());
            }
            saved
        }
        Pressed::Cancel => app.may_cancel(tab),
    };
    if close {
        app.actions.push(crate::Action::CloseTab(tab.clone()));
    }
    close
}

/// Why `tab`'s document could not be saved the last time, under its
/// editor (above its OK and Cancel): the editor's own Save, its OK and
/// the module's Save say it there, not only in the log.
fn failed_foot(app: &Moonglow, ui: &mut egui::Ui, tab: &Tab) {
    let Some(why) = app.failed_saving(tab) else { return };
    // (A 2DA's editor says it already, above its grid: "Not saved: …".)
    if let Tab::Table(id) = *tab
        && app.tables.iter().any(|d| d.id == id && d.problem.is_some())
    {
        return;
    }
    let text = failed_text(why);
    egui::Panel::bottom(egui::Id::new(("side-failed", tab)))
        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 4)))
        .show_separator_line(false)
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(text).color(ui.visuals().error_fg_color))
                    .wrap(),
            );
        });
}

/// The sentence that says why a side editor's document was not saved.
fn failed_text(why: &str) -> String {
    format!("It could not be saved: {why}")
}

/// Whether a tab is a side editor's (whether or not its document is still
/// open).
pub(crate) fn is_side(tab: &Tab) -> bool {
    matches!(tab, Tab::Hak(_) | Tab::Table(_) | Tab::Tileset(_))
}

impl Moonglow {
    /// Every side editor's document, in the order the module's Save saves
    /// them: the tables first (one of a hak's is saved into the hak, which
    /// is then written), then the haks and the tilesets.
    pub(crate) fn side_docs(&self) -> impl Iterator<Item = &dyn SideDoc> {
        (self.tables.iter().map(|d| d as &dyn SideDoc))
            .chain(self.haks.iter().map(|d| d as &dyn SideDoc))
            .chain(self.tilesets.iter().map(|d| d as &dyn SideDoc))
    }

    /// The document a side editor's tab edits, while it is open.
    pub(crate) fn side_doc(&self, tab: &Tab) -> Option<&dyn SideDoc> {
        match *tab {
            Tab::Hak(id) => self.haks.iter().find(|d| d.id == id).map(|d| d as &dyn SideDoc),
            Tab::Table(id) => self.tables.iter().find(|d| d.id == id).map(|d| d as &dyn SideDoc),
            Tab::Tileset(id) => {
                self.tilesets.iter().find(|d| d.id == id).map(|d| d as &dyn SideDoc)
            }
            _ => None,
        }
    }

    fn side_doc_mut(&mut self, tab: &Tab) -> Option<&mut dyn SideDoc> {
        match *tab {
            Tab::Hak(id) => {
                self.haks.iter_mut().find(|d| d.id == id).map(|d| d as &mut dyn SideDoc)
            }
            Tab::Table(id) => {
                self.tables.iter_mut().find(|d| d.id == id).map(|d| d as &mut dyn SideDoc)
            }
            Tab::Tileset(id) => {
                self.tilesets.iter_mut().find(|d| d.id == id).map(|d| d as &mut dyn SideDoc)
            }
            _ => None,
        }
    }

    /// Saves a side editor's document (or the talk table, a script):
    /// whether it was. Why not, where it failed (not where the user called
    /// it off, the Save dialog canceled), is kept for its question and its
    /// editor ([`Moonglow::failed_saving`]): the reason its own save gave,
    /// else a 2DA's problem, else the newest error logged meanwhile.
    fn save_side(&mut self, tab: &Tab) -> bool {
        // (The talk table's too: its question and its foot are these.)
        let side = is_side(tab) || *tab == Tab::TalkTable;
        if side {
            self.clear_failed(tab);
        }
        let logged = self.log.entries.len();
        let saved = self.save_side_now(tab);
        if saved || !side || self.failed_saving(tab).is_some() {
            return saved;
        }
        let problem = match *tab {
            Tab::Table(id) => {
                self.tables.iter().find(|d| d.id == id).and_then(|d| d.problem.clone())
            }
            _ => None,
        };
        let why = problem.map(|p| p.trim_start_matches("Not saved: ").to_string()).or_else(|| {
            (self.log.entries[logged.min(self.log.entries.len())..].iter())
                .rev()
                .find(|(level, _)| *level == crate::Level::Error)
                .map(|(_, m)| m.clone())
        });
        if let Some(why) = why {
            self.side_failed = Some((tab.clone(), why));
        }
        saved
    }

    /// Why `tab`'s document could not be saved the last time it was
    /// tried, if it could not and it still has unsaved changes: one made
    /// clean another way (Undo, Remove) has nothing left to save
    /// ([`Moonglow::let_go_of_failed`] forgets the reason then).
    pub(crate) fn failed_saving(&self, tab: &Tab) -> Option<&str> {
        let (t, why) = self.side_failed.as_ref().filter(|(t, _)| t == tab)?;
        self.asked_about(t).is_some_and(|(_, unsaved)| unsaved).then_some(why.as_str())
    }

    /// Forgets why a save failed once its document is seen with nothing
    /// unsaved (or gone), so that it does not come back with the next,
    /// unrelated change. Each frame, before the question is drawn.
    pub(crate) fn let_go_of_failed(&mut self) {
        if let Some((tab, _)) = &self.side_failed
            && self.failed_saving(&tab.clone()).is_none()
        {
            self.side_failed = None;
        }
    }

    /// Notes why `tab`'s document could not be saved.
    pub(crate) fn note_failed(&mut self, tab: &Tab, why: impl Into<String>) {
        self.side_failed = Some((tab.clone(), why.into()));
    }

    /// Forgets why `tab`'s document could not be saved: tried again,
    /// saved, or let go.
    pub(crate) fn clear_failed(&mut self, tab: &Tab) {
        if self.side_failed.as_ref().is_some_and(|(t, _)| t == tab) {
            self.side_failed = None;
        }
    }

    fn save_side_now(&mut self, tab: &Tab) -> bool {
        match *tab {
            Tab::Hak(id) => <hak_view::HakDoc as SideDoc>::save(self, id),
            Tab::Table(id) => <table_view::TableDoc as SideDoc>::save(self, id),
            Tab::Tileset(id) => <tileset_view::TilesetDoc as SideDoc>::save(self, id),
            Tab::TalkTable => crate::talk_view::save(self),
            Tab::Script(key) => {
                crate::script_view::save_script(self, key, crate::script_view::ScriptSave::Save);
                true
            }
            _ => false,
        }
    }

    /// Lets a side editor's document go (its changes with it), or the
    /// talk table's and a script's unsaved changes (a Cancel's Don't
    /// Save: the table is read again from its file, the script from the
    /// module, when next shown).
    pub(crate) fn forget_side(&mut self, tab: &Tab) {
        self.clear_failed(tab);
        match *tab {
            Tab::Hak(id) => self.haks.retain(|d| d.id != id),
            Tab::Table(id) => self.tables.retain(|d| d.id != id),
            Tab::Tileset(id) => self.tilesets.retain(|d| d.id != id),
            // (Lines typed for the module's fields are the module's: its
            // own changes are dropped, those kept.)
            Tab::TalkTable if self.talk.as_ref().is_some_and(|t| t.has_typed_unsaved()) => {
                if let Some(t) = self.talk.as_mut()
                    && !t.drop_own_changes()
                {
                    self.log.warn(
                        "The talk table keeps its changes: it has lines typed for the module's \
                         fields, which the module's Undo takes back",
                    );
                }
                self.talk_view.changed_outside();
            }
            Tab::TalkTable if self.talk.as_ref().is_some_and(|t| t.is_dirty()) => {
                self.talk = None;
                self.talk_view = Default::default();
            }
            Tab::Script(key) => {
                self.scripts.remove(&key);
            }
            _ => {}
        }
    }

    /// What the closing question asks about a tab: its document's name
    /// and whether it has unsaved changes. A side editor's, or (for its
    /// Cancel: their tabs' close keeps what is unsaved, for the module's
    /// Save) the talk table's and a script's.
    fn asked_about(&self, tab: &Tab) -> Option<(String, bool)> {
        match tab {
            Tab::TalkTable => {
                let t = self.talk.as_ref()?;
                Some((format!("the talk table {}", t.name), t.is_dirty() && t.editable()))
            }
            Tab::Script(key) => {
                let dirty = self.scripts.get(key).is_some_and(|b| b.is_dirty());
                Some((key.to_string(), dirty))
            }
            _ => self.side_doc(tab).map(|d| (d.title(), d.has_unsaved())),
        }
    }

    /// A window's Cancel, for an editor of a file that is saved when asked
    /// (a hak, a 2DA, a tileset, the talk table, a script): with unsaved
    /// changes, Save, Don't Save or Cancel is asked first ([`closing_window`],
    /// which closes it), and it stays; without, it may close now.
    pub(crate) fn may_cancel(&mut self, tab: &Tab) -> bool {
        match self.asked_about(tab) {
            Some((_, true)) => {
                if !self.side_closing.contains(tab) {
                    self.side_closing.push_back(tab.clone());
                }
                false
            }
            Some(_) => {
                self.forget_side(tab);
                true
            }
            None => true,
        }
    }

    /// The side editors' documents follow their tabs where the dock was
    /// made anew (the module closed, another opened, its kept tabs put
    /// back; those open when another module is opened keep theirs,
    /// `Moonglow::close_keeping`): one left without a tab, and none on
    /// its way (in the actions still to run, this batch's too), is let go
    /// if nothing of it is unsaved, else its tab is opened again. A changed
    /// hak that outlived its tab counted as unsaved work for ever, with no
    /// editor to save it from, and the window would not quit. (Closing,
    /// Exit and opening another module ask of each changed one first,
    /// through [`Moonglow::run`]; the second step of New Module and of a
    /// `.mod` with a folder beside it is not asked again for a side
    /// editor changed between the steps, and keeps it so.) A hak is kept
    /// with a changed 2DA of its own that is kept.
    pub(crate) fn side_docs_follow_tabs(&mut self) {
        let opening = |app: &Moonglow, tab: &Tab| {
            (app.actions.iter().chain(&app.batch_rest))
                .any(|a| matches!(a, crate::Action::OpenTab(t) if t == tab))
        };
        let tabless =
            |app: &Moonglow, tab: &Tab| app.dock.find_tab(tab).is_none() && !opening(app, tab);
        let holding: Vec<u32> = (self.tables.iter())
            .filter(|d| d.has_unsaved())
            .filter_map(|d| match d.home {
                table_view::Home::Hak(hak) => Some(hak),
                _ => None,
            })
            .collect();
        let lost: Vec<(Tab, bool)> = (self.side_docs())
            .map(|d| {
                let tab = d.tab();
                let keep = d.has_unsaved() || matches!(tab, Tab::Hak(id) if holding.contains(&id));
                (tab, keep)
            })
            .filter(|(tab, _)| tabless(self, tab))
            .collect();
        for (tab, keep) in lost {
            if keep {
                self.dock.main_surface_mut().push_to_first_leaf(tab);
            } else {
                self.forget_side(&tab);
            }
        }
    }

    /// Whether a side editor has unsaved changes.
    pub(crate) fn side_unsaved(&self) -> bool {
        self.side_docs().any(|d| d.has_unsaved())
    }

    /// The module's Save: every side editor's document with unsaved changes
    /// that is saved with it.
    pub(crate) fn save_side_docs(&mut self) {
        let tabs: Vec<Tab> = (self.side_docs())
            .filter(|d| d.has_unsaved() && d.saved_with_module(self))
            .map(|d| d.tab())
            .collect();
        for tab in tabs {
            self.save_side(&tab);
        }
    }

    /// A side editor's tab asked to close: with unsaved changes, it waits
    /// for the question to be answered (after those asked before it), and
    /// stays; without, its document is let go. Whether it may close now.
    pub(crate) fn may_close_side(&mut self, tab: &Tab) -> bool {
        match self.side_doc(tab) {
            Some(d) if d.has_unsaved() => {
                if !self.side_closing.contains(tab) {
                    self.side_closing.push_back(tab.clone());
                }
                false
            }
            Some(_) => {
                self.forget_side(tab);
                true
            }
            None => true,
        }
    }

    /// The side editor Undo and Redo are for, if one: the one under the
    /// pointer, else the 2DA grid that has the keys, else, with the
    /// pointer on the menu bar or the toolbar (or a menu open), the one it
    /// was over before (Edit › Undo and the toolbar's Undo are its).
    fn local_undo_target(&self) -> Option<Tab> {
        self.undo_under_pointer
            .clone()
            .or_else(|| self.undo_keys.clone())
            .or_else(|| self.undo_before_bars.clone().filter(|_| self.pointer_on_bars))
    }

    /// Whether Undo (or, `redo`, Redo) is a side editor's or the talk
    /// table's, and then whether it has a step to take: None, the
    /// module's.
    pub(crate) fn local_can_undo(&self, redo: bool) -> Option<bool> {
        let tab = self.local_undo_target()?;
        if tab == Tab::TalkTable {
            let t = self.talk.as_ref()?;
            return Some(if redo { t.can_redo() } else { t.can_undo() });
        }
        self.side_doc(&tab).map(|d| d.can_undo(redo))
    }

    /// The name of the step a side editor's Undo (or, `redo`, Redo) would
    /// take, when Undo is that editor's and it names its steps (a hak's,
    /// a 2DA's; a tileset's and the talk table's name none).
    pub(crate) fn local_step_name(&self, redo: bool) -> Option<String> {
        let tab = self.local_undo_target()?;
        self.side_doc(&tab)?.step_name(redo)
    }

    /// Undo (or, `redo`, Redo) in the editor of a file outside the module
    /// (a hak, a 2DA, a tileset, the talk table) under the pointer, or in
    /// the 2DA grid that has the keys: whether the step was one of those
    /// editors' (with nothing to undo there, nothing is: not the module's
    /// last change, which is not what is in front).
    pub(crate) fn local_undo(&mut self, redo: bool) -> bool {
        let Some(tab) = self.local_undo_target() else { return false };
        if tab == Tab::TalkTable {
            return crate::talk_view::undo(self, redo);
        }
        let Some(d) = self.side_doc_mut(&tab) else { return false };
        if d.can_undo(redo) {
            d.undo(redo);
        }
        true
    }
}

/// Notes the side editor (or the talk table's) drawn in `ui` as the one
/// Undo is for while the pointer is over it.
pub(crate) fn note_pointer(app: &mut Moonglow, ui: &egui::Ui, tab: &Tab) {
    if ui.rect_contains_pointer(ui.max_rect()) {
        app.undo_under_pointer = Some(tab.clone());
    }
}

/// The question a side editor's tab asks when it is closed with unsaved
/// changes, as the module's own asks it: Save, Don't Save or Cancel. The
/// tabs waiting are asked of in turn; Cancel keeps them all open and asks
/// no more.
pub(crate) fn closing_window(app: &mut Moonglow, ctx: &egui::Context) {
    app.let_go_of_failed();
    // (A document let go meanwhile is not asked about; one saved meanwhile
    // closes, as was asked.)
    while let Some(tab) = app.side_closing.front().cloned() {
        match app.asked_about(&tab) {
            None => {}
            Some((_, false)) => close(app, &tab),
            Some(_) => break,
        }
        app.side_closing.pop_front();
    }
    let Some(tab) = app.side_closing.front().cloned() else {
        carry_on(app, ctx);
        return;
    };
    // (A hak being saved for this question's Save: it closes once it is
    // written, and is asked about again if it could not be.)
    let saving = app.job.as_ref().is_some_and(|j| j.title == hak_view::SAVE_JOB);
    if saving && matches!(tab, Tab::Hak(_)) {
        return;
    }
    let Some((title, _)) = app.asked_about(&tab) else { return };
    let failed = app.failed_saving(&tab).map(failed_text);
    let more = app.side_closing.len() - 1;
    enum Answer {
        Save,
        DontSave,
        Cancel,
    }
    let mut answer = None;
    widgets::modal(ctx, "Unsaved Changes").id(egui::Id::new("side-doc-closing")).show(ctx, |ui| {
        ui.label(format!("Save changes to {title}?"));
        if let Some(failed) = &failed {
            // (A path is long: wide enough to read in a few lines.)
            ui.set_min_width(440.0);
            ui.add(
                egui::Label::new(egui::RichText::new(failed).color(ui.visuals().error_fg_color))
                    .wrap(),
            );
        }
        if more > 0 {
            ui.weak(format!(
                "{more} more {} unsaved changes after this.",
                if more == 1 { "tab has" } else { "tabs have" }
            ));
        }
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                answer = Some(Answer::Save);
            }
            if ui.button("Don't Save").clicked() {
                answer = Some(Answer::DontSave);
            }
            if widgets::cancel(ui) {
                answer = Some(Answer::Cancel);
            }
        });
    });
    match answer {
        None => {}
        // (Nothing more is asked, and what waited for the answers is not
        // done. Why a save failed stays under its editor, which stays
        // open: until it is saved, tried again or let go.)
        Some(Answer::Cancel) => {
            app.side_closing.clear();
            app.side_then = None;
        }
        // (A hak is written as a job, which closes its tab once it is
        // saved; not saved, it stays, and is asked about again, saying
        // why.)
        Some(Answer::Save) if matches!(tab, Tab::Hak(_)) => {
            if let Tab::Hak(id) = tab {
                hak_view::save_then(app, id, false, hak_view::Then::Close);
            }
        }
        // (Not saved: it stays, and is asked about again, saying why.)
        Some(Answer::Save) if !app.save_side(&tab) => {}
        Some(Answer::Save | Answer::DontSave) => {
            close(app, &tab);
            app.side_closing.pop_front();
        }
    }
    if answer.is_some() {
        // (The next question, or what waited for the answers, at once:
        // not at the next input.)
        ctx.request_repaint();
        if app.side_closing.is_empty() {
            carry_on(app, ctx);
        }
    }
}

/// Every question answered: what waited for the answers (closing the
/// window, Exit, File › Close, another module) goes on, through
/// [`Moonglow::run`], which looks again at what is unsaved: a document
/// changed meanwhile is asked of again. Not under a job (a hak being
/// written for its question's Save), which `run` would refuse.
fn carry_on(app: &mut Moonglow, ctx: &egui::Context) {
    if app.job.is_some() {
        return;
    }
    if let Some(then) = app.side_then.take() {
        app.actions.push(then);
        ctx.request_repaint();
    }
}

/// A side editor's tab closed, its document let go.
fn close(app: &mut Moonglow, tab: &Tab) {
    app.forget_side(tab);
    if let Some(path) = app.dock.find_tab(tab) {
        app.dock.remove_tab(path);
    }
}
