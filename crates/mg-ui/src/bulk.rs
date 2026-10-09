//! Bulk edits: Update Instances for any set of blueprints (a selection in
//! the custom palette, a whole category, or one blueprint's editor), as
//! Aurora's palette does it for one: every area or the one shown, with the
//! objects it would change listed to untick.

use std::collections::HashSet;

use egui::Context;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Struct;
use mg_module::instances::{Held, Placing, git_list, held_items, update, update_held};
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
}

/// The kind of object a GIT list holds, for people.
fn kind_name(list: &str) -> &'static str {
    match list {
        "Creature List" => "creature",
        "Door List" => "door",
        "Encounter List" => "encounter",
        "List" => "item",
        "Placeable List" => "placeable",
        "SoundList" => "sound",
        "StoreList" => "store",
        "TriggerList" => "trigger",
        _ => "waypoint",
    }
}

/// The blueprint field that names a placed object's blueprint.
fn template_field(restype: ResType) -> &'static str {
    if restype == ResType::UTM { "ResRef" } else { "TemplateResRef" }
}

impl Moonglow {
    /// Opens Update Instances for `blueprints` (the module's): what it
    /// would change, in every area. Logs it when nothing is placed from
    /// them.
    pub fn update_instances_of(&mut self, blueprints: Vec<ResKey>) {
        let Some(ws) = self.ws.as_mut() else { return };
        let wanted: HashSet<ResKey> = blueprints.iter().copied().collect();
        let mut objects = Vec::new();
        for area in ws.module.areas().unwrap_or_default() {
            let Ok(git) = ws.doc(&ResKey::new(area, ResType::GIT)) else { continue };
            for restype in wanted.iter().map(|k| k.restype).collect::<HashSet<_>>() {
                let Some((list, _)) = git_list(restype) else { continue };
                for (index, s) in git.root.list(list).unwrap_or(&[]).iter().enumerate() {
                    let Some(r) = s.resref(template_field(restype)) else { continue };
                    if !wanted.contains(&ResKey::new(r, restype)) {
                        continue;
                    }
                    let tag = String::from_utf8_lossy(s.string("Tag").unwrap_or_default());
                    let tag = if tag.is_empty() { format!("#{}", index + 1) } else { tag.into() };
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
                    .map(|s| String::from_utf8_lossy(s.string("Tag").unwrap_or_default()))
                    .filter(|t| !t.is_empty())
                    .map_or_else(|| format!("#{}", index + 1), |t| t.into_owned());
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
            self.log.info(format!("Update Instances: nothing is placed from {}", names.join(", ")));
            return;
        }
        let shown = self.palette.area;
        self.update_draft = Some(UpdateDraft { blueprints, only: None, shown, objects });
    }

    /// Makes the ticked objects again from their blueprints, as one
    /// command; how many.
    pub fn apply_update(&mut self, draft: &UpdateDraft) -> usize {
        let (Some(ws), Some(game)) = (self.ws.as_mut(), self.game.as_deref()) else { return 0 };
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
        let module = &ws.module;
        let items =
            |r: ResRef| mg_module::gff_root(Some(module), game, &ResKey::new(r, ResType::UTI));
        let placing = Placing { game, item: &items };
        let blueprint = |t: ResType, r: ResRef| blueprints.get(&ResKey::new(r, t)).cloned();
        let mut edits = Vec::new();
        let mut total = 0;
        for (key, git) in &gits {
            let (key, area) = (*key, key.resref);
            let which = |list: &str, index: usize| {
                draft.objects.iter().any(|o| {
                    o.area == area
                        && o.held.is_empty()
                        && o.list == list
                        && o.index == index
                        && o.on
                })
            };
            // The items held first (where they are is by their places
            // now), then the placed objects.
            let held = |at: &[(&'static str, usize)]| {
                draft.objects.iter().any(|o| o.area == area && o.held == at && o.on)
            };
            let item = |r: ResRef| blueprint(ResType::UTI, r);
            let (with_held, held_n) =
                update_held(&placing, git, &item, &held).unwrap_or_else(|| (git.clone(), 0));
            let (new, n) = update(&placing, &with_held, &blueprint, &which)
                .map_or((with_held, held_n), |(new, n)| (new, n + held_n));
            if n == 0 {
                continue;
            }
            total += n;
            for (list, _) in mg_module::instances::GIT_LISTS {
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
        if total > 0 {
            let what = match draft.blueprints.as_slice() {
                [one] => format!("Update instances of {}", one.resref),
                many => format!("Update instances of {} blueprints", many.len()),
            };
            self.actions.push(Action::Apply(Command::new(what, edits)));
            self.log.info(format!(
                "Updated {total} object(s) from {} blueprint(s):",
                draft.blueprints.len()
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
    let (mut ok, mut cancel) = (false, false);
    egui::Window::new("Update Instances")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .collapsible(false)
        .open(crate::widgets::open_unless_escape(ctx, "Update Instances", &mut open))
        .show(ctx, |ui| {
            let names: Vec<String> =
                draft.blueprints.iter().map(|k| k.resref.to_string()).collect();
            let shown_names = if names.len() > 6 {
                format!("{} and {} more", names[..6].join(", "), names.len() - 6)
            } else {
                names.join(", ")
            };
            ui.label(format!(
                "Makes each object placed from {shown_names} again from its blueprint, \
                 where it stands: its tag, name, scripts and variables become the blueprint's."
            ));
            ui.horizontal(|ui| {
                ui.radio_value(&mut draft.only, None, "Every area");
                if let Some(area) = draft.shown {
                    ui.radio_value(&mut draft.only, Some(area), format!("Only {area}"));
                }
            });
            let in_scope = |o: &Affected, only: Option<ResRef>| only.is_none_or(|a| a == o.area);
            ui.horizontal(|ui| {
                if ui.small_button("All").clicked() {
                    draft.objects.iter_mut().for_each(|o| o.on = true);
                }
                if ui.small_button("None").clicked() {
                    draft.objects.iter_mut().for_each(|o| o.on = false);
                }
            });
            let only = draft.only;
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for o in draft.objects.iter_mut().filter(|o| in_scope(o, only)) {
                    ui.checkbox(&mut o.on, &o.label);
                }
            });
            let n = draft.objects.iter().filter(|o| o.on && in_scope(o, only)).count();
            ui.horizontal(|ui| {
                ok = ui.add_enabled(n > 0, egui::Button::new(format!("Update {n}"))).clicked();
                cancel = crate::widgets::cancel(ui);
            });
        });
    if ok {
        app.apply_update(&draft);
    } else if open && !cancel {
        app.update_draft = Some(draft);
    }
}

/// The Find and Replace Text window (Edit › Find and Replace Text…): the
/// module's player-facing strings (`mg_module::text`).
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
    /// Find again next frame (after a replace is applied).
    refind: bool,
}

impl Default for TextReplace {
    fn default() -> TextReplace {
        TextReplace {
            find: String::new(),
            with: String::new(),
            options: Default::default(),
            kinds: mg_module::text::TextKind::ALL.to_vec(),
            hits: Vec::new(),
            searched: None,
            refind: false,
        }
    }
}

impl Moonglow {
    /// Finds the window's text in the module (its unsaved edits included).
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
        draft.hits = hits.into_iter().map(|h| (h, true)).collect();
        draft.searched = Some((draft.find.clone(), draft.options));
    }

    /// Replaces the found text in the ticked strings, as one command; how
    /// many times.
    pub fn replace_text(&mut self, draft: &TextReplace) -> usize {
        let Some((find, options)) = draft.searched.clone() else { return 0 };
        let game = self.game.as_deref().map(|g| g.codepage());
        let options = mg_module::text::Options { game, ..options };
        let Some(ws) = self.ws.as_mut() else { return 0 };
        let mut edits = Vec::new();
        let mut total = 0;
        let mut errors = Vec::new();
        for (hit, _) in draft.hits.iter().filter(|(_, on)| *on) {
            let path = GffPath(
                hit.path
                    .iter()
                    .map(|s| match s {
                        mg_module::text::Step::Field(l) => mg_edit::Step::Field(l.clone()),
                        mg_module::text::Step::Item(l, i) => mg_edit::Step::Item(l.clone(), *i),
                    })
                    .collect(),
            );
            let Ok(doc) = ws.doc(&hit.key) else { continue };
            let Some(ls) = path.get(&doc.root).and_then(|s| s.locstring(&hit.label)).cloned()
            else {
                continue;
            };
            match mg_module::text::replace_locstring(&ls, &find, &draft.with, options) {
                Ok(new) if new != ls => {
                    total += hit.count;
                    edits.push(Edit::SetField {
                        key: hit.key,
                        path,
                        label: hit.label.clone(),
                        value: Some(mg_gff::Value::LocString(new)),
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
            self.log.info(format!("Replaced {find:?} {total} time(s)"));
        }
        total
    }
}

/// The Find and Replace Text window.
pub(crate) fn text_window(app: &mut Moonglow, ctx: &Context) {
    use mg_module::text::TextKind;
    let Some(mut draft) = app.text_replace.take() else { return };
    if std::mem::take(&mut draft.refind) {
        app.find_text(&mut draft);
    }
    let mut open = true;
    let (mut find, mut replace, mut go) = (false, false, None);
    egui::Window::new("Find and Replace Text")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_width(560.0)
        .open(crate::widgets::open_unless_escape(ctx, "Find and Replace Text", &mut open))
        .show(ctx, |ui| {
            ui.weak(
                "The module's names, descriptions, conversation lines and journal, in every \
                 language they're written in (not the game's talk table).",
            );
            egui::Grid::new("text-replace").num_columns(2).show(ui, |ui| {
                crate::widgets::field_label(ui, "Find what");
                let r = ui.add(egui::TextEdit::singleline(&mut draft.find).desired_width(320.0));
                crate::widgets::autofocus(ui, &r);
                find = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.end_row();
                crate::widgets::field_label(ui, "Replace with");
                ui.add(egui::TextEdit::singleline(&mut draft.with).desired_width(320.0));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut draft.options.match_case, "Match case");
                ui.checkbox(&mut draft.options.whole_word, "Whole words");
            });
            ui.horizontal_wrapped(|ui| {
                for k in TextKind::ALL {
                    let mut on = draft.kinds.contains(&k);
                    if ui.checkbox(&mut on, k.label()).changed() {
                        draft.kinds.retain(|x| *x != k);
                        if on {
                            draft.kinds.push(k);
                        }
                    }
                }
            });
            find |= ui.add_enabled(!draft.find.is_empty(), egui::Button::new("Find")).clicked();
            // What was found, if it's still what's asked for.
            let current = draft.searched.as_ref() == Some(&(draft.find.clone(), draft.options));
            if current {
                let times: usize = draft.hits.iter().filter(|h| h.1).map(|h| h.0.count).sum();
                ui.label(format!("{} string(s), {times} time(s)", draft.hits.len()));
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for (i, (hit, on)) in draft.hits.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.checkbox(on, "");
                            if ui.link(&hit.place).on_hover_text("Open it").clicked() {
                                go = Some(i);
                            }
                        });
                        let text: String = hit.text.chars().take(120).collect();
                        ui.indent(("hit", i), |ui| ui.weak(text));
                    }
                });
                let n = draft.hits.iter().filter(|h| h.1).count();
                replace =
                    ui.add_enabled(n > 0, egui::Button::new(format!("Replace in {n}"))).clicked();
            } else if draft.searched.is_some() {
                ui.weak("Find again for what's asked now.");
            }
        });
    if find && !draft.find.is_empty() {
        app.find_text(&mut draft);
    }
    if replace {
        app.replace_text(&draft);
        // Found again once applied: what's left.
        draft.refind = true;
        ctx.request_repaint();
    }
    if let Some(i) = go {
        let hit = &draft.hits[i].0;
        let usage = mg_module::rename::Usage {
            from: hit.key,
            path: hit.path_text(),
            place: hit.place.clone(),
        };
        app.go_to_usage(&usage);
    }
    if open {
        app.text_replace = Some(draft);
    }
}
