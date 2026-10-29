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
        let root = app.install.as_ref().map(|gi| &gi.root);
        self.hak.path.as_ref().is_some_and(|p| root.is_none_or(|r| !p.starts_with(r)))
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

    /// Saves a side editor's document: whether it was.
    fn save_side(&mut self, tab: &Tab) -> bool {
        match *tab {
            Tab::Hak(id) => <hak_view::HakDoc as SideDoc>::save(self, id),
            Tab::Table(id) => <table_view::TableDoc as SideDoc>::save(self, id),
            Tab::Tileset(id) => <tileset_view::TilesetDoc as SideDoc>::save(self, id),
            _ => false,
        }
    }

    /// Lets a side editor's document go (its changes with it).
    pub(crate) fn forget_side(&mut self, tab: &Tab) {
        match *tab {
            Tab::Hak(id) => self.haks.retain(|d| d.id != id),
            Tab::Table(id) => self.tables.retain(|d| d.id != id),
            Tab::Tileset(id) => self.tilesets.retain(|d| d.id != id),
            _ => {}
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
    // (A document let go meanwhile is not asked about; one saved meanwhile
    // closes, as was asked.)
    while let Some(tab) = app.side_closing.front().cloned() {
        match app.side_doc(&tab) {
            None => {}
            Some(d) if !d.has_unsaved() => close(app, &tab),
            Some(_) => break,
        }
        app.side_closing.pop_front();
    }
    let Some(tab) = app.side_closing.front().cloned() else { return };
    let Some(title) = app.side_doc(&tab).map(|d| d.title()) else { return };
    let more = app.side_closing.len() - 1;
    enum Answer {
        Save,
        DontSave,
        Cancel,
    }
    let mut answer = None;
    widgets::modal(ctx, "Unsaved Changes").id(egui::Id::new("side-doc-closing")).show(ctx, |ui| {
        ui.label(format!("Save changes to {title}?"));
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
        Some(Answer::Cancel) => app.side_closing.clear(),
        // (Not saved: it stays, and is asked about again.)
        Some(Answer::Save) if !app.save_side(&tab) => {}
        Some(Answer::Save | Answer::DontSave) => {
            close(app, &tab);
            app.side_closing.pop_front();
        }
    }
}

/// A side editor's tab closed, its document let go.
fn close(app: &mut Moonglow, tab: &Tab) {
    app.forget_side(tab);
    if let Some(path) = app.dock.find_tab(tab) {
        app.dock.remove_tab(path);
    }
}
