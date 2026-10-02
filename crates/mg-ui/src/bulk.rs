//! Bulk edits: Update Instances for any set of blueprints (a selection in
//! the custom palette, a whole category, or one blueprint's editor), as
//! Aurora's palette does it for one: every area or the one shown, with the
//! objects it would change listed to untick.

use std::collections::HashSet;

use egui::Context;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Struct;
use mg_module::instances::{Placing, git_list, update};
use mg_resman::ResKey;

use crate::{Action, Moonglow};

/// A placed object Update Instances would change.
#[derive(Debug, Clone, PartialEq)]
pub struct Affected {
    pub area: ResRef,
    /// Its GIT list and index there.
    pub list: &'static str,
    pub index: usize,
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
                    objects.push(Affected { area, list, index, label, on: true });
                }
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
        let (Some(ws), Some(game)) = (self.ws.as_mut(), self.game.as_ref()) else { return 0 };
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
        let items = |r: ResRef| -> Option<Struct> {
            let k = ResKey::new(r, ResType::UTI);
            let data = module
                .get(&k)
                .map(<[u8]>::to_vec)
                .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
            mg_gff::Gff::read(&data).ok().map(|g| g.root)
        };
        let placing = Placing { game, item: &items };
        let blueprint = |t: ResType, r: ResRef| blueprints.get(&ResKey::new(r, t)).cloned();
        let mut edits = Vec::new();
        let mut total = 0;
        for (key, git) in &gits {
            let (key, area) = (*key, key.resref);
            let which = |list: &str, index: usize| {
                draft
                    .objects
                    .iter()
                    .any(|o| o.area == area && o.list == list && o.index == index && o.on)
            };
            let Some((new, n)) = update(&placing, git, &blueprint, &which) else { continue };
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
                "Updated {total} object(s) from {} blueprint(s)",
                draft.blueprints.len()
            ));
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
        .open(&mut open)
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
                cancel = ui.button("Cancel").clicked();
            });
        });
    if ok {
        app.apply_update(&draft);
    } else if open && !cancel {
        app.update_draft = Some(draft);
    }
}
