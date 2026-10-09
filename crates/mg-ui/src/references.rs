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
use crate::text::decode;
use crate::widgets::{autofocus, enter};
use crate::{Action, Moonglow, Tab};

/// What the References tab looks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    Resource(ResKey),
    Tag(String),
}

/// The References tab's state.
#[derive(Debug, Clone, Default)]
pub struct References {
    pub query: Option<Query>,
    pub usages: Vec<Usage>,
    pub mentions: Vec<Mention>,
    /// The workspace revision the results are for.
    revision: Option<u64>,
    /// The Find field.
    pub input: String,
}

/// The Rename window, while open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDraft {
    pub from: ResKey,
    pub to: String,
    /// Also rename script strings that spell the old name.
    pub literals: bool,
    pub usages: usize,
    pub mentions: usize,
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
        };
        self.references.query = Some(query);
        self.references.revision = None;
        self.actions.push(Action::OpenTab(Tab::References));
    }

    /// Opens the Rename window for a resource.
    pub fn rename_dialog(&mut self, key: ResKey) {
        let key = main_key(key);
        let Some(ws) = &mut self.ws else { return };
        if !rename::renamable(key.restype) {
            self.log.error(format!("{key} can't be renamed"));
            return;
        }
        if ws.flush().is_err() {
            return;
        }
        let usages = rename::usages(&ws.module, key).len();
        let mentions = rename::mentions(&ws.module, &key.resref.to_string(), true).len();
        self.rename = Some(RenameDraft {
            from: key,
            to: key.resref.to_string(),
            literals: false,
            usages,
            mentions,
        });
    }

    /// Renames a resource everywhere as one undoable command: its family,
    /// what names it, and (with `literals`) script strings; scripts whose
    /// source changed are compiled again.
    pub fn rename_resource(&mut self, from: ResKey, to: ResRef, literals: bool) {
        self.store_script_text();
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
                    let results = mg_module::build::compile_scripts(
                        &mut staged,
                        &game.resman,
                        mg_module::build::ScriptSelection::Uncompiled,
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
        self.retarget(from.resref, to);
        self.log.info(format!(
            "Renamed {from} to {new}: {} reference(s), {} in scripts",
            report.references, report.in_scripts
        ));
        if let Some(ws) = &self.ws {
            let left = rename::mentions(&ws.module, &from.resref.to_string(), true);
            if !left.is_empty() {
                self.log.warn(format!(
                    "{} still spelled “{}” in scripts ({}); the game won't find it by that name. \
                     Find References lists them",
                    if left.len() == 1 {
                        "A string".to_string()
                    } else {
                        format!("{} strings", left.len())
                    },
                    from.resref,
                    left.iter()
                        .map(|m| format!("{} line {}", m.script, m.line))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
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

    /// Points open editors at a renamed resource's new name.
    fn retarget(&mut self, from: ResRef, to: ResRef) {
        let moved = |k: &mut ResKey| {
            if k.resref == from {
                k.resref = to;
            }
        };
        let moved_ref = |r: &mut ResRef| {
            if *r == from {
                *r = to;
            }
        };
        for (_, tab) in self.dock.iter_all_tabs_mut() {
            match tab {
                Tab::Script(k) | Tab::Gff(k) | Tab::Dialog(k) | Tab::Blueprint(k) => moved(k),
                Tab::Area(r) | Tab::AreaProperties(r) => moved_ref(r),
                Tab::Instance { area, .. }
                | Tab::Instances { area, .. }
                | Tab::InstanceModel { area, .. } => moved_ref(area),
                _ => {}
            }
        }
        let old_keys: Vec<ResKey> =
            self.scripts.keys().filter(|k| k.resref == from).copied().collect();
        for k in old_keys {
            if let Some(b) = self.scripts.remove(&k) {
                self.scripts.insert(ResKey::new(to, k.restype), b);
            }
        }
        let old_keys: Vec<ResKey> =
            self.dialog_views.keys().filter(|k| k.resref == from).copied().collect();
        for k in old_keys {
            if let Some(v) = self.dialog_views.remove(&k) {
                self.dialog_views.insert(ResKey::new(to, k.restype), v);
            }
        }
        // An area's view is rebuilt under its new name.
        self.area_views.remove(&from);
        let pages: Vec<_> =
            self.blueprint_pages.keys().filter(|(k, _)| k.resref == from).cloned().collect();
        for (k, p) in pages {
            if let Some(v) = self.blueprint_pages.remove(&(k, p.clone())) {
                self.blueprint_pages.insert((ResKey::new(to, k.restype), p), v);
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

    /// Deletes `key` from the module, as one command: an area with its
    /// objects (its GIT and GIC) and its entry in the module's area list,
    /// a script with its compiled one. Its tabs close.
    pub(crate) fn delete_resource(&mut self, key: ResKey) {
        if let Some(why) = self.undeletable(key) {
            self.log.warn(why);
            return;
        }
        let Some(ws) = self.ws.as_mut() else { return };
        let with: &[ResType] = match key.restype {
            ResType::ARE => &[ResType::GIT, ResType::GIC],
            ResType::NSS => &[ResType::NCS],
            _ => &[],
        };
        let mut edits = Vec::new();
        let ifo = crate::module_props::info_key();
        if key.restype == ResType::ARE
            && let Ok(info) = ws.doc(&ifo)
            && let Some(index) = info
                .root
                .list("Mod_Area_list")
                .and_then(|l| l.iter().position(|a| a.resref("Area_Name") == Some(key.resref)))
        {
            edits.push(Edit::RemoveItem {
                key: ifo,
                path: mg_edit::GffPath::root(),
                list: "Mod_Area_list".into(),
                index,
            });
        }
        edits.extend(
            std::iter::once(key)
                .chain(with.iter().map(|t| ResKey::new(key.resref, *t)))
                .filter(|k| ws.module.contains(k))
                .map(|k| Edit::SetResource { key: k, data: None }),
        );
        if let Err(e) = self.apply(Command::new(format!("Delete {key}"), edits)) {
            self.log.error(e.to_string());
            return;
        }
        let area = (key.restype == ResType::ARE).then_some(key.resref);
        self.dock.retain_tabs(|t| match t {
            Tab::Script(k) | Tab::Gff(k) | Tab::Dialog(k) | Tab::Blueprint(k) => {
                *k != key && !(area == Some(k.resref) && with.contains(&k.restype))
            }
            Tab::Area(r) | Tab::AreaProperties(r) => area != Some(*r),
            Tab::Instance { area: r, .. }
            | Tab::Instances { area: r, .. }
            | Tab::InstanceModel { area: r, .. } => area != Some(*r),
            _ => true,
        });
        if let Some(area) = area {
            self.area_views.remove(&area);
        }
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
        let text = self
            .scripts
            .get(&key)
            .map(|b| b.text.clone())
            .or_else(|| self.ws.as_ref()?.module.get(&key).map(decode))
            .unwrap_or_default();
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
                .hint_text("a script, area, conversation, blueprint or tag")
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
        app.references.query = Some(match resource {
            Some(k) => Query::Resource(k),
            None => Query::Tag(name),
        });
        app.references.revision = None;
    }
    let Some(query) = app.references.query.clone() else {
        ui.weak("Choose Find References on a script, area, conversation or blueprint, or type a name or tag above.");
        return;
    };
    if app.references.revision != Some(ws.revision()) {
        if ws.flush().is_err() {
            return;
        }
        let (usages, mentions) = match &query {
            Query::Resource(k) => (
                rename::usages(&ws.module, *k),
                rename::mentions(&ws.module, &k.resref.to_string(), true),
            ),
            Query::Tag(t) => {
                (rename::tag_usages(&ws.module, t), rename::mentions(&ws.module, t, false))
            }
        };
        app.references.usages = usages;
        app.references.mentions = mentions;
        app.references.revision = Some(ws.revision());
    }
    ui.separator();
    let mut go = None;
    let mut go_line = None;
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
    });
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        // An area's objects are said to be in the area as the rest of the
        // window names it (Options › General: Show areas by name).
        let shown: Vec<String> = app
            .references
            .usages
            .clone()
            .iter()
            .map(|u| {
                let of_area = matches!(u.from.restype, ResType::ARE | ResType::GIT | ResType::GIC);
                if !of_area || !app.settings.area_names {
                    return u.place.clone();
                }
                let name = crate::tabs::area_label(app, u.from.resref);
                u.place.replacen(&u.from.resref.to_string(), &name, 1)
            })
            .collect();
        let refs = &app.references;
        if refs.usages.is_empty() {
            ui.weak("Nothing in the module names it.");
        } else {
            ui.label(format!("Used in {}:", places(refs.usages.len())));
            for (i, u) in refs.usages.iter().enumerate() {
                if ui.link(&shown[i]).on_hover_text(format!("{} {}", u.from, u.path)).clicked() {
                    go = Some(i);
                }
            }
        }
        if !refs.mentions.is_empty() {
            ui.add_space(8.0);
            ui.label(format!("Spelled out in scripts ({}):", refs.mentions.len())).on_hover_text(
                "Strings in scripts with this name: the script may use it, or mean something else",
            );
            for m in &refs.mentions {
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
    if let Some(k) = rename_key {
        app.rename_dialog(k);
    }
}

/// "1 place", "3 places".
fn places(n: usize) -> String {
    if n == 1 { "1 place".into() } else { format!("{n} places") }
}

/// The Rename window.
/// The module tree's Delete…: asks first (the area's objects go with it).
pub(crate) fn delete_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(key) = app.confirm_delete else { return };
    let why_not = app.undeletable(key);
    let mut open = true;
    egui::Window::new("Delete from Module")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some(why) = &why_not {
                ui.label(why);
                if ui.button("OK").clicked() {
                    open = false;
                }
                return;
            }
            ui.label(match key.restype {
                ResType::ARE => {
                    format!("Delete the area {} and everything placed in it?", key.resref)
                }
                ResType::NSS => format!("Delete {key} and its compiled script?"),
                _ => format!("Delete {key}?"),
            });
            ui.weak("Edit › Undo brings it back.");
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    app.actions.push(Action::DeleteResource(key));
                    open = false;
                }
                if crate::widgets::cancel(ui) {
                    open = false;
                }
            });
        });
    if !open {
        app.confirm_delete = None;
    }
}

pub(crate) fn rename_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut draft) = app.rename.take() else { return };
    let mut open = true;
    let mut cancel = false;
    let mut done = false;
    let family = match draft.from.restype {
        ResType::NSS => vec![ResType::NSS, ResType::NCS],
        ResType::ARE => vec![ResType::ARE, ResType::GIT, ResType::GIC],
        t => vec![t],
    };
    let exists = |to: &ResRef| {
        app.ws
            .as_ref()
            .is_some_and(|ws| family.iter().any(|t| ws.module.contains(&ResKey::new(*to, *t))))
    };
    egui::Window::new(format!("Rename {}", draft.from))
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .open(crate::widgets::open_unless_escape(ctx, &format!("Rename {}", draft.from), &mut open))
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
                Ok(_) => None,
            };
            ui.label(match draft.usages {
                0 => "Nothing else in the module names it.".to_string(),
                n => format!("{} in the module {} it; they will name the new one.", places(n), if n == 1 { "names" } else { "name" }),
            });
            if draft.mentions > 0 {
                ui.checkbox(
                    &mut draft.literals,
                    format!(
                        "Also change the {} spelling “{}” in scripts",
                        if draft.mentions == 1 { "string".to_string() } else { format!("{} strings", draft.mentions) },
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
            ui.horizontal(|ui| {
                let ok = ui.add_enabled(problem.is_none(), egui::Button::new("Rename"));
                if ok.clicked() || (problem.is_none() && enter(ui)) {
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
